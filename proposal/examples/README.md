# Examples

Illustrative Sisu programs in the indentation syntax of
[02-syntax.md](../02-syntax.md). They follow the proposal as written;
nothing compiles yet. Each file focuses on one area and is referenced from
the corresponding design document.

| File | Shows |
|---|---|
| `hello.sisu` | Entry point, capabilities, printing |
| `wordcount.sisu` | A complete command-line tool: arguments, files, errors, maps, sorting |
| `errors.sisu` | Error sets, inference, propagation, handling, panics |
| `ownership.sisu` | Parameter conventions, borrows, views, `Shared`, arenas |
| `traits.sisu` | Traits, generics, `dyn`, derive, delegation |
| `concurrency.sisu` | Scopes, tasks, channels, cancellation, `Send`/`Sync` errors |
| `capabilities.sisu` | Attenuation and testing with fakes |
| `contracts.sisu` | Pre/postconditions, invariants, refinement types |
| `comptime.sisu` | Compile-time tables, reflection, a derive, a checked string DSL |
| `testing.sisu` | Tests, fixtures via capabilities, parametrised, property, snapshot, bench |
| `ffi.sisu` | Header import, exporting a C ABI |
