# Write a derive

This guide gives a trait a derive, so that `@derive(Trait)` writes the
trait's impl for any struct from its fields: it writes the derive over
`reflect.fields_of`, refuses the types it cannot handle, and tests it
on more than one type. It assumes traits and compile-time code, from
chapters 14 and 25 of the tutorial.

**See also:** the tutorial's
[chapter 25](../tutorial/25-compile-time.md#writing-a-derive) (a first
derive, and reflection); the source of `std.x.cli`,
[`compiler/std/cli.sisu`](../../compiler/std/cli.sisu), which reads a
struct's fields in the same way; the proposal's
[08, Metaprogramming](../../proposal/08-metaprogramming.md) and D-018.

## 1. Declare the trait

Write the trait with the methods that the derive will implement, as
for any trait: here, a header of column names and a row of values, for
writing structs as comma-separated values. The whole file, which the
next steps take apart:

<!-- file: examples/guide-derive/csv.sisu -->
```sisu
# A derive that writes a struct's lines of comma-separated values.
## A value written as a line of comma-separated values.
trait Csv:
    ## The names of the columns, separated by commas.
    def header() -> String
    ## The values of the fields, separated by commas.
    def row(self) -> String

    comptime def derive(T: type) -> impl Csv for T:
        comptime if not reflect.is_struct(T):
            comptime panic("`{reflect.name_of(T)}` is not a struct")
        return impl Csv for T:
            def header() -> String:
                var names: list[String] = []
                comptime for f in reflect.fields_of(T):
                    names.push(f.name.to_string())
                return names.join(",")

            def row(self) -> String:
                var cells: list[String] = []
                comptime for f in reflect.fields_of(T):
                    cells.push("{self.[f]}")
                return cells.join(",")

## A turn in the sauna.
@derive(Csv)
struct Booking:
    name: String
    hour: int
    paid: bool = False

@derive(Csv)
struct Reading:
    minute: int
    celsius: f64

test "a booking and a reading as lines":
    assert Booking.header() == "name,hour,paid"
    assert Booking(name="Aino", hour=18).row() == "Aino,18,False"
    assert Reading(minute=5, celsius=78.5).row() == "5,78.5"
```

## 2. Write the derive

Inside the trait, `comptime def derive(T: type) -> impl Csv for T:`
runs while compiling, once for each type that asks for it, and its body
is `return impl Csv for T:` followed by the impl's methods:

- `comptime for f in reflect.fields_of(T):` unrolls its body once for
  each field, in order. `f.name` is the field's name, and `self.[f]` the
  field of the value.
- What is left after the unrolling is ordinary code, checked as an impl
  you wrote would be, so a derive cannot write code that does not
  type-check.
- The derive writes only the impl of its own trait, for the type it is
  given.

## 3. Use it, and test it on more than one type

`@derive(Csv)` on a struct runs the derive for it. Test the derived
methods on structs with different fields, including one with a
default, since each struct gets code of its own:

<!-- output: examples/guide-derive/csv.test -->
```text
test "a booking and a reading as lines" ... ok
1 passed, 0 failed, 0 blocked
```

## 4. Refuse the types it cannot handle

Check the type's shape before the `return impl`: `comptime if not
reflect.is_struct(T):` with `comptime panic(message)`, or `comptime
assert CONDITION, "MESSAGE"`, runs first for each type, writes no code,
and stops the compilation where `@derive` asks for the wrong kind of
type. Only such checks may come before the `return impl`; anything else
there, a `let` among them, is E0607 (`comptime.derive-template`), whose
note says what may come first, so a value the methods need is computed
inside them ([R12](../reference/compile-time.md#the-derive-protocol)).
Asked for an enum, the derive of step 1 says:

<!-- output: examples/guide-derive/enum.console -->
```console
$ sisu check enum.sisu
error[E0608]: deriving `Csv` for `Heater` stopped at compile time: `Heater` is not a struct
  --> enum.sisu:26:9
   |
26 | @derive(Csv)
   |         ^^^
   |
11 |             comptime panic("`{reflect.name_of(T)}` is not a struct")
   |                      ----------------------------------------------- the derive's template, where it stopped

```

E0608 (`comptime.derive-failed`) points at the `@derive` that asked,
with the message of the `comptime panic`, and at the line of the derive
where it stopped. Write the message for the person who wrote the
`@derive`: it is all they see of your code.

## 5. Say what each field needs

The derived `row` interpolates each field, so each field's type needs
`Display`. A struct with a field that has none gets E0347 at the
`@derive` that asked for the impl, with a note that quotes the line of
the code that the derive wrote:

<!-- output: examples/guide-derive/stove.console -->
```console
$ sisu check stove.sisu
error[E0347]: `Stove` does not implement `Display`
  --> stove.sisu:30:9
   |
30 | @derive(Csv)
   |         ^^^
  = note: in the impl of `Csv` that the derive writes for `Sauna`: `cells.push("{self.stove}")`

```

E0347 offers no fix here, since the line it quotes is the derive's,
which the person who wrote the `@derive` cannot edit: implement
`Display` for `Stove` instead. Say in the trait's documentation what each field's
type must implement, so that the error is no surprise.

**Status.** In stage 0 (D-018). A derive's template runs before the
types are resolved, so a field's `f.ty` is the type as the field
writes it, for a struct's fields and a variant's alike: the kinds of
the prelude's types are known (`reflect.is_str(f.ty)` chooses text),
and so are those of the structs and enums the module declares or
imports (`reflect.is_struct(f.ty)`;
[R12](../reference/compile-time.md#the-derive-protocol) has a derive
over an enum's variants), but not what a type implements,
and `reflect.implements(f.ty, "Display")`, which a generic function
that reflects on its type parameter can ask, is `False` there for every
field, a `String` included.
