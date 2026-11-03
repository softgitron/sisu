#!/usr/bin/env bash
# The Sisu backend's fixed point (D-087, the done-criterion of step 2;
# decision 330), on its own:
#
#   sisu2-C    stage 2 as the Cranelift path builds it: through stage 1, the
#              front end of `sisu`, with the Rust lowering and Cranelift
#              (`sisu build --native --lower=rust --backend=cranelift` in
#              this directory), as the driver builds target/stage2/sisu2
#   A.stage2.sir, A.stage1.sir
#              the SIR of stage 2's package and of stage 1's, as `sisu sir`
#              prints them in their directories (stage 1 as `sisu` runs it)
#   sisu2-A, stage1-A
#              that SIR lowered by sisu2-C and compiled by its backend, the
#              one written in Sisu (`sisu build --native --backend=sisu
#              --from-sir`, with `SISU_STAGE2` naming sisu2-C)
#   B.stage2.sir, B.stage1.sir
#              the SIR stage1-A, stage 1 compiled by the Sisu backend,
#              prints of the two packages
#   sisu2-B, stage1-B
#              that SIR lowered and compiled by sisu2-A, stage 2 compiled
#              by itself
#
# The fixed point holds when each B is its A byte for byte: stage 2 built
# by the Sisu backend lowers and compiles itself and stage 1 to the bytes
# the Cranelift build of it gives, so building again changes nothing, and
# stage 1 built by it prints the SIR that stage 1 built by Cranelift
# prints. The backend is deterministic (no addresses, no hash order, no
# time), so any difference is a miscompilation. The script prints the size
# and SHA-256 of each file, and exits non-zero when a pair differs.
#
# Usage: compiler/stage2/bootstrap.sh [--keep DIR] [--sisu PATH]
#
#   --keep DIR   leave the executables and the SIR files in DIR instead of
#                a temporary directory
#   --sisu PATH  the `sisu` that drives the builds (default: the release
#                build in compiler/target, built first)
set -uo pipefail

here=$(cd "$(dirname "$0")" && pwd)
compiler=$(dirname "$here")
stage1="$compiler/stage1"
sisu=""
keep=""
while [ $# -gt 0 ]; do
    case "$1" in
        --keep) keep="$2"; shift 2 ;;
        --sisu) sisu="$2"; shift 2 ;;
        *) echo "usage: $0 [--keep DIR] [--sisu PATH]" >&2; exit 2 ;;
    esac
done
if [ -z "$sisu" ]; then
    sisu="$compiler/target/release/sisu"
    echo "building stage 0 (cargo build --release)"
    if ! (cd "$compiler" && cargo build --release --quiet); then
        echo "bootstrap.sh: stage 0 does not build" >&2
        exit 1
    fi
fi
sisu=$(cd "$(dirname "$sisu")" && pwd)/$(basename "$sisu")
if [ -n "$keep" ]; then
    work="$keep"
    rm -rf "$work"
    mkdir -p "$work"
    work=$(cd "$work" && pwd)
else
    work=$(mktemp -d)
    trap 'rm -rf "$work"' EXIT
fi
# The choices are the script's: no backend, lowering or stage 2 from the
# caller's environment.
unset SISU_BACKEND SISU_LOWER SISU_STAGE2 SISU_FRONT_END

seconds() { awk "BEGIN { printf \"%.1f\", $2 - $1 }"; }
step() {
    # step LOG COMMAND...: runs COMMAND, its output into LOG, and stops on
    # a failure.
    local log="$1"
    shift
    if ! "$@" > "$log" 2>&1; then
        cat "$log" >&2
        echo "bootstrap.sh: failed: $*" >&2
        exit 1
    fi
}
sir_of() {
    # sir_of FRONT DIR OUT: the SIR the front end FRONT (`sisu`, or a
    # stage-1 executable) prints of the package in DIR.
    if ! (cd "$2" && "$1" sir > "$3" 2> "$3.err"); then
        cat "$3.err" >&2
        echo "bootstrap.sh: $1 failed printing the SIR of $2" >&2
        exit 1
    fi
    rm -f "$3.err"
}
build_with() {
    # build_with STAGE2 DIR SIR OUT: the SIR built in DIR (where the debug
    # information says the sources are) by the Sisu backend, with STAGE2
    # lowering and compiling it.
    step "$4.log" env SISU_STAGE2="$1" bash -c 'cd "$1" && "$2" build --native --backend=sisu --from-sir "$3" --output "$4"' \
        _ "$2" "$sisu" "$3" "$4"
}

t0=$(date +%s.%N)
echo "sisu2-C: stage 2 built through the Cranelift path"
step "$work/sisu2-C.log" bash -c 'cd "$1" && "$2" build --native --lower=rust --backend=cranelift --output "$3"' \
    _ "$here" "$sisu" "$work/sisu2-C"
t1=$(date +%s.%N)
echo "A.stage2.sir, A.stage1.sir: the packages' SIR (sisu sir)"
sir_of "$sisu" "$here" "$work/A.stage2.sir"
sir_of "$sisu" "$stage1" "$work/A.stage1.sir"
t2=$(date +%s.%N)
echo "sisu2-A, stage1-A: lowered and compiled by sisu2-C"
build_with "$work/sisu2-C" "$here" "$work/A.stage2.sir" "$work/sisu2-A"
t3=$(date +%s.%N)
build_with "$work/sisu2-C" "$stage1" "$work/A.stage1.sir" "$work/stage1-A"
t4=$(date +%s.%N)
echo "B.stage2.sir, B.stage1.sir: printed by stage1-A"
sir_of "$work/stage1-A" "$here" "$work/B.stage2.sir"
sir_of "$work/stage1-A" "$stage1" "$work/B.stage1.sir"
t5=$(date +%s.%N)
echo "sisu2-B, stage1-B: lowered and compiled by sisu2-A"
build_with "$work/sisu2-A" "$here" "$work/B.stage2.sir" "$work/sisu2-B"
t6=$(date +%s.%N)
build_with "$work/sisu2-A" "$stage1" "$work/B.stage1.sir" "$work/stage1-B"
t7=$(date +%s.%N)

describe() {
    # describe NAME FILE WHAT
    printf '  %-12s %9d bytes  sha256 %s  %s\n' "$1" "$(wc -c < "$2" | tr -d ' ')" \
        "$(sha256sum "$2" | cut -d ' ' -f 1)" "$3"
}
echo
describe sisu2-C "$work/sisu2-C" "built through the Cranelift path ($(seconds "$t0" "$t1") s)"
describe A.stage2.sir "$work/A.stage2.sir" "printed by sisu sir"
describe A.stage1.sir "$work/A.stage1.sir" "printed by sisu sir (both $(seconds "$t1" "$t2") s)"
describe sisu2-A "$work/sisu2-A" "built by sisu2-C ($(seconds "$t2" "$t3") s)"
describe stage1-A "$work/stage1-A" "built by sisu2-C ($(seconds "$t3" "$t4") s)"
describe B.stage2.sir "$work/B.stage2.sir" "printed by stage1-A"
describe B.stage1.sir "$work/B.stage1.sir" "printed by stage1-A (both $(seconds "$t4" "$t5") s)"
describe sisu2-B "$work/sisu2-B" "built by sisu2-A ($(seconds "$t5" "$t6") s)"
describe stage1-B "$work/stage1-B" "built by sisu2-A ($(seconds "$t6" "$t7") s)"
echo
same() {
    # same FILE FILE: whether both exist, are not empty and are equal.
    [ -s "$1" ] && cmp -s "$1" "$2"
}
failed=0
verdict() {
    # verdict WHAT FILE FILE
    if same "$2" "$3"; then
        echo "$1: identical"
    else
        echo "$1: DIFFERENT"
        failed=1
    fi
}
verdict "stage 2's SIR, A.stage2.sir and B.stage2.sir" "$work/A.stage2.sir" "$work/B.stage2.sir"
verdict "stage 1's SIR, A.stage1.sir and B.stage1.sir" "$work/A.stage1.sir" "$work/B.stage1.sir"
verdict "stage 2, sisu2-A and sisu2-B" "$work/sisu2-A" "$work/sisu2-B"
verdict "stage 1, stage1-A and stage1-B" "$work/stage1-A" "$work/stage1-B"
if [ "$failed" = 0 ]; then
    echo "the fixed point holds"
else
    [ -z "$keep" ] && echo "(run with --keep DIR to inspect the files)"
    echo "the fixed point does not hold"
fi
exit "$failed"
