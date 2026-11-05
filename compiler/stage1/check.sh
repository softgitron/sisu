#!/usr/bin/env bash
# Checks stage 1 (this package, run on the stage-0 interpreter, or with
# --native compiled by stage 0's native backend), the front end since the
# Rust one was frozen: against the conformance suite's expectations of
# what each program does, the class `behaviour`, which is the primary
# oracle, and against the frozen Rust front end, byte for byte, but where
# RULES.md lists a rule stage 1 decides otherwise. First its lexer and
# parser against the Rust front end:
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
# (with the std modules only stage 1 reads, compiler/stage1/std/) and
# proposal/examples/, and stage 1's own cases (compiler/stage1/cases/).
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
# each of which must be within the slice and match. Then the SIR of the
# programs of `sir.list` with tests or benchmarks, built with them as
# `sisu test` and `sisu bench` run them (decision 285):
#
#   sir-tests    `sisu sir --tests FILE`, standard output: each test's
#                line with its tags and each benchmark's line, and the
#                functions they reach
#
# each of which must match. Then the transcripts' roots (RULES.md, R120
# and R132), for each program of the conformance suite whose `NAME.files`
# holds `tests/*.transcript`, in a sandbox as the runner makes one:
#
#   transcripts  `sisu sir --tests NAME.sisu` there, standard output,
#                against stage 1's, which must keep `transcript_world` and
#                `transcript_system` as functions; and `sisu test
#                NAME.sisu`, which runs its tests and its transcripts from
#                stage 1's SIR (decision 722), its standard output against
#                the case's `NAME.test`
#
# each of which must match or differ as listed. Then what the programs do:
#
#   behaviour    every runnable program of the corpus and of stage 1's own
#                cases (one with an `.out`, `.err` or `.status`), lowered
#                by stage 1 in a sandbox as the conformance runner makes
#                one and run from that SIR by stage 0's interpreter (`sisu
#                run --from-sir`) and natively (`sisu build --native
#                --from-sir`; of the conformance suite's cases, those its
#                native list names): each must print the case's `.out`
#                and `.err` and exit with its `.status`, natively with no
#                allocation live at exit; a case a rule of RULES.md
#                refuses (its output `behaviour`) is not run, and one whose
#                errors stop it (its `.err` ends with `D0006` or `D0021`)
#                must get no SIR and that `.err` from `build --for-run`
#
# Then what `check` prints, every diagnostic of a program in the order
# `sisu check` prints them, from one run of stage 1 per class:
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
#   expected     the whole of stage 1's `check --format=ids` of each of its
#                own cases that has a `NAME.diag` beside it (a rule of
#                RULES.md's, whose diagnostics the frozen Rust front end does
#                not give), against that file, as the conformance suite
#                holds its cases to theirs (a package case's with --check),
#                and of each with a `NAME.audit`, a `NAME.lint` or a
#                `NAME.sir`, what `audit --unsafe`, `lint --format=ids` or
#                `sir` prints of it
# Then the formatter, from one run of stage 1:
#
#   fmt          the text `sisu fmt FILE` leaves (stage 0 formats a copy),
#                for every `.sisu` file of the conformance suite, the
#                standard library, the proposal's examples, stage 1's own
#                sources and its cases; a file with a syntax error is left
#                as it is
#   fmt-check    `sisu fmt --check FILE`, standard output and error and the
#                exit status, for the same files
#   fmt-again    stage 1's formatted text formatted again, which must be
#                itself (the formatter's fixed point)
#   fmt-joined   with --native, all three, of the same files with the lines
#                inside each bracket joined (stage 1's `deform`), what the
#                formatters must break again
#   fmt-spread   with --native, all three, of the same files with a line
#                break after each comma inside brackets, what the formatters
#                must join again
#   fmt-mutants  with --mutants N, all three, of the mutants: the refusals of
#                those that do not parse, and those that do formatted
#
# Then the documentation (decision 258), from one run of stage 1:
#
#   doc          `sisu doc`, standard output and error and the exit status,
#                for every program of the corpus, stage 1's own cases and,
#                with --self, stage 1's own package (a file run in its
#                directory, a package inside it), for the standard library
#                (`--std`, all of it and some modules) and for arguments it
#                refuses
#   doc-output   `sisu doc --output DIR`: the pages it writes into DIR, file
#                for file, what it prints and its exit status, for the same
#
# Then what `sisu repl` and `sisu ws` run (RULES.md, R79), from stage 1's
# `sir --repl` and `sir --worksheet`, run by stage 0's interpreter (`sisu
# run --from-sir`):
#
#   repl         each session of cases/repl/NAME.repl/, its inputs 1.in,
#                2.in, ... in order (a `load.sisu` loaded first, as `sisu
#                repl FILE` loads it): what each input shows, its
#                diagnostics, or what its run adds to the output of the run
#                before it (an input whose run fails is not kept), against
#                NAME.repl.out
#   ws           each worksheet cases/repl/NAME.ws.sisu rendered as the
#                driver renders it (`repl.rs`: each line with its values,
#                and what it printed under it), with its standard error and
#                status, against what `sisu ws FILE` prints of it
#
# A difference listed in RULES.md or DISCREPANCIES.md ("- Differs: `path`:
# outputs", under the rule or the discrepancy that makes it) is expected;
# any other fails the check, and so does any program of `behaviour` that
# does not do what its case says.
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
#                what `sisu sir` prints of it, the whole file, but as
#                RULES.md lists (R124: the Rust front end, an oracle that
#                falls behind, decides nothing of the fixed point); stage 0's
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
#                --self and --check, and adds the classes `build`,
#                `fmt-joined` and `fmt-spread`
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

# Stage 0 is the oracle of what has not changed only through its own front
# end: `sisu` runs stage 1's unless `SISU_FRONT_END=rust` says otherwise
# (decision 268), so the run asks for the Rust one unless told which.
export SISU_FRONT_END="${SISU_FRONT_END:-rust}"

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

# The Unicode tables of names are what `tools/unicode_tables.py` writes from
# Python's `unicodedata` (RULES.md, R136): a stale file fails the run.
if ! python3 -I "$here/tools/unicode_tables.py" --check; then
    echo "check.sh: src/unicode_tables.sisu is stale (python3 tools/unicode_tables.py)" >&2
    exit 1
fi

echo "building stage 0 (cargo build --release)"
if ! (cd "$compiler" && cargo build --release --quiet); then
    echo "check.sh: stage 0 does not build" >&2
    exit 1
fi

# Stage 0 writes a package's `sisu.lock` beside its manifest when it checks
# it (09); the run removes the ones it wrote, and leaves the cases as they
# were.
lock_dirs=("$compiler/conformance/cases" "$here/cases/packages")
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
export sisu work compiler
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
# `in_package DIR COMMAND...`: COMMAND run in the package directory DIR,
# whose lockfile is left as it was. Stage 0 rewrites a lockfile without the
# `unsafe` levels that stage 1 records (RULES.md, R6), and stage 1, which
# refuses a level that rises past what the lockfile records, would then
# refuse the case's own; stage 1's batches write no lockfile.
in_package() {
    local dir="$1" saved="" status
    shift
    if [ -f "$dir/sisu.lock" ]; then
        saved=$(mktemp)
        cp "$dir/sisu.lock" "$saved"
    fi
    (cd "$dir" && "$@")
    status=$?
    if [ -n "$saved" ]; then
        cp "$saved" "$dir/sisu.lock"
        rm -f "$saved"
    else
        rm -f "$dir/sisu.lock"
    fi
    return "$status"
}
stage0_check() {
    local kept
    kept=$(check0 "$2")
    [ -e "$kept" ] && return 0
    if [ -d "$2" ]; then
        in_package "$2" "$sisu" check --format=ids 2> /dev/null > "$kept.part"
    else
        (cd "$(dirname "$2")" && "$sisu" check --format=ids "$(basename "$2")" 2> /dev/null) > "$kept.part"
    fi
    mv "$kept.part" "$kept"
}
stage0_sir() {
    if [ -d "$2" ]; then
        in_package "$2" "$sisu" sir 2> /dev/null > "$3/$1.sir"
    else
        "$sisu" sir "$2" > "$3/$1.sir" 2> /dev/null
    fi
    stage0_check "$@"
}
# What `sisu check` prints in the human form, on standard error, run as
# stage0_check runs it.
stage0_human() {
    if [ -d "$2" ]; then
        in_package "$2" "$sisu" check > /dev/null 2> "$3/$1.human"
    else
        (cd "$(dirname "$2")" && "$sisu" check "$(basename "$2")" > /dev/null 2> "$3/$1.human")
    fi
    stage0_check "$@"
}
# What `sisu sir --contracts=LEVEL` prints, the level in `$level`.
stage0_sir_level() {
    if [ -d "$2" ]; then
        in_package "$2" "$sisu" sir --contracts="$level" 2> /dev/null > "$3/$1.sir"
    else
        "$sisu" sir --contracts="$level" "$2" > "$3/$1.sir" 2> /dev/null
    fi
}
# What `sisu fmt --check FILE` prints and its status, and the text `sisu
# fmt` leaves of a copy of the file.
# (Each in a subshell, whose report of a stage 0 that aborted goes.)
stage0_fmt() {
    ("$sisu" fmt --check "$2" > "$3/$1.check.out" 2> "$3/$1.check.err"; exit $?) 2> /dev/null
    echo "$?" > "$3/$1.check.status"
    mkdir -p "$3/$1.copy"
    cp "$2" "$3/$1.copy/file.sisu"
    ("$sisu" fmt "$3/$1.copy/file.sisu" > /dev/null 2>&1; exit $?) 2> /dev/null
    mv "$3/$1.copy/file.sisu" "$3/$1.fmt"
    rmdir "$3/$1.copy"
}
# What `sisu doc` prints of a program (a file, run in its directory, or a
# package, run inside it), its status, and the pages `sisu doc --output
# DIR` writes, with what that prints and its status.
stage0_doc() {
    local dir="$2" what=()
    if [ ! -d "$2" ]; then
        dir=$(dirname "$2")
        what=("$(basename "$2")")
    fi
    if [ -d "$2" ]; then
        in_package "$dir" "$sisu" doc > "$3/$1.doc.out" 2> "$3/$1.doc.err"
        echo "$?" > "$3/$1.doc.status"
        in_package "$dir" "$sisu" doc --output "$3/$1.pages" > "$3/$1.pages.out" 2> "$3/$1.pages.err"
        echo "$?" > "$3/$1.pages.status"
        return
    fi
    (cd "$dir" && "$sisu" doc "${what[@]}" > "$3/$1.doc.out" 2> "$3/$1.doc.err"; echo "$?" > "$3/$1.doc.status")
    (cd "$dir" && "$sisu" doc "${what[@]}" --output "$3/$1.pages" > "$3/$1.pages.out" 2> "$3/$1.pages.err"
     echo "$?" > "$3/$1.pages.status")
}
# What `sisu sir --tests` prints: the program built with its tests
# (decision 285).
stage0_sir_tests() {
    if [ -d "$2" ]; then
        in_package "$2" "$sisu" sir --tests 2> /dev/null > "$3/$1.sir"
    else
        "$sisu" sir --tests "$2" > "$3/$1.sir" 2> /dev/null
    fi
}
export -f stage0_syntax check0 in_package stage0_check stage0_sir stage0_human stage0_sir_level stage0_fmt stage0_doc stage0_sir_tests

# The differences DISCREPANCIES.md lists (stage 0's accidents) and those
# RULES.md lists (the rules stage 1 decides where the frozen Rust front end
# keeps its own), one "path output" per line.
allowed="$work/allowed.txt"
cat "$here/DISCREPANCIES.md" "$here/RULES.md" | sed -n 's/^- Differs: `\([^`]*\)`: \(.*\)$/\1 \2/p' |
    while read -r path outputs; do
        for o in $(echo "$outputs" | tr ',' ' '); do echo "$path $o"; done
    done > "$allowed"

# The file classes, as lists of absolute paths.
find "$compiler/conformance/cases" -name '*.sisu' | sort > "$work/conformance.list"
{ find "$compiler/std" -name '*.sisu'; find "$here/std" -name '*.sisu'; } | sort > "$work/std.list"
find "$repo/proposal/examples" -name '*.sisu' | sort > "$work/examples.list"
# Not cases/repl/, whose sessions and worksheets the classes `repl` and `ws`
# take (RULES.md, R79).
find "$here/cases" -name '*.sisu' -not -path "$here/cases/repl/*" | sort > "$work/cases.list"
# The programs of several modules and packages (09): each package case of
# the conformance suite and of stage 1's own, a directory `NAME.pkg` (or the
# one `NAME.cwd` names inside it), which stage 0 checks from inside it, and
# the conformance suite's scripts beside them.
{
    find "$compiler/conformance/cases" -name '*.pkg' -type d -prune | sort
    find "$here/cases/packages" -maxdepth 1 -name '*.pkg' -type d | sort
} | while read -r d; do
    if [ -f "${d%.pkg}.cwd" ]; then echo "$d/$(cat "${d%.pkg}.cwd")"; else echo "$d"; fi
done > "$work/packages.list"
find "$compiler/conformance/cases/packages" -maxdepth 1 -name '*.sisu' | sort >> "$work/packages.list"
classes="conformance std examples cases"
if [ "$mutants" -gt 0 ]; then
    mkdir -p "$work/mutants.src"
    # Not the std modules only stage 1 reads: stage 0 parses none of their
    # mutants as stage 1 does.
    { cat "$work/conformance.list"; grep -vF "$here/std/" "$work/std.list"; cat "$work/examples.list"; } \
        > "$work/corpus.list"
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
                grep -qxF -e "$rel $o" "$allowed" || expected=0
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
    find "$compiler/conformance/cases" -name '*.sisu' | grep -v '/packages/' | grep -v '\.pkg/' | grep -v '/names/' | sort \
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
        if grep -qxF -e "$rel names" "$allowed"; then
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
    find "$compiler/conformance/cases" -name '*.sisu' | grep -v '/packages/' | grep -v '\.pkg/' | sort > "$work/corpus-types.list"
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
        if grep -qxF -e "$rel types" "$allowed"; then
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
    find "$compiler/conformance/cases" -name '*.sisu' | grep -v '/packages/' | grep -v '\.pkg/' | sort > "$work/corpus-sir.list"
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
            if grep -qxF -e "$rel $output" "$allowed"; then
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
    same=0; class_listed=0; class_unlisted=0
    i=0
    while read -r f; do
        i=$((i + 1))
        rel="${f#"$repo"/}"
        if [ -s "$out/$i.beyond" ]; then
            class_unlisted=$((class_unlisted + 1))
            report="$report\n  UNLISTED: $rel: $class (beyond the slice: $(head -1 "$out/$i.beyond"))"
        elif cmp -s "$ref/$i.sir" "$out/$i.sir"; then
            same=$((same + 1))
        elif grep -qxF -e "$rel $class" "$allowed"; then
            class_listed=$((class_listed + 1))
            report="$report\n  listed:   $rel: $class"
        else
            class_unlisted=$((class_unlisted + 1))
            report="$report\n  UNLISTED: $rel: $class (stage 0 in $ref/$i.sir, stage 1 in $out/$i.sir)"
        fi
    done < "$list"
    total=$((total + n)); identical=$((identical + same))
    listed=$((listed + class_listed)); unlisted=$((unlisted + class_unlisted))
    contracts_summary="$contracts_summary\n$(printf '  %-24s %4d files, %4d identical, %3d differing as listed, %3d otherwise; stage 1 %6ss' \
        "$class" "$n" "$same" "$class_listed" "$class_unlisted" "$seconds")"
done

# The SIR of the programs of `sir.list` that have tests or benchmarks,
# built with them as `sisu test` and `sisu bench` run them (decision 285:
# each test's tags and the benchmarks written), against `sisu sir --tests`.
while read -r f; do
    if grep -rqE '^ *(test|bench|suite) "' "$f" 2> /dev/null; then echo "$f"; fi
done < "$work/sir.list" > "$work/sir-tests.list"
class=sir-tests
list="$work/$class.list"
n=$(wc -l < "$list" | tr -d ' ')
ref="$work/$class.stage0"; out="$work/$class.stage1"
mkdir -p "$ref" "$out"
parallel stage0_sir_tests "$list" "$ref"
start=$(date +%s.%N)
if ! stage1 sir-batch "$out" "$list" --tests > "$work/$class.log" 2> "$work/$class.stderr"; then
    cat "$work/$class.stderr" >&2
    echo "check.sh: stage 1 failed lowering the $class files" >&2
    exit 1
fi
end=$(date +%s.%N)
seconds=$(awk "BEGIN { printf \"%.1f\", $end - $start }")
same=0; class_listed=0; class_unlisted=0
i=0
while read -r f; do
    i=$((i + 1))
    rel="${f#"$repo"/}"
    if [ -s "$out/$i.beyond" ]; then
        class_unlisted=$((class_unlisted + 1))
        report="$report\n  UNLISTED: $rel: $class (beyond the slice: $(head -1 "$out/$i.beyond"))"
    elif cmp -s "$ref/$i.sir" "$out/$i.sir"; then
        same=$((same + 1))
    elif grep -qxF -e "$rel $class" "$allowed"; then
        class_listed=$((class_listed + 1))
        report="$report\n  listed:   $rel: $class"
    else
        class_unlisted=$((class_unlisted + 1))
        report="$report\n  UNLISTED: $rel: $class (stage 0 in $ref/$i.sir, stage 1 in $out/$i.sir)"
    fi
done < "$list"
total=$((total + n)); identical=$((identical + same))
listed=$((listed + class_listed)); unlisted=$((unlisted + class_unlisted))
tests_summary="$(printf '  %-24s %4d files, %4d identical, %3d differing as listed, %3d otherwise; stage 1 %6ss' \
    "$class" "$n" "$same" "$class_listed" "$class_unlisted" "$seconds")"

# The transcripts' roots (RULES.md, R120 and R132): each program of the
# conformance suite with transcripts, `NAME.files/tests/*.transcript`, in a
# sandbox as the runner makes one (the file and what its `.files/` holds).
# Its SIR built with its tests, stage 0's `sisu sir --tests` against stage
# 1's, which must hold `transcript_world` and `transcript_system`, so that
# the driver runs the transcripts from it (decision 722); and the driver's
# `sisu test` there, on stage 1 (`SISU_STAGE1_TRANSCRIPTS=1`, and with
# `--native` the executable this run built), whose standard output must be
# the case's `NAME.test`: the transcripts run from stage 1's SIR on every
# run, where R120 checked them by hand.
class=transcripts
find "$compiler/conformance/cases" -path '*.files/tests/*.transcript' | sed 's|\.files/tests/.*||' | sort -u |
    while read -r b; do [ -f "$b.sisu" ] && [ -f "$b.test" ] && echo "$b.sisu"; done > "$work/$class.list"
list="$work/$class.list"
n=$(wc -l < "$list" | tr -d ' ')
same=0; class_listed=0; class_unlisted=0
start=$(date +%s.%N)
i=0
while read -r f; do
    i=$((i + 1))
    rel="${f#"$repo"/}"
    name=$(basename "$f")
    box="$work/$class/$i"
    mkdir -p "$box"
    cp "$f" "$box/"
    cp -r "${f%.sisu}.files/." "$box/"
    (cd "$box" && "$sisu" sir --tests "$name" > stage0.sir 2> /dev/null)
    boxed=$(printf '%s' "$box" | sed 's/[.[\*^$|]/\\&/g')
    stage1 sir --tests "$box/$name" 2> "$box/stage1.err" | sed "s|$boxed/||g" > "$box/stage1.sir"
    roots=0
    grep -qE '^fn transcript_world\(' "$box/stage1.sir" && grep -qE '^fn transcript_system\(' "$box/stage1.sir" && roots=1
    if [ "$roots" = 0 ]; then
        class_unlisted=$((class_unlisted + 1))
        report="$report\n  UNLISTED: $rel: $class (stage 1's SIR in $box/stage1.sir keeps no transcript_world and transcript_system)"
    elif cmp -s "$box/stage0.sir" "$box/stage1.sir"; then
        same=$((same + 1))
    elif grep -qxF -e "$rel $class" "$allowed"; then
        class_listed=$((class_listed + 1))
        report="$report\n  listed:   $rel: $class"
    else
        class_unlisted=$((class_unlisted + 1))
        report="$report\n  UNLISTED: $rel: $class (stage 0 in $box/stage0.sir, stage 1 in $box/stage1.sir)"
    fi
    if [ -n "$stage1_exe" ]; then
        (cd "$box" && env -u SISU_FRONT_END SISU_STAGE1="$stage1_exe" SISU_STAGE1_TRANSCRIPTS=1 \
            "$sisu" test --no-cache "$name" > test.out 2> test.err)
    else
        (cd "$box" && env -u SISU_FRONT_END SISU_STAGE1_TRANSCRIPTS=1 \
            "$sisu" test --no-cache "$name" > test.out 2> test.err)
    fi
    if cmp -s "${f%.sisu}.test" "$box/test.out"; then
        same=$((same + 1))
    else
        class_unlisted=$((class_unlisted + 1))
        report="$report\n  UNLISTED: $rel: $class (sisu test on stage 1 in $box/test.out against ${name%.sisu}.test)"
    fi
done < "$list"
end=$(date +%s.%N)
seconds=$(awk "BEGIN { printf \"%.1f\", $end - $start }")
n=$((2 * $(wc -l < "$list" | tr -d ' ')))
total=$((total + n)); identical=$((identical + same))
listed=$((listed + class_listed)); unlisted=$((unlisted + class_unlisted))
transcripts_summary="$(printf '  %-24s %4d runs,  %4d identical, %3d differing as listed, %3d otherwise; %6ss' \
    "$class" "$n" "$same" "$class_listed" "$class_unlisted" "$seconds")"

# The behaviour of stage 1's SIR, the primary oracle since the Rust front
# end was frozen (RULES.md): every runnable program of the corpus and of
# stage 1's own cases (one with an `.out`, `.err` or `.status`, as the
# conformance runner has it), in a sandbox as the runner makes one (the
# file and what its `.files/` holds, or a package case's directory, run in
# the one its `.cwd` names), lowered by stage 1 (one run of `sir-batch`
# over the sandboxes; the paths it writes into the SIR are the sandbox's,
# which the runner's `sisu sir NAME.sisu` writes as the bare name), and
# run with the case's `.args` and `.stdin` and an empty environment by
# stage 0's interpreter (`sisu run --from-sir`) and by its native backend
# (`sisu build --native --from-sir`, the executable run with
# `SISU_RT_ALLOCS=1`), each of which must give the case's `.out`, `.err`
# and `.status` (the warnings of a front end that warns come first in the
# `.err`, and a run from SIR does not print them) and, natively, leave no
# allocation live at exit (but for a run that aborts, 134).
behaviour_box() {
    # behaviour_box K PROGRAM DIR: the sandbox of the Kth program, and the
    # path stage 1 lowers there.
    local k="$1" p="$2" box="$3/$1" base pkg="" sub=""
    mkdir -p "$box/run"
    case "$p" in
        *.pkg | *.pkg/*)
            pkg="${p%%.pkg*}.pkg"
            sub="${p#"$pkg"}"
            base="${pkg%.pkg}"
            cp -r "$pkg/." "$box/run/"
            echo "$box/run$sub" > "$box/program"
            ;;
        *)
            base="${p%.sisu}"
            cp "$p" "$box/run/"
            [ -d "$base.files" ] && cp -r "$base.files/." "$box/run/"
            echo "$box/run/$(basename "$p")" > "$box/program"
            ;;
    esac
    echo "$base" > "$box/base"
}
behaviour_one() {
    # behaviour_one K PROGRAM DIR: the Kth program's runs, and what of them
    # differs from the case, into DIR/K/result.
    local k="$1" box="$3/$1" base cwd program args=() stdin=/dev/null
    base=$(cat "$box/base")
    program=$(cat "$box/program")
    cwd="$program"
    [ -d "$cwd" ] || cwd=$(dirname "$program")
    # The sandbox's paths, which the runner's run in it names relatively.
    local dir
    dir=$(printf '%s' "$cwd" | sed 's/[.[\*^$|]/\\&/g')
    sed "s|$dir/||g" "$work/behaviour.sir/$k.sir" > "$box/program.sir" 2> /dev/null
    [ -f "$base.args" ] && mapfile -t args < "$base.args"
    [ -f "$base.stdin" ] && stdin="$base.stdin"
    local out="$box/expected.out" err="$box/expected.err" status=0
    if [ -f "$base.out" ]; then cp "$base.out" "$out"; else : > "$out"; fi
    if [ -f "$base.err" ]; then cp "$base.err" "$err"; else : > "$err"; fi
    [ -f "$base.status" ] && status=$(tr -d ' \n' < "$base.status")
    # A rule of RULES.md that changes what a case prints (its output
    # `behaviour`) has the case's file as the rule has it under stage 1's
    # `expected/`, at the case's path from the compiler's directory, which
    # the run is held to instead (and which the coordinator copies into the
    # suite).
    local override="$compiler/stage1/expected/${base#"$compiler"/}"
    if [ -f "$override.out" ]; then cp "$override.out" "$out"; echo out >> "$box/overridden"; fi
    if [ -f "$override.err" ]; then cp "$override.err" "$err"; echo err >> "$box/overridden"; fi
    if [ -f "$override.status" ]; then
        status=$(tr -d ' \n' < "$override.status")
        echo status >> "$box/overridden"
    fi
    echo "$status" > "$box/expected.status"
    if [ ! -s "$work/behaviour.sir/$k.sir" ]; then
        echo "stage 1 printed no SIR" > "$box/result"
        return 0
    fi
    local failed=""
    # same_err ERR: the run's standard error is the case's, or what follows
    # the front end's warnings in it (each a block ending in a blank line).
    same_err() {
        cmp -s "$err" "$1" && return 0
        local n e
        n=$(wc -c < "$1" | tr -d ' ')
        e=$(wc -c < "$err" | tr -d ' ')
        [ "$e" -gt "$n" ] || return 1
        cmp -s <(tail -c "$n" "$err") "$1" || return 1
        head -c $((e - n)) "$err" > "$box/warnings"
        head -c 8 "$box/warnings" | grep -q '^warning\[' && [ "$(tail -c 2 "$box/warnings" | od -An -c | tr -d ' ')" = '\n\n' ]
    }
    (cd "$cwd" && timeout 60 env -i "$sisu" run --from-sir "$box/program.sir" "${args[@]}" \
        < "$stdin" > "$box/i.out" 2> "$box/i.err"; echo "$?" > "$box/i.status")
    cmp -s "$out" "$box/i.out" || failed="$failed interpreted:stdout"
    [ "$(cat "$box/i.status")" = "$status" ] || failed="$failed interpreted:status"
    same_err "$box/i.err" || failed="$failed interpreted:stderr"
    # A case of the conformance suite runs natively when its native list
    # names it (conformance/native.list): the others expect what only the
    # interpreter does, such as the checked memory model's reports (D-085).
    local native_case=1
    case "$base" in
        "$compiler/conformance/cases/"*)
            grep -qxF -e "${base#"$compiler/conformance/cases/"}" "$compiler/conformance/native.list" || native_case=0
            ;;
    esac
    echo "$native_case" > "$box/native"
    if [ "$native_case" = 0 ]; then
        :
    elif ! (cd "$cwd" && timeout 120 "$sisu" build --native --from-sir "$box/program.sir" --output "$box/exe" \
            > "$box/build.log" 2>&1); then
        failed="$failed native:build"
    else
        (cd "$cwd" && timeout 60 env -i SISU_RT_ALLOCS=1 "$box/exe" "${args[@]}" \
            < "$stdin" > "$box/n.out" 2> "$box/n.all"; echo "$?" > "$box/n.status")
        local last ran
        ran=$(cat "$box/n.status")
        last=$(tail -n 1 "$box/n.all")
        case "$last" in
            "allocations: "*" of the runtime's live at exit")
                sed '$d' "$box/n.all" > "$box/n.err"
                [ "$last" = "allocations: 0 of the runtime's live at exit" ] || failed="$failed native:allocations"
                ;;
            *)
                cp "$box/n.all" "$box/n.err"
                [ "$ran" = 134 ] || failed="$failed native:allocations"
                ;;
        esac
        cmp -s "$out" "$box/n.out" || failed="$failed native:stdout"
        [ "$ran" = "$status" ] || failed="$failed native:status"
        same_err "$box/n.err" || failed="$failed native:stderr"
    fi
    # An executable is kept only to look into a difference.
    [ -z "$failed" ] && rm -f "$box/exe"
    echo "${failed# }" > "$box/result"
}
export -f behaviour_box behaviour_one
{
    find "$compiler/conformance/cases" "$here/cases" -name '*.sisu' | grep -v '\.pkg/' | grep -v '\.files/' | grep -v "^$here/cases/repl/" | sort |
        while read -r f; do
            b="${f%.sisu}"
            if [ -e "$b.out" ] || [ -e "$b.err" ] || [ -e "$b.status" ]; then echo "$f"; fi
        done
    find "$compiler/conformance/cases" "$here/cases" -name '*.pkg' -type d -prune | sort | while read -r d; do
        b="${d%.pkg}"
        if [ -e "$b.out" ] || [ -e "$b.err" ] || [ -e "$b.status" ]; then
            if [ -f "$b.cwd" ]; then echo "$d/$(cat "$b.cwd")"; else echo "$d"; fi
        fi
    done
} > "$work/behaviour.list"
class=behaviour
list="$work/$class.list"
n=$(wc -l < "$list" | tr -d ' ')
mkdir -p "$work/behaviour" "$work/behaviour.sir"
parallel behaviour_box "$list" "$work/behaviour"
for k in $(seq 1 "$n"); do cat "$work/behaviour/$k/program"; done > "$work/behaviour.sandboxes"
start=$(date +%s.%N)
if ! stage1 sir-batch "$work/behaviour.sir" "$work/behaviour.sandboxes" > "$work/$class.log" 2> "$work/$class.stderr"; then
    cat "$work/$class.stderr" >&2
    echo "check.sh: stage 1 failed lowering the $class programs" >&2
    exit 1
fi
# A batch makes one base, of the runtime profile `full`; a package of
# another profile (its manifest's `runtime`, RULES.md R122) is lowered by
# the single command in its directory, as the driver lowers it (R141).
for k in $(seq 1 "$n"); do
    prog=$(sed -n "${k}p" "$work/behaviour.sandboxes")
    if [ -d "$prog" ] && [ -f "$prog/sisu.toml" ] &&
        grep -Eq '^runtime *= *"(single|freestanding)"' "$prog/sisu.toml"; then
        stage1 sir "$prog" > "$work/behaviour.sir/$k.sir" 2> /dev/null
    fi
done
middle=$(date +%s.%N)
parallel behaviour_one "$list" "$work/behaviour"
end=$(date +%s.%N)
took_sir=$(awk "BEGIN { printf \"%.1f\", $middle - $start }")
seconds=$(awk "BEGIN { printf \"%.1f\", $end - $middle }")
same=0; class_listed=0; class_unlisted=0; natively=0
i=0
while read -r f; do
    i=$((i + 1))
    rel="${f#"$repo"/}"
    result=$(cat "$work/behaviour/$i/result" 2> /dev/null || echo "not run")
    # A case whose expectation a rule changes, held to stage 1's `expected/`
    # (RULES.md lists its output `behaviour`).
    if [ -z "$result" ] && [ -f "$work/behaviour/$i/overridden" ]; then
        [ "$(cat "$work/behaviour/$i/native" 2> /dev/null)" = 1 ] && natively=$((natively + 1))
        if grep -qxF -e "$rel behaviour" "$allowed"; then
            class_listed=$((class_listed + 1))
            report="$report\n  listed:   $rel: behaviour (as the rule has the case)"
        else
            class_unlisted=$((class_unlisted + 1))
            report="$report\n  UNLISTED: $rel: behaviour: held to stage 1's expected/, which RULES.md does not list"
        fi
        continue
    fi
    if [ -z "$result" ]; then
        same=$((same + 1))
        [ "$(cat "$work/behaviour/$i/native" 2> /dev/null)" = 1 ] && natively=$((natively + 1))
        continue
    fi
    # A program a rule of RULES.md refuses (a line "path: behaviour"), whose
    # case still expects the run of the frozen Rust front end: stage 1
    # lowers nothing, as its `ids` output, also listed, says why.
    if [ "$result" = "stage 1 printed no SIR" ] && grep -qxF -e "$rel behaviour" "$allowed"; then
        class_listed=$((class_listed + 1))
        report="$report\n  listed:   $rel: behaviour (refused by a rule)"
        continue
    fi
    # A case that expects its errors to stop the run (decision 427: its
    # `.err` ends with `D0006` or `D0021`, its status 1): stage 1 lowers
    # nothing, and what it prints of the run, the errors and that line
    # (`build --for-run`, which `sisu run` shows, RULES.md R57), must be the
    # case's `.err`.
    box="$work/behaviour/$i"
    if [ "$result" = "stage 1 printed no SIR" ] && [ -f "$box/expected.err" ] &&
        tail -n 1 "$box/expected.err" | grep -q '^error\[D00\(06\|21\)\]: the program was not run' &&
        [ "$(tr -d ' \n' < "$box/expected.status" 2> /dev/null || echo 1)" = 1 ]; then
        program=$(cat "$box/program")
        cwd="$program"
        [ -d "$cwd" ] || cwd=$(dirname "$program")
        stage1 build --for-run "$program" > "$box/stopped.out" 2> "$box/stopped.err"
        echo "$?" > "$box/stopped.status"
        # The line a Sisu program's `main` ends with when it fails, which
        # the driver takes off (`error: Failed`).
        sed -i -e "s|$(printf '%s' "$cwd" | sed 's/[.[\*^$|]/\\&/g')/||g" -e '${/^error: Failed$/d}' "$box/stopped.err"
        if cmp -s "$box/stopped.err" "$box/expected.err" && [ ! -s "$box/stopped.out" ] &&
            [ "$(cat "$box/stopped.status")" = 1 ]; then
            same=$((same + 1))
            continue
        fi
        result="stopped by errors, as the case expects, but stage 1 printed otherwise (stopped.err)"
    fi
    class_unlisted=$((class_unlisted + 1))
    report="$report\n  UNLISTED: $rel: behaviour: $result (in $work/behaviour/$i)"
done < "$list"
total=$((total + n)); identical=$((identical + same)); unlisted=$((unlisted + class_unlisted))
listed=$((listed + class_listed))
behaviour_summary="$(printf '  %-12s %4d programs, %4d as expected interpreted (%d of them natively too), %3d refused or changed as listed, %3d otherwise; stage 1 %6ss, the runs %6ss' \
    "$class" "$n" "$same" "$natively" "$class_listed" "$class_unlisted" "$took_sir" "$seconds")"

# What `check` prints: every diagnostic of a program, in the conformance
# form (the whole of `sisu check --format=ids`) and in the human form (what
# `sisu check` prints on standard error), against stage 1's `check-batch`.
find "$compiler/conformance/cases/wrong" -name '*.sisu' | sort > "$work/human.list"
grep -v '/packages/' "$work/cases.list" > "$work/cases-check.list"
check_classes="human cases-check"
if [ "$all_check" = 1 ]; then
    find "$compiler/conformance/cases" -name '*.sisu' | grep -v '/packages/' | grep -v '\.pkg/' | sort > "$work/corpus-check.list"
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
            if grep -qxF -e "$rel $output" "$allowed"; then
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

# Stage 1's own cases against what they expect: each with a `NAME.diag`
# beside it (or, for a package case `NAME.pkg`, beside the directory), the
# whole of what stage 1's `check --format=ids` printed of it above.
n=0; same=0; class_unlisted=0
for class in cases-check corpus-check; do
    list="$work/$class.list"
    [ -f "$list" ] || continue
    i=0
    while read -r f; do
        i=$((i + 1))
        case "$f" in "$here/cases/"*) ;; *) continue ;; esac
        base="${f%.sisu}"
        case "$base" in *.pkg/*) base="${base%%.pkg/*}" ;; *.pkg) base="${base%.pkg}" ;; esac
        [ -f "$base.diag" ] || continue
        n=$((n + 1))
        rel="${f#"$repo"/}"
        if cmp -s "$base.diag" "$work/$class.stage1/$i.ids"; then
            same=$((same + 1))
        else
            class_unlisted=$((class_unlisted + 1))
            report="$report\n  UNLISTED: $rel: expected (the case's ${base##*/}.diag against stage 1 in $work/$class.stage1/$i.ids)"
        fi
    done < "$list"
done
# And what `audit --unsafe` prints of each case with a `NAME.audit` (its
# names are relative to the program, so it runs from here), against that
# file.
mkdir -p "$work/audit"
k=0
while read -r f; do
    base="${f%.sisu}"
    case "$base" in *.pkg/*) base="${base%%.pkg/*}" ;; *.pkg) base="${base%.pkg}" ;; esac
    [ -f "$base.audit" ] || continue
    k=$((k + 1)); n=$((n + 1))
    stage1 audit --unsafe "$f" > "$work/audit/$k.out" 2> /dev/null
    if cmp -s "$base.audit" "$work/audit/$k.out"; then
        same=$((same + 1))
    else
        class_unlisted=$((class_unlisted + 1))
        report="$report\n  UNLISTED: ${f#"$repo"/}: expected audit (the case's ${base##*/}.audit against stage 1 in $work/audit/$k.out)"
    fi
done < <(grep -F "$here/cases/" "$work/cases-check.list"; grep -F "$here/cases/" "$work/packages.list")
# And what `lint --format=ids` prints of each case with a `NAME.lint`
# (RULES.md, R11: `sisu lint`'s warnings), against that file.
mkdir -p "$work/lint"
k=0
while read -r f; do
    base="${f%.sisu}"
    case "$base" in *.pkg/*) base="${base%%.pkg/*}" ;; *.pkg) base="${base%.pkg}" ;; esac
    [ -f "$base.lint" ] || continue
    k=$((k + 1)); n=$((n + 1))
    stage1 lint --format=ids "$f" > "$work/lint/$k.out" 2> /dev/null
    if cmp -s "$base.lint" "$work/lint/$k.out"; then
        same=$((same + 1))
    else
        class_unlisted=$((class_unlisted + 1))
        report="$report\n  UNLISTED: ${f#"$repo"/}: expected lint (the case's ${base##*/}.lint against stage 1 in $work/lint/$k.out)"
    fi
done < <(grep -F "$here/cases/" "$work/cases-check.list"; grep -F "$here/cases/" "$work/packages.list")
# And what `sir` prints of each case with a `NAME.sir` (RULES.md, R17:
# the form of a call), its directory taken off the paths it names, as
# `sisu sir NAME.sisu` names them run there, against that file.
mkdir -p "$work/sir-expected"
k=0
while read -r f; do
    base="${f%.sisu}"
    [ -f "$base.sir" ] || continue
    k=$((k + 1)); n=$((n + 1))
    dir=$(dirname "$f" | sed 's/[.[\*^$|]/\\&/g')
    stage1 sir "$f" 2> /dev/null | sed "s|$dir/||g" > "$work/sir-expected/$k.out"
    if cmp -s "$base.sir" "$work/sir-expected/$k.out"; then
        same=$((same + 1))
    else
        class_unlisted=$((class_unlisted + 1))
        report="$report\n  UNLISTED: ${f#"$repo"/}: expected sir (the case's ${base##*/}.sir against stage 1 in $work/sir-expected/$k.out)"
    fi
done < <(grep -F "$here/cases/" "$work/cases-check.list")
# And what `sir --tests` prints of each case with a `NAME.sir-tests`
# (RULES.md, R38: what a build with its tests lowers `expect` to), the same
# way, followed, when there is any, by `--- stderr` and its standard error,
# whose last line says the status (R46: a program with an error of a phase
# has no SIR, and its diagnostics say why it is not run).
mkdir -p "$work/sir-tests-expected"
k=0
while read -r f; do
    base="${f%.sisu}"
    [ -f "$base.sir-tests" ] || continue
    k=$((k + 1)); n=$((n + 1))
    dir=$(dirname "$f" | sed 's/[.[\*^$|]/\\&/g')
    given="$work/sir-tests-expected/$k"
    stage1 sir --tests "$f" > "$given.sir" 2> "$given.err"
    {
        cat "$given.sir"
        if [ -s "$given.err" ]; then
            echo "--- stderr"
            cat "$given.err"
        fi
    } | sed "s|$dir/||g" > "$given.out"
    if cmp -s "$base.sir-tests" "$work/sir-tests-expected/$k.out"; then
        same=$((same + 1))
    else
        class_unlisted=$((class_unlisted + 1))
        report="$report\n  UNLISTED: ${f#"$repo"/}: expected sir-tests (the case's ${base##*/}.sir-tests against stage 1 in $work/sir-tests-expected/$k.out)"
    fi
done < <(grep -F "$here/cases/" "$work/cases-check.list")
# And what `build --native --release` prints of each case with a
# `NAME.release` (RULES.md, R27: a release build rejects typed holes), its
# directory taken off the paths it names, against that file.
mkdir -p "$work/release-expected"
k=0
while read -r f; do
    base="${f%.sisu}"
    [ -f "$base.release" ] || continue
    k=$((k + 1)); n=$((n + 1))
    dir=$(dirname "$f" | sed 's/[.[\*^$|]/\\&/g')
    stage1 build --native --release "$f" --output "$work/release-expected/$k.exe" 2>&1 > /dev/null |
        sed "s|$dir/||g" > "$work/release-expected/$k.out"
    if cmp -s "$base.release" "$work/release-expected/$k.out"; then
        same=$((same + 1))
    else
        class_unlisted=$((class_unlisted + 1))
        report="$report\n  UNLISTED: ${f#"$repo"/}: expected release (the case's ${base##*/}.release against stage 1 in $work/release-expected/$k.out)"
    fi
done < <(grep -F "$here/cases/" "$work/cases-check.list")
# And what `test` prints of each case with a `NAME.tests` (RULES.md, R27:
# the closure of a blocked test's hole, and the status of a run with one),
# a copy run in a directory of its own: standard output, then `---
# stderr` and standard error, whose last line says the status, with the
# directory taken off the paths and the hash of a replay token, which
# hashes the SIR that names the copy's path, written `HASH`.
mkdir -p "$work/tests-expected"
k=0
# (A package case, `NAME.pkg` with its `NAME.pkg.tests`, is copied whole
# and tested in its directory: RULES.md, R52.)
while read -r f; do
    base="${f%.sisu}"
    [ -f "$base.tests" ] || continue
    k=$((k + 1)); n=$((n + 1))
    box="$work/tests-expected/$k"
    mkdir -p "$box"
    cp -r "$f" "$box/"
    name=$(basename "$f")
    stage1 test "$box/$name" > "$box/out" 2> "$box/err"
    { cat "$box/out"; echo "--- stderr"; cat "$box/err"; } | sed "s|$box/||g" |
        sed -E 's/(--replay [0-9]+-)[0-9a-f]+-/\1HASH-/' > "$box/all"
    if cmp -s "$base.tests" "$box/all"; then
        same=$((same + 1))
    else
        class_unlisted=$((class_unlisted + 1))
        report="$report\n  UNLISTED: ${f#"$repo"/}: expected tests (the case's ${base##*/}.tests against stage 1 in $box/all)"
    fi
done < <(grep -F "$here/cases/" "$work/cases-check.list"; grep -F "$here/cases/packages/" "$work/packages.list")
# And the lines before the first declaration of what `sir --tests
# --modules` prints of each case with a `NAME.sir-modules` (RULES.md, R142:
# each test's and benchmark's module, which the driver reads), a package
# case's from its directory as `NAME.pkg.sir-modules`, with the directory
# taken off the paths.
mkdir -p "$work/sir-modules-expected"
k=0
while read -r f; do
    base="${f%.sisu}"
    [ -f "$base.sir-modules" ] || continue
    k=$((k + 1)); n=$((n + 1))
    dir=$(dirname "$f" | sed 's/[.[\*^$|]/\\&/g')
    given="$work/sir-modules-expected/$k"
    had_lock=0
    [ -f "$f/sisu.lock" ] && had_lock=1
    stage1 sir --tests --modules "$f" 2> /dev/null | sed '/^$/q' | sed "s|$dir/||g" > "$given.out"
    [ "$had_lock" = 1 ] || rm -f "$f/sisu.lock"
    if cmp -s "$base.sir-modules" "$given.out"; then
        same=$((same + 1))
    else
        class_unlisted=$((class_unlisted + 1))
        report="$report\n  UNLISTED: ${f#"$repo"/}: expected sir-modules (the case's ${base##*/}.sir-modules against stage 1 in $given.out)"
    fi
done < <(grep -F "$here/cases/" "$work/cases-check.list"; grep -F "$here/cases/packages/" "$work/packages.list")
total=$((total + n)); identical=$((identical + same)); unlisted=$((unlisted + class_unlisted))
check_summary="$check_summary\n$(printf '  %-12s %4d files, %4d as their .diag, .audit, .lint, .sir, .sir-tests, .sir-modules, .release or .tests expects, %3d otherwise' expected "$n" "$same" "$class_unlisted")"

# The formatter: the text `sisu fmt` leaves of each file, what `sisu fmt
# --check` prints of it and its status, against stage 1's `fmt-batch`; and
# stage 1's text formatted again, which must be itself.
find "$here/src" -name '*.sisu' | sort > "$work/sources.list"
cat "$work/conformance.list" "$work/std.list" "$work/examples.list" "$work/sources.list" \
    "$work/cases.list" > "$work/fmt.list"
fmt_summary=""
class=fmt
list="$work/$class.list"
n=$(wc -l < "$list" | tr -d ' ')
ref="$work/$class.stage0"; out="$work/$class.stage1"
mkdir -p "$ref" "$out"
parallel stage0_fmt "$list" "$ref"
start=$(date +%s.%N)
if ! stage1 fmt-batch "$out" "$list" > "$work/$class.log" 2> "$work/$class.stderr"; then
    cat "$work/$class.stderr" >&2
    echo "check.sh: stage 1 failed formatting the $class files" >&2
    exit 1
fi
end=$(date +%s.%N)
seconds=$(awk "BEGIN { printf \"%.1f\", $end - $start }")
for output in fmt fmt-check; do
    same=0; class_listed=0; class_unlisted=0
    i=0
    while read -r f; do
        i=$((i + 1))
        rel="${f#"$repo"/}"
        if [ "$output" = fmt ]; then
            cmp -s "$ref/$i.fmt" "$out/$i.fmt" && { same=$((same + 1)); continue; }
            where="stage 0 in $ref/$i.fmt, stage 1 in $out/$i.fmt"
        else
            cmp -s "$ref/$i.check.out" "$out/$i.check.out" && cmp -s "$ref/$i.check.err" "$out/$i.check.err" &&
                cmp -s "$ref/$i.check.status" "$out/$i.check.status" && { same=$((same + 1)); continue; }
            where="stage 0 in $ref/$i.check.*, stage 1 in $out/$i.check.*"
        fi
        if grep -qxF -e "$rel $output" "$allowed"; then
            class_listed=$((class_listed + 1))
            report="$report\n  listed:   $rel: $output"
        else
            class_unlisted=$((class_unlisted + 1))
            report="$report\n  UNLISTED: $rel: $output ($where)"
        fi
    done < "$list"
    total=$((total + n)); identical=$((identical + same))
    listed=$((listed + class_listed)); unlisted=$((unlisted + class_unlisted))
    line="$(printf '  %-12s %4d files, %4d identical, %3d differing as listed, %3d otherwise' \
        "$output" "$n" "$same" "$class_listed" "$class_unlisted")"
    if [ "$output" = fmt ]; then
        fmt_summary="$fmt_summary\n$line; stage 1 $(printf '%6s' "$seconds")s"
    else
        fmt_summary="$fmt_summary\n$line (the same run)"
    fi
done
# The fixed point: stage 1's text, formatted again, is itself.
same=0; differing=0
i=0
while read -r f; do
    i=$((i + 1))
    if [ -s "$out/$i.again" ]; then
        differing=$((differing + 1))
        report="$report\n  UNLISTED: ${f#"$repo"/}: fmt-again (stage 1 in $out/$i.again)"
    else
        same=$((same + 1))
    fi
done < "$list"
total=$((total + n)); identical=$((identical + same)); unlisted=$((unlisted + differing))
fmt_summary="$fmt_summary\n$(printf '  %-12s %4d files, %4d unchanged, %3d otherwise (the same run)' \
    fmt-again "$n" "$same" "$differing")"
# The same files laid out otherwise, which the formatters must break and
# join again: a file's text, its verdict and stage 1's fixed point at once.
# A difference is a listed discrepancy's when it has its mark (a test of
# the outputs that only that discrepancy makes). None is listed now (D6
# and D7, the formatter's, were decided by decision 267). A deformed copy
# of a file whose `fmt-check` differs as RULES.md lists (one that stage
# 0's parser refuses, written in a rule's new syntax) differs as listed.
fmt_explained() {
    local orig
    orig=$(sed -n "${4}p" "$work/fmt.list")
    grep -qxF -e "${orig#"$repo"/} fmt-check" "$allowed"
}
others=""
if [ "$native" = 1 ]; then
    others="fmt-joined fmt-spread"
    mkdir -p "$work/deformed"
    if ! stage1 deform "$work/deformed" "$list" > "$work/deform.log" 2>&1; then
        cat "$work/deform.log" >&2
        echo "check.sh: stage 1 could not deform the fmt files" >&2
        exit 1
    fi
    for k in $(seq 1 "$n"); do echo "$work/deformed/j$k.sisu"; done > "$work/fmt-joined.list"
    for k in $(seq 1 "$n"); do echo "$work/deformed/s$k.sisu"; done > "$work/fmt-spread.list"
fi
if [ "$mutants" -gt 0 ]; then
    others="$others fmt-mutants"
    cp "$work/mutants.list" "$work/fmt-mutants.list"
fi
for class in $others; do
    list="$work/$class.list"
    n=$(wc -l < "$list" | tr -d ' ')
    ref="$work/$class.stage0"; out="$work/$class.stage1"
    mkdir -p "$ref" "$out"
    parallel stage0_fmt "$list" "$ref"
    start=$(date +%s.%N)
    if ! stage1 fmt-batch "$out" "$list" > "$work/$class.log" 2> "$work/$class.stderr"; then
        cat "$work/$class.stderr" >&2
        echo "check.sh: stage 1 failed formatting the $class files" >&2
        exit 1
    fi
    end=$(date +%s.%N)
    seconds=$(awk "BEGIN { printf \"%.1f\", $end - $start }")
    same=0; class_listed=0; class_unlisted=0
    i=0
    while read -r f; do
        i=$((i + 1))
        if cmp -s "$ref/$i.fmt" "$out/$i.fmt" && cmp -s "$ref/$i.check.out" "$out/$i.check.out" &&
            cmp -s "$ref/$i.check.err" "$out/$i.check.err" &&
            cmp -s "$ref/$i.check.status" "$out/$i.check.status" && [ ! -s "$out/$i.again" ]; then
            same=$((same + 1))
        elif [ ! -s "$out/$i.again" ] && fmt_explained "$ref/$i" "$out/$i" "$f" "$i"; then
            class_listed=$((class_listed + 1))
            report="$report\n  listed:   $class: $f"
        else
            class_unlisted=$((class_unlisted + 1))
            report="$report\n  UNLISTED: $class: $f (stage 0 in $ref/$i.*, stage 1 in $out/$i.*)"
        fi
    done < "$list"
    total=$((total + n)); identical=$((identical + same))
    listed=$((listed + class_listed)); unlisted=$((unlisted + class_unlisted))
    fmt_summary="$fmt_summary\n$(printf '  %-12s %4d files, %4d identical, %3d differing as listed, %3d otherwise; stage 1 %6ss' \
        "$class" "$n" "$same" "$class_listed" "$class_unlisted" "$seconds")"
done

# The documentation (decision 258): what `sisu doc` prints of each program
# of the corpus, of stage 1's own cases and, with --self, of stage 1's own
# package, with its status, and the pages `sisu doc --output DIR` writes,
# against stage 1's `doc-batch`; then the same for the standard library's
# pages (`--std`, all of them and some) and for arguments stage 0 refuses.
doc_summary=""
class=doc
list="$work/doc.list"
find "$compiler/conformance/cases" -name '*.sisu' | grep -v '/packages/' | grep -v '\.pkg/' | sort > "$list"
cat "$work/examples.list" "$work/packages.list" >> "$list"
grep -v '/packages/' "$work/cases.list" >> "$list"
[ "$self_check" = 1 ] && echo "$here" >> "$list"
programs=$(wc -l < "$list" | tr -d ' ')
ref="$work/$class.stage0"; out="$work/$class.stage1"
mkdir -p "$ref" "$out"
parallel stage0_doc "$list" "$ref"
# The arguments of a `doc` that names no program, run in this directory.
i=$programs
while read -r line; do
    i=$((i + 1))
    echo "$line" >> "$list"
    # shellcheck disable=SC2086
    (cd "$here" && "$sisu" doc $line > "$ref/$i.doc.out" 2> "$ref/$i.doc.err"; echo "$?" > "$ref/$i.doc.status"
     "$sisu" doc $line --output "$ref/$i.pages" > "$ref/$i.pages.out" 2> "$ref/$i.pages.err"
     echo "$?" > "$ref/$i.pages.status")
done <<'ARGS'
--std
--std std.fmt std.string std.time std.collections
--std std.core
--std std.json std.nothing
--bogus
ARGS
n=$(wc -l < "$list" | tr -d ' ')
start=$(date +%s.%N)
if ! stage1 doc-batch "$out" "$list" > "$work/$class.log" 2> "$work/$class.stderr"; then
    cat "$work/$class.stderr" >&2
    echo "check.sh: stage 1 failed documenting the $class programs" >&2
    exit 1
fi
end=$(date +%s.%N)
seconds=$(awk "BEGIN { printf \"%.1f\", $end - $start }")
for output in doc doc-output; do
    same=0; class_listed=0; class_unlisted=0
    i=0
    while read -r f; do
        i=$((i + 1))
        rel="${f#"$repo"/}"
        if [ "$output" = doc ]; then
            cmp -s "$ref/$i.doc.out" "$out/$i.doc.out" && cmp -s "$ref/$i.doc.err" "$out/$i.doc.err" &&
                cmp -s "$ref/$i.doc.status" "$out/$i.doc.status" && { same=$((same + 1)); continue; }
            where="stage 0 in $ref/$i.doc.*, stage 1 in $out/$i.doc.*"
        else
            # What `--output` writes: the same pages, nothing printed but
            # what stops it, and the same status.
            if [ ! -s "$ref/$i.pages.out" ] && cmp -s "$ref/$i.pages.err" "$out/$i.pages.err" &&
                cmp -s "$ref/$i.pages.status" "$out/$i.pages.status" &&
                diff -r "$ref/$i.pages" "$out/$i.pages" > /dev/null 2>&1; then
                same=$((same + 1)); continue
            fi
            if [ ! -e "$ref/$i.pages" ] && [ ! -e "$out/$i.pages" ] && [ ! -s "$ref/$i.pages.out" ] &&
                cmp -s "$ref/$i.pages.err" "$out/$i.pages.err" &&
                cmp -s "$ref/$i.pages.status" "$out/$i.pages.status"; then
                same=$((same + 1)); continue
            fi
            where="stage 0 in $ref/$i.pages*, stage 1 in $out/$i.pages*"
        fi
        if grep -qxF -e "$rel $output" "$allowed"; then
            class_listed=$((class_listed + 1))
            report="$report\n  listed:   $rel: $output"
            continue
        fi
        class_unlisted=$((class_unlisted + 1))
        report="$report\n  UNLISTED: $rel: $output ($where)"
    done < "$list"
    total=$((total + n)); identical=$((identical + same))
    listed=$((listed + class_listed)); unlisted=$((unlisted + class_unlisted))
    line="$(printf '  %-12s %4d programs, %4d identical, %3d differing as listed, %3d otherwise' \
        "$output" "$n" "$same" "$class_listed" "$class_unlisted")"
    if [ "$output" = doc ]; then
        doc_summary="$doc_summary\n$line; stage 1 $(printf '%6s' "$seconds")s"
    else
        doc_summary="$doc_summary\n$line (the same run)"
    fi
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
    builds=0; same=0; build_listed=0
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
        # The case a build of a file is of, for a difference RULES.md lists
        # (its output `build`, R124: an executable whose SIR a rule changes).
        case_of=""
        case "$path" in
            hello.sisu) case_of="compiler/conformance/cases/examples/hello.sisu" ;;
            checks.sisu) case_of="compiler/conformance/cases/contracts/checks.sisu" ;;
        esac
        if [ -z "$differs" ]; then
            same=$((same + 1))
        elif [ -n "$case_of" ] && [ "$differs" = " ${path%.sisu}" ] && grep -qxF -e "$case_of build" "$allowed"; then
            listed=$((listed + 1)); build_listed=$((build_listed + 1))
            report="$report\n  listed:   build: $what:$differs"
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
    build_summary="$(printf '  %-12s %4d builds, %4d identical, %3d differing as listed, %3d otherwise; both %6ss' build "$builds" "$same" "$build_listed" $((builds - same - build_listed)) "$seconds")"
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
    n=0; same=0; beyond=0; differing=0; self_listed=0
    for f in $( (ls "$work/self.stage0"; ls "$work/self.stage1") | sort -u); do
        n=$((n + 1))
        shown=$(printf '%s' "$f" | tr '%' '/')
        if cmp -s "$work/self.stage0/$f" "$work/self.stage1/$f"; then
            same=$((same + 1))
        elif grep -qxF -e "compiler/stage1/$shown self" "$allowed"; then
            # A module whose SIR a rule changes (RULES.md, R124: "- Differs:
            # `compiler/stage1/src/...`: self").
            self_listed=$((self_listed + 1))
            report="$report\n  listed:   self: $shown"
        elif [ -n "$why" ]; then
            beyond=$((beyond + 1))
            report="$report\n  beyond:   self: $shown (stage 1 is beyond the slice of lowering: $why)"
        else
            differing=$((differing + 1))
            report="$report\n  UNLISTED: self: $shown (stage 0 in $work/self.stage0/$f, stage 1 in $work/self.stage1/$f)"
        fi
    done
    total=$((total + n)); identical=$((identical + same)); lowered_beyond=$((lowered_beyond + beyond))
    unlisted=$((unlisted + differing)); listed=$((listed + self_listed))
    self_summary="$self_summary\n$(printf '  %-12s %4d files, %4d identical, %3d differing as listed, %3d beyond the slice, %3d otherwise; stage 1 %6ss' \
        self-sir "$n" "$same" "$self_listed" "$beyond" "$differing" "$seconds")"
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
        elif [ -s "$2" ] && grep -qxF -e "compiler/stage1 $1" "$allowed"; then
            # Stage 0's SIR of the package, which decides nothing of the
            # fixed point (RULES.md, R124: "- Differs: `compiler/stage1`:
            # fixed-sir0").
            listed=$((listed + 1))
            report="$report\n  listed:   fixed: $1, $4"
            fixed_summary="$fixed_summary\n$(printf '  %-12s differing as listed: %s' "$1" "$4")"
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

# What `sisu repl` and `sisu ws` run (RULES.md, R79).
repl_summary=""
repl_session() {
    # repl_session DIR OUT: the session of DIR's inputs, transcribed to OUT.
    local dir="$1" transcript="$2" k=1 at
    : > "$work/repl.session"; : > "$work/repl.prev"; : > "$transcript"
    local steps=()
    [ -f "$dir/load.sisu" ] && steps+=(load)
    while [ -f "$dir/$k.in" ]; do steps+=("$k"); k=$((k + 1)); done
    for step in "${steps[@]}"; do
        echo "## $step" >> "$transcript"
        if [ "$step" = load ]; then
            set -- sir --repl "$work/repl.session" "${dir#"$here"/}/load.sisu" --kept "$work/repl.next" --load
        else
            set -- sir --repl "$work/repl.session" "$dir/$step.in" --kept "$work/repl.next"
        fi
        if stage1 "$@" > "$work/repl.sir" 2> "$work/repl.err"; then
            at=$(($(wc -c < "$work/repl.prev") + 1))
            if (cd "$dir" && "$sisu" run --from-sir "$work/repl.sir") > "$work/repl.run" 2> "$work/repl.run.err"; then
                tail -c +"$at" "$work/repl.run" >> "$transcript"
                cp "$work/repl.next" "$work/repl.session"
                cp "$work/repl.run" "$work/repl.prev"
            else
                tail -c +"$at" "$work/repl.run" >> "$transcript"
                cat "$work/repl.run.err" >> "$transcript"
                echo "(not kept)" >> "$transcript"
            fi
        else
            grep -vx 'error: Failed' "$work/repl.err" >> "$transcript"
        fi
    done
}
ws_render() {
    # ws_render SOURCE RUN: the worksheet SOURCE with the values and output
    # of its run RUN, as the driver writes them.
    awk -v src="$1" '
        substr($0, 1, 1) == "\001" { cur = substr($0, 2) + 0; next }
        substr($0, 1, 1) == "\002" {
            rest = substr($0, 2); i = index(rest, " ")
            n = (i ? substr(rest, 1, i - 1) : rest) + 0; v = (i ? substr(rest, i + 1) : "")
            if (n in vals) vals[n] = vals[n] ", " v; else vals[n] = v
            next
        }
        { outs[cur] = outs[cur] "# | " $0 "\n" }
        END {
            n = 0
            while ((getline line < src) > 0) {
                n++; printf "%s", line
                if (n in vals) printf "  # => %s", vals[n]
                printf "\n"
                if (n in outs) printf "%s", outs[n]
            }
        }' "$2"
}
same=0; class_listed=0; class_unlisted=0; n=0
for d in "$here"/cases/repl/*.repl; do
    [ -d "$d" ] || continue
    n=$((n + 1))
    rel="${d#"$repo"/}"
    repl_session "$d" "$work/repl.transcript"
    if cmp -s "$work/repl.transcript" "${d}.out"; then
        same=$((same + 1))
    else
        class_unlisted=$((class_unlisted + 1))
        cp "$work/repl.transcript" "$work/$(basename "$d").transcript"
        report="$report\n  UNLISTED: $rel: repl (expected ${d}.out, stage 1 in $work/$(basename "$d").transcript)"
    fi
done
total=$((total + n)); identical=$((identical + same)); unlisted=$((unlisted + class_unlisted))
repl_summary="$repl_summary\n$(printf '  %-24s %4d sessions, %4d as expected, %3d otherwise' repl "$n" "$same" "$class_unlisted")"
same=0; class_listed=0; class_unlisted=0; n=0
for f in "$here"/cases/repl/*.ws.sisu; do
    [ -f "$f" ] || continue
    n=$((n + 1))
    rel="${f#"$repo"/}"
    base="$work/ws.$(basename "$f")"
    (cd "$(dirname "$f")" && "$sisu" ws "$(basename "$f")" > "$base.stage0" 2>&1; echo "status $?" >> "$base.stage0")
    if (cd "$(dirname "$f")" && stage1 sir --worksheet "$f" > "$base.sir" 2> "$base.err"); then
        { (cd "$(dirname "$f")" && "$sisu" run --from-sir "$base.sir" > "$base.run" 2> "$base.run.err"; echo "status $?" > "$base.status")
          ws_render "$f" "$base.run"; cat "$base.run.err" "$base.status"; } > "$base.stage1"
    else
        { grep -vx 'error: Failed' "$base.err" | sed "s|$f|$(basename "$f")|g"; echo "status 1"; } > "$base.stage1"
    fi
    if cmp -s "$base.stage0" "$base.stage1"; then
        same=$((same + 1))
    elif grep -qxF -e "$rel ws" "$allowed"; then
        class_listed=$((class_listed + 1))
        report="$report\n  listed:   $rel: ws"
    else
        class_unlisted=$((class_unlisted + 1))
        report="$report\n  UNLISTED: $rel: ws (stage 0 in $base.stage0, stage 1 in $base.stage1)"
    fi
done
total=$((total + n)); identical=$((identical + same))
listed=$((listed + class_listed)); unlisted=$((unlisted + class_unlisted))
repl_summary="$repl_summary\n$(printf '  %-24s %4d worksheets, %4d identical, %3d differing as listed, %3d otherwise' ws "$n" "$same" "$class_listed" "$class_unlisted")"

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
echo "stage 1's lowering of the programs built with their tests against stage 0's (sir-tests):"
printf "%b\n" "$tests_summary"
echo "stage 1's SIR of the programs with transcripts against stage 0's, and their transcripts run from it (transcripts):"
printf "%b\n" "$transcripts_summary"
echo "stage 1's SIR run against the cases' expectations (behaviour: interpreted and native, from \`sisu sir\` as stage 1 prints it):"
printf "%b\n" "$behaviour_summary"
echo "stage 1's check against stage 0's (every diagnostic, in the human and the conformance form):"
printf "%b\n" "${check_summary#\\n}"
echo "stage 1's formatter against stage 0's (fmt: the text \`sisu fmt\` leaves, what \`sisu fmt --check\` prints, and its fixed point):"
printf "%b\n" "${fmt_summary#\\n}"
echo "stage 1's documentation against stage 0's (doc: what \`sisu doc\` prints and its status, and the pages \`--output\` writes):"
printf "%b\n" "${doc_summary#\\n}"
echo "stage 1's programs of \`sisu repl\` and \`sisu ws\` (repl against the cases' transcripts, ws against \`sisu ws\`):"
printf "%b\n" "${repl_summary#\\n}"
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
echo "total: $total files checked, $identical identical$beyond_note, $((listed + unlisted)) differing ($listed listed in RULES.md or DISCREPANCIES.md, $unlisted not)"
if [ "$unlisted" -gt 0 ]; then
    [ -z "$keep" ] && echo "(run with --keep DIR to inspect the outputs)"
    exit 1
fi
exit 0
