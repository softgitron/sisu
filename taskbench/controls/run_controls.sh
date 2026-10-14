#!/usr/bin/env bash
# Builds and runs the Go and Java (virtual threads) controls of the bet-3 harness
# with the same workloads, scale and CSV schema as run_all.sh (both take their
# parameters from scale.sh), appending to OUT_DIR/<benchmark>.csv and recording
# versions and flags in OUT_DIR/controls-versions.txt and the machine's load
# during every process in OUT_DIR/load.csv (load.sh).
#
# Usage: controls/run_controls.sh OUT_DIR [--threads N] [--machine-id ID] [--scale smoke|full]
#                                 [--only BENCH[,BENCH...]] [--build-only | --no-build]
#                                 [--quiet-below CPUS]
#
# --build-only builds both controls into OUT_DIR/.controls-build and writes
# controls-versions.txt; --no-build runs from an earlier --build-only. run_all.sh
# builds once and then runs the controls of each benchmark right after the
# harness's own configurations, repetition by repetition.
#
# GOMAXPROCS and the virtual-thread scheduler's parallelism are both set to
# --threads, matching the harness's worker count. Java warm-up: spawn-join runs
# 20000 unrecorded rounds first (the JIT's warm-up; at least the protocol's 10k);
# echo runs --warmup-secs unrecorded; both are recorded in the params column.
#
# Starvation: Go runs with its default asynchronous preemption and no manual
# runtime.Gosched, the bet's baseline (the owner's policy 8); Java, which has no
# preemption of virtual threads, runs with no yields and with Thread.yield every
# K iterations for each K of the harness.
set -uo pipefail

usage() {
  echo "usage: $0 OUT_DIR [--threads N] [--machine-id ID] [--scale smoke|full] [--only BENCH,...] [--build-only|--no-build] [--quiet-below CPUS]" >&2
  exit 2
}
[ $# -ge 1 ] || usage
OUT=$1; shift
THREADS=$(nproc)
MACHINE=$(cat /proc/sys/kernel/hostname 2>/dev/null || hostname)
SCALE=smoke
ONLY=""
BUILD=1
RUN=1
LOAD_QUIET=off
while [ $# -gt 0 ]; do
  case "$1" in
    --threads) THREADS=$2; shift 2 ;;
    --machine-id) MACHINE=$2; shift 2 ;;
    --scale) SCALE=$2; shift 2 ;;
    --only) ONLY=$2; shift 2 ;;
    --build-only) RUN=0; shift ;;
    --no-build) BUILD=0; shift ;;
    --quiet-below) LOAD_QUIET=$2; shift 2 ;;
    *) usage ;;
  esac
done
HERE=$(cd "$(dirname "$0")" && pwd)
# shellcheck source=../scale.sh
. "$HERE/../scale.sh"
scale_params "$SCALE" "$THREADS"
mkdir -p "$OUT"
OUT=$(cd "$OUT" && pwd)
BUILD_DIR="$OUT/.controls-build"
# shellcheck source=../load.sh
. "$HERE/../load.sh"

GO=${GO:-$(command -v go || echo /usr/local/go/bin/go)}
JAVA=${JAVA:-java}
JAVAC=${JAVAC:-javac}
JFLAGS=${JFLAGS:-"-Djdk.virtualThreadScheduler.parallelism=$THREADS -Djdk.virtualThreadScheduler.maxPoolSize=$THREADS -XX:+UseG1GC"}

if [ $BUILD = 1 ]; then
  mkdir -p "$BUILD_DIR/java"
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
    echo "=== controls build"
    echo "date_utc: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
    echo "machine_id: $MACHINE"
    echo "scale: $SCALE"
    echo "threads: $THREADS (GOMAXPROCS and jdk.virtualThreadScheduler.parallelism)"
    echo "go: $("$GO" version 2>&1)"
    echo "java: $("$JAVA" -version 2>&1 | grep -v 'Picked up' | tr '\n' ' ')"
    echo "javac: $("$JAVAC" -version 2>&1 | grep -v 'Picked up')"
    echo "java_flags: $JFLAGS"
    echo "java_tool_options: flags:${JTO_FLAGS:- none}; system properties set (values not recorded):${JTO_PROPS:- none}"
    echo "warm-up: spawn-join Go ${SJ_WARMUP} rounds, Java 20000 rounds (JIT); echo ${ECHO_WARMUP} s unrecorded"
  } >> "$VERS"
  (cd "$HERE/go" && "$GO" build -o "$BUILD_DIR/gocontrol" .) || echo "go build failed; skipping Go control" >&2
  "$JAVAC" -d "$BUILD_DIR/java" "$HERE/java/TaskBench.java" 2>&1 | grep -v 'Picked up' >&2
  [ -f "$BUILD_DIR/java/TaskBench.class" ] || echo "javac failed (virtual threads need JDK 21+); skipping Java control" >&2
fi
[ $RUN = 1 ] || exit 0
go_ok=0; java_ok=0
[ -x "$BUILD_DIR/gocontrol" ] && go_ok=1
[ -f "$BUILD_DIR/java/TaskBench.class" ] && java_ok=1

HEADER="schema,run_id,timestamp_utc,machine_id,implementation,benchmark,strategy,reservation_bytes,stack_bytes,threads,tasks,params,metric,value,unit,status,note"

# record IMPL BENCH METRIC TASKS PARAMS NOTE: a failed row written by the script.
record() {
  local impl=$1 bench=$2 metric=$3 tasks=$4 params=$5 note=$6
  local f="$OUT/$bench.csv"
  [ -s "$f" ] || echo "$HEADER" > "$f"
  printf '1,script-%s,%s,%s,%s,%s,%s,,,%s,%s,"%s",%s,,,failed,"%s"\n' \
    "$(date -u +%s)" "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$MACHINE" "$impl" "$bench" "$impl" "$THREADS" "$tasks" \
    "$params" "$metric" "$note" >> "$f"
}

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

# go_proc / java_proc BENCH ARGS...: one control process, its rows in $LOAD_TMP.
go_proc() {
  GOMAXPROCS=$THREADS "$BUILD_DIR/gocontrol" "$@"
}
java_proc() {
  # shellcheck disable=SC2086
  "$JAVA" $JFLAGS -cp "$BUILD_DIR/java" TaskBench "$@" 2>&1 | grep -v 'Picked up' >&2
  return "${PIPESTATUS[0]}"
}
go_run() {
  local bench=$1; shift
  [ $go_ok = 1 ] || return 0
  echo "== go $bench $*"
  load_run go "$bench" "$*" go_proc "$bench" --threads "$THREADS" --machine-id "$MACHINE" --csv "$LOAD_TMP" --quiet "$@"
  record_exit go "$bench" $? "$@"
}
java_run() {
  local bench=$1; shift
  [ $java_ok = 1 ] || return 0
  echo "== java $bench $*"
  load_run java "$bench" "$*" java_proc "$bench" --threads "$THREADS" --machine-id "$MACHINE" --csv "$LOAD_TMP" --quiet "$@"
  record_exit java "$bench" $? "$@"
}
# connections IMPL BENCH N ARGS...: runs a connection benchmark, or records a
# failed row when N in-process connections exceed the RLIMIT_NOFILE hard limit.
connections() {
  local impl=$1 bench=$2 n=$3; shift 3
  if ! fits_connections "$n"; then
    [ "$impl" = go ] && [ $go_ok = 0 ] && return 0
    [ "$impl" = java ] && [ $java_ok = 0 ] && return 0
    record "$impl" "$bench" "${bench//-/_}" "$n" "connections=$n" \
      "RLIMIT_NOFILE hard limit $(nofile_hard) is too low for $n connections in one process (needs $((2 * n + 64)))"
    echo "   -> $impl $bench at $n connections: not run (RLIMIT_NOFILE)" >&2
    return 0
  fi
  "${impl}_run" "$bench" --connections "$n" "$@"
}

for impl in go java; do
  run="${impl}_run"
  if selected spawn-join; then $run spawn-join --rounds "$SJ_ROUNDS" $([ $impl = go ] && echo --warmup "$SJ_WARMUP"); fi
  if selected idle-memory; then
    for n in $IM_SIZES; do
      # Go keeps an excursion's stack until a collection shrinks it: at IM_SKIP_AT
      # tasks it measures only the shallow phase (policy 7). Java copies frames into
      # heap chunks when a virtual thread parks, so it runs both phases everywhere.
      # shellcheck disable=SC2046
      $run idle-memory --tasks "$n" --settle-ms "$IM_SETTLE_MS" $([ $impl = go ] && skip_excursion_at "$n")
    done
  fi
  if selected max-tasks; then $run max-tasks --tasks "$MT_CAP" --mem-guard "$(numfmt --from=iec "$MT_GUARD")" --hold-secs "$MT_HOLD"; fi
  if selected echo; then
    for c in $ECHO_CONNS; do
      connections $impl echo "$c" --duration "$ECHO_DURATION" --warmup-secs "$ECHO_WARMUP"
    done
  fi
  if selected idle-connections; then connections $impl idle-connections "$IC_CONNS" --settle-ms "$IC_SETTLE_MS"; fi
  if selected starvation; then
    if [ $impl = go ]; then
      $run starvation --tasks "$ST_TASKS" --duration "$ST_DURATION"
    else
      for k in 0 $ST_KS; do $run starvation --tasks "$ST_TASKS" --duration "$ST_DURATION" --yield-every "$k"; done
    fi
  fi
done
echo "controls written to $OUT"
