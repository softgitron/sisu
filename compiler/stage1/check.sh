#!/usr/bin/env bash
# Checks stage 1 (this package, run on the stage-0 interpreter, or with
# --native compiled by stage 0's native backend) against stage 0, byte for
# byte. First its lexer and parser against the Rust front end:
#
#   tokens       `sisu tokens --trivia FILE`, standard output and error
#   tree         `sisu parse --tree FILE`, standard output and error
#   trivia-tree  `sisu parse --tree --trivia FILE`, standard output
#   print        the source reprinted from stage 1's tree, against the file
#   ast          `sisu parse --ast FILE`, standard output: the AST of
#                `ast::build::module`, in Rust's `{:#?}` notation; a file
#                whose tree differs as listed may differ here too
#
# for every `.sisu` file under compiler/conformance/cases/, compiler/std/
# and proposal/examples/, and stage 1's own cases (compiler/stage1/cases/).
# Then stage 1's name resolution against stage 0's:
#
#   names        `sisu check --format=ids FILE`, its name diagnostics only
#                (E0200, E0201, E0202, E0220, E0234, E0389, W0002)
#
# for the conformance suite's names/ cases, stage 1's own cases (but those
# whose names its type checker reports, cases/checker_names/), and the
# programs of several modules: the conformance suite's packages (each
# directory NAME.pkg, or the one its NAME.cwd names inside it, which stage
# 0 checks from inside it) and scripts, and stage 1's own cases/packages/;
# and with --names for every program of the corpus that is one module (the
# conformance cases outside packages/ and names/, and the proposal's
# examples). Then stage 1's type checker, which covers a slice of the
# language:
#
#   types        `sisu check --format=ids FILE`, the diagnostics whose
#                identifiers begin with `type.` only (the type checker's,
#                and six of the ownership checker's)
#
# for the programs `types.list` names (a file, or the directory of a
# package or workspace), each of which must be within the slice and match;
# and with --types for every program of the corpus (the conformance cases
# outside packages/, the proposal's examples and the programs of several
# modules), where a file within the slice must match and any other may only
# report fewer of stage 0's diagnostics (stage 1 says why it is beyond).
# Then stage 1's lowering and ownership checker, which cover a slice of
# their own, both from one run of stage 1 per class:
#
#   sir          `sisu sir FILE`, standard output: the program's SIR after
#                drop placement and jump threading, or nothing when the
#                front end finds an error
#   ownership    `sisu check --format=ids FILE`, the ownership checker's
#                diagnostics only (E0333, E0366, E0367, E0392, E0393,
#                E0397, E0400 to E0410, E0416 to E0420, E0428, E0429)
#
# for the programs `sir.list` names, each of which must be within the slice
# and match; and with --sir for every program of the corpus, where a file
# within the slice must match and any other is beyond it (stage 1 says
# why). Then the SIR of a build that checks fewer contracts (07; decision
# 232's release profile), for the programs of `sir.list` that have any:
#
#   contracts-off           `sisu sir --contracts=off FILE`, standard output
#   contracts-preconditions `sisu sir --contracts=preconditions FILE`
#
# each of which must be within the slice and match. Then what `check`
# prints, every diagnostic of a program in the order `sisu check` prints
# them, from one run of stage 1 per class:
#
#   human        `sisu check FILE`, standard error: the human form, the
#                diagnostics with their source lines, labels, notes and
#                fixes, for the wrong programs of conformance/cases/wrong/,
#                each of which must be within the slice and match
#   cases-check  `sisu check --format=ids FILE`, the whole of it, and the
#   cases-human  human form, for stage 1's own cases (but its packages, which
#                are programs of the corpus), each of which must be within
#                the slice and match
#   corpus-check `sisu check --format=ids FILE`, the whole of it, and
#   corpus-human the human form, with --check for every program of the
#                corpus, where a file within the slice must match and any
#                other is beyond it (it has an error of stage 0's front end
#                that stage 1 notes without rendering it, which stage 1
#                names) and may only report fewer of stage 0's diagnostics
# A difference listed in DISCREPANCIES.md ("- Differs: `path`: outputs") is
# expected; any other fails the check.
#
# Then stage 1 over its own package, the first half of D-078's fixed point:
#
#   self         this package's name and type diagnostics, and its SIR
#                module by module: the functions each source file gives
#                (and the declarations) in what `sisu sir` prints of this
#                package, against what stage 1 makes of it (`sir
#                --partial`, the SIR of what lowering reaches also when it
#                is beyond the slice, which then names why)
#
# and the second half, the fixed point itself (`bootstrap.sh` runs it
# alone):
#
#   fixed        stage 1 built by stage 0 (`sisu build --native`, stage1-A)
#                prints the SIR of its own package (`sir`), which must be
#                what `sisu sir` prints of it, the whole file; stage 0's
#                backend alone builds that SIR into stage1-B (`sisu build
#                --native --from-sir`); stage1-B prints the SIR of the
#                package again, which must be stage1-A's; and since the
#                backend is deterministic, stage1-B must be stage1-A byte
#                for byte
#
# Usage: compiler/stage1/check.sh [--native] [--self] [--mutants N] [--seed S] [--names] [--types] [--sir] [--check] [--keep DIR]
#
#   --native     build stage 1 once with stage 0's native backend (`sisu
#                build --native`) and run that executable for every batch,
#                instead of `sisu run --` on the interpreter; the outputs
#                are the same, in minutes instead of an hour; it implies
#                --self and --check
#   --self       check stage 1 over its own package and the fixed point
#                (the classes `self` and `fixed`; the second builds stage 1
#                natively also without --native)
#   --mutants N  also compare N mutants of the corpus (deleted, inserted and
#                swapped characters, from stage 1's `mutate` with seed S);
#                a mutant may differ only in the way a listed discrepancy
#                does (its mark is checked below)
#   --names      resolve the names of the whole corpus, not only names/
#   --types      check the types of the whole corpus, not only types.list
#   --sir        lower the whole corpus, not only sir.list
#   --check      check the whole corpus as `sisu check` does, not only
#                conformance/cases/wrong/ (--native implies it)
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
all_types=0
all_sir=0
all_check=0
native=0
self_check=0
while [ $# -gt 0 ]; do
    case "$1" in
        --mutants) mutants="$2"; shift 2 ;;
        --seed) seed="$2"; shift 2 ;;
        --keep) keep="$2"; shift 2 ;;
        --names) all_names=1; shift ;;
        --types) all_types=1; shift ;;
        --sir) all_sir=1; shift ;;
        --check) all_check=1; shift ;;
        --native) native=1; self_check=1; all_check=1; shift ;;
        --self) self_check=1; shift ;;
        *) echo "usage: $0 [--native] [--self] [--mutants N] [--seed S] [--names] [--types] [--sir] [--check] [--keep DIR]" >&2; exit 2 ;;
    esac
done

echo "building stage 0 (cargo build --release)"
if ! (cd "$compiler" && cargo build --release --quiet); then
    echo "check.sh: stage 0 does not build" >&2
    exit 1
fi

# Stage 0 writes a package's `sisu.lock` beside its manifest when it checks
# it (09); the run removes the ones it wrote, and leaves the cases as they
# were.
lock_dirs=("$compiler/conformance/cases/packages" "$here/cases/packages")
locks_before=$(find "${lock_dirs[@]}" -name sisu.lock 2> /dev/null | sort)
forget_locks() {
    find "${lock_dirs[@]}" -name sisu.lock 2> /dev/null | while read -r f; do
        grep -qxF "$f" <<< "$locks_before" || rm -f "$f"
    done
}

if [ -n "$keep" ]; then
    work="$keep"
    rm -rf "$work"
    mkdir -p "$work"
    work=$(cd "$work" && pwd)
    trap forget_locks EXIT
else
    work=$(mktemp -d)
    trap 'forget_locks; rm -rf "$work"' EXIT
fi

# Stage 1 itself, run from its directory (it reads the catalog and the
# standard library relative to it): on the interpreter, or with --native
# as the executable stage 0's backend builds once into the work directory.
stage1_exe=""
if [ "$native" = 1 ]; then
    echo "building stage 1 natively (sisu build --native)"
    if ! (cd "$here" && "$sisu" build --native --output "$work/stage1" > "$work/native.log" 2>&1); then
        cat "$work/native.log" >&2
        echo "check.sh: stage 1 does not build natively" >&2
        exit 1
    fi
    stage1_exe="$work/stage1"
fi
stage1() {
    if [ -n "$stage1_exe" ]; then
        (cd "$here" && "$stage1_exe" "$@")
    else
        (cd "$here" && "$sisu" run -- "$@")
    fi
}

# Stage 0's outputs, made `jobs` at a time: `parallel FUNCTION LIST REF`
# runs `FUNCTION N FILE REF` for the Nth file of LIST. `sisu check
# --format=ids` runs once per file (`check0`), from the file's directory,
# and the names, types and ownership classes read what it printed.
jobs=$(nproc 2> /dev/null || echo 4)
mkdir -p "$work/check0"
export sisu work
parallel() {
    awk -v ref="$3" '{ print NR " " $0 " " ref }' "$2" | xargs -P "$jobs" -L 1 bash -c "$1"' "$@"' _
}
stage0_syntax() {
    "$sisu" tokens --trivia "$2" > "$3/$1.tokens" 2> "$3/$1.tokens.err"
    "$sisu" parse --tree "$2" > "$3/$1.tree" 2> "$3/$1.tree.err"
    "$sisu" parse --tree --trivia "$2" > "$3/$1.trivia.tree" 2> /dev/null
    "$sisu" parse --ast "$2" > "$3/$1.ast" 2> /dev/null
}
check0() {
    echo "$work/check0/$(printf '%s' "$1" | tr '/' '%')"
}
stage0_check() {
    local kept
    kept=$(check0 "$2")
    [ -e "$kept" ] && return 0
    if [ -d "$2" ]; then
        (cd "$2" && "$sisu" check --format=ids 2> /dev/null) > "$kept.part"
    else
        (cd "$(dirname "$2")" && "$sisu" check --format=ids "$(basename "$2")" 2> /dev/null) > "$kept.part"
    fi
    mv "$kept.part" "$kept"
}
stage0_sir() {
    if [ -d "$2" ]; then
        (cd "$2" && "$sisu" sir 2> /dev/null) > "$3/$1.sir"
    else
        "$sisu" sir "$2" > "$3/$1.sir" 2> /dev/null
    fi
    stage0_check "$@"
}
# What `sisu check` prints in the human form, on standard error, run as
# stage0_check runs it.
stage0_human() {
    if [ -d "$2" ]; then
        (cd "$2" && "$sisu" check > /dev/null 2> "$3/$1.human")
    else
        (cd "$(dirname "$2")" && "$sisu" check "$(basename "$2")" > /dev/null 2> "$3/$1.human")
    fi
    stage0_check "$@"
}
# What `sisu sir --contracts=LEVEL` prints, the level in `$level`.
stage0_sir_level() {
    if [ -d "$2" ]; then
        (cd "$2" && "$sisu" sir --contracts="$level" 2> /dev/null) > "$3/$1.sir"
    else
        "$sisu" sir --contracts="$level" "$2" > "$3/$1.sir" 2> /dev/null
    fi
}
export -f stage0_syntax check0 stage0_check stage0_sir stage0_human stage0_sir_level

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
# The programs of several modules and packages (09): each package case of
# the conformance suite and of stage 1's own, a directory `NAME.pkg` (or the
# one `NAME.cwd` names inside it), which stage 0 checks from inside it, and
# the conformance suite's scripts beside them.
for cases in "$compiler/conformance/cases/packages" "$here/cases/packages"; do
    for d in "$cases"/*.pkg; do
        [ -d "$d" ] || continue
        if [ -f "${d%.pkg}.cwd" ]; then echo "$d/$(cat "${d%.pkg}.cwd")"; else echo "$d"; fi
    done
done > "$work/packages.list"
find "$compiler/conformance/cases/packages" -maxdepth 1 -name '*.sisu' | sort >> "$work/packages.list"
classes="conformance std examples cases"
if [ "$mutants" -gt 0 ]; then
    mkdir -p "$work/mutants.src"
    cat "$work/conformance.list" "$work/std.list" "$work/examples.list" > "$work/corpus.list"
    if ! stage1 mutate "$seed" "$mutants" "$work/mutants.src" "$work/corpus.list" \
            > "$work/mutate.log" 2>&1; then
        cat "$work/mutate.log" >&2
        echo "check.sh: stage 1 could not write the mutants" >&2
        exit 1
    fi
    for k in $(seq 1 "$mutants"); do echo "$work/mutants.src/m$k.sisu"; done > "$work/mutants.list"
    classes="$classes mutants"
fi

# A mutant's difference is a listed discrepancy's when it has its mark (a
# test of the outputs that only that discrepancy makes). None is listed
# now, so every mutant must give identical outputs.
explained() {
    return 1
}

total=0; identical=0; subsets=0; lowered_beyond=0; check_beyond=0; listed=0; unlisted=0
report=""
summary=""
for class in $classes; do
    list="$work/$class.list"
    n=$(wc -l < "$list" | tr -d ' ')
    ref="$work/$class.stage0"; out="$work/$class.stage1"
    mkdir -p "$ref" "$out"
    # Stage 0's outputs.
    parallel stage0_syntax "$list" "$ref"
    # Stage 1's, in one run of the interpreter.
    start=$(date +%s.%N)
    if ! stage1 batch "$out" "$list" > "$work/$class.log" 2> "$work/$class.stderr"; then
        cat "$work/$class.stderr" >&2
        echo "check.sh: stage 1 failed on the $class files" >&2
        exit 1
    fi
    end=$(date +%s.%N)
    seconds=$(awk "BEGIN { printf \"%.1f\", $end - $start }")
    phases=$(sed -n 's/^files [0-9]* bytes \([0-9]*\) lex \([0-9]*\) ms parse \([0-9]*\) ms dump \([0-9]*\) ms ast \([0-9]*\) ms$/\1 bytes; lex \2 ms, parse \3 ms, dumps \4 ms, ast \5 ms/p' "$work/$class.log")
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
        cmp -s "$ref/$i.ast" "$out/$i.ast" || differs="$differs ast"
        if [ -z "$differs" ]; then
            same=$((same + 1))
            continue
        fi
        rel="${f#"$repo"/}"
        expected=1
        # The AST follows the tree: it may differ only where the tree does.
        ast_follows=0
        case "$differs" in *tree*) ast_follows=1 ;; esac
        if [ "$class" = mutants ]; then
            explained "$f" "$ref/$i" "$out/$i" || expected=0
            # The listed discrepancies never touch the reprint.
            case "$differs" in *print*) expected=0 ;; esac
            case "$differs" in *ast*) [ "$ast_follows" = 1 ] || expected=0 ;; esac
        else
            for o in $differs; do
                [ "$o" = ast ] && [ "$ast_follows" = 1 ] && continue
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

# Name resolution, one module at a time. Stage 1's cases in
# `cases/checker_names/` have name diagnostics that its type checker
# reports (in code the compiler writes, or a body's type whose path's
# first name is a module), which the name pass alone does not: the classes
# of `check` compare them.
find "$compiler/conformance/cases/names" -name '*.sisu' | sort > "$work/names.list"
grep -v '/packages/' "$work/cases.list" | grep -v '/checker_names/' >> "$work/names.list"
cat "$work/packages.list" >> "$work/names.list"
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
    parallel stage0_check "$list" "$ref"
    i=0
    while read -r f; do
        i=$((i + 1))
        awk '/^  / { if (keep) print; next }
             { keep = ($2 == "E0200" || $2 == "E0201" || $2 == "E0202" || $2 == "E0220" ||
                       $2 == "E0234" || $2 == "E0389" || $2 == "W0002"); if (keep) print }' \
            "$(check0 "$f")" > "$ref/$i.names"
    done < "$list"
    start=$(date +%s.%N)
    if ! stage1 names-batch "$out" "$list" > "$work/$class.log" 2> "$work/$class.stderr"; then
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

# The type checker's diagnostics, one module at a time. A diagnostic and
# its labels and fixes are one line, for comparing them as sets.
type_blocks() {
    awk '/^  / { printf "\037%s", $0; next } { if (NR > 1) printf "\n"; printf "%s", $0 } END { if (NR) printf "\n" }' "$1"
}
sed -n 's/^\([^#].*\)$/\1/p' "$here/types.list" | while read -r p; do echo "$repo/$p"; done > "$work/types.list"
type_classes="types"
if [ "$all_types" = 1 ]; then
    find "$compiler/conformance/cases" -name '*.sisu' | grep -v '/packages/' | sort > "$work/corpus-types.list"
    cat "$work/examples.list" "$work/packages.list" >> "$work/corpus-types.list"
    type_classes="types corpus-types"
fi
types_summary=""
for class in $type_classes; do
    list="$work/$class.list"
    n=$(wc -l < "$list" | tr -d ' ')
    ref="$work/$class.stage0"; out="$work/$class.stage1"
    mkdir -p "$ref" "$out"
    parallel stage0_check "$list" "$ref"
    i=0
    while read -r f; do
        i=$((i + 1))
        awk '/^  / { if (keep) print; next } { keep = ($3 ~ /^type\./); if (keep) print }' \
            "$(check0 "$f")" > "$ref/$i.types"
    done < "$list"
    start=$(date +%s.%N)
    if ! stage1 types-batch "$out" "$list" > "$work/$class.log" 2> "$work/$class.stderr"; then
        cat "$work/$class.stderr" >&2
        echo "check.sh: stage 1 failed checking the types of the $class files" >&2
        exit 1
    fi
    end=$(date +%s.%N)
    seconds=$(awk "BEGIN { printf \"%.1f\", $end - $start }")
    same=0; beyond=0; class_listed=0; class_unlisted=0
    i=0
    while read -r f; do
        i=$((i + 1))
        rel="${f#"$repo"/}"
        if [ -s "$out/$i.beyond" ]; then
            why=$(head -1 "$out/$i.beyond")
            if [ "$class" = types ]; then
                class_unlisted=$((class_unlisted + 1))
                report="$report\n  UNLISTED: $rel: types (beyond the slice: $why)"
                continue
            fi
            # Beyond the slice: only fewer of stage 0's diagnostics.
            type_blocks "$ref/$i.types" > "$ref/$i.blocks"
            type_blocks "$out/$i.types" > "$out/$i.blocks"
            if grep -vxFf "$ref/$i.blocks" "$out/$i.blocks" > /dev/null; then
                class_unlisted=$((class_unlisted + 1))
                report="$report\n  UNLISTED: $rel: types, beyond the slice ($why) but not a subset (stage 0 in $ref/$i.types, stage 1 in $out/$i.types)"
            else
                beyond=$((beyond + 1))
            fi
            continue
        fi
        if cmp -s "$ref/$i.types" "$out/$i.types"; then
            same=$((same + 1))
            continue
        fi
        if grep -qxF "$rel types" "$allowed"; then
            class_listed=$((class_listed + 1))
            report="$report\n  listed:   $rel: types"
        else
            class_unlisted=$((class_unlisted + 1))
            report="$report\n  UNLISTED: $rel: types (stage 0 in $ref/$i.types, stage 1 in $out/$i.types)"
        fi
    done < "$list"
    total=$((total + n)); identical=$((identical + same)); subsets=$((subsets + beyond))
    listed=$((listed + class_listed)); unlisted=$((unlisted + class_unlisted))
    types_summary="$types_summary\n$(printf '  %-12s %4d files, %4d identical, %3d beyond the slice and a subset, %3d differing as listed, %3d otherwise; stage 1 %6ss' \
        "$class" "$n" "$same" "$beyond" "$class_listed" "$class_unlisted" "$seconds")"
done

# Lowering and the ownership checker, one program at a time: what `sisu
# sir` prints on standard output, and the ownership checker's diagnostics
# of `sisu check --format=ids` (with their labels and fixes), against
# stage 1's SIR and diagnostics.
own_ids='^(borrow\.(use-after-move|use-after-partial-move|modify-while-borrowed|use-while-mut-borrowed|move-while-borrowed|mut-while-borrowed|does-not-live-long-enough|return-local|return-not-from|store-in-plain-type|store-into-caller|move-out-of-drop|must-consume|must-consume-exit|move-out-of-place|must-consume-intrinsic)|type\.(impure-call|uninitialized|reassign-let|impure-value-call|impure-drop|must-consume-argument)|task\.(borrows-body-local|borrows-elsewhere))$'
sed -n 's/^\([^#].*\)$/\1/p' "$here/sir.list" | while read -r p; do echo "$repo/$p"; done > "$work/sir.list"
sir_classes="sir"
if [ "$all_sir" = 1 ]; then
    find "$compiler/conformance/cases" -name '*.sisu' | grep -v '/packages/' | sort > "$work/corpus-sir.list"
    cat "$work/examples.list" "$work/packages.list" >> "$work/corpus-sir.list"
    sir_classes="sir corpus-sir"
fi
sir_summary=""
own_summary=""
for class in $sir_classes; do
    list="$work/$class.list"
    n=$(wc -l < "$list" | tr -d ' ')
    ref="$work/$class.stage0"; out="$work/$class.stage1"
    mkdir -p "$ref" "$out"
    parallel stage0_sir "$list" "$ref"
    i=0
    while read -r f; do
        i=$((i + 1))
        awk -v ids="$own_ids" '/^  / { if (keep) print; next } { keep = ($3 ~ ids); if (keep) print }' \
            "$(check0 "$f")" > "$ref/$i.ownership"
    done < "$list"
    start=$(date +%s.%N)
    if ! stage1 sir-batch "$out" "$list" > "$work/$class.log" 2> "$work/$class.stderr"; then
        cat "$work/$class.stderr" >&2
        echo "check.sh: stage 1 failed lowering the $class files" >&2
        exit 1
    fi
    end=$(date +%s.%N)
    seconds=$(awk "BEGIN { printf \"%.1f\", $end - $start }")
    # `sir-batch` ends with where its time went: "files N check MS ms
    # lower MS ms ownership MS ms".
    phases=$(awk '/^files / { printf "front end %.0fs, lowering %.0fs, ownership %.0fs", $4 / 1000, $7 / 1000, $10 / 1000 }' "$work/$class.log")
    for output in sir ownership; do
        same=0; beyond=0; class_listed=0; class_unlisted=0
        i=0
        while read -r f; do
            i=$((i + 1))
            rel="${f#"$repo"/}"
            if [ -s "$out/$i.beyond" ]; then
                if [ "$class" = sir ]; then
                    class_unlisted=$((class_unlisted + 1))
                    report="$report\n  UNLISTED: $rel: $output (beyond the slice: $(head -1 "$out/$i.beyond"))"
                else
                    beyond=$((beyond + 1))
                fi
                continue
            fi
            if cmp -s "$ref/$i.$output" "$out/$i.$output"; then
                same=$((same + 1))
                continue
            fi
            if grep -qxF "$rel $output" "$allowed"; then
                class_listed=$((class_listed + 1))
                report="$report\n  listed:   $rel: $output"
            else
                class_unlisted=$((class_unlisted + 1))
                report="$report\n  UNLISTED: $rel: $output (stage 0 in $ref/$i.$output, stage 1 in $out/$i.$output)"
            fi
        done < "$list"
        total=$((total + n)); identical=$((identical + same)); lowered_beyond=$((lowered_beyond + beyond))
        listed=$((listed + class_listed)); unlisted=$((unlisted + class_unlisted))
        label="$class"
        if [ "$output" = ownership ]; then
            label=ownership
            [ "$class" = corpus-sir ] && label=corpus-own
        fi
        line="$(printf '  %-12s %4d files, %4d identical, %3d beyond the slice, %3d differing as listed, %3d otherwise' \
            "$label" "$n" "$same" "$beyond" "$class_listed" "$class_unlisted")"
        if [ "$output" = sir ]; then
            sir_summary="$sir_summary\n$line; stage 1 $(printf '%6s' "$seconds")s ($phases)"
        else
            own_summary="$own_summary\n$line (the same run)"
        fi
    done
done

# The SIR of the programs of `sir.list` that have contracts, lowered with
# the fewer contracts a build may ask for (07; decision 232: a release
# build checks its manifest's `[profile.release] contracts`, preconditions
# by default), against `sisu sir --contracts=LEVEL`.
while read -r f; do
    if grep -rqE '^ *(requires|ensures|invariant) ' "$f" 2> /dev/null; then echo "$f"; fi
done < "$work/sir.list" > "$work/contracts.list"
contracts_summary=""
for level in off preconditions; do
    class="contracts-$level"
    list="$work/contracts.list"
    n=$(wc -l < "$list" | tr -d ' ')
    ref="$work/$class.stage0"; out="$work/$class.stage1"
    mkdir -p "$ref" "$out"
    export level
    parallel stage0_sir_level "$list" "$ref"
    start=$(date +%s.%N)
    if ! stage1 sir-batch "$out" "$list" --contracts="$level" > "$work/$class.log" 2> "$work/$class.stderr"; then
        cat "$work/$class.stderr" >&2
        echo "check.sh: stage 1 failed lowering the $class files" >&2
        exit 1
    fi
    end=$(date +%s.%N)
    seconds=$(awk "BEGIN { printf \"%.1f\", $end - $start }")
    same=0; class_unlisted=0
    i=0
    while read -r f; do
        i=$((i + 1))
        rel="${f#"$repo"/}"
        if [ -s "$out/$i.beyond" ]; then
            class_unlisted=$((class_unlisted + 1))
            report="$report\n  UNLISTED: $rel: $class (beyond the slice: $(head -1 "$out/$i.beyond"))"
        elif cmp -s "$ref/$i.sir" "$out/$i.sir"; then
            same=$((same + 1))
        else
            class_unlisted=$((class_unlisted + 1))
            report="$report\n  UNLISTED: $rel: $class (stage 0 in $ref/$i.sir, stage 1 in $out/$i.sir)"
        fi
    done < "$list"
    total=$((total + n)); identical=$((identical + same)); unlisted=$((unlisted + class_unlisted))
    contracts_summary="$contracts_summary\n$(printf '  %-24s %4d files, %4d identical, %3d otherwise; stage 1 %6ss' \
        "$class" "$n" "$same" "$class_unlisted" "$seconds")"
done

# What `check` prints: every diagnostic of a program, in the conformance
# form (the whole of `sisu check --format=ids`) and in the human form (what
# `sisu check` prints on standard error), against stage 1's `check-batch`.
find "$compiler/conformance/cases/wrong" -name '*.sisu' | sort > "$work/human.list"
grep -v '/packages/' "$work/cases.list" > "$work/cases-check.list"
check_classes="human cases-check"
if [ "$all_check" = 1 ]; then
    find "$compiler/conformance/cases" -name '*.sisu' | grep -v '/packages/' | sort > "$work/corpus-check.list"
    cat "$work/examples.list" "$work/packages.list" >> "$work/corpus-check.list"
    check_classes="human corpus-check cases-check"
fi
check_summary=""
for class in $check_classes; do
    list="$work/$class.list"
    n=$(wc -l < "$list" | tr -d ' ')
    ref="$work/$class.stage0"; out="$work/$class.stage1"
    mkdir -p "$ref" "$out"
    parallel stage0_human "$list" "$ref"
    start=$(date +%s.%N)
    if ! stage1 check-batch "$out" "$list" > "$work/$class.log" 2> "$work/$class.stderr"; then
        cat "$work/$class.stderr" >&2
        echo "check.sh: stage 1 failed checking the $class files" >&2
        exit 1
    fi
    end=$(date +%s.%N)
    seconds=$(awk "BEGIN { printf \"%.1f\", $end - $start }")
    outputs="human"
    [ "$class" = corpus-check ] && outputs="ids human"
    [ "$class" = cases-check ] && outputs="ids human"
    for output in $outputs; do
        same=0; beyond=0; class_listed=0; class_unlisted=0
        i=0
        while read -r f; do
            i=$((i + 1))
            rel="${f#"$repo"/}"
            theirs="$ref/$i.human"
            [ "$output" = ids ] && theirs="$(check0 "$f")"
            if [ -s "$out/$i.beyond" ]; then
                why=$(head -1 "$out/$i.beyond")
                if [ "$class" != corpus-check ]; then
                    class_unlisted=$((class_unlisted + 1))
                    report="$report\n  UNLISTED: $rel: $output (beyond the slice: $why)"
                    continue
                fi
                # Beyond the slice: only fewer of stage 0's diagnostics.
                if [ "$output" = ids ]; then
                    type_blocks "$theirs" > "$ref/$i.blocks"
                    type_blocks "$out/$i.ids" > "$out/$i.blocks"
                    if grep -vxFf "$ref/$i.blocks" "$out/$i.blocks" > /dev/null; then
                        class_unlisted=$((class_unlisted + 1))
                        report="$report\n  UNLISTED: $rel: check, beyond the slice ($why) but not a subset (stage 0 in $theirs, stage 1 in $out/$i.ids)"
                        continue
                    fi
                fi
                beyond=$((beyond + 1))
                continue
            fi
            if cmp -s "$theirs" "$out/$i.$output"; then
                same=$((same + 1))
                continue
            fi
            if grep -qxF "$rel $output" "$allowed"; then
                class_listed=$((class_listed + 1))
                report="$report\n  listed:   $rel: $output"
            else
                class_unlisted=$((class_unlisted + 1))
                report="$report\n  UNLISTED: $rel: $output (stage 0 in $theirs, stage 1 in $out/$i.$output)"
            fi
        done < "$list"
        total=$((total + n)); identical=$((identical + same)); check_beyond=$((check_beyond + beyond))
        listed=$((listed + class_listed)); unlisted=$((unlisted + class_unlisted))
        label="$class"
        [ "$class" = corpus-check ] && [ "$output" = human ] && label=corpus-human
        [ "$class" = cases-check ] && [ "$output" = human ] && label=cases-human
        line="$(printf '  %-12s %4d files, %4d identical, %3d beyond the slice, %3d differing as listed, %3d otherwise' \
            "$label" "$n" "$same" "$beyond" "$class_listed" "$class_unlisted")"
        if [ "$output" = human ] && [ "$class" != human ]; then
            check_summary="$check_summary\n$line (the same run)"
        else
            check_summary="$check_summary\n$line; stage 1 $(printf '%6s' "$seconds")s"
        fi
    done
done

# What `build` makes (with --native): the executables of `sisu build
# --native`, in the debug and the release profile (decision 232), the
# latter with its manifest's contracts or those `--contracts` names, of a
# file, a package, a workspace (each member) and a package with features
# built with `--features` and `--no-default-features` (decision 250),
# copied into the work directory, against those stage 1's `build` makes
# there from its own SIR, with what each prints and the lockfile it writes
# (09), also by a plain build, one that `--locked` stops and one that names
# no feature (`D0112`). A run of stage 1 that fails ends with the line
# `error: Failed` (its `main` returned an error), which is not stage 0's
# and is left out.
build_summary=""
if [ "$native" = 1 ]; then
    mkdir -p "$work/build"
    cp "$compiler/conformance/cases/examples/hello.sisu" "$compiler/conformance/cases/contracts/checks.sisu" "$work/build/"
    cp -r "$compiler/conformance/cases/packages/workspace.pkg" "$work/build/workspace"
    cp -r "$compiler/conformance/cases/packages/features.pkg" "$work/build/features"
    mkdir -p "$work/build/contracts/src"
    cp "$compiler/conformance/cases/contracts/checks.sisu" "$work/build/contracts/src/main.sisu"
    printf '[package]\nname = "contracts"\nversion = "0.1.0"\n\n[profile.release]\ncontracts = "off"\n' \
        > "$work/build/contracts/sisu.toml"
    builds=0; same=0
    start=$(date +%s.%N)
    while read -r dir path made flags; do
        builds=$((builds + 1))
        what="$dir${path:+/$path} $flags"
        here_dir="$work/build/$dir"
        # What the build makes, and the lockfile, each once.
        made=$(printf '%s\n' $made sisu.lock | sort -u)
        # Stage 0's build, moved aside; then stage 1's, in the same
        # directory (the debug information names it, decision 227).
        (cd "$here_dir" && "$sisu" build $flags $path > "$work/build.0.out" 2>&1)
        rm -rf "$work/build.0" && mkdir -p "$work/build.0"
        for m in $made; do
            [ -e "$here_dir/$m" ] && mkdir -p "$work/build.0/$(dirname "$m")" && mv "$here_dir/$m" "$work/build.0/$m"
        done
        if [ -n "$stage1_exe" ]; then
            (cd "$here_dir" && "$stage1_exe" --home "$compiler" build $flags $path 2>&1) |
                sed '${/^error: Failed$/d;}' > "$work/build.1.out"
        fi
        differs=""
        kept="$work/build.$builds"
        for m in $made; do
            if [ -e "$work/build.0/$m" ] || [ -e "$here_dir/$m" ]; then
                if ! cmp -s "$work/build.0/$m" "$here_dir/$m"; then
                    differs="$differs $m"
                    mkdir -p "$kept/0/$(dirname "$m")" "$kept/1/$(dirname "$m")"
                    cp "$work/build.0/$m" "$kept/0/$m" 2> /dev/null
                    cp "$here_dir/$m" "$kept/1/$m" 2> /dev/null
                fi
            fi
            rm -f "$here_dir/$m"
        done
        if ! cmp -s "$work/build.0.out" "$work/build.1.out"; then
            differs="$differs output"
            mkdir -p "$kept/0" "$kept/1"
            cp "$work/build.0.out" "$kept/0/out"
            cp "$work/build.1.out" "$kept/1/out"
        fi
        rm -rf "$here_dir/build"
        if [ -z "$differs" ]; then
            same=$((same + 1))
        else
            unlisted=$((unlisted + 1))
            report="$report\n  UNLISTED: build: $what:$differs (stage 0 in $kept/0, stage 1 in $kept/1)"
        fi
    done <<'BUILDS'
. hello.sisu hello --native
. hello.sisu hello --native --release
. checks.sisu checks --native --release
. checks.sisu checks --native --contracts=off
. checks.sisu checks --native --release --contracts all
contracts . build/release/contracts --native --release
contracts . build/release/contracts --native --release --contracts=preconditions
contracts . sisu.lock
contracts . sisu.lock --native --locked
workspace . build/release/greeter --native --release
workspace . build/debug/greeter --native
workspace . sisu.lock
features/app . build/debug/app --native --features verbose
features/app . build/release/app --native --release --no-default-features
features/app . build/debug/app --native --features=render/borders,verbose
features/app . sisu.lock --native --features bogus
BUILDS
    end=$(date +%s.%N)
    total=$((total + builds)); identical=$((identical + same))
    seconds=$(awk "BEGIN { printf \"%.1f\", $end - $start }")
    build_summary="$(printf '  %-12s %4d builds, %4d identical, %3d otherwise; both %6ss' build "$builds" "$same" $((builds - same)) "$seconds")"
fi

# Stage 1 over its own package: its name and type diagnostics, and its
# SIR, split by the file each function comes from (each function's source
# written by name), with what is not a function as the declarations.
self_summary=""
if [ "$self_check" = 1 ]; then
    mkdir -p "$work/self.stage0" "$work/self.stage1"
    cat > "$work/split.awk" <<'SPLIT'
/^source [0-9]+ "/ {
    num = $2; name = $0
    sub(/^source [0-9]+ "/, "", name); sub(/"$/, "", name)
    src[num] = name
    next
}
/^fn / {
    line = $0; name = "(no source)"
    if (match(line, / source [0-9]+ /)) {
        num = substr(line, RSTART + 8, RLENGTH - 9)
        name = src[num]
        line = substr(line, 1, RSTART) "source \"" name "\" " substr(line, RSTART + RLENGTH)
    }
    file = name; gsub("/", "%", file); out = dir "/" file
    print line > out
    infn = 1
    next
}
infn { print > out; if ($0 == "}") infn = 0; next }
{ print > (dir "/(declarations)") }
SPLIT
    (cd "$here" && "$sisu" check --format=ids 2> /dev/null) > "$work/self.stage0.check"
    (cd "$here" && "$sisu" sir 2> /dev/null) > "$work/self.stage0.sir"
    awk '/^  / { if (keep) print; next }
         { keep = ($2 == "E0200" || $2 == "E0201" || $2 == "E0202" || $2 == "E0220" ||
                   $2 == "E0234" || $2 == "E0389" || $2 == "W0002"); if (keep) print }' \
        "$work/self.stage0.check" > "$work/self.stage0.names"
    awk '/^  / { if (keep) print; next } { keep = ($3 ~ /^type\./); if (keep) print }' \
        "$work/self.stage0.check" > "$work/self.stage0.types"
    start=$(date +%s.%N)
    if ! { stage1 names "$here" > "$work/self.stage1.names" 2> "$work/self.stderr" &&
           stage1 types "$here" > "$work/self.stage1.types" 2> "$work/self.types.beyond" &&
           stage1 sir --partial "$here" > "$work/self.stage1.sir" 2> "$work/self.beyond"; }; then
        cat "$work/self.stderr" >&2
        echo "check.sh: stage 1 failed over its own package" >&2
        exit 1
    fi
    end=$(date +%s.%N)
    seconds=$(awk "BEGIN { printf \"%.1f\", $end - $start }")
    awk -v dir="$work/self.stage0" -f "$work/split.awk" "$work/self.stage0.sir"
    awk -v dir="$work/self.stage1" -f "$work/split.awk" "$work/self.stage1.sir"
    why=$(sed -n 's/^beyond the slice: //p' "$work/self.beyond" | head -1)
    for output in names types; do
        total=$((total + 1))
        if cmp -s "$work/self.stage0.$output" "$work/self.stage1.$output" && [ ! -s "$work/self.types.beyond" ]; then
            identical=$((identical + 1))
            self_summary="$self_summary\n$(printf '  %-12s identical' "self-$output")"
        else
            unlisted=$((unlisted + 1))
            report="$report\n  UNLISTED: self: $output (stage 0 in $work/self.stage0.$output, stage 1 in $work/self.stage1.$output)"
            self_summary="$self_summary\n$(printf '  %-12s differing' "self-$output")"
        fi
    done
    n=0; same=0; beyond=0; differing=0
    for f in $( (ls "$work/self.stage0"; ls "$work/self.stage1") | sort -u); do
        n=$((n + 1))
        shown=$(printf '%s' "$f" | tr '%' '/')
        if cmp -s "$work/self.stage0/$f" "$work/self.stage1/$f"; then
            same=$((same + 1))
        elif [ -n "$why" ]; then
            beyond=$((beyond + 1))
            report="$report\n  beyond:   self: $shown (stage 1 is beyond the slice of lowering: $why)"
        else
            differing=$((differing + 1))
            report="$report\n  UNLISTED: self: $shown (stage 0 in $work/self.stage0/$f, stage 1 in $work/self.stage1/$f)"
        fi
    done
    total=$((total + n)); identical=$((identical + same)); lowered_beyond=$((lowered_beyond + beyond))
    unlisted=$((unlisted + differing))
    self_summary="$self_summary\n$(printf '  %-12s %4d files, %4d identical, %3d beyond the slice, %3d otherwise; stage 1 %6ss' \
        self-sir "$n" "$same" "$beyond" "$differing" "$seconds")"
fi

# The fixed point (D-078): stage1-A, stage 1 as stage 0 builds it (the
# executable the batches ran with --native), prints the SIR of its own
# package (`sir`, run in its directory), which must be stage 0's (`sisu
# sir`, from the class `self`) as a whole file; the backend alone builds
# that SIR into stage1-B; stage1-B prints the SIR of the package again,
# which must be stage1-A's; and the backend being deterministic, stage1-B
# must be stage1-A byte for byte.
fixed_summary=""
if [ "$self_check" = 1 ]; then
    stage1_a="$stage1_exe"
    if [ -z "$stage1_a" ]; then
        echo "building stage 1 natively for the fixed point (sisu build --native)"
        if ! (cd "$here" && "$sisu" build --native --output "$work/stage1-A" > "$work/fixed.A.log" 2>&1); then
            cat "$work/fixed.A.log" >&2
            echo "check.sh: stage 1 does not build natively" >&2
            exit 1
        fi
        stage1_a="$work/stage1-A"
    fi
    start=$(date +%s.%N)
    if ! (cd "$here" && "$stage1_a" sir > "$work/fixed.A.sir" 2> "$work/fixed.A.err"); then
        cat "$work/fixed.A.err" >&2
        echo "check.sh: stage1-A failed printing the SIR of its own package" >&2
        exit 1
    fi
    middle=$(date +%s.%N)
    # In this directory, as stage1-A is built: the debug information
    # records the directory a build runs in (decision 227).
    if ! (cd "$here" && "$sisu" build --native --from-sir "$work/fixed.A.sir" --output "$work/stage1-B" > "$work/fixed.B.log" 2>&1); then
        cat "$work/fixed.B.log" >&2
        echo "check.sh: the backend does not build stage1-A's SIR" >&2
        exit 1
    fi
    built=$(date +%s.%N)
    if ! (cd "$here" && "$work/stage1-B" sir > "$work/fixed.B.sir" 2> "$work/fixed.B.err"); then
        cat "$work/fixed.B.err" >&2
        echo "check.sh: stage1-B failed printing the SIR of its own package" >&2
        exit 1
    fi
    end=$(date +%s.%N)
    took_a=$(awk "BEGIN { printf \"%.1f\", $middle - $start }")
    took_backend=$(awk "BEGIN { printf \"%.1f\", $built - $middle }")
    took_b=$(awk "BEGIN { printf \"%.1f\", $end - $built }")
    size() { wc -c < "$1" | tr -d ' '; }
    fixed_pair() {
        # fixed_pair LABEL OURS THEIRS WHAT
        total=$((total + 1))
        if [ -s "$2" ] && cmp -s "$2" "$3"; then
            identical=$((identical + 1))
            fixed_summary="$fixed_summary\n$(printf '  %-12s identical: %s (%s bytes)' "$1" "$4" "$(size "$2")")"
        else
            unlisted=$((unlisted + 1))
            report="$report\n  UNLISTED: fixed: $1, $4 ($2 against $3)"
            fixed_summary="$fixed_summary\n$(printf '  %-12s differing: %s' "$1" "$4")"
        fi
    }
    fixed_pair fixed-sir0 "$work/fixed.A.sir" "$work/self.stage0.sir" "stage1-A's SIR of its own package and stage 0's"
    fixed_pair fixed-sir "$work/fixed.B.sir" "$work/fixed.A.sir" "stage1-B's SIR of the package and stage1-A's"
    fixed_pair fixed-exe "$work/stage1-B" "$stage1_a" "stage1-B and stage1-A"
    fixed_summary="$fixed_summary\n$(printf '  stage1-A %ss, the backend %ss, stage1-B %ss' "$took_a" "$took_backend" "$took_b")"
fi

echo "stage 1 against stage 0 (tokens, tree, trivia-tree, print, ast):"
printf "%b\n" "${summary#\\n}"
echo "stage 1's name resolution against stage 0's (names):"
printf "%b\n" "${names_summary#\\n}"
echo "stage 1's type checker against stage 0's (types):"
printf "%b\n" "${types_summary#\\n}"
echo "stage 1's lowering against stage 0's (sir):"
printf "%b\n" "${sir_summary#\\n}"
echo "stage 1's ownership checker against stage 0's (ownership):"
printf "%b\n" "${own_summary#\\n}"
echo "stage 1's lowering at the contract levels of a build against stage 0's (contracts):"
printf "%b\n" "${contracts_summary#\\n}"
echo "stage 1's check against stage 0's (every diagnostic, in the human and the conformance form):"
printf "%b\n" "${check_summary#\\n}"
if [ -n "$build_summary" ]; then
    echo "stage 1's build against stage 0's (build: the executables, the lockfile and what each prints):"
    printf "%b\n" "$build_summary"
fi
if [ -n "$self_summary" ]; then
    echo "stage 1 over its own package against stage 0 (self: names, types, the SIR by source file):"
    printf "%b\n" "${self_summary#\\n}"
fi
if [ -n "$fixed_summary" ]; then
    echo "the fixed point (fixed: stage1-A built by stage 0, stage1-B by the backend from stage1-A's SIR):"
    printf "%b\n" "${fixed_summary#\\n}"
fi
[ -n "$report" ] && printf "differences:%b\n" "$report"
beyond_note=""
[ "$subsets" -gt 0 ] && beyond_note=", $subsets beyond the type checker's slice and a subset"
[ "$lowered_beyond" -gt 0 ] && beyond_note="$beyond_note, $lowered_beyond beyond the slice of lowering"
[ "$check_beyond" -gt 0 ] && beyond_note="$beyond_note, $check_beyond with an error stage 1 does not render"
echo "total: $total files checked, $identical identical$beyond_note, $((listed + unlisted)) differing ($listed listed in DISCREPANCIES.md, $unlisted not)"
if [ "$unlisted" -gt 0 ]; then
    [ -z "$keep" ] && echo "(run with --keep DIR to inspect the outputs)"
    exit 1
fi
exit 0
