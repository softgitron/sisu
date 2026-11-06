# Read and write JSON

This guide writes values as JSON, reads JSON back into a type, reports
text that does not fit, lets refinement types and invariants check the
text where it comes in, gives a type a JSON form of its own, and reads
JSON whose shape is not known in advance. It assumes structs and errors
as values, from chapters 7 and 8 of the tutorial.

**See also:** the tutorial's
[chapter 21](../tutorial/21-standard-library.md#json) (`std.json`) and
[chapter 23](../tutorial/23-contracts.md) (refinement types and
`try_from`); the source of `std.json`,
[`compiler/stage1/std/json.sisu`](../../compiler/stage1/std/json.sisu),
stage 1's copy, which declares `Encode` and `Decode` and holds
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

## 4. Let the types check the text

A type says that `hour` is an integer, and a refinement type says
more: `type Hour = int where 17 <= self and self <= 21` holds only the
hours when the sauna is warm. `decode` checks a field of such a type
where the text comes in, and a struct's `invariant` once its fields are
read, so text that breaks either is a `Mismatch` with the path and the
type, as any text that does not fit is:

<!-- file: examples/guide-json/refined.sisu -->
```sisu
# `decode` checks refinements and invariants where the text comes in.
import std.json

## An hour of the evening, when the sauna is warm.
type Hour = int where 17 <= self and self <= 21

## A turn in the sauna, from its first hour to its last.
struct Turn:
    name: String
    first: Hour
    last: Hour
    invariant self.first <= self.last

def main():
    let texts = [
        r"""{"name": "Aino", "first": 18, "last": 19}""",
        r"""{"name": "Eero", "first": 18, "last": 25}""",
        r"""{"name": "Roni", "first": 20, "last": 18}""",
    ]
    for text in texts:
        print(json.decode[Turn](text))
```

<!-- output: examples/guide-json/refined.out -->
```text
Ok(Turn(name="Aino", first=18, last=19))
Err(Mismatch(path="$.last", expected="`Hour`"))
Err(Mismatch(path="$", expected="`Turn` whose invariant holds"))
```

The rest of the program then receives a `Turn` whose hours are hours of
the evening and in order, and never needs to check them again: untrusted
text is malformed input, which `decode` reports as an error, never a
panic. Write the rules into the types, and decode into them.

## 5. Give a type a JSON form of its own

`@derive(Encode, Decode)` gives a type the form that `encode` and
`decode` give any type of its shape, and says in the declaration that
JSON is one of the type's forms. The traits are `std.json`'s, so the
file imports them. An impl written by hand gives a type a form of its
own, which `encode` uses wherever the type is, inside other values too:

<!-- file: examples/guide-json/derive.sisu -->
```sisu
# `@derive(Encode, Decode)`, and an `Encode` written by hand.
import std.json
from std.json import Decode, Encode

## A sum in cents, which the JSON writes as text with two decimals.
struct Money:
    cents: int

impl Encode for Money:
    def encode_json(self) -> String:
        return "\"{self.cents // 100}.{self.cents % 100:02}\""

## A turn in the sauna, as the JSON has it.
@derive(Encode, Decode, Debug)
struct Booking:
    name: String
    hour: int

## What a booking costs.
@derive(Encode)
struct Receipt:
    booking: Booking
    price: Money

def main() -> ! json.JsonError:
    let b = json.decode[Booking](r"""{"name": "Aino", "hour": 18}""")?
    print("{b:?}")
    print(json.encode(Receipt(booking=b, price=Money(cents=1250))))
    return Ok(())
```

<!-- output: examples/guide-json/derive.out -->
```text
Booking(name="Aino", hour=18)
{"booking":{"name":"Aino","hour":18},"price":"12.50"}
```

`Money` is written as text with two decimals, `"12.50"`, inside the
`Receipt` that holds it, where its shape would be `{"cents":1250}`. A
`Decode` written by hand, `decode_json(j: Json, path: str) -> Self !
JsonError`, reads such a form back.

## 6. Read JSON of an unknown shape

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

**Status.** In stage 0 (11), with two differences: an enum is encoded
and not decoded, since `decode` makes no enum's variants yet (E0609),
and `@derive(Decode)`, which 11 gives a struct, is E0608 on an enum; a
struct that derives `Decode` and has a field of an enum passes
`sisu check`, and `sisu run` stops it at E0501, an error of the
compiler's own. Without `from std.json import Decode, Encode`, the
derive is E0243, whose note names the import.
