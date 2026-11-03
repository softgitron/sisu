# Check what a dependency can do

This guide finds out what a library can reach before you build it into
a program: what its manifest declares, which of its functions take
which capabilities, and how far its authority goes once your program
narrows it. It assumes capabilities and packages, from chapters 17 and
18 of the tutorial.

**See also:** the tutorial's
[chapter 18](../tutorial/18-capabilities.md) (capabilities and `sisu
audit --caps`) and [chapter 27](../tutorial/27-systems.md#packages-that-contain-unsafe-code)
(packages that contain unsafe code); [R16](../reference/sisu-command.md#sisu-audit)
for `sisu audit`; [Call a C library](c-library.md), for the `[trust]`
that unsafe code needs; the proposal's
[09, Modules, packages and build](../../proposal/09-modules-packages-and-build.md)
and D-066.

## 1. Read the manifest

A dependency's `sisu.toml` says whether it holds unsafe code: `unsafe =
"memory"` or `unsafe = "extern"` in its `[package]`. Such a package
builds only when your root package lists it under `[trust]`, and the
lockfile records the level, so an update that raises it is refused
unless you accept it. A manifest with no `unsafe` line, as this weather
library's, holds none, and the compiler refuses any it finds (E1019):

<!-- file: examples/guide-audit/app.pkg/forecast/sisu.toml -->
```toml
[package]
name = "forecast"
version = "0.1.0"
edition = "2027"

[dependencies]
```

A package has no build script, so nothing in it runs while your
program compiles beyond `comptime` code, which does no I/O.

## 2. List the capabilities its functions take

Without unsafe code, a function reaches the world only through a
capability it is given. `sisu audit --caps FILE` lists each function
that does I/O with the number of capability parameters it takes, and
each use of a capability's operation:

<!-- file: examples/guide-audit/app.pkg/forecast/src/lib.sisu -->
```sisu
##! Tomorrow's weather, from a service on the network.
import http
from std.net import Url, UrlError

## The forecast for `town`, as the service at `host` gives it.
pub def tomorrow(
    net: Net,
    host: str,
    town: str,
) -> String ! {UrlError, HttpError}:
    let url = Url.parse("http://{host}/{town}")?
    let body = http.get(net, url)?
    return Ok(body.decode().or(String()))

## Whether `text` says that it will rain.
pub def rain(text: str) -> bool:
    return text.contains("rain")

## Tomorrow's forecast for `town`, and the time it was asked, as a line
## of a log.
pub def logged(
    net: Net,
    clock: Clock,
    town: str,
) -> String ! {UrlError, HttpError}:
    let text = tomorrow(net, "weather.example", town)?
    return Ok("{clock.now()} {town}: {text}")
```

<!-- output: examples/guide-audit/app.console -->
```console
$ sisu audit --caps forecast/src/lib.sisu
forecast/src/lib.sisu function logged caps=2 io=direct depth=-
forecast/src/lib.sisu function tomorrow caps=1 io=direct depth=-
forecast/src/lib.sisu use logged ClockOps.now depth=-
forecast/src/lib.sisu use tomorrow http.get depth=-
forecast/src/lib.sisu summary functions=2 median=1.5 max=2 uses=2 deepest=- structs=0
```

- `tomorrow` takes one capability, a `Net`, and uses it through
  `http.get`; `logged` takes two, and reads the clock itself.
- `rain` is not listed: it takes no capability, so it can only compute,
  whoever calls it.
- `depth=-` says that no `main` in the file calls the function; in a
  program, it is the fewest calls from `main` to the use.

## 3. Give it only what it needs

The library can reach only the capabilities your program passes it, so
the last check is your own `main`. This one gives `tomorrow` a network
narrowed to the one host, so the library cannot reach another,
whatever its code does:

<!-- file: examples/guide-audit/app.pkg/src/main.sisu -->
```sisu
##! Says whether to heat the sauna by the lake tomorrow.
import forecast
from std.net import UrlError

def main(sys: System) -> ! {UrlError, HttpError}:
    let net = sys.net.only_hosts(["weather.example"])
    let text = forecast.tomorrow(net, "weather.example", "Oulu")?
    let rain = forecast.rain(text)
    print(if rain: "heat the sauna indoors" else: "go to the lake")
    return Ok(())
```

<!-- file: examples/guide-audit/app.pkg/sisu.toml -->
```toml
[package]
name = "evening"
version = "0.1.0"
edition = "2027"

[dependencies]
forecast = { path = "forecast" }
```

Read the signatures of the functions you call as you read the audit:
a function whose parameters hold no capability touches nothing, and one
that takes `sys: System` takes everything.

**Status.** `sisu audit --caps` and the manifest's `unsafe` levels are
in stage 0. `sisu audit` reads each file alone, so a file that imports
a dependency, as the application's `main` imports `forecast`, or
another module of its own package is reported as E0220, and a package
cannot be audited as a whole. The capability summary that the registry shows for
each published version, and admission's authority delta, which says
what a new version of a dependency can reach that the old one could
not (D-066), are designed and wait for the registry.
