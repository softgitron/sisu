#!/usr/bin/env bash
# Sisu vs Python, nine rounds and a race.
#
# Each round runs a small Python program with a classic bug (it runs, then
# crashes or prints a wrong answer), then asks the Sisu compiler to check
# the same program written in Sisu (it refuses, and says why). Then the
# correct Sisu program runs, and the same arithmetic is timed in both.
#
# Usage: ./demo.sh [--pause]
#   --pause  wait for Enter between rounds, for presenting
# Set SISU to the `sisu` binary to use; by default it is the one built in
# ../../compiler/target/release.

set -u
here="$(cd "$(dirname "$0")" && pwd)"
compiler="$here/../../compiler"
sisu="${SISU:-$compiler/target/release/sisu}"
pause=0
[ "${1:-}" = "--pause" ] && pause=1

if [ ! -x "$sisu" ]; then
    echo "demo: building the Sisu toolchain (once; needs cargo)..." >&2
    (cd "$compiler" && cargo build --release -p sisu-driver) || exit 1
fi
if ! command -v python3 > /dev/null; then
    echo "demo: python3 is needed for the Python side" >&2
    exit 1
fi

bold=$'\033[1m'; red=$'\033[31m'; green=$'\033[32m'; yellow=$'\033[33m'; dim=$'\033[2m'; off=$'\033[0m'

wait_key() {
    if [ "$pause" = 1 ]; then
        read -r -p "${dim}(Enter for the next round)${off}" _
    fi
}

round() {
    local n="$1" title="$2" verdict="$3"
    echo
    echo "${bold}== Round $n: $title ==${off}"
    echo "${yellow}Python${off} ${dim}(python/$n_file.py)${off}"
    # What it printed, then the last line of a traceback, if it crashed.
    local out err
    out="$(mktemp)"; err="$(mktemp)"
    (cd "$here/python" && python3 "$n_file.py" > "$out" 2> "$err")
    sed 's/^/    /' "$out"
    if [ -s "$err" ]; then
        echo "    ${red}$(tail -n 1 "$err")${off}"
    fi
    rm -f "$out" "$err"
    echo "    ${red}-> $verdict${off}"
    echo "${yellow}Sisu${off} ${dim}(sisu/$n_file.sisu), before it runs:${off}"
    (cd "$here/sisu" && "$sisu" check "$n_file.sisu" 2>&1 | sed 's/^/    /')
    echo "    ${green}-> rejected at compile time${off}"
    wait_key
}

n_file=01_typo;         round 01 "a typo in a branch that rarely runs" "crashed at the first refund, months after release"
n_file=02_none;         round 02 "a lookup that finds nothing" "crashed on the first new customer"
n_file=03_truthiness;   round 03 "0 is falsy" "mika's 0% discount silently became 10%"
n_file=04_division;     round 04 "dividing money" "fractions of a cent, and a cent lost"
n_file=05_exhaustive;   round 05 "a new status nobody handled" "printed None to the customer"
n_file=06_aliasing;     round 06 "two names for one list" "four orders and double the revenue"
n_file=07_exceptions;   round 07 "an error the signature does not mention" "the batch died half done, cards half charged"
n_file=08_types;        round 08 "a number that is really text" "a price of 2502.50 EUR, and no error"
n_file=09_capabilities; round 09 "a helper that can read your files" "a money formatter listed your home directory"

echo
echo "${bold}== The job done right, in Sisu ==${off}"
echo "${dim}orders.sisu handles every case above; the compiler checked that it does.${off}"
(cd "$here" && "$sisu" run orders.sisu 2>&1 | sed 's/^/    /')
(cd "$here" && "$sisu" test orders.sisu 2>&1 | tail -n 1 | sed 's/^/    /'; rm -f orders.test-cache)
wait_key

echo
echo "${bold}== The race: the same arithmetic, 20 million orders ==${off}"
mkdir -p "$here/build"
"$sisu" build --native --release --output "$here/build/speed" "$here/speed.sisu" > /dev/null || exit 1
t0=$(date +%s.%N)
py=$(python3 "$here/speed.py" 20000000)
t1=$(date +%s.%N)
si=$("$here/build/speed" 20000000)
t2=$(date +%s.%N)
python3 - "$t0" "$t1" "$t2" "$py" "$si" <<'EOF'
import sys
t0, t1, t2 = map(float, sys.argv[1:4])
py, si = sys.argv[4], sys.argv[5]
print(f"    Python 3:     {t1 - t0:6.2f} s   {py}")
print(f"    Sisu native:  {t2 - t1:6.2f} s   {si}")
print(f"    same answer: {py == si}, Sisu {(t1 - t0) / (t2 - t1):.0f}x faster")
EOF
