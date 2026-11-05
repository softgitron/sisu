#!/usr/bin/env bash
# The bootstrap's fixed point (proposal/18-next-steps.md, "Bootstrapping",
# D-078), on its own; `check.sh --self` runs it as its class `fixed`:
#
#   stage1-A   stage 1 as stage 0 builds it (`sisu build --native`: the
#              Rust front end, then the native backend)
#   A.sir      the SIR stage1-A prints of stage 1's own package (`sir`, run
#              in this directory, as `sisu sir` is)
#   stage1-B   A.sir built by the backend alone (`sisu build --native
#              --from-sir A.sir`): stage 1 compiled by itself as its front
#              end
#   B.sir      the SIR stage1-B prints of the package
#
# The fixed point holds when B.sir is A.sir byte for byte; the backend is
# deterministic, so stage1-B must then be stage1-A byte for byte too
# (D-078). It also reports how A.sir compares with what stage 0 prints of
# the package (`sisu sir`), which does not decide the fixed point: the Rust
# front end is an oracle that falls behind (decision 268), so the two
# differ where a rule of RULES.md changes stage 1's lowering (R124, the
# shared reborrow of a `mut` parameter), and `check.sh`'s class `self`
# lists the difference under its rule as any other. It prints the size and
# SHA-256 of each file, and exits non-zero when A.sir and B.sir, or
# stage1-A and stage1-B, differ.
#
# Usage: compiler/stage1/bootstrap.sh [--keep DIR]
#
#   --keep DIR   leave the executables and the SIR files in DIR instead of
#                a temporary directory
set -uo pipefail

# Stage 0 builds stage1-A and prints its SIR with its own front end, the
# Rust one, unless `SISU_FRONT_END` names another (decision 268).
export SISU_FRONT_END="${SISU_FRONT_END:-rust}"

here=$(cd "$(dirname "$0")" && pwd)
compiler=$(dirname "$here")
sisu="$compiler/target/release/sisu"
keep=""
while [ $# -gt 0 ]; do
    case "$1" in
        --keep) keep="$2"; shift 2 ;;
        *) echo "usage: $0 [--keep DIR]" >&2; exit 2 ;;
    esac
done

echo "building stage 0 (cargo build --release)"
if ! (cd "$compiler" && cargo build --release --quiet); then
    echo "bootstrap.sh: stage 0 does not build" >&2
    exit 1
fi
if [ -n "$keep" ]; then
    work="$keep"
    rm -rf "$work"
    mkdir -p "$work"
    work=$(cd "$work" && pwd)
else
    work=$(mktemp -d)
    trap 'rm -rf "$work"' EXIT
fi

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

t0=$(date +%s.%N)
echo "stage1-A: stage 0 builds stage 1 (sisu build --native)"
step "$work/A.log" bash -c 'cd "$1" && "$2" build --native --output "$3"' _ "$here" "$sisu" "$work/stage1-A"
t1=$(date +%s.%N)
echo "A.sir: stage1-A prints the SIR of its own package (sir)"
if ! (cd "$here" && "$work/stage1-A" sir > "$work/A.sir" 2> "$work/A.err"); then
    cat "$work/A.err" >&2
    echo "bootstrap.sh: stage1-A failed printing the SIR" >&2
    exit 1
fi
t2=$(date +%s.%N)
echo "stage1-B: the backend builds A.sir (sisu build --native --from-sir)"
# In this directory, as stage1-A is built: the debug information records
# the directory a build runs in (decision 227).
step "$work/B.log" bash -c 'cd "$1" && "$2" build --native --from-sir "$3" --output "$4"' _ \
    "$here" "$sisu" "$work/A.sir" "$work/stage1-B"
t3=$(date +%s.%N)
echo "B.sir: stage1-B prints the SIR of the package (sir)"
if ! (cd "$here" && "$work/stage1-B" sir > "$work/B.sir" 2> "$work/B.err"); then
    cat "$work/B.err" >&2
    echo "bootstrap.sh: stage1-B failed printing the SIR" >&2
    exit 1
fi
t4=$(date +%s.%N)
(cd "$here" && "$sisu" sir 2> /dev/null) > "$work/stage0.sir"

describe() {
    # describe NAME FILE WHAT
    printf '  %-9s %9d bytes  sha256 %s  %s\n' "$1" "$(wc -c < "$2" | tr -d ' ')" \
        "$(sha256sum "$2" | cut -d ' ' -f 1)" "$3"
}
echo
describe stage1-A "$work/stage1-A" "built by stage 0 ($(seconds "$t0" "$t1") s)"
describe A.sir "$work/A.sir" "printed by stage1-A ($(seconds "$t1" "$t2") s)"
describe stage1-B "$work/stage1-B" "built by the backend from A.sir ($(seconds "$t2" "$t3") s)"
describe B.sir "$work/B.sir" "printed by stage1-B ($(seconds "$t3" "$t4") s)"
describe stage0 "$work/stage0.sir" "printed by stage 0 (sisu sir)"
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
verdict "the SIR, A.sir and B.sir" "$work/A.sir" "$work/B.sir"
verdict "the executables, stage1-A and stage1-B" "$work/stage1-A" "$work/stage1-B"
# A report, not a verdict (RULES.md, R124): stage 0's front end is frozen.
if same "$work/A.sir" "$work/stage0.sir"; then
    echo "stage 1's SIR and stage 0's, A.sir and sisu sir: identical"
else
    echo "stage 1's SIR and stage 0's, A.sir and sisu sir: differs from stage 0, as the rules list"
fi
if [ "$failed" = 0 ]; then
    echo "the fixed point holds"
else
    [ -z "$keep" ] && echo "(run with --keep DIR to inspect the files)"
    echo "the fixed point does not hold"
fi
exit "$failed"
