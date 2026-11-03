# Change a running program

This guide changes a program while it runs, without stopping it: it
marks where new code may take over, runs the program under `sisu run
--watch`, and shows what happens to a save that changes a signature or
does not check. It assumes tasks and loops, from chapters 4 and 20 of
the tutorial.

**See also:** the tutorial's
[chapter 22](../tutorial/22-web-service.md#changing-a-handler-while-the-server-runs)
(a server whose handler changes); [R16](../reference/sisu-command.md)
for `sisu run` and its options; the source of `live`,
[`compiler/std/live.sisu`](../../compiler/std/live.sisu); the proposal's
[10, Tooling](../../proposal/10-tooling.md), "Debugging and profiling",
and D-053 and D-068.

## 1. Mark where new code may take over

Under `--watch`, each save of the program's files that checks becomes
an epoch: the functions whose bodies changed, swapped in together. A
task takes the newest epoch when it starts and at each
`live.boundary()`, and a call that is running keeps the body it started
with. A loop that runs for the life of the program therefore calls
`live.boundary()` at the top of each turn, so that a turn never mixes
old code with new:

<!-- file: examples/guide-live/loop.sisu -->
```sisu
# A loop that takes the newest code at the top of each turn. After its
# first turn it saves its own next version, as an editor would.

## What each turn prints.
def greeting(turn: int) -> String:
    return "turn {turn}: the sauna is warming"

def main(sys: System) -> ! IoError:
    for turn in range(3):
        live.boundary()
        print("{greeting(turn)}, epoch {live.epoch()}")
        if turn == 0:
            let next = sys.fs.read_to_string("v2.sisu")?
            sys.fs.write_text("loop.sisu", next)?
    return Ok(())
```

`live.epoch()` is the epoch that the task has taken, 0 until the first
save. Without `--watch`, `live.boundary()` does nothing. `Ticker.wait`
of `std.time` calls it, so a loop that waits on a ticker needs no call
of its own, and a server whose connections are tasks answers each new
connection with the newest code, as chapter 22 shows.

## 2. Run it under --watch

`sisu run --watch FILE` runs the program and watches its files. In
practice you save the file in your editor; this example saves its own
next version, `v2.sisu`, which changes `greeting` alone, so that the
manual can show what happens:

<!-- output: examples/guide-live/loop.console -->
```console
$ sisu run --watch loop.sisu
turn 0: the sauna is warming, epoch 0
watch: epoch 1 replaces `greeting`
turn 1: the sauna is hot, epoch 1
turn 2: the sauna is hot, epoch 1
```

The `watch:` line, on standard error, names the epoch and what it
replaced. The first turn ran in epoch 0; the boundary at the top of the
second took epoch 1, so the second and third turns print the new
greeting.

## 3. Know what starts the program again

A change that the running program cannot take as new bodies starts it
again from `main`: a function's signature (its parameters, result,
conventions or purity), a type's layout, or an `impl`. Values that the
program held are dropped without their `Drop`, since their code may be
gone. This program's next version gives `greeting` a second parameter,
and removes `v2.sisu` so that the new run does not save it again:

<!-- file: examples/guide-live/restart.sisu -->
```sisu
# A save that changes a signature restarts the program from `main`.

## What each turn prints.
def greeting(turn: int) -> String:
    return "turn {turn}: the sauna is warming"

def main(sys: System) -> ! IoError:
    for turn in range(3):
        live.boundary()
        print("{greeting(turn)}, epoch {live.epoch()}")
        if turn == 0 and sys.fs.exists("v2.sisu"):
            let next = sys.fs.read_to_string("v2.sisu")?
            sys.fs.remove("v2.sisu")?
            sys.fs.write_text("restart.sisu", next)?
    return Ok(())
```

<!-- output: examples/guide-live/restart.console -->
```console
$ sisu run --watch restart.sisu
turn 0: the sauna is warming, epoch 0
watch: the signature of `greeting` changed, so the program starts again
turn 0: the sauna is hot, epoch 0
turn 1: the sauna is hot, epoch 0
turn 2: the sauna is hot, epoch 0
```

Keep a long-running program's state where a restart does not lose it,
such as a file, if a change of signature must not cost it.

## 4. Save code that does not check

A save that does not check publishes nothing: `--watch` prints its
diagnostics, and the program goes on in the epoch it was in. Here the
next version adds an `int` to a `str`:

<!-- output: examples/guide-live/broken.console -->
```console
$ sisu run --watch broken.sisu
turn 0: the sauna is warming, epoch 0
watch: the save of `broken.sisu` does not check, so the program keeps running epoch 0:
error[E0310]: `+` cannot be applied to `int` and `str`
  --> broken.sisu:5:12
  |
5 |     return turn + ": the sauna is hot"
  |            ^^^^^^^^^^^^^^^^^^^^^^^^^^^
  = fix: write the value into the text: `"{turn}: the sauna is hot"`
turn 1: the sauna is warming, epoch 0
turn 2: the sauna is warming, epoch 0
```

Fix the file and save it again; the next save that checks is the next
epoch.

## 5. Record a run that changes

`--record` under `--watch` writes each save into the capability log at
the point where the program saw it, so `sisu replay` applies the same
saves at the same points and publishes the same epochs:

<!-- output: examples/guide-live/recorded.console -->
```console
$ sisu run --watch --record watch.log loop.sisu
turn 0: the sauna is warming, epoch 0
watch: epoch 1 replaces `greeting`
turn 1: the sauna is hot, epoch 1
turn 2: the sauna is hot, epoch 1
$ sisu replay watch.log
turn 0: the sauna is warming, epoch 0
watch: epoch 1 replaces `greeting`
turn 1: the sauna is hot, epoch 1
turn 2: the sauna is hot, epoch 1
```

A run that went wrong after a change can thus be replayed with its
change, on another machine and without the editor.

**Status.** In stage 0, in the interpreter (D-053, D-068): `--watch`
does not combine with `--native` (D0109), and an executable built by
`sisu build --native` runs the code it was built with.
