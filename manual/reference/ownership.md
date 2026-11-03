# Ownership and borrowing

This page states the rules of Sisu's memory model: who owns a value,
when it moves, copies and is dropped, how parameters borrow, what a
reference and a view may do, how closures capture and loops iterate,
which values must be consumed, and the library types for sharing. The
compiler's ownership checker enforces the rules, and the ownership
monitor checks them again while a program runs.

**See also:** the tutorial's [chapter 10](../tutorial/10-ownership.md)
(moves and drops), [chapter 11](../tutorial/11-borrowing.md)
(borrowing), [chapter 12](../tutorial/12-views.md) (views) and
[chapter 13](../tutorial/13-sharing.md) (sharing and graphs); the
proposal's [04, Memory and ownership](../../proposal/04-memory-and-ownership.md);
the compiler's [architecture notes](../../compiler/docs/architecture.md),
"The ownership checker"; [R17, Diagnostics](diagnostics.md#ownership)
for the codes.

Each section ends with a status, in the words that [R16](sisu-command.md#how-to-read-this-page)
explains. The model is decided: ownership with value semantics,
second-class references and explicit shared ownership (D-004, D-005,
D-006, D-007, D-008).

## Every value has one owner

Every variable owns its value. These move a value to a new owner, after
which the old name can no longer be used:

- assigning it to another binding or to a field;
- passing it to an `own` parameter;
- returning it;
- putting it into a collection or a struct being built.

<!-- file: examples/reference-ownership/moves.sisu -->
```sisu
# Assignment, an `own` parameter and a return move a value.
def keep(own names: list[String]) -> list[String]:
    return names

def main():
    let names = ["Aino".to_string(), "Eero".to_string()]
    let team = names
    let kept = keep(team)
    print(kept)
```

<!-- output: examples/reference-ownership/moves.out -->
```text
["Aino", "Eero"]
```

A use after a move is E0400, which names the line of the move. In a
loop, a move in one iteration and a use in the next are the same
error.

<!-- file: examples/reference-ownership/used_after_move.sisu -->
```sisu
# A value used after it moved.
def main():
    let names = ["Aino", "Eero"]
    let team = names
    print(names)
    print(team)
```

<!-- output: examples/reference-ownership/used_after_move.console -->
```console
$ sisu run used_after_move.sisu
error[E0400]: `names` was moved on line 4 and cannot be used afterwards
  --> used_after_move.sisu:5:11
  |
5 |     print(names)
  |           ^^^^^
  |
4 |     let team = names
  |                ----- `names` is moved here
  = note: copy it with `.copy()` before it is moved, or move it later

error[D0021]: the program was not run: the ownership checker found errors (1); `--defer-borrow` runs it under the monitor
```

**Status.** In stage 0 (D-004).

## Copy types and explicit copies

A value of a `Copy` type is copied where another value would move, so
both names stay usable. The builtin numbers, `bool`, `char`, `str` (a
view of text, copied as the view), `()`, and tuples of `Copy` values
are `Copy`; a struct or an enum is `Copy` when it says
`@derive(Copy)` and all its fields are `Copy` (E0244 otherwise). Copying
is opt-in for a type of your own (D-007).

Any other value is copied only by `.copy()`, which says where memory is
taken: a list's copy is a new list with copies of its elements, and a
`Shared`'s copy is one more owner of the same value.

<!-- file: examples/reference-ownership/copies.sisu -->
```sisu
# `Copy` values copy; any other value copies only with `.copy()`.
@derive(Copy, Debug)
struct Point:
    x: int
    y: int

def main():
    let a = Point(1, 2)
    let b = a
    let names = ["Aino"]
    var more = names.copy()
    more.push("Eero")
    print("{a:?} {b:?} {names} {more}")
```

<!-- output: examples/reference-ownership/copies.out -->
```text
Point(x=1, y=2) Point(x=1, y=2) ["Aino"] ["Aino", "Eero"]
```

**Status.** In stage 0 (D-007).

## Moving out of part of a value

A field can be moved out of a struct that its owner holds. The other
fields stay usable, and the struct as a whole cannot be used until the
field is given a value again (E0401):

<!-- file: examples/reference-ownership/partial.sisu -->
```sisu
# A field moved out of a struct, which cannot then be used whole.
struct Order:
    customer: String
    items: list[String]

def main():
    let order = Order("Aino", ["coffee", "bun"])
    let items = order.items
    print("{order.customer} has {len(items)} items")
    print("{order:?}")
```

<!-- output: examples/reference-ownership/partial.console -->
```console
$ sisu run partial.sisu
error[E0401]: `order` cannot be used as a whole, because `order.items` was moved out of it on line 8
  --> partial.sisu:10:13
   |
10 |     print("{order:?}")
   |             ^^^^^
   |
 8 |     let items = order.items
   |                 ----------- `order.items` is moved here
  = note: copy it with `.copy()` before it is moved, or move it later

error[D0021]: the program was not run: the ownership checker found errors (1); `--defer-borrow` runs it under the monitor
```

Nothing moves out of a value reached through a reference, an index or a
key: indexing a list borrows it, so taking an element that is not `Copy`
is E0338, whose fix copies it, and the same through a reference is
E0419. A field of a type that implements `Drop` cannot move out on its
own (E0416), because `drop` needs the whole value.

**Status.** In stage 0.

## Drops and their order

When an owner goes out of scope, its value is dropped: the value's own
`Drop`, if its type implements one, runs first, then its fields are
dropped in reverse declaration order. Within a scope, values are dropped
in reverse order of their declaration, at the end of the scope rather
than at their last use, so a lock guard or a file is released at a place
you can see (D-008). In addition:

- a moved value is dropped by its new owner, not by the old one;
- a value that an assignment replaces is dropped at the assignment;
- a temporary is dropped at the end of its statement;
- a panic unwinds the task and drops every value on its way, in the
  same order; a panic during unwinding aborts the program.

<!-- file: examples/reference-ownership/drops.sisu -->
```sisu
# Values are dropped at the end of their scope, the last made first.
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
    let couple = Couple(Guest("Liisa"), Guest("Pekka"))
    stay(Guest("Eero"))
    var host = Guest("Roni")
    host = Guest("Ella")
    print("the evening ends")
```

<!-- output: examples/reference-ownership/drops.out -->
```text
Eero stays for tea
Eero goes home
Roni goes home
the evening ends
Ella goes home
Pekka goes home
Liisa goes home
Aino goes home
```

`Eero` is dropped when `stay`, its owner, returns; `Roni` when `Ella`
replaces him; the rest at the end of `main`, the last declared first,
and the couple's fields second first.

**Status.** In stage 0 (D-008).

## Parameter conventions

A parameter says how it takes its argument (D-006):

| Convention | Declared | At the call | The function may |
|---|---|---|---|
| Borrow | `name: T` | `f(x)` | read the value for the call |
| Mutable borrow | `mut name: T` | `f(mut x)` | change the caller's value in place |
| Own | `own name: T` | `f(x)` | keep the value: it moves in, or is copied when `Copy` |

- A method's first parameter is `self`, `mut self` or `own self`, with
  the same meanings.
- A call to a `mut` parameter says `mut` before the argument, so a
  reader of the call sees what can change: E0339 without it, E0340 with
  it where the parameter is not `mut`.
- A `mut` argument is a place that can change: a `var` binding, a
  `mut` parameter, or a field or element of one (E0341 otherwise).
- A plain parameter that is assigned to is E0312; its fix makes it
  `mut`, which changes every call, where a local `var` copy is often
  what is wanted.

<!-- file: examples/reference-ownership/conventions.sisu -->
```sisu
# A plain parameter borrows, `mut` borrows to change, `own` takes.
struct Account:
    owner: String
    balance: int

def show(account: Account) -> String:
    return "{account.owner}: {account.balance}"

def deposit(mut account: Account, amount: int):
    account.balance += amount

def close(own account: Account, mut closed: list[Account]):
    closed.push(account)

def main():
    var aino = Account("Aino", 100)
    deposit(mut aino, 50)
    print(show(aino))
    var closed: list[Account] = []
    close(aino, mut closed)
    print("closed: {show(closed[0])}")
```

<!-- output: examples/reference-ownership/conventions.out -->
```text
Aino: 150
closed: Aino: 150
```

**Status.** In stage 0 (D-006).

## References

`ref x` borrows `x` and `ref mut x` borrows it to change it; their types
are `ref T` and `ref mut T`. A reference can be bound to a local,
passed as an argument and returned from a function, tied to a borrowed
parameter. References are second-class: a plain struct, an enum, a
`Shared`, a collection or a closure that escapes cannot hold one, so
every reference's lifetime is the frame or the borrowed parameter it
came from, and no reference has a lifetime name.

A `ref mut` local is used as the value it borrows: `last += 1` changes
the element it borrows.

**Status.** In stage 0 (D-004).

## One mutable borrow or many shared ones

At any point a value has either one `mut` borrow, or any number of
shared ones, and its owner can change or move it only while it has
none. A borrow lasts from where it is made to its last use, not to the
end of its block, so a change after the last use is allowed:

<!-- file: examples/reference-ownership/aliasing.sisu -->
```sisu
# A borrow ends at its last use, after which the value can change.
def main():
    var scores = [7, 9, 4]
    let first = ref scores[0]
    print("first {first}")
    scores.push(10)
    let last = ref mut scores[^1]
    last += 1
    print(scores)
```

<!-- output: examples/reference-ownership/aliasing.out -->
```text
first 7
[7, 9, 4, 11]
```

Breaking the rule is one of four errors, each naming the line of the
borrow and the line where it ends: a change while borrowed (E0402), a
use while borrowed with `mut` (E0403), a move while borrowed (E0404),
and a `mut` borrow while borrowed (E0405), such as a call of a `mut
self` method:

<!-- file: examples/reference-ownership/conflict.sisu -->
```sisu
# A value changed while a borrow of it is still used.
def main():
    var scores = [7, 9, 4]
    let first = ref scores[0]
    scores.push(10)
    print("first {first}")
```

<!-- output: examples/reference-ownership/conflict.console -->
```console
$ sisu run conflict.sisu
error[E0405]: `scores` is already borrowed on line 4, so it cannot be borrowed with `mut` until that borrow ends on line 6
  --> conflict.sisu:5:5
  |
5 |     scores.push(10)
  |     ^^^^^^^^^^^^^^^
  |
4 |     let first = ref scores[0]
  |                 ------------- borrowed here by `first`
  |
6 |     print("first {first}")
  |                   ----- the borrow is still used here
  = note: move this after line 6, where the borrow ends, or work on a copy made with `.copy()`

error[D0021]: the program was not run: the ownership checker found errors (1); `--defer-borrow` runs it under the monitor
```

A borrow that outlives what it borrows is E0406. `sisu explain-borrow
FILE:LINE` lists the borrows live on a line ([R16](sisu-command.md#sisu-explain-borrow)).

**Status.** In stage 0.

## Returning a borrow

A function that returns a reference or a view returns a borrow of one
of its parameters, found by these rules, in order:

1. a function with one borrowed parameter returns a borrow of it;
2. otherwise, a method returns a borrow of `self`;
3. otherwise, a `from` clause names the parameters the result may
   borrow from: `-> str from a, b` (E0413 without it).

A parameter counts as borrowed when it is not `own` and its type is not
`Copy`, or when its type is a view. Returning a borrow of a local is
E0407, of a parameter the `from` clause does not name E0408; a `from`
that names something else is E0414, and one on a result that does not
borrow E0415.

<!-- file: examples/reference-ownership/returned.sisu -->
```sisu
# A returned borrow comes from a parameter, named by `from` if need be.
struct Team:
    players: list[String]

    def captain(self) -> ref String:
        return ref self.players[0]

def first(words: list[String]) -> ref String:
    return ref words[0]

def longer(a: str, b: str) -> str from a, b:
    return if len(a) >= len(b): a else: b

def main():
    let owls = Team(["Aino", "Eero"])
    print("{owls.captain()} {first(owls.players)} {longer("sauna", "sisu")}")
```

<!-- output: examples/reference-ownership/returned.out -->
```text
Aino Aino sauna
```

**Status.** In stage 0 (D-004).

## Views

A view is a value that holds a borrow. `str`, `Slice[T]`, an iterator
over a borrowed collection and a `dyn Trait` behind `ref` are views,
and a `view struct` declares one of your own: its fields may be
references and views. A view has one lifetime, the shortest of its
fields', and is second-class like a reference: it cannot leave the frame
that owns what it borrows. A collection or a struct that holds a view
is a view too.

<!-- file: examples/reference-ownership/views.sisu -->
```sisu
# A `view struct` holds borrows of text that lives elsewhere.
view struct Token:
    text: str
    line: int

def tokens(source: str) -> list[Token]:
    var found: list[Token] = []
    for i, line in enumerate(source.lines()):
        for word in line.split_whitespace():
            found.push(Token(word, i + 1))
    return found

def main():
    let source = "let x\nprint x"
    for token in tokens(source):
        print("{token.line}: {token.text}")
```

<!-- output: examples/reference-ownership/views.out -->
```text
1: let
1: x
2: print
2: x
```

- A plain struct with a field that borrows is E0411, whose fix makes it
  a `view struct`; a field of type `String` instead of `str` makes it
  own its text, which is usually the fix you want.
- An enum has no view form, so a variant that would hold a borrow is
  E0412.
- A view put into a value that can outlive it, such as a `Shared` or a
  `Mutex`, is E0421; a value stored into a parameter of the caller that
  would borrow from the callee, E0410; a borrow kept in a plain type,
  E0409.
- A view returned past the frame that owns what it borrows is E0407:

<!-- file: examples/reference-ownership/escaping_view.sisu -->
```sisu
# A view that would outlive the text it borrows.
view struct Token:
    text: str

def first_token() -> Token:
    let source = "let x".to_string()
    return Token(source.split_whitespace().next() ?? "")

def main():
    print(first_token().text)
```

<!-- output: examples/reference-ownership/escaping_view.console -->
```console
$ sisu run escaping_view.sisu
error[E0407]: this returns a borrow of `source`, which belongs to `first_token` and is gone when it returns
  --> escaping_view.sisu:7:5
  |
7 |     return Token(source.split_whitespace().next() ?? "")
  |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  |                  ------ borrowed here
  = note: return an owned value instead, such as a copy made with `.copy()`

error[D0021]: the program was not run: the ownership checker found errors (1); `--defer-borrow` runs it under the monitor
```

**Status.** In stage 0 (D-004).

## Closures and their captures

A closure borrows what it captures, for as long as the closure is used,
under the same rules as any borrow. A closure that is stored or returned
must be an `own` closure, `own (x) -> ...` or `do own x:`, which keeps
its own copies of what it captures. Function types do not record how a
closure captures; the checker infers it for each closure.

<!-- file: examples/reference-ownership/closures.sisu -->
```sisu
# A closure borrows what it captures; an `own` closure keeps copies.
def make_adder(step: int) -> (int) -> int:
    return own (n: int) -> n + step

def main():
    let limit = 10
    let small = [n for n in [3, 12, 7] if n < limit]
    let add_two = make_adder(2)
    print("{small} {add_two(5)}")
```

<!-- output: examples/reference-ownership/closures.out -->
```text
[3, 7] 7
```

**Stage 0 differs.** [04](../../proposal/04-memory-and-ownership.md)
lets an `own` closure move its captures in. Stage 0 copies them, so it
captures `Copy` values only (E0423), and it cannot change them (E0424):

<!-- file: examples/reference-ownership/own_string_closure.sisu -->
```sisu
# An `own` closure that would keep a `String`.
def make_greeter(own greeting: String) -> (str) -> String:
    return own (name: str) -> "{greeting}, {name}!"

def main():
    let hei = make_greeter("Hei".to_string())
    print(hei("Aino"))
```

<!-- output: examples/reference-ownership/own_string_closure.console -->
```console
$ sisu run own_string_closure.sisu
error[E0423]: an `own` closure keeps a copy of `greeting`, and `String` is not `Copy`; stage 0 copies captures (a capture by move arrives with tasks)
  --> own_string_closure.sisu:3:12
  |
3 |     return own (name: str) -> "{greeting}, {name}!"
  |            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  = note: copy it into a local first (`let c = name.copy()`) and capture that, or use a plain closure, which borrows it

error[D0006]: the program was not run: errors found (1)
```

The note's advice does not help here, since a copy of a `String` is a
`String` too; until moves arrive, keep the value outside the closure.

**Status.** In stage 0, with the difference above (D-026).

## Iterating in three modes

| Loop | Yields | The collection |
|---|---|---|
| `for x in xs` | `ref T` | is borrowed for the loop |
| `for x in mut xs` | `ref mut T` | is borrowed with `mut`, and each element can change |
| `for x in own xs` | `T` | is consumed: each element moves out |

<!-- file: examples/reference-ownership/iteration.sisu -->
```sisu
# `for` borrows, `for ... in mut` changes in place, `for ... in own` takes.
def main():
    var prices = [12, 5, 30]
    var total = 0
    for p in prices:
        total += p
    for p in mut prices:
        p += 1
    var names: list[String] = ["Aino", "Eero"]
    var kept: list[String] = []
    for name in own names:
        kept.push(name)
    print("{total} {prices} {kept}")
```

<!-- output: examples/reference-ownership/iteration.out -->
```text
47 [13, 6, 31] ["Aino", "Eero"]
```

Changing a collection inside a loop over it is E0405, since the loop
borrows it. A method that returns a reference to part of `self`, such as
`get`, `first` or `entry`, is an ordinary function returning `ref`.

**Status.** In stage 0.

## Values that must be consumed

A type marked `@must_consume` cannot be dropped silently: each of its
values is passed to an `own` parameter before it goes out of scope,
usually a method that takes `own self` and says what ending it means,
such as `commit`, `rollback` or `close` (D-054). Unwinding after a panic
is exempt.

<!-- file: examples/reference-ownership/must_consume.sisu -->
```sisu
# A must-consume value is ended by a method that takes `own self`.
@must_consume
struct Draft:
    text: String

    def publish(own self, mut shelf: list[String]):
        shelf.push(self.text)

    def discard(own self):
        pass

def main():
    var shelf: list[String] = []
    let letter = Draft("Hei Aino")
    let scrap = Draft("a first try")
    letter.publish(mut shelf)
    scrap.discard()
    print(shelf)
```

<!-- output: examples/reference-ownership/must_consume.out -->
```text
["Hei Aino"]
```

A value that goes out of scope unconsumed is E0417, whose note lists the
methods that consume it; leaving a scope early with one unconsumed, by
`return`, `break` or `?`, is E0418:

<!-- file: examples/reference-ownership/forgotten.sisu -->
```sisu
# A must-consume value that goes out of scope unconsumed.
@must_consume
struct Draft:
    text: String

    def discard(own self):
        pass

def main():
    let scrap = Draft("a first try")
    print(scrap.text)
```

<!-- output: examples/reference-ownership/forgotten.console -->
```console
$ sisu run forgotten.sisu
error[E0417]: `scrap` is never consumed: a `Draft` must be passed to an `own` parameter before it goes out of scope
  --> forgotten.sisu:11:22
   |
11 |     print(scrap.text)
   |                      ^
   |
10 |     let scrap = Draft("a first try")
   |         ----- `scrap` is declared here
  = note: consume it with one of its methods that take `own self`: `discard`

error[D0021]: the program was not run: the ownership checker found errors (1); `--defer-borrow` runs it under the monitor
```

A collection of must-consume values is itself must-consume, and a
consuming loop discharges it (leaning, D-064). Such a value cannot go
into a type that may drop it out of the checker's sight (E0422), nor to
a function of the standard library that may drop it (E0420).

**Status.** In stage 0 (D-054; containers lean, D-064).

## Guaranteed drop

Three kinds of value end only by running their `Drop`, on scope exit or
while unwinding: a suspended task's frame, a `task.scope`, and every
view. The standard library has no safe `forget` or `leak` for them, so
a buffer lent to the kernel or to a device is never written after the
call that lent it has returned, even when the call is cancelled. Only an
abort of the process, with `panic = abort` or on a panic during
unwinding, skips the drops.

**Status.** Leaning (D-058); stage 0 has no safe `forget`.

## Sharing, cells and graphs

Values that need more than one owner, or change behind a shared one, use
the standard library's types, each visible in the program's types:

| Type | For | Cost |
|---|---|---|
| `Box[T]` | a value of its own allocation; a recursive enum is boxed for you (D-043) | one allocation |
| `Shared[T]` | several owners of one value, which is dropped with the last; `.copy()` adds an owner, `Shared.count(s)` counts them | a count, atomic when the value can reach another task |
| `Weak[T]` | a reference that does not keep a `Shared` value alive, to break cycles; `upgrade` gives `Shared[T]?` | a second count |
| `Cell[T]` | a value that changes behind shared owners, in one task: `get`, `set` | none |
| `Mutex[T]`, `RwLock[T]`, `Atomic` | a value that changes behind owners in several tasks ([R9](concurrency.md#sharing-a-value-that-changes)) | a lock or an atomic operation |
| `Arena[T]` and `Key[T]` | the nodes of a graph, named by keys that are checked at use: a removed node's key finds nothing | a check of the key's generation |

A `Shared` value is read through, and changed only through a `Cell`, a
`Mutex` or an `Atomic` inside it, so that the `Send` and `Sync` rules can
tell which is right. There is no cycle collector: a cycle of `Shared`
values leaks unless a `Weak` breaks it.

<!-- file: examples/reference-ownership/shared.sisu -->
```sisu
# `Shared` owns a value with others; `Cell` lets them change it.
struct Counter:
    hits: Cell[int]

def main():
    let counter = Shared(Counter(Cell(0)))
    let other = counter.copy()
    other.hits.set(other.hits.get() + 1)
    counter.hits.set(counter.hits.get() + 1)
    print("{Shared.count(counter)} owners, {counter.hits.get()} hits")
```

<!-- output: examples/reference-ownership/shared.out -->
```text
2 owners, 2 hits
```

<!-- file: examples/reference-ownership/arena.sisu -->
```sisu
# An `Arena` holds the nodes of a graph, and a `Key` names one.
struct Town:
    name: String
    roads: list[Key[Town]]

def main():
    var map = Arena[Town]()
    let oulu = map.insert(Town("Oulu", []))
    let kemi = map.insert(Town("Kemi", []))
    map[oulu].roads.push(kemi)
    map[kemi].roads.push(oulu)
    let next = map[oulu].roads[0]
    print("from {map[oulu].name} to {map[next].name}")
    map.remove(kemi)
    print(map.get(kemi)?.name ?? "Kemi is gone")
```

<!-- output: examples/reference-ownership/arena.out -->
```text
from Oulu to Kemi
Kemi is gone
```

`sisu audit --ownership` counts the uses of these types and of
`.copy()` in each file ([R16](sisu-command.md#sisu-audit)).

**Stage 0 differs.** `Shared.same(a, b)`, which asks whether two owners
hold the very same value, is designed in
[02](../../proposal/02-syntax.md) and stage 0 does not have it.

**Status.** In stage 0 (D-005, D-043).

## Allocators

`list`, `dict`, `set`, `String`, `Box` and `Shared` allocate; a struct,
an array, an enum and a tuple do not, and closures, generics and `dyn`
allocate nothing hidden. A collection's allocator is a type parameter
with a default, `list[T, A: Allocator = Global]`, so ordinary code never
names one, and `list.with_allocator(a)` builds a collection on another.
A borrowed bump region, `Region.bump(capacity=n)`, is an allocator: a
collection built on it holds a reference to it and is a view, so it
cannot leave the region's frame, and resetting the region waits until
every such collection is dead. Allocation failure panics; `try_push`
and its kind return `! OutOfMemory` for code that must handle it.

**Status.** Designed (D-060, leaning; Q-4); stage 0 has the global
allocator only, and none of `with_allocator`, `Region` or `try_push`.

## Unsafe code and raw pointers

`unsafe:` blocks and `unsafe e` permit raw pointers (`ptr[T]`, `ptr mut
T`), foreign calls and the few operations whose safety the compiler
cannot check. The rules of safe code apply unchanged around them, and
the code inside must keep the invariants that safe code relies on,
guaranteed drop among them. A dependency may contain `unsafe` only when
the root manifest lists it under `[trust]`.

**Status.** Designed ([04](../../proposal/04-memory-and-ownership.md),
[22](../../proposal/22-unsafe.md)), and arriving: the compiler checks
`unsafe` blocks and expressions, and warns about one that holds no
unsafe operation (W0009); raw pointers and foreign calls do not run in
stage 0 yet.

## How the rules are checked

- The ownership checker runs on SIR, one function at a time, after the
  type checker. It is sound before it is complete: where it cannot tell,
  it rejects, and each such case is a known over-rejection that the
  [architecture notes](../../compiler/docs/architecture.md) list.
- `sisu run` does not run a program the checker rejects, and says so
  with D0021, which names `--defer-borrow`: that runs the program under
  the ownership monitor to see whether the conflict happens
  ([R16](sisu-command.md#running-a-rejected-program---defer-borrow)).
- `sisu test` runs every test under the monitor, and `sisu run
  --monitor` runs a program under it; a program the checker accepted
  never trips it.

**Status.** In stage 0 (18, Phase 3).

## Diagnostics of this page

| Code | Identifier | Section |
|---|---|---|
| [E0244](diagnostics.md#e0244) | `resolve.derive-field` | [Copy types and explicit copies](#copy-types-and-explicit-copies) |
| [E0338](diagnostics.md#e0338) | `type.move-out-of-borrow` | [Moving out of part of a value](#moving-out-of-part-of-a-value) |
| [E0339](diagnostics.md#e0339), [E0340](diagnostics.md#e0340) | `type.mut-argument-missing`, `type.mut-argument-unexpected` | [Parameter conventions](#parameter-conventions) |
| [E0341](diagnostics.md#e0341), [E0312](diagnostics.md#e0312) | `type.not-mutable`, `type.assign-immutable` | [Parameter conventions](#parameter-conventions) |
| [E0400](diagnostics.md#e0400) | `borrow.use-after-move` | [Every value has one owner](#every-value-has-one-owner) |
| [E0401](diagnostics.md#e0401) | `borrow.use-after-partial-move` | [Moving out of part of a value](#moving-out-of-part-of-a-value) |
| [E0402](diagnostics.md#e0402), [E0403](diagnostics.md#e0403), [E0404](diagnostics.md#e0404), [E0405](diagnostics.md#e0405) | `borrow.modify-while-borrowed`, `borrow.use-while-mut-borrowed`, `borrow.move-while-borrowed`, `borrow.mut-while-borrowed` | [One mutable borrow or many shared ones](#one-mutable-borrow-or-many-shared-ones) |
| [E0406](diagnostics.md#e0406) | `borrow.does-not-live-long-enough` | [One mutable borrow or many shared ones](#one-mutable-borrow-or-many-shared-ones) |
| [E0407](diagnostics.md#e0407), [E0408](diagnostics.md#e0408), [E0413](diagnostics.md#e0413), E0414, E0415 | `borrow.return-local`, `borrow.return-not-from`, `borrow.from-required`, `borrow.from-not-a-parameter`, `borrow.from-without-borrow` | [Returning a borrow](#returning-a-borrow) |
| E0409, E0410, [E0411](diagnostics.md#e0411), [E0412](diagnostics.md#e0412), E0421 | `borrow.store-in-plain-type`, `borrow.store-into-caller`, `borrow.view-field-in-plain-struct`, `borrow.view-in-enum`, `borrow.view-in-owner` | [Views](#views) |
| E0416, E0419 | `borrow.move-out-of-drop`, `borrow.move-out-of-place` | [Moving out of part of a value](#moving-out-of-part-of-a-value) |
| [E0417](diagnostics.md#e0417), [E0418](diagnostics.md#e0418), E0420, E0422 | `borrow.must-consume`, `borrow.must-consume-exit`, `borrow.must-consume-intrinsic`, `borrow.must-consume-in-owner` | [Values that must be consumed](#values-that-must-be-consumed) |
| E0423, E0424 | `type.own-closure-capture`, `type.own-closure-changes` | [Closures and their captures](#closures-and-their-captures) |

The codes of tasks, E0425 to E0437, belong to
[R9](concurrency.md#diagnostics-of-this-page), Concurrency.
