# Reproduce a failure

This guide makes a failure happen again, on demand and on any machine:
a failing test from its replay token, and a failing run from a log of
everything it took from the world. It then fixes the bug and checks the
fix. It assumes tests and capabilities, from chapters 18 and 19 of the
tutorial.

**See also:** the tutorial's
[chapter 19](../tutorial/19-testing.md#replaying-a-failure) (replay
tokens) and
[chapter 22](../tutorial/22-web-service.md#trying-it-and-replaying-the-run)
(a recorded run); [R16](../reference/sisu-command.md#recording-a-run---record-and---redact)
for `--record`, `--redact` and `sisu replay`; the proposal's
[17, "Simulation and replay"](../../proposal/17-testing.md#simulation-and-replay).

## 1. Replay a failing test by its token

Every failing test prints a `replay:` line. Its token holds what
decided the run: the code's content hash, the toolchain's version, the
test, the scheduler's seed, the seed of a property's cases and its
shrunk case, and the simulator's fault rate. This program totals the
orders in a file, and its second test fails:

<!-- file: examples/guide-replay/orders.sisu -->
```sisu
# Totals the orders in `orders.txt`, one `ITEM COUNT` on each line.

## The total of each item over the lines of `text`.
def totals(text: str) -> dict[String, int]:
    var out: dict[String, int] = {}
    for line in text.lines():
        let parts: list[str] = line.split(" ").collect()
        let n = parts[1].parse_int() ?? 1
        out.entry(parts[0].to_string()).or_insert(0) += n
    return out

def main(sys: System) -> ! IoError:
    let text = sys.fs.read_to_string("orders.txt")?
    for item, n in totals(text).items():
        print("{item}: {n}")
    return Ok(())

test "two orders of coffee":
    assert totals("coffee 2\ntea 1\ncoffee 3") == {"coffee": 5, "tea": 1}

test "an order without a count is one":
    assert totals("coffee 2\nbun") == {"coffee": 2, "bun": 1}
```

<!-- output: examples/guide-replay/token.console -->
```console
$ sisu test orders.sisu
test "two orders of coffee" ... ok
test "an order without a count is one" ... FAILED

test "an order without a count is one" ... FAILED
panic: index out of bounds: the length is 1 but the index is 1
  at totals (orders.sisu:8:17)
  at test "an order without a count is one" (orders.sisu:22:12)
replay: sisu test --replay 1-f7555e679cbec94d-0.0.1-t1-s0-r0 orders.sisu

1 passed, 1 failed, 0 blocked
$ sisu test --replay 1-f7555e679cbec94d-0.0.1-t1-s0-r0 orders.sisu
test "an order without a count is one" ... FAILED

test "an order without a count is one" ... FAILED
panic: index out of bounds: the length is 1 but the index is 1
  at totals (orders.sisu:8:17)
  at test "an order without a count is one" (orders.sisu:22:12)
replay: sisu test --replay 1-f7555e679cbec94d-0.0.1-t1-s0-r0 orders.sisu

0 passed, 1 failed, 0 blocked
```

The second command runs the failing test alone, the same way, so it
is the command to run while you look for the bug. Since every source
of chance inside a test is a fake or the seeded scheduler, the token
fails the same way on any machine that has the same code; put it in
the bug report.

## 2. Record the run that fails, and replay it

A program that fails outside the tests has no token, so record it:
`sisu run --record LOG` runs it and writes its capability log, which
holds the program, its standard input, and every result that a
capability gave it, in order. `sisu replay LOG` runs the logged program
again with the log in place of the world. With an `orders.txt` that
holds the line `bun`, without a count, the run panics, and so does its
replay:

<!-- output: examples/guide-replay/orders.console -->
```console
$ sisu run --record bug.log orders.sisu
panic: index out of bounds: the length is 1 but the index is 1
  at totals (orders.sisu:8:17)
  at main (orders.sisu:14:20)
$ sisu replay bug.log
panic: index out of bounds: the length is 1 but the index is 1
  at totals (orders.sisu:8:17)
  at main (orders.sisu:14:20)
```

- The replay gives each call the answer it got the first time, so it
  needs neither `orders.txt` nor the source, and prints and exits as
  the recorded run did. Send the log, not a description of the input:
  whoever replays it sees the failure, whatever their files hold.
- A replay that goes another way than the log is D0037, and one that
  exits with another status D0040.
- `--redact fs,env`, or any of `fs`, `net`, `clock`, `rand`, `env`,
  `proc` and `stdin`, leaves the results of those capabilities out of
  the log, for a log that would hold secrets; its replay stops at the
  first call whose result it lacks.
- A failure of the compiled program records the same way: `sisu run
  --native --record LOG` writes the log the interpreter's run writes,
  and `sisu replay --native LOG` replays a log natively, so a log
  from either runtime reproduces the failure in the other
  ([R16](../reference/sisu-command.md#recording-a-run---record-and---redact)).

## 3. Fix the bug, and check the fix

Fix the code, and run the tests again. A token is for the code it came
from, so the old token is refused once the file changes, and the test
that failed now passes:

<!-- output: examples/guide-replay/orders_fixed.console -->
```console
$ sisu test --replay 1-f7555e679cbec94d-0.0.1-t1-s0-r0 orders_fixed.sisu
error[D0033]: the token is for other code (content hash f7555e679cbec94d); `orders_fixed.sisu` has content hash 968a397df8661a6f
$ sisu test orders_fixed.sisu
test "two orders of coffee" ... ok
test "an order without a count is one" ... ok
2 passed, 0 failed, 0 blocked
$ sisu run orders_fixed.sisu
coffee: 5
tea: 1
bun: 1
```

Keep the failing case as a test, as the second test of step 1 does
with the line from the log, so that the bug cannot come back
unnoticed. A recorded log replays the program it holds, not your fix:
check the fix with a test, or by running it on the same input.

**Status.** Replay tokens, `sisu run --record` and `sisu replay` are in
stage 0, a prototype of the experiment (Q-49, D-070), and what they
print may change.
