#!/usr/bin/env bash
# Runs tetris.sisu in the terminal.
#
# Sisu's standard input gives whole lines, so this script puts the
# terminal into character mode and feeds the game one line per key (the
# key byte's decimal code), plus a `t` heartbeat about every 30 ms so
# pieces keep falling while no key is pressed.
#
# Usage: ./tetris.sh [--native]
#   --native  compile the game to an executable first (faster), instead of
#             running it in the interpreter
# Set SISU to the `sisu` binary to use; by default it is the one built in
# ../../compiler/target/release.

set -u
here="$(cd "$(dirname "$0")" && pwd)"
compiler="$here/../../compiler"
sisu="${SISU:-$compiler/target/release/sisu}"

if [ ! -t 0 ] || [ ! -t 1 ]; then
    echo "tetris: run this in an interactive terminal" >&2
    exit 1
fi

if [ ! -x "$sisu" ]; then
    echo "tetris: building the Sisu toolchain (once; needs cargo)..." >&2
    (cd "$compiler" && cargo build --release -p sisu-driver) || exit 1
fi

run=("$sisu" run "$here/tetris.sisu")
if [ "${1:-}" = "--native" ]; then
    exe="$here/build/tetris"
    mkdir -p "$here/build"
    "$sisu" build --native --release --output "$exe" "$here/tetris.sisu" || exit 1
    run=("$exe")
else
    "$sisu" check "$here/tetris.sisu" || exit 1
fi

saved="$(stty -g)"
restore() {
    stty "$saved"
    printf '\033[0m\033[?25h\n'
}
trap restore EXIT
trap 'exit 130' INT TERM

# No line editing and no echo; Ctrl-C still interrupts.
stty -icanon -echo min 1 time 0

feed() {
    local k
    while true; do
        if IFS= read -r -s -n 1 -d '' -t 0.03 k; then
            printf '%d\n' "'$k" || return
        else
            printf 't\n' || return
        fi
    done
}

export LC_ALL=C
feed 2>/dev/null | "${run[@]}"
