# 22. Project: a small web service

In this chapter you build a web service with what Part IV taught: a
service that books a sauna's evening turns, answers in JSON, keeps its
bookings in a file that survives a crash, and answers each connection
on a task of its own. You test it over a network in memory and under a
simulator that crashes machines and makes calls fail, record a run and
replay it, change its code while it runs, and build it into an
executable. At the end, Challenge IV asks you to put a limit and a
cache in front of a slow service. You need about two and a half hours.

## The service

The service answers two requests. `POST /bookings`, with a JSON body
such as `{"name": "Aino", "hour": 18}`, books the turn at that hour, or
says that it is taken; `GET /bookings` gives every booking as a JSON
list. It is a package, `bookings`, with the manifest that `sisu new`
writes:

<!-- file: examples/22-web-service/service.pkg/sisu.toml -->
```toml
[package]
name = "bookings"
version = "0.1.0"
edition = "2027"

[dependencies]
```

Its code is in six modules: `store` keeps the bookings in a file,
`service` answers requests, `main` starts the server, `demo` gives it a
short session of its own, and `tests` and `sim_tests` test it. The
sections take them in that order.

## Bookings kept in a file

The bookings live in a file, `bookings.json`, as JSON:

<!-- file: examples/22-web-service/service.pkg/src/store.sisu -->
```sisu
##! Bookings kept in a file, as JSON.
import std.json

## A turn in the sauna: who, and at which hour.
pub struct Booking:
    pub name: String
    pub hour: int

## Why the bookings could not be read.
pub type LoadError = {IoError, json.JsonError}

## The bookings kept in `bookings.json`, none when there is no file.
pub def load(fs: Fs) -> list[Booking] ! LoadError:
    if not fs.exists("bookings.json"):
        return Ok([])
    let text = fs.read_to_string("bookings.json")?
    return json.decode[list[Booking]](text)

## Keeps `bookings` in `bookings.json`, durably: once `save` returns, a
## crash cannot lose them.
pub def save(fs: Fs, bookings: list[Booking]) -> ! IoError:
    fs.write_text("bookings.json", json.encode(bookings))?
    fs.sync("bookings.json")?
    fs.sync(".")?
    return Ok(())
```

- `json.decode[list[Booking]](text)` reads the file into a list of
  bookings, and `json.encode(bookings)` writes one, from the struct's
  fields, as chapter 21 showed: the struct is the format, and nothing
  else has to be written for it.
- `type LoadError = {IoError, json.JsonError}` names a union of error
  sets, so that a signature can say `! LoadError`.
- `save` writes the file and then syncs it twice: `fs.sync("bookings.json")`
  makes the file's contents durable, and `fs.sync(".")` makes durable
  the directory's entry for it, its name. A write that is not synced
  may be lost when the machine stops; the section "Two nodes and a
  crash" shows a test that catches the difference.

## Answering requests

The service is a struct that implements `http.Handler`, the trait of
`std.x.http` whose `handle` turns a request into a response:

<!-- file: examples/22-web-service/service.pkg/src/service.sisu -->
```sisu
##! The bookings service: a sauna's evening turns over HTTP.
import http
import std.json
from bookings.store import Booking, LoadError, load, save

## The bookings, and the file system that keeps them.
pub struct Service:
    fs: Fs
    bookings: Mutex[list[Booking]]

    ## A service over the bookings that `fs` keeps.
    pub def open(fs: Fs) -> Service ! LoadError:
        return Ok(Service(fs=fs, bookings=Mutex(load(fs)?)))

    ## Books the turn that the JSON `text` asks for.
    def book(self, text: str) -> http.Response:
        let wanted = json.decode[Booking](text) else as e:
            return http.Response.text(400, "Bad Request", "{e}\n")
        var guard = self.bookings.lock()
        for b in guard.value:
            if b.hour == wanted.hour:
                return http.Response.text(409, "Conflict", "taken\n")
        log.info("{wanted.name} books {wanted.hour}:00")
        guard.value.push(wanted)
        if let Err(e) = save(self.fs, guard.value):
            _ = guard.value.pop()
            return http.Response.text(503, "Service Unavailable", "{e}\n")
        return http.Response.text(201, "Created", "booked\n")

impl http.Handler for Service:
    def handle(self, request: http.Request) -> http.Response:
        if request.target != "/bookings":
            return http.Response.text(404, "Not Found", "no such page\n")
        if request.method == "POST":
            return self.book(request.body.decode().or(String()))
        let all = json.encode(self.bookings.lock().value)
        return http.Response.ok(all.encode())
```

- `Service.open(fs)` loads the bookings that `fs` keeps, and the
  service keeps them in a `Mutex`, so that two requests that come at
  once take turns: two guests cannot both book 18:00.
- `book` answers `400 Bad Request` for a body that is not a booking,
  `409 Conflict` for a turn that is taken, and `201 Created` once the
  booking is saved. When saving fails, it takes the booking back and
  answers `503 Service Unavailable`, so a guest is never told that a
  booking was kept when it was not.
- `log.info` records each booking on standard error.

`handle` is an ordinary method: given a request, it gives a response,
and nothing in it knows about connections or tasks.

## Starting the server

`main` narrows the capabilities, opens the service and starts the
server:

<!-- file: examples/22-web-service/service.pkg/src/main.sisu -->
```sisu
##! bookings: a JSON service for a sauna's evening turns.
import bookings.demo
import cli
import http
from bookings.service import Service

## What the command line can say.
struct Options:
    port: int = 8080
    ## Serves a short session of its own, kept in memory, and stops.
    demo: bool

def main(sys: System) -> !:
    let options = cli.parse[Options](sys.env.args())?
    let net = sys.net.only_hosts(["localhost"])
    if options.demo:
        let service = Service.open(Fs.in_memory())?
        return demo.session(net, net.listen(0)?, service)
    sys.fs.create_dir("data")?
    let service = Service.open(sys.fs.restrict_to("data"))?
    let listener = net.listen(options.port)?
    log.info("listening at port {listener.port}")
    return http.serve(listener, service)
```

- `sys.net.only_hosts(["localhost"])` is the only network the service
  gets, and `sys.fs.restrict_to("data")` the only file system, so the
  rest of the program can neither reach another machine nor touch a
  file outside `data`.
- `net.listen(options.port)` listens at the port, 8080 unless `--port`
  says otherwise.
- `http.serve(listener, service)` accepts connections until it is
  cancelled, and answers each on a task of its own, in a scope that the
  call opens, so a slow client does not hold up the others.

Run it with `sisu run`, and it serves until you stop it with Ctrl-C;
from another terminal, `curl -d '{"name": "Aino", "hour": 18}'
localhost:8080/bookings` books a turn. The `--demo` option is for
trying it without another program, as the next section shows.

## Trying it, and replaying the run

`demo.session` starts the server and sends it two bookings and a
request for the list, as a client on the same machine would:

<!-- file: examples/22-web-service/service.pkg/src/demo.sisu -->
```sisu
##! A short session with the service, for trying it without a client.
import http
from bookings.service import Service
from std.net import Url, UrlError

## Serves on `listener`, sends the server two bookings and a request for
## the list through `net`, prints the answers, and stops the server.
pub def session(
    net: Net,
    listener: Listener,
    service: Service,
) -> ! {UrlError, HttpError, NetError, TaskError}:
    let url = Url.parse("http://localhost:{listener.port}/bookings")?
    return task.scope do s:
        s.spawn(() -> http.serve(listener, service))
        for who in ["Aino", "Eero"]:
            let body = "{{\"name\": \"{who}\", \"hour\": 18}}"
            let answer = http.request(net, "POST", url, body.encode())?
            print("{who}: {answer.status} {answer.reason}")
        let all = http.get(net, url)?
        print("bookings: {all.decode().or(String())}")
        s.cancel()
        return Ok(())
```

With `--demo`, `main` gives the session a service that keeps its
bookings in memory, so that every demo starts empty, and a listener at
a port the system chooses, through the network `main` narrowed:

<!-- output: examples/22-web-service/demo.console -->
```console
$ sisu run -- --demo
info [task 1.1]: Aino books 18:00
Aino: 201 Created
Eero: 409 Conflict
bookings: [{"name":"Aino","hour":18}]
$ sisu run --record demo.log -- --demo
info [task 1.1]: Aino books 18:00
Aino: 201 Created
Eero: 409 Conflict
bookings: [{"name":"Aino","hour":18}]
$ sisu replay demo.log
info [task 1.1]: Aino books 18:00
Aino: 201 Created
Eero: 409 Conflict
bookings: [{"name":"Aino","hour":18}]
```

- The first command runs the demo: Aino books 18:00, Eero is turned
  away from the same hour, and the list holds Aino's booking. The `info`
  line is the server's log, from the connection's task, `task 1.1`.
- `sisu run --record demo.log` runs it the same way, and writes every
  call the program made to the world into the log `demo.log`: each
  connection, read and write of the network, the clock, and what it
  printed.
- `sisu replay demo.log` runs the program again with the log standing
  in for the world: each call gets the answer it got the first time,
  with no network at all. A run that went wrong in production can be
  recorded and then replayed, as often as needed, on a developer's
  machine.

The demo reaches only the machine's own loopback interface, at a port
it chooses, so it needs no network beyond the machine.

## Tests over a network in memory

The tests start the server on a network that exists only in the
test's process:

<!-- file: examples/22-web-service/service.pkg/src/tests.sisu -->
```sisu
##! The service's tests, over a network in the test's own process.
import http
from bookings.service import Service
from std.net import Url

## What a server on `net` answers to `method` for `/bookings`.
@cfg(test)
def ask(net: Net, method: str, body: str) -> (int, String) ! HttpError:
    let url = Url.parse("http://localhost/bookings").expect("a URL")
    let answer = http.request(net, method, url, body.encode())?
    return Ok((answer.status, answer.body.decode().or(String())))

test "a turn is booked once"(fs: Fs, net: Net):
    let listener = net.listen(80).expect("a free port")
    let service = Service.open(fs).expect("an empty file system")
    let answers = task.scope do s:
        s.spawn(() -> http.serve(listener, service))
        let aino = ask(net, "POST", r"""{"name": "Aino", "hour": 18}""")?
        let eero = ask(net, "POST", r"""{"name": "Eero", "hour": 18}""")?
        let all = ask(net, "GET", "")?
        s.cancel()
        return Ok([aino, eero, all])
    let got = answers.expect("every request answered")
    assert got[0] == (201, "booked\n")
    assert got[1] == (409, "taken\n")
    assert got[2] == (200, r"""[{"name":"Aino","hour":18}]""")
    assert fs.exists("bookings.json")

test "a request that is not a booking"(fs: Fs, net: Net):
    let listener = net.listen(80).expect("a free port")
    let service = Service.open(fs).expect("an empty file system")
    let answer = task.scope do s:
        s.spawn(() -> http.serve(listener, service))
        let bad = ask(net, "POST", "Aino at six")?
        s.cancel()
        return Ok(bad.0)
    assert answer == Ok(400)
```

- `@cfg(test)` keeps `ask` out of every build but the tests', as
  chapter 17 showed.
- `(fs: Fs, net: Net)` gives each test a fresh in-memory file system
  and a fresh network of its own, as chapter 19 showed: `net.listen(80)`
  can never collide with another test, or with a server on the machine.
- `http.request(net, method, url, body)` is the client's side, through
  the same network.
- `s.cancel()` stops the server once the test has its answers, and the
  scope waits for it to stop.

`sisu test` in the package's directory runs the tests of every module:

<!-- output: examples/22-web-service/service.console -->
```console
$ sisu test
info [task 1.1]: Aino books 18:00
info [task 1.1]: Aino books 18:00
test "sim_tests/a booking outlives a crash of the server" ... ok
test "tests/a turn is booked once" ... ok
test "tests/a request that is not a booking" ... ok
3 passed, 0 failed, 0 blocked
```

The `info` lines come first, since the log is written as the tests
run, and each test's report is written once it has ended.

## Two nodes and a crash

A simulated system runs several machines, called nodes, in one
process, each with its own file system, on one network that the test
controls. `sim.crash(node)` stops a node as a power cut would:

<!-- file: examples/22-web-service/service.pkg/src/sim_tests.sisu -->
```sisu
##! The service on two nodes of a simulated system.
import http
from bookings.service import Service
from bookings.store import load
from std.net import Url
from std.sim import Sim

## Books `hour` for `name` on the node `server`, trying again after a
## fault, ten times at most; the status of the last try.
@cfg(test)
def book(net: Net, name: str, hour: int) -> int:
    let url = Url.parse("http://server/bookings").expect("a URL")
    let body = "{{\"name\": \"{name}\", \"hour\": {hour}}}".encode()
    var status = 0
    for _ in range(10):
        let answer = http.request(net, "POST", url, body)
        status = answer.map((r) -> r.status).or(0)
        if status != 0 and status != 503:
            return status
    return status

test "a booking outlives a crash of the server":
    let sim = Sim.new(seed=1)
    let server = sim.node("server")
    let client = sim.node("client")
    let listener = sim.system(server).net.listen(80).expect("listening")
    let service = Service.open(sim.system(server).fs).expect("a new disk")
    let status = task.scope do s:
        s.spawn(() -> http.serve(listener, service))
        let status = book(sim.system(client).net, "Aino", 18)
        s.cancel()
        return status
    assert status == 201
    sim.crash(server)
    sim.restart(server)
    let kept = load(sim.system(server).fs).expect("the disk after the crash")
    assert len(kept) == 1
```

- `Sim.new(seed=1)` makes a simulation, and `sim.node("server")` a node,
  which other nodes reach by its name: `http://server/bookings`.
- `sim.system(node)` is the node's `System`. Its file system is
  pessimistic: a write is seen at once, but only a sync makes it
  durable, and a crash puts back what was synced.
- The test books a turn, crashes the server, restarts it, and reads the
  bookings from the disk that the crash left.

**Try it.** Remove the two `sync` lines from `save` in `store.sisu`, and
run the tests.

<details>
<summary>What Sisu says</summary>

<!-- output: examples/22-web-service/unsynced.console -->
```console
$ sisu test
info [task 1.1]: Aino books 18:00
info [task 1.1]: Aino books 18:00
test "sim_tests/a booking outlives a crash of the server" ... FAILED
test "tests/a turn is booked once" ... ok
test "tests/a request that is not a booking" ... ok

test "sim_tests/a booking outlives a crash of the server" ... FAILED
panic: assertion failed: len(kept) == 1
    len(kept) == 1
    │   │
    0   []
  at test "sim_tests/a booking outlives a crash of the server" (src/sim_tests.sisu:37:5)
replay: sisu test --replay 1-22c643d824ddb621-0.0.1-t0-s0-r0 .

2 passed, 1 failed, 0 blocked
```

Without the syncs, the booking was in the file when the service said
`201 Created`, and gone after the crash: the server told Aino a lie.
On a real disk the same can happen in a power cut, rarely, and is
almost never seen in a test. The simulator makes it happen every time.

</details>

## Faults from the simulator

`sisu test --sim` runs the tests on a simulated system, where the
calls to the world fail now and then, at the rate `--faults` gives:

<!-- output: examples/22-web-service/faults.console -->
```console
$ sisu test --sim --faults 0.1
info [task 1.1]: Aino books 18:00
info [task 1.2]: Aino books 18:00
info [task 1.1]: Aino books 18:00
info [task 1.2]: Eero books 18:00
test "sim_tests/a booking outlives a crash of the server" ... ok
test "tests/a turn is booked once" ... FAILED
test "tests/a request that is not a booking" ... ok

test "tests/a turn is booked once" ... FAILED
panic: assertion failed: got[0] == (201, "booked\n")
    got[0] == (201, "booked\n")
    │
    (503, "Interrupted\n")
  at test "tests/a turn is booked once" (src/tests.sisu:24:5)
replay: sisu test --replay 1-0dcd89d9ab9c2483-0.0.1-t1-s0-r0-m100000 .

2 passed, 1 failed, 0 blocked
$ sisu test --replay 1-0dcd89d9ab9c2483-0.0.1-t1-s0-r0-m100000 .
info [task 1.1]: Aino books 18:00
info [task 1.2]: Eero books 18:00
test "tests/a turn is booked once" ... FAILED

test "tests/a turn is booked once" ... FAILED
panic: assertion failed: got[0] == (201, "booked\n")
    got[0] == (201, "booked\n")
    │
    (503, "Interrupted\n")
  at test "tests/a turn is booked once" (src/tests.sisu:24:5)
replay: sisu test --replay 1-0dcd89d9ab9c2483-0.0.1-t1-s0-r0-m100000 .

0 passed, 1 failed, 0 blocked
```

- Each call through a capability may fail with an error of its own
  error set, and with nothing else: a disk write may give
  `Interrupted`, as an `IoError` may, but never something the code
  could not be written to handle.
- The test of the first section assumed that every booking succeeds.
  Here a disk write failed, the service answered `503`, as it was
  written to, and the test, not the service, was wrong. The simulator
  test of the section "Two nodes and a crash" tries again after a
  `503`, and passes.
- The `replay:` command runs the failing test again with the same
  faults, as the second command does, so a failure the simulator found
  is never lost.

## Changing a handler while the server runs

`sisu run --watch` runs a program and watches its files. When a saved
file checks, the bodies that changed are swapped into the running
program as a new epoch, and each task takes the newest epoch when it
starts. A server whose connections are tasks thus answers the next
request with the new code:

<!-- file: examples/22-web-service/live.sisu -->
```sisu
# A handler changed while its server runs: each connection's task takes
# the newest code when it starts. Between two requests, the program saves
# its own next version, `v2.sisu`, as an editor would.
import http
from std.net import Url, UrlError

struct Greeter:
    pass

impl http.Handler for Greeter:
    def handle(self, request: http.Request) -> http.Response:
        return http.Response.text(200, "OK", greeting())

def greeting() -> str:
    return "Hei!"

## The server's answer to a request through `net`.
def ask(net: Net) -> String ! {UrlError, HttpError}:
    let body = http.get(net, Url.parse("http://localhost/")?)?
    return Ok(body.decode().or(String()))

def main(
    sys: System,
) -> ! {IoError, NetError, TaskError, UrlError, HttpError}:
    let net = Net.fake()
    let listener = net.listen(80)?
    return task.scope do s:
        s.spawn(() -> http.serve(listener, Greeter()))
        print("first: {ask(net)?}, epoch {live.epoch()}")
        sys.fs.write_text("live.sisu", sys.fs.read_to_string("v2.sisu")?)?
        print("second: {ask(net)?}, epoch {live.epoch()}")
        s.cancel()
        return Ok(())
```

In practice you edit and save the file in your editor. This example
saves its own next version, which differs in `greeting` alone, so that
the manual can check what happens:

<!-- output: examples/22-web-service/live.console -->
```console
$ sisu run --watch live.sisu
first: Hei!, epoch 0
watch: epoch 1 replaces `greeting`
second: Hei taas!, epoch 0
```

- The first request is answered with the old `greeting`, in epoch 0.
- The save makes epoch 1, which replaces `greeting` and nothing else.
- The second request's connection is a new task, which starts in epoch
  1 and answers with the new `greeting`. `main`, which was running all
  along, is still in epoch 0: a frame that is running keeps the code it
  started with.

A task that runs a long loop takes the newest epoch at
`live.boundary()`, which `Ticker.wait` of chapter 21 calls, so one turn
of a loop never mixes old code with new.

## A native build

`sisu build` checks the package and prepares it to run:

<!-- output: examples/22-web-service/service.build -->
```text
built the package `bookings` 0.1.0 (modules: 6)
```

`sisu build --native` also compiles it into an executable,
`build/debug/bookings`, which runs without `sisu`: `./build/debug/bookings
--port 8080` serves as `sisu run` does. `--release` builds it with the
release profile, into `build/release`. TLS is not in the standard
library yet, so a service that must speak HTTPS sits behind a proxy
that does.

## Exercise 22.1: a note that outlives a crash

Open
[`manual/exercises/22-web-service/notes.sisu`](../exercises/22-web-service/notes.sisu):

<!-- file: exercises/22-web-service/notes.sisu -->
```sisu
##! Exercise 22.1, a note that outlives a crash. Kind: fix. Level: 2.
##!
##! `keep` writes a note to a file of its own under `notes/`, and must
##! make it durable: once `keep` returns, a crash must not lose the note.
##! Its tests crash a node of a simulated system, and fail. Run
##! `sisu test notes.sisu`, read the reports, and fix `keep`, not the
##! tests, so that every test passes. The hints are in chapter 22 of the
##! tutorial, under "Exercise 22.1: a note that outlives a crash".

from std.sim import Sim

## Writes `text` to the file `notes/NAME`, durably.
def keep(fs: Fs, name: str, text: str) -> ! IoError:
    fs.create_dir("notes")?
    let path = Path.new("notes/{name}")
    fs.write_text(path, text)?
    fs.sync(path)?
    return Ok(())

test "a note outlives a crash":
    let sim = Sim.new()
    let laptop = sim.node("laptop")
    let fs = sim.system(laptop).fs
    keep(fs, "sauna", "heat it at six").expect("written")
    sim.crash(laptop)
    assert fs.read_to_string("notes/sauna") == Ok("heat it at six")

test "every note outlives it":
    let sim = Sim.new()
    let laptop = sim.node("laptop")
    let fs = sim.system(laptop).fs
    keep(fs, "lake", "too cold").expect("written")
    keep(fs, "stove", "more wood").expect("written")
    sim.crash(laptop)
    assert fs.list_dir("notes").map((paths) -> len(paths)) == Ok(2)
```

<!-- output: solutions/22-web-service/notes.console -->
```console
$ sisu test notes.sisu
test "a note outlives a crash" ... FAILED
test "every note outlives it" ... FAILED

test "a note outlives a crash" ... FAILED
panic: assertion failed: fs.read_to_string("notes/sauna") == Ok("heat it at six")
  left:  Err(NotFound(path="notes/sauna"))
  right: Ok("heat it at six")
  at test "a note outlives a crash" (notes.sisu:26:5)
replay: sisu test --replay 1-ed1d9d8a570fb83d-0.0.1-t0-s0-r0 notes.sisu

test "every note outlives it" ... FAILED
panic: assertion failed: fs.list_dir("notes").map((paths) -> len(paths)) == Ok(2)
  left:  Err(NotFound(path="notes"))
  right: Ok(2)
  at test "every note outlives it" (notes.sisu:35:5)
replay: sisu test --replay 1-ed1d9d8a570fb83d-0.0.1-t1-s0-r0 notes.sisu

0 passed, 2 failed, 0 blocked
```

<details>
<summary>Hint 1</summary>

The file's contents are synced, and the file is gone after the crash
all the same. A file can be found only through the names of the
directories on the way to it.

</details>

<details>
<summary>Hint 2</summary>

A sync of a directory makes its entries durable. `notes` is an entry
of the top directory, `"."`, and the note an entry of `notes`.

</details>

When every test passes, compare your answer with
[`manual/solutions/22-web-service/notes.sisu`](../solutions/22-web-service/notes.sisu).

## Exercise 22.2: a guest book's handler

Open
[`manual/exercises/22-web-service/guests.sisu`](../exercises/22-web-service/guests.sisu):

<!-- file: exercises/22-web-service/guests.sisu -->
```sisu
##! Exercise 22.2, a guest book's handler. Kind: complete. Level: 2.
##!
##! A sauna's guest book answers two requests: `POST /guests`, whose body
##! is a name, adds the name, and `GET /guests` gives the names as a
##! JSON list. Replace the two holes so that every test passes; the
##! second needs `std.json`. The tests hand the handler its requests
##! directly, as `http.serve` would. Run them with
##! `sisu test guests.sisu`. The hints are in chapter 22 of the tutorial,
##! under "Exercise 22.2: a guest book's handler".

import http

## The names written in the guest book so far.
struct GuestBook:
    names: Mutex[list[String]]

impl http.Handler for GuestBook:
    def handle(self, request: http.Request) -> http.Response:
        if request.target != "/guests":
            return http.Response.text(404, "Not Found", "no such page\n")
        if request.method == "POST":
            let name = request.body.decode().or(String())
            if name.trim().is_empty():
                return http.Response.text(400, "Bad Request", "a name?\n")
            return todo("add the name, and answer 201 Created")
        return todo("answer 200 with the names as JSON")

## A request of `method` for `/guests`, with `body`.
def request(method: str, body: str) -> http.Request:
    let target = "/guests".to_string()
    return http.Request(method.to_string(), target, [], body.encode())

test "names in the book":
    let book = GuestBook(names=Mutex([]))
    assert book.handle(request("POST", "Aino")).status == 201
    assert book.handle(request("POST", " Eero\n")).status == 201
    let all = book.handle(request("GET", ""))
    assert all.status == 200
    assert all.body == b"[\"Aino\",\"Eero\"]"

test "an empty name is not a guest":
    let book = GuestBook(names=Mutex([]))
    assert book.handle(request("POST", "  ")).status == 400
    assert book.handle(request("GET", "")).body == b"[]"
```

<!-- output: solutions/22-web-service/guests.console -->
```console
$ sisu test guests.sisu
in scope at the typed hole `todo("add the name, and answer 201 Created")`:
  self = GuestBook(names=Mutex([]))
  request = Request(method="POST", target="/guests", headers=[], body=b"Aino")
  name = "Aino"
in scope at the typed hole `todo("answer 200 with the names as JSON")`:
  self = GuestBook(names=Mutex([]))
  request = Request(method="GET", target="/guests", headers=[], body=b"")
test "names in the book" ... blocked by todo("add the name, and answer 201 Created")
test "an empty name is not a guest" ... blocked by todo("answer 200 with the names as JSON")
0 passed, 0 failed, 2 blocked
```

<details>
<summary>Hint 1</summary>

The names are behind a `Mutex`: `lock()` gives the guard through which
the list is changed or read. The second test writes a name with spaces
around it, which the book keeps without them.

</details>

<details>
<summary>Hint 2</summary>

`http.Response.text(status, reason, body)` and `http.Response.ok(body)`
make the two answers. `json.encode` writes a list of names as JSON,
once `std.json` is imported.

</details>

When every test passes, compare your answer with
[`manual/solutions/22-web-service/guests.sisu`](../solutions/22-web-service/guests.sisu).

## Challenge IV: a guarded service

The challenge puts a limit on the requests of each client and a cache
in front of a slow weather service, in three parts with a suite of
tests each. Open
[`manual/exercises/22-web-service/guard.sisu`](../exercises/22-web-service/guard.sisu):

<!-- file: exercises/22-web-service/guard.sisu -->
```sisu
##! Exercise 22.3, Challenge IV: a guarded service. Kind: challenge. Level: 3.
##!
##! A weather service answers in two seconds and must not be asked too
##! often. This challenge puts a limit and a cache in front of it, in
##! three parts with a suite of tests each. Solve the parts in order. The
##! tests run on the test clock, and `sisu test --explore 50 guard.sisu`
##! runs them under fifty orders of their tasks, which a solution must
##! pass as well as the default order. Run the tests with
##! `sisu test guard.sisu`.
##!
##! Part 1. `Limiter.allow(client)` says whether a request from `client`
##! may go on: each client may make `limit` requests in each window of
##! `window`, and a window starts at the client's first request after the
##! last window ended. Tasks share a limiter.
##!
##! Part 2. `Cache.forecast(service, town)` gives the forecast for
##! `town`, and asks `service` only when it keeps no answer for the town
##! younger than `fresh`; an error is not kept. Tasks that ask about one
##! town at the same time must make one request to the service between
##! them, not one each.
##!
##! Part 3. Tasks that ask about different towns at the same time must
##! not wait for each other: two towns at once take as long as one. The
##! last test checks, at every switch between tasks, that no two requests
##! for one town are in flight together.
##!
##! Add to the structs what they need. The hints are in chapter 22 of the
##! tutorial, under "Challenge IV: a guarded service".

import std.sim

error WeatherError:
    UnknownTown(name: String)

trait Weather:
    def forecast(self, town: str) -> String ! {WeatherError, TaskError}

## How many requests each client may make in each window of time.
struct Limiter:
    clock: Clock
    limit: int
    window: Duration

impl Limiter:
    ## A limiter of `limit` requests for each client in each `window`.
    def new(clock: Clock, limit: int, window: Duration) -> Limiter:
        return todo("Limiter.new")

    ## Whether a request from `client` may go on, counting it if it may.
    def allow(self, client: str) -> bool:
        return todo("allow")

## Forecasts kept for `fresh` after the service gave them.
struct Cache:
    clock: Clock
    fresh: Duration

impl Cache:
    ## A cache whose answers stay fresh for `fresh`.
    def new(clock: Clock, fresh: Duration) -> Cache:
        return todo("Cache.new")

    ## The forecast for `town`, asked of `service` only when no fresh
    ## answer is kept.
    def forecast[W: Weather + Sync](
        self, service: W, town: str
    ) -> String ! {WeatherError, TaskError}:
        return todo("forecast")

## A fake weather service that answers after two seconds on `clock`. It
## counts the requests for each town, and notes when a request for a
## town came while another for the same town was still in flight.
struct SlowWeather:
    clock: Clock
    asked: Mutex[dict[String, int]]
    flying: Mutex[set[String]]
    overlapped: Atomic[bool]

impl SlowWeather:
    def new(clock: Clock) -> SlowWeather:
        let asked = Mutex({})
        return SlowWeather(clock, asked, Mutex(set()), Atomic(False))

    ## How many requests for `town` the service got.
    def requests(self, town: str) -> int:
        return self.asked.lock().value.get(town.to_string()) ?? 0

impl Weather for SlowWeather:
    def forecast(self, town: str) -> String ! {WeatherError, TaskError}:
        self.asked.lock().value.entry(town.to_string()).or_insert(0) += 1
        if not self.flying.lock().value.add(town.to_string()):
            self.overlapped.store(True)
        let slept = self.clock.sleep(Duration.seconds(2))
        _ = self.flying.lock().value.remove(town.to_string())
        slept?
        match town:
            case "Oulu":
                return Ok("snow")
            case "Kemi":
                return Ok("sun")
            case _:
                return Err(UnknownTown(town.to_string()))

suite "part 1, a limit for each client":
    test "two requests in a window"(clock: TestClock):
        let limiter = Limiter.new(clock, 2, Duration.seconds(10))
        assert limiter.allow("aino") and limiter.allow("aino")
        assert not limiter.allow("aino")
        assert limiter.allow("eero")

    test "a new window"(clock: TestClock):
        let limiter = Limiter.new(clock, 1, Duration.seconds(10))
        assert limiter.allow("aino")
        clock.advance(Duration.seconds(9))
        assert not limiter.allow("aino")
        clock.advance(Duration.seconds(1))
        assert limiter.allow("aino")

    test "tasks share one limit"(clock: TestClock):
        let limiter = Limiter.new(clock, 2, Duration.seconds(10))
        let allowed = Atomic(0)
        task.scope do s:
            for _ in range(5):
                s.spawn do:
                    if limiter.allow("roni"):
                        _ = allowed.fetch_add(1)
        assert allowed.load() == 2

suite "part 2, a cache":
    test "a fresh answer is kept"(clock: TestClock):
        let service = SlowWeather.new(clock)
        let cache = Cache.new(clock, Duration.minutes(5))
        assert cache.forecast(service, "Oulu") == Ok("snow")
        clock.advance(Duration.minutes(4))
        assert cache.forecast(service, "Oulu") == Ok("snow")
        assert service.requests("Oulu") == 1

    test "a stale answer is asked for again"(clock: TestClock):
        let service = SlowWeather.new(clock)
        let cache = Cache.new(clock, Duration.minutes(5))
        assert cache.forecast(service, "Kemi") == Ok("sun")
        clock.advance(Duration.minutes(6))
        assert cache.forecast(service, "Kemi") == Ok("sun")
        assert service.requests("Kemi") == 2

    test "an error is not kept"(clock: TestClock):
        let service = SlowWeather.new(clock)
        let cache = Cache.new(clock, Duration.minutes(5))
        let first = cache.forecast(service, "Atlantis")
        assert first == Err(UnknownTown("Atlantis"))
        assert cache.forecast(service, "Atlantis").is_err()
        assert service.requests("Atlantis") == 2

    test "tasks asking at once ask the service once"(clock: TestClock):
        let service = SlowWeather.new(clock)
        let cache = Cache.new(clock, Duration.minutes(5))
        task.scope do s:
            for _ in range(3):
                s.spawn(() -> cache.forecast(service, "Oulu"))
        assert service.requests("Oulu") == 1

suite "part 3, towns side by side":
    test "two towns at once take as long as one"(clock: TestClock):
        let service = SlowWeather.new(clock)
        let cache = Cache.new(clock, Duration.minutes(5))
        let start = clock.now()
        task.scope do s:
            s.spawn(() -> cache.forecast(service, "Oulu"))
            s.spawn(() -> cache.forecast(service, "Kemi"))
        assert clock.now() - start == Duration.seconds(2)

    test "one request for a town at a time, at every switch"(clock: TestClock):
        let service = SlowWeather.new(clock)
        let cache = Cache.new(clock, Duration.seconds(1))
        let towns = ["Oulu", "Kemi", "Oulu", "Kemi", "Oulu"]
        let one_at_a_time = () -> not service.overlapped.load()
        sim.always("one request for a town at a time", one_at_a_time) do:
            task.scope do s:
                for town in towns:
                    s.spawn(() -> cache.forecast(service, town))
        assert service.requests("Oulu") >= 1
```

Before you change anything, every test is blocked:

<!-- output: solutions/22-web-service/guard.console -->
```console
$ sisu test guard.sisu
in scope at the typed hole `todo("Limiter.new")`:
  limit = 2
  window = Duration(nanos=10000000000)
in scope at the typed hole `todo("Limiter.new")`:
  limit = 1
  window = Duration(nanos=10000000000)
in scope at the typed hole `todo("Limiter.new")`:
  limit = 2
  window = Duration(nanos=10000000000)
in scope at the typed hole `todo("Cache.new")`:
  fresh = Duration(nanos=300000000000)
in scope at the typed hole `todo("Cache.new")`:
  fresh = Duration(nanos=300000000000)
in scope at the typed hole `todo("Cache.new")`:
  fresh = Duration(nanos=300000000000)
in scope at the typed hole `todo("Cache.new")`:
  fresh = Duration(nanos=300000000000)
in scope at the typed hole `todo("Cache.new")`:
  fresh = Duration(nanos=300000000000)
in scope at the typed hole `todo("Cache.new")`:
  fresh = Duration(nanos=1000000000)
test "part 1, a limit for each client/two requests in a window" ... blocked by todo("Limiter.new")
test "part 1, a limit for each client/a new window" ... blocked by todo("Limiter.new")
test "part 1, a limit for each client/tasks share one limit" ... blocked by todo("Limiter.new")
test "part 2, a cache/a fresh answer is kept" ... blocked by todo("Cache.new")
test "part 2, a cache/a stale answer is asked for again" ... blocked by todo("Cache.new")
test "part 2, a cache/an error is not kept" ... blocked by todo("Cache.new")
test "part 2, a cache/tasks asking at once ask the service once" ... blocked by todo("Cache.new")
test "part 3, towns side by side/two towns at once take as long as one" ... blocked by todo("Cache.new")
test "part 3, towns side by side/one request for a town at a time, at every switch" ... blocked by todo("Cache.new")
0 passed, 0 failed, 9 blocked
```

`SlowWeather` is the fake service of the tests, written by hand as
chapter 20's fakes were: it takes two seconds on the test clock, counts
what it is asked, and notes a request for a town that comes while
another for the same town is still in flight. The last test checks that
note at every switch between tasks with `sim.always(what, holds)`,
which stops the test the first time `holds` is false. When every test
passes, run `sisu test --explore 50 guard.sisu` as well: a cache that
works in one order of its tasks can fail in another.

<details>
<summary>Hint for part 1</summary>

A limiter remembers, for each client, when its window started and how
many requests it made since. Tasks share it, so that memory is behind a
`Mutex`, and one lock must cover the whole of `allow`, the reading and
the counting.

</details>

<details>
<summary>Hint for part 2</summary>

Keep each answer with the time it came. For the tasks that ask at once,
the first must still hold a lock while it waits for the service, so
that the others wait for its answer rather than ask again.

</details>

<details>
<summary>Hint for part 3</summary>

One lock over the whole cache makes every town wait for the town being
asked about. Give each town a lock of its own: a dict from towns to
shared mutexes, which a short lock on the dict finds or makes, and which
is held only while that town is asked about.

</details>

When every test passes, under fifty schedules too, compare your answer
with
[`manual/solutions/22-web-service/guard.sisu`](../solutions/22-web-service/guard.sisu).

## What you learned

- A service is a struct that implements `http.Handler`; `http.serve`
  answers each connection on a task of its own until it is cancelled.
- `main` narrows the network and the file system before the service
  gets them, and a `Mutex` makes concurrent requests take turns.
- `std.json` reads and writes the service's data from its structs;
  `fs.sync` of a file and of its directory makes a write durable.
- Tests start the server on `Net` fakes of their own; `Sim` runs nodes
  that crash and restart; `sisu test --sim --faults P` makes calls fail
  with errors of their declared sets, and every failure has a replay
  token.
- `sisu run --record LOG` writes a run's every call to the world, and
  `sisu replay LOG` runs it again from the log alone.
- `sisu run --watch` swaps saved code into a running program as an
  epoch, which each task takes when it starts or at `live.boundary()`.
- `sisu build --native` makes an executable that runs without `sisu`.

That is the end of Part IV. You can write a program that works with
files, the clock, the network and other programs through capabilities
it is given, runs work in tasks, and is tested deterministically, faults
and crashes included. Part V, which starts with chapter 23, goes
further: contracts, numbers in depth, compile-time code, purity and
data parallelism, systems programming, and code that lasts.
