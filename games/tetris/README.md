# Tetris in Sisu

A terminal Tetris written in Sisu ([`tetris.sisu`](tetris.sisu)): the
seven pieces from a shuffled bag, rotation with wall kicks, a shadow of
where the piece will land, a preview of the next piece, line clears,
levels that speed the fall up, scoring, pause and restart.

## Requirements

- Linux or macOS with `bash` 4 or newer (on macOS, `brew install bash`;
  the system bash 3.2 cannot wait for fractions of a second) and a
  terminal that understands ANSI colours
  (any modern terminal emulator; at least 60 x 25 characters).
- Rust's `cargo`, to build the Sisu toolchain in [`compiler/`](../../compiler)
  once.

## How to run

From the repository root:

```sh
# 1. Build the Sisu toolchain (once, about two minutes)
cd compiler
cargo build --release -p sisu-driver
cd ..

# 2. Play
./games/tetris/tetris.sh
```

`tetris.sh` builds the toolchain itself if it is not there yet, so step 1
is optional. By default the game runs in Sisu's interpreter. To compile it
to a native executable first with Sisu's Cranelift backend (it compiles in
under a second and runs faster), pass `--native`:

```sh
./games/tetris/tetris.sh --native
```

To use a `sisu` binary from somewhere else, set `SISU`:

```sh
SISU=/path/to/sisu ./games/tetris/tetris.sh
```

## Controls

| Key | Action |
|---|---|
| `←` `→` or `a` `d` (or `h` `l`) | move left / right |
| `↑`, `w` or `x` (or `k`) | rotate clockwise |
| `z` | rotate counter-clockwise |
| `↓` or `s` (or `j`) | soft drop (1 point per row) |
| `space` | hard drop (2 points per row) |
| `p` | pause / resume |
| `r` | restart |
| `q` | quit |

Clearing 1, 2, 3 or 4 lines at once scores 100, 300, 500 or 800 points
times the level. The level goes up every 10 lines, and each level makes
pieces fall faster.

## Running the tests

The game logic (rotation, collisions, line clears, scoring, key decoding)
has tests written in Sisu next to the code:

```sh
compiler/target/release/sisu test games/tetris/tetris.sisu
```

## How it works

Sisu's standard input capability (`sys.stdin.read_line()`) gives the
program whole lines, and the language has no terminal raw mode. So the
work is split in two:

- [`tetris.sh`](tetris.sh) switches the terminal into character mode
  (`stty -icanon -echo`), reads one key at a time and writes its byte code
  as a line to the game's standard input. When no key arrives within
  30 ms it writes a `t` heartbeat instead, so the game can let the piece
  fall. On exit (including Ctrl-C) it restores the terminal.
- [`tetris.sisu`](tetris.sisu) is the whole game. It reads those lines,
  decodes arrow-key escape sequences (`ESC [ A`..`D`), times gravity with
  the clock capability (`sys.clock`), shuffles pieces with the randomness
  capability (`sys.rand`), and redraws the screen with ANSI escape codes
  through `print`.

You can drive the game without a terminal too, which shows the protocol:

```sh
printf '32\n32\n113\n' | compiler/target/release/sisu run games/tetris/tetris.sisu
```

(two hard drops, then `q`).

## How it looks

```
  S I S U   T E T R I S
  <! . . . .     . . . .!>
  <! . . . .     . . . .!>  NEXT
  <! . . . . . . . . . .!>    ██████
  <! . . . . . . . . . .!>      ██
  <! . . . . . . . . . .!>
  <! . . . . . . . . . .!>
  <! . . . . . . . . . .!>  SCORE  60
  <! . . . . . . . . . .!>  LINES  0
  <! . . . . . . . . . .!>  LEVEL  1
  ...
  <! . .   . . . . . . .!>
  <! . .   .[][]     . .!>
  <! . .    [][] .     .!>
  <!====================!>
    \/\/\/\/\/\/\/\/\/\/
```

Pieces are drawn as coloured blocks; `[]` is the shadow showing where the
falling piece will land.
