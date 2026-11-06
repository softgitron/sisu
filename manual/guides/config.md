# Generate configuration

This guide writes a deployment's configuration as a Sisu module of
constants and renders it with `sisu eval` as JSON, TOML or YAML, so that
the module's types and invariants check every value before a file
reaches a deployment. It assumes structs, enums, refinement types and
invariants, from chapters 7 and 23 of the tutorial.

**See also:** the tutorial's
[chapter 25](../tutorial/25-compile-time.md#configuration-as-constants)
(a first `sisu eval`); [R16](../reference/sisu-command.md#sisu-eval)
for the command and [R12](../reference/compile-time.md#configuration-files)
for the rule; [R17](../reference/diagnostics.md#e0617) for E0617; the
proposal's [08, Metaprogramming](../../proposal/08-metaprogramming.md#configuration-files)
and D-063.

## 1. Write the configuration as constants

A configuration module is an ordinary module: types that say what a
value may be, `pure` functions that compute values, and a `const` for
each top-level key. It needs no `main`. Here a port is a refinement of
`int`, a service has an invariant on its replicas, and a function
works out how many replicas a share of the traffic needs:

<!-- file: examples/guide-config/services.sisu -->
```sisu
# A deployment's configuration, as a module of constants.
type Port = int where 1024 <= self <= 65535

enum Tier:
    Free
    Paid(seats: int)

struct Service:
    name: String
    port: Port
    replicas: int = 1
    tier: Tier = Tier.Free
    invariant self.replicas >= 1

## The replicas for a service that takes `share` percent of the traffic.
pure def replicas_for(share: int) -> int:
    return (share + 24) // 25

const REGION: String = "eu-north"
const SERVICES: list[Service] = [
    Service(name="web", port=8080, replicas=replicas_for(60)),
    Service(name="api", port=9000, tier=Tier.Paid(seats=40)),
]
```

The defaults of a struct's fields are the configuration's defaults, so
the `api` service writes only what differs. A constant may call the
prelude's functions too, such as `max`, `min`, `sorted`, `any` and
`zip`, as [R12](../reference/compile-time.md#configuration-files)
lists them.

## 2. Render it as JSON

`sisu eval FILE` evaluates each constant with the compile-time
evaluator, in the order written, and prints them as one JSON object
keyed by the constants' names:

<!-- output: examples/guide-config/json.console -->
```console
$ sisu eval services.sisu
{
  "REGION": "eu-north",
  "SERVICES": [
    {
      "name": "web",
      "port": 8080,
      "replicas": 3,
      "tier": "Free"
    },
    {
      "name": "api",
      "port": 9000,
      "replicas": 1,
      "tier": {
        "Paid": {
          "seats": 40
        }
      }
    }
  ]
}
```

A struct is a mapping of its fields, in order; a variant without
fields is its name, and one with fields a mapping of its name to them.
A list, a set and a tuple are sequences, and a `dict` keyed by text or
integers is a mapping. The evaluation runs with the compile-time budget
and reaches nothing outside the program, so the same module gives the
same file on every machine.

## 3. Choose TOML or YAML

`--to toml` and `--to yaml` write the same values in the other two
forms:

<!-- output: examples/guide-config/toml_yaml.console -->
```console
$ sisu eval services.sisu --to toml
REGION = "eu-north"
SERVICES = [{ name = "web", port = 8080, replicas = 3, tier = "Free" }, { name = "api", port = 9000, replicas = 1, tier = { Paid = { seats = 40 } } }]
$ sisu eval services.sisu --to yaml
REGION: "eu-north"
SERVICES:
  - name: "web"
    port: 8080
    replicas: 3
    tier: "Free"
  - name: "api"
    port: 9000
    replicas: 1
    tier:
      Paid:
        seats: 40
```

TOML writes a struct as an inline table on its constant's line, and
YAML as a mapping in block style. Redirect the output to the file your
deployment reads: `sisu eval services.sisu --to yaml > services.yaml`.

## 4. Let the schema refuse a wrong value

A value that breaks an invariant stops the evaluation, and `sisu eval`
reports it at the constant's name, writes nothing on standard output
and ends with status 1. `sisu check` does not evaluate the constants,
so it passes the same module:

<!-- file: examples/guide-config/refused.sisu -->
```sisu
# A service with no replicas, which the configuration's invariant refuses.
struct Service:
    name: String
    replicas: int = 1
    invariant self.replicas >= 1

## The replicas for a service that takes `share` percent of the traffic.
pure def replicas_for(share: int) -> int:
    return (share + 24) // 25

const REGION: String = "eu-north"
const BATCH: Service = Service(name="batch", replicas=replicas_for(0))
```

<!-- output: examples/guide-config/refused.console -->
```console
$ sisu check refused.sisu
$ sisu eval refused.sisu
error[E0602]: the compile-time evaluation of this expression panicked: invariant of `Service` failed: self.replicas >= 1
    self.replicas >= 1
    │
    0
  --> refused.sisu:12:7
   |
12 | const BATCH: Service = Service(name="batch", replicas=replicas_for(0))
   |       ^^^^^

```

Run `sisu eval` in the job that writes the file, so a wrong value fails
the job rather than a deployment.

## 5. Keep to what the form can write

Each form has values it cannot write. JSON has no infinite float and
TOML no `null`, and none of them can hold a function or a reference.
A constant that holds such a value is E0617 (`eval.no-form`), named by
the constant, and the same module may render in another form:

<!-- file: examples/guide-config/forms.sisu -->
```sisu
# Values that one form writes and another cannot.
struct Limits:
    timeout: f64
    mirror: String?

## No timeout: the infinite float, which JSON has no number for.
const LIMITS: Limits = Limits(timeout=1.0 / 0.0, mirror=None)
```

<!-- output: examples/guide-config/forms.console -->
```console
$ sisu eval forms.sisu
error[E0617]: `LIMITS` holds an infinite or not-a-number float, which JSON cannot write
  --> forms.sisu:7:7
  |
7 | const LIMITS: Limits = Limits(timeout=1.0 / 0.0, mirror=None)
  |       ^^^^^^

$ sisu eval forms.sisu --to toml
LIMITS = { timeout = inf }
$ sisu eval forms.sisu --to yaml
LIMITS:
  timeout: .inf
  mirror: null
```

`None` is `null` in JSON and YAML, and TOML leaves out the field that
holds it; a `None` inside a list has no TOML at all, and is E0617
there. Where a form refuses a value, choose a value it has, such as a
large number of seconds for JSON, or the form that has it.

**Status.** In stage 0, with the differences noted.
[08](../../proposal/08-metaprogramming.md#configuration-files) renders
the values through each type's `Encode` (leaning, D-063); `sisu eval`
writes a value's own structure, which is what `@derive(Encode)` would
write of it, whether or not the type derives it. `sisu eval` is stage
1's, so `--front-end=rust` answers it with D0003.
