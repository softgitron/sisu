# Test code that uses the network

This guide tests a client of a web service without the service: it
passes the network in, scripts the answers, counts the requests, and
scripts a busy service and a slow one, and tests a client and a server
together over a network in the test's own process. It assumes capabilities and tests, from chapters 18 and 19 of
the tutorial.

**See also:** the tutorial's
[chapter 22](../tutorial/22-web-service.md#tests-over-a-network-in-memory)
(a service and its tests) and
[chapter 19](../tutorial/19-testing.md#fixtures-are-values) (a test's
fakes); the source of `std.x.http`,
[`compiler/std/http.sisu`](../../compiler/std/http.sisu); the
proposal's [17, Testing](../../proposal/17-testing.md#mocks).

## 1. Take the network as a parameter

A function that makes requests takes a `Net` and passes it to
`http.get` or `http.request`, and `main` gives it `sys.net`, narrowed
with `only_hosts` to the hosts it needs. This client asks a weather
service for a town's temperature:

<!-- file: examples/guide-network/weather.sisu -->
```sisu
# A client of a weather service, tested over a scripted network.
import http
from std.net import Url, UrlError

## Why an answer is not a temperature.
error WeatherError:
    NotANumber(text: String)

## The temperature in `town` now, from the weather service.
def temperature(
    net: Net,
    town: str,
) -> int ! {UrlError, HttpError, WeatherError}:
    let url = Url.parse("http://weather.example/{town}")?
    let body = http.get(net, url)?
    let text = body.decode().or(String())
    let degrees = text.trim().parse_int() else:
        return Err(NotANumber(text))
    return Ok(degrees)

test "the temperature in a town":
    let net = http.FakeNet().respond_to("http://weather.example/Oulu", b"-4")
    assert temperature(net, "Oulu") == Ok(-4)

test "a town that the service does not know":
    let net = http.FakeNet().respond_to("http://weather.example/Oulu", b"-4")
    assert temperature(net, "Atlantis") == Err(Status(404))

test "an answer that is not a number":
    let net = http.FakeNet().respond_to("http://weather.example/Oulu", b"cold")
    assert temperature(net, "Oulu") == Err(NotANumber("cold"))
```

`Url.parse` reads a URL that the code builds, here with the town in
it, and fails with a `UrlError`. A URL written whole, with nothing
interpolated, is a literal that becomes a `Url` where one is expected,
`let home: Url = "http://weather.example/"`, and stops the program if
it is not a URL ([chapter 6](../tutorial/06-text.md#other-string-literals)).

`http.get` gives the body of a `2xx` answer, and the error
`Status(code)` for any other status; `Network(cause)` when the
connection fails, with the network's own `NetError` as its cause, so
that a refused connection, `Refused(address)`, is told from one closed
early, `Closed`; `Interrupted(cause)` when the wait for the answer was
cancelled or its task panicked, a `TaskError`; and `Malformed(detail)`
for an answer that is not HTTP.

## 2. Script the answers

`http.FakeNet()` is a network that answers from a script, for the
tests of a client. `respond_to(url, body)` answers each `GET` of the
URL with `200 OK` and the body, and every other request with `404 Not
Found`. It becomes a `Net` where one is expected, so the client runs
unchanged. The tests of step 1 give it an answer, a town it does not
know, and an answer that is not a number:

<!-- output: examples/guide-network/weather.test -->
```text
test "the temperature in a town" ... ok
test "a town that the service does not know" ... ok
test "an answer that is not a number" ... ok
3 passed, 0 failed, 0 blocked
```

Script each answer a test needs, and the cases that go wrong as well
as the one that goes right: a fake never answers what nobody wrote
into it, so each test says what the service did.

## 3. Script a busy service and a slow one

`respond(method, url, response)` answers a method of a URL with any
`http.Response`, such as a `503` from a busy service, and
`respond_after(method, url, response, delay)` answers once `delay` has
passed on the test clock, which the client waits for at its first
read. The URL is compared as `Url` writes it, so
`http://weather.example:80/Oulu` is the same route:

<!-- file: examples/guide-network/scripted.sisu -->
```sisu
# A busy service and a slow one, scripted on a `FakeNet`.
import http
from std.net import Url, UrlError

const OULU: str = "http://weather.example/Oulu"

## The weather service's answer for `town`.
def weather(net: Net, town: str) -> String ! {UrlError, HttpError}:
    let url = Url.parse("http://weather.example/{town}")?
    let body = http.get(net, url)?
    return Ok(body.decode().or(String()))

test "a busy service":
    let busy = http.Response.text(503, "Service Unavailable", "busy")
    let net = http.FakeNet().respond("GET", OULU, busy)
    assert weather(net, "Oulu") == Err(Status(503))

test "a slow answer"(clock: TestClock):
    let wait = Duration.seconds(5)
    let answer = http.Response.ok(b"-4")
    let net = http.FakeNet().respond_after("GET", OULU, answer, wait)
    let start = clock.now()
    assert weather(net, "Oulu") == Ok("-4")
    assert clock.now() - start == wait
```

<!-- output: examples/guide-network/scripted.test -->
```text
test "a busy service" ... ok
test "a slow answer" ... ok
2 passed, 0 failed, 0 blocked
```

The slow answer takes five seconds of the test clock and none of the
test's time, so a test of a client's timeout or retry runs at once.

## 4. Count the requests

`request_count()` gives the number of requests the fake has answered,
which tells a test whether the code asked too often. The handles share
one script, so a count read through `net` includes the requests that
went through `cache.net`. This cache must ask once for each town:

<!-- file: examples/guide-network/cache.sisu -->
```sisu
# A cache in front of the weather service asks once for each town.
import http
from std.net import Url, UrlError

## The weather service's answers so far, by town.
struct Cache:
    net: Net
    answers: dict[String, String] = {}

    ## The service's answer for `town`, which it is asked for once.
    def weather(mut self, town: str) -> String ! {UrlError, HttpError}:
        if town in self.answers:
            return Ok(self.answers[town].copy())
        let url = Url.parse("http://weather.example/{town}")?
        let body = http.get(self.net, url)?
        let text = body.decode().or(String())
        _ = self.answers.insert(town.to_string(), text.copy())
        return Ok(text)

test "each town is asked for once":
    let oulu = http.FakeNet().respond_to("http://weather.example/Oulu", b"-4")
    let net = oulu.respond_to("http://weather.example/Turku", b"2")
    var cache = Cache(net)
    for town in ["Oulu", "Turku", "Oulu", "Oulu"]:
        _ = cache.weather(town)
    assert net.request_count() == 2
```

<!-- output: examples/guide-network/cache.test -->
```text
test "each town is asked for once" ... ok
1 passed, 0 failed, 0 blocked
```

## 5. Test a client and a server together

A test that names `net: Net` as a parameter receives a network of its
own, in the test's process: `listen` registers an address on it,
`connect` reaches what listens there, and nothing else exists, so a
port never collides with another test or with the machine. Serve on it
in a task scope, ask, and cancel the server:

<!-- file: examples/guide-network/served.sisu -->
```sisu
# A server and its client, tested over a network in the test's process.
import http
from std.net import Url, UrlError

## A weather service that knows one town.
struct Service:
    pass

impl http.Handler for Service:
    def handle(self, request: http.Request) -> http.Response:
        if request.target == "/Oulu":
            return http.Response.ok(b"-4")
        return http.Response.text(404, "Not Found", "no such town\n")

## The answer of the weather service at `localhost` for `town`.
def weather(net: Net, town: str) -> String ! {UrlError, HttpError}:
    let url = Url.parse("http://localhost/{town}")?
    let body = http.get(net, url)?
    return Ok(body.decode().or(String()))

test "the client and the server agree"(net: Net):
    let listener = net.listen(80).expect("a free port")
    let answers = task.scope do s:
        s.spawn(() -> http.serve(listener, Service()))
        let oulu = weather(net, "Oulu")
        let atlantis = weather(net, "Atlantis")
        s.cancel()
        return Ok((oulu, atlantis))
    let (oulu, atlantis) = answers.expect("the server stopped cleanly")
    assert oulu == Ok("-4")
    assert atlantis == Err(Status(404))

test "a client with no server to ask"(net: Net):
    let refused = match weather(net, "Oulu"):
        case Err(Network(Refused(_))): True
        case _: False
    assert refused
```

<!-- output: examples/guide-network/served.test -->
```text
test "the client and the server agree" ... ok
test "a client with no server to ask" ... ok
2 passed, 0 failed, 0 blocked
```

- `http.serve(listener, Service())` answers each connection with the
  handler, until `s.cancel()` stops it; the scope waits for it.
- The second test has no server at all, so `connect` is refused and
  the client gets `Network(Refused(_))`: the way to test what a client
  does when its service is down. A pattern inside a pattern reaches the
  cause, and `Network(_)` would take any failure of the network.
- `Net.fake()` makes the same kind of network outside a test's
  parameters, for a demo or a `main` that runs without a network.
