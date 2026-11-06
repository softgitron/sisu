# 13. Sharing and graphs

In this chapter you meet the values that ownership alone does not
cover: a value on the heap, a value with several owners, a value that
changes behind a shared owner, handles that reach a value without
owning it, and graphs whose nodes point at each other. Each is a type
in the standard library with a cost you can see, and a table at the end
helps you choose among them. Last come values that must not be dropped
by accident. Challenge II closes Part II. You need about two hours.

## A value on the heap

A struct or an enum can hold a value of its own type, as chapter 7's
`Route` did. The compiler keeps such a value in memory of its own, on
the heap, and stores a pointer to it, which is called boxing (D-043):

<!-- file: examples/13-sharing/steps.sisu -->
```sisu
# The compiler boxes a recursive type; `Box` puts a value on the heap.
struct Step:
    text: String
    next: Step?

def count(step: Step) -> int:
    return match step.next:
        case Some(rest): 1 + count(rest)
        case None: 1

def main():
    let wait = Step("wait", None)
    let tea = Step("boil water", Some(Step("add the leaves", Some(wait))))
    print("{count(tea)} steps, starting with {tea.text}")
    let boxed = Box(Step("pour", None))
    let moved = boxed
    print("then {moved.text}: {count(moved.boxed)} step")
```

<!-- output: examples/13-sharing/steps.out -->
```text
3 steps, starting with boil water
then pour: 1 step
```

`next: Step?` holds the rest of the recipe, and the compiler boxes it
without a word from you. `Box(value)` does the same by hand: it moves
a value to the heap, and owns it there. Fields and methods are reached
through the box, as `moved.text` does, and `moved.boxed` is the value
itself. Moving a box moves only its pointer. You write `Box` when you
want that, and for a `Box[dyn Trait]`, which chapter 15 shows; a type
that contains itself needs no `Box` of yours.

## Values with several owners

Sometimes a value has no single owner: two services read one
configuration, and neither outlives the other in any order you can
name. `Shared` gives a value several owners:

<!-- file: examples/13-sharing/services.sisu -->
```sisu
# `Shared` gives a value several owners; `.copy()` makes one more.
struct Config:
    region: String
    retries: int

struct Service:
    name: String
    config: Shared[Config]

def main():
    let config = Shared(Config("north", 3))
    let web = Service("web", config.copy())
    let mail = Service("mail", config.copy())
    print("{Shared.count(config)} owners of one config")
    print("{web.name} retries {web.config.retries} times")
    print("{mail.name} runs in the {mail.config.region}")
```

<!-- output: examples/13-sharing/services.out -->
```text
3 owners of one config
web retries 3 times
mail runs in the north
```

- `Shared(Config(...))` moves the config to the heap with a count of
  its owners, here 1.
- `config.copy()` makes another owner, not another config: it adds 1
  to the count, which is all it costs (D-005). `Shared.count(config)`
  reads the count.
- The fields of the config are reached through any owner, as
  `web.config.retries` does.
- Each owner that is dropped takes 1 off the count, and the config is
  dropped with its last owner.

A `Shared` owns its value, so a plain struct can hold one, as `Service`
does, where it could not hold a view. `Shared.same(a, b)` asks whether
two owners hold the very same value, which a copy does and a second
`Shared` of an equal value does not:

<!-- file: examples/13-sharing/same.sisu -->
```sisu
# `Shared.same` asks whether two owners hold the very same value.
@derive(Eq)
struct Config:
    retries: int

def main():
    let config = Shared(Config(retries=3))
    let copy = config.copy()
    let other = Shared(Config(retries=3))
    print("{Shared.same(config, copy)} {Shared.same(config, other)}")
    print(config == other)
```

<!-- output: examples/13-sharing/same.out -->
```text
True False
True
```

`==` is the other question, as the last line asks it: whether the
values are equal, which `config` and `other` are, so `Shared[Config]`
compares when `Config` does. A `Shared` hashes as its value too, so it
can be a dict's key. It also prints as the value it holds, with `{x}`
and with `{x:?}`:

<!-- file: examples/13-sharing/shown.sisu -->
```sisu
# A `Shared` prints as the value it holds.
def main():
    let region = Shared("north".to_string())
    let copy = region.copy()
    print("{region} and {copy}, {Shared.count(region)} owners")
    print("{region:?}")
```

<!-- output: examples/13-sharing/shown.out -->
```text
north and north, 2 owners
"north"
```

**Try it.** Add `web.config.retries = 5` before the first `print`, and
run the program.

<details>
<summary>What Sisu says</summary>

<!-- output: examples/13-sharing/change_shared.console -->
```console
$ sisu run change_shared.sisu
error[E0343]: cannot modify `web.config.retries`: a `Shared` holds it, and a shared value is not changed in place, even through a `var`
  --> change_shared.sisu:14:5
   |
14 |     web.config.retries = 5
   |     ^^^^^^^^^^^^^^^^^^
  = note: what a `Shared` holds is shared, so it is not changed in place: put the part that changes in a `Cell` or a `Mutex` inside it, and change it through that

error[D0006]: the program was not run: errors found (1)
```

Nothing changes a value through a `Shared`, since every other owner
would see the change without asking for it. E0343
(`type.shared-not-mutable`) says that a `Shared` holds the field, so
declaring `web` with `var` would not help. Its note says what does: a
shared value that must change holds a `Cell` or a `Mutex`, which the
next section and chapter 20 show.

</details>

## Changing a shared value

A `Cell` holds a value that can be replaced through a plain borrow, so
the owners of a `Shared[Cell[T]]` can all change it:

<!-- file: examples/13-sharing/visits.sisu -->
```sisu
# A `Cell` holds a value that every owner of a `Shared` can replace.
struct Page:
    path: String
    visits: Shared[Cell[int]]

def visit(page: Page):
    page.visits.set(page.visits.get() + 1)

def main():
    let total = Shared(Cell(0))
    let home = Page("/", total.copy())
    let news = Page("/news", total.copy())
    visit(home)
    visit(news)
    visit(news)
    print("{total.get()} visits in all")
```

<!-- output: examples/13-sharing/visits.out -->
```text
3 visits in all
```

`get` gives a copy of the value, so it works for a `Copy` type such as
`int`, and `set` puts a new value in. For a value that is not `Copy`,
`replace(new)` puts `new` in and gives back the old value, so
`cell.replace([])` takes a list out of a cell and leaves an empty one.
No reference into a `Cell` is ever given out, so replacing its value
leaves no borrow pointing at the old one. A `Cell` belongs to one task;
values that tasks share change through a `Mutex` or an `Atomic`, which
chapter 20 shows.

## Cycles, and handles that do not own

Two `Shared` values that own each other keep each other alive, and
neither is ever dropped:

<!-- file: examples/13-sharing/cycle.sisu -->
```sisu
# Two `Shared` values that own each other are never dropped.
struct Person:
    name: String
    friend: Cell[Shared[Person]?]

impl Drop for Person:
    def drop(mut self):
        print("{self.name} is dropped")

def main():
    let aino = Shared(Person("Aino", Cell(None)))
    let eero = Shared(Person("Eero", Cell(None)))
    aino.friend.set(Some(eero.copy()))
    eero.friend.set(Some(aino.copy()))
    print("{Shared.count(aino)} owners of Aino")
    print("the end of main")
```

<!-- output: examples/13-sharing/cycle.out -->
```text
2 owners of Aino
the end of main
```

Aino owns Eero and Eero owns Aino, so when `main` ends each still has
an owner, and the two are never dropped: their `Drop` never runs, and
their memory is never released. Sisu has no collector that would find
them (D-005). A `Weak` handle breaks the cycle, because it reaches a
value without owning it:

<!-- file: examples/13-sharing/weak.sisu -->
```sisu
# A `Weak` handle does not own, so two people can point at each other.
struct Person:
    name: String
    friend: Cell[Weak[Person]?]

impl Drop for Person:
    def drop(mut self):
        print("{self.name} is dropped")

def main():
    let aino = Shared(Person("Aino", Cell(None)))
    let eero = Shared(Person("Eero", Cell(None)))
    aino.friend.set(Some(Shared.downgrade(eero)))
    eero.friend.set(Some(Shared.downgrade(aino)))
    print("{Shared.count(aino)} owner of Aino")
    print("the end of main")
```

<!-- output: examples/13-sharing/weak.out -->
```text
1 owner of Aino
the end of main
Eero is dropped
Aino is dropped
```

`Shared.downgrade(eero)` makes a `Weak` handle, which the count leaves
out, so each person has one owner, and both are dropped when `main`
ends. To use the value, `upgrade()` gives a new owner, as an optional
value, which is `None` once the value is gone:

<!-- file: examples/13-sharing/friend_of.sisu -->
```sisu
# `upgrade` gives a `Weak` handle's value, while an owner keeps it.
struct Person:
    name: String
    friend: Weak[Person]?

impl Drop for Person:
    def drop(mut self):
        print("{self.name} is dropped")

def friend_of(person: Shared[Person]) -> String:
    if let Some(handle) = person.friend, let Some(friend) = handle.upgrade():
        return "{person.name}'s friend is {friend.name}"
    return "{person.name} has no friend here"

def main():
    let eero = Shared(Person("Eero", None))
    let aino = Shared(Person("Aino", Some(Shared.downgrade(eero))))
    print(friend_of(aino))
    print("{Shared.count(eero)} owner of Eero")
```

<!-- output: examples/13-sharing/friend_of.out -->
```text
Aino's friend is Eero
1 owner of Eero
Aino is dropped
Eero is dropped
```

The rule for a structure of `Shared` values: owners point one way, and
handles that point back are `Weak`, as a child points to its parent.

## Graphs in an arena

A graph whose nodes point at each other in every direction, cycles and
all, is simplest in an `Arena`. The arena owns every node, and a node
names another by a `Key`:

<!-- file: examples/13-sharing/towns.sisu -->
```sisu
# An `Arena` keeps the nodes of a graph, and a `Key` names a node.
struct Town:
    name: String
    roads: list[Key[Town]]

def main():
    var map = Arena[Town]()
    let oulu = map.insert(Town("Oulu", []))
    let kemi = map.insert(Town("Kemi", []))
    let tornio = map.insert(Town("Tornio", []))
    map[oulu].roads.push(kemi)
    map[kemi].roads.push(tornio)
    map[tornio].roads.push(oulu)
    var here = oulu
    for _ in range(4):
        let next = map[here].roads[0]
        print("{map[here].name} to {map[next].name}")
        here = next
    map.remove(kemi)
    print("Kemi is still there: {map.contains(kemi)}")
    print(map.get(kemi)?.name ?? "no town")
```

<!-- output: examples/13-sharing/towns.out -->
```text
Oulu to Kemi
Kemi to Tornio
Tornio to Oulu
Oulu to Kemi
Kemi is still there: False
no town
```

- `map.insert(town)` moves a town into the arena and gives its key. A
  `Key` is `Copy`: two numbers, the slot and its generation.
- `map[oulu]` reaches the town of a key, to read or to change, so the
  roads form a cycle with no question of who owns whom.
- `map.remove(kemi)` drops Kemi. Its key is now stale: `contains` and
  `get` see that, and `map[kemi]` would panic, so a stale key is caught
  where it is used, never followed to memory that something else uses.

## Choosing an owner

The default is a value with one owner, and most programs need little
else. When one owner is not enough, this table gives each choice and
what it costs:

| You need | Use | It costs |
|---|---|---|
| a value with one owner | the value itself | nothing |
| a value on the heap | `Box[T]` | an allocation |
| a value that several owners read | `Shared[T]` | an allocation, and a count change for each `.copy()` and drop |
| a value that several owners change, in one task | `Shared[Cell[T]]` | that of `Shared`, and a copy in or out for each change |
| a handle back that does not own | `Weak[T]` | an `upgrade()` that may find nothing |
| a graph with cycles | `Arena[T]` and `Key[T]` | a check of the key for each use |
| a part of a value, for a while | a borrow or a view (chapters 11 and 12) | nothing, but it stays in its frame |

Reach for them in that order. An arena is often the better answer for
a graph than `Shared` and `Weak`, because its nodes need no counts and
its cycles leak nothing.

## Values that must be used up

A `Drop` cannot fail and cannot return an error, but some clean-up
can: a file whose last write may not reach the disk, a payment that
must be made or refused. A type marked `@must_consume` cannot be
dropped by accident. Each value must be passed to an `own` parameter,
most often to one of its own methods that takes `own self`, such as a
`close(own self) -> ! E` that reports what went wrong (D-054):

<!-- file: examples/13-sharing/drafts.sisu -->
```sisu
# A `@must_consume` value must end in a method that takes `own self`.
error SaveError:
    Empty

@must_consume
struct Draft:
    text: String

    ## Saves the draft into `saved`, which ends it; this can fail.
    def save(own self, mut saved: list[String]) -> ! SaveError:
        if self.text.is_empty():
            return Err(Empty)
        saved.push(self.text)
        return Ok(())

    ## Throws the draft away, which ends it too.
    def discard(own self):
        pass

def main():
    var saved: list[String] = []
    let letter = Draft("Hei Aino")
    let note = Draft("")
    let scrap = Draft("a first try")
    print(letter.save(mut saved))
    print(note.save(mut saved))
    scrap.discard()
    print(saved)
```

<!-- output: examples/13-sharing/drafts.out -->
```text
Ok(())
Err(Empty)
["Hei Aino"]
```

`save` and `discard` take `own self`, so each call ends a draft, and
`save` can fail with an error the caller sees. Inside its own methods a
draft may end as it likes: `save` moves its text out of it.

**Try it.** Replace `scrap.discard()` with `print(scrap.text)`.

<details>
<summary>What Sisu says</summary>

<!-- output: examples/13-sharing/forgotten_draft.console -->
```console
$ sisu run forgotten_draft.sisu
error[E0417]: `scrap` is never consumed: a value of `Draft` must be passed to an `own` parameter before it goes out of scope
  --> forgotten_draft.sisu:28:17
   |
28 |     print(saved)
   |                 ^
   |
24 |     let scrap = Draft("a first try")
   |         ----- `scrap` is declared here
  = note: consume it with one of its methods that take `own self`: `discard`, `save`

error[D0021]: the program was not run: the ownership checker found errors (1); `--defer-borrow` runs it under the monitor
```

E0417 (`borrow.must-consume`) points at the end of the scope, where
the draft would be dropped, and its note lists the methods that end
one. A panic is the exception: unwinding drops a must-consume value
without a word, since the program is failing anyway.

</details>

A collection of must-consume values is must-consume too (D-064), so a
`list[Draft]` must be consumed, and `for draft in own drafts` does it,
one draft at a time. Leaving such a loop early, with `return` or `?`,
would drop the drafts not yet visited, and gets E0418
(`borrow.must-consume-exit`); exercise 13.2 is about that error.

## Counting what a program shares

`sisu audit --ownership` counts, for each file, the lines of code and
the uses of each kind of owner beyond the plain one:

<!-- output: examples/13-sharing/audit.console -->
```console
$ sisu audit --ownership towns.sisu cycle.sisu weak.sisu
towns.sisu lines=19 Shared=0 Weak=0 Cell=0 Arena=1 Key=1 copy=0 unsafe=0
cycle.sisu lines=13 Shared=4 Weak=0 Cell=3 Arena=0 Key=0 copy=2 unsafe=0
weak.sisu lines=13 Shared=5 Weak=1 Cell=3 Arena=0 Key=0 copy=0 unsafe=0
```

The counts are of names in the code: `Shared=5` means the name
`Shared` appears five times. A program whose counts grow from version
to version is reaching for shared owners more often, and is worth a
second look; most code needs none of them, as `towns.sisu` needs
only its arena.

## Exercise 13.1: predict the owners

Open
[`manual/exercises/13-sharing/predict_owners.sisu`](../exercises/13-sharing/predict_owners.sisu):

<!-- file: exercises/13-sharing/predict_owners.sisu -->
```sisu
##! Exercise 13.1, predict the owners. Kind: predict. Level: 1.
##!
##! Each constant is your prediction of what a function returns. Write
##! each as a literal, then run `sisu test predict_owners.sisu`. The
##! hints are in chapter 13 of the tutorial, under "Exercise 13.1:
##! predict the owners".

struct Song:
    title: String

def owners_after_copies() -> int:
    let song = Shared(Song("Lake Song"))
    let first = song.copy()
    let second = first.copy()
    return Shared.count(song)

def owners_after_a_loop() -> int:
    let song = Shared(Song("Lake Song"))
    for _ in range(3):
        let copy = song.copy()
    return Shared.count(song)

def owners_in_a_list() -> int:
    let song = Shared(Song("Lake Song"))
    let playlist = [song.copy(), song.copy()]
    return Shared.count(playlist[1])

def owners_with_a_weak_handle() -> int:
    let song = Shared(Song("Lake Song"))
    let handle = Shared.downgrade(song)
    return Shared.count(song)

def handle_of_a_dropped_song() -> Weak[Song]:
    let song = Shared(Song("Lake Song"))
    return Shared.downgrade(song)

def plays() -> int:
    let plays = Shared(Cell(0))
    let other = plays.copy()
    plays.set(plays.get() + 1)
    other.set(other.get() + 1)
    return plays.get()

const COPIES: int = todo("owners after two copies")
const LOOP: int = todo("owners after a loop")
const LIST: int = todo("owners with a list of copies")
const WEAK: int = todo("owners with a weak handle")
const STILL_THERE: bool = todo("whether the dropped song is still there")
const PLAYS: int = todo("plays counted through two owners")

test "copies":
    assert owners_after_copies() == COPIES

test "a loop":
    assert owners_after_a_loop() == LOOP

test "a list":
    assert owners_in_a_list() == LIST

test "a weak handle":
    assert owners_with_a_weak_handle() == WEAK

test "a dropped song":
    assert handle_of_a_dropped_song().upgrade().is_some() == STILL_THERE

test "a shared cell":
    assert plays() == PLAYS
```

<!-- output: solutions/13-sharing/predict_owners.console -->
```console
$ sisu test predict_owners.sisu
warning[W0001]: typed hole `owners after two copies` of type `int`
  --> predict_owners.sisu:44:21
   |
44 | const COPIES: int = todo("owners after two copies")
   |                     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `owners after a loop` of type `int`
  --> predict_owners.sisu:45:19
   |
45 | const LOOP: int = todo("owners after a loop")
   |                   ^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `owners with a list of copies` of type `int`
  --> predict_owners.sisu:46:19
   |
46 | const LIST: int = todo("owners with a list of copies")
   |                   ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `owners with a weak handle` of type `int`
  --> predict_owners.sisu:47:19
   |
47 | const WEAK: int = todo("owners with a weak handle")
   |                   ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `whether the dropped song is still there` of type `bool`
  --> predict_owners.sisu:48:27
   |
48 | const STILL_THERE: bool = todo("whether the dropped song is still there")
   |                           ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `plays counted through two owners` of type `int`
  --> predict_owners.sisu:49:20
   |
49 | const PLAYS: int = todo("plays counted through two owners")
   |                    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

test "copies" ... blocked by todo("owners after two copies")
test "a loop" ... blocked by todo("owners after a loop")
test "a list" ... blocked by todo("owners with a list of copies")
test "a weak handle" ... blocked by todo("owners with a weak handle")
test "a dropped song" ... blocked by todo("whether the dropped song is still there")
test "a shared cell" ... blocked by todo("plays counted through two owners")
0 passed, 0 failed, 6 blocked
```

<details>
<summary>Hint 1</summary>

Every `.copy()` of a `Shared` adds an owner, wherever the copy is
kept, and every owner that is dropped takes one away. A `Weak` handle
is not an owner.

</details>

<details>
<summary>Hint 2</summary>

A binding inside a loop is dropped at the end of each round. A value
whose last owner was dropped when its function returned is gone, and
its `Weak` handles find nothing.

</details>

When every test passes, compare your reasoning with
[`manual/solutions/13-sharing/predict_owners.sisu`](../solutions/13-sharing/predict_owners.sisu).

## Exercise 13.2: every draft

Open
[`manual/exercises/13-sharing/drafts.sisu`](../exercises/13-sharing/drafts.sisu):

<!-- file: exercises/13-sharing/drafts.sisu -->
```sisu
##! Exercise 13.2, every draft. Kind: fix. Level: 2.
##!
##! A `Draft` must be saved or discarded, and `save_all` saves a list of
##! them. The file does not compile. Run `sisu test drafts.sisu`, read
##! the diagnostic, and fix `save_all`, not `Draft` or the tests, so that
##! every test passes. The hints are in chapter 13 of the tutorial, under
##! "Exercise 13.2: every draft".

error SaveError:
    Empty(position: int)

## A text that must be saved or discarded before it goes away.
@must_consume
struct Draft:
    text: String

    ## Saves the draft into `saved`, which ends it; an empty draft cannot
    ## be saved, and ends unsaved.
    def save(own self, mut saved: list[String]) -> bool:
        if self.text.is_empty():
            return False
        saved.push(self.text)
        return True

## Saves each of `drafts` into `saved`, in order. A draft that cannot
## be saved does not stop the others; the result is the error for the
## first of them, with its position in the list, counted from 0.
def save_all(own drafts: list[Draft], mut saved: list[String]) -> ! SaveError:
    var position = 0
    for draft in own drafts:
        if not draft.save(mut saved):
            return Err(Empty(position))
        position += 1
    return Ok(())

test "every draft is saved":
    var saved: list[String] = []
    assert save_all([Draft("a"), Draft("b")], mut saved) == Ok(())
    assert saved == ["a", "b"]

test "an empty draft does not stop the others":
    var saved: list[String] = []
    let drafts = [Draft("a"), Draft(""), Draft("c"), Draft("")]
    assert save_all(drafts, mut saved) == Err(Empty(1))
    assert saved == ["a", "c"]
```

<!-- output: solutions/13-sharing/drafts.console -->
```console
$ sisu test drafts.sisu
error[E0418]: this leaves the scope with `drafts` unconsumed: a value of `list[Draft]` must be passed to an `own` parameter first
  --> drafts.sisu:32:13
   |
32 |             return Err(Empty(position))
   |             ^^^^^^^^^^^^^^^^^^^^^^^^^^^
   |
28 | def save_all(own drafts: list[Draft], mut saved: list[String]) -> ! SaveError:
   |              ----------------------- `drafts` is declared here
  = note: the loop that consumes `drafts` has not visited every element yet; finish the loop first, or return what is left

error[D0021]: the program was not run: the ownership checker found errors (1); `--defer-borrow` runs it under the monitor
```

<details>
<summary>Hint 1</summary>

The `return` inside the loop would leave the drafts after it unsaved
and undiscarded, which a `list[Draft]` does not allow. The note gives
two ways out; the tests say which one the doc comment means.

</details>

<details>
<summary>Hint 2</summary>

Let the loop visit every draft, and remember the position of the first
one that could not be saved. Return its error after the loop.

</details>

When every test passes, compare your answer with
[`manual/solutions/13-sharing/drafts.sisu`](../solutions/13-sharing/drafts.sisu).

## Challenge II: a dependency graph

The challenge is a package manager's graph of packages, each needing
others, built in three parts with a suite of tests for each. Open
[`manual/exercises/13-sharing/dependencies.sisu`](../exercises/13-sharing/dependencies.sisu):

<!-- file: exercises/13-sharing/dependencies.sisu -->
```sisu
##! Exercise 13.3, Challenge II: a dependency graph. Kind: challenge. Level: 3.
##!
##! A package manager installs each package after the packages it needs.
##! This challenge builds its graph in three parts, with a suite of tests
##! for each. Solve them in order: each part's tests are blocked until it
##! is done. Run the tests with `sisu test dependencies.sisu`.
##!
##! Part 1. A `Graph` keeps its packages in an `Arena`, and the needs of
##! a package are the `Key`s of other packages. Write `key`, `add` and
##! `install_order`, which gives the names of the packages in an order in
##! which each comes after every package it needs: take the packages in
##! the order they were added, and put the needs of each first, in the
##! order they were given.
##!
##! Part 2. A graph can have a cycle, such as "a needs b, b needs a", and
##! then no order exists. `install_order` then gives `Err(Cycle(names))`,
##! with the packages of the first cycle it meets, starting from the one
##! it meets twice, in the order of their needs. The tests of part 1
##! still pass.
##!
##! Part 3. `to_shared` gives the same packages as `Shared` items, by
##! name. An item owns the items it needs, and reaches the items that
##! need it through `Weak` handles, so that no two items own each other.
##! Write `to_shared` and `Item.dependents`.
##!
##! The hints are in chapter 13 of the tutorial, under "Challenge II: a
##! dependency graph".

error CycleError:
    Cycle(names: list[String])

## A package and the keys of the packages it needs.
struct Package:
    name: String
    needs: list[Key[Package]]

## A package that owns the items of the packages it needs, and reaches
## the items that need it through `Weak` handles.
struct Item:
    name: String
    needs: list[Shared[Item]]
    needed_by: Cell[list[Weak[Item]]]

    ## The names of the items that need this one and still exist, in the
    ## order in which they were made.
    def dependents(self) -> list[String]:
        return todo("dependents")

## Packages in an arena, and the key of each by its name.
struct Graph:
    packages: Arena[Package] = Arena[Package]()
    keys: dict[String, Key[Package]] = {}

    ## The key of the package `name`, added with no needs if it is new.
    def key(mut self, name: str) -> Key[Package]:
        return todo("key")

    ## Records that `name` needs each of `needs`, in order; adds every
    ## package that is new.
    def add(mut self, name: str, needs: list[str]):
        todo("add")

    ## The names of the packages in an order in which each comes after
    ## every package it needs.
    def install_order(self) -> list[String] ! CycleError:
        return todo("install_order")

    ## The packages as `Shared` items, by name.
    def to_shared(self) -> dict[String, Shared[Item]] ! CycleError:
        return todo("to_shared")

## The graph of a small program: "app" needs "web" and "log", and "web"
## needs "log" and "json".
def example() -> Graph:
    var graph = Graph()
    graph.add("app", ["web", "log"])
    graph.add("web", ["log", "json"])
    graph.add("json", [])
    return graph

## The item "log" of the example, once every other item is gone.
def log_alone() -> Shared[Item]:
    let items = example().to_shared().expect("the example has no cycle")
    return items["log"].copy()

suite "part 1, the install order":
    test "a chain":
        var graph = Graph()
        graph.add("c", ["b"])
        graph.add("b", ["a"])
        assert graph.install_order() == Ok(["a", "b", "c"])

    test "needs that are shared":
        assert example().install_order() == Ok(["log", "json", "web", "app"])

suite "part 2, cycles":
    test "a cycle of three":
        var graph = Graph()
        graph.add("a", ["b"])
        graph.add("b", ["c"])
        graph.add("c", ["a"])
        assert graph.install_order() == Err(Cycle(["a", "b", "c"]))

    test "a package that needs itself":
        var graph = Graph()
        graph.add("a", ["a"])
        assert graph.install_order() == Err(Cycle(["a"]))

    test "a cycle further on":
        var graph = Graph()
        graph.add("app", ["web"])
        graph.add("web", ["json"])
        graph.add("json", ["web"])
        assert graph.install_order() == Err(Cycle(["web", "json"]))

suite "part 3, shared and weak":
    test "items own what they need":
        let items = example().to_shared().expect("no cycle")
        let needs = [need.name.copy() for need in items["app"].needs]
        assert needs == ["web", "log"]
        assert items["log"].dependents() == ["web", "app"]

    test "a weak handle keeps nothing":
        let log = log_alone()
        assert log.dependents() == []
        assert Shared.count(log) == 1

    test "a cycle has no items":
        var graph = Graph()
        graph.add("a", ["a"])
        assert graph.to_shared().is_err()
```

Before you change anything, every test is blocked, since `example` and
the tests call `add`, which is still a hole:

<!-- output: solutions/13-sharing/dependencies.console -->
```console
$ sisu test dependencies.sisu
warning[W0001]: typed hole `dependents` of type `list[String]`
  --> dependencies.sisu:47:16
   |
47 |         return todo("dependents")
   |                ^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `key` of type `Key[Package]`
  --> dependencies.sisu:56:16
   |
56 |         return todo("key")
   |                ^^^^^^^^^^^

warning[W0001]: typed hole `add` of type `()`
  --> dependencies.sisu:61:9
   |
61 |         todo("add")
   |         ^^^^^^^^^^^

warning[W0001]: typed hole `install_order` of type `list[String] ! CycleError`
  --> dependencies.sisu:66:16
   |
66 |         return todo("install_order")
   |                ^^^^^^^^^^^^^^^^^^^^^

warning[W0001]: typed hole `to_shared` of type `dict[String, Shared[Item]] ! CycleError`
  --> dependencies.sisu:70:16
   |
70 |         return todo("to_shared")
   |                ^^^^^^^^^^^^^^^^^

test "part 1, the install order/a chain" ... blocked by todo("add")
test "part 1, the install order/needs that are shared" ... blocked by todo("add")
test "part 2, cycles/a cycle of three" ... blocked by todo("add")
test "part 2, cycles/a package that needs itself" ... blocked by todo("add")
test "part 2, cycles/a cycle further on" ... blocked by todo("add")
test "part 3, shared and weak/items own what they need" ... blocked by todo("add")
test "part 3, shared and weak/a weak handle keeps nothing" ... blocked by todo("add")
test "part 3, shared and weak/a cycle has no items" ... blocked by todo("add")

test "part 1, the install order/a chain" ... blocked by todo("add")
what the test printed:
  in scope at the typed hole `todo("add")`:
    self = Graph(packages=Arena([]), keys={})
    name = "c"
    needs = ["b"]

test "part 1, the install order/needs that are shared" ... blocked by todo("add")
what the test printed:
  in scope at the typed hole `todo("add")`:
    self = Graph(packages=Arena([]), keys={})
    name = "app"
    needs = ["web", "log"]

test "part 2, cycles/a cycle of three" ... blocked by todo("add")
what the test printed:
  in scope at the typed hole `todo("add")`:
    self = Graph(packages=Arena([]), keys={})
    name = "a"
    needs = ["b"]

test "part 2, cycles/a package that needs itself" ... blocked by todo("add")
what the test printed:
  in scope at the typed hole `todo("add")`:
    self = Graph(packages=Arena([]), keys={})
    name = "a"
    needs = ["a"]

test "part 2, cycles/a cycle further on" ... blocked by todo("add")
what the test printed:
  in scope at the typed hole `todo("add")`:
    self = Graph(packages=Arena([]), keys={})
    name = "app"
    needs = ["web"]

test "part 3, shared and weak/items own what they need" ... blocked by todo("add")
what the test printed:
  in scope at the typed hole `todo("add")`:
    self = Graph(packages=Arena([]), keys={})
    name = "app"
    needs = ["web", "log"]

test "part 3, shared and weak/a weak handle keeps nothing" ... blocked by todo("add")
what the test printed:
  in scope at the typed hole `todo("add")`:
    self = Graph(packages=Arena([]), keys={})
    name = "app"
    needs = ["web", "log"]

test "part 3, shared and weak/a cycle has no items" ... blocked by todo("add")
what the test printed:
  in scope at the typed hole `todo("add")`:
    self = Graph(packages=Arena([]), keys={})
    name = "a"
    needs = ["a"]

0 passed, 0 failed, 8 blocked
```

Solve the parts in order. When part 1 passes, the tests of part 2 may
fail with a stack overflow: a search that does not watch for cycles
goes round one without end.

<details>
<summary>Hint for part 1</summary>

`key` looks the name up in `keys` and inserts a new package only when
it is not there. `install_order` is a depth-first search: a helper
method visits a package's needs before it adds the package to the
order, and a set of the packages already added keeps each one to one
place.

</details>

<details>
<summary>Hint for part 2</summary>

A cycle is a package that the search meets again while it is still
visiting that package's needs. Keep the packages being visited in a
list, in order, and when the search meets one of them, the cycle is
the end of that list from that package on. A `for` loop with
`enumerate` over that `mut` list gives each package with its position,
and the loop may read the list too, so the cycle is a slice of it from
that position on.

</details>

<details>
<summary>Hint for part 3</summary>

Make the items in the install order of part 1, so that the items a
package needs exist before it does, and keep them in a dict by name. An
item's `needed_by` is in a `Cell`, so adding a handle to it means
taking the list out with `replace`, adding to it and putting it back.

</details>

When every test passes, compare your answer with
[`manual/solutions/13-sharing/dependencies.sisu`](../solutions/13-sharing/dependencies.sisu).

## What you learned

- The compiler boxes recursive types; `Box[T]` puts a value on the heap
  by hand.
- `Shared[T]` gives a value several owners. `.copy()` adds an owner,
  and the value is dropped with the last one. Nothing changes through a
  `Shared` except through a `Cell` inside it.
- Owners that own each other are never dropped; a `Weak` handle reaches
  a value without owning it, and `upgrade()` finds it while an owner
  keeps it.
- An `Arena[T]` owns the nodes of a graph, and a `Key[T]` names one; a
  stale key is caught where it is used.
- A `@must_consume` value must end in an `own` parameter, usually a
  method such as `close(own self)`, and so must a collection of them
  (E0417, E0418).
- `sisu audit --ownership` counts the shared owners of each file.

That is the end of Part II. You know who owns each value, when it is
moved, copied, lent or dropped, and which type to use when one owner is
not enough. Part III, which starts with chapter 14, is about
abstraction: traits, generics, closures and iterators, and modules.
