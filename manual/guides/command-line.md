# Parse command-line arguments

This guide reads a program's options and arguments into a struct with
`std.x.cli`, prints a help text, reports arguments that do not fit, and
tests the command line with a transcript. It assumes the packages and
capabilities of [chapter 9](../tutorial/09-word-counter.md) of the
tutorial.

**See also:** the tutorial's [chapter 9](../tutorial/09-word-counter.md)
(a first use of `cli`) and
[chapter 19](../tutorial/19-testing.md#transcripts) (transcripts); the
source of `std.x.cli`, [`compiler/std/cli.sisu`](../../compiler/std/cli.sisu),
which documents what each kind of field reads; G12 for testing a tool
in more depth.

## 1. Describe the command line as a struct

Write a struct with a field for each option, and pass the arguments of
`sys.env.args()` to `cli.parse[Options]`. This program splits a bill
between the names it is given:

<!-- file: examples/guide-command-line/bill.sisu -->
```sisu
# Splits a bill: the options are the fields of a struct.
import cli

## What the command line can say.
struct Options:
    total: f64
    tip: int = 10
    round: bool
    names: list[String]

def main(sys: System) -> ! cli.Error:
    let args = sys.env.args()
    if "--help" in args:
        print("usage: bill [OPTIONS] NAME...")
        print(cli.help_text[Options]())
        return Ok(())
    let options = cli.parse[Options](args) else as e:
        print_err("usage: bill [OPTIONS] NAME...; `bill --help` lists them")
        return Err(e)
    let with_tip = options.total * (1.0 + options.tip.to_f64() / 100.0)
    var share = with_tip / len(options.names).to_f64()
    if options.round:
        share = share.ceil()
    for name in options.names:
        print("{name} pays {share:.2f}")
    return Ok(())
```

The fields decide what the command line can say:

- A field of type `int`, `f64`, `String` or `Path` is an option that
  takes a value, written `--tip 5` or `--tip=5`. A field named
  `min_count` is the option `--min-count`.
- A field with a default, `tip: int = 10`, may be left out; any other
  is required.
- A `bool` field is a flag, `--round`, which is `False` when it is not
  given.
- One field of type `list[...]` collects the arguments that are not
  options, in order; `--` ends the options.

A file's arguments follow its name; in a package they follow `--`, as
in `sisu run -- --total 84.50 Aino`:

<!-- output: examples/guide-command-line/run.console -->
```console
$ sisu run bill.sisu --total 84.50 Aino Eero Roni
Aino pays 30.98
Eero pays 30.98
Roni pays 30.98
$ sisu run bill.sisu --total=84.50 --tip 0 --round Aino Eero Roni
Aino pays 29.00
Eero pays 29.00
Roni pays 29.00
```

## 2. Print a help text

`cli.help_text[Options]()` gives a line for each option, with the kind
of value it takes and its default, and a line for the positional
arguments. `main` tests for `--help` before it parses, since a help
request without the required `--total` would otherwise be an error:

<!-- output: examples/guide-command-line/help.console -->
```console
$ sisu run bill.sisu --help
usage: bill [OPTIONS] NAME...
  --total <f64>
  --tip <int>  (default: 10)
  --round
  names...  (String)
```

## 3. Report arguments that do not fit

`cli.parse` returns a `cli.Error` for arguments that do not fit the
struct: `UnknownOption`, `MissingValue` (an option at the end, with no
value), `InvalidValue`, `MissingArgument` (a required option left out)
and `UnexpectedArgument` (a positional argument with no `list` field to
collect it). The program prints a usage line on standard error and
returns the error, which ends it with status 1:

<!-- output: examples/guide-command-line/errors.console -->
```console
$ sisu run bill.sisu --total lots Aino
usage: bill [OPTIONS] NAME...; `bill --help` lists them
error: InvalidValue(name="--total", value="lots")
$ sisu run bill.sisu --tip 5 Aino
usage: bill [OPTIONS] NAME...; `bill --help` lists them
error: MissingArgument(name="--total")
$ sisu run bill.sisu --totl 84.50 Aino
usage: bill [OPTIONS] NAME...; `bill --help` lists them
error: UnknownOption(name="--totl")
```

`main` returns the error from its `else` block, rather than passing it
on with `?`, so that the message has no line of the program's source
under it: `?` would add one, which means something to the program's
author and nothing to its user.

## 4. Test the command line with a transcript

A transcript, `tests/NAME.transcript` beside the program, runs `main`
with the words of each `$` line as its arguments and compares what it
prints with the lines that follow. A run that fails ends with `(exit
N)`. This is the program's:

<!-- file: examples/guide-command-line/bill.files/tests/bill.transcript -->
```text
$ bill --total 84.50 Aino Eero Roni
Aino pays 30.98
Eero pays 30.98
Roni pays 30.98
$ bill --total 84.50 --tip 0 --round Aino Eero Roni
Aino pays 29.00
Eero pays 29.00
Roni pays 29.00
$ bill --help
usage: bill [OPTIONS] NAME...
  --total <f64>
  --tip <int>  (default: 10)
  --round
  names...  (String)
$ bill --total lots Aino
usage: bill [OPTIONS] NAME...; `bill --help` lists them
error: InvalidValue(name="--total", value="lots") (exit 1)
```

`sisu test` runs it:

<!-- output: examples/guide-command-line/bill.console -->
```console
$ sisu test bill.sisu
transcript "tests/bill.transcript" ... ok
1 passed, 0 failed, 0 blocked
```

You need not write the expected lines by hand. Write the `$` lines,
run `sisu test --update`, and read what it stored before you keep it;
G12 shows the review.

**Status.** In stage 0, as `std.x.cli`, which reads the struct by
reflection where [11](../../proposal/11-stdlib.md) plans a derive. The
help text leaves out the fields' `##` comments, so it says what kind of
value an option takes and not what it is for.
