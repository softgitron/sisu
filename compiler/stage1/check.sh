#!/usr/bin/env bash
# Checks the stage-1 lexer and parser (this package, run on the stage-0
# interpreter) against the stage-0 Rust front end, byte for byte:
#
#   tokens       `sisu tokens --trivia FILE`, standard output and error
#   tree         `sisu parse --tree FILE`, standard output and error
#   trivia-tree  `sisu parse --tree --trivia FILE`, standard output
#   print        the source reprinted from stage 1's tree, against the file
#
# for every `.sisu` file under compiler/conformance/cases/, compiler/std/
# and proposal/examples/, and stage 1's own cases (compiler/stage1/cases/).
# Then stage 1's name resolution against stage 0's:
#
#   names        `sisu check --format=ids FILE`, its name diagnostics only
#                (E0200, E0201, E0202, E0220, E0234, E0389, W0002)
#
# for the conformance suite's names/ cases and stage 1's own cases, and
# with --names for every program of the corpus that is one module (the
# conformance cases outside packages/, and the proposal's examples).
# A difference listed in DISCREPANCIES.md ("- Differs: `path`: outputs") is
# expected; any other fails the check.
#
# Usage: compiler/stage1/check.sh [--mutants N] [--seed S] [--names] [--keep DIR]
#
#   --mutants N  also compare N mutants of the corpus (deleted, inserted and
#                swapped characters, from stage 1's `mutate` with seed S);
#                a mutant may differ only in the way a listed discrepancy
#                does (its signature is checked below)
#   --names      resolve the names of the whole corpus, not only names/
#   --keep DIR   leave every output in DIR instead of a temporary directory
set -uo pipefail

here=$(cd "$(dirname "$0")" && pwd)
compiler=$(dirname "$here")
repo=$(dirname "$compiler")
sisu="$compiler/target/release/sisu"
mutants=0
seed=1
keep=""
all_names=0
while [ $# -gt 0 ]; do
    case "$1" in
        --mutants) mutants="$2"; shift 2 ;;
        --seed) seed="$2"; shift 2 ;;
        --keep) keep="$2"; shift 2 ;;
        --names) all_names=1; shift ;;
        *) echo "usage: $0 [--mutants N] [--seed S] [--names] [--keep DIR]" >&2; exit 2 ;;
    esac
done

echo "building stage 0 (cargo build --release)"
if ! (cd "$compiler" && cargo build --release --quiet); then
    echo "check.sh: stage 0 does not build" >&2
    exit 1
fi

if [ -n "$keep" ]; then
    work="$keep"
    rm -rf "$work"
    mkdir -p "$work"
else
    work=$(mktemp -d)
    trap 'rm -rf "$work"' EXIT
fi

# The differences DISCREPANCIES.md lists, one "path output" per line.
allowed="$work/allowed.txt"
sed -n 's/^- Differs: `\([^`]*\)`: \(.*\)$/\1 \2/p' "$here/DISCREPANCIES.md" |
    while read -r path outputs; do
        for o in $(echo "$outputs" | tr ',' ' '); do echo "$path $o"; done
    done > "$allowed"

# The file classes, as lists of absolute paths.
find "$compiler/conformance/cases" -name '*.sisu' | sort > "$work/conformance.list"
find "$compiler/std" -name '*.sisu' | sort > "$work/std.list"
find "$repo/proposal/examples" -name '*.sisu' | sort > "$work/examples.list"
find "$here/cases" -name '*.sisu' | sort > "$work/cases.list"
classes="conformance std examples cases"
if [ "$mutants" -gt 0 ]; then
    mkdir -p "$work/mutants.src"
    cat "$work/conformance.list" "$work/std.list" "$work/examples.list" > "$work/corpus.list"
    if ! (cd "$here" && "$sisu" run -- mutate "$seed" "$mutants" "$work/mutants.src" "$work/corpus.list" \
            > "$work/mutate.log" 2>&1); then
        cat "$work/mutate.log" >&2
        echo "check.sh: stage 1 could not write the mutants" >&2
        exit 1
    fi
    for k in $(seq 1 "$mutants"); do echo "$work/mutants.src/m$k.sisu"; done > "$work/mutants.list"
    classes="$classes mutants"
fi

# A mutant's difference is a listed discrepancy's when it has its mark:
# D1, a line break that stage 0 made an ERROR_TOKEN; D2, a `\u{+` escape;
# D3, trees and tokens alike and only E0126 diagnostics differing.
explained() {
    local src="$1" ref="$2" out="$3"
    if grep -Eq '^ERROR_TOKEN [0-9]+\.\.[0-9]+ "(\\r\\n|\\n|\\r)"$' "$ref.tokens"; then return 0; fi
    if grep -Fq '\u{+' "$src"; then return 0; fi
    if cmp -s "$ref.tokens" "$out.tokens" && cmp -s "$ref.tree" "$out.tree" &&
        ! diff "$ref.tree.err" "$out.tree.err" | grep -E '^[<>]' | grep -vq 'E0126'; then
        return 0
    fi
    return 1
}

total=0; identical=0; listed=0; unlisted=0
report=""
summary=""
for class in $classes; do
    list="$work/$class.list"
    n=$(wc -l < "$list" | tr -d ' ')
    ref="$work/$class.stage0"; out="$work/$class.stage1"
    mkdir -p "$ref" "$out"
    # Stage 0's outputs.
    i=0
    while read -r f; do
        i=$((i + 1))
        "$sisu" tokens --trivia "$f" > "$ref/$i.tokens" 2> "$ref/$i.tokens.err"
        "$sisu" parse --tree "$f" > "$ref/$i.tree" 2> "$ref/$i.tree.err"
        "$sisu" parse --tree --trivia "$f" > "$ref/$i.trivia.tree" 2> /dev/null
    done < "$list"
    # Stage 1's, in one run of the interpreter.
    start=$(date +%s.%N)
    if ! (cd "$here" && "$sisu" run -- batch "$out" "$list" > "$work/$class.log" 2> "$work/$class.stderr"); then
        cat "$work/$class.stderr" >&2
        echo "check.sh: stage 1 failed on the $class files" >&2
        exit 1
    fi
    end=$(date +%s.%N)
    seconds=$(awk "BEGIN { printf \"%.1f\", $end - $start }")
    phases=$(sed -n 's/^files [0-9]* bytes \([0-9]*\) lex \([0-9]*\) ms parse \([0-9]*\) ms dump \([0-9]*\) ms$/\1 bytes; lex \2 ms, parse \3 ms, dumps \4 ms/p' "$work/$class.log")
    # Compare.
    same=0; class_listed=0; class_unlisted=0
    i=0
    while read -r f; do
        i=$((i + 1))
        differs=""
        { cmp -s "$ref/$i.tokens" "$out/$i.tokens" && cmp -s "$ref/$i.tokens.err" "$out/$i.tokens.err"; } ||
            differs="$differs tokens"
        { cmp -s "$ref/$i.tree" "$out/$i.tree" && cmp -s "$ref/$i.tree.err" "$out/$i.tree.err"; } ||
            differs="$differs tree"
        cmp -s "$ref/$i.trivia.tree" "$out/$i.trivia.tree" || differs="$differs trivia-tree"
        cmp -s "$f" "$out/$i.print" || differs="$differs print"
        if [ -z "$differs" ]; then
            same=$((same + 1))
            continue
        fi
        rel="${f#"$repo"/}"
        expected=1
        if [ "$class" = mutants ]; then
            explained "$f" "$ref/$i" "$out/$i" || expected=0
            # The listed discrepancies never touch the reprint.
            case "$differs" in *print*) expected=0 ;; esac
        else
            for o in $differs; do
                grep -qxF "$rel $o" "$allowed" || expected=0
            done
        fi
        if [ "$expected" = 1 ]; then
            class_listed=$((class_listed + 1))
            report="$report\n  listed:   $rel:$differs"
        else
            class_unlisted=$((class_unlisted + 1))
            report="$report\n  UNLISTED: $rel:$differs (stage 0 in $ref/$i.*, stage 1 in $out/$i.*)"
        fi
    done < "$list"
    total=$((total + n)); identical=$((identical + same))
    listed=$((listed + class_listed)); unlisted=$((unlisted + class_unlisted))
    summary="$summary\n$(printf '  %-12s %4d files, %4d identical, %3d differing as listed, %3d differing otherwise; stage 1 %6ss (%s)' \
        "$class" "$n" "$same" "$class_listed" "$class_unlisted" "$seconds" "$phases")"
done

# Name resolution, one module at a time.
find "$compiler/conformance/cases/names" -name '*.sisu' | sort > "$work/names.list"
cat "$work/cases.list" >> "$work/names.list"
name_classes="names"
if [ "$all_names" = 1 ]; then
    find "$compiler/conformance/cases" -name '*.sisu' | grep -v '/packages/' | grep -v '/names/' | sort \
        > "$work/corpus-names.list"
    cat "$work/examples.list" >> "$work/corpus-names.list"
    name_classes="names corpus-names"
fi
names_summary=""
for class in $name_classes; do
    list="$work/$class.list"
    n=$(wc -l < "$list" | tr -d ' ')
    ref="$work/$class.stage0"; out="$work/$class.stage1"
    mkdir -p "$ref" "$out"
    i=0
    while read -r f; do
        i=$((i + 1))
        (cd "$(dirname "$f")" && "$sisu" check --format=ids "$(basename "$f")" 2> /dev/null) |
            awk '/^  / { if (keep) print; next }
                 { keep = ($2 == "E0200" || $2 == "E0201" || $2 == "E0202" || $2 == "E0220" ||
                           $2 == "E0234" || $2 == "E0389" || $2 == "W0002"); if (keep) print }' \
            > "$ref/$i.names"
    done < "$list"
    start=$(date +%s.%N)
    if ! (cd "$here" && "$sisu" run -- names-batch "$out" "$list" > "$work/$class.log" 2> "$work/$class.stderr"); then
        cat "$work/$class.stderr" >&2
        echo "check.sh: stage 1 failed resolving the $class files" >&2
        exit 1
    fi
    end=$(date +%s.%N)
    seconds=$(awk "BEGIN { printf \"%.1f\", $end - $start }")
    same=0; class_listed=0; class_unlisted=0
    i=0
    while read -r f; do
        i=$((i + 1))
        if cmp -s "$ref/$i.names" "$out/$i.names"; then
            same=$((same + 1))
            continue
        fi
        rel="${f#"$repo"/}"
        if grep -qxF "$rel names" "$allowed"; then
            class_listed=$((class_listed + 1))
            report="$report\n  listed:   $rel: names"
        else
            class_unlisted=$((class_unlisted + 1))
            report="$report\n  UNLISTED: $rel: names (stage 0 in $ref/$i.names, stage 1 in $out/$i.names)"
        fi
    done < "$list"
    total=$((total + n)); identical=$((identical + same))
    listed=$((listed + class_listed)); unlisted=$((unlisted + class_unlisted))
    names_summary="$names_summary\n$(printf '  %-12s %4d files, %4d identical, %3d differing as listed, %3d differing otherwise; stage 1 %6ss' \
        "$class" "$n" "$same" "$class_listed" "$class_unlisted" "$seconds")"
done

echo "stage 1 against stage 0 (tokens, tree, trivia-tree, print):"
printf "%b\n" "$summary"
echo "stage 1's name resolution against stage 0's (names):"
printf "%b\n" "$names_summary"
[ -n "$report" ] && printf "differences:%b\n" "$report"
echo "total: $total files checked, $identical identical, $((listed + unlisted)) differing ($listed listed in DISCREPANCIES.md, $unlisted not)"
if [ "$unlisted" -gt 0 ]; then
    [ -z "$keep" ] && echo "(run with --keep DIR to inspect the outputs)"
    exit 1
fi
exit 0
