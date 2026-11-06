# Set up your editor

This guide connects an editor to Sisu's language server, `sisu lsp`,
and shows what it gives: diagnostics with their fixes, types and error
sets on hover and inline, the borrows of a line, and a script's values
beside its lines. It assumes that `sisu` is installed and on your
`PATH`, as chapter 1 of the tutorial shows.

**See also:** the tutorial's [chapter 1](../tutorial/01-hello.md)
(diagnostics and worksheets); [R16](../reference/sisu-command.md) for
`sisu lsp`, `sisu explain-borrow` and `sisu ws`; the proposal's
[10, Tooling](../../proposal/10-tooling.md); the compiler's
[architecture notes](../../compiler/docs/architecture.md), "The
language server", for every request it answers.

## 1. Point the editor at sisu lsp

`sisu lsp` is a language server that speaks the Language Server
Protocol over its standard input and output, and takes no options. An
editor with a client of the protocol starts it for each file whose name
ends in `.sisu`, with the directory that holds `sisu.toml` as the root.
In Neovim 0.11 and later, put this in your configuration:

<!-- unchecked: editor configuration, which no check of the manual runs -->
```lua
vim.filetype.add({ extension = { sisu = "sisu" } })
vim.lsp.config("sisu", {
  cmd = { "sisu", "lsp" },
  filetypes = { "sisu" },
  root_markers = { "sisu.toml" },
})
vim.lsp.enable("sisu")
```

In Helix, add this to `languages.toml`:

<!-- unchecked: editor configuration, which no check of the manual runs -->
```toml
[language-server.sisu]
command = "sisu"
args = ["lsp"]

[[language]]
name = "sisu"
scope = "source.sisu"
file-types = ["sisu"]
roots = ["sisu.toml"]
comment-token = "#"
indent = { tab-width = 4, unit = "    " }
language-servers = ["sisu"]
```

Another editor needs the same three facts: the command `sisu lsp`, the
extension `.sisu`, and `sisu.toml` as the mark of a package's root.
Indent with four spaces: Sisu has no tabs.

## 2. Use what the server gives

The server answers from the same queries as `sisu check`, so the editor
and the command line never disagree:

- **Diagnostics** on every change, each with its code and identifier,
  as `sisu check` prints them, and each fix as a code action.
- **Hover**: an expression's type; a function's signature with the error
  set it infers, the capabilities it takes and its contracts; a type's
  fields; the documentation.
- **Inlay hints**: the type of each `let` that writes none, the error
  set of each function declared `-> T !`, and `mut` or `own` before an
  argument passed to such a parameter.
- **Completion** after a `.` and of the names in scope, **go to
  definition** (into the standard library too), **rename** and
  **references**, and **formatting** as `sisu fmt` writes.

Each of these, and the document's symbols and selection ranges, comes
from stage 1, the front end that `sisu check` runs, through the same
server of it that every command asks
([R16](../reference/sisu-command.md#the-server-of-stage-1)), so a hover
answers in about 9 ms. A field's references and its rename include the
keyword of each construction, the `turn` of `Sauna(turn=1)`, and the
field's name in each struct pattern, the `turn` of `case
Sauna(turn=t)`. A file with a syntax error still gets what its
tree allows: the symbols of its whole items, and the members after a
`.` that is not finished yet. Build stage 1 once with any command, such
as `sisu check`, before you open the editor: the server does not build
it, and answers from the Rust front end until it is built.

## 3. Ask about the borrows of a line

The command `sisu.explainBorrow`, run on a line, lists the borrows live
there, each with what holds it and where it is last used. It is what
`sisu explain-borrow FILE:LINE` prints, for a program that checks:

<!-- file: examples/guide-editor/borrow.sisu -->
```sisu
# The borrows that the editor's command shows for a line.
def main():
    var turns: list[String] = ["Aino 18", "Eero 19"]
    let first = ref turns[0]
    print("first: {first}")
    turns.push("Roni 20")
    print("{len(turns)} turns")
```

<!-- output: examples/guide-editor/borrow.console -->
```console
$ sisu explain-borrow borrow.sisu:5
borrows live on line 5 of borrow.sisu, in `main`:
  a shared borrow of `turns[...]` from 4:17, held by `first`, used until 5:20
$ sisu explain-borrow borrow.sisu:6
borrows live on line 6 of borrow.sisu, in `main`:
  a `mut` borrow of `turns` from 6:5, held by a temporary, used until 6:16
```

Stage 1 answers the command, from its twenty-second round, and the
server asks it the same question for the editor. Bind the command to a key,
and use it when a borrow error is hard to read; [Find and fix a borrow error](borrow-errors.md) shows the rest.

## 4. See a script's values beside its lines

For a script, a file that starts with `#!/usr/bin/env sisu`, the server
answers the editor's request for inline values with each top-level
line's value and what it printed, as `sisu ws` writes them:

<!-- file: examples/guide-editor/stove.sisu -->
```sisu
#!/usr/bin/env sisu
# A script that the editor shows with each line's value beside it.
let kilowatts = 9
let minutes = 60 * 10 // kilowatts
"{minutes} minutes to heat"
print("Kota is warm")
```

<!-- output: examples/guide-editor/stove.console -->
```console
$ sisu ws stove.sisu
#!/usr/bin/env sisu
# A script that the editor shows with each line's value beside it.
let kilowatts = 9  # => 9
let minutes = 60 * 10 // kilowatts  # => 66
"{minutes} minutes to heat"  # => "66 minutes to heat"
print("Kota is warm")
# | Kota is warm
```

The values come from a run against a test's fakes, with a budget of
statements, so typing a loop that never ends cannot hang the editor and
nothing the script does reaches the world. They are kept until the text
changes.

**Status.** In stage 0, with stage 1 answering every request but one:
a script's values still come from the Rust front end. Stage 1 answers
them too from its twenty-third round, run by its own evaluator, which
stops at the first line that reaches the world, and the server asks it
once stage 0's round in progress routes the query. The Rust front end
also answers everything when stage 1 is not built or
`--front-end=rust` is given. Stage 1's completion offers only the names
a program sees, and its references count the names in import lines. The structural editing with a typed palette
and the stepper of the teaching profile, which
[10](../../proposal/10-tooling.md) describes for editors, are designed.
