#!/usr/bin/env bash
# Runs every benchmark of the bet-3 harness for every strategy configuration and
# the Go and Java controls, writing results/<machine>/<date>/<benchmark>.csv,
# machine-spec.csv and versions.txt.
#
# Usage: ./run_all.sh [--scale smoke|full] [--threads N] [--machine-id ID] [--out DIR] [--no-controls]
#                     [--only BENCH[,BENCH...]]
#
# smoke (default) sizes every run for a development machine (minutes in total).
# full is for the dedicated machines of the Phase 1 report: 100k and 1M idle
# tasks, 10k echo connections, 100k idle connections, 1M cancelled reads. It
# needs RLIMIT_NOFILE >= 200064 (raise the hard limit; sysctls stay at their
# defaults, which is what the bet-3 scale row is about) and about 16 GiB of RAM.
#
# --only runs a subset of the benchmarks at the chosen scale, for example
# "--scale full --only idle-memory,max-tasks" on a 16 GiB machine whose
# RLIMIT_NOFILE is too low for the full echo runs.
#
# A configuration that fails (for example strategy A at 100k tasks, which hits
# vm.max_map_count) is a result and is recorded with status "failed". A process
# that dies (a crash would be a harness bug) is recorded as a "process_exit" row.
set -uo pipefail
cd "$(dirname "$0")"

SCALE=smoke
THREADS=$(nproc)
MACHINE=${MACHINE_ID:-$(cat /proc/sys/kernel/hostname 2>/dev/null || hostname)}
OUT=""
CONTROLS=1
ONLY=""
while [ $# -gt 0 ]; do
  case "$1" in
    --scale) SCALE=$2; shift 2 ;;
    --threads) THREADS=$2; shift 2 ;;
    --machine-id) MACHINE=$2; shift 2 ;;
    --out) OUT=$2; shift 2 ;;
    --no-controls) CONTROLS=0; shift ;;
    --only) ONLY=$2; shift 2 ;;
    *) echo "usage: $0 [--scale smoke|full] [--threads N] [--machine-id ID] [--out DIR] [--no-controls] [--only BENCH,...]" >&2; exit 2 ;;
  esac
done
DATE=$(date -u +%Y-%m-%d)
OUT=${OUT:-results/$MACHINE/$DATE}
mkdir -p "$OUT"

cargo build --release --quiet || exit 1
BIN=target/release/taskbench

{
  echo "date_utc: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "scale: $SCALE threads: $THREADS machine_id: $MACHINE"
  echo "rustc: $(rustc --version)"
  echo "cargo: $(cargo --version)"
  echo "uname: $(uname -a)"
  echo "taskbench: $($BIN help | head -1)"
} > "$OUT/versions.txt"
"$BIN" machine-spec --machine-id "$MACHINE" --csv "$OUT/machine-spec.csv" --quiet

# run BENCH ARGS...: one benchmark process; a non-zero exit becomes a CSV row.
run() {
  local bench=$1; shift
  if [ -n "$ONLY" ] && [[ ",$ONLY," != *",$bench,"* ]]; then return 0; fi
  echo "== $bench $*"
  "$BIN" "$bench" "$@" --threads "$THREADS" --machine-id "$MACHINE" --csv "$OUT/$bench.csv" --quiet
  local rc=$?
  if [ $rc -ne 0 ]; then
    local strat=B base=A prev=""
    for a in "$@"; do
      [ "$prev" = "--strategy" ] && strat=$a
      [ "$prev" = "--base" ] && base=$a
      prev=$a
    done
    [ "$strat" = D ] && strat="D($base)"
    local f="$OUT/$bench.csv"
    [ -s "$f" ] || "$BIN" csv-header > "$f"
    printf '1,crash-%s,%s,%s,taskbench,%s,%s,,,%s,,"%s",process_exit,%s,code,failed,"exit status %s (>128: killed by signal %s)"\n' \
      "$(date -u +%s)" "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$MACHINE" "$bench" "$strat" "$THREADS" "$*" "$rc" "$rc" "$((rc - 128))" >> "$f"
    echo "   -> exit status $rc recorded" >&2
  fi
}

if [ "$SCALE" = full ]; then
  ROUNDS=200000; IM_SIZES="100000 1000000"; MT=1000000; GUARD=512M
  ECHO_ARGS="--connections 10000 --duration 30 --warmup-secs 5"; IC=100000
  ST_ARGS="--duration 10"; PO_REPS=31; CU_TASKS=1000000; CU_CONC=8192; OS_MT=30000; OS_IM=10000
else
  ROUNDS=20000; IM_SIZES="10000"; MT=100000; GUARD=1G
  ECHO_ARGS="--connections 1000 --duration 3 --warmup-secs 1"; IC=5000
  ST_ARGS="--duration 2"; PO_REPS=11; CU_TASKS=4096; CU_CONC=512; OS_MT=5000; OS_IM=2000
fi

# Stackful configurations: the five strategies and the reservation sizes of Phase 1.
RESERVATIONS="128K 1M 8M"

# spawn-join
# shellcheck disable=SC2086
for s in A B C D "D --base B" E tokio os-threads; do run spawn-join --strategy $s --rounds $ROUNDS; done
run spawn-join --strategy A --stack-cache 1024 --rounds $ROUNDS
run spawn-join --strategy D --stack-cache 1024 --rounds $ROUNDS
run spawn-join --strategy B --rounds $((ROUNDS / 10)) --batch 100
run spawn-join --strategy B --rounds $ROUNDS --scope-per-round

# idle-memory, after shallow use and after a 32 KiB excursion
for n in $IM_SIZES; do
  for r in $RESERVATIONS; do
    for s in A B D; do run idle-memory --strategy "$s" --reservation "$r" --tasks "$n"; done
    run idle-memory --strategy D --base B --reservation "$r" --tasks "$n"
  done
  for c in 4K 8K 16K; do run idle-memory --strategy C --stack-size "$c" --tasks "$n"; done
  run idle-memory --strategy E --tasks "$n"
  run idle-memory --strategy tokio --tasks "$n"
done
run idle-memory --strategy os-threads --tasks $OS_IM

# max-tasks under the current sysctls
for s in A B C D "D --base B" E tokio; do run max-tasks --strategy $s --tasks $MT --mem-guard $GUARD; done
run max-tasks --strategy os-threads --tasks $OS_MT --mem-guard $GUARD

# echo and idle connections ("D --base B" is split into words on purpose)
# shellcheck disable=SC2086
for s in A B C D "D --base B" E tokio; do run echo --strategy $s $ECHO_ARGS; done
# shellcheck disable=SC2086
for s in A B C D "D --base B" E tokio; do run idle-connections --strategy $s --connections $IC; done

# deep recursion: clean "stack exhausted" everywhere
for r in $RESERVATIONS; do
  for s in A B D; do run deep-recursion --strategy "$s" --reservation "$r"; done
  run deep-recursion --strategy D --base B --reservation "$r"
done
for c in 2K 8K 16K; do run deep-recursion --strategy C --stack-size "$c"; done
run deep-recursion --strategy E
run deep-recursion --strategy tokio

# starvation: yield points every 10000 iterations (about 20 us), none, and dense
# shellcheck disable=SC2086
for s in A B C D "D --base B" E tokio os-threads; do run starvation --strategy $s $ST_ARGS; done
# shellcheck disable=SC2086
run starvation --strategy B $ST_ARGS --yield-every 0
# shellcheck disable=SC2086
run starvation --strategy B $ST_ARGS --yield-every 1000

# probe overhead
for s in B C A E; do run probe-overhead --strategy "$s" --reps $PO_REPS; done

# io_uring cancellation (amendment A1)
for s in A B C D "D --base B"; do run cancel-uring --strategy $s --tasks $CU_TASKS --concurrency $CU_CONC; done
run cancel-uring --strategy B --tasks $CU_TASKS --concurrency $CU_CONC --source file
run cancel-uring --strategy B --tasks $CU_TASKS --concurrency $CU_CONC --source file-buffered --async-percent 100
run cancel-uring --strategy B --tasks $((CU_TASKS / 4)) --concurrency $CU_CONC --break-contract

if [ $CONTROLS = 1 ]; then
  controls/run_controls.sh "$OUT" --threads "$THREADS" --machine-id "$MACHINE" --scale "$SCALE" ${ONLY:+--only "$ONLY"}
fi
rm -rf "$OUT/.controls-build"
echo "results in $OUT"
