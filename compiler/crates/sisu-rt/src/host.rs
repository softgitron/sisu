//! The host's network and processes (the interpreter's `net_call` and
//! `proc.run` over its `StdHost`): sockets in a table the handles number,
//! made non-blocking, so a task whose socket is not ready waits a
//! millisecond on the host's clock and tries again while the others run;
//! a process runs to its end.

use crate::abi::{self, Intrinsic, Kind};
use crate::intrinsics::{Call, Part, write_err, write_parts};
use crate::mem::{self, List};
use crate::rt::{self, ty};
use crate::sched::{self, Attempt, Site, Wait, with};
use crate::world;
use sisu_diag::msg;
use std::cell::RefCell;
use std::io::{Read as _, Write as _};

enum Socket {
    Stream(std::net::TcpStream),
    Listener(std::net::TcpListener),
}

thread_local! {
    /// The program's sockets, by handle; a closed listener's slot is empty.
    static SOCKETS: RefCell<Vec<Option<Socket>>> = const { RefCell::new(Vec::new()) };
}

fn add(s: Socket) -> u64 {
    SOCKETS.with(|t| {
        let mut t = t.borrow_mut();
        t.push(Some(s));
        (t.len() - 1) as u64
    })
}

/// The interpreter's `NetFail`.
#[derive(Clone)]
enum Fail {
    Refused,
    NotAllowed,
    Closed,
    Other(String),
}

fn net_fail(e: &std::io::Error) -> Fail {
    use std::io::ErrorKind as K;
    match e.kind() {
        K::ConnectionRefused | K::NotFound => Fail::Refused,
        K::ConnectionReset | K::ConnectionAborted | K::BrokenPipe | K::UnexpectedEof => Fail::Closed,
        K::PermissionDenied => Fail::NotAllowed,
        _ => Fail::Other(e.to_string()),
    }
}

/// A failure as the capability log writes it (decision 1321).
fn fail_word(f: &Fail) -> String {
    match f {
        Fail::Refused => "refused".into(),
        Fail::NotAllowed => "not-allowed".into(),
        Fail::Closed => "closed".into(),
        Fail::Other(m) => format!("other {}", world::quote(m)),
    }
}

fn fail_read(f: &[String], i: usize) -> Fail {
    match world::word(f, i) {
        "refused" => Fail::Refused,
        "not-allowed" => Fail::NotAllowed,
        "closed" => Fail::Closed,
        _ => Fail::Other(world::word(f, i + 1).to_string()),
    }
}

/// A poll of a socket as the log writes it: `ok ready ...`, `ok wait`, or
/// the failure.
fn poll_word<T>(r: &Result<Option<T>, Fail>, ready: impl FnOnce(&T) -> String) -> String {
    match r {
        Ok(Some(v)) => format!("ok ready {}", ready(v)),
        Ok(None) => "ok wait".into(),
        Err(f) => format!("err {}", fail_word(f)),
    }
}

/// A poll read back from the log's fields.
fn poll_read<T>(f: &[String], ready: impl FnOnce(&[String]) -> T) -> Result<Option<T>, Fail> {
    match (world::word(f, 0), world::word(f, 1)) {
        ("ok", "ready") => Ok(Some(ready(&f[2.min(f.len())..]))),
        ("ok", _) => Ok(None),
        _ => Err(fail_read(f, 1)),
    }
}

fn not_ready(e: &std::io::Error) -> bool {
    matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::Interrupted)
}

/// Runs `f` on the socket `h`; `Closed` when there is none.
fn with_socket<R>(h: u64, f: impl FnOnce(&mut Socket) -> Result<R, Fail>) -> Result<R, Fail> {
    SOCKETS.with(|t| match t.borrow_mut().get_mut(h as usize).and_then(Option::as_mut) {
        Some(s) => f(s),
        None => Err(Fail::Closed),
    })
}

/// Writes `Err(NetError)` at the result, about `address`.
///
/// # Safety
/// The result's error must be able to hold a `NetError`.
unsafe fn write_fail(c: &Call<'_>, f: Fail, address: &str) {
    let (variant, text) = match f {
        Fail::Refused => ("Refused", Some(address.to_string())),
        Fail::NotAllowed => ("NotAllowed", Some(address.to_string())),
        Fail::Closed => ("Closed", None),
        Fail::Other(m) => ("Other", Some(m)),
    };
    let parts: Vec<Part<'_>> = text.iter().map(|t| Part::Text(t.as_bytes())).collect();
    // SAFETY: as the caller guarantees.
    unsafe { write_err(c, "NetError", variant, &parts) }
}

/// The fields of the struct `t`.
fn fields(t: u32) -> &'static [abi::Field] {
    match &ty(t).kind {
        Kind::Struct { fields, .. } => fields,
        _ => rt::internal("a host value that is not a struct"),
    }
}

/// The `Ok` payload's type of the result.
fn ok_type(c: &Call<'_>) -> u32 {
    match ty(c.ret_ty).kind {
        Kind::Result(ok, _) => ok,
        _ => rt::internal("a host result that is not a result"),
    }
}

/// A `Box[dyn T]` of a host socket, its object of type `object`: the word.
fn boxed_socket(object: &str, h: u64) -> u64 {
    let Some(t) = crate::caps::opaque_type(object) else {
        rt::internal("a host socket whose type the program does not name")
    };
    let value = mem::alloc(8);
    let words = mem::alloc(16);
    // SAFETY: the cells have room for a word and for a `dyn`'s two words.
    unsafe {
        mem::set_word(value, 0, h);
        mem::set_word(words, 0, u64::from(t));
        mem::set_word(words, 8, value as u64);
    }
    words as u64
}

/// Writes `Ok(Conn(stream, peer))` at the result.
///
/// # Safety
/// The result must be a `Result` of a `Conn`.
unsafe fn write_conn(c: &Call<'_>, h: u64, peer: &str) {
    let stream = boxed_socket("HostStream", h);
    // SAFETY: as the caller guarantees.
    unsafe {
        mem::set_word(c.ret, 0, 0);
        write_parts(c.ret.add(8), fields(ok_type(c)), &[Part::Word(stream), Part::Text(peer.as_bytes())]);
    }
}

/// A port number; one beyond `u16` panics as the interpreter's does.
fn port(n: i64) -> Result<u16, u8> {
    u16::try_from(n).map_err(|_| rt::raise_msg(msg::PANIC_BAD_PORT, &[("port", n.to_string())]))
}

/// Whether the running task is cancelled: then the result is
/// `Err(Cancelled)`.
///
/// # Safety
/// The result's error must be able to hold a `TaskError`.
unsafe fn cancelled(c: &Call<'_>) -> bool {
    if with(|s| s.cancelled_now()) {
        // SAFETY: as the caller guarantees.
        unsafe { crate::tasks::write_task_error(c.ret, c.ret_ty, "Cancelled", None) };
        return true;
    }
    false
}

/// Waits a millisecond on the host's clock before the next try.
fn wait_io(site: Site) -> Attempt {
    with(|s| {
        let at = s.clock_now(0) + 1_000_000;
        s.block_on(Wait::Io, &[], Some((0, at)), site);
    });
    Attempt::Wait
}

/// The intrinsics that may wait for a socket, on the task's stack.
pub(crate) fn handles(i: Intrinsic) -> bool {
    matches!(i, Intrinsic::HostNetAccept | Intrinsic::HostNetRead | Intrinsic::HostNetWrite)
}

/// A call of one of [`handles`]'s intrinsics.
///
/// # Safety
/// The arguments are of the types the descriptor gives.
pub(crate) unsafe fn call(i: Intrinsic, c: &Call<'_>) -> u8 {
    let site = Site::current();
    // SAFETY: each reads its arguments as their types say.
    unsafe {
        let h = c.word(0);
        match i {
            Intrinsic::HostNetAccept => sched::blocking(|| {
                if cancelled(c) {
                    return Attempt::Done(abi::OK);
                }
                let real = || {
                    with_socket(h, |s| match s {
                        Socket::Listener(l) => match l.accept() {
                            Ok((s, peer)) => {
                                s.set_nonblocking(true).map_err(|e| net_fail(&e))?;
                                Ok(Some((s, peer.to_string())))
                            }
                            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(None),
                            Err(e) => Err(net_fail(&e)),
                        },
                        Socket::Stream(_) => Err(Fail::Closed),
                    })
                    .map(|o| o.map(|(s, peer)| (add(Socket::Stream(s)), peer)))
                };
                let r = world::call(
                    "net",
                    format!("tcp_accept {h}"),
                    real,
                    |r| poll_word(r, |(c, peer)| format!("{c} {}", world::quote(peer))),
                    |f| poll_read(f, |f| (world::int(f, 0), world::word(f, 1).to_string())),
                    || Err(Fail::Closed),
                );
                match r {
                    Ok(Some((conn, peer))) => {
                        write_conn(c, conn, &peer);
                        Attempt::Done(abi::OK)
                    }
                    Ok(None) => wait_io(site),
                    Err(f) => {
                        write_fail(c, f, "");
                        Attempt::Done(abi::OK)
                    }
                }
            }),
            Intrinsic::HostNetRead => {
                let asked = usize::try_from(c.int(1)).unwrap_or(0);
                let max = asked.clamp(1, 1 << 20);
                sched::blocking(|| {
                    if cancelled(c) {
                        return Attempt::Done(abi::OK);
                    }
                    let real = || {
                        with_socket(h, |s| match s {
                            Socket::Stream(s) => {
                                let mut buf = vec![0u8; max];
                                match s.read(&mut buf) {
                                    Ok(n) => {
                                        buf.truncate(n);
                                        Ok(Some(buf))
                                    }
                                    Err(e) if not_ready(&e) => Ok(None),
                                    Err(e) => Err(net_fail(&e)),
                                }
                            }
                            Socket::Listener(_) => Err(Fail::Closed),
                        })
                    };
                    let r = world::call(
                        "net",
                        format!("tcp_read {h} {asked}"),
                        real,
                        |r| poll_word(r, |b| world::quote(&world::hex(b))),
                        |f| poll_read(f, |f| world::unhex(world::word(f, 0)).unwrap_or_default()),
                        || Err(Fail::Closed),
                    );
                    match r {
                        Ok(Some(b)) => {
                            mem::set_word(c.ret, 0, 0);
                            crate::bytes::write(c.ret.add(8), &b);
                            Attempt::Done(abi::OK)
                        }
                        Ok(None) => wait_io(site),
                        Err(f) => {
                            write_fail(c, f, "");
                            Attempt::Done(abi::OK)
                        }
                    }
                })
            }
            _ => {
                // What was not sent is sent on the next try.
                let data = crate::bytes::arg(c, 1).to_vec();
                let mut sent = 0;
                sched::blocking(|| {
                    if cancelled(c) {
                        return Attempt::Done(abi::OK);
                    }
                    let left = &data[sent..];
                    let real = || {
                        with_socket(h, |s| match s {
                            Socket::Stream(s) => match s.write(left) {
                                Ok(n) => Ok(Some(n)),
                                Err(e) if not_ready(&e) => Ok(None),
                                Err(e) => Err(net_fail(&e)),
                            },
                            Socket::Listener(_) => Err(Fail::Closed),
                        })
                    };
                    let r = world::call(
                        "net",
                        format!("tcp_write {h} #{:016x}", world::content_hash(left)),
                        real,
                        |r| poll_word(r, usize::to_string),
                        |f| poll_read(f, |f| world::int(f, 0)),
                        || Err(Fail::Closed),
                    );
                    match r {
                        Ok(Some(n)) if sent + n >= data.len() => {
                            mem::set_word(c.ret, 0, 0);
                            Attempt::Done(abi::OK)
                        }
                        Ok(Some(n)) => {
                            sent += n;
                            wait_io(site)
                        }
                        Ok(None) => wait_io(site),
                        Err(f) => {
                            write_fail(c, f, "");
                            Attempt::Done(abi::OK)
                        }
                    }
                })
            }
        }
    }
}

/// The host's operations that do not wait for a socket: connecting (which
/// the host does at once), listening, closing, and running a process. On
/// the system stack; `None` for any other intrinsic.
///
/// # Safety
/// The arguments are of the types the descriptor gives.
pub(crate) unsafe fn call_system(i: Intrinsic, c: &Call<'_>) -> Option<u8> {
    // SAFETY: each reads its arguments as their types say.
    unsafe {
        match i {
            Intrinsic::HostNetConnect => {
                if cancelled(c) {
                    return Some(abi::OK);
                }
                let host = String::from_utf8_lossy(c.text(1)).into_owned();
                let p = match port(c.int(2)) {
                    Ok(p) => p,
                    Err(st) => return Some(st),
                };
                let address = format!("{host}:{p}");
                let r = world::call(
                    "net",
                    format!("tcp_connect {} {p}", world::quote(&host)),
                    || {
                        std::net::TcpStream::connect((host.as_str(), p))
                            .and_then(|s| s.set_nonblocking(true).map(|()| s))
                            .map(|s| add(Socket::Stream(s)))
                            .map_err(|e| net_fail(&e))
                    },
                    |r| match r {
                        Ok(h) => format!("ok {h}"),
                        Err(f) => format!("err {}", fail_word(f)),
                    },
                    |f| if world::word(f, 0) == "ok" { Ok(world::int(f, 1)) } else { Err(fail_read(f, 1)) },
                    || Err(Fail::Closed),
                );
                match r {
                    Ok(h) => write_conn(c, h, &address),
                    Err(f) => write_fail(c, f, &address),
                }
            }
            Intrinsic::HostNetListen => {
                let p = match port(c.int(1)) {
                    Ok(p) => p,
                    Err(st) => return Some(st),
                };
                let host = String::from_utf8_lossy(c.text(2)).into_owned();
                let address = format!("{host}:{p}");
                let r = world::call(
                    "net",
                    format!("tcp_listen {} {p}", world::quote(&host)),
                    || {
                        std::net::TcpListener::bind((host.as_str(), p))
                            .and_then(|l| l.set_nonblocking(true).map(|()| l))
                            .map(|l| {
                                let at = l.local_addr().map(|a| a.port()).unwrap_or(p);
                                (add(Socket::Listener(l)), at)
                            })
                            .map_err(|e| net_fail(&e))
                    },
                    |r| match r {
                        Ok((h, at)) => format!("ok {h} {at}"),
                        Err(f) => format!("err {}", fail_word(f)),
                    },
                    |f| {
                        if world::word(f, 0) == "ok" {
                            Ok((world::int(f, 1), world::int(f, 2)))
                        } else {
                            Err(fail_read(f, 1))
                        }
                    },
                    || Err(Fail::Closed),
                );
                match r {
                    Ok((handle, at)) => {
                        let inner = boxed_socket("HostListener", handle);
                        mem::set_word(c.ret, 0, 0);
                        write_parts(
                            c.ret.add(8),
                            fields(ok_type(c)),
                            &[Part::Word(inner), Part::Word(u64::from(at))],
                        );
                    }
                    Err(f) => write_fail(c, f, &address),
                }
            }
            Intrinsic::HostNetClose => {
                let h = c.word(0);
                let real = || {
                    SOCKETS.with(|t| match t.borrow_mut().get_mut(h as usize) {
                        Some(Some(Socket::Stream(s))) => {
                            let _ = s.shutdown(std::net::Shutdown::Write);
                        }
                        Some(slot @ Some(Socket::Listener(_))) => *slot = None,
                        _ => {}
                    })
                };
                world::call("net", format!("tcp_close {h}"), real, |()| String::new(), |_| (), || ());
            }
            Intrinsic::HostProcRun => proc_run(c),
            _ => return None,
        }
    }
    Some(abi::OK)
}

/// `Proc.run` on the host: the program with its arguments to its end, with
/// no input.
///
/// # Safety
/// The arguments are `(HostProc, str, list[str])` and the result a
/// `ProcOutput ! ProcError`.
unsafe fn proc_run(c: &Call<'_>) {
    // SAFETY: as the caller guarantees.
    unsafe {
        let program = String::from_utf8_lossy(c.text(1)).into_owned();
        let (p, t) = c.place(2);
        let Kind::List(e) = ty(t).kind else { rt::internal("`Proc.run`'s arguments are not a list") };
        let l = mem::at::<List>(p);
        let size = ty(e).size;
        let words: Vec<String> = (0..l.len)
            .map(|k| String::from_utf8_lossy(mem::str_at(l.at(k, size)).bytes()).into_owned())
            .collect();
        let real = || {
            std::process::Command::new(&program)
                .args(&words)
                .stdin(std::process::Stdio::null())
                .output()
                .map(|o| {
                    let status = i64::from(o.status.code().unwrap_or(-1));
                    let out = String::from_utf8_lossy(&o.stdout).into_owned();
                    (status, out, String::from_utf8_lossy(&o.stderr).into_owned())
                })
                .map_err(|e| crate::intrinsics::io_kind(&e))
        };
        let shown: Vec<String> = words.iter().map(|w| world::quote(w)).collect();
        let what = format!("run_process {} {} {}", world::quote(&program), words.len(), shown.join(" "));
        let r = world::call(
            "proc",
            what.trim_end().to_string(),
            real,
            |r| match r {
                Ok((status, out, err)) => format!("ok {status} {} {}", world::quote(out), world::quote(err)),
                Err(k) => format!("err {}", world::io_word(k)),
            },
            |f| {
                if world::word(f, 0) == "ok" {
                    Ok((world::int(f, 1), world::word(f, 2).to_string(), world::word(f, 3).to_string()))
                } else {
                    Err(world::io_read(f, 1))
                }
            },
            world::io_gone,
        );
        match r {
            Ok((status, out, err)) => {
                mem::set_word(c.ret, 0, 0);
                write_parts(
                    c.ret.add(8),
                    fields(ok_type(c)),
                    &[Part::Word(status as u64), Part::Text(out.as_bytes()), Part::Text(err.as_bytes())],
                );
            }
            Err(k) => {
                use crate::intrinsics::IoKind as K;
                let (variant, text) = match k {
                    K::NotFound => ("NotFound", program),
                    K::PermissionDenied => ("PermissionDenied", program),
                    K::AlreadyExists => ("Other", "AlreadyExists".to_string()),
                    K::InvalidData => ("Other", "InvalidData".to_string()),
                    K::Interrupted => ("Other", "Interrupted".to_string()),
                    K::Other(m) => ("Other", m),
                };
                write_err(c, "ProcError", variant, &[Part::Text(text.as_bytes())]);
            }
        }
    }
}
