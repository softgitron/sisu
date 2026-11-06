//! Tests in parallel (17, "Isolation and parallelism"). Every test runs in
//! an interpreter of its own, so the tests that take no `System` run at
//! the same time, on OS threads; the tests that take the real `System`
//! run one at a time after them, since they share the machine. Each test
//! is seeded as it is when the tests run one after another, nothing a
//! test that takes no `System` can reach is shared but the host's clock
//! and output, and each test's output is held and written in declaration
//! order: a run prints the same at any number of threads.
//!
//! The tests that read one value of a module's fixture (stage 1's R219,
//! decision 1202) run in one interpreter, in turn and in declaration
//! order, on one thread beside the others: the value is made before the
//! first of them that names it, each receives a `ref` to it, and it is
//! dropped after the last, failed or not.

use crate::host::{Host, IoErrorKind, NetFail, Poll};
use crate::{Interp, Options, RunReport, STACK_BYTES, TestResult, exit, on_big_stack};
use sisu_sir as sir;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

/// The most threads a run uses when it is not told how many.
pub const DEFAULT_JOBS: usize = 8;

/// One test's result, its report, and the output it held.
type Done = (TestResult, RunReport, String, String);

/// A test's view of a host it shares with tests on other threads: its own
/// output, held, and everything else through the lock.
struct Shared<'a, 'h> {
    inner: &'a Mutex<&'h mut dyn Host>,
    out: String,
    err: String,
    virtual_time: bool,
}

impl Shared<'_, '_> {
    fn with<R>(&self, f: impl FnOnce(&mut dyn Host) -> R) -> R {
        // A thread that panicked holding the lock left the host as it was.
        let mut g = match self.inner.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        f(&mut **g)
    }
}

impl Host for Shared<'_, '_> {
    fn stdout(&mut self, text: &str) {
        self.out.push_str(text)
    }
    fn stderr(&mut self, text: &str) {
        self.err.push_str(text)
    }
    fn take_held(&mut self) -> (String, String) {
        (std::mem::take(&mut self.out), std::mem::take(&mut self.err))
    }
    fn args(&self) -> Vec<String> {
        self.with(|h| h.args())
    }
    fn env_var(&self, name: &str) -> Option<String> {
        self.with(|h| h.env_var(name))
    }
    fn now(&self) -> i64 {
        self.with(|h| h.now())
    }
    fn read_file(&mut self, path: &str) -> Result<String, IoErrorKind> {
        self.with(|h| h.read_file(path))
    }
    fn write_file(&mut self, path: &str, text: &str) -> Result<(), IoErrorKind> {
        self.with(|h| h.write_file(path, text))
    }
    fn read_bytes(&mut self, path: &str) -> Result<Vec<u8>, IoErrorKind> {
        self.with(|h| h.read_bytes(path))
    }
    fn write_bytes(&mut self, path: &str, data: &[u8]) -> Result<(), IoErrorKind> {
        self.with(|h| h.write_bytes(path, data))
    }
    fn exists(&self, path: &str) -> bool {
        self.with(|h| h.exists(path))
    }
    fn now_nanos(&self) -> i128 {
        self.with(|h| h.now_nanos())
    }
    fn monotonic_nanos(&self) -> i128 {
        self.with(|h| h.monotonic_nanos())
    }
    fn virtual_time(&self) -> bool {
        self.virtual_time
    }
    fn sleep_nanos(&mut self, nanos: u64) {
        // A real wait holds no lock: the other tests go on meanwhile.
        if self.virtual_time {
            self.with(|h| h.sleep_nanos(nanos))
        } else {
            std::thread::sleep(std::time::Duration::from_nanos(nanos))
        }
    }
    fn metadata(&self, path: &str) -> Result<(i64, i64, bool), IoErrorKind> {
        self.with(|h| h.metadata(path))
    }
    fn remove_file(&mut self, path: &str) -> Result<(), IoErrorKind> {
        self.with(|h| h.remove_file(path))
    }
    fn rename_file(&mut self, from: &str, to: &str) -> Result<(), IoErrorKind> {
        self.with(|h| h.rename_file(from, to))
    }
    fn list_dir(&self, path: &str) -> Result<Vec<String>, IoErrorKind> {
        self.with(|h| h.list_dir(path))
    }
    fn create_dir(&mut self, path: &str) -> Result<(), IoErrorKind> {
        self.with(|h| h.create_dir(path))
    }
    fn sync_path(&mut self, path: &str) -> Result<(), IoErrorKind> {
        self.with(|h| h.sync_path(path))
    }
    fn random_bits(&mut self) -> u64 {
        self.with(|h| h.random_bits())
    }
    fn run_process(&mut self, program: &str, args: &[String]) -> Result<(i64, String, String), IoErrorKind> {
        self.with(|h| h.run_process(program, args))
    }
    fn tcp_connect(&mut self, host: &str, port: u16) -> Result<u64, NetFail> {
        self.with(|h| h.tcp_connect(host, port))
    }
    fn tcp_listen(&mut self, host: &str, port: u16) -> Result<(u64, u16), NetFail> {
        self.with(|h| h.tcp_listen(host, port))
    }
    fn tcp_accept(&mut self, listener: u64) -> Result<Poll<(u64, String)>, NetFail> {
        self.with(|h| h.tcp_accept(listener))
    }
    fn tcp_read(&mut self, conn: u64, max: usize) -> Result<Poll<Vec<u8>>, NetFail> {
        self.with(|h| h.tcp_read(conn, max))
    }
    fn tcp_write(&mut self, conn: u64, data: &[u8]) -> Result<Poll<usize>, NetFail> {
        self.with(|h| h.tcp_write(conn, data))
    }
    fn tcp_close(&mut self, handle: u64) {
        self.with(|h| h.tcp_close(handle))
    }
    fn read_line(&mut self) -> Option<String> {
        self.with(|h| h.read_line())
    }
}

/// Runs one test in a fresh interpreter over `host`.
fn run_one(
    prog: &sir::Program,
    host: &mut dyn Host,
    options: &Options,
    function: &str,
) -> (TestResult, RunReport) {
    let mut i = Interp::new(prog, host, options);
    if i.replay_case.is_none() {
        i.replay_case = options.saved_cases.get(function).cloned();
    }
    let r = i.run_test(function);
    let status = if matches!(r, TestResult::Passed) { exit::OK } else { exit::ERROR };
    let report = i.report(status);
    (r, report)
}

/// The module's fixtures a test takes: each fixture's function and the
/// parameter that receives its value (decision 1202).
type Fixtures = Vec<(String, u32)>;

/// Runs tests that share the values of a module's fixtures in one
/// interpreter over `host`, in the order given: each value made before
/// the first test that names it (its failure that test's), a `ref` to it
/// given to each, and the value dropped after the last of them, failed or
/// not (a drop that panics fails that test). Each test's output is its
/// own; the steps of its report count the group's so far.
fn run_group(
    prog: &sir::Program,
    host: &mut dyn Host,
    options: &Options,
    tests: &[(String, String)],
    fixtures: &[Fixtures],
    members: &[usize],
) -> Vec<(usize, Done)> {
    let mut last: Vec<(&str, usize)> = Vec::new();
    for (pos, &k) in members.iter().enumerate() {
        for (f, _) in &fixtures[k] {
            match last.iter_mut().find(|(g, _)| *g == f.as_str()) {
                Some(l) => l.1 = pos,
                None => last.push((f.as_str(), pos)),
            }
        }
    }
    let mut i = Interp::new(prog, host, options);
    let mut made: Vec<(&str, u32)> = Vec::new();
    let mut out = Vec::new();
    for (pos, &k) in members.iter().enumerate() {
        let mut given = Vec::new();
        let mut failed = None;
        for (f, param) in &fixtures[k] {
            let id = match made.iter().find(|(g, _)| *g == f.as_str()) {
                Some(&(_, id)) => id,
                None => match quietly(&mut i, options.fixture_lines.as_ref().map(|l| &l.0), f, |i| {
                    i.module_fixture(f)
                }) {
                    Ok(id) => {
                        made.push((f.as_str(), id));
                        id
                    }
                    Err(r) => {
                        failed = Some(r);
                        break;
                    }
                },
            };
            given.push((*param, i.fixture_ref(id)));
        }
        if options.replay_case.is_none() {
            i.replay_case = options.saved_cases.get(&tests[k].1).cloned();
        }
        let mut r = match failed {
            Some(r) => r,
            None => i.run_test_given(&tests[k].1, &given),
        };
        for (f, _) in last.iter().filter(|(_, at)| *at == pos) {
            let Some(at) = made.iter().position(|(g, _)| g == f) else { continue };
            let (function, id) = made.remove(at);
            let dropped = quietly(&mut i, options.fixture_lines.as_ref().map(|l| &l.1), function, |i| {
                i.drop_fixture(id, function)
            });
            if let Some(m) = dropped
                && !matches!(r, TestResult::Failed(_))
            {
                r = TestResult::Failed(m);
            }
        }
        let status = if matches!(r, TestResult::Passed) { exit::OK } else { exit::ERROR };
        let report = i.report(status);
        let (o, e) = i.host.take_held();
        out.push((k, (r, report, o, e)));
    }
    out
}

/// `f` on the interpreter, what it prints dropped unless `shown` (the
/// fixtures whose lines a test run alone shows, decision 1284) is none or
/// names `function`.
fn quietly<R>(
    i: &mut Interp<'_, '_>,
    shown: Option<&Vec<String>>,
    function: &str,
    f: impl FnOnce(&mut Interp<'_, '_>) -> R,
) -> R {
    if shown.is_none_or(|s| s.iter().any(|g| g == function)) {
        return f(i);
    }
    let (o, e) = i.host.take_held();
    let r = f(i);
    let _ = i.host.take_held();
    i.host.stdout(&o);
    i.host.stderr(&e);
    r
}

/// The units the tests run in: each test alone, but those that share a
/// module's fixture, joined (in declaration order) with every test that
/// shares one of theirs.
fn units(of: &[usize], fixtures: &[Fixtures]) -> Vec<Vec<usize>> {
    let mut units: Vec<Vec<usize>> = Vec::new();
    let mut unit_of: Vec<(String, usize)> = Vec::new();
    for &k in of {
        if fixtures[k].is_empty() {
            units.push(vec![k]);
            continue;
        }
        let mut found: Vec<usize> = fixtures[k]
            .iter()
            .filter_map(|(f, _)| unit_of.iter().find(|(g, _)| g == f).map(|(_, u)| *u))
            .collect();
        found.sort_unstable();
        found.dedup();
        let u = match found.first() {
            Some(&u) => u,
            None => {
                units.push(Vec::new());
                units.len() - 1
            }
        };
        for &other in found.iter().skip(1) {
            let moved = std::mem::take(&mut units[other]);
            units[u].extend(moved);
            for entry in unit_of.iter_mut().filter(|(_, v)| *v == other) {
                entry.1 = u;
            }
        }
        units[u].push(k);
        for (f, _) in &fixtures[k] {
            match unit_of.iter_mut().find(|(g, _)| g == f) {
                Some(entry) => entry.1 = u,
                None => unit_of.push((f.clone(), u)),
            }
        }
    }
    for u in &mut units {
        u.sort_unstable();
    }
    units.retain(|u| !u.is_empty());
    units
}

/// Whether the test function takes the real `System`, which it gets in
/// the `sim` profile too (decision 822): it runs alone.
fn takes_real_system(prog: &sir::Program, function: &str) -> bool {
    prog.functions.iter().any(|f| f.name == function && sir::takes_real_system(f))
}

/// Runs the tests at these indices (in declaration order); see the
/// module's comment.
pub(crate) fn run(
    prog: &sir::Program,
    host: &mut dyn Host,
    options: &Options,
    which: &[usize],
) -> Vec<(String, TestResult, RunReport)> {
    let tests: Vec<(String, String)> = which.iter().filter_map(|i| prog.tests.get(*i).cloned()).collect();
    let fixtures: Vec<Fixtures> = which
        .iter()
        .filter(|i| prog.tests.get(**i).is_some())
        .map(|i| {
            prog.test_fixtures
                .get(*i)
                .into_iter()
                .flatten()
                .filter_map(|x| Some((x.function.clone(), x.module?)))
                .collect()
        })
        .collect();
    let alone: Vec<usize> = (0..tests.len()).filter(|&k| !takes_real_system(prog, &tests[k].1)).collect();
    let parallel = units(&alone, &fixtures);
    let jobs = if options.jobs == 0 { DEFAULT_JOBS } else { options.jobs }.min(parallel.len()).max(1);
    let virtual_time = host.virtual_time();
    let done: Mutex<Vec<Option<Done>>> = Mutex::new(vec![None; tests.len()]);
    let next = AtomicUsize::new(0);
    {
        let shared: Mutex<&mut dyn Host> = Mutex::new(&mut *host);
        std::thread::scope(|s| {
            for _ in 0..jobs {
                let worker = || loop {
                    let at = next.fetch_add(1, Ordering::SeqCst);
                    let Some(unit) = parallel.get(at) else { break };
                    let mut h =
                        Shared { inner: &shared, out: String::new(), err: String::new(), virtual_time };
                    let slots = match unit.as_slice() {
                        &[k] if fixtures[k].is_empty() => {
                            let (r, report) = run_one(prog, &mut h, options, &tests[k].1);
                            vec![(k, (r, report, h.out, h.err))]
                        }
                        members => run_group(prog, &mut h, options, &tests, &fixtures, members),
                    };
                    let mut d = match done.lock() {
                        Ok(d) => d,
                        Err(p) => p.into_inner(),
                    };
                    for (k, slot) in slots {
                        d[k] = Some(slot);
                    }
                };
                // A thread that cannot start leaves its share to the others;
                // the tests left over run below.
                let _ = std::thread::Builder::new()
                    .name("sisu-test".into())
                    .stack_size(STACK_BYTES)
                    .spawn_scoped(s, worker);
            }
        });
    }
    let mut done = match done.into_inner() {
        Ok(d) => d,
        Err(p) => p.into_inner(),
    };
    // The tests that take the real `System`, one at a time, and any a
    // thread did not get to.
    let rest: Vec<usize> = (0..tests.len()).filter(|&k| done[k].is_none()).collect();
    let later: Vec<(usize, Done)> = on_big_stack(host, |host| {
        rest.iter()
            .flat_map(|&k| {
                let mut h = Held { inner: &mut *host, out: String::new(), err: String::new() };
                if fixtures[k].is_empty() {
                    let (r, report) = run_one(prog, &mut h, options, &tests[k].1);
                    vec![(k, (r, report, h.out, h.err))]
                } else {
                    run_group(prog, &mut h, options, &tests, &fixtures, &[k])
                }
            })
            .collect()
    })
    .unwrap_or_default();
    for (k, d) in later {
        done[k] = Some(d);
    }
    // Each test's output, in declaration order, then the results.
    let mut out = Vec::new();
    for (k, (name, _)) in tests.into_iter().enumerate() {
        let Some((r, mut report, o, e)) = done[k].take() else { continue };
        host.stdout(&o);
        host.stderr(&e);
        report.out = o;
        report.err = e;
        out.push((name, r, report));
    }
    out
}

/// A host whose standard output and error go, in the order written, to
/// one shared text (a transcript's runs, 17).
pub(crate) struct Merged<'a> {
    pub(crate) inner: &'a mut dyn Host,
    pub(crate) out: std::sync::Arc<Mutex<String>>,
}

impl Merged<'_> {
    fn push(&self, text: &str) {
        match self.out.lock() {
            Ok(mut o) => o.push_str(text),
            Err(p) => p.into_inner().push_str(text),
        }
    }
}

impl Host for Merged<'_> {
    fn stdout(&mut self, text: &str) {
        self.push(text)
    }
    fn stderr(&mut self, text: &str) {
        self.push(text)
    }
    fn args(&self) -> Vec<String> {
        self.inner.args()
    }
    fn env_var(&self, name: &str) -> Option<String> {
        self.inner.env_var(name)
    }
    fn now(&self) -> i64 {
        self.inner.now()
    }
    fn read_file(&mut self, path: &str) -> Result<String, IoErrorKind> {
        self.inner.read_file(path)
    }
    fn write_file(&mut self, path: &str, text: &str) -> Result<(), IoErrorKind> {
        self.inner.write_file(path, text)
    }
    fn exists(&self, path: &str) -> bool {
        self.inner.exists(path)
    }
    fn now_nanos(&self) -> i128 {
        self.inner.now_nanos()
    }
    fn monotonic_nanos(&self) -> i128 {
        self.inner.monotonic_nanos()
    }
    fn virtual_time(&self) -> bool {
        self.inner.virtual_time()
    }
    fn sleep_nanos(&mut self, nanos: u64) {
        self.inner.sleep_nanos(nanos)
    }
}

/// A host whose output is held, for a test that runs on the real one.
struct Held<'a> {
    inner: &'a mut dyn Host,
    out: String,
    err: String,
}

impl Host for Held<'_> {
    fn stdout(&mut self, text: &str) {
        self.out.push_str(text)
    }
    fn stderr(&mut self, text: &str) {
        self.err.push_str(text)
    }
    fn take_held(&mut self) -> (String, String) {
        (std::mem::take(&mut self.out), std::mem::take(&mut self.err))
    }
    fn args(&self) -> Vec<String> {
        self.inner.args()
    }
    fn env_var(&self, name: &str) -> Option<String> {
        self.inner.env_var(name)
    }
    fn now(&self) -> i64 {
        self.inner.now()
    }
    fn read_file(&mut self, path: &str) -> Result<String, IoErrorKind> {
        self.inner.read_file(path)
    }
    fn write_file(&mut self, path: &str, text: &str) -> Result<(), IoErrorKind> {
        self.inner.write_file(path, text)
    }
    fn read_bytes(&mut self, path: &str) -> Result<Vec<u8>, IoErrorKind> {
        self.inner.read_bytes(path)
    }
    fn write_bytes(&mut self, path: &str, data: &[u8]) -> Result<(), IoErrorKind> {
        self.inner.write_bytes(path, data)
    }
    fn exists(&self, path: &str) -> bool {
        self.inner.exists(path)
    }
    fn now_nanos(&self) -> i128 {
        self.inner.now_nanos()
    }
    fn monotonic_nanos(&self) -> i128 {
        self.inner.monotonic_nanos()
    }
    fn virtual_time(&self) -> bool {
        self.inner.virtual_time()
    }
    fn sleep_nanos(&mut self, nanos: u64) {
        self.inner.sleep_nanos(nanos)
    }
    fn metadata(&self, path: &str) -> Result<(i64, i64, bool), IoErrorKind> {
        self.inner.metadata(path)
    }
    fn remove_file(&mut self, path: &str) -> Result<(), IoErrorKind> {
        self.inner.remove_file(path)
    }
    fn rename_file(&mut self, from: &str, to: &str) -> Result<(), IoErrorKind> {
        self.inner.rename_file(from, to)
    }
    fn list_dir(&self, path: &str) -> Result<Vec<String>, IoErrorKind> {
        self.inner.list_dir(path)
    }
    fn create_dir(&mut self, path: &str) -> Result<(), IoErrorKind> {
        self.inner.create_dir(path)
    }
    fn sync_path(&mut self, path: &str) -> Result<(), IoErrorKind> {
        self.inner.sync_path(path)
    }
    fn random_bits(&mut self) -> u64 {
        self.inner.random_bits()
    }
    fn run_process(&mut self, program: &str, args: &[String]) -> Result<(i64, String, String), IoErrorKind> {
        self.inner.run_process(program, args)
    }
    fn tcp_connect(&mut self, host: &str, port: u16) -> Result<u64, NetFail> {
        self.inner.tcp_connect(host, port)
    }
    fn tcp_listen(&mut self, host: &str, port: u16) -> Result<(u64, u16), NetFail> {
        self.inner.tcp_listen(host, port)
    }
    fn tcp_accept(&mut self, listener: u64) -> Result<Poll<(u64, String)>, NetFail> {
        self.inner.tcp_accept(listener)
    }
    fn tcp_read(&mut self, conn: u64, max: usize) -> Result<Poll<Vec<u8>>, NetFail> {
        self.inner.tcp_read(conn, max)
    }
    fn tcp_write(&mut self, conn: u64, data: &[u8]) -> Result<Poll<usize>, NetFail> {
        self.inner.tcp_write(conn, data)
    }
    fn tcp_close(&mut self, handle: u64) {
        self.inner.tcp_close(handle)
    }
    fn read_line(&mut self) -> Option<String> {
        self.inner.read_line()
    }
}
