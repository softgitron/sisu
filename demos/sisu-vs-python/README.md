# Sisu vs Python

Nine bugs that every Python team has shipped, each as a small runnable
Python program next to the same program in Sisu. The Python one runs and
then crashes, or worse, prints a wrong answer and carries on. The Sisu
one never runs: the compiler rejects it and says what is wrong, usually
with the fix. Then [`orders.sisu`](orders.sisu) does the whole job
correctly, and the same arithmetic is raced in both languages.

## Run it

You need `python3`, and Rust's `cargo` once to build the Sisu toolchain.
From the repository root:

```sh
(cd compiler && cargo build --release -p sisu-driver)   # once
./demos/sisu-vs-python/demo.sh            # all rounds in one go
./demos/sisu-vs-python/demo.sh --pause    # wait for Enter between rounds, for presenting
```

Or try one pair by hand:

```sh
cd demos/sisu-vs-python
python3 python/03_truthiness.py
../../compiler/target/release/sisu check sisu/03_truthiness.sisu
```

## The nine rounds

| # | The bug | Python | Sisu |
|---|---|---|---|
| 1 | A typo in a branch that rarely runs ([py](python/01_typo.py), [sisu](sisu/01_typo.sisu)) | Works until the first refund, then `AttributeError` | `E0303`: no field `quantitiy`; did you mean `quantity`? |
| 2 | A lookup that finds nothing ([py](python/02_none.py), [sisu](sisu/02_none.sisu)) | Crashes on the first new customer: `'NoneType' object is not subscriptable` | `E0310`: the value may be missing; give a default with `??` or match it |
| 3 | `0` is falsy ([py](python/03_truthiness.py), [sisu](sisu/03_truthiness.sisu)) | A 0% discount silently becomes 10% | `E0302`: a condition must be a `bool`; Sisu has no truthiness |
| 4 | Dividing money ([py](python/04_division.py), [sisu](sisu/04_division.sisu)) | `333.3333333333333` cents, and a cent lost | `E0301`: `/` divides floats; use `//` for integers |
| 5 | A new status nobody handled ([py](python/05_exhaustive.py), [sisu](sisu/05_exhaustive.sisu)) | The `if`/`elif` falls through and the customer sees `None` | `E0314`: this `match` does not cover `Refunded` |
| 6 | Two names for one list ([py](python/06_aliasing.py), [sisu](sisu/06_aliasing.sisu)) | Appending a totals row to the report doubles the day's revenue | `E0400`: the list was moved; there is never a second name for it |
| 7 | An error the signature does not mention ([py](python/07_exceptions.py), [sisu](sisu/07_exceptions.sisu)) | The batch dies half done, with some cards charged | `E0310`: the result is `int ! ParseError`, so the caller must decide what a bad line means |
| 8 | A number that is really text ([py](python/08_types.py), [sisu](sisu/08_types.sisu)) | `2 * "250"` is `"250250"`: a price of 2502.50 EUR, no error | `E0300`: expected `int`, found `str` |
| 9 | A helper that can read your files ([py](python/09_capabilities.py), [sisu](sisu/09_capabilities.sisu)) | A money formatter lists your home directory and reads your environment | `E0200`: there is no `sys` there; only `main` gets the system, and passes on what it chooses |

Every error in the last column is real compiler output: `demo.sh` runs the
compiler on each file.

## The job done right

[`orders.sisu`](orders.sisu) reads [`orders.csv`](orders.csv), applies
each customer's discount and prints the day's report. It handles every
case above, and the compiler checked that it does:

- the customer lookup returns `ref Customer?`, matched for `None`;
- "no discount agreed" is `None`, not `0`, so `discount ?? 10` cannot
  mix them up;
- `charge` and `label` match `Status` exhaustively, so a new status will
  not compile until it is priced and labelled;
- parsing returns `Order ! OrderError`: the bad line `5,liisa,paid,two,800`
  is reported, and the rest of the batch goes on;
- money stays in whole cents, divided with `//` on purpose;
- `money()` takes no capability, so it cannot read or send anything, and
  `main` hands on only a read-only file system (`sys.fs.read_only()`).

```
order  1  aino   paid, packing it      8.50 EUR
order  2  roni   on its way           10.80 EUR
order  3  mika   paid, packing it      7.50 EUR
order  4  aino   refunded             -4.25 EUR
order  6  roni   paid, packing it      7.17 EUR
total                                 29.72 EUR
5 orders, 6 report rows, 1 problem:
  line 6: bad quantity `two`
```

It has tests next to the code (`sisu test orders.sisu`). It reads much
like Python: indentation, no semicolons or braces, and locals need no
type annotations; only function signatures are written out.

## The race

[`speed.py`](speed.py) and [`speed.sisu`](speed.sisu) are the same loop,
line for line: price 20 million generated orders in whole cents and print
a checksum. On one core of the cloud machine this was written on:

| | Time | Checksum |
|---|---|---|
| Python 3.13 | 5.43 s | 815956408 |
| Sisu, compiled natively (`sisu build --native --release`) | 0.31 s | 815956408 |

The same answer, 17 times faster. Your numbers will differ; `demo.sh`
measures them on your machine.

## To be fair to Python

- **Type checkers help.** With every function annotated, `mypy --strict`
  (1.x, checked while writing this) catches rounds 1, 2, 5 and 8. It
  passes rounds 3, 4, 6 and 7 without a word, and round 9 is not a
  question of types at all. And it is optional: a codebase is only as
  checked as its least annotated module. In Sisu the checks are the
  language; there is no unchecked mode.
- **Python's ecosystem and maturity are in a different league.** Sisu is
  a research language with one implementation, written in this
  repository; nobody should port production code to it yet.
- **Sisu's interpreter is slower than CPython.** `sisu run` is the
  development tool; the speed above is the native compiler's.
