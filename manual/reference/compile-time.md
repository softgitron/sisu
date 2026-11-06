# Compile-time evaluation and reflection

This page states the rules of code that runs while a program compiles:
`comptime` and what it may do, budgets of steps, the `reflect` API,
templates, the derive protocol, `embed`, the facts of the target,
conditional compilation, and what is designed for configuration files
and checked string literals. Compile-time code is ordinary Sisu, run by
the compiler's interpreter with the semantics of run time, and Sisu has
no macros (D-018).

**See also:** the tutorial's [chapter 25](../tutorial/25-compile-time.md);
the guides [Write a derive](../guides/derive.md) and [Embed a file at
compile time](../guides/embed.md); the explanation [Compile-time code
instead of macros](../explanations/compile-time.md); the proposal's
[08, Metaprogramming](../../proposal/08-metaprogramming.md); the
compiler's [architecture notes](../../compiler/docs/architecture.md),
"Compile-time evaluation" and "Templates and instances";
[R14](attributes.md) for `@derive`, `@cfg` and `@comptime_budget`;
[R17, Diagnostics](diagnostics.md#codes-and-their-ranges) for the
codes, E06xx.

Each section ends with a status, in the words that [R16](sisu-command.md#how-to-read-this-page)
explains. Compile-time evaluation and reflection instead of macros are
decided (D-018, Q-15); budgets owned by the item that spends them lean
(D-063).

## Asking for compile-time evaluation

- `comptime EXPR` asks the compiler to compute `EXPR`, and the program
  keeps the value as if it were written as a literal. As an operator,
  `comptime` takes the operand after it: `comptime f() + 1` adds at run
  time. Before a condition it takes all of it, `if comptime a or b:`.
- `const NAME: T = comptime f()` is a constant that the compiler
  computes, once, even if nothing uses it.
- `comptime assert CONDITION, MESSAGE` checks while compiling and leaves
  nothing for run time; `comptime EXPR` as a statement runs and leaves
  nothing either.
- `comptime def` declares a function that only compile-time code may
  call, inside `comptime` or another `comptime def`; a call at run time,
  or the function taken as a value, is E0605 (`comptime.only`). A trait's
  `comptime def` other than its `derive` is called as `Trait.name(...)`.
- Any other function can run at compile time too, `pure` or not, as long
  as it reaches nothing in the world.

<!-- file: examples/reference-compile-time/values.sisu -->
```sisu
# `comptime` computes a value while compiling; the program keeps it.
def squares(n: int) -> list[int]:
    var out: list[int] = []
    for i in range(n):
        out.push(i * i)
    return out

const SQUARES: list[int] = comptime squares(6)
const SLOTS: int = 16

def slot(n: int) -> int:
    comptime assert SLOTS & (SLOTS - 1) == 0, "SLOTS is a power of two"
    return n & (SLOTS - 1)

def main():
    print(SQUARES)
    print(comptime len(squares(4)) + 1)
    print("{slot(3)} {slot(21)}")
```

<!-- output: examples/reference-compile-time/values.out -->
```text
[0, 1, 4, 9, 16, 25]
5
3 5
```

`comptime len(squares(4)) + 1` computes the length while compiling and
adds 1 when the program runs.

**Status.** In stage 0 (D-018).

## What compile-time code may do

The evaluator is the program's interpreter, so compile-time code has
the semantics of run time: the same widths of integers and overflow
traps, the same order of drops, the same insertion order of a `dict`
and a `set`, and the same strict floating point, so a value has the
same bits whether it is computed while compiling or while running
(D-074). Four rules bound it:

- It reaches nothing in the world: no standard output, arguments,
  environment, clock or file system, which is E0601 (`comptime.io`),
  stopped where it was used; `embed` is the one way to read a file
  ([Embedding a file](#embedding-a-file)).
- It sees only what is known while compiling, constants and literals:
  a local or a parameter of the function around it is E0600
  (`comptime.not-constant`), and so is a value that depends on the
  function's type parameters.
- Its value must have a literal form, for the program to keep it:
  numbers, text, booleans, characters, and lists, sets, dicts, tuples,
  optional values, results, structs and enums of them. A function is
  kept as the function it names, and a closure as that closure when it
  captures nothing or only numbers, text, booleans and characters; a
  closure that captures anything else (a tuple, a list, a struct, or a
  binding of the compile-time code), a reference, a `Shared` or another
  handle is E0604 (`comptime.not-kept`), whose message names what it
  captures and whose note `comptime.kept-functions` says what is kept.
- A panic, a failed `comptime assert` included, is E0602
  (`comptime.panicked`) with its message. An expression that uses one
  that failed is not evaluated again: its error is that one's.
- A typed hole is not a panic: an expression that reaches one has no
  value to keep, so it is left to run time, where reaching the hole
  blocks the test or stops the run, as any hole does (D-061). The build
  warns of the hole (W0001), and a release build refuses it.

Code that compile-time evaluation writes holds no `unsafe` (E0615,
`comptime.unsafe-written`), so that every unsafe block stays in a source
file.

<!-- file: examples/reference-compile-time/limits.sisu -->
```sisu
# Compile-time code sees no run-time value and reaches no world.
def greet(name: str) -> String:
    print("greeting {name}")
    return "Hei, {name}!"

def shift(by: (int, int)) -> (int) -> int:
    return own x -> x + by.0 - by.1

const GREETING: String = comptime greet("Aino")
const SHIFT: (int) -> int = comptime shift((5, 3))

def main():
    print(GREETING)
```

<!-- output: examples/reference-compile-time/limits.console -->
```console
$ sisu run limits.sisu
error[E0601]: the compile-time evaluation of this expression used the standard output; nothing reaches the world at compile time
  --> limits.sisu:9:26
  |
9 | const GREETING: String = comptime greet("Aino")
  |                          ^^^^^^^^^^^^^^^^^^^^^^

error[E0604]: a closure that captures a tuple made at compile time cannot be kept for run time; numbers, text, booleans, characters, and lists, sets, dicts, tuples, options, results, structs and enums of them can
  --> limits.sisu:10:29
   |
10 | const SHIFT: (int) -> int = comptime shift((5, 3))
   |                             ^^^^^^^^^^^^^^^^^^^^^^
  = note: a function is kept as the function it names, and a closure as that closure when it captures nothing or only numbers, text, booleans and characters; a closure that captures a binding would reach the compile-time code's bindings, which run time does not have

error[D0006]: the program was not run: errors found (2)
```

**Status.** In stage 0 (D-018, D-074).

## Budgets

Each `comptime` expression has a budget of 1,000,000 steps of the
evaluator, counted in SIR's steps, so the count is the same on every
machine and a build that fits its budget fits it everywhere. So does
the computation of each constant, which every check of the program
runs ([R3](declarations.md#constants)). `@comptime_budget(steps=N)` on the function or the constant that holds
the expression gives it `N` instead
([R14](attributes.md#compile-time-budgets)). The budget belongs to the
item that spends it, so a package that uses another never raises a
budget for the other's internals. An evaluation that spends its budget
is E0603 (`comptime.budget`), which names the item and its budget; an
attribute without `steps=` and a positive number is E0606
(`comptime.budget-attribute`).

**Stage 0 differs.** A long evaluation does not print its progress, as
[08](../../proposal/08-metaprogramming.md#compile-time-evaluation) has
it do, and stage 1's evaluator takes a few seconds for a budget of a
million steps ([chapter 25](../tutorial/25-compile-time.md#a-budget-of-steps)).

**Status.** In stage 0 (D-063 leans), with the difference noted.

## Reflection

`reflect` is a module of compile-time queries about types. A
`comptime def` takes a type as a parameter, `T: type`, and a generic
function asks about its type parameters ([Templates](#templates)).

| Query | Gives |
|---|---|
| `reflect.name_of(T)` | the type's name |
| `reflect.is_struct(T)`, `is_enum`, `is_trait`, `is_int`, `is_float`, `is_bool`, `is_char`, `is_str` (`str` or `String`), `is_list`, `is_option`, `is_tuple` | whether the type is of that kind |
| `reflect.fields_of(T)` | a struct's fields, each with `name`, `index`, `named`, `ty`, `default`, whether it has a default, `doc`, its `##` comment (empty without one), and `attributes`, the field's attributes; an empty list for a type declared without fields, such as `int` |
| `reflect.variants_of(T)` | an enum's variants, each with `name`, `index`, `fields`, whose fields have a `name`, a `ty` and a `doc`, and `attributes`, the variant's attributes |
| `reflect.doc_of(T)` | the `##` comment of the type's declaration, a struct's, an enum's or an error set's, its attributes passed over; empty without one, and for a type of the standard library |
| `reflect.elem_of(T)` | the element type of a list or an optional value |
| `reflect.methods_of(T)` | a trait's methods or a type's own, each with `name`, `index`, `receiver`, `params` (each with `name`, `index`, `mode` and `ty`), `ret`, `provided` and `attributes`, the method's attributes |
| `reflect.attributes_of(T)` | the attributes on the type's declaration, or on a method `m` that `methods_of` gives, or a function `f` that `functions_of` gives, each with `name`, `args`, the arguments' values, and `keys`, their keywords ([R14](attributes.md#attributes-are-data)) |
| `reflect.functions_of(m)` | the functions of a module `m` that the code imports, in the order declared ([A module's functions](#a-modules-functions)) |
| `reflect.implements(T, "Trait")` | whether the type has an impl of the trait |
| `reflect.default_of(f)` | the default of the field `f`, evaluated where it is written |
| `reflect.construct(T, {NAME: VALUE, ...})` | the struct built from the fields given, each field without a default given, its invariant checked |

Compile-time values are types, the descriptors of their fields,
variants, methods and attributes, lists and pairs of them, integers,
text and booleans, with `enumerate`, `len`, `range`, `zip`, `sorted`
and the other builtins that [Templates](#templates) lists, arithmetic,
comparisons, `and`, `or`, `not` and interpolation. Code works with them in these
forms:

- `comptime for x in EXPR:` unrolls its body, once for each item;
- `if comptime CONDITION:` and `comptime if CONDITION:` decide their
  branch, and both branches are checked; a condition on `T` narrows
  `T` inside the branch, which is the one way a generic body depends on
  the shape of its type;
- `value.[f]` is the field of `value` that the compile-time `f` names,
  and `value.[m]` the method that `m` names, so `value.[m]()` calls it;
  a variant's field is reached inside the arm `case V(..)` that
  `comptime for V in reflect.variants_of(T)` makes among a `match`'s
  arms, where it is the arm's binding;
- a comprehension over compile-time lists is unrolled into the list,
  set or dict it makes;
- `comptime panic(MESSAGE)` stops the compilation for the types that
  reach it.

Reflection makes values and the code of a template, never a
declaration: a `comptime` function cannot add a struct or a function to
a module, and the one declaration it can write is a derive's impl
([The derive protocol](#the-derive-protocol)). Nor does it make a type
yet, which 08 allows for use in expressions: the first step is planned,
a `comptime def pick(wide: bool) -> type` whose body the compiler runs
and a type position that calls it, but today `-> type` is E0201 and a
call where a type goes a syntax error. How that call is written is the
owner's question Q-81, and `@diagnostic_name`
([R14](attributes.md#names-of-types-in-diagnostics)) waits for it.

<!-- file: examples/reference-compile-time/reflect.sisu -->
```sisu
# `reflect` answers questions about a type while the program compiles.
struct Booking:
    name: String
    hour: int = 18

    def label(self) -> String:
        return "{self.name} at {self.hour}"

enum Heater:
    Wood
    Electric(kilowatts: int)

comptime def shape(T: type) -> String:
    var out = String.new(reflect.name_of(T))
    comptime for f in reflect.fields_of(T):
        out = "{out}, field {f.index} {f.name} default={f.default}"
    comptime for m in reflect.methods_of(T):
        out = "{out}, method {m.name}"
    comptime for v in reflect.variants_of(T):
        out = "{out}, variant {v.name} of {len(v.fields)} fields"
    return out

def main():
    print(comptime shape(Booking))
    print(comptime shape(Heater))
    print(comptime reflect.is_enum(Heater))
```

<!-- output: examples/reference-compile-time/reflect.out -->
```text
Booking, field 0 name default=False, field 1 hour default=True, method label
Heater, variant Wood of 0 fields, variant Electric of 1 fields
True
```

A field's `doc` is the text of its `##` lines, without the `##` and the
space after it, the lines joined by line breaks; `std.x.cli`'s help text
prints it under each option ([G6](../guides/command-line.md#2-print-a-help-text)):

<!-- file: examples/reference-compile-time/field_docs.sisu -->
```sisu
# A field's `##` comment is its `doc`, which reflection gives.
struct Booking:
    ## Who booked the sauna.
    name: String
    ## The hour of the turn, from 16 to 21.
    hour: int = 18

def describe[T]() -> String:
    var out = String.new(reflect.name_of(T))
    comptime for f in reflect.fields_of(T):
        out = "{out}\n  {f.name}: {f.doc}"
    return out

def main():
    print(describe[Booking]())
```

<!-- output: examples/reference-compile-time/field_docs.out -->
```text
Booking
  name: Who booked the sauna.
  hour: The hour of the turn, from 16 to 21.
```

`reflect.doc_of(T)` is the type's own comment, read the same way, and
a variant's field that starts a line of its own has its `doc` too; a
field on the variant's own line, as in `Guessed(int)`, has none, since
the comment above that line is the variant's. `std.x.cli`'s
`cli.about[T]()` prints a struct's own comment
([G6](../guides/command-line.md#2-print-a-help-text)):

<!-- file: examples/reference-compile-time/item_docs.sisu -->
```sisu
# A type's own `##` comment, and a variant's fields', from reflection.
## How a sauna is heated.
enum Heater:
    Wood
    Electric(
        ## The power of the stove, in kilowatts.
        kilowatts: int,
        ## The number of phases it draws on.
        phases: int
    )

def describe[T]() -> String:
    var out = "{reflect.name_of(T)}: {reflect.doc_of(T)}"
    comptime for v in reflect.variants_of(T):
        comptime for f in v.fields:
            out = "{out}\n  {v.name}.{f.name}: {f.doc}"
    return out

def main():
    print(describe[Heater]())
```

<!-- output: examples/reference-compile-time/item_docs.out -->
```text
Heater: How a sauna is heated.
  Electric.kilowatts: The power of the stove, in kilowatts.
  Electric.phases: The number of phases it draws on.
```

A type of the prelude, `int`, `f64`, `bool`, `char` or `str`, is a
type compile time knows in any function, as a type of the program is:

<!-- file: examples/reference-compile-time/builtin_types.sisu -->
```sisu
# Reflection on the prelude's types where no type parameter is in scope.
def main():
    print(comptime reflect.name_of(int))
    print(comptime reflect.name_of(str))
    print(comptime reflect.is_float(f64))
    print(comptime reflect.is_int(char))
    print(comptime len(reflect.fields_of(bool)))
```

<!-- output: examples/reference-compile-time/builtin_types.out -->
```text
int
str
True
False
0
```

It is a type argument too:

<!-- file: examples/reference-compile-time/type_argument.sisu -->
```sisu
# A builtin type given to a `comptime def` that takes a type.
comptime def name(T: type) -> String:
    return String.new(reflect.name_of(T))

def main():
    print(comptime name(String))
    print(comptime name(bool))
```

<!-- output: examples/reference-compile-time/type_argument.console -->
```console
$ sisu run type_argument.sisu
String
bool
```

A query takes a type, a type parameter or a type or trait in scope,
and a module for `functions_of`. Given anything else, a run-time value
or a name of nothing, the function's compile-time code stops: in a
function without type parameters, made once, it is E0616
(`comptime.function-failed`) at the function, whose reason says what
the query takes, with the label `comptime.function-here` at the
argument, and in a template it is E0609 at the call that made the
instance ([Templates](#templates)):

<!-- file: examples/reference-compile-time/not_a_type.sisu -->
```sisu
# A reflection query given a run-time value, where it takes a type.
def main():
    let degrees = 80
    print(comptime reflect.name_of(degrees))
```

<!-- output: examples/reference-compile-time/not_a_type.console -->
```console
$ sisu run not_a_type.sisu
error[E0616]: the compile-time code of `main` stopped: `reflect.name_of` takes a type (a type parameter, or a type or trait in scope), and compile time knows no type `degrees`
  --> not_a_type.sisu:2:5
  |
2 | def main():
  |     ^^^^
  |
4 |     print(comptime reflect.name_of(degrees))
  |                                    ------- the compile-time code, where it stopped

error[D0006]: the program was not run: errors found (1)
```

Give the query the type, `int` here, or a type parameter of a generic
function that takes the value.

### A module's functions

`reflect.functions_of(m)` lists the functions of a module that the code
imports by name, `from sauna import stove`: its `def`s, but not its
`comptime def`s or generic functions, and of another module its public
ones only. Each is a descriptor with `name`, `params` (each with
`name`, `index`, `mode` and `ty`, as a method's) and `ret`, its result
type, and `reflect.name_of(f)`, `reflect.doc_of(f)` (its `##` comment)
and `reflect.attributes_of(f)` answer for it. In code that runs, the
descriptor is the function itself: `f(...)` calls it, and a list of
them is a table of function values, as the routes of [chapter
25](../tutorial/25-compile-time.md#attributes-are-data) are. A type
given to `functions_of`, or a type's query given a function, is E0609,
and so is a module of the standard library.

<!-- file: examples/reference-compile-time/functions.pkg/src/stove.sisu -->
```sisu
# The module `sauna.stove`, whose functions `main` lists.

## The heat after `minutes` of firing, from `start`.
@unit("celsius")
pub def heat(start: int, minutes: int) -> int:
    return start + minutes * 2

## Whether the stove is ready for the first turn.
pub def ready(celsius: int) -> bool:
    return celsius >= 70

def private_note() -> String:
    return "seen from this module only".to_string()
```

<!-- file: examples/reference-compile-time/functions.pkg/src/main.sisu -->
```sisu
# `reflect.functions_of` lists a module's functions while compiling.
from sauna import stove

def main():
    comptime for f in reflect.functions_of(stove):
        var params = String.new("")
        var comma = ""
        comptime for p in f.params:
            params = "{params}{comma}{p.name}: {reflect.name_of(p.ty)}"
            comma = ", "
        print("{f.name}({params}) -> {reflect.name_of(f.ret)}")
        print("  {reflect.doc_of(f)}")
        comptime for a in reflect.attributes_of(f):
            print("  @{a.name}({a.args[0]})")
    let warm = stove.heat(20, 30)
    print("{warm} {stove.ready(warm)}")
```

<!-- output: examples/reference-compile-time/functions.console -->
```console
$ sisu run
heat(start: int, minutes: int) -> int
  The heat after `minutes` of firing, from `start`.
  @unit(celsius)
ready(celsius: int) -> bool
  Whether the stove is ready for the first turn.
80 True
```

A table built from the descriptors, `comptime [(a.args[0], f) for f in
reflect.functions_of(m) for a in reflect.attributes_of(f)]`, is
evaluated by the compiler: a function in a compile-time result is kept
as the function it names, so the table is a list of function values at
run time. Written without `comptime`, the comprehension is unrolled
into the same list. A closure in the table is kept when it captures
constants only ([What compile-time code may do](#what-compile-time-code-may-do)).

The attributes of a field, a variant and a method are data too:
`f.attributes` of a field that `reflect.fields_of` gives and of a
variant that `reflect.variants_of` gives, and `reflect.attributes_of(m)`
(or `m.attributes`) of a method that `reflect.methods_of` gives, each
in the form of a type's ([R14](attributes.md#attributes-are-data)); a
variant's field has none. `value.[m]()` calls the method `m` names, so
a template routes a request to the method whose attribute names it, as
[chapter 25](../tutorial/25-compile-time.md#attributes-of-methods-and-fields)
does.

**Stage 0 differs.** Of the documentation of items, which
[08](../../proposal/08-metaprogramming.md#reflection) lists among what
reflection exposes, a type's, its fields', a variant's fields' and a
function's have theirs, but a method's and a variant's own have no
query. The front end written in
Rust has neither `doc` nor `reflect.doc_of`, so the standard library's
doc examples, which `sisu test --std` runs on it, read neither.
Reflection in a refinement's `where` clause or an `invariant` is E0900.

**Status.** In stage 0 (D-018), with the differences noted.

## Templates

A generic function whose body reflects on its type parameters, through
a `reflect` query, `x.[f]` or a `comptime` expression that names one,
is a template: it is not checked as written but made again for each
list of type arguments it is called with, an instance, and each
instance is checked on its own, for the types really used.

- A function without type parameters whose body reflects, such as one
  that reflects on the program's own types, or a test, is made once; a
  `comptime def` that takes a type is made for each type; a method of a
  generic type for each type argument; a trait's provided method in each
  impl that does not write it.
- An instance that cannot be made is E0609
  (`comptime.instance-failed`), at the call in the program that first
  needed it, with a label where the template stopped.
- An error inside an instance, a type's or an ownership's, is placed at
  the call that made the instance, the first in a module the program
  wrote, with a note that names the instance and quotes its line, such
  as "in the instance of `total[Team]` that this call makes", and the
  label `type.instance-line`, "the template's line that wrote it", at the
  statement of the template that wrote that line, the innermost: the
  body of the `comptime for` that unrolled it, or the branch of the `if
  comptime` that chose it. The error's own labels and fixes, which would
  point into the made code, are left out. A template of the standard
  library gets no such label, since a program's diagnostics show none of
  its text. An `E0300` that
  finds a reference where the instance gives a value on, as to
  `reflect.construct`, adds a note that such a parameter is taken `own`.
- A template's compile-time code, its `comptime for`, its `comptime
  if` and the conditions of its `if comptime`, may call `reflect`,
  `enumerate` and `len`; `range` with one, two or three integers;
  `zip` of two lists of one length; `reversed` and `sorted` of a list;
  `min`, `max` and `sum` of a list, or `min` and `max` of two or more
  values; and the methods `to_lowercase`, `to_uppercase`, `to_string`
  and `byte_len` of text. A call of anything else there is E0609, whose
  reason lists what may be called, as does a `range` with a step of 0
  or of more than 100,000 numbers. The code the template writes for run
  time may call anything.
- A typed hole in a derive's template is reported once, at the hole,
  however many types derive it.
- A syntax error in a template's body, such as a `:` that an
  interpolation reads as a format spec, is reported once, at its line
  in the template's file, however many copies the `comptime for`
  unrolls, as the last example of this section shows.
- An interpolated literal of compile-time values only, `var out =
  "{reflect.name_of(T)}"`, stays the `String` it is in the template.
- A template that calls itself with ever more types is E0610
  (`comptime.instances-unbounded`), and a template used as a value,
  rather than called, is E0611 (`comptime.template-value`), since only a
  call makes an instance.

<!-- file: examples/reference-compile-time/template.sisu -->
```sisu
# A template asks what kind of type it was given.
struct Booking:
    name: String

def kind[T](value: T) -> String:
    if comptime reflect.is_int(T):
        return "an int"
    elif comptime reflect.is_str(T):
        return "text"
    elif comptime reflect.is_list(T):
        return "a list of {len(value)}"
    elif comptime reflect.is_option(T):
        return "an optional value"
    elif comptime reflect.implements(T, "Display"):
        return "something that prints"
    else:
        return "a {reflect.name_of(T)}"

def main():
    print(kind(3))
    print(kind("hei"))
    print(kind([1, 2]))
    print(kind(Some(1)))
    print(kind(2.5))
    print(kind(Booking("Aino")))
```

<!-- output: examples/reference-compile-time/template.out -->
```text
an int
text
a list of 2
an optional value
something that prints
a Booking
```

<!-- file: examples/reference-compile-time/template_stops.sisu -->
```sisu
# A template that stops for a type it cannot be made for.
struct Settings:
    heat: int = 80
    name: String

def blank[T](own name: String) -> T:
    if comptime reflect.is_struct(T):
        return reflect.construct(T, {"name": name})
    else:
        comptime panic("`{reflect.name_of(T)}` is not a struct")

def main():
    let settings: Settings = blank("sauna".to_string())
    print("{settings.name} {settings.heat}")
    let count: int = blank("count".to_string())
    print(count)
```

<!-- output: examples/reference-compile-time/template_stops.console -->
```console
$ sisu run template_stops.sisu
error[E0609]: making `blank[int]` stopped at compile time: `int` is not a struct
  --> template_stops.sisu:15:22
   |
15 |     let count: int = blank("count".to_string())
   |                      ^^^^^^^^^^^^^^^^^^^^^^^^^^
   |
10 |         comptime panic("`{reflect.name_of(T)}` is not a struct")
   |                  ----------------------------------------------- the template, where it stopped

error[D0006]: the program was not run: errors found (1)
```

A template sorts its type's field names and writes them in capitals
while it is made, so the instance holds the finished list:

<!-- file: examples/reference-compile-time/template_builtins.sisu -->
```sisu
# A template's compile-time code calls `sorted`, `zip` and text's methods.
struct Stove:
    wood: bool
    kilowatts: int
    name: String

## The fields of `T` in alphabetical order, in capitals.
def headings[T]() -> list[String]:
    var out: list[String] = []
    comptime for name in sorted([f.name for f in reflect.fields_of(T)]):
        out.push(name.to_uppercase())
    return out

def main():
    print(headings[Stove]())
```

<!-- output: examples/reference-compile-time/template_builtins.out -->
```text
["KILOWATTS", "NAME", "WOOD"]
```

A call of a function that compile time does not run, `abs` in a
`comptime if`, stops the instance:

<!-- file: examples/reference-compile-time/template_calls.sisu -->
```sisu
# A template's compile-time code calls `abs`, which it cannot.
struct Stove:
    kilowatts: int

def long_names[T]() -> int:
    var count = 0
    comptime for f in reflect.fields_of(T):
        comptime if abs(len(f.name) - 5) > 2:
            count += 1
    return count

def main():
    print(long_names[Stove]())
```

<!-- output: examples/reference-compile-time/template_calls.console -->
```console
$ sisu run template_calls.sisu
error[E0609]: making `long_names[Stove]` stopped at compile time: only `reflect`, `enumerate`, `len`, `range`, `zip`, `reversed`, `sorted`, `min`, `max`, `sum` and the methods `to_lowercase`, `to_uppercase`, `to_string` and `byte_len` of text can be called at compile time here
  --> template_calls.sisu:13:11
   |
13 |     print(long_names[Stove]())
   |           ^^^^^^^^^^^^^^^^^^^
   |
 8 |         comptime if abs(len(f.name) - 5) > 2:
   |                     -------------------- the template, where it stopped

error[D0006]: the program was not run: errors found (1)
```

Compare the lengths without `abs`, as `len(f.name) > 7 or len(f.name) <
3`, or compute the value in the code the template writes.

A syntax error in a template's body is the template's own:

<!-- file: examples/reference-compile-time/syntax_once.sisu -->
```sisu
# A syntax error in a template's body, reported once, at its line.
struct Point:
    x: int
    y: int

def show[T](v: T) -> String:
    var out = String.new()
    comptime for f in reflect.fields_of(T):
        out.push("{if f.index > 0: ", " else: ""}{f.name}")
    return out

def main():
    print(show(Point(x=1, y=2)))
```

<!-- output: examples/reference-compile-time/syntax_once.console -->
```console
$ sisu run syntax_once.sisu
error[E0103]: expected an expression, found FORMAT_SPEC
  --> syntax_once.sisu:9:35
  |
9 |         out.push("{if f.index > 0: ", " else: ""}{f.name}")
  |                                   ^

error[D0006]: the program was not run: errors found (1)
```

The error stands once, at the template's line, for both of `Point`'s
fields. In parentheses, `{(if f.index > 0: ", " else: "")}`, the `if`
is an expression again, and the `:` no longer starts a format spec.

**Status.** In stage 0 (D-018).

## The derive protocol

A trait makes itself derivable by declaring

`comptime def derive(T: type) -> impl Trait for T`

whose body is `return impl Trait for T:` with the impl's methods,
after checks of the type, if any: `comptime assert CONDITION,
"MESSAGE"`, `comptime panic(MESSAGE)`, and `comptime if` or `if
comptime` whose branches hold only checks. Anything else before or after
the `return impl` is E0607 (`comptime.derive-template`), with the note
`comptime.derive-checks`, which says what may come first.
`@derive(Trait)` on a type runs it for that type: the checks run first
and write no code, the compile-time code in the methods is evaluated,
and what is left becomes the source of an impl declared beside the type
and checked like an impl written by hand.

- A derive writes only an impl of its own trait for the type it is
  given, so coherence holds ([R2](types.md#one-impl-for-each-type-and-trait)).
- The impl of a generic type bounds each type parameter that a field
  uses by the trait, as `impl[T: Eq] Eq for Pair[T]`.
- A derive that stops, by a failing check, `comptime panic` or a query
  that does not apply, is E0608 (`comptime.derive-failed`), at the `@derive` with a
  label where the template stopped; an error in the code it wrote is
  placed at the derive's argument, with a note that quotes the line of
  the impl it wrote.
- The traits of the prelude derive this way; `@derive` and the traits
  it takes are [R14](attributes.md#deriving-traits).

<!-- file: examples/reference-compile-time/derive.sisu -->
```sisu
# A trait's `derive` writes its impl for a type, with reflection.
trait Columns:
    ## The names of the fields, as the columns of a table.
    def columns(self) -> list[String]

    comptime def derive(T: type) -> impl Columns for T:
        return impl Columns for T:
            def columns(self) -> list[String]:
                var names: list[String] = []
                comptime for f in reflect.fields_of(T):
                    names.push(f.name.to_uppercase())
                return names

@derive(Columns)
struct Booking:
    name: String
    hour: int

def main():
    print(Booking("Aino", 18).columns())
```

<!-- output: examples/reference-compile-time/derive.out -->
```text
["NAME", "HOUR"]
```

A `let` before the `return impl` is E0607, whose note says what may
stand there ([R17](diagnostics.md#e0607)). It is the one error: the
`@derive` of the trait and the use of `Booking.width()`, which the
derive would have written, report nothing more:

<!-- output: examples/reference-diagnostics/e0607_derive_template.console -->
```console
$ sisu check e0607_derive_template.sisu
error[E0607]: a trait's derive is `comptime def derive(T: type) -> impl Columns for T`, whose body is `return impl Columns for T:` with the impl's methods
  --> e0607_derive_template.sisu:6:18
  |
6 |     comptime def derive(T: type) -> impl Columns for T:
  |                  ^^^^^^
  = note: only checks of the type may come before the `return impl`: `comptime assert`, and `comptime panic` and `comptime if` of them, which write no code

```

A value the methods need is computed inside them, at compile time,
since a name bound before the `return impl` would be a variable of
every method the engine writes as text; a check is the one statement
that may come first:

<!-- file: examples/reference-diagnostics/e0607_derive_template_fixed.sisu -->
```sisu
# The method counts the fields itself, at compile time.
trait Columns:
    ## The number of the type's fields.
    def width() -> int

    comptime def derive(T: type) -> impl Columns for T:
        comptime assert reflect.is_struct(T), "columns are a struct's"
        return impl Columns for T:
            def width() -> int:
                return comptime len(reflect.fields_of(T))

@derive(Columns)
struct Booking:
    name: String
    hour: int

def main():
    print(Booking.width())
```

<!-- output: examples/reference-diagnostics/e0607_derive_template_fixed.out -->
```text
2
```

A variant's fields have a `ty` as a struct's fields do, so a derive
over an enum reads them inside `comptime for v in
reflect.variants_of(T)`:

<!-- file: examples/reference-compile-time/variants.sisu -->
```sisu
# A derive reads the types of the fields of an enum's variants.
trait Schema:
    ## Each variant with the types of its fields.
    def schema() -> list[String]

    comptime def derive(T: type) -> impl Schema for T:
        return impl Schema for T:
            def schema() -> list[String]:
                var lines: list[String] = []
                comptime for v in reflect.variants_of(T):
                    var types: list[String] = []
                    comptime for f in v.fields:
                        let name = reflect.name_of(f.ty)
                        if comptime reflect.is_struct(f.ty):
                            types.push("{name} (a struct)")
                        else:
                            types.push(name.to_string())
                    lines.push("{v.name}: {types.join(", ")}")
                return lines

## A volume of water.
struct Litres:
    amount: f64

## How a sauna is heated.
@derive(Schema)
enum Heater:
    Wood(logs: int)
    Electric(kilowatts: f64, phase: String)
    Water(tank: Litres)

def main():
    for line in Heater.schema():
        print(line)
```

<!-- output: examples/reference-compile-time/variants.out -->
```text
Wood: int
Electric: f64, String
Water: Litres (a struct)
```

**Stage 0 differs.** Inside a derive's template, which runs before the
types are resolved, a field's `ty`, of a struct or of a variant, is the
type as its declaration writes it: `reflect.name_of(f.ty)` is that
text, and `reflect.is_str`, `is_list` and the other kinds are known for
the prelude's types (`int`, `f64`, `bool`, `char`, `str`, `String`, a
list, an optional value and a tuple of them) and for the structs and
enums the module declares or imports by name, so a derive can choose a
form field by field. What a type implements is not known there, and
the fields of a program's own type are not looked into.

**Status.** In stage 0 (D-018).

## Embedding a file

`embed("PATH")` is the text of a file, read while the program compiles,
as a `str`; the path is a string literal (E0613, `comptime.embed-path`)
that names a file inside the program's directory, relative to the
package's root, the directory of its `sisu.toml`, in a package, and to
the file's directory for a single file. A file that cannot be
read, or a path outside the directory, is E0612
(`comptime.embed-failed`), with a reason that is the same on every
machine. The file is one of the build's inputs, so a build that embeds
it gives the same program whenever the file is the same, and the
program needs no `Fs` to read it.

<!-- file: examples/reference-compile-time/embed.sisu -->
```sisu
# `embed` makes a file's text part of the program while it compiles.
const WORDS: str = embed("words.txt")

def main():
    let words = WORDS.split_whitespace().collect()
    print("{len(words)} words, the first {words[0]}")
```

<!-- output: examples/reference-compile-time/embed.out -->
```text
3 words, the first kiuas
```

A failing test's replay token covers the files the program embeds as
well as its source, so a token made before one of them changed is
refused ([guide](../guides/embed.md#4-know-what-the-build-records)).

**Status.** In stage 0 (D-018).

## The facts of the target

`target.os`, `target.arch` and `target.runtime` are compile-time
constants that describe what the program is compiled for, so `if
comptime target.os == "windows":` chooses code for a system while both
branches are checked ([R22](targets.md#the-facts-of-the-target)). The
one exception is a name that an import under `@cfg(target.os ==
"windows")` binds: the branch that the build leaves out may use it, and
is not checked for it, since the build has no such name
([R13](modules.md#target-conditions)).

`target.runtime` is `native` in a native build and `interpreter`
otherwise, and `target.runtime_profile` is the runtime profile of the
package's manifest, `"full"` or `"single"`
([R22](targets.md#runtime-profiles)).

An import under `@cfg(target.runtime == "native")`, and a dependency
whose `when` names the runtime, read the same runtime as a body's `if
comptime`, so a native build imports what its `native` branch uses
([R13](modules.md#target-conditions)).

**Status.** In stage 0.

## Conditional compilation and features

`@cfg(CONDITION)` leaves out a declaration whose condition does not
hold, an item or a member of a type alike, before names are resolved:
`test`, `feature="NAME"`, `not`, `all(...)` and `any(...)`
([R14](attributes.md#conditional-compilation)).
Inside a body, `if comptime` chooses instead, and keeps both branches
checked, so code for a platform that one build does not use keeps
compiling. A feature is a package's named, additive part
([R13](modules.md#features)).

**Status.** In stage 0 (08).

## Configuration files

`sisu eval FILE [--to json|toml|yaml]` checks a module as `sisu check`
checks a file, without its tests, evaluates each of its `const` items
with the compile-time evaluator, in the order written, and writes them
in the form `--to` names, JSON without it: one object, a TOML document,
or one mapping in block style, keyed by the constants' names. Sisu is so its own configuration language: the
module's types, refinements and invariants are the schema, and a value
outside them fails with a diagnostic at the constant's name, as E0602
for a failed invariant, with nothing written and status 1. The budget
of steps applies, since a configuration module is ordinary code
([08](../../proposal/08-metaprogramming.md#configuration-files)).

- A struct is a mapping of its fields, in order; a variant without
  fields is its name, and one with fields a mapping of its name to
  them.
- `Some(x)` is `x`, and `None` is `null` in JSON and YAML; TOML leaves
  out a field or a constant that holds nothing.
- A list, a set and a tuple are sequences, a `dict` keyed by text or
  integers a mapping, and text is written in double quotes.
- TOML writes the values that fit on a line first, `NAME = value`, in
  the order written, and then each struct, `dict` or variant with
  fields as a table, `[LIMITS]`, its fields of those kinds as tables
  under it, `[LIMITS.retry]`, and a list whose every item is one as an
  array of tables, a `[[SERVICES]]` header for each item. A list of
  values, or one that mixes mappings with other values, stays on its
  line.
- A value the form cannot write, a function, a reference, an infinite
  float in JSON or a `None` in a list in TOML, is E0617
  (`eval.no-form`, [R17](diagnostics.md#e0617)), at the constant's name,
  and its message gives the path of the value from the constant:
  `LIMITS.retry.backoff`, `MAYBE[0]`.

Every command that checks a program computes its constants too, so
`sisu check` reports the same failures as `sisu eval`, all but E0617,
which only a form has ([R3](declarations.md#constants)).

A constant may call the prelude's functions as code that runs does:
`min` and `max`, of two values or of a collection, `abs`, `any`, `all`,
`sorted`, `enumerate`, `zip`, `zip_shortest` and `reversed`, besides
`range`, `len` and `sum`:

<!-- file: examples/reference-compile-time/benches.sisu -->
```sisu
# A configuration's constants may call the prelude's functions.
const BENCHES: list[int] = [4, 9, 2]
const HIGHEST: int = max(BENCHES)
const LOWEST: int = min(4, 9)
const ORDER: list[int] = sorted(BENCHES)
const ALL_WARM: bool = all([b > 1 for b in BENCHES])
const PAIRS: list[(int, int)] = [(i, b) for i, b in enumerate(BENCHES)]
```

<!-- output: examples/reference-compile-time/benches.console -->
```console
$ sisu eval benches.sisu --to toml
BENCHES = [4, 9, 2]
HIGHEST = 9
LOWEST = 4
ORDER = [2, 4, 9]
ALL_WARM = true
PAIRS = [[0, 4], [1, 9], [2, 2]]
```

A call that panics when the program runs stops the evaluation in the
same way: `abs` of the least `int` overflows, and `zip` of two lists of
different lengths is refused, each an error at its constant's name.

[Chapter 25](../tutorial/25-compile-time.md#configuration-as-constants)
shows a first module, and [Generate configuration](../guides/config.md)
each form and each refusal.

**Stage 0 differs.** 08 renders the values through `Encode`; stage 0
writes a value's own structure, which is what `@derive(Encode)` writes
of it, whether or not the type derives it. `sisu eval` is stage 1's, so the Rust
front end answers it with D0003 ([R16](sisu-command.md#sisu-eval)).

**Status.** In stage 0, with the differences noted; rendering through
`Encode` leans (D-063).

## Checked string literals

A `comptime` function that parses a string literal and returns a typed
value gives a language of its own, such as a query, a route or a
regular expression, without new syntax: the literal is checked while
the program compiles, and a mistake is reported at its place inside the
literal ([08](../../proposal/08-metaprogramming.md#what-you-cannot-do-and-the-intended-workaround)).

**Stage 0 differs.** A failed `comptime` call points at the call, not
inside the literal.

**Status.** Designed (D-018).

## What compile-time code leaves out

- No macros of any kind: no textual, pattern or procedural macros, and
  no AST macros, ever (Q-15). A new syntax is a checked string literal;
  a new control structure is a closure with a trailing `do` block;
  generated functions are a `comptime for` over a table, or generics.
- No declarations made by code, besides a derive's impl.
- No reflection at run time beyond a type's name and the names of an
  enum's variants: more would stop the removal of unused code and break
  the model of capabilities ([08](../../proposal/08-metaprogramming.md#decision)).
- No I/O while compiling but `embed`, and no build scripts (D-032).

**Status.** Decided (D-018, D-032, Q-15).

## Diagnostics of this page

| Code | Identifier | Section |
|---|---|---|
| [E0605](diagnostics.md#e0605) | `comptime.only` | [Asking for compile-time evaluation](#asking-for-compile-time-evaluation) |
| E0600, [E0601](diagnostics.md#e0601), [E0602](diagnostics.md#e0602), [E0604](diagnostics.md#e0604), E0615 | `comptime.not-constant`, `comptime.io`, `comptime.panicked`, `comptime.not-kept`, `comptime.unsafe-written` | [What compile-time code may do](#what-compile-time-code-may-do) |
| [E0603](diagnostics.md#e0603), [E0606](diagnostics.md#e0606) | `comptime.budget`, `comptime.budget-attribute` | [Budgets](#budgets) |
| [E0200](diagnostics.md#e0200), [E0900](diagnostics.md#e0900), [E0616](diagnostics.md#e0616) | `resolve.undefined-name`, `stage0.unsupported`, `comptime.function-failed` | [Reflection](#reflection) |
| [E0103](diagnostics.md#e0103) | `parse.expected-expression` | [Templates](#templates) |
| [E0609](diagnostics.md#e0609), E0610, E0611 | `comptime.instance-failed`, `comptime.instances-unbounded`, `comptime.template-value` | [Templates](#templates) |
| [E0607](diagnostics.md#e0607), [E0608](diagnostics.md#e0608) | `comptime.derive-template`, `comptime.derive-failed` | [The derive protocol](#the-derive-protocol) |
| [E0612](diagnostics.md#e0612), [E0613](diagnostics.md#e0613) | `comptime.embed-failed`, `comptime.embed-path` | [Embedding a file](#embedding-a-file) |
| [D0003](diagnostics.md#d0003) | `cli.unknown-command` | [Configuration files](#configuration-files) |
