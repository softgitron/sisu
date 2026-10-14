# shellcheck shell=bash
# load.sh: the machine-load record of run_all.sh and the controls (sourced by
# both, after OUT is set).
#
# Every benchmark process gets a row in OUT/load.csv: its wall-clock time, the
# CPU time it used, and the CPU time the rest of the machine used meanwhile (the
# user, nice, system, interrupt and softirq time of all CPUs from /proc/stat,
# less the process's own from the shell's `times`), with the hypervisor's steal
# time. Interrupt and softirq time count because a kernel with tick-based
# accounting (no CONFIG_IRQ_TIME_ACCOUNTING, as on machine 1) charges them to
# the task they interrupted: the echo benchmark's loopback traffic is in its own
# time. On a kernel with that option, the process's own softirq time lands in
# others_cpu_s instead. The two clocks count differently (ticks of 1/CLK_TCK s
# against the scheduler's own time), so others_cpu_s can be a few ticks below
# zero on a quiet machine.
#
# --quiet-below CPUS (run_all.sh and controls/run_controls.sh) is for a machine
# shared with other work, such as machine 1 of the Phase 1 report, a development
# VM: before every process the script waits until the other processes have used
# fewer than CPUS CPUs over LOAD_WINDOW (2) seconds, for at most LOAD_MAX_WAIT
# (900) seconds, and it measures a process again, up to LOAD_ATTEMPTS (3)
# attempts in all, when the others used CPUS or more on average while it ran
# (and at least 0.05 CPU seconds in all, five ticks, so that a process of a few
# milliseconds is not measured again for one stray tick). The last attempt is
# kept whatever its load. A discarded attempt keeps its load.csv row (kept=no)
# and loses its CSV rows. The default, off, measures every process once, as on a
# dedicated runner, and still writes load.csv.

LOAD_HZ=$(getconf CLK_TCK 2>/dev/null || echo 100)
LOAD_WINDOW=${LOAD_WINDOW:-2}
LOAD_MAX_WAIT=${LOAD_MAX_WAIT:-900}
LOAD_ATTEMPTS=${LOAD_ATTEMPTS:-3}
LOAD_QUIET=${LOAD_QUIET:-off}
LOAD_TMP="$OUT/.attempt-$$.csv"
LOAD_HEADER="start_utc,benchmark,implementation,args,attempt,kept,wall_s,own_cpu_s,others_cpu_s,others_cpus,steal_s,quiet_below,gate,gate_wait_s"

# load_ticks: the busy ticks (user, nice, system, irq, softirq) and the steal
# ticks of all CPUs.
load_ticks() { awk '/^cpu /{print $2 + $3 + $4 + $7 + $8, $9; exit}' /proc/stat; }

# load_own VAR: sets VAR to the CPU seconds of this shell and of its children
# that have ended. `times` must run in this shell, not in a $(...) subshell,
# whose own and children's times start at zero.
load_own() {
  local t="$OUT/.times-$$"
  times > "$t"
  read -r "$1" < <(awk '{for (i = 1; i <= NF; i++) {split($i, a, "m"); sub("s", "", a[2]); s += a[1] * 60 + a[2]}}
                        END {printf "%.3f\n", s}' "$t")
  rm -f "$t"
}

# load_gate: waits for the other processes to use fewer than LOAD_QUIET CPUs over
# LOAD_WINDOW seconds; sets LOAD_GATE (quiet, timeout or off) and LOAD_WAITED.
load_gate() {
  local t0=$SECONDS b0 b1 x
  LOAD_GATE=off; LOAD_WAITED=0
  [ "$LOAD_QUIET" = off ] && return 0
  while :; do
    read -r b0 x < <(load_ticks)
    sleep "$LOAD_WINDOW"
    read -r b1 x < <(load_ticks)
    if awk -v d=$((b1 - b0)) -v hz="$LOAD_HZ" -v w="$LOAD_WINDOW" -v q="$LOAD_QUIET" \
        'BEGIN {exit !(d / hz / w < q)}'; then
      LOAD_GATE=quiet; break
    fi
    if [ $((SECONDS - t0)) -ge "$LOAD_MAX_WAIT" ]; then LOAD_GATE=timeout; break; fi
  done
  LOAD_WAITED=$((SECONDS - t0))
}

# load_run IMPL BENCH DESC CMD...: runs CMD, which must write its CSV rows to
# $LOAD_TMP, behind the gate; records it in load.csv; measures it again while
# the machine was busy (see above); appends the kept attempt's rows to
# OUT/BENCH.csv and returns CMD's exit status.
load_run() {
  local impl=$1 bench=$2 desc=$3; shift 3
  local attempt=1 rc b0 s0 o0 t0 start b1 s1 o1 t1 verdict
  local f="$OUT/load.csv"
  [ -s "$f" ] || echo "$LOAD_HEADER" > "$f"
  while :; do
    rm -f "$LOAD_TMP"
    load_gate
    load_own o0
    read -r b0 s0 < <(load_ticks)
    start=$(date -u +%Y-%m-%dT%H:%M:%SZ)
    t0=$(date +%s.%N)
    "$@"
    rc=$?
    t1=$(date +%s.%N)
    read -r b1 s1 < <(load_ticks)
    load_own o1
    # verdict: "busy" when the others used LOAD_QUIET CPUs or more on average
    verdict=$(awk -v b0="$b0" -v b1="$b1" -v s0="$s0" -v s1="$s1" -v o0="$o0" -v o1="$o1" \
        -v t0="$t0" -v t1="$t1" -v hz="$LOAD_HZ" -v q="$LOAD_QUIET" -v n="$attempt" -v max="$LOAD_ATTEMPTS" \
        -v start="$start" -v bench="$bench" -v impl="$impl" -v desc="$desc" -v gate="$LOAD_GATE" -v waited="$LOAD_WAITED" \
        -v f="$f" 'BEGIN {
          w = t1 - t0; own = o1 - o0; others = (b1 - b0) / hz - own
          cpus = w > 0.05 ? others / w : 0
          busy = (q != "off" && others >= 0.05 && cpus >= q + 0)
          kept = (busy && n < max) ? "no" : "yes"
          gsub(/"/, "\"\"", desc)
          printf "%s,%s,%s,\"%s\",%d,%s,%.2f,%.2f,%.2f,%.3f,%.2f,%s,%s,%d\n", start, bench, impl, desc, n, kept, w, own, others, cpus, (s1 - s0) / hz, q, gate, waited >> f
          print (kept == "no") ? "again" : "keep"
        }')
    [ "$verdict" = keep ] && break
    echo "   -> other processes were busy while it ran; measuring again (attempt $((attempt + 1)) of $LOAD_ATTEMPTS)" >&2
    attempt=$((attempt + 1))
  done
  if [ -s "$LOAD_TMP" ]; then
    if [ -s "$OUT/$bench.csv" ]; then tail -n +2 "$LOAD_TMP" >> "$OUT/$bench.csv"; else cat "$LOAD_TMP" > "$OUT/$bench.csv"; fi
  fi
  rm -f "$LOAD_TMP"
  return $rc
}
