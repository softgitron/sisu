# 10. Ownership and moves

In this chapter you learn who owns each value in a Sisu program, and
what happens to a value when you give it to another binding, pass it to
a function or put it in a list: it moves, or it is copied. You also
learn when a value ends, and how to run code at that moment. Chapter 5
met the first of these rules. This chapter gives the rest, and shows
that each diagnostic of the ownership check says where a value went and
how to get it back. You need about an hour and a half.

## Every value has one owner

Each value in a running program belongs to exactly one owner: a
binding, a field of a struct, or a collection that holds it. This
program makes a list of guests and prints it:

<!-- file: examples/10-ownership/owners.sisu -->
```sisu
# Every value has one owner: a binding, a field or a collection.
struct Guest:
    name: String
    nights: int

def guest_list(names: list[str]) -> list[Guest]:
    var guests: list[Guest] = []
    for name in names:
        let guest = Guest(name.to_string(), 2)
        guests.push(guest)
    return guests

def main():
    let guests = guest_list(["Aino", "Eero", "Roni"])
    for guest in guests:
        print("{guest.name} stays {guest.nights} nights")
```

<!-- output: examples/10-ownership/owners.out -->
```text
Aino stays 2 nights
Eero stays 2 nights
Roni stays 2 nights
```

Follow one guest through the program:

- `name.to_string()` makes a `String`, and `Guest(...)` takes it as its
  field `name`. The guest owns its name.
- `let guest = ...` makes the binding `guest` the owner of the guest.
- `guests.push(guest)` moves the guest into the list. The list owns it
  now, and `guest` owns nothing until the next round of the loop gives
  it a new guest.
- `return guests` moves the list, with every guest in it, to the
  caller. In `main`, the binding `guests` owns the list.
- When `main` ends, `guests` goes out of scope, and the list is
  dropped: its memory is released, with every guest in it and every
  name in each guest.

No guest was copied: three were made, each passed from owner to owner,
and each ended once. That is the whole model, in three rules:

1. Every value has exactly one owner.
2. An owner can give its value away. That is a move, and the old owner
   cannot use the value afterwards.
3. When the scope of an owner ends, the value it still owns is dropped.

The compiler checks the rules before the program runs, so it knows
where each value ends. Sisu needs no garbage collector to find the
values that nothing uses any more, and keeps no count of the names that
refer to a value (D-004).

> **From Python.** In Python, `guests.append(guest)` puts a second
> reference to the object into the list, and `guest` still refers to
> it; the object lives while any reference does. In Sisu the list holds
> the guest and `guest` does not, so the guest's end is in one place.

## Giving a value to a function

Chapter 5 showed that a function borrows the list it is given, and the
caller keeps it. A function can also take a value for good, when it
needs to keep it. Its parameter then says `own`:

<!-- file: examples/10-ownership/playlist.sisu -->
```sisu
# An `own` parameter keeps what it is given; a plain one borrows it.
struct Song:
    title: String
    seconds: int

struct Playlist:
    songs: list[Song]

    def add(mut self, own song: Song):
        self.songs.push(song)

    def length(self) -> int:
        return sum([song.seconds for song in self.songs])

def describe(song: Song) -> String:
    return "{song.title}, {song.seconds} s"

def main():
    var evening = Playlist([])
    let song = Song("Northern Lights", 245)
    print(describe(song))
    evening.add(song)
    evening.add(Song("Lake Song", 187))
    print("{len(evening.songs)} songs, {evening.length()} s")
```

<!-- output: examples/10-ownership/playlist.out -->
```text
Northern Lights, 245 s
2 songs, 432 s
```

- `describe(song: Song)` has a plain parameter, which borrows: the
  function reads the song during the call, and the caller keeps it.
  Chapter 11 covers borrowing and the third convention, `mut`.
- `add(mut self, own song: Song)` keeps the song in the playlist, so it
  takes it with `own`. The call is written like any other call,
  `evening.add(song)`, and the signature says what happens: after the
  call, the playlist owns the song and `song` owns nothing.
- `Song("Lake Song", 187)` is made in the call and moved straight into
  the playlist, without a binding of its own.

A move copies nothing that the song owns; only the owner changes. A
value moves wherever it goes somewhere new: a `let` or `var` that takes
another binding's value, an assignment, an `own` parameter, a `return`,
a field of a new struct, an element of a list. The prelude declares
`push` as `push(mut self, own x: T)`, which is why `guests.push(guest)`
moved the guest.

**Try it.** Add `print(describe(song))` as the last line of `main`,
and run the program.

<details>
<summary>What Sisu says</summary>

<!-- output: examples/10-ownership/song_moved.console -->
```console
$ sisu run song_moved.sisu
error[E0400]: `song` was moved on line 22 and cannot be used afterwards
  --> song_moved.sisu:25:20
   |
25 |     print(describe(song))
   |                    ^^^^
   |
22 |     evening.add(song)
   |     ----------------- `song` is moved here
  = note: copy it with `.copy()` before the move, which makes a second value (for a list or a `String`, an allocation); move it later, after its last use, which costs nothing; or, where what it moves into only reads it, take it there without `own`, and the call lends it

error[D0021]: the program was not run: the ownership checker found errors (1); `--defer-borrow` runs it under the monitor
```

E0400 (`borrow.use-after-move`) tells you where the song went: the
line marked with `^` is the use, and the line marked with `-` is the
move, however far apart they are. The note names three fixes and what
each costs, which the section "Choosing a fix for a move" weighs. The
last line says that the
program was not run; chapter 11 shows `--defer-borrow`, which runs it
under a monitor that stops it where the moved value is used.

</details>

## Values that are copied

Some values never move. Giving a number, or a small value made of
numbers, to another binding copies it, and both bindings keep a value:

<!-- file: examples/10-ownership/copies.sisu -->
```sisu
# A value of a `Copy` type is copied, so both names keep a value.
@derive(Copy)
struct Point:
    x: int
    y: int

def main():
    let start = Point(0, 0)
    var here = start
    here.x += 3
    print("{start:?} {here:?}")
    let pair = (7, "Aino")
    let other = pair
    print("{pair} {other}")
```

<!-- output: examples/10-ownership/copies.out -->
```text
Point(x=0, y=0) Point(x=3, y=0)
(7, "Aino") (7, "Aino")
```

`var here = start` copies the point, so changing `here` leaves `start`
as it was. A type whose values are copied in this way is a `Copy`
type. These are:

- the numbers, `bool` and `char`;
- `str`, which is a view of text kept somewhere else: copying it copies
  where the text is, not the text, as chapter 12 explains;
- tuples and arrays of `Copy` values, such as `(7, "Aino")`;
- a struct or an enum that derives `Copy`, as `Point` does with
  `@derive(Copy)`, which chapter 7 met.

`String`, `list`, `dict` and `set` are not `Copy`. What they hold is in
memory of its own, of any size, so copying one takes time and memory in
proportion to its size. Sisu does that only where you write it
(D-007). For the same reason, a struct with a `String` field cannot
derive `Copy`: it gets E0244 (`resolve.derive-field`), as chapter 7
said.

## Copying on purpose

When you need two values that change separately, `.copy()` makes the
second:

<!-- file: examples/10-ownership/usual_order.sisu -->
```sisu
# `.copy()` makes a second value, which changes on its own.
struct Order:
    customer: String
    items: list[String]

def main():
    let usual = Order("Aino", ["coffee", "bun"])
    var today = usual.copy()
    today.customer = "Eero"
    today.items.push("juice")
    print("{usual:?}")
    print("{today:?}")
```

<!-- output: examples/10-ownership/usual_order.out -->
```text
Order(customer="Aino", items=["coffee", "bun"])
Order(customer="Eero", items=["coffee", "bun", "juice"])
```

`usual.copy()` copies the order, the customer's name, the list and each
item in it, so `today` owns a whole order of its own. Every type has
`.copy()`, including the ones you declare.

The cost of a copy grows with the value: copying a list of a million
items copies a million items. That is why Sisu copies a `String` or a
collection only where you write `.copy()`, and why a copy is the fix to
reach for last, as the next section shows.

> **From Python.** `today = usual` in Python makes a second name for
> the same order, so `today.items.append("juice")` changes what `usual`
> holds too. `copy.copy` copies the order but shares its list, and
> `copy.deepcopy` copies everything. Sisu's `.copy()` is closer to
> `deepcopy`: the copy owns everything it holds, and changing it changes
> nothing in the value it came from.

## Choosing a fix for a move

A move error has three fixes, and each costs something different. This
program does not compile:

<!-- file: examples/10-ownership/minutes_own.sisu -->
```sisu
# A function that takes a list it only reads.
struct Song:
    title: String
    seconds: int

def minutes(own songs: list[Song]) -> int:
    return sum([song.seconds for song in songs]) // 60

def main():
    let songs = [Song("Northern Lights", 245), Song("Lake Song", 187)]
    print("{minutes(songs)} minutes:")
    for song in songs:
        print("  {song.title}")
```

<!-- output: examples/10-ownership/minutes_own.console -->
```console
$ sisu run minutes_own.sisu
error[E0400]: `songs` was moved on line 11 and cannot be used afterwards
  --> minutes_own.sisu:12:17
   |
12 |     for song in songs:
   |                 ^^^^^
   |
11 |     print("{minutes(songs)} minutes:")
   |             -------------- `songs` is moved here
  = note: copy it with `.copy()` before the move, which makes a second value (for a list or a `String`, an allocation); move it later, after its last use, which costs nothing; or, where what it moves into only reads it, take it there without `own`, and the call lends it

error[D0021]: the program was not run: the ownership checker found errors (1); `--defer-borrow` runs it under the monitor
```

The note offers three fixes, each with what it costs. In the order to
try them:

1. **Move it later.** Use the value before the line that moves it. This
   costs nothing, and works when nothing needs the value after the
   move, as `describe` read the song before `add` took it.
2. **Borrow instead.** If the function that takes the value only reads
   it, its parameter should not say `own`. `minutes` only adds up
   lengths, so delete `own`, and the call borrows the list. This costs
   nothing when the program runs, but it changes the function, which
   only you can judge, so the note offers it only where what the value
   moves into only reads it.
3. **Copy it.** Write `.copy()` where the value moves, so that the
   function gets a copy and you keep the original. This always works,
   and costs the time and the memory of the copy. It is right when two
   owners each need a value of their own, as `usual` and `today` did.

For a small type made of numbers, such as `Point`, there is one more
fix: derive `Copy`, and its values never move.

Here the second fix is right, and the program runs:

<!-- file: examples/10-ownership/minutes.sisu -->
```sisu
# A function that only reads a list borrows it.
struct Song:
    title: String
    seconds: int

def minutes(songs: list[Song]) -> int:
    return sum([song.seconds for song in songs]) // 60

def main():
    let songs = [Song("Northern Lights", 245), Song("Lake Song", 187)]
    print("{minutes(songs)} minutes:")
    for song in songs:
        print("  {song.title}")
```

<!-- output: examples/10-ownership/minutes.out -->
```text
7 minutes:
  Northern Lights
  Lake Song
```

A function asks for `own` only when it keeps what it is given: in a
struct, a collection or its result.

## Moves in loops and branches

A move inside a loop happens on every round, but a value can move only
once:

<!-- file: examples/10-ownership/cards.sisu -->
```sisu
# A value moved in a loop is gone when the loop comes round again.
struct Card:
    to: String
    text: String

def main():
    let text: String = "Happy new year"
    var cards: list[Card] = []
    for name in ["Aino", "Eero"]:
        cards.push(Card(name.to_string(), text))
    print("{len(cards)} cards")
```

<!-- output: examples/10-ownership/cards.console -->
```console
$ sisu run cards.sisu
error[E0400]: `text` was moved on line 10 and cannot be used afterwards
  --> cards.sisu:10:20
   |
10 |         cards.push(Card(name.to_string(), text))
   |                    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   |                    ---------------------------- `text` is moved here, in an earlier iteration of the loop
  = note: copy it with `.copy()` before the move, which makes a second value (for a list or a `String`, an allocation); move it later, after its last use, which costs nothing; or, where what it moves into only reads it, take it there without `own`, and the call lends it

error[D0021]: the program was not run: the ownership checker found errors (1); `--defer-borrow` runs it under the monitor
```

The first round moves `text` into Aino's card, and the second has no
text left for Eero's. The diagnostic marks one line twice: as the use,
and as the move "in an earlier iteration of the loop". Each card needs
a text of its own, so the fix is a copy on each round,
`Card(name.to_string(), text.copy())`.

A move in one branch of an `if` counts after the `if`, whichever
branch ran. The check does not run the program, so it cannot know which
branch will; after the `if`, the value may have been moved, and a value
that may have been moved cannot be used.

## Taking a value out of a list or a struct

Indexing a list reads an element where it is, inside the list. It
cannot move the element out:

<!-- file: examples/10-ownership/first_in_line.sisu -->
```sisu
# An element cannot move out of a list through its index.
def main():
    let queue: list[String] = ["Aino", "Eero"]
    let first = queue[0]
    print("{first} is first of {len(queue)}")
```

<!-- output: examples/10-ownership/first_in_line.console -->
```console
$ sisu run first_in_line.sisu
error[E0338]: cannot move `queue[0]` out of a borrowed value; copy it with `.copy()`
  --> first_in_line.sisu:4:17
  |
4 |     let first = queue[0]
  |                 ^^^^^^^^
  = note: `queue` owns its list, which keeps the item `queue[0]` reads and only lends it: copy the item, or take it out of the list
  = fix: copy it: `.copy()`

error[D0006]: the program was not run: errors found (1)
```

`queue[0]` borrows the element from the list, which is why E0338
(`type.move-out-of-borrow`) speaks of a borrowed value: moving it out
would leave a hole in the list. Its note says the same in the words of
this program: `queue` keeps the item and only lends it. The fix copies
the element. To take an element out instead, ask the list: `remove(i)`
and `pop()` move an element out and make the list shorter.

<!-- file: examples/10-ownership/queue.sisu -->
```sisu
# `remove` and `pop` move an element out of a list.
def main():
    var queue: list[String] = ["Aino", "Eero", "Roni"]
    let first = queue.remove(0)
    let last = queue.pop() ?? "nobody"
    print("{first} goes first and {last} last")
    print("still waiting: {queue}")
```

<!-- output: examples/10-ownership/queue.out -->
```text
Aino goes first and Roni last
still waiting: ["Eero"]
```

`pop` gives an optional value, which is `None` for an empty list, so
`??` gives a name for that case, as in chapter 8.

A field can move out of a struct, and the struct is then incomplete:

<!-- file: examples/10-ownership/half_order.sisu -->
```sisu
# Moving a field out leaves the struct with a hole in it.
struct Order:
    customer: String
    items: list[String]

def main():
    let order = Order("Aino", ["coffee", "bun"])
    let items = order.items
    print("{len(items)} items for {order.customer}")
    print("{order:?}")
```

<!-- output: examples/10-ownership/half_order.console -->
```console
$ sisu run half_order.sisu
error[E0401]: `order` cannot be used as a whole, because `order.items` was moved out of it on line 8
  --> half_order.sisu:10:13
   |
10 |     print("{order:?}")
   |             ^^^^^
   |
 8 |     let items = order.items
   |                 ----------- `order.items` is moved here
  = note: copy it with `.copy()` before the move, which makes a second value (for a list or a `String`, an allocation); move it later, after its last use, which costs nothing; or, where what it moves into only reads it, take it there without `own`, and the call lends it

error[D0021]: the program was not run: the ownership checker found errors (1); `--defer-borrow` runs it under the monitor
```

E0401 (`borrow.use-after-partial-move`) allows `order.customer`, which
is still there, and refuses `order` as a whole, which has lost its
items. The same fixes apply: print the order before taking the items
out, or take a copy of them.

## When a value is dropped

When the scope of an owner ends, the value it still owns is dropped and
its memory released. A type can run code at that moment by
implementing `Drop`, which lets you watch the drops:

<!-- file: examples/10-ownership/sauna.sisu -->
```sisu
# A value is dropped when the scope of its owner ends.
struct Guest:
    name: String

impl Drop for Guest:
    def drop(mut self):
        print("{self.name} goes home")

def main():
    let aino = Guest("Aino")
    let eero = Guest("Eero")
    print("the sauna is warm")
    for round in range(1, 3):
        let roni = Guest("Roni")
        print("{roni.name} joins for round {round}")
    print("the evening ends")
```

**Predict.** Before you run it, decide in which order the eight lines
come out. Then compare:

<!-- output: examples/10-ownership/sauna.out -->
```text
the sauna is warm
Roni joins for round 1
Roni goes home
Roni joins for round 2
Roni goes home
the evening ends
Eero goes home
Aino goes home
```

- `impl Drop for Guest:` gives `Guest` the one method of the trait
  `Drop`, `drop(mut self)`, which runs when a guest is dropped. Chapter
  14 shows `impl` for any trait.
- The body of a `for` loop is a new scope on each round, so each
  round's `roni` is dropped at the end of its round.
- `aino` and `eero` live until `main` ends, and are dropped in the
  reverse of the order in which they were made, `eero` first (D-008).
  A value made later may use the ones made before it, so it goes first.

A value is dropped at the end of its owner's scope, not at its last
use, so the code shows where. Most types need no `Drop`, since their
memory is released without one. Write one for a value that holds
something outside the program's memory, such as a lock or a
connection, which it must let go when it ends.

> **From Python.** Python runs `__del__` when the last reference to an
> object goes, which is hard to see in the code, and the language does
> not promise when that is. A `Drop` runs at a place you can point to.

## The order of drops

A value is dropped by the owner it has when that owner's scope ends,
wherever the value was made:

<!-- file: examples/10-ownership/drop_order.sisu -->
```sisu
# A moved value is dropped by its new owner, a replaced one at once.
struct Guest:
    name: String

impl Drop for Guest:
    def drop(mut self):
        print("{self.name} goes home")

struct Couple:
    first: Guest
    second: Guest

def stay(own guest: Guest):
    print("{guest.name} stays for tea")

def main():
    let aino = Guest("Aino")
    stay(aino)
    var host = Guest("Eero")
    host = Guest("Roni")
    let couple = Couple(Guest("Liisa"), Guest("Pekka"))
    let family = [Guest("Ella"), Guest("Mikko")]
    print("the evening ends")
```

**Predict.** Decide which guest goes home first, and which goes last.
Then compare:

<!-- output: examples/10-ownership/drop_order.out -->
```text
Aino stays for tea
Aino goes home
Eero goes home
the evening ends
Ella goes home
Mikko goes home
Pekka goes home
Liisa goes home
Roni goes home
```

- `stay(aino)` moves Aino into `stay`, whose parameter owns her from
  then on. She goes home when `stay` returns, not at the end of `main`.
- `host = Guest("Roni")` replaces Eero, so the assignment drops Eero at
  once.
- At the end of `main`, the owners that are left drop their values in
  reverse order: `family`, then `couple`, then `host`. `aino` was moved,
  so nothing is dropped for it there: a value ends exactly once.
- A list drops its elements in order, first to last: Ella, then Mikko.
- A struct runs its own `Drop`, if it has one, and then drops its
  fields in the reverse of their order in the declaration: Pekka, then
  Liisa.

## Keeping a value for one block

A `let` keeps its value until the end of the function. `with` keeps it
for one indented block, and drops it where the block ends:

<!-- file: examples/10-ownership/bookings.sisu -->
```sisu
# `with` keeps a value for one block and drops it where the block ends.
struct Booking:
    room: String

    def start(room: str) -> Booking:
        print("{room} is booked")
        return Booking(room.to_string())

impl Drop for Booking:
    def drop(mut self):
        print("{self.room} is free again")

def main():
    with Booking.start("the sauna") as sauna:
        print("an hour in {sauna.room}")
    print("dinner")
    with Booking.start("the boat") as boat:
        with Booking.start("the fishing rod") as rod:
            print("out with {boat.room} and {rod.room}")
    print("goodnight")
```

<!-- output: examples/10-ownership/bookings.out -->
```text
the sauna is booked
an hour in the sauna
the sauna is free again
dinner
the boat is booked
the fishing rod is booked
out with the boat and the fishing rod
the fishing rod is free again
the boat is free again
goodnight
```

`with Booking.start("the sauna") as sauna:` binds the booking to
`sauna` for the block under it, and drops it at the end of that block,
before `print("dinner")`. Nested `with` blocks end from the inside out.
Use `with` for a value that holds something others wait for, such as a
lock or a booked room, so that it is released as soon as the block that
needs it is done.

> **From Python.** A Python `with` calls the object's `__enter__` and
> `__exit__` methods. A Sisu `with` needs no such methods: every value
> can be dropped, so ending the block is enough, and the type's `Drop`
> does what `__exit__` would.

## Running code at the end of a block

`defer` runs a statement when its block ends, however the block ends:

<!-- file: examples/10-ownership/rooms.sisu -->
```sisu
# `defer` runs a statement at the end of its block, however it ends.
def room_of(guest: str, rooms: dict[String, int]) -> String:
    print("looking up {guest}")
    defer print("done with {guest}")
    let room = rooms.get(guest) else:
        return "no booking for {guest}"
    return "{guest} sleeps in room {room}"

def main():
    let rooms: dict[String, int] = {"Aino": 12, "Eero": 7}
    print(room_of("Aino", rooms))
    print(room_of("Roni", rooms))
```

<!-- output: examples/10-ownership/rooms.out -->
```text
looking up Aino
done with Aino
Aino sleeps in room 12
looking up Roni
done with Roni
no booking for Roni
```

`defer print("done with {guest}")` runs when `room_of` returns, both
when it finds a room and when `return` leaves early from the `else`
block. It runs as well when `?` passes an error on, and when a panic
unwinds the function. `defer:` followed by an indented block defers
several statements at once.

The dict owns its keys, each a `String`, and `rooms.get(guest)` looks
one up by the `str` it is given: a lookup only reads its key, so you
write no `.to_string()`. The `str` is compared with the keys as it is,
so the lookup makes no `String` either, and a native build runs it in
place, faster than a lookup that copies the key. The same holds for an
index that is only read, such as `notes[guest].byte_len()` of a
`dict[String, String]`.

A block's drops and `defer`s run together, in the reverse of the order
in which they appear in the code: a `defer` runs where a value declared
on its line would be dropped.

<!-- file: examples/10-ownership/endings.sisu -->
```sisu
# Drops and `defer`s run in one order: the reverse of the code's.
struct Guest:
    name: String

impl Drop for Guest:
    def drop(mut self):
        print("{self.name} goes home")

def main():
    defer print("the lights go out")
    let aino = Guest("Aino")
    defer print("the door is locked")
    let eero = Guest("Eero")
    print("the evening ends")
```

**Predict.** Decide where the two deferred lines come among the guests
who go home. Then compare:

<!-- output: examples/10-ownership/endings.out -->
```text
the evening ends
Eero goes home
the door is locked
Aino goes home
the lights go out
```

> **From Python.** `defer` does the work of `try:` with `finally:`,
> with the clean-up written next to what it cleans up.

## Exercise 10.1: predict the endings

Open
[`manual/exercises/10-ownership/predict_endings.sisu`](../exercises/10-ownership/predict_endings.sisu):

<!-- file: exercises/10-ownership/predict_endings.sisu -->
```sisu
##! Exercise 10.1, predict the endings. Kind: predict. Level: 1.
##!
##! Each constant is your prediction of the log that a method of
##! `Evening` writes, as a list of its lines in order. Write each as a
##! literal, then run `sisu test predict_endings.sisu`. The hints are in
##! chapter 10 of the tutorial, under "Exercise 10.1: predict the
##! endings".

## The log of an evening at a summer cottage.
struct Evening:
    log: list[String] = []

    ## Adds `line` to the log.
    def note(mut self, line: str):
        self.log.push(line.to_string())

    def sauna(mut self):
        self.note("stove lit")
        defer self.note("stove out")
        self.note("Aino bathes")

    def rounds(mut self):
        for name in ["Aino", "Eero"]:
            defer self.note("{name} swims")
            self.note("{name} bathes")

    def walk(mut self, raining: bool):
        self.note("boots on")
        defer self.note("boots off")
        if raining:
            self.note("tea on the porch")
            return
        self.note("a walk to the lake")

    def night(mut self):
        defer self.note("goodnight")
        self.sauna()
        self.note("tea")

const SAUNA: list[String] = todo("the log of sauna()")
const ROUNDS: list[String] = todo("the log of rounds()")
const RAINING: list[String] = todo("the log of walk(True)")
const NIGHT: list[String] = todo("the log of night()")

test "a sauna":
    var evening = Evening()
    evening.sauna()
    assert evening.log == SAUNA

test "two rounds":
    var evening = Evening()
    evening.rounds()
    assert evening.log == ROUNDS

test "a walk in the rain":
    var evening = Evening()
    evening.walk(True)
    assert evening.log == RAINING

test "a whole night":
    var evening = Evening()
    evening.night()
    assert evening.log == NIGHT
```

<!-- output: solutions/10-ownership/predict_endings.console -->
```console
$ sisu test predict_endings.sisu
warning[W0001]: typed hole `the log of sauna()` of type `list[String]`
  --> predict_endings.sisu:40:29
   |
40 | const SAUNA: list[String] = todo("the log of sauna()")
   |                             ^^^^^^^^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `the log of rounds()` of type `list[String]`
  --> predict_endings.sisu:41:30
   |
41 | const ROUNDS: list[String] = todo("the log of rounds()")
   |                              ^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `the log of walk(True)` of type `list[String]`
  --> predict_endings.sisu:42:31
   |
42 | const RAINING: list[String] = todo("the log of walk(True)")
   |                               ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `the log of night()` of type `list[String]`
  --> predict_endings.sisu:43:29
   |
43 | const NIGHT: list[String] = todo("the log of night()")
   |                             ^^^^^^^^^^^^^^^^^^^^^^^^^^

test "a sauna" ... blocked by todo("the log of sauna()")
test "two rounds" ... blocked by todo("the log of rounds()")
test "a walk in the rain" ... blocked by todo("the log of walk(True)")
test "a whole night" ... blocked by todo("the log of night()")
0 passed, 0 failed, 4 blocked
```

`Evening()` builds an evening with an empty log, since `log` has a
default.

<details>
<summary>Hint 1</summary>

A `defer` runs when its block ends: for a method, when the method
returns; for the body of a loop, at the end of each round.

</details>

<details>
<summary>Hint 2</summary>

Of several `defer`s in one block, the one written last runs first. A
`return` ends the method's block early, and its `defer`s still run. A
method called from another finishes, `defer`s and all, before the
caller's next line.

</details>

When every test passes, compare your reasoning with
[`manual/solutions/10-ownership/predict_endings.sisu`](../solutions/10-ownership/predict_endings.sisu).

## Exercise 10.2: a walk on a grid

Open
[`manual/exercises/10-ownership/walk.sisu`](../exercises/10-ownership/walk.sisu):

<!-- file: exercises/10-ownership/walk.sisu -->
```sisu
##! Exercise 10.2, a walk on a grid. Kind: fix. Level: 1.
##!
##! `walk` follows a list of steps and keeps every position it passes.
##! The file does not compile. Run `sisu test walk.sisu`, read the
##! diagnostic, and change the type `Position`, not `walk` or the tests,
##! so that every test passes. The hints are in chapter 10 of the
##! tutorial, under "Exercise 10.2: a walk on a grid".

## A position on a grid: `x` grows to the east and `y` to the north.
@derive(Eq)
struct Position:
    x: int
    y: int

## The positions of a walk that starts at `x`, `y` and takes one step
## for each letter of `steps`: "n", "s", "e" or "w".
def walk(x: int, y: int, steps: str) -> list[Position]:
    var here = Position(x, y)
    var visited = [here]
    for step in steps.chars():
        if step == 'n':
            here.y += 1
        elif step == 's':
            here.y -= 1
        elif step == 'e':
            here.x += 1
        elif step == 'w':
            here.x -= 1
        visited.push(here)
    return visited

test "a walk around a block":
    let path = walk(0, 0, "nesw")
    assert len(path) == 5
    assert path[2] == Position(1, 1)
    assert path[^1] == Position(0, 0)

test "a walk from elsewhere":
    let path = walk(5, -2, "ww")
    assert path == [Position(5, -2), Position(4, -2), Position(3, -2)]
```

<!-- output: solutions/10-ownership/walk.console -->
```console
$ sisu test walk.sisu
error[E0400]: `here` was moved on line 29 and cannot be used afterwards
  --> walk.sisu:29:9
   |
29 |         visited.push(here)
   |         ^^^^^^^^^^^^^^^^^^
   |         ------------------ `here` is moved here, in an earlier iteration of the loop
  = note: copy it with `.copy()` before the move, which makes a second value (for a list or a `String`, an allocation); move it later, after its last use, which costs nothing; or, where what it moves into only reads it, take it there without `own`, and the call lends it

error[D0021]: the program was not run: the ownership checker found errors (1); `--defer-borrow` runs it under the monitor
```

<details>
<summary>Hint 1</summary>

The diagnostic says that `here` moved into the list on an earlier round
of the loop, and the loop needs `here` again on every round.

</details>

<details>
<summary>Hint 2</summary>

A position is two numbers, and copying it costs no more than copying
two numbers. The section "Values that are copied" says how a struct
becomes a type that is copied rather than moved.

</details>

When every test passes, compare your answer with
[`manual/solutions/10-ownership/walk.sisu`](../solutions/10-ownership/walk.sisu).

## Exercise 10.3: three moves

Open
[`manual/exercises/10-ownership/shop.sisu`](../exercises/10-ownership/shop.sisu):

<!-- file: exercises/10-ownership/shop.sisu -->
```sisu
##! Exercise 10.3, three moves. Kind: fix. Level: 2.
##!
##! A small shop keeps its stock as a list of items. The file has three
##! move errors, and each needs a different fix: move later, borrow
##! instead, or copy. Run `sisu test shop.sisu`, read the diagnostics,
##! and fix each error with the fix that costs least and keeps what the
##! doc comments promise. Change the functions, not `Item` or the tests.
##! The hints are in chapter 10 of the tutorial, under "Exercise 10.3:
##! three moves".

@derive(Eq)
struct Item:
    name: String
    price: int

## The total price of `items`, in cents.
def total(own items: list[Item]) -> int:
    return sum([item.price for item in items])

## The stock before a sale, and the stock with each price lowered by
## `percent`, rounded down to a cent.
def sale(own stock: list[Item], percent: int) -> (list[Item], list[Item]):
    let before = stock
    var after = stock
    for i in range(len(after)):
        after[i].price -= after[i].price * percent // 100
    return (before, after)

struct Shop:
    stock: list[Item]

    ## Adds `item` to the stock and says so, such as "added tea at 300".
    def restock(mut self, own item: Item) -> String:
        self.stock.push(item)
        return "added {item.name} at {item.price}"

test "a total":
    let items = [Item("tea", 300), Item("bun", 280)]
    assert total(items) == 580
    assert len(items) == 2
    assert total([]) == 0

test "a sale":
    let stock = [Item("tea", 300), Item("bun", 280)]
    let before, after = sale(stock, 10)
    assert before == [Item("tea", 300), Item("bun", 280)]
    assert after == [Item("tea", 270), Item("bun", 252)]

test "restocking":
    var shop = Shop([])
    assert shop.restock(Item("coffee", 350)) == "added coffee at 350"
    assert shop.restock(Item("tea", 300)) == "added tea at 300"
    assert len(shop.stock) == 2
```

<!-- output: solutions/10-ownership/shop.console -->
```console
$ sisu test shop.sisu
error[E0400]: `stock` was moved on line 23 and cannot be used afterwards
  --> shop.sisu:24:17
   |
24 |     var after = stock
   |                 ^^^^^
   |
23 |     let before = stock
   |                  ----- `stock` is moved here
  = note: copy it with `.copy()` before the move, which makes a second value (for a list or a `String`, an allocation); move it later, after its last use, which costs nothing; or, where what it moves into only reads it, take it there without `own`, and the call lends it

error[E0400]: `item` was moved on line 34 and cannot be used afterwards
  --> shop.sisu:35:24
   |
35 |         return "added {item.name} at {item.price}"
   |                        ^^^^^^^^^
   |
34 |         self.stock.push(item)
   |         --------------------- `item` is moved here
  = note: copy it with `.copy()` before the move, which makes a second value (for a list or a `String`, an allocation); move it later, after its last use, which costs nothing; or, where what it moves into only reads it, take it there without `own`, and the call lends it

error[E0400]: `items` was moved on line 39 and cannot be used afterwards
  --> shop.sisu:40:16
   |
40 |     assert len(items) == 2
   |                ^^^^^
   |
39 |     assert total(items) == 580
   |            ------------ `items` is moved here
  = note: copy it with `.copy()` before the move, which makes a second value (for a list or a `String`, an allocation); move it later, after its last use, which costs nothing; or, where what it moves into only reads it, take it there without `own`, and the call lends it

error[D0021]: the program was not run: the ownership checker found errors (3); `--defer-borrow` runs it under the monitor
```

<details>
<summary>Hint 1</summary>

Read each diagnostic's two places, where the value moved and where it
was used afterwards. Then read what the doc comment of the function
promises.

</details>

<details>
<summary>Hint 2</summary>

`total` only reads its items, and the test goes on using them. `sale`
returns the stock twice, once as it was and once changed. `restock`
keeps the item, and needs its name and price for the line it returns.

</details>

<details>
<summary>Hint 3</summary>

The section "Choosing a fix for a move" gives the three fixes in the
order of their cost, and each error here needs a different one.

</details>

When every test passes, compare your answer with
[`manual/solutions/10-ownership/shop.sisu`](../solutions/10-ownership/shop.sisu).

## Exercise 10.4: a cloakroom

Open
[`manual/exercises/10-ownership/cloakroom.sisu`](../exercises/10-ownership/cloakroom.sisu):

<!-- file: exercises/10-ownership/cloakroom.sisu -->
```sisu
##! Exercise 10.4, a cloakroom. Kind: write. Level: 3.
##!
##! A cloakroom keeps coats and gives a ticket for each, and the ticket
##! gets the coat back. Write the bodies of `leave`, `collect` and `close`
##! so that every test passes, without copying a coat. Run the tests with
##! `sisu test cloakroom.sisu`. The hints are in chapter 10 of the
##! tutorial, under "Exercise 10.4: a cloakroom".

@derive(Eq)
struct Coat:
    owner: String
    colour: String

error CloakroomError:
    UnknownTicket(ticket: int)

## A cloakroom keeps coats and gives a ticket for each. A coat moves in
## and moves out again: it is never copied.
struct Cloakroom:
    coats: dict[int, Coat] = {}
    next_ticket: int = 1

    ## Keeps `coat` and gives its ticket: 1 for the first coat, then 2,
    ## and so on. No number is given twice.
    def leave(mut self, own coat: Coat) -> int:
        return todo("leave")

    ## Gives back the coat of `ticket`, which cannot be used again.
    def collect(mut self, ticket: int) -> Coat ! CloakroomError:
        return todo("collect")

    ## Gives back every coat still here, in the order they were left,
    ## and leaves the cloakroom empty.
    def close(mut self) -> list[Coat]:
        return todo("close")

test "tickets count up":
    var room = Cloakroom()
    assert room.leave(Coat("Aino", "red")) == 1
    assert room.leave(Coat("Eero", "grey")) == 2

test "a ticket gives its coat back":
    var room = Cloakroom()
    let aino = room.leave(Coat("Aino", "red"))
    let eero = room.leave(Coat("Eero", "grey"))
    assert room.collect(eero) == Ok(Coat("Eero", "grey"))
    assert room.collect(aino) == Ok(Coat("Aino", "red"))
    assert len(room.coats) == 0

test "a ticket works once":
    var room = Cloakroom()
    let ticket = room.leave(Coat("Roni", "blue"))
    _ = room.collect(ticket)
    assert room.collect(ticket) == Err(UnknownTicket(ticket))
    assert room.collect(7) == Err(UnknownTicket(7))
    assert room.leave(Coat("Aino", "red")) == 2

test "closing time":
    var room = Cloakroom()
    let aino = room.leave(Coat("Aino", "red"))
    room.leave(Coat("Eero", "grey"))
    room.leave(Coat("Roni", "blue"))
    _ = room.collect(aino)
    assert room.close() == [Coat("Eero", "grey"), Coat("Roni", "blue")]
    assert len(room.coats) == 0
    assert room.close() == []
```

<!-- output: solutions/10-ownership/cloakroom.console -->
```console
$ sisu test cloakroom.sisu
warning[W0001]: typed hole `leave` of type `int`
  --> cloakroom.sisu:26:16
   |
26 |         return todo("leave")
   |                ^^^^^^^^^^^^^

warning[W0001]: typed hole `collect` of type `Coat ! CloakroomError`
  --> cloakroom.sisu:30:16
   |
30 |         return todo("collect")
   |                ^^^^^^^^^^^^^^^

warning[W0001]: typed hole `close` of type `list[Coat]`
  --> cloakroom.sisu:35:16
   |
35 |         return todo("close")
   |                ^^^^^^^^^^^^^

test "tickets count up" ... blocked by todo("leave")
test "a ticket gives its coat back" ... blocked by todo("leave")
test "a ticket works once" ... blocked by todo("leave")
test "closing time" ... blocked by todo("leave")

test "tickets count up" ... blocked by todo("leave")
what the test printed:
  in scope at the typed hole `todo("leave")`:
    self = Cloakroom(coats={}, next_ticket=1)
    coat = Coat(owner="Aino", colour="red")

test "a ticket gives its coat back" ... blocked by todo("leave")
what the test printed:
  in scope at the typed hole `todo("leave")`:
    self = Cloakroom(coats={}, next_ticket=1)
    coat = Coat(owner="Aino", colour="red")

test "a ticket works once" ... blocked by todo("leave")
what the test printed:
  in scope at the typed hole `todo("leave")`:
    self = Cloakroom(coats={}, next_ticket=1)
    coat = Coat(owner="Roni", colour="blue")

test "closing time" ... blocked by todo("leave")
what the test printed:
  in scope at the typed hole `todo("leave")`:
    self = Cloakroom(coats={}, next_ticket=1)
    coat = Coat(owner="Aino", colour="red")

0 passed, 0 failed, 4 blocked
```

`Cloakroom()` builds an empty cloakroom, since both fields have
defaults.

<details>
<summary>Hint 1</summary>

A coat moves into the dict when you store it under its ticket, as you
set any key of a dict. `remove` on a dict moves a value out again, as
an optional value, which is `None` when the key is not there.

</details>

<details>
<summary>Hint 2</summary>

`leave` gives the ticket that `next_ticket` holds, and moves the
counter on, so that no number is given twice, even after a coat is
collected.

</details>

<details>
<summary>Hint 3</summary>

`close` cannot take coats out of the dict while a `for` loop reads the
dict. Go through the tickets that were given, in order, and take out
each coat that is still there.

</details>

When every test passes, compare your answer with
[`manual/solutions/10-ownership/cloakroom.sisu`](../solutions/10-ownership/cloakroom.sisu).

## What you learned

- Every value has one owner: a binding, a field or a collection.
  Giving it to another binding, an `own` parameter, a struct or a list
  moves it, and the old owner cannot use it afterwards.
- A value of a `Copy` type, such as a number, a `str` or a struct that
  derives `Copy`, is copied instead. `.copy()` copies any value, at a
  cost that grows with its size.
- E0400 shows the use and the move. Fix it by moving later, by
  borrowing instead of taking with `own`, or by copying, in that order
  of cost.
- Indexing cannot move an element out of a list (E0338), and `remove`
  and `pop` can. A struct whose field moved out cannot be used whole
  (E0401).
- A value is dropped when the scope of its owner ends, in the reverse
  of the order in which the values were made, and `Drop` runs code
  then. `with` drops a value at the end of a block, and `defer` runs a
  statement there.

Chapter 11 is about lending: the three parameter conventions,
references, and the rule that keeps a borrowed value from changing
under the code that borrowed it.
