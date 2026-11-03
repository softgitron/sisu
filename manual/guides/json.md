# Read and write JSON

This guide writes values as JSON, reads JSON back into a type, reports
text that does not fit, checks what a field's type cannot say, and
reads JSON whose shape is not known in advance. It assumes structs and
errors as values, from chapters 7 and 8 of the tutorial.

**See also:** the tutorial's
[chapter 21](../tutorial/21-standard-library.md#json) (`std.json`) and
[chapter 23](../tutorial/23-contracts.md) (refinement types and
`try_from`); the source of `std.json`,
[`compiler/std/json.sisu`](../../compiler/std/json.sisu), which holds
its documentation; the proposal's
[11, Standard library](../../proposal/11-stdlib.md).

## 1. Write the struct, and encode it

Write a struct with a field for each member of the JSON object, and
`json.encode(value)` writes it: a struct as an object of its fields in
order, a list as an array, `None` as `null`. This program writes two
bookings to a file and prints what it wrote:

<!-- file: examples/guide-json/write.sisu -->
```sisu
# Writes a list of bookings to a file as JSON.
import std.json

## A turn in the sauna, as the JSON has it.
struct Booking:
    name: String
    hour: int
    towels: int = 1
    note: String? = None

def main(sys: System) -> ! IoError:
    let bookings = [
        Booking(name="Aino", hour=18),
        Booking(name="Eero", hour=19, towels=2, note=Some("late")),
    ]
    sys.fs.write_text("bookings.json", json.encode(bookings))?
    print(sys.fs.read_to_string("bookings.json")?)
    return Ok(())
```

<!-- output: examples/guide-json/write.out -->
```text
[{"name":"Aino","hour":18,"towels":1,"note":null},{"name":"Eero","hour":19,"towels":2,"note":"late"}]
```

The struct is the format, so nothing else needs writing: `std.json`
reads the fields by reflection, for each type it is called with.

## 2. Decode JSON into the struct

`json.decode[T](text)` reads text into a `T` and gives `T !
json.JsonError`. This program reads the file that step 1 wrote, laid
out by hand, with the members that have defaults left out of Aino's
booking:

<!-- file: examples/guide-json/read.files/bookings.json -->
```text
[
  {"name": "Aino", "hour": 18},
  {"name": "Eero", "hour": 19, "towels": 2, "note": "late"}
]
```

<!-- file: examples/guide-json/read.sisu -->
```sisu
# Reads a file of JSON into a list of bookings.
import std.json

## A turn in the sauna, as the JSON has it.
struct Booking:
    name: String
    hour: int
    towels: int = 1
    note: String? = None

def main(sys: System) -> ! {IoError, json.JsonError}:
    let text = sys.fs.read_to_string("bookings.json")?
    let bookings = json.decode[list[Booking]](text)?
    for b in bookings:
        print("{b.hour}:00 {b.name}, towels {b.towels}, note {b.note:?}")
    return Ok(())
```

<!-- output: examples/guide-json/read.out -->
```text
18:00 Aino, towels 1, note None
19:00 Eero, towels 2, note Some("late")
```

- A member that the text leaves out takes the field's default, so
  `towels: int = 1` may be missing, and a field of type `String?` with
  the default `None` may be missing or `null`.
- A member the struct does not have is skipped.

## 3. Report text that does not fit

The error tells text that is not JSON, `Syntax(at, expected)` at a
character's index, from JSON that does not fit the type, `Mismatch(path,
expected)` with the path to the value from the root: `$[1].hour` is the
member `hour` of the second element. This program decodes five texts
and prints what became of each:

<!-- file: examples/guide-json/errors.sisu -->
```sisu
# What `json.decode` says about text that does not fit the type.
import std.json

## A turn in the sauna, as the JSON has it.
struct Booking:
    name: String
    hour: int

def main():
    let texts = [
        r"""[{"name": "Aino", "hour": 18, "sauna": "Kota"}]""",
        r"""[{"name": "Aino", "hour": "six"}]""",
        r"""[{"name": "Aino", "hour": 18}, {"name": "Eero"}]""",
        r"""{"name": "Aino", "hour": 18}""",
        r"""[{"name": "Aino", "hour": 18]""",
    ]
    for text in texts:
        match json.decode[list[Booking]](text):
            case Ok(bookings):
                print("{len(bookings)} booking: {bookings[0].name}")
            case Err(e):
                print(e)
```

<!-- output: examples/guide-json/errors.out -->
```text
1 booking: Aino
Mismatch(path="$[0].hour", expected="an integer")
Mismatch(path="$[1].hour", expected="a member")
Mismatch(path="$", expected="an array")
Syntax(at=28, expected="`,` or `}`")
```

Report the path with the error: it is what someone needs to fix the
text.

## 4. Check what a field's type cannot say

A type says that `hour` is an integer, and a refinement type says
more: `type Hour = int where 17 <= self and self <= 21` holds only the
hours when the sauna is warm. Decode the plain shape, then check each
such field with `try_from`, which gives a `RefinementError` rather
than a panic:

<!-- file: examples/guide-json/checked.sisu -->
```sisu
# Checks what a field's type cannot say, once the JSON is decoded.
import std.json

## An hour of the evening, when the sauna is warm.
type Hour = int where 17 <= self and self <= 21

## A turn in the sauna, as the JSON has it.
struct Booking:
    name: String
    hour: int

## The name and the evening hour of the booking that `text` holds.
def turn(text: str) -> (String, Hour) ! {json.JsonError, RefinementError}:
    let booking = json.decode[Booking](text)?
    let hour = Hour.try_from(booking.hour)?
    return Ok((booking.name, hour))

def main():
    print(turn(r"""{"name": "Aino", "hour": 18}"""))
    print(turn(r"""{"name": "Eero", "hour": 25}"""))
    print(turn(r"""{"name": "Roni", "hour": "six"}"""))
```

<!-- output: examples/guide-json/checked.out -->
```text
Ok(("Aino", 18))
Err(Unsatisfied(ty="Hour", predicate="17 <= self and self <= 21"))
Err(Mismatch(path="$.hour", expected="an integer"))
```

Do the check where the text comes in, so that the rest of the program
receives a `Hour` and never needs to check it again.

A field of the refinement type itself, `hour: Hour`, does not compile
with `json.decode` yet. E0811 (`refine.needs-try-from`) reports it
inside the decoder that `std.json` makes for the type, at a line of its
own:

<!-- file: examples/guide-json/refined.sisu -->
```sisu
# A field of a refinement type, which `json.decode` cannot read yet.
import std.json

## An hour of the evening, when the sauna is warm.
type Hour = int where 17 <= self and self <= 21

## A turn in the sauna, with the hour checked.
struct Booking:
    name: String
    hour: Hour

def main():
    print(json.decode[Booking](r"""{"name": "Eero", "hour": 25}"""))
```

<!-- output: examples/guide-json/refined.console -->
```console
$ sisu check refined.sisu
error[E0811]: expected `Hour`, found `int`; a value becomes a `Hour` where its predicate is checked, with `Hour.try_from(...)`
  --> <std>/<from_json[Hour]>/json.sisu:3:19
  |
3 |         return Ok(value)
  |                   ^^^^^
  = note: check it where it runs: `Hour.try_from(...)`

```

## 5. Read JSON of an unknown shape

`json.parse(text)` reads any JSON into a `Json` value, whose variants
are `Null`, `Bool`, `Int`, `Float`, `Str`, `Array` and `Object`, and
`get(name)` finds a member of an object. A `Json` value prints as
compact JSON. This program looks for four members and says what it
found:

<!-- file: examples/guide-json/shape.sisu -->
```sisu
# Reads JSON of a shape not known in advance into a `Json` value.
import std.json

def main() -> ! json.JsonError:
    let text = r"""{"sauna": "Kota", "open": [17, 18], "lake": 14.5}"""
    let doc = json.parse(text)?
    for key in ["sauna", "open", "lake", "towels"]:
        match doc.get(key):
            case Some(Str(value)):
                print("{key}: the text {value}")
            case Some(Array(items)):
                print("{key}: {len(items)} items, {items[0]} first")
            case Some(other):
                print("{key}: {other}")
            case None:
                print("{key}: missing")
    return Ok(())
```

<!-- output: examples/guide-json/shape.out -->
```text
sauna: the text Kota
open: 2 items, 17 first
lake: 14.5
towels: missing
```

Prefer a struct and `decode` wherever the shape is known: the type then
does the checking that a `match` on `Json` does by hand.

**Status.** In stage 0, with two differences from
[11](../../proposal/11-stdlib.md): `@derive(Encode, Decode)` is not
needed and not accepted (E0243), since `encode` and `decode` read any
struct by reflection; and `decode` does not check refinements at the
boundary, as 11 plans. A field of a refinement type is E0811, as step 4
shows, and text that breaks a struct's `invariant` makes `decode`
panic, so decode such a struct's plain shape and check it with
`try_from`. An enum is encoded and not decoded.
