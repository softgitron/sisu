# Find and fix a borrow error

This guide takes a program that the ownership checker refuses, finds
the borrow that is in the way, finds out whether the conflict is real,
and fixes it. It assumes the borrowing of
[chapter 11](../tutorial/11-borrowing.md) of the tutorial.

**See also:** [R7, Ownership and borrowing](../reference/ownership.md),
for the rules; [R17, Diagnostics](../reference/diagnostics.md#ownership),
for each E04xx code with a wrong program and its fix;
[R16](../reference/sisu-command.md#sisu-explain-borrow) for `sisu
explain-borrow` and
[`--defer-borrow`](../reference/sisu-command.md#running-a-rejected-program---defer-borrow).

## 1. Read the diagnostic

This program removes the bookings that have ended, from inside a loop
over the bookings, and does not compile:

<!-- file: examples/guide-borrow-errors/ended.sisu -->
```sisu
# Removes the bookings that have ended, while looping over them.
def remove_ended(mut bookings: dict[str, int], now: int):
    for name, hour in bookings.items():
        if hour < now:
            _ = bookings.remove(name)

def main():
    var bookings = {"Aino": 18, "Eero": 16, "Roni": 20}
    remove_ended(mut bookings, 17)
    print(bookings)
```

<!-- output: examples/guide-borrow-errors/ended.console -->
```console
$ sisu run ended.sisu
error[E0405]: `bookings` is already borrowed on line 3, so it cannot be borrowed with `mut` until that borrow ends on line 5
  --> ended.sisu:5:17
  |
5 |             _ = bookings.remove(name)
  |                 ^^^^^^^^^^^^^^^^^^^^^
  |
3 |     for name, hour in bookings.items():
  |                       -------- borrowed here
  |     ---------------------------------- the borrow is still used here
  = note: move this after line 5, where the borrow ends, or work on a copy made with `.copy()`

error[D0021]: the program was not run: the ownership checker found errors (1); `--defer-borrow` runs it under the monitor
```

Read the parts in this order:

- The code, E0405 (`borrow.mut-while-borrowed`): something needs a
  `mut` borrow while another borrow of the same value is live.
- The `^` line: what needs the `mut` borrow, the call of `remove`.
- The `-` labels: where the other borrow starts ("borrowed here",
  under `bookings.items()`) and where it is still used ("the borrow is
  still used here", the `for` line, whose every step reads the dict).
- The note: the two usual fixes, finishing with the borrow before the
  change, or changing a copy. In a loop, "after line 5", the last line
  of the loop's body, means after the loop.

These are the codes you meet most, and what fixes each most often:

| Code | What happened | The usual fix |
|---|---|---|
| [E0400](../reference/diagnostics.md#e0400) | a value was used after it moved | `.copy()` before the move, or a borrowed parameter in place of an `own` one |
| [E0402](../reference/diagnostics.md#e0402), [E0405](../reference/diagnostics.md#e0405) | a change, or a `mut` borrow, while a borrow is live | the change after the borrow's last use |
| E0403 | a use while a `mut` borrow is live | the use after the `mut` borrow's last use |
| [E0406](../reference/diagnostics.md#e0406), [E0407](../reference/diagnostics.md#e0407) | a borrow outlives what it borrows | an owned value, such as a `String`, in place of the borrow |

## 2. List the borrows on the line

`sisu explain-borrow FILE:LINE` lists every borrow live on a line,
with what holds it and where it is last used:

<!-- output: examples/guide-borrow-errors/explain.console -->
```console
$ sisu explain-borrow ended.sisu:5
borrows live on line 5 of ended.sisu, in `remove_ended`:
  a shared borrow of `bookings` from 3:23, held by `name`, used until 5:33
  a `mut` borrow of `bookings` from 5:17, held by a temporary, used until 5:33
```

The loop variable `name` holds a shared borrow of `bookings`, since a
key in a dict is part of the dict, and `remove` needs a `mut` borrow on
the same line. Whatever holds a borrow is what a fix changes: here the
loop and its `name`.

## 3. Find out whether the conflict is real

`sisu run --defer-borrow` runs the refused program under the ownership
monitor, which watches every borrow as the program runs. It stops at
the first real conflict, with status 134, and prints an account of it
and the blocks of the function's SIR that the conflict passes through:
where the loan was made, where it ended and where it was used, with
`...` for the blocks left out:

<details>
<summary>What <code>sisu run --defer-borrow</code> prints</summary>

<!-- output: examples/guide-borrow-errors/counterexample.console -->
```console
$ sisu run --defer-borrow ended.sisu
counterexample run: the ownership checker rejected this program; its errors are deferred to the monitor
ownership violation: a reference was used after its loan had ended
  the loan: a shared borrow of `bookings` in `remove_ended`, created at ended.sisu:3:23 by `_4 = ref _1.*`
  ended by: a `mut` borrow of `bookings` at ended.sisu:5:17 (`_11 = call intrinsic dict.remove(copy _12, copy _13)`)
  used at: ended.sisu:3:5 (`_6 = call intrinsic Iter.next(copy _5)`)
  value: `bookings` = {"Aino": 18, "Roni": 20}
  loans of `bookings` not ended yet: 0 shared, 4 with `mut`
  at remove_ended (ended.sisu:3:5)
  at main (ended.sisu:9:5)
SIR of `remove_ended`, its blocks that the conflict passes through:
fn remove_ended(2) source 0 @2:5-2:17 {
  bb0:
    _4 = ref _1.* @3:23-3:31
    _3 = call intrinsic dict.items(copy _4) @3:23-3:39
    goto bb2
  ...
  bb2 (unwind bb1):
    _5 = ref mut _3 @3:5-3:39
    _6 = call intrinsic Iter.next(copy _5) @3:5-3:39
    _7 = discriminant _6 @3:5-3:39
    switch copy _7 [0: bb3, else: bb5] @3:5-3:39
  ...
  bb4 (unwind bb1):
    _12 = ref mut _1.* @5:17-5:25
    _13 = copy _8.* @5:33-5:37
    _11 = call intrinsic dict.remove(copy _12, copy _13) @5:17-5:38
    goto bb2
  ...
}
outcome: conflict reached; the run stopped at a concrete ownership conflict
```

</details>

The run is a counterexample. Its account names the loan, the shared
borrow that the loop made at 3:23; what ended it, the `mut` borrow of
`remove` at 5:17; and where the loan was used after it ended, at 3:5,
where the loop takes its next step through a dict that has changed. The
line `value:` shows the dict at that moment, with Eero's booking gone.
Python stops the same loop with `RuntimeError: dictionary changed size
during iteration`; Sisu refuses it before it runs.

When the run ends with `outcome: no conflict` instead, the checker has
refused a program that ran safely this time: a candidate false
positive, as chapter 11's section "Asking about borrows" shows. The
program still does not compile, so it needs a fix too, though often a
smaller one, such as moving a line.

## 4. End the borrow before the change

Here the loop must not change what it walks. Find the bookings first,
and remove them once the loop has ended. This version prints the
bookings that are left:

<!-- file: examples/guide-borrow-errors/ended_fixed.sisu -->
```sisu
# Finds the bookings that have ended first, then removes them.
def remove_ended(mut bookings: dict[str, int], now: int):
    let ended = [name for name, hour in bookings.items() if hour < now]
    for name in ended:
        _ = bookings.remove(name)

def main():
    var bookings = {"Aino": 18, "Eero": 16, "Roni": 20}
    remove_ended(mut bookings, 17)
    print(bookings)
```

<!-- output: examples/guide-borrow-errors/ended_fixed.out -->
```text
{"Aino": 18, "Roni": 20}
```

The comprehension only reads `bookings`, and its borrow ends with it,
so `remove` gets its `mut` borrow. With `String` keys, collect copies,
`name.copy()`, since a key that is not copied is still part of the dict.
The other fixes, by what they cost:

- Move the change after the borrow's last use. It costs nothing, when
  the order of the work allows it.
- Copy what the borrow reads, with `.copy()`. It costs a copy, and the
  copy does not see later changes.
- Build a new collection, as `{name: hour for name, hour in
  bookings.items() if hour >= now}` does, and assign it in place of the
  old one. It costs a second collection while both exist.
- Give the data an owner of its own, such as a `String` in place of a
  `str`, when a borrow outlives what it borrows (E0406, E0407).

**Status.** Everything in this guide is in stage 0. `sisu
explain-borrow` and `--defer-borrow` are tools of the experiment
([18](../../proposal/18-next-steps.md), Phase 3), and what they print
may change.
