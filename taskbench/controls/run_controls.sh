#!/usr/bin/env bash
# Builds and runs the Go and Java (virtual threads) controls of the bet-3 harness
# with the same workloads, scale and CSV schema as run_all.sh, appending to
# OUT_DIR/<benchmark>.csv and recording versions and flags in
# OUT_DIR/controls-versions.txt.
#
# Usage: controls/run_controls.sh OUT_DIR [--threads N] [--machine-id ID] [--scale smoke|full]
#                                 [--only BENCH[,BENCH...]]
#
# GOMAXPROCS and the virtual-thread scheduler's parallelism are both set to
# --threads, matching the harness's worker count. Java warm-up: spawn-join runs
# 20000 unrecorded rounds first; echo runs --warmup-secs unrecorded; both are
# recorded in the params column.
set -uo pipefail

usage() { echo "usage: $0 OUT_DIR [--threads N] [--machine-id ID] [--scale smoke|full] [--only BENCH,...]" >&2; exit 2; }
[ $# -ge 1 ] || usage
OUT=$1; shift
THREADS=$(nproc)
MACHINE=$(cat /proc/sys/kernel/hostname 2>/dev/null || hostname)
SCALE=smoke
ONLY=""
while [ $# -gt 0 ]; do
  case "$1" in
    --threads) THREADS=$2; shift 2 ;;
    --machine-id) MACHINE=$2; shift 2 ;;
    --scale) SCALE=$2; shift 2 ;;
    --only) ONLY=$2; shift 2 ;;
    *) usage ;;
  esac
done
HERE=$(cd "$(dirname "$0")" && pwd)
mkdir -p "$OUT"
OUT=$(cd "$OUT" && pwd)
BUILD="$OUT/.controls-build"
mkdir -p "$BUILD/java"

GO=${GO:-$(command -v go || echo /usr/local/go/bin/go)}
JAVA=${JAVA:-java}
JAVAC=${JAVAC:-javac}
JFLAGS=${JFLAGS:-"-Djdk.virtualThreadScheduler.parallelism=$THREADS -Djdk.virtualThreadScheduler.maxPoolSize=$THREADS -XX:+UseG1GC"}

# JAVA_TOOL_OPTIONS often carries site settings (proxies, trust stores) that do
# not affect the benchmarks: record its JVM flags, and of its -D system
# properties only the names.
JTO_FLAGS=""; JTO_PROPS=""
for o in ${JAVA_TOOL_OPTIONS:-}; do
  case "$o" in
    -D*) k=${o%%=*}; JTO_PROPS="$JTO_PROPS ${k#-D}" ;;
    *) JTO_FLAGS="$JTO_FLAGS $o" ;;
  esac
done

VERS="$OUT/controls-versions.txt"
{
  echo "date_utc: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "machine_id: $MACHINE"
  echo "scale: $SCALE"
  echo "threads: $THREADS (GOMAXPROCS and jdk.virtualThreadScheduler.parallelism)"
  echo "go: $("$GO" version 2>&1)"
  echo "java: $("$JAVA" -version 2>&1 | grep -v 'Picked up' | tr '\n' ' ')"
  echo "javac: $("$JAVAC" -version 2>&1 | grep -v 'Picked up')"
  echo "java_flags: $JFLAGS"
  echo "java_tool_options: flags:${JTO_FLAGS:- none}; system properties set (values not recorded):${JTO_PROPS:- none}"
} > "$VERS"

go_ok=1; java_ok=1
(cd "$HERE/go" && "$GO" build -o "$BUILD/gocontrol" .) || { echo "go build failed; skipping Go control" >&2; go_ok=0; }
"$JAVAC" -d "$BUILD/java" "$HERE/java/TaskBench.java" 2>&1 | grep -v 'Picked up' >&2
[ -f "$BUILD/java/TaskBench.class" ] || { echo "javac failed; skipping Java control" >&2; java_ok=0; }

if [ "$SCALE" = full ]; then
  SJ="--rounds 200000"; IM_SIZES="100000 1000000"; MT=1000000; ECHO="--connections 10000 --duration 30 --warmup-secs 5"
  IC="--connections 100000"; ST="--duration 10"
else
  SJ="--rounds 20000"; IM_SIZES="10000"; MT=100000; ECHO="--connections 1000 --duration 3 --warmup-secs 1"
  IC="--connections 5000"; ST="--duration 2"
fi

HEADER="schema,run_id,timestamp_utc,machine_id,implementation,benchmark,strategy,reservation_bytes,stack_bytes,threads,tasks,params,metric,value,unit,status,note"

# record_exit IMPL BENCH STATUS ARGS...: a control that died (or was killed, for
# example by the OOM killer) becomes a process_exit row, as in run_all.sh.
record_exit() {
  local impl=$1 bench=$2 rc=$3; shift 3
  [ "$rc" -eq 0 ] && return 0
  local f="$OUT/$bench.csv"
  [ -s "$f" ] || echo "$HEADER" > "$f"
  printf '1,crash-%s,%s,%s,%s,%s,%s,,,%s,,"%s",process_exit,%s,code,failed,"exit status %s (>128: killed by signal %s)"\n' \
    "$(date -u +%s)" "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$MACHINE" "$impl" "$bench" "$impl" "$THREADS" "$*" "$rc" "$rc" "$((rc - 128))" >> "$f"
  echo "   -> $impl $bench exit status $rc recorded" >&2
}

# The Go flag package accepts --name value, so both controls take the same options.
# selected BENCH: whether --only (if given) includes BENCH.
selected() { [ -z "$ONLY" ] || [[ ",$ONLY," == *",$1,"* ]]; }

go_run() {
  local bench=$1; shift
  [ $go_ok = 1 ] || return 0
  selected "$bench" || return 0
  echo "== go $bench $*"
  GOMAXPROCS=$THREADS "$BUILD/gocontrol" "$bench" --threads "$THREADS" --machine-id "$MACHINE" --csv "$OUT/$bench.csv" --quiet "$@"
  record_exit go "$bench" $? "$@"
}
java_run() {
  local bench=$1; shift
  [ $java_ok = 1 ] || return 0
  selected "$bench" || return 0
  echo "== java $bench $*"
  # shellcheck disable=SC2086
  "$JAVA" $JFLAGS -cp "$BUILD/java" TaskBench "$bench" --threads "$THREADS" --machine-id "$MACHINE" --csv "$OUT/$bench.csv" --quiet "$@" 2>&1 \
    | grep -v 'Picked up' >&2
  record_exit java "$bench" "${PIPESTATUS[0]}" "$@"
}

for run in go_run java_run; do
  # shellcheck disable=SC2086
  $run spawn-join $SJ
  for n in $IM_SIZES; do $run idle-memory --tasks "$n"; done
  $run max-tasks --tasks "$MT"
  # shellcheck disable=SC2086
  $run echo $ECHO
  # shellcheck disable=SC2086
  $run idle-connections $IC
  # shellcheck disable=SC2086
  $run starvation $ST
  # shellcheck disable=SC2086
  $run starvation $ST --yield-every 10000
done
echo "controls written to $OUT (versions in $VERS)"
