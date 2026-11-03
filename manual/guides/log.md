# Log and trace

This guide writes log records at their levels, keeps the records of
one piece of work together across its tasks, and traces every call
that a part of the program makes through a capability. It assumes
tasks and capabilities, from chapters 18 and 20 of the tutorial.

**See also:** the tutorial's
[chapter 20](../tutorial/20-concurrency.md#logs-from-tasks) (logs from
tasks), [chapter 21](../tutorial/21-standard-library.md#logging)
(`std.log`) and [chapter 18](../tutorial/18-capabilities.md)
(interceptors); the source of `std.log`,
[`compiler/std/log.sisu`](../../compiler/std/log.sisu); the proposal's
[06, "Interceptors and trace context"](../../proposal/06-concurrency-and-capabilities.md#interceptors-and-trace-context)
and D-075.

## 1. Write records at their levels

`log.error`, `log.warn`, `log.info` and `log.debug` each write a record,
`level: message`, to standard error. `log` is ambient, as `print` is: it
needs no capability and no import. This program logs at every level
while it prints its results on standard output:

<!-- file: examples/guide-log/levels.sisu -->
```sisu
# A log record has a level, and goes to standard error.

## The stove's temperature after `minutes`, from 20 degrees.
def heat(minutes: int) -> int:
    log.debug("heating for {minutes} minutes")
    let celsius = 20 + 3 * minutes
    if celsius > 100:
        log.warn("{celsius} degrees is too hot")
    return celsius

def main():
    log.info("the stove is on")
    print(heat(20))
    print(heat(30))
    log.error("the stove switched itself off")
```

<!-- output: examples/guide-log/levels.console -->
```console
$ sisu run levels.sisu
info: the stove is on
debug: heating for 20 minutes
80
debug: heating for 30 minutes
warn: 110 degrees is too hot
110
error: the stove switched itself off
```

Keep standard output for the program's results and the log for what a
person investigating it needs, so that the one can be piped to another
program and the other read. Use `error` for what failed, `warn` for
what may fail, `info` for what happened, and `debug` for the detail of
how.

## 2. Keep the records of one piece of work together

Inside a task, each record names the task by its path from the root:
`[task 2]` is the second task that the root's scope spawned, and
`[task 2.1]` the first that task 2 spawned. A child inherits its
parent's path, so the records of one request carry one prefix with no
parameter to pass around:

<!-- file: examples/guide-log/requests.sisu -->
```sisu
# The records of a task name it by its path from the root task, so the
# records of one piece of work stay together.

## Books a turn for `name`, checking the hour and the towels side by side.
def book(name: str, hour: int) -> ! TaskError:
    log.info("{name} asks for {hour}:00")
    return task.scope do s:
        s.spawn(() -> log.info("{hour}:00 is free"))
        s.spawn(() -> log.info("a towel for {name}"))
        return Ok(())

def main() -> ! TaskError:
    return task.scope do s:
        s.spawn(() -> book("Aino", 18))
        s.spawn(() -> book("Eero", 19))
        return Ok(())
```

<!-- output: examples/guide-log/requests.console -->
```console
$ sisu run requests.sisu
info [task 2]: Eero asks for 19:00
info [task 2.1]: 19:00 is free
info [task 2.2]: a towel for Eero
info [task 1]: Aino asks for 18:00
info [task 1.2]: a towel for Aino
info [task 1.1]: 18:00 is free
```

The tasks ran in the scheduler's order, so the records of the two
bookings are not in the order of the code; their paths sort them out.

## 3. Trace every call through a capability

To see every file a part of the program reads, wrap its file system in
`main` with an interceptor: a type that implements `cap.Interceptor`,
whose `before` is shown each call and whose `after` is shown its
outcome. `cap.Wrap(inner=sys.fs, hook=Tracer())` is an `Fs` that tells
the interceptor of each call and then makes it:

<!-- file: examples/guide-log/traced.sisu -->
```sisu
# An interceptor, put around the file system in `main`, logs each call.

## Logs each call it is shown, and what the call gave.
struct Tracer:
    pass

impl cap.Interceptor for Tracer:
    def before(self, call: cap.Call):
        log.debug("{call.capability}.{call.method}({call.args})")

    def after(self, call: cap.Call, outcome: str):
        log.debug("{call.method} gave {outcome}")

## The stove's setting from `stove.txt`, or 80 without one.
def setting(fs: Fs) -> int:
    let text = fs.read_to_string("stove.txt") else:
        return 80
    return text.trim().parse_int() ?? 80

def main(sys: System):
    let fs: Fs = cap.Wrap(inner=sys.fs, hook=Tracer())
    print(setting(fs))
    print(setting(fs.restrict_to("old")))
```

With a `stove.txt` that holds `90`, the program prints:

<!-- output: examples/guide-log/traced.console -->
```console
$ sisu run traced.sisu
debug: Fs.read_to_string("stove.txt")
debug: read_to_string gave Ok("90\n")
90
debug: Fs.read_to_string("old/stove.txt")
debug: read_to_string gave Err(NotFound(path="old/stove.txt"))
80
```

- The code that is traced takes an `Fs` and does not know that it is
  watched; the tracing is one line of `main`, where a reader sees it.
- An attenuation made from the wrapped file system, here
  `restrict_to("old")`, goes through the interceptor too, with the path
  as the file system sees it.
- The same works for `Net`, `Clock` and the other capabilities, and for
  the program's own traits.

**Status.** In stage 0, with differences from
[11](../../proposal/11-stdlib.md), which makes `std.log` a structured
facade with levels and context, and
[06](../../proposal/06-concurrency-and-capabilities.md#interceptors-and-trace-context):
a record is a message without fields, every level is written (there is
no setting that hides `debug`), the trace context is the path of spawn
indices rather than trace and span identifiers, and `trace.wrap` with a
`Tracer` is designed, so an interceptor of your own does its work.
