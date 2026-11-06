# 11. Borrowing

In this chapter you lend values instead of giving them away. A borrow
lets code read a value, or change it, while its owner keeps it, and the
compiler checks that no borrow outlives what it borrows and that
nothing changes a value while something else reads it. You meet the
three ways a function takes a parameter, references with names,
functions that return a borrow, the three ways a loop goes through a
list, and two commands that show you what is borrowed. You need about
an hour and a half.

## Three ways to take a parameter

Each parameter says how a function takes its argument: it borrows it to
read, borrows it with `mut` to change it, or takes it with `own`:

<!-- file: examples/11-borrowing/accounts.sisu -->
```sisu
# A parameter borrows, borrows with `mut`, or takes with `own`.
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

<!-- output: examples/11-borrowing/accounts.out -->
```text
Aino: 150
closed: Aino: 150
```

The three functions take an account in the three ways, which this
table sums up:

| The parameter | The function may | The call | Afterwards, the caller |
|---|---|---|---|
| `account: Account` | read it | `show(aino)` | has it, as it was |
| `mut account: Account` | read and change it | `deposit(mut aino, 50)` | has it, perhaps changed |
| `own account: Account` | keep it | `close(aino, mut closed)` | no longer has it |

A plain parameter is a borrow that only reads, and it is the default,
because most functions only read. A `mut` parameter is a borrow that
can change the value in place: `deposit` adds to the caller's account,
not to a copy of it. The call says `mut` as well, so that a reader of
`main` sees which lines can change `aino`, and the argument must be a
`var`. An `own` parameter, as in chapter 10, takes the value for good.

**Try it.** Delete `mut` from the call `deposit(mut aino, 50)` and run
the program.

<details>
<summary>What Sisu says</summary>

<!-- output: examples/11-borrowing/no_mut_at_call.console -->
```console
$ sisu run no_mut_at_call.sisu
error[E0339]: `account` is a `mut` parameter, so the call must say `mut` before the argument
  --> no_mut_at_call.sisu:17:13
   |
17 |     deposit(aino, 50)
   |             ^^^^
  = fix: write `mut` before the argument: `mut `

error[D0006]: the program was not run: errors found (1)
```

E0339 (`type.mut-argument-missing`) asks for the `mut` that the
parameter requires, and its fix writes it. The opposite mistake, `mut`
in a call to a parameter that only reads, gets E0340
(`type.mut-argument-unexpected`), whose fix removes it.

</details>

> **From Python.** A Python function can append to a list it is given,
> and the caller sees the change, with nothing at the call to say so.
> In Sisu a function changes an argument only through a `mut`
> parameter, and the call says `mut` too.

## Borrows with a name

`ref` borrows a value, or a part of one, and gives the borrow a name.
`ref mut` does the same, to change what it borrows:

<!-- file: examples/11-borrowing/players.sisu -->
```sisu
# `ref mut` names a place inside a value, to change it there.
struct Player:
    name: String
    points: int
    wins: int

def main():
    var players = [Player("Aino", 6, 2), Player("Eero", 2, 0)]
    let winner = ref mut players[1]
    winner.points += 3
    winner.wins += 1
    let leader = ref players[0]
    print("{leader.name} leads with {leader.points} points")
    print("{players[1].name}: {players[1].points} points")
```

<!-- output: examples/11-borrowing/players.out -->
```text
Aino leads with 6 points
Eero: 5 points
```

- `ref mut players[1]` borrows the second player where it is, inside
  the list. `winner.points += 3` changes that player: nothing is copied
  and nothing moves out of the list.
- `ref players[0]` borrows the first player to read it.
- A borrow is used like the value it borrows, so `leader.name` reads a
  field through it.

A borrow is a value of type `ref T` or `ref mut T`, a reference, and
the compiler follows it wherever it goes. A plain struct cannot hold
one, and a list that holds one cannot outlive what it borrows, as
chapter 12 shows. Those limits are what let the compiler check every
borrow without a word from you (D-004).

## One writer or many readers

Any number of borrows can read a value at the same time:

<!-- file: examples/11-borrowing/readers.sisu -->
```sisu
# Many borrows can read a value at once, and each ends at its last use.
def main():
    var scores = [7, 9, 4]
    let first = ref scores[0]
    let last = ref scores[^1]
    print("first {first}, last {last}, of {len(scores)}")
    scores.push(10)
    print(scores)
```

<!-- output: examples/11-borrowing/readers.out -->
```text
first 7, last 4, of 3
[7, 9, 4, 10]
```

`first` and `last` both borrow from `scores`, and `len(scores)` reads
it too. `scores.push(10)` then changes the list, and that is allowed
because neither borrow is used after it: a borrow ends at its last use,
not at the end of its block. Change the list while a borrow of it is
still to be used, and the program no longer compiles:

<!-- file: examples/11-borrowing/grow_while_reading.sisu -->
```sisu
# A list is changed while a borrow of its first element is still used.
def main():
    var scores = [7, 9, 4]
    let first = ref scores[0]
    scores.push(10)
    print("first {first}, of {len(scores)}")
```

<!-- output: examples/11-borrowing/grow_while_reading.console -->
```console
$ sisu run grow_while_reading.sisu
error[E0405]: `scores` is already borrowed on line 4, so it cannot be borrowed with `mut` until that borrow ends on line 6
  --> grow_while_reading.sisu:5:5
  |
5 |     scores.push(10)
  |     ^^^^^^^^^^^^^^^
  |
4 |     let first = ref scores[0]
  |                 ------------- borrowed here by `first`
  |
6 |     print("first {first}, of {len(scores)}")
  |                   ----- the borrow is still used here
  = note: move this after line 6, where the borrow ends, or work on a copy made with `.copy()`

error[D0021]: the program was not run: the ownership checker found errors (1); `--defer-borrow` runs it under the monitor
```

E0405 (`borrow.mut-while-borrowed`) shows the conflict on three lines:
the borrow and what holds it, the change, and where the borrow is still
used. The danger is real: `push` may move the elements to a larger
block of memory, and `first` would then point to memory that the list
no longer uses. The rule that rules this out has two halves:

- While a value is borrowed to read, it can be read, but not changed
  or moved: E0402 for an assignment, E0404 for a move and E0405 for a
  borrow with `mut`.
- While a value is borrowed with `mut`, it can be used only through
  that borrow (E0403).

The note gives two fixes. Make the change after the last use of the
borrow, as `readers.sisu` does; or keep a copy of what you need, so
that no borrow is left: `let first = scores[0]` copies the number,
which has no tie to the list.

## Returning a borrow

A function can return a borrow of what it was given, which gives its
caller a part of a value without a copy:

<!-- file: examples/11-borrowing/team.sisu -->
```sisu
# A function can return a borrow of what it was given.
struct Team:
    name: String
    players: list[String]

    def captain(self) -> ref String:
        return ref self.players[0]

def longest(words: list[String]) -> ref String:
    var best = 0
    for i, word in enumerate(words):
        if len(word) > len(words[best]):
            best = i
    return ref words[best]

def longer(a: str, b: str) -> str from a, b:
    return if len(a) >= len(b): a else: b

def main():
    let owls = Team("Owls", ["Aino", "Eero", "Pekka", "Ella"])
    print("{owls.name} are led by {owls.captain()}")
    print("the longest name: {longest(owls.players)}")
    print(longer("sauna", "sisu"))
```

<!-- output: examples/11-borrowing/team.out -->
```text
Owls are led by Aino
the longest name: Pekka
sauna
```

A caller must keep alive whatever a returned borrow borrows from, for
as long as it uses the borrow, so the signature says what that is. The
compiler works it out in two cases, and asks in the third:

- A method's result borrows from `self`: `captain` returns a borrow of
  a player of the team.
- A function with one borrowed parameter returns a borrow of it:
  `longest` borrows from `words`.
- Otherwise a `from` clause names the parameters: `longer` returns `a`
  or `b`, so it says `-> str from a, b`.

**Try it.** Delete ` from a, b` from the line `def longer` and run the
program.

<details>
<summary>What Sisu says</summary>

<!-- output: examples/11-borrowing/no_from.console -->
```console
$ sisu run no_from.sisu
error[E0413]: `longer` returns `str`, which borrows, and has more than one borrowed parameter (a, b); name the ones it borrows from with `from`
  --> no_from.sisu:16:31
   |
16 | def longer(a: str, b: str) -> str:
   |                               ^^^
   |            ------ `a` is a borrowed parameter
   |                    ------ `b` is a borrowed parameter
  = fix: name `a, b` in the `from` clause: ` from a, b`
  = fix: return owned text instead: `String`

error[D0006]: the program was not run: errors found (1)
```

E0413 (`borrow.from-required`) marks the two borrowed parameters. Its
first fix writes the clause back, since `longer` returns one of them;
the second, a `String` result, would copy text that `longer` only
passes on, and is the one to take for text the function makes. With the clause, the caller
knows that the result may borrow from either argument, and keeps both
alive while it uses the result.

</details>

A function cannot return a borrow of a value it made itself, because
that value ends when the function returns:

<!-- file: examples/11-borrowing/greeting.sisu -->
```sisu
# A function returns a borrow of text that it made itself.
def greeting(name: str) -> str:
    let text = "Hei, {name}!"
    return text

def main():
    print(greeting("Aino"))
```

<!-- output: examples/11-borrowing/greeting.console -->
```console
$ sisu run greeting.sisu
error[E0407]: this returns a borrow of `text`, which belongs to `greeting` and is gone when it returns
  --> greeting.sisu:4:5
  |
4 |     return text
  |     ^^^^^^^^^^^
  |            ---- borrowed here
  = note: a `str` views text, and a copy of it is the same view: return a `String`, which owns its text (`-> String`, or `list[String]` for views in a list)

error[D0021]: the program was not run: the ownership checker found errors (1); `--defer-borrow` runs it under the monitor
```

The interpolation makes a `String` that belongs to `greeting`, and a
`str` result would borrow it. E0407 (`borrow.return-local`) says so,
and its note asks for an owned result: a copy of a `str` is the same
view, so the fix is the result type `String`, which moves the text out
to the caller.

## Three ways through a list

A `for` loop takes its list in the three ways that a parameter does:

<!-- file: examples/11-borrowing/loops.sisu -->
```sisu
# A `for` loop borrows the list, borrows it with `mut`, or takes it.
def main():
    var prices = [300, 280, 350]
    var total = 0
    for price in prices:
        total += price
    print("{prices} add up to {total}")
    for price in mut prices:
        price = price * 9 // 10
    print("on sale: {prices}")
    let words: list[String] = ["sauna", "sisu", "lake", "birch"]
    var short: list[String] = []
    var long: list[String] = []
    for word in own words:
        if len(word) <= 4:
            short.push(word)
        else:
            long.push(word)
    print("short: {short}, long: {long}")
```

<!-- output: examples/11-borrowing/loops.out -->
```text
[300, 280, 350] add up to 930
on sale: [270, 252, 315]
short: ["sisu", "lake"], long: ["sauna", "birch"]
```

- `for price in prices` borrows the list, and `price` borrows each
  element to read it.
- `for price in mut prices` borrows the list with `mut`, and `price`
  is a `ref mut` to each element, so `price = price * 9 // 10` changes
  the element inside the list.
- `for word in own words` takes the list and moves each element out to
  `word`, which can then move on, here into one of two lists. After the
  loop, `words` is gone.

The loop's borrow lasts until the loop ends, so the body cannot change
the list that the loop goes through: no `push` or `remove`, and for a
dict, no keys added or removed. Collect the changes in a list of their
own, and make them after the loop.

> **From Python.** Changing a list while a `for` loop goes through it
> makes Python skip or repeat elements, and changing a dict's keys
> raises `RuntimeError` when the loop goes on. Sisu refuses both before
> the program runs.

## Closures borrow what they use

A closure is a function written as an expression, such as `x -> x *
rate`, which chapter 16 covers. A closure can use the variables around
it, and it borrows them for as long as the closure is used:

<!-- file: examples/11-borrowing/closures.sisu -->
```sisu
# A closure borrows the variables it uses, while it is used.
def apply_all(xs: list[int], f: (int) -> int) -> list[int]:
    return [f(x) for x in xs]

def multiplier(factor: int) -> (int) -> int:
    return own x -> x * factor

def main():
    var rate = 2
    let scale = x -> x * rate
    print(apply_all([1, 2, 3], scale))
    rate = 10
    print(apply_all([1, 2, 3], x -> x * rate))
    let triple = multiplier(3)
    print(triple(5))
```

<!-- output: examples/11-borrowing/closures.out -->
```text
[2, 4, 6]
[10, 20, 30]
15
```

`scale` borrows `rate`, so `rate` cannot change while `scale` may still
be called. After the last use of `scale`, `rate = 10` is allowed, and
the next closure sees the new rate. `multiplier` returns its closure,
which outlives the call, so it is an `own` closure: it keeps a copy of
`factor` instead of borrowing it. Without `own`, the closure would
borrow a parameter that ends with the call, and E0407 says so, with the
note that a closure that is stored or returned must own what it
captures.

**Try it.** Add `print(scale(1))` as the last line of `main`.

<details>
<summary>What Sisu says</summary>

<!-- output: examples/11-borrowing/rate_changed.console -->
```console
$ sisu run rate_changed.sisu
error[E0402]: `rate` is borrowed on line 10 and cannot be modified until that borrow ends on line 16
  --> rate_changed.sisu:12:5
   |
12 |     rate = 10
   |     ^^^^^^^^^
   |
10 |     let scale = x -> x * rate
   |                 ------------- borrowed here by `scale`
   |
16 |     print(scale(1))
   |           -------- the borrow is still used here
  = note: move this after line 16, where the borrow ends, or work on a copy made with `.copy()`

error[D0021]: the program was not run: the ownership checker found errors (1); `--defer-borrow` runs it under the monitor
```

E0402 (`borrow.modify-while-borrowed`) names `scale` as the holder of
the borrow, and the new last line as where it is still used. Change
`rate` after that line, or give the last call a closure of its own.

</details>

## Asking about borrows

When a conflict is hard to see, `sisu explain-borrow FILE:LINE` lists
every borrow that is live on a line, with what holds it and how long it
lasts:

<!-- output: examples/11-borrowing/explain.console -->
```console
$ sisu explain-borrow grow_while_reading.sisu:5
borrows live on line 5 of grow_while_reading.sisu, in `main`:
  a shared borrow of `scores[...]` from 4:17, held by `first`, used until 6:19
  a `mut` borrow of `scores` from 5:5, held by a temporary, used until 5:5
```

Line 5 of `grow_while_reading.sisu` has two: the borrow that `first`
holds, made on line 4 and used until line 6, and the `mut` borrow that
`push` makes for its call. `scores[...]` stands for any element,
because the check does not tell the elements of a list apart.

That means the check can refuse a program that is safe. This one
changes the second name while it holds a borrow of the first:

<!-- file: examples/11-borrowing/elements.sisu -->
```sisu
# One element changes while another is borrowed: safe, but rejected.
def main():
    var names: list[String] = ["Aino", "Eero", "Roni"]
    let first = ref names[0]
    names[1] = "Ella"
    print("{first} is first: {names}")
```

<!-- output: examples/11-borrowing/elements.console -->
```console
$ sisu run elements.sisu
error[E0402]: `names` is borrowed on line 4 and cannot be modified until that borrow ends on line 6
  --> elements.sisu:5:5
  |
5 |     names[1] = "Ella"
  |     ^^^^^^^^^^^^^^^^^
  |
4 |     let first = ref names[0]
  |                 ------------ borrowed here by `first`
  |
6 |     print("{first} is first: {names}")
  |             ----- the borrow is still used here
  = note: move this after line 6, where the borrow ends, or work on a copy made with `.copy()`

error[D0021]: the program was not run: the ownership checker found errors (1); `--defer-borrow` runs it under the monitor
$ sisu run --defer-borrow elements.sisu
counterexample run: the ownership checker rejected this program; its errors are deferred to the monitor
Aino is first: ["Aino", "Ella", "Roni"]
outcome: no conflict; this run ended without an ownership conflict
  the checker still rejects line 5 (E0402 borrow.modify-while-borrowed), which this run reached without a conflict (count: 2)
every rejected statement was reached without a conflict: a candidate false positive
```

`sisu run --defer-borrow` runs a program that the ownership check
refused, under a monitor that watches every borrow as the program runs
and stops it at the first real conflict. Here it found none, because
the monitor tells the two elements apart. Its verdict, a candidate
false positive, holds for this run and no other, and the program still
does not compile: move the change after the `print`, or copy the first
name. The check refuses what it cannot prove safe, and the monitor
shows you whether a refusal is about your program or about the limits
of the check. The same monitor watches every run of `sisu test`.
Both commands take the borrows from the compiler's Rust front end for
now; stage 1, the front end written in Sisu, answers them too, and
`sisu` will ask it once the driver routes them
([R16](../reference/sisu-command.md#sisu-explain-borrow)).

## Exercise 11.1: a shopping list

Open
[`manual/exercises/11-borrowing/shopping.sisu`](../exercises/11-borrowing/shopping.sisu):

<!-- file: exercises/11-borrowing/shopping.sisu -->
```sisu
##! Exercise 11.1, a shopping list. Kind: fix. Level: 1.
##!
##! `add_item` adds an item to a shopping list that belongs to its
##! caller. The file does not compile. Run `sisu test shopping.sisu`,
##! read the diagnostics, and fix `add_item`, not the test, so that the
##! test passes. The hints are in chapter 11 of the tutorial, under
##! "Exercise 11.1: a shopping list".

## Adds `item` to `items`, unless it is there already.
def add_item(items: list[String], item: str):
    for existing in items:
        if existing == item:
            return
    items.push(item.to_string())

test "each item once":
    var items: list[String] = []
    add_item(mut items, "milk")
    add_item(mut items, "bread")
    add_item(mut items, "milk")
    assert items == ["milk", "bread"]
```

<!-- output: solutions/11-borrowing/shopping.console -->
```console
$ sisu test shopping.sisu
error[E0341]: cannot modify `items`; it is not declared with `var`
  --> shopping.sisu:14:5
   |
14 |     items.push(item.to_string())
   |     ^^^^^
  = fix: take `items` with `mut`: `mut `

error[E0340]: `items` is not a `mut` parameter, so the argument takes no `mut`
  --> shopping.sisu:18:14
   |
18 |     add_item(mut items, "milk")
   |              ^^^^^^^^^
  = fix: take `items` with `mut`: `mut `

error[E0340]: `items` is not a `mut` parameter, so the argument takes no `mut`
  --> shopping.sisu:19:14
   |
19 |     add_item(mut items, "bread")
   |              ^^^^^^^^^
  = fix: take `items` with `mut`: `mut `

error[E0340]: `items` is not a `mut` parameter, so the argument takes no `mut`
  --> shopping.sisu:20:14
   |
20 |     add_item(mut items, "milk")
   |              ^^^^^^^^^
  = fix: take `items` with `mut`: `mut `

error[D0006]: the program was not run: errors found (4)
```

<details>
<summary>Hint 1</summary>

Every diagnostic offers the same fix, which takes `items` with `mut`:
the function changes the list, and the calls in the test, which you do
not change, already lend it with `mut`.

</details>

<details>
<summary>Hint 2</summary>

A function that changes its caller's list says so in its parameter, as
the section "Three ways to take a parameter" shows.

</details>

When the test passes, compare your answer with
[`manual/solutions/11-borrowing/shopping.sisu`](../solutions/11-borrowing/shopping.sisu).

## Exercise 11.2: a friend each

Open
[`manual/exercises/11-borrowing/friends.sisu`](../exercises/11-borrowing/friends.sisu):

<!-- file: exercises/11-borrowing/friends.sisu -->
```sisu
##! Exercise 11.2, a friend each. Kind: fix. Level: 1.
##!
##! Every guest at a party brings a friend. The file does not compile.
##! Run `sisu test friends.sisu`, read the diagnostic, and fix
##! `invite_friends` so that every test passes. The hints are in chapter
##! 11 of the tutorial, under "Exercise 11.2: a friend each".

## Adds a friend for each guest, after all the guests: "Aino's friend"
## for "Aino".
def invite_friends(mut guests: list[String]):
    for guest in guests:
        guests.push("{guest}'s friend")

test "two guests bring two friends":
    var guests: list[String] = ["Aino", "Eero"]
    invite_friends(mut guests)
    assert guests == ["Aino", "Eero", "Aino's friend", "Eero's friend"]

test "nobody brings nobody":
    var guests: list[String] = []
    invite_friends(mut guests)
    assert guests == []
```

<!-- output: solutions/11-borrowing/friends.console -->
```console
$ sisu test friends.sisu
error[E0405]: `guests` is already borrowed on line 11, so it cannot be borrowed with `mut` until that borrow ends on line 12
  --> friends.sisu:12:9
   |
12 |         guests.push("{guest}'s friend")
   |         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   |
11 |     for guest in guests:
   |                  ------ borrowed here
   |     ------------------- the borrow is still used here
  = note: the borrow lasts through the loop, which ends after line 12: change `guests` after the loop, or loop over a copy made with `.copy()`

error[D0021]: the program was not run: the ownership checker found errors (1); `--defer-borrow` runs it under the monitor
```

<details>
<summary>Hint 1</summary>

The loop borrows `guests` for as long as it runs, and the body tries to
change the list that the loop is reading.

</details>

<details>
<summary>Hint 2</summary>

The section "Three ways through a list" says where changes to a list
go when a loop reads it.

</details>

When every test passes, compare your answer with
[`manual/solutions/11-borrowing/friends.sisu`](../solutions/11-borrowing/friends.sisu).

## Exercise 11.3: the leader

Open
[`manual/exercises/11-borrowing/leader.sisu`](../exercises/11-borrowing/leader.sisu):

<!-- file: exercises/11-borrowing/leader.sisu -->
```sisu
##! Exercise 11.3, the leader. Kind: fix. Level: 2.
##!
##! `sort_and_check` sorts a list of names and tells whether the name
##! that was first is still first. The file does not compile. Run
##! `sisu test leader.sisu`, read the diagnostic, and fix
##! `sort_and_check` so that every test passes. The hints are in chapter
##! 11 of the tutorial, under "Exercise 11.3: the leader".

## Sorts `names` in alphabetical order, and says whether the name that
## was first before the sort is first after it. `names` is not empty.
def sort_and_check(mut names: list[String]) -> bool:
    let first = ref names[0]
    names.sort()
    return names[0] == first

test "a leader who stays first":
    var names: list[String] = ["Aino", "Roni", "Eero"]
    assert sort_and_check(mut names)
    assert names == ["Aino", "Eero", "Roni"]

test "a leader who does not":
    var names: list[String] = ["Roni", "Aino"]
    assert not sort_and_check(mut names)
    assert names == ["Aino", "Roni"]
```

<!-- output: solutions/11-borrowing/leader.console -->
```console
$ sisu test leader.sisu
error[E0405]: `names` is already borrowed on line 12, so it cannot be borrowed with `mut` until that borrow ends on line 14
  --> leader.sisu:13:5
   |
13 |     names.sort()
   |     ^^^^^^^^^^^^
   |
12 |     let first = ref names[0]
   |                 ------------ borrowed here by `first`
   |
14 |     return names[0] == first
   |            ----------------- the borrow is still used here
  = note: move this after line 14, where the borrow ends, or work on a copy made with `.copy()`

error[D0021]: the program was not run: the ownership checker found errors (1); `--defer-borrow` runs it under the monitor
```

<details>
<summary>Hint 1</summary>

The note offers to make the change after the last use of the borrow.
Here the old first name is needed after the sort, so that fix cannot
work.

</details>

<details>
<summary>Hint 2</summary>

Keep the first name in a value that does not depend on the list.

</details>

When every test passes, compare your answer with
[`manual/solutions/11-borrowing/leader.sisu`](../solutions/11-borrowing/leader.sisu).

## Exercise 11.4: the winner

Open
[`manual/exercises/11-borrowing/winner.sisu`](../exercises/11-borrowing/winner.sisu):

<!-- file: exercises/11-borrowing/winner.sisu -->
```sisu
##! Exercise 11.4, the winner. Kind: fix. Level: 2.
##!
##! `winner` returns a borrow of the winner's name, without copying it.
##! The file does not compile. Run `sisu test winner.sisu`, read the
##! diagnostic, and fix `winner` so that the test passes. The hints are
##! in chapter 11 of the tutorial, under "Exercise 11.4: the winner".

struct Player:
    name: String
    points: int

## The name of whichever of `a` and `b` has more points; `a` on a tie.
def winner(a: Player, b: Player) -> ref String from a:
    if b.points > a.points:
        return ref b.name
    return ref a.name

test "the winner":
    let aino = Player("Aino", 7)
    let eero = Player("Eero", 9)
    assert winner(aino, eero) == "Eero"
    assert winner(eero, aino) == "Eero"
    assert winner(aino, Player("Roni", 7)) == "Aino"
```

<!-- output: solutions/11-borrowing/winner.console -->
```console
$ sisu test winner.sisu
error[E0408]: this returns a borrow of the parameter `b`, which is not in the `from` clause of `winner`
  --> winner.sisu:15:9
   |
15 |         return ref b.name
   |         ^^^^^^^^^^^^^^^^^
   |
13 | def winner(a: Player, b: Player) -> ref String from a:
   |                       --------- `b` is declared here
  = fix: name `b` in the `from` clause

error[D0021]: the program was not run: the ownership checker found errors (1); `--defer-borrow` runs it under the monitor
```

<details>
<summary>Hint 1</summary>

A `from` clause names every parameter that the result may borrow from.
Read each `return` and the parameter it borrows from.

</details>

When the test passes, compare your answer with
[`manual/solutions/11-borrowing/winner.sisu`](../solutions/11-borrowing/winner.sisu).

## Exercise 11.5: sold out

Open
[`manual/exercises/11-borrowing/stock.sisu`](../exercises/11-borrowing/stock.sisu):

<!-- file: exercises/11-borrowing/stock.sisu -->
```sisu
##! Exercise 11.5, sold out. Kind: fix. Level: 3.
##!
##! A café clears the items it has none left of from its stock. The file
##! does not compile. Run `sisu test stock.sisu`, read the diagnostic,
##! and fix `remove_sold_out` so that every test passes. The hints are in
##! chapter 11 of the tutorial, under "Exercise 11.5: sold out".

## Removes every item that has none left from `stock`, and gives their
## names in the order of the stock.
def remove_sold_out(mut stock: dict[String, int]) -> list[String]:
    var gone: list[String] = []
    for name, count in stock.items():
        if count == 0:
            _ = stock.remove(name)
            gone.push(name.copy())
    return gone

test "two items sold out":
    var stock: dict[String, int] = {"tea": 0, "bun": 4, "jam": 0}
    assert remove_sold_out(mut stock) == ["tea", "jam"]
    assert stock == {"bun": 4}

test "nothing sold out":
    var stock: dict[String, int] = {"tea": 2}
    assert remove_sold_out(mut stock) == []
    assert stock == {"tea": 2}
```

<!-- output: solutions/11-borrowing/stock.console -->
```console
$ sisu test stock.sisu
error[E0405]: `stock` is already borrowed on line 12, so it cannot be borrowed with `mut` until that borrow ends on line 15
  --> stock.sisu:14:17
   |
14 |             _ = stock.remove(name)
   |                 ^^^^^^^^^^^^^^^^^^
   |
12 |     for name, count in stock.items():
   |                        ----- borrowed here by `name`
   |
14 |             _ = stock.remove(name)
   |                 ------------------ the borrow is still used here
  = note: the borrow lasts through the loop, which ends after line 15: change `stock` after the loop, or loop over a copy made with `.copy()`

error[D0021]: the program was not run: the ownership checker found errors (1); `--defer-borrow` runs it under the monitor
```

<details>
<summary>Hint 1</summary>

The loop over `stock.items()` borrows the dict for as long as it runs,
and `remove` needs to change it.

</details>

<details>
<summary>Hint 2</summary>

Do the work in two passes over different things: first find the names,
then change the dict.

</details>

<details>
<summary>Hint 3</summary>

The names you find in the first pass are the function's result as
well.

</details>

When every test passes, compare your answer with
[`manual/solutions/11-borrowing/stock.sisu`](../solutions/11-borrowing/stock.sisu).

## What you learned

- A parameter borrows to read, borrows with `mut` to change in place,
  or takes with `own`. A call to a `mut` parameter says `mut` too.
- `ref` and `ref mut` name a borrow of a value or a part of it. A
  reference never outlives what it borrows.
- Many borrows can read a value at once; a borrow with `mut` excludes
  every other use. A borrow ends at its last use (E0402 to E0405).
- A returned borrow borrows from `self`, from the one borrowed
  parameter, or from the parameters its `from` clause names (E0413),
  and never from the function's own values (E0407).
- `for x in xs` borrows, `for x in mut xs` changes the elements in
  place, and `for x in own xs` takes them; the loop's list cannot
  change inside the loop.
- A closure borrows the variables it uses; an `own` closure keeps
  copies, and can be returned or stored.
- `sisu explain-borrow FILE:LINE` lists the live borrows on a line, and
  `sisu run --defer-borrow` runs a refused program under the monitor.

Chapter 12 is about views: values such as `str` that borrow, structs
that hold borrows, and what keeps them from outliving what they
borrow.
