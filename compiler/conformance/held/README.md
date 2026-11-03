# Held cases

Cases that wait for a change outside stage 0 before they join
`../cases/`, each group with the reason (port-debt.md names the change).

- `unsafe/` waits for two changes of stage 1 (port-debt.md, "Shortcuts in
  the interpreter"): its lowering writing the type arguments of
  `mem.size_of[T]()` and `mem.align_of[T]()`, whose `T` appears in no
  parameter, and std.mem calling a free function where it calls
  `Layout.array[T](n)`, which neither front end types. `raw_buffer` is a
  stack on `RawBuf` (writes, moves out, references, growth, `drop_range`,
  a buffer over a `Region`), and `layouts` the sizes and alignments
  `size_of` and `align_of` give. Their `.out` files are what a stage 1
  with both changes made them print, interpreted and native (debug and
  release, nothing live at exit). Once stage 1 has changed, they move to
  `../cases/unsafe/` and their names to `../native.list`, with the other
  unsafe cases' `not from LIR:` mark; `cargo run -p sisu-conformance --
  --sisu target/debug/sisu conformance/held` checks them where they are.
