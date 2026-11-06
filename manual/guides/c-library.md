# Call a C library

This guide wraps a C function in a package of its own whose API is
safe and takes the capability for what the function does, and uses the
package from an application that trusts it. It assumes unsafe code and
`extern "C"`, from chapter 27 of the tutorial, and capabilities, from
chapter 18.

**See also:** the tutorial's
[chapter 27](../tutorial/27-systems.md#calling-c) (calling C, and
packages that contain unsafe code); the proposal's
[22, Unsafe](../../proposal/22-unsafe.md) and
[12, Interop and targets](../../proposal/12-interop-and-targets.md);
[R13, Modules](../reference/modules.md) for the manifest.

## 1. Declare the C function, and wrap it in a safe one

The wrapper is a library, `cenv`, here over C's `getenv`. An `extern
"C":` block declares the function with the C types of `std.ffi`, under
a `# safety:` comment that says where the signature comes from. A
public function around it does the unsafe work, with a comment that
answers each obligation, and gives the caller ordinary values:

<!-- file: examples/guide-c-library/ffi.pkg/cenv/src/lib.sisu -->
```sisu
##! C's `getenv`, behind a safe function that takes an `Env`.
from std.ffi import CStr, CString, c_char
from std.mem import ptr

# safety: the signature as <stdlib.h> declares it; it returns promptly.
extern "C":
    def getenv(name: ptr[c_char]) -> ptr[c_char]?

## The value of the environment variable `name`, if it is set and is
## text. `env` is the authority to read the environment.
pub def lookup(env: Env, name: str) -> String?:
    let c_name = CString.new(name) else:
        return None
    var found: String? = None
    # safety: valid: `c_name` lives across the call and ends with its
    #   NUL, and getenv keeps no pointer to it; terminated, stable: the
    #   result ends with a NUL and stays put while it is read.
    unsafe:
        if let Some(value) = getenv(c_name.as_ptr()):
            found = CStr.from_ptr(value).to_text().ok()
    return found
```

- `CString.new(name)` copies the name with the NUL after it that C
  needs, and `c_name.as_ptr()` points to those bytes while `c_name`
  lives, in the interpreter and natively. `CString.new` gives `None`
  for a name that holds a NUL of its own, which C would cut short.
- `getenv` gives an optional pointer, `None` for C's null, and
  `CStr.from_ptr` views the C string it points to until `to_text`
  copies it into a `String`.
- `lookup` takes an `Env`, the capability whose authority the C call
  exercises, though it never reads it: it is the permission. Without
  it, a public safe function that reaches the call is W0014, as chapter
  27 shows, since any code could then read the environment.

The `Env` is the permission and not the source: `getenv` reads the
process's own environment, whatever `Env` the caller holds.

## 2. Declare the package's unsafe level

The wrapper's manifest says that the package holds foreign code, with
`unsafe = "extern"`:

<!-- file: examples/guide-c-library/ffi.pkg/cenv/sisu.toml -->
```toml
[package]
name = "cenv"
version = "0.1.0"
edition = "2027"
unsafe = "extern"

[dependencies]
```

Without the line, every build of the package stops with E1019
(`package.unsafe-undeclared`), which names the level to declare:

<!-- output: examples/guide-c-library/undeclared.console -->
```console
$ sisu run
error[E1019]: the package `cenv` contains foreign code (an `extern` block or a header import), and its `sisu.toml` declares no `unsafe`: add `unsafe = "extern"` to its `[package]`
  --> cenv/src/lib.sisu:6:1
  |
6 | extern "C":
  | ^^^^^^

error[D0006]: the program was not run: errors found (1)
```

## 3. Trust the wrapper from the application

An application that uses the wrapper lists it under `[trust]`: the
program's author consents, for every package in the build, to the ones
that may hold unsafe code:

<!-- file: examples/guide-c-library/ffi.pkg/sisu.toml -->
```toml
[package]
name = "greeter"
version = "0.1.0"
edition = "2027"

[dependencies]
cenv = { path = "cenv" }

[trust]
unsafe = ["cenv"]
```

<!-- file: examples/guide-c-library/ffi.pkg/src/main.sisu -->
```sisu
##! Greets whoever the environment says is logged in.
import cenv

def main(sys: System):
    let who = cenv.lookup(sys.env, "USER") ?? "stranger"
    print("Hei, {who}!")
    print("the path: {cenv.lookup(sys.env, "PATH") ?? "not set"}")

test "getenv reads the process's variables, whatever the Env":
    let env = Env.fake(vars={"SAUNA": "Kotiharju"})
    assert cenv.lookup(env, "SAUNA") == None
    assert cenv.lookup(env, "PATH") != None
```

Without `[trust]`, the build refuses the wrapper with E1021
(`package.unsafe-untrusted`) at its manifest's level, which gives the
line to add; the `extern` and `unsafe` blocks it declares are not
reported one by one:

<!-- output: examples/guide-c-library/untrusted.console -->
```console
$ sisu run
error[E1021]: the dependency `cenv` declares `unsafe = "extern"`, and `[trust] unsafe` in the root package's `sisu.toml` does not list it
  --> cenv/sisu.toml:5:10
  |
5 | unsafe = "extern"
  |          ^^^^^^^^
  = note: the root package consents by listing it: `[trust] unsafe = ["cenv"]`

error[D0006]: the program was not run: errors found (1)
```

The lockfile records the level too, so that an update of the wrapper
that raises it is refused unless you accept it, as chapter 27 shows.

## 4. Test it, and run it in the interpreter and natively

`sisu run --native` calls the C library itself. The interpreter cannot,
and runs instead the six C functions it knows (`strlen`, `memcpy`,
`malloc`, `free`, `getenv` and `write`) on its model of memory, where
the obligations are checked as the program runs. The native debug build
checks them too, on the blocks that `Global` gives out and on the
memory those six functions give out: `getenv`'s text is read and never written, and `strlen`
reads no further than the block that holds its argument. The test in `main.sisu`
runs there too: it hands `lookup` a fake `Env`, which `getenv` does not
read, so the process's own `PATH` is found and an unset name is not.
The runs print the same:

<!-- output: examples/guide-c-library/ffi.console -->
```console
$ sisu test
test "getenv reads the process's variables, whatever the Env" ... ok
1 passed, 0 failed, 0 blocked
$ sisu run
Hei, stranger!
the path: /usr/bin:/bin
$ sisu run --native
Hei, stranger!
the path: /usr/bin:/bin
$ sisu audit --unsafe
greeter 0.1.0   no `unsafe`   the root package
  no unsafe code
cenv 0.1.0   unsafe = "extern"   trusted in sisu.toml:10
  cenv (cenv/src/lib.sisu)   1 block, 1 extern block, module hash 68e1a0bbbd9410b3
    cenv/src/lib.sisu:6   extern block   # safety: the signature as <stdlib.h> declares it; it returns promptly.
    cenv/src/lib.sisu:18   unsafe block in lookup   # safety: valid: `c_name` lives across the call and ends with its NUL, and getenv keeps no pointer to it; term...
  foreign functions: getenv (called by lookup (takes `Env`))
std (toolchain)   unsafe = "extern"   see `sisu audit --unsafe --std`
```

A C function that the interpreter does not know runs only natively,
and so does a test that reaches one. `getenv` is one of the few that it
emulates, so the test of `lookup` runs on the checked memory model, as a
run does. Keep the logic that needs no C outside the wrapper anyway,
where every test reaches it, and keep the wrapper thin.

The session's last command, `sisu audit --unsafe`, is what a reviewer
of the application reads: each package with its level and the line of
`[trust]` that consents to it, each `extern` and `unsafe` block with the
start of its `# safety:` reason, and the foreign functions with the safe
functions that call them, `lookup` taking an `Env`
([R16](../reference/sisu-command.md#sisu-audit)).

**Status.** In stage 0 (22). Importing a C header, `import c
"stdlib.h"`, which would write the `extern` block, is designed (E0900).
The
other direction, C calling Sisu, is `@export("NAME")`: a native build
makes the function the global symbol `NAME` of C's own convention,
whether or not the program calls it, so a C caller declares `int32_t
sauna_heat(int32_t minutes);` and calls it; E0991 refuses a type C has
no layout for, a `@repr(c)` struct goes by a pointer to it, its fields
at C's offsets (by value it is D0103, and so is a pointer to a tuple
with a part narrower than a word natively), and a panic that leaves the function ends the program with
status 134 ([R14](../reference/attributes.md#exported-functions)).
