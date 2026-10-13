# Scale parameters shared by run_all.sh and controls/run_controls.sh, so that the
# harness and the Go and Java controls always run the same workloads.
#
# Usage (from bash): . scale.sh; scale_params smoke|full THREADS
#
# "full" follows the harness protocol of proposal/20, section 6.5, where the
# harness can (deviations are listed in docs/report-phase1.md, section 10):
# five repetitions in fresh processes; spawn-join 100k pairs after 10k warm-up
# pairs; idle memory at 100k and 1M tasks read one second after parking; 1M
# tasks held parked for 60 seconds; four compute tasks per worker in the
# starvation test with yield points every 10k and 100k iterations; 1M
# cancelled io_uring reads in batches of at most 10k (8192) in flight.
#
# Connection counts follow the RLIMIT_NOFILE hard limit, which both ends of an
# in-process connection count against (2N + 64 descriptors): the 10k-connection
# echo and the idle-connection count run only as far as the default hard limit
# allows (the owner's policy 3); the binary records a failed row otherwise.

# nofile_hard: the hard RLIMIT_NOFILE (a large number when unlimited).
nofile_hard() {
  local h
  h=$(ulimit -Hn)
  if [ "$h" = unlimited ]; then echo 1048576; else echo "$h"; fi
}

# fits_connections N: whether N in-process connections fit the hard limit.
fits_connections() {
  [ $((2 * $1 + 64)) -le "$(nofile_hard)" ]
}

scale_params() {
  local scale=$1 threads=$2
  ST_TASKS=$((4 * threads))
  if [ "$scale" = full ]; then
    REPS_DEFAULT=5
    SJ_ROUNDS=100000; SJ_WARMUP=10000
    IM_SIZES="100000 1000000"; IM_SETTLE_MS=1000; IM_SKIP_AT=1000000; OS_IM=10000
    MT_CAP=1000000; MT_GUARD=512M; MT_HOLD=60; OS_MT=30000
    ECHO_CONNS="10000 1000"; ECHO_DURATION=30; ECHO_WARMUP=5
    # The largest idle-connection count up to 100k that the hard limit allows,
    # rounded down to a multiple of 100.
    local most=$((($(nofile_hard) - 64) / 2 / 100 * 100))
    IC_CONNS=$((most < 100000 ? most : 100000)); IC_SETTLE_MS=30000
    ST_KS="10000 100000"; ST_DURATION=11
    PO_REPS=31
    CU_TASKS=1000000; CU_CONC=8192; CU_FILE_TASKS=1000000
  else
    REPS_DEFAULT=1
    SJ_ROUNDS=20000; SJ_WARMUP=2000
    IM_SIZES="10000"; IM_SETTLE_MS=200; IM_SKIP_AT=0; OS_IM=2000
    MT_CAP=100000; MT_GUARD=1G; MT_HOLD=0; OS_MT=5000
    ECHO_CONNS="1000"; ECHO_DURATION=3; ECHO_WARMUP=1
    IC_CONNS=5000; IC_SETTLE_MS=200
    ST_KS="10000"; ST_DURATION=2
    PO_REPS=11
    CU_TASKS=4096; CU_CONC=512; CU_FILE_TASKS=4096
  fi
}

# skip_excursion_at N: the flag for strategies that keep their pages at N tasks
# (the owner's policy 7: the 1M-after-excursion cell is required only of
# strategies that release pages on park, and cannot fit 16 GiB for the others).
skip_excursion_at() {
  if [ "$IM_SKIP_AT" -gt 0 ] && [ "$1" -ge "$IM_SKIP_AT" ]; then echo --skip-excursion; fi
}
