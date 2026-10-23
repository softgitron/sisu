#!/usr/bin/env bash
# Compiles raytrace.sisu to a native executable and renders the scene.
#
# Usage: ./raytrace.sh [WIDTH] [SAMPLES] [OUTPUT]
#   defaults: 640 pixels wide (16:9), 16 samples per pixel, render.bmp
# Set SISU to the `sisu` binary to use; by default it is the one built in
# ../../compiler/target/release.

set -eu
here="$(cd "$(dirname "$0")" && pwd)"
compiler="$here/../../compiler"
sisu="${SISU:-$compiler/target/release/sisu}"

if [ ! -x "$sisu" ]; then
    echo "raytrace: building the Sisu toolchain (once; needs cargo)..." >&2
    (cd "$compiler" && cargo build --release -p sisu-driver)
fi

mkdir -p "$here/build"
"$sisu" build --native --release --output "$here/build/raytrace" "$here/raytrace.sisu"
"$here/build/raytrace" "$@"
