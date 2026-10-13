#!/usr/bin/env bash
# Runs every benchmark of the bet-3 harness for every strategy configuration and
# the Go and Java controls, writing results/<machine>/<date>/<benchmark>.csv,
# machine-spec.csv, versions.txt, controls-versions.txt and timing.csv.
#
# Usage: ./run_all.sh [--scale smoke|full] [--reps N] [--threads N] [--machine-id ID] [--out DIR]
#                     [--no-controls] [--only BENCH[,BENCH...]]
#
# smoke (default) sizes every run for a development machine (minutes in total,
# one repetition). full is the Phase 1 report's scale (scale.sh): 100k and 1M
# idle tasks, 1M tasks held for 60 s, 10k and 1000 echo connections, the most
# idle connections the RLIMIT_NOFILE hard limit allows (up to 100k), 1M cancelled
# reads, and five repetitions of every cell in fresh processes (proposal/20,
# section 6.5). It needs about 16 GiB of RAM; sysctls stay at their defaults,
# which is what the bet-3 scale row is about.
#
# Repetitions run in rep-major order: every benchmark (with its controls) once,
# then again, so a run that is cut short still has every benchmark.
#
# --only runs a subset of the benchmarks at the chosen scale, for example
# "--scale full --only idle-memory,max-tasks".
#
# A configuration that fails (for example strategy A at 100k tasks, which hits
# vm.max_map_count) is a result and is recorded with status "failed". A process
# that dies (a crash would be a harness bug) is recorded as a "process_exit" row.
set -uo pipefail
cd "$(dirname "$0")"
. ./scale.sh

SCALE=smoke
REPS=""
THREADS=$(nproc)
MACHINE=${MACHINE_ID:-$(cat /proc/sys/kernel/hostname 2>/dev/null || hostname)}
OUT=""
CONTROLS=1
ONLY=""
usage() {
  echo "usage: $0 [--scale smoke|full] [--reps N] [--threads N] [--machine-id ID] [--out DIR] [--no-controls] [--only BENCH,...]" >&2
  exit 2
}
while [ $# -gt 0 ]; do
  case "$1" in
    --scale) SCALE=$2; shift 2 ;;
    --reps) REPS=$2; shift 2 ;;
    --threads) THREADS=$2; shift 2 ;;
    --machine-id) MACHINE=$2; shift 2 ;;
    --out) OUT=$2; shift 2 ;;
    --no-controls) CONTROLS=0; shift ;;
    --only) ONLY=$2; shift 2 ;;
    *) usage ;;
  esac
done
case "$SCALE" in smoke|full) ;; *) usage ;; esac
scale_params "$SCALE" "$THREADS"
REPS=${REPS:-$REPS_DEFAULT}
DATE=$(date -u +%Y-%m-%d)
OUT=${OUT:-results/$MACHINE/$DATE}
mkdir -p "$OUT"

cargo build --release --quiet || exit 1
BIN=target/release/taskbench

# A digest of the sources the binary and the controls were built from, so a
# results directory names its harness even when it was produced from a working
# tree (the commit is GITHUB_SHA on CI, or whatever commit the results land in).
source_digest() {
  find src tests controls Cargo.toml Cargo.lock run_all.sh scale.sh -type f \
    \( -name '*.rs' -o -name '*.go' -o -name '*.java' -o -name '*.sh' -o -name '*.py' -o -name '*.toml' \
       -o -name '*.lock' -o -name 'go.mod' \) -print0 | sort -z | xargs -0 sha256sum | sha256sum | cut -d' ' -f1
}
# One record per invocation (a results directory may be filled by several, one
# per --only group).
{
  echo "=== run_all.sh --scale $SCALE --reps $REPS${ONLY:+ --only $ONLY}"
  echo "date_utc: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "scale: $SCALE reps: $REPS threads: $THREADS machine_id: $MACHINE"
  echo "rustc: $(rustc --version)"
  echo "cargo: $(cargo --version)"
  echo "uname: $(uname -a)"
  echo "taskbench: $($BIN help | head -1) version $(grep -m1 '^version' Cargo.toml | cut -d'"' -f2)"
  echo "source_sha256: $(source_digest)"
  echo "commit: ${GITHUB_SHA:-not recorded (working tree; see source_sha256)}"
  echo "rlimit_nofile_hard: $(nofile_hard); idle connections: $IC_CONNS; echo connections: $ECHO_CONNS"
} >> "$OUT/versions.txt"
"$BIN" machine-spec --machine-id "$MACHINE" --csv "$OUT/machine-spec.csv" --quiet
TIMING="$OUT/timing.csv"
[ -s "$TIMING" ] || echo "rep,benchmark,start_utc,seconds" > "$TIMING"

# run BENCH ARGS...: one benchmark process; a non-zero exit becomes a CSV row.
run() {
  local bench=$1; shift
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

# The reservation sizes of Phase 1. "D --base B" is split into words on purpose
# wherever it is used unquoted.
RESERVATIONS="128K 1M 8M"

bench_spawn_join() {
  local sj="--rounds $SJ_ROUNDS --warmup $SJ_WARMUP"
  # shellcheck disable=SC2086
  for s in A B C D "D --base B" E tokio os-threads; do run spawn-join --strategy $s $sj; done
  # shellcheck disable=SC2086
  run spawn-join --strategy A --stack-cache 1024 $sj
  # shellcheck disable=SC2086
  run spawn-join --strategy D --stack-cache 1024 $sj
  run spawn-join --strategy B --rounds $((SJ_ROUNDS / 10)) --warmup $((SJ_WARMUP / 10)) --batch 100
  # shellcheck disable=SC2086
  run spawn-join --strategy B $sj --scope-per-round
}

# idle-memory, after shallow use and after a 32 KiB excursion. Strategies that
# keep their pages (A, B, C) measure only the shallow phase at IM_SKIP_AT tasks
# and above (policy 7); D(A), D(B), E and tokio run both phases everywhere.
bench_idle_memory() {
  local im="--settle-ms $IM_SETTLE_MS"
  for n in $IM_SIZES; do
    local skip
    skip=$(skip_excursion_at "$n")
    for r in $RESERVATIONS; do
      # shellcheck disable=SC2086
      for s in A B; do run idle-memory --strategy "$s" --reservation "$r" --tasks "$n" $im $skip; done
      # shellcheck disable=SC2086
      run idle-memory --strategy D --reservation "$r" --tasks "$n" $im
      # shellcheck disable=SC2086
      run idle-memory --strategy D --base B --reservation "$r" --tasks "$n" $im
    done
    # shellcheck disable=SC2086
    for c in 4K 8K 16K; do run idle-memory --strategy C --stack-size "$c" --tasks "$n" $im $skip; done
    # shellcheck disable=SC2086
    run idle-memory --strategy E --tasks "$n" $im
    # shellcheck disable=SC2086
    run idle-memory --strategy tokio --tasks "$n" $im
  done
  # shellcheck disable=SC2086
  run idle-memory --strategy os-threads --tasks "$OS_IM" $im
}

# max-tasks under the current sysctls; a run that reaches the cap holds 60 s at full scale.
bench_max_tasks() {
  local hold=""
  [ "$MT_HOLD" != 0 ] && hold="--hold-secs $MT_HOLD"
  # shellcheck disable=SC2086
  for s in A B C D "D --base B" E tokio; do run max-tasks --strategy $s --tasks "$MT_CAP" --mem-guard "$MT_GUARD" $hold; done
  # shellcheck disable=SC2086
  run max-tasks --strategy os-threads --tasks "$OS_MT" --mem-guard "$MT_GUARD" $hold
}

bench_echo() {
  for c in $ECHO_CONNS; do
    # shellcheck disable=SC2086
    for s in A B C D "D --base B" E tokio; do
      run echo --strategy $s --connections "$c" --duration "$ECHO_DURATION" --warmup-secs "$ECHO_WARMUP"
    done
  done
}

bench_idle_connections() {
  # shellcheck disable=SC2086
  for s in A B C D "D --base B" E tokio; do
    run idle-connections --strategy $s --connections "$IC_CONNS" --settle-ms "$IC_SETTLE_MS"
  done
}

# deep recursion: clean "stack exhausted" everywhere, at the default (policy) probe
# reserve; --framed adds the worst frame shape for the reserve (stackful only)
bench_deep_recursion() {
  for r in $RESERVATIONS; do
    for s in A B D; do run deep-recursion --strategy "$s" --reservation "$r" --framed; done
    run deep-recursion --strategy D --base B --reservation "$r" --framed
  done
  for c in 2K 8K 16K; do run deep-recursion --strategy C --stack-size "$c" --framed; done
  run deep-recursion --strategy E
  run deep-recursion --strategy tokio
}

# starvation: four compute tasks per worker with yield points every K iterations
# (ST_KS), plus a dense budget (1000) and none at all (0: reported, not counted)
bench_starvation() {
  local st="--tasks $ST_TASKS --duration $ST_DURATION"
  for k in $ST_KS; do
    # shellcheck disable=SC2086
    for s in A B C D "D --base B" E tokio; do run starvation --strategy $s $st --yield-every "$k"; done
  done
  # shellcheck disable=SC2086
  run starvation --strategy os-threads $st
  # shellcheck disable=SC2086
  run starvation --strategy B $st --yield-every 1000
  # shellcheck disable=SC2086
  run starvation --strategy B $st --yield-every 0
}

bench_probe_overhead() {
  # shellcheck disable=SC2086
  for s in B C "D --base B" A E; do run probe-overhead --strategy $s --reps "$PO_REPS"; done
}

# io_uring cancellation (amendment A1): the registered procedure (every read
# cancelled in flight, 4 KiB stack buffers) and the harness's races, on each
# stackful strategy; then file-backed reads and the negative control on B.
bench_cancel_uring() {
  local cu="--tasks $CU_TASKS --concurrency $CU_CONC"
  # shellcheck disable=SC2086
  for s in A B C D "D --base B"; do run cancel-uring --strategy $s $cu --pipe-mix cancel --buf-bytes 4096; done
  # shellcheck disable=SC2086
  for s in A B C D "D --base B"; do run cancel-uring --strategy $s $cu; done
  run cancel-uring --strategy B --tasks "$CU_FILE_TASKS" --concurrency "$CU_CONC" --source file
  run cancel-uring --strategy B --tasks "$CU_FILE_TASKS" --concurrency "$CU_CONC" --source file-buffered --async-percent 100
  run cancel-uring --strategy B --tasks $((CU_TASKS / 4)) --concurrency "$CU_CONC" --break-contract
}

BENCHES="spawn-join idle-memory max-tasks echo idle-connections deep-recursion starvation probe-overhead cancel-uring"
CONTROLLED=",spawn-join,idle-memory,max-tasks,echo,idle-connections,starvation,"
selected() { [ -z "$ONLY" ] || [[ ",$ONLY," == *",$1,"* ]]; }

ctl=(controls/run_controls.sh "$OUT" --threads "$THREADS" --machine-id "$MACHINE" --scale "$SCALE")
if [ $CONTROLS = 1 ]; then
  "${ctl[@]}" --build-only || CONTROLS=0
fi
for rep in $(seq 1 "$REPS"); do
  for b in $BENCHES; do
    selected "$b" || continue
    echo "==== repetition $rep of $REPS: $b"
    t0=$(date +%s)
    start=$(date -u +%Y-%m-%dT%H:%M:%SZ)
    "bench_${b//-/_}"
    if [ $CONTROLS = 1 ] && [[ "$CONTROLLED" == *",$b,"* ]]; then
      "${ctl[@]}" --no-build --only "$b"
    fi
    echo "$rep,$b,$start,$(($(date +%s) - t0))" >> "$TIMING"
  done
done
rm -rf "$OUT/.controls-build"
echo "results in $OUT"
