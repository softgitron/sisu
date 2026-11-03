# Glossary

This page defines each term that the manual uses for Sisu, in a
sentence or two, with the tutorial chapter that introduces it and the
reference page that states its rules. Terms are in alphabetical order,
and a term that names a keyword, a type or a function is listed under
its word, as `Shared` under S.

**See also:** [STYLE.md](../STYLE.md#terms), whose table of terms this
page follows; the proposal's [README](../../proposal/README.md) for the
documents that decide each feature.

## How to read this page

Each entry gives the meaning, then "Chapter N" for the chapter of the
tutorial that teaches the term and an R-page for the reference page
that states its rules. A term that the tutorial does not teach names
the page that introduces it instead. A term for something designed and
not in stage 0 says so; the reference page has its status.

**Status.** The terms are the proposal's and the manual's, in stage 0
unless an entry says otherwise.

## A

### Adapter

A method of an iterator that makes another iterator, such as `map`,
`filter` or `take`; it does its work only when an item is asked for.
[Chapter 16](../tutorial/16-closures-and-iterators.md).

### Alias

A second name for a type, `type Scores = dict[String, int]`, which is
the same type under either name. [Chapter 7](../tutorial/07-structs-and-enums.md);
[R2](types.md#type-aliases-and-newtypes).

### Ambient

Reachable without a capability: `print`, `print_err`, `log` and
`task.scope` are the ambient operations. [Chapter 18](../tutorial/18-capabilities.md);
[R10](capabilities.md#what-needs-no-capability).

### Arena

An `Arena[T]` owns the values of a graph, and a `Key[T]` names one of
them; a key whose value was removed is caught where it is used.
[Chapter 13](../tutorial/13-sharing.md); [R7](ownership.md#sharing-cells-and-graphs).

### Argument

A value passed in a call, by position or by name (a keyword argument,
`port=443`). [Chapter 3](../tutorial/03-functions.md);
[R4](expressions.md#calls).

### Array

`Array[T, N]`, a collection of `N` values of type `T`, whose length is
part of its type. [Chapter 5](../tutorial/05-collections.md);
[R2](types.md#collections).

### Assertion

`assert CONDITION, MESSAGE`, a check that panics when the condition is
false; in a test, its failure shows the values of the parts.
[Chapter 3](../tutorial/03-functions.md); [R15](testing.md#assertions).

### Associated function

A function declared in a type's body without `self`, called on the
type, as `Point.origin()`; Sisu has no static methods.
[Chapter 7](../tutorial/07-structs-and-enums.md); [R3](declarations.md#structs).

### Associated type

A type that a trait names and each impl chooses, as `type Item` of
`Iterator`. [Chapter 14](../tutorial/14-traits.md); [R3](declarations.md#traits).

### Attenuation

Making a narrower capability from a wider one, as `fs.restrict_to(dir)`
or `net.only_hosts(...)`; it never widens.
[Chapter 18](../tutorial/18-capabilities.md); [R10](capabilities.md#attenuation).

### Attribute

`@name` or `@name(args)` on the line before a declaration: compile-time
data that the compiler, the tools and compile-time code read, never a
call. [Chapter 7](../tutorial/07-structs-and-enums.md) and
[chapter 25](../tutorial/25-compile-time.md); [R14](attributes.md).

### Audit

`sisu audit`, which counts what a program shares, the capabilities it
passes around, or its unsafe code. [Chapter 13](../tutorial/13-sharing.md);
[R16](sisu-command.md#sisu-audit).

### Augmented assignment

`x += 1` and the other operators followed by `=`, which change a `var`
in place. [Chapter 2](../tutorial/02-values.md); [R5](statements.md#assignment).

### Auto trait

A trait that a type has because of its fields, without an impl: `Send`,
`Sync` and `Escapable`. [Chapter 20](../tutorial/20-concurrency.md);
[R2](types.md#the-auto-traits).

## B

### Benchmark

A `bench "name":` block, which `sisu bench` times.
[Chapter 19](../tutorial/19-testing.md); [R15](testing.md#benchmarks).

### Binding

A name for a value, made by `let`, whose value cannot change, or `var`,
whose value can. [Chapter 1](../tutorial/01-hello.md);
[R3](declarations.md#bindings).

### Blame

Which side a failed contract accuses: a `requires` blames the caller,
an `ensures` the function. [Chapter 23](../tutorial/23-contracts.md);
[R11](contracts.md#preconditions).

### Block

The indented lines after a `:` that ends a line; a block has no value.
[Chapter 1](../tutorial/01-hello.md); [R1](lexical-structure.md#indentation-and-blocks).

### Blocked

The outcome of a test that reached a typed hole: neither passed nor
failed. [Chapter 1](../tutorial/01-hello.md); [R15](testing.md#outcomes).

### Borrow

Using a value without taking it from its owner. A plain parameter
borrows to read and a `mut` parameter borrows to change; a borrow ends
at its last use. [Chapter 11](../tutorial/11-borrowing.md);
[R7](ownership.md#references).

### Bound

A trait that a type parameter must implement, `[T: Ord]`; the body is
checked against its bounds where it is written.
[Chapter 15](../tutorial/15-generics.md); [R2](types.md#generic-types-and-functions).

### Box

`Box[T]`, a value in an allocation of its own; the compiler boxes a
recursive enum without asking. [Chapter 13](../tutorial/13-sharing.md);
[R7](ownership.md#sharing-cells-and-graphs).

### Budget

The number of steps that a compile-time evaluation may take, a million
unless `@comptime_budget(steps=N)` says otherwise.
[Chapter 25](../tutorial/25-compile-time.md); [R14](attributes.md#compile-time-budgets).

### Build profile

`debug` or `release`: how a build is compiled and which contracts it
checks; not a runtime profile. [Chapter 22](../tutorial/22-web-service.md);
[R16](sisu-command.md#compiling-and-running-natively---native-and---release).

## C

### Cancellation

Asking the tasks of a scope to stop: their blocking operations give
`Err(Cancelled)`. [Chapter 20](../tutorial/20-concurrency.md);
[R9](concurrency.md#cancellation).

### Capability

A value that grants access to part of the world, such as `Fs` for files
or `Clock` for time. A function acts on the world only through the
capabilities it is given. [Chapter 9](../tutorial/09-word-counter.md);
[R10](capabilities.md#the-rule).

### Catalog

The file that holds every message of the compiler, each with its code
and identifier, so that the text can change while the code stays.
[Chapter 1](../tutorial/01-hello.md); [R17](diagnostics.md).

### Cell

`Cell[T]`, a value that changes behind shared owners within one task.
[Chapter 13](../tutorial/13-sharing.md); [R7](ownership.md#sharing-cells-and-graphs).

### Chained condition

A condition of several parts joined by commas, each `let` binding for
the parts after it: `if let Some(x) = a, x > 0:`.
[Chapter 8](../tutorial/08-errors.md); [R5](statements.md#conditions).

### Channel

A bounded queue of owned values between tasks; closing it ends the
receiver's loop. [Chapter 20](../tutorial/20-concurrency.md);
[R9](concurrency.md#channels).

### Check level

Which contracts a run checks: `--contracts=all`, `preconditions` or
`off`. [Chapter 23](../tutorial/23-contracts.md);
[R16](sisu-command.md#what-a-run-checks---contracts).

### Closure

A function written where it is used, `x -> x * 2`, or with `it`, or as
a `do` block; it borrows what it uses, and an `own` closure keeps
copies. [Chapter 16](../tutorial/16-closures-and-iterators.md);
[R4](expressions.md#closures).

### Code

The stable name of a diagnostic, a letter and four digits such as
E0200; the letter is the kind and the digits the stage that reports it.
[Chapter 1](../tutorial/01-hello.md); [R17](diagnostics.md#codes-and-their-ranges).

### Coherence

The rule that a type implements a trait at most once in a program, so
that every use agrees. [Chapter 14](../tutorial/14-traits.md);
[R2](types.md#one-impl-for-each-type-and-trait).

### Compile-time evaluation

Running ordinary code while the program compiles, asked for with
`comptime`; it reaches nothing in the world.
[Chapter 25](../tutorial/25-compile-time.md); [R12](compile-time.md).

### Comprehension

`[f(x) for x in xs if c]`, a list, set or dict built from a loop in one
expression. [Chapter 5](../tutorial/05-collections.md);
[R4](expressions.md#comprehensions).

### Constant

A `const`, a value that the compiler computes before the program runs.
[Chapter 2](../tutorial/02-values.md); [R3](declarations.md#constants).

### Constant parameter

A number that is part of a type, `const N: int`, as in `Array[T, N]`.
[Chapter 15](../tutorial/15-generics.md); [R2](types.md#constant-parameters).

### Consumer

A method of an iterator that takes its items and gives a value, such as
`collect`, `fold` or `count`. [Chapter 16](../tutorial/16-closures-and-iterators.md).

### Context

`.context("what was happening")` on a result, which wraps its error
with a message. [Chapter 8](../tutorial/08-errors.md);
[R8](errors.md#traces-and-context).

### Contract

A condition that the language checks: `requires` on entry, `ensures` at
each exit, `invariant` for a type. [Chapter 23](../tutorial/23-contracts.md);
[R11](contracts.md).

### Copy

`.copy()` makes a second value of any type, at a cost that grows with
its size; a `Copy` type, such as a number or a struct that derives it,
is copied instead of moved. [Chapter 10](../tutorial/10-ownership.md);
[R7](ownership.md#copy-types-and-explicit-copies).

## D

### Deadline

A time after which a task is cancelled, set with
`spawn_with(deadline=...)`. [Chapter 20](../tutorial/20-concurrency.md);
[R9](concurrency.md#deadlines-and-races).

### Debug form

What `{x:?}` writes: a value as it is written in code. Every type has
one. [Chapter 6](../tutorial/06-text.md); [R4](expressions.md#interpolation-and-format-specs).

### Default argument

A parameter's value when a call leaves it out, computed at each call.
[Chapter 3](../tutorial/03-functions.md); [R3](declarations.md#parameters).

### `defer`

A statement that runs when its block ends, however it ends.
[Chapter 10](../tutorial/10-ownership.md); [R5](statements.md#with-and-defer).

### Dependency

A package that another names in `[dependencies]` and may import.
[Chapter 17](../tutorial/17-modules-and-packages.md);
[R13](modules.md#dependencies).

### Deprecation

Marking an item that will go, with `@deprecated("...")`; the warning at
each use is designed. [Chapter 28](../tutorial/28-code-that-lasts.md);
[R14](attributes.md#deprecation-and-migrations).

### Derive

`@derive(Trait)`: the trait's impl, written by the compiler from the
type's fields. [Chapter 7](../tutorial/07-structs-and-enums.md);
[R14](attributes.md#deriving-traits).

### Determinism

The promise of `pure`: the same arguments give the same bits on every
run, process, thread count and target.
[Chapter 26](../tutorial/26-purity.md); [X6](../explanations/determinism.md).

### Diagnostic

What Sisu says when something is wrong: a code, a message, the place,
and often a fix. [Chapter 1](../tutorial/01-hello.md);
[R17](diagnostics.md#the-parts-of-a-diagnostic).

### Dict

`dict[K, V]`, a map from keys to values that keeps the order in which
keys were added. [Chapter 5](../tutorial/05-collections.md);
[R2](types.md#collections).

### Doc comment

A `##` comment, which documents the item after it, or `##!`, which
documents the module; the examples in it are tests.
[Chapter 3](../tutorial/03-functions.md);
[R1](lexical-structure.md#comments-and-doc-comments).

### Drop

What happens to a value when the scope of its owner ends: it is
destroyed, in the reverse of the order in which the values were made,
and its `Drop` impl runs. [Chapter 10](../tutorial/10-ownership.md);
[R7](ownership.md#drops-and-their-order).

### `dyn`

`dyn Trait`, a value that carries its type, behind a `Box` or a `ref`;
its methods are looked up as it runs. [Chapter 15](../tutorial/15-generics.md);
[R2](types.md#trait-objects).

## E

### Edition

A year that a package declares; a new edition may change the language
in ways that are not compatible, and packages of every edition build
together. Designed. [Chapter 28](../tutorial/28-code-that-lasts.md);
[R13](modules.md#editions).

### `embed`

`embed("path")`, the text of a file of the program, read while it
compiles. [Chapter 25](../tutorial/25-compile-time.md);
[R12](compile-time.md#embedding-a-file).

### Enum

A type whose value is one of its variants, each of which may carry
values. [Chapter 7](../tutorial/07-structs-and-enums.md);
[R3](declarations.md#enums).

### Epoch

A version of a running program's code that `sisu run --watch` swaps in;
a task takes the newest when it starts or at `live.boundary()`.
[Chapter 22](../tutorial/22-web-service.md);
[R16](sisu-command.md#watching-for-saves---watch).

### Error

An expected failure that a correct program handles, such as a missing
file: a value of an error set, in a function's result. Not a panic.
[Chapter 8](../tutorial/08-errors.md); [R8](errors.md#errors-and-panics).

### Error set

The type of a function's errors, declared with `error`; sets combine
into unions without conversion code. [Chapter 8](../tutorial/08-errors.md);
[R8](errors.md#error-sets).

### Exhaustive

Covering every value: a `match` must be, and the diagnostic for one
that is not lists the cases it misses. [Chapter 4](../tutorial/04-control-flow.md);
[R6](patterns.md#a-match-covers-every-value).

### Exit status

The number a program ends with: 0, 1 for an error from `main` or a
program that did not compile, 101 for a panic.
[Chapter 8](../tutorial/08-errors.md); [R8](errors.md#exit-statuses).

### `extern`

An `extern "C":` block, which declares functions written in C; calling
one is unsafe. [Chapter 27](../tutorial/27-systems.md);
[R3](declarations.md#foreign-functions).

## F

### Fake

A capability that does its work in memory, such as `Fs.in_memory(...)`
or the test clock, so that a test touches nothing real.
[Chapter 9](../tutorial/09-word-counter.md); [R10](capabilities.md#fakes).

### False friend

A spelling that Sisu shares with Python or Rust and whose familiar
meaning Sisu does not have; it fails to compile rather than mean
something else (D-062). [Chapter 28](../tutorial/28-code-that-lasts.md);
[R19](false-friends.md).

### Feature

A named, additive part of a package, which `@cfg(feature="NAME")` keeps
or leaves out. [Chapter 17](../tutorial/17-modules-and-packages.md);
[R13](modules.md#features).

### Field

A named value inside a struct, or inside a variant.
[Chapter 7](../tutorial/07-structs-and-enums.md); [R3](declarations.md#structs).

### Fix

The part of a diagnostic that says how to change the code, with the
text it writes; the language server applies it.
[Chapter 1](../tutorial/01-hello.md); [R17](diagnostics.md#the-parts-of-a-diagnostic).

### Fixture

A value that a test needs, made by a function the test calls.
[Chapter 19](../tutorial/19-testing.md); [R15](testing.md#fixtures).

### Format spec

What follows the `:` of an interpolation, such as `>8` or `.2f`: width,
alignment, digits and base. [Chapter 6](../tutorial/06-text.md);
[R4](expressions.md#interpolation-and-format-specs).

### Formatter

`sisu fmt`, which writes the one layout of Sisu code; it has no
options. [Chapter 1](../tutorial/01-hello.md);
[R1](lexical-structure.md#the-formatters-layout).

### Frame

The part of the stack that a function's call owns; a view cannot leave
the frame that owns what it borrows. [Chapter 12](../tutorial/12-views.md);
[R7](ownership.md#views).

### `from` clause

`-> ref str from a`, which names the parameters that a returned borrow
borrows from. [Chapter 11](../tutorial/11-borrowing.md);
[R7](ownership.md#returning-a-borrow).

### `FromLiteral`

The trait through which a literal becomes a value of another type where
that type is expected, as a string literal becomes a `Path`.
[Chapter 6](../tutorial/06-text.md);
[R2](types.md#literals-that-take-the-type-expected).

### Front end

The part of the compiler that reads, checks and lowers a program to
SIR; `sisu` runs stage 1's, the one written in Sisu.
[X9](../explanations/compiler.md); [R16](sisu-command.md#choosing-the-front-end).

### Function type

The type of a function value, `(int) -> int`, or `pure (int) -> int`
for a pure one. [Chapter 16](../tutorial/16-closures-and-iterators.md);
[R2](types.md#function-types).

## G

### Generator

A function that yields its items with `yield` and returns an `Iter[T]`.
[Chapter 16](../tutorial/16-closures-and-iterators.md);
[R5](statements.md#generators).

### Generic

A function or type with type parameters, `def largest[T: Ord](...)`,
compiled once for each list of types it is used with.
[Chapter 15](../tutorial/15-generics.md);
[R2](types.md#generic-types-and-functions).

### Guaranteed drop

The rule that every view, scope and suspended frame ends by being
dropped, never leaked (D-058). [Chapter 12](../tutorial/12-views.md);
[R7](ownership.md#guaranteed-drop).

### Guard

`case P if CONDITION:`, a condition that an arm must also meet.
[Chapter 4](../tutorial/04-control-flow.md);
[R6](patterns.md#nested-patterns-and-guards).

## H

### Handle

What `spawn` gives back for a task, whose `join()` waits for its
result. [Chapter 20](../tutorial/20-concurrency.md);
[R9](concurrency.md#joining-a-task).

### Hot replacement

Swapping saved code into a running program, `sisu run --watch`.
[Chapter 22](../tutorial/22-web-service.md);
[R16](sisu-command.md#watching-for-saves---watch).

## I

### Identifier

The catalog's name for a diagnostic, such as
`resolve.undefined-name`, which the identifier form of
`sisu check --format=ids` prints. [Chapter 1](../tutorial/01-hello.md);
[R17](diagnostics.md#the-parts-of-a-diagnostic).

### `if let`

A condition that matches a pattern and binds its names for the block,
as `if let Some(x) = found:`. [Chapter 8](../tutorial/08-errors.md);
[R5](statements.md#conditions).

### Impl

`impl Trait for Type:`, which gives a type a trait's methods, or
`impl Type:`, which adds methods to a type.
[Chapter 14](../tutorial/14-traits.md); [R3](declarations.md#impls).

### Import

`import a.b` or `from a.b import x`, which binds a module or its items
in the module that writes it. [Chapter 17](../tutorial/17-modules-and-packages.md);
[R13](modules.md#imports).

### Inference

Working out a binding's type from its value and its uses inside a body;
never across a function's signature. [Chapter 2](../tutorial/02-values.md);
[R2](types.md#static-types-and-local-inference).

### Instance

The copy of a template that the compiler makes for one list of type
arguments. [Chapter 25](../tutorial/25-compile-time.md);
[R12](compile-time.md#templates).

### Interceptor

A value that sees every call through a capability that `cap.Wrap` puts
it around. [Chapter 18](../tutorial/18-capabilities.md);
[R10](capabilities.md#interceptors).

### Interpolation

`{expr}` inside a string, which puts the expression's text there;
`{expr=}` puts its source too. [Chapter 1](../tutorial/01-hello.md);
[R4](expressions.md#interpolation-and-format-specs).

### Interpreter

The part of stage 0 that runs a program's SIR; `sisu run` uses it
unless `--native` asks for machine code.
[X9](../explanations/compiler.md);
[R16](sisu-command.md#sisu-run).

### Invariant

A condition on a type that holds after construction and after each
method that changes it. [Chapter 23](../tutorial/23-contracts.md);
[R11](contracts.md#invariants).

### Iterator

A value with `next`, which gives items one at a time; `Iter[T]` is one
that a generator makes. [Chapter 16](../tutorial/16-closures-and-iterators.md);
[R2](types.md#collections).

## K

### Keyword argument

An argument passed by the parameter's name, `port=443`; a parameter
after `*` is keyword-only. [Chapter 3](../tutorial/03-functions.md);
[R3](declarations.md#parameters).

## L

### Labelled tuple

A tuple whose elements have names, `(low: int, high: int)`, as a
function returns several values. [Chapter 3](../tutorial/03-functions.md);
[R2](types.md#tuples).

### Language server

`sisu lsp`, which gives an editor the diagnostics, fixes and types of
the code being edited. [Chapter 1](../tutorial/01-hello.md);
[R16](sisu-command.md#sisu-lsp).

### Lending

Handing a capability for one call, `fs.lend()`, as a `Lent[T]` that the
callee cannot keep. [Chapter 18](../tutorial/18-capabilities.md);
[R10](capabilities.md#lending).

### `let ... else`

A binding whose pattern may not match, with an `else` that gives a
fallback or leaves the function. [Chapter 8](../tutorial/08-errors.md);
[R3](declarations.md#bindings).

### Line, logical

A line as the parser reads it: one line of text, or several when a
bracket is open or a line starts with an operator.
[Chapter 2](../tutorial/02-values.md);
[R1](lexical-structure.md#lines-and-statements).

### List

`list[T]`, a growable collection of values of one type, in order.
[Chapter 5](../tutorial/05-collections.md); [R2](types.md#collections).

### Lockfile

`sisu.lock`, the record of every package that a build resolved.
[Chapter 17](../tutorial/17-modules-and-packages.md);
[R13](modules.md#the-lockfile).

## M

### Manifest

`sisu.toml`, a package's description: its name, version, edition and
dependencies, as data. [Chapter 9](../tutorial/09-word-counter.md);
[R13](modules.md#the-manifest).

### `match`

The statement or expression that compares a value with patterns, case
by case. [Chapter 4](../tutorial/04-control-flow.md);
[R6](patterns.md#where-patterns-stand).

### Method

A function of a type that takes `self`, `mut self` or `own self`, called
as `value.name()`. [Chapter 7](../tutorial/07-structs-and-enums.md);
[R3](declarations.md#structs).

### Module

A file of a package, named by its path; a directory is a module too.
[Chapter 17](../tutorial/17-modules-and-packages.md);
[R13](modules.md#the-module-tree).

### Monitor

The ownership monitor, which checks moves and borrows while a program
runs: `sisu test` runs under it, and `sisu run --monitor` asks for it.
[Chapter 11](../tutorial/11-borrowing.md);
[R16](sisu-command.md#checking-ownership-while-running---monitor).

### Monomorphisation

Compiling a generic function once for each list of types it is used
with, so that generic code costs nothing as it runs.
[Chapter 15](../tutorial/15-generics.md); [R2](types.md#monomorphisation).

### Move

Giving a value to a new owner, which an assignment, an `own` parameter
or a collection does; the old owner cannot use it after.
[Chapter 5](../tutorial/05-collections.md) and
[chapter 10](../tutorial/10-ownership.md);
[R7](ownership.md#every-value-has-one-owner).

### Must-consume

A type marked `@must_consume`, each of whose values must end in an
`own` parameter, usually a method such as `close(own self)`.
[Chapter 13](../tutorial/13-sharing.md);
[R7](ownership.md#values-that-must-be-consumed).

### Mutex

`Mutex[T]`, a value that tasks change in turn, each holding the lock.
[Chapter 20](../tutorial/20-concurrency.md);
[R9](concurrency.md#sharing-a-value-that-changes).

## N

### Native build

An executable that runs without `sisu`, made by
`sisu build --native`. [Chapter 22](../tutorial/22-web-service.md);
[R16](sisu-command.md#compiling-and-running-natively---native-and---release).

### Never

The type of an expression that never gives a value, such as
`panic(...)`; it fits where any type is expected.
[R2](types.md#unit-and-never).

### Newtype

A struct with one field that makes a new type over an old value, so
that metres do not add to seconds. [Chapter 14](../tutorial/14-traits.md);
[R2](types.md#type-aliases-and-newtypes).

## O

### Object safety

What a trait needs for `dyn Trait`: methods that can be called without
knowing the type. [Chapter 15](../tutorial/15-generics.md);
[R2](types.md#trait-objects).

### Optional value

A `T?`: `Some(x)` or `None`, the value or its absence. Sisu has no
`null`. [Chapter 8](../tutorial/08-errors.md); [R2](types.md#optional-values).

### Orphan rule

An impl stands in the package of its trait or of its type, except in
the root package. [Chapter 14](../tutorial/14-traits.md);
[R2](types.md#one-impl-for-each-type-and-trait).

### Overflow

An integer result that does not fit in its type; it panics, unless the
operator says to wrap (`+%`) or saturate (`+|`).
[Chapter 2](../tutorial/02-values.md); [R2](types.md#overflow-traps).

### Owner

The binding, field or collection that a value belongs to; every value
has one. [Chapter 10](../tutorial/10-ownership.md);
[R7](ownership.md#every-value-has-one-owner).

## P

### Package

A directory with a manifest, `sisu.toml`, and a `src/` tree of modules.
[Chapter 9](../tutorial/09-word-counter.md);
[R13](modules.md#what-a-program-is-made-of).

### Panic

A bug that stops the task: an overflow, an index out of range, a failed
`assert`, a `todo` reached. It unwinds, and nothing catches it.
[Chapter 8](../tutorial/08-errors.md); [R8](errors.md#panics-and-unwinding).

### Parameter convention

How a parameter receives its argument: a plain parameter borrows to
read, `mut` borrows to change, and `own` takes it.
[Chapter 11](../tutorial/11-borrowing.md);
[R7](ownership.md#parameter-conventions).

### Parametrised test

`test "name" for x in CASES:`, a test for each case, named by it.
[Chapter 19](../tutorial/19-testing.md); [R15](testing.md#the-forms-of-a-test).

### Pattern

The shape that `match`, `if let` and `let` compare a value with, which
binds the names it holds. [Chapter 4](../tutorial/04-control-flow.md);
[R6](patterns.md).

### Place

What an assignment can change: a `var`, a field, an index or a key, as
in `grid[1][0] = 5`. [Chapter 2](../tutorial/02-values.md);
[R5](statements.md#assignment).

### Plain data

A type whose every bit pattern is a value, `@derive(Plain)` of
`std.mem`, which can be read from bytes without unsafe code.
[Chapter 27](../tutorial/27-systems.md);
[R14](attributes.md#the-layout-of-a-type).

### Poisoning

What happens to a `Mutex` whose guard a panic drops, when the
protected value's invariant no longer holds: each later `lock()` panics
(D-077). [Chapter 20](../tutorial/20-concurrency.md);
[R9](concurrency.md#sharing-a-value-that-changes).

### Prelude

The fixed list of names that every module sees without an import.
[Chapter 17](../tutorial/17-modules-and-packages.md);
[R13](modules.md#names-every-module-sees).

### Preview

A feature that is not stable yet, which a program asks for by name with
`@preview`. Leaning. [Chapter 28](../tutorial/28-code-that-lasts.md);
[R14](attributes.md#preview-features).

### Property test

`check(GENERATOR) do x:`, a test of a property on many generated
cases, which shrinks a failing case to a small one.
[Chapter 19](../tutorial/19-testing.md); [R15](testing.md#property-tests).

### `pub`

The visibility that exports an item to every module that imports it;
`pub(package)` exports it to its own package only.
[Chapter 17](../tutorial/17-modules-and-packages.md);
[R13](modules.md#visibility).

### `pure`

A function that takes no capability and calls only pure functions, so
that it gives the same bits for the same arguments.
[Chapter 26](../tutorial/26-purity.md); [R2](types.md#pure-function-types).

## R

### Raw pointer

`ptr[T]` or `ptr mut T` of `std.mem`, an address that unsafe code
reads and writes. [Chapter 27](../tutorial/27-systems.md);
[R7](ownership.md#unsafe-code-and-raw-pointers).

### Record and replay

`sisu run --record LOG` writes every call a run makes to the world, and
`sisu replay LOG` runs the program again from the log alone.
[Chapter 22](../tutorial/22-web-service.md);
[R16](sisu-command.md#recording-a-run---record-and---redact).

### Reducer

A type with `empty`, `add`, `merge` and `finish` that `std.par`'s
`reduce` combines in a fixed order. [Chapter 26](../tutorial/26-purity.md);
[R9](concurrency.md#parallel-computation-over-data).

### Re-export

`pub from m import x`, which makes `x` visible through the module that
imports it. [Chapter 17](../tutorial/17-modules-and-packages.md);
[R13](modules.md#re-exports).

### Reference

`ref x` or `ref mut x`, a borrow of a value or a part of it; a
reference never outlives what it borrows.
[Chapter 11](../tutorial/11-borrowing.md); [R7](ownership.md#references).

### Refinement type

A type with a predicate, `type Port = int where 1 <= self <= 65535`.
[Chapter 23](../tutorial/23-contracts.md); [R2](types.md#refinement-types).

### Reflection

Asking a type about itself while compiling: `reflect.fields_of(T)`,
`reflect.name_of(T)` and the rest. [Chapter 25](../tutorial/25-compile-time.md);
[R12](compile-time.md#reflection).

### Registry

Where packages will be published and found by version. Designed.
[Chapter 17](../tutorial/17-modules-and-packages.md);
[R13](modules.md#the-registry).

### REPL session

What `sisu repl` runs: Sisu read and run a line at a time.
[Chapter 1](../tutorial/01-hello.md); [R16](sisu-command.md#sisu-repl).

### Replay token

The line a failing test prints, which `sisu test --replay` takes to run
the same failure again. [Chapter 19](../tutorial/19-testing.md);
[R15](testing.md#replay-tokens).

### Result

A `T ! E`, read "T or error E": `Ok(value)` or `Err(error)`. A result
must be used. [Chapter 8](../tutorial/08-errors.md);
[R2](types.md#results-and-error-sets).

### Revocation

Ending a capability's access, through the revoker of a capability made
with `revocable()`; its calls then fail with `Revoked`.
[Chapter 18](../tutorial/18-capabilities.md);
[R10](capabilities.md#revocation).

### Root capability

One of the capabilities that `main(sys: System)` receives: `fs`, `net`,
`clock`, `rand`, `env`, `proc` and `stdin`.
[Chapter 9](../tutorial/09-word-counter.md);
[R10](capabilities.md#the-root-capabilities).

### Root module

The module that a package's name names: `src/main.sisu` for an
application, `src/lib.sisu` for a library.
[Chapter 9](../tutorial/09-word-counter.md);
[R13](modules.md#the-module-tree).

### Root package

The package that a build starts from, which nothing depends on; it may
implement any trait for any type. [Chapter 14](../tutorial/14-traits.md);
[R2](types.md#one-impl-for-each-type-and-trait).

### Runtime profile

What a program runs on: `full`, `single`, `freestanding` or `sim`; not
a build profile. [Chapter 27](../tutorial/27-systems.md);
[R22](targets.md#runtime-profiles).

## S

### Scheduler

What decides the order in which tasks run; it is seeded, so a test can
run under many orders and repeat any of them.
[Chapter 20](../tutorial/20-concurrency.md); [R9](concurrency.md#the-scheduler).

### Scope

Where a binding exists: from its declaration to the end of its block.
A task scope, `task.scope do s:`, is the block whose tasks finish
before it does. [Chapter 10](../tutorial/10-ownership.md) and
[chapter 20](../tutorial/20-concurrency.md);
[R9](concurrency.md#tasks-and-scopes).

### Script

A file whose first line is `#!/usr/bin/env sisu`, whose statements are
its `main`. [Chapter 1](../tutorial/01-hello.md);
[R13](modules.md#scripts-and-inline-metadata).

### `select`

The statement that waits for the first of several channel operations
or timers. [Chapter 20](../tutorial/20-concurrency.md);
[R9](concurrency.md#waiting-for-the-first-of-several).

### `Send` and `Sync`

The auto traits of values that may cross to another task (`Send`) and
be shared between tasks (`Sync`). [Chapter 20](../tutorial/20-concurrency.md);
[R9](concurrency.md#values-that-cross-to-a-task).

### Session

What a terminal shows: a line that starts with `$ sisu` is a command,
and the lines after it are what it printed. The manual's sessions are
checked. [Chapter 1](../tutorial/01-hello.md);
[R16](sisu-command.md#how-to-read-this-page).

### Set

`set[T]`, a collection that holds each value once, in the order added.
[Chapter 5](../tutorial/05-collections.md); [R2](types.md#collections).

### Shadowing

A binding in an inner block that reuses an outer name, which gets a
warning; a new `let` in the same block rebinds the name without one.
[Chapter 2](../tutorial/02-values.md);
[R3](declarations.md#rebinding-and-shadowing).

### `Shared`

`Shared[T]`, a value with several owners, dropped with the last one;
`.copy()` adds an owner. [Chapter 13](../tutorial/13-sharing.md);
[R7](ownership.md#sharing-cells-and-graphs).

### Shield

`task.shield`, a block that cancellation does not interrupt.
[Chapter 20](../tutorial/20-concurrency.md); [R9](concurrency.md#cancellation).

### Simulation

Running a test against simulated capabilities whose calls fail at
random with the errors of their sets, `sisu test --sim --faults P`.
[Chapter 22](../tutorial/22-web-service.md); [R15](testing.md#simulation).

### SIR

Sisu's intermediate representation, the program that the front end
writes and the interpreter and the native backend run.
[X9](../explanations/compiler.md); [R16](sisu-command.md#sisu-sir).

### Slice

`xs[i:j]`, a new list of a run of a list's elements; a `Slice[T]` is a
view of a run, which borrows the list.
[Chapter 5](../tutorial/05-collections.md) and
[chapter 12](../tutorial/12-views.md);
[R4](expressions.md#indexing-and-slicing).

### Snapshot

A value's `Debug` form kept in a file, which `assert_snapshot` compares
with. [Chapter 19](../tutorial/19-testing.md); [R15](testing.md#snapshots).

### Stage

A step of the compiler's bootstrap: stage 0 is the implementation in
Rust, whose heart is an interpreter; stage 1 is the front end written in
Sisu, which `sisu` now checks, runs and builds with; stage 2 compiles to
machine code; and stage 3 compiles itself. "Stage 0" in the manual means
the compiler of this repository. [X9](../explanations/compiler.md);
[R16](sisu-command.md#how-to-read-this-page).

### Standard library

`std`, the modules that come with Sisu, and `std.x`, the packages
versioned on their own, such as `std.x.http`.
[Chapter 21](../tutorial/21-standard-library.md);
[R16](sisu-command.md#the-standard-librarys-reference).

### `str` and `String`

A `str` is a view of text and a `String` owns text; a `str` parameter
accepts both. [Chapter 6](../tutorial/06-text.md); [R2](types.md#text).

### Struct

A type of named fields, with methods. [Chapter 7](../tutorial/07-structs-and-enums.md);
[R3](declarations.md#structs).

### Suite

`suite "name":`, a group of tests one level deep.
[Chapter 19](../tutorial/19-testing.md); [R15](testing.md#the-forms-of-a-test).

### Supertrait

A trait that a trait requires: `trait Ord(Eq, PartialOrd)`.
[Chapter 14](../tutorial/14-traits.md); [R3](declarations.md#traits).

## T

### Tag

`@tag("slow")` on a test, by which `--tag` and `--skip-tag` choose
tests. [Chapter 19](../tutorial/19-testing.md);
[R14](attributes.md#tags-on-tests).

### Target

What a build is for: an operating system, an architecture and a runtime
profile, which `target.os` and the others describe.
[Chapter 25](../tutorial/25-compile-time.md) and
[chapter 27](../tutorial/27-systems.md);
[R22](targets.md#the-facts-of-the-target).

### Task

Work that runs at the same time as other work, started with `spawn` in
a scope. [Chapter 20](../tutorial/20-concurrency.md);
[R9](concurrency.md#tasks-and-scopes).

### Teaching profile

A level of a ladder of language subsets for learners, set in the
manifest. Leaning (D-073). [R13](modules.md#the-teaching-profile).

### Template

A generic function whose body reflects on its type parameters, made
again for each list of types it is called with.
[Chapter 25](../tutorial/25-compile-time.md);
[R12](compile-time.md#templates).

### Test

A `test "name":` block beside the code it tests; `sisu test` runs it
and reports it ok, failed or blocked. [Chapter 1](../tutorial/01-hello.md);
[R15](testing.md#the-forms-of-a-test).

### Test clock

The clock of a test, which moves only when the test moves it or when
every task waits. [Chapter 19](../tutorial/19-testing.md);
[R15](testing.md#isolation-and-the-test-clock).

### Trace

The places where `?` passed an error on, which an error from `main`
prints; the trace context of a log line names its task.
[Chapter 8](../tutorial/08-errors.md); [R8](errors.md#traces-and-context).

### Trait

A named ability: methods that a type gets through an impl.
[Chapter 14](../tutorial/14-traits.md); [R3](declarations.md#traits).

### Transcript

A file of commands and what the program printed for them, which
`sisu test` runs as a test. [Chapter 19](../tutorial/19-testing.md);
[R15](testing.md#transcripts).

### `[trust]`

The table of the manifest that lists the dependencies that may hold
unsafe code. [Chapter 17](../tutorial/17-modules-and-packages.md);
[R13](modules.md#trust).

### Truthiness

The idea that a value other than a `bool` counts as true or false,
which Sisu leaves out: a condition is a `bool`.
[Chapter 2](../tutorial/02-values.md); [R4](expressions.md#logic).

### Tuple

A fixed number of values of any types, `(a, b)`, read by position or
unpacked. [Chapter 5](../tutorial/05-collections.md);
[R2](types.md#tuples).

### Type

What a value is, which the compiler knows before the program runs:
`int`, `str`, a struct of the program. [Chapter 2](../tutorial/02-values.md);
[R2](types.md).

### Typed hole

`todo("name")`, a placeholder for code not written yet, which compiles
and stops a run or blocks a test that reaches it.
[Chapter 1](../tutorial/01-hello.md) and
[chapter 3](../tutorial/03-functions.md); [R5](statements.md#typed-holes).

## U

### Union

`T ! {A, B}`, the errors of both sets; a set is a subtype of every set
that holds all its variants. [Chapter 8](../tutorial/08-errors.md);
[R8](errors.md#error-sets).

### Unit

`()`, the value of a function that returns nothing.
[Chapter 3](../tutorial/03-functions.md); [R2](types.md#unit-and-never).

### Unsafe

An operation that the compiler cannot check: a call of an `unsafe def`,
a call of a foreign function, or an `unsafe impl`, each in an `unsafe:`
block or expression with a `# safety:` comment.
[Chapter 27](../tutorial/27-systems.md);
[R7](ownership.md#unsafe-code-and-raw-pointers).

### Unwinding

What a panic does: the task stops, and every value on the way is
dropped and every `defer` runs. [Chapter 10](../tutorial/10-ownership.md);
[R8](errors.md#panics-and-unwinding).

## V

### Variant

One of the values of an enum or an error set, which may carry fields.
[Chapter 7](../tutorial/07-structs-and-enums.md);
[R3](declarations.md#enums).

### View

A value that borrows what it shows: a `str`, a `Slice[T]` or a
`view struct`. It cannot outlive what it borrows.
[Chapter 12](../tutorial/12-views.md); [R7](ownership.md#views).

### Visibility

Who may use an item: its own module (the default), its package
(`pub(package)`), or everyone (`pub`).
[Chapter 17](../tutorial/17-modules-and-packages.md);
[R13](modules.md#visibility).

## W

### `where` clause

`where T: Display`, bounds written after a signature.
[Chapter 15](../tutorial/15-generics.md);
[R2](types.md#generic-types-and-functions).

### Widening

A conversion that loses nothing, as from `u8` to `int`, which happens
without a word; every other conversion is a method.
[Chapter 2](../tutorial/02-values.md); [R2](types.md#widening-and-conversions).

### `with`

`with VALUE as NAME:`, a block at whose end the value is dropped.
[Chapter 10](../tutorial/10-ownership.md); [R5](statements.md#with-and-defer).

### Worksheet

A file that `sisu ws` runs a statement at a time, showing each value.
[Chapter 1](../tutorial/01-hello.md); [R16](sisu-command.md#sisu-ws).

### Workspace

A directory whose manifest lists packages that build together, with
one lockfile. [Chapter 17](../tutorial/17-modules-and-packages.md);
[R13](modules.md#workspaces).

### Wrapping and saturating arithmetic

`+%`, `-%` and `*%` wrap around on overflow, and `+|`, `-|` and `*|`
stay at the type's least or greatest value.
[Chapter 2](../tutorial/02-values.md) and
[chapter 24](../tutorial/24-numbers.md); [R2](types.md#overflow-traps).

## Y

### `yield`

The statement that gives a generator's next item.
[Chapter 16](../tutorial/16-closures-and-iterators.md);
[R5](statements.md#generators).
