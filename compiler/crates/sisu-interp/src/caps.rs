//! Capabilities (06, "Capabilities"): a handle (`Fs`, `Clock`, ...) is a
//! cell of the heap that holds its object as a `dyn` of the object's type,
//! which every copy of the handle reaches; a `Cap[T]` is the same cell
//! holding the `T`, and a `Lent` is the handle itself. The root objects of
//! a `System` are the host's (`HostFs`, `HostClock`, ...), whose operations
//! are the intrinsics `host.*` over [`crate::Host`]. The run's test clock is
//! a virtual clock of the scheduler, made on first use.
//!
//! A handle is `Copy`, and each copy is counted (decision 237): a copy into
//! a place, an aggregate, a closure's environment, a function's argument or
//! an argument an intrinsic keeps adds a count, a drop takes one, and the
//! last one's drop drops the object (running its `Drop`) and empties the
//! cell. A `Lent` counts nothing.

use crate::value::{DynVal, HEAP, HeapCell, Pointer, Value};
use crate::{Interp, IoErrorKind, Unwind, ice};
use sisu_sir as sir;
use std::rc::Rc;

impl Interp<'_, '_> {
    /// A new cell of the heap holding `value` for good: a capability's
    /// object or a `Cap`'s value. Every task may reach it (its type is
    /// `Sync`, checked where it is made), so it counts as crossing.
    fn cap_cell(&mut self, value: Value, label: &str) -> u32 {
        self.serial += 1;
        let id = self.heap.len() as u32;
        self.heap.push(HeapCell {
            value,
            strong: 1,
            weak: 0,
            serial: self.serial,
            atomic: true,
            creator: self.sched.cur,
            crossed: false,
            label: Some(Rc::from(label)),
            site: None,
        });
        id
    }

    /// Adds a count to every handle and `Cap` a value of type `ty` (when
    /// it is known) holds: a copy of it was made (decision 237). A `Lent`
    /// holds its handle without a count.
    pub(crate) fn retain_caps(&mut self, v: &Value, ty: Option<&sir::Type>) {
        if !holds_cap(v) || matches!(ty, Some(sir::Type::Named(n, _)) if n == "Lent") {
            return;
        }
        match v {
            Value::Cap(id) => {
                if let Some(c) = self.heap.get_mut(*id as usize) {
                    c.strong += 1;
                }
            }
            Value::Struct(a) | Value::Variant(a) => {
                let variant = matches!(v, Value::Variant(_)).then_some(a.variant);
                for (i, f) in a.fields.iter().enumerate() {
                    let ft = ty.and_then(|t| {
                        sir::types::field_ty(&|n| self.types.get(n).copied(), t, variant, i as u32)
                    });
                    self.retain_caps(f, ft.as_ref());
                }
            }
            Value::Tuple(xs) => {
                for (i, x) in xs.iter().enumerate() {
                    let et = match ty {
                        Some(sir::Type::Tuple(ts)) => ts.get(i).cloned(),
                        _ => None,
                    };
                    self.retain_caps(x, et.as_ref());
                }
            }
            // A copy of a function value copies its environment (decision
            // 248).
            Value::Closure(c) => {
                let tys = self.env_types(c);
                for (i, x) in c.env.iter().enumerate() {
                    self.retain_caps(x, tys.get(i));
                }
            }
            _ => {}
        }
    }

    /// The types of what a closure's environment holds, as its function's
    /// environment parameter declares them.
    pub(crate) fn env_types(&self, c: &crate::value::ClosureVal) -> Vec<sir::Type> {
        let Ok(fi) = self.fn_index(&c.name) else { return Vec::new() };
        match self.func(fi).and_then(|f| f.locals.get(1)).map(|l| l.ty.subst(&c.tys)) {
            Some(sir::Type::Tuple(ts)) => ts,
            _ => Vec::new(),
        }
    }

    /// Takes a count from a handle's cell: the last one drops the object
    /// (or the `Cap`'s value), queued as any drop is (decision 237).
    pub(crate) fn release_cap(&mut self, id: u32) {
        let Some(c) = self.heap.get_mut(id as usize) else { return };
        if c.strong == 0 {
            return;
        }
        c.strong -= 1;
        if c.strong > 0 {
            return;
        }
        let v = std::mem::replace(&mut c.value, Value::Moved);
        let serial = c.serial;
        self.mon_heap_freed(serial, id);
        self.queue_drop(v, None);
    }

    /// A handle on one of the host's root objects.
    fn host_handle(&mut self, ty: &str, handle: &str, value: Value) -> Value {
        let ty = sir::Type::Named(ty.into(), Vec::new());
        Value::Cap(self.cap_cell(Value::Dyn(Box::new(DynVal { ty, value })), handle))
    }

    /// The `System` the runtime gives `main`: the host's root objects.
    pub(crate) fn system(&mut self) -> Value {
        let fs = self.host_handle("HostFs", "Fs", Value::Unit);
        let net = self.host_handle("HostNet", "Net", Value::Unit);
        let clock = self.host_handle("HostClock", "Clock", Value::Clock(0));
        let rand = self.host_handle("HostRand", "Rand", Value::Unit);
        let env = self.host_handle("HostEnv", "Env", Value::Unit);
        let proc = self.host_handle("HostProc", "Proc", Value::Unit);
        let stdin = self.host_handle("HostStdin", "Stdin", Value::Unit);
        Value::Struct(Value::adt("System", 0, vec![fs, net, clock, rand, env, proc, stdin]))
    }

    fn cap_id(&self, v: &Value) -> Result<u32, Unwind> {
        match self.target(v)? {
            Value::Cap(id) => Ok(*id),
            other => Err(ice(format!("expected a capability, found {other:?}"))),
        }
    }

    /// The run's test clock (`Clock.fake`), made at the epoch on first use.
    fn fake_clock(&mut self) -> u32 {
        if let Some(c) = self.sched.fake_clock {
            return c;
        }
        let c = self.sched.clocks.len() as u32;
        self.sched.clocks.push(crate::sched::RtClock { virtual_: true, now: 0 });
        self.sched.fake_clock = Some(c);
        c
    }

    /// The intrinsics of capabilities, `Cap`, `Lent`, clocks and times;
    /// `None` for any other.
    pub(crate) fn cap_intrinsic(
        &mut self,
        name: &str,
        tys: &[sir::Type],
        args: &mut [Value],
    ) -> Option<Result<Value, Unwind>> {
        let take = |args: &mut [Value], i: usize| -> Result<Value, Unwind> {
            args.get_mut(i)
                .map(|v| std::mem::replace(v, Value::Moved))
                .ok_or_else(|| ice(format!("missing argument {i}")))
        };
        let r = match name {
            // The handle takes the object over, with its type for dispatch.
            "cap.new" => (|| {
                let ty = tys.first().cloned().ok_or_else(|| ice("cap.new without a type"))?;
                let handle = match tys.get(1) {
                    Some(sir::Type::Named(n, _)) => n.clone(),
                    _ => "Cap".to_string(),
                };
                let value = take(args, 0)?;
                Ok(Value::Cap(self.cap_cell(Value::Dyn(Box::new(DynVal { ty, value })), &handle)))
            })(),
            "Cap.new" => take(args, 0).map(|v| Value::Cap(self.cap_cell(v, "Cap"))),
            "cap.ops" | "Cap.get" => (|| {
                let id = self.cap_id(args.first().ok_or_else(|| ice("no handle"))?)?;
                let serial = self.heap.get(id as usize).map(|c| c.serial).unwrap_or(0);
                Ok(Value::Ref(Rc::new(Pointer {
                    task: 0,
                    frame: HEAP,
                    serial,
                    local: id,
                    path: Vec::new(),
                    tag: 0,
                })))
            })(),
            // A lent capability is its handle.
            "cap.lend" | "Lent.target" => {
                (|| Ok(self.target(args.first().ok_or_else(|| ice("no handle"))?)?.clone()))()
            }
            "System.assemble" => Ok(Value::Struct(Value::adt(
                "System",
                0,
                args.iter_mut().map(|v| std::mem::replace(v, Value::Moved)).collect(),
            ))),
            "Clock.fake" => Ok(Value::Clock(self.fake_clock())),
            "TestClock.advance" | "TestClock.set" => (|| {
                let c = self.clock_index(&args[0])?;
                let n = match name {
                    "TestClock.advance" => self.duration_nanos(&args[1])?,
                    _ => i128::from(self.datetime(&args[1])?),
                };
                if let Some(k) = self.sched.clocks.get_mut(c as usize) {
                    k.now = if name == "TestClock.advance" { k.now + n } else { n };
                }
                self.wake_due_timers();
                Ok(Value::Unit)
            })(),
            "clock.now" => (|| {
                let c = self.clock_index(&args[0])?;
                let t = self.clock_now(c);
                Ok(Value::DateTime(i64::try_from(t).unwrap_or(i64::MAX)))
            })(),
            "DateTime.from_unix_nanos" => (|| Ok(Value::DateTime(self.int(&args[0])?)))(),
            "DateTime.unix_nanos" => (|| Ok(Value::Int(self.datetime(&args[0])?)))(),
            n => self.host_call(n.strip_prefix("host.")?, args),
        };
        Some(r)
    }

    fn clock_index(&self, v: &Value) -> Result<u32, Unwind> {
        match self.target(v)? {
            Value::Clock(i) => Ok(*i),
            other => Err(ice(format!("expected a clock, found {other:?}"))),
        }
    }

    fn datetime(&self, v: &Value) -> Result<i64, Unwind> {
        match self.target(v)? {
            Value::DateTime(t) => Ok(*t),
            other => Err(ice(format!("expected a DateTime, found {other:?}"))),
        }
    }

    /// The operations of the host's root objects (their contracts are the
    /// prelude's operations traits).
    fn host_call(&mut self, op: &str, args: &mut [Value]) -> Result<Value, Unwind> {
        let arg_text = |me: &Self, i: usize| -> Result<String, Unwind> {
            me.text(args.get(i).ok_or_else(|| ice(format!("missing argument {i}")))?).map(str::to_string)
        };
        match op {
            "fs.read_to_string" => {
                let path = arg_text(self, 1)?;
                Ok(match self.host.read_file(&path) {
                    Ok(t) => Value::ok(Value::String(t)),
                    Err(k) => Value::err(self.io_error(k, &path)),
                })
            }
            "fs.read" => {
                let path = arg_text(self, 1)?;
                Ok(match self.host.read_bytes(&path) {
                    Ok(b) => Value::ok(Value::Bytes(Rc::from(b))),
                    Err(k) => Value::err(self.io_error(k, &path)),
                })
            }
            "fs.write" => {
                let path = arg_text(self, 1)?;
                let data = match self.target(args.get(2).ok_or_else(|| ice("missing data"))?)? {
                    Value::Bytes(b) => b.clone(),
                    other => return Err(ice(format!("Fs.write of {other:?}"))),
                };
                let r = self.host.write_bytes(&path, &data);
                Ok(self.unit_result(r, &path))
            }
            "fs.write_text" => {
                let path = arg_text(self, 1)?;
                let text = arg_text(self, 2)?;
                let r = self.host.write_file(&path, &text);
                Ok(self.unit_result(r, &path))
            }
            "fs.exists" => {
                let path = arg_text(self, 1)?;
                Ok(Value::Bool(self.host.exists(&path)))
            }
            "fs.metadata" => {
                let path = arg_text(self, 1)?;
                Ok(match self.host.metadata(&path) {
                    Ok((len, modified, dir)) => Value::ok(Value::Struct(Value::adt(
                        "Metadata",
                        0,
                        vec![Value::Int(len), Value::DateTime(modified), Value::Bool(dir)],
                    ))),
                    Err(k) => Value::err(self.io_error(k, &path)),
                })
            }
            "fs.remove" => {
                let path = arg_text(self, 1)?;
                let r = self.host.remove_file(&path);
                Ok(self.unit_result(r, &path))
            }
            // A failure names the source when it is not there (which the
            // host is asked after a `NotFound`), else the target.
            "fs.rename" => {
                let (from, to) = (arg_text(self, 1)?, arg_text(self, 2)?);
                let r = self.host.rename_file(&from, &to);
                let named =
                    if r == Err(IoErrorKind::NotFound) && !self.host.exists(&from) { from } else { to };
                Ok(self.unit_result(r, &named))
            }
            "fs.list_dir" => {
                let path = arg_text(self, 1)?;
                Ok(match self.host.list_dir(&path) {
                    Ok(names) => {
                        let dir = path.trim_end_matches('/');
                        let items = names
                            .iter()
                            .map(|n| {
                                let p = if dir.is_empty() || dir == "." {
                                    n.clone()
                                } else {
                                    format!("{dir}/{n}")
                                };
                                Value::Path(Rc::from(p.as_str()))
                            })
                            .collect();
                        Value::ok(Value::List(items))
                    }
                    Err(k) => Value::err(self.io_error(k, &path)),
                })
            }
            "fs.create_dir" => {
                let path = arg_text(self, 1)?;
                let r = self.host.create_dir(&path);
                Ok(self.unit_result(r, &path))
            }
            "fs.sync" => {
                let path = arg_text(self, 1)?;
                let r = self.host.sync_path(&path);
                Ok(self.unit_result(r, &path))
            }
            "env.args" => Ok(Value::List(self.host.args().iter().map(|a| Value::str(a)).collect())),
            "env.get" => {
                let name = arg_text(self, 1)?;
                Ok(Value::option(self.host.env_var(&name).map(Value::String)))
            }
            "rand.bits" => Ok(Value::Int(self.host.random_bits() as i64)),
            "stdin.read_line" => Ok(Value::option(self.host.read_line().map(Value::String))),
            "proc.run" => {
                let program = arg_text(self, 1)?;
                let list = match self.target(args.get(2).ok_or_else(|| ice("missing arguments"))?)? {
                    Value::List(xs) => xs.clone(),
                    other => return Err(ice(format!("Proc.run arguments {other:?}"))),
                };
                let mut words = Vec::new();
                for x in &list {
                    words.push(self.text(x)?.to_string());
                }
                Ok(match self.host.run_process(&program, &words) {
                    Ok((status, out, err)) => Value::ok(Value::Struct(Value::adt(
                        "ProcOutput",
                        0,
                        vec![Value::Int(status), Value::String(out), Value::String(err)],
                    ))),
                    Err(k) => {
                        let (variant, fields) = match k {
                            IoErrorKind::NotFound => ("NotFound", vec![Value::String(program)]),
                            IoErrorKind::PermissionDenied => {
                                ("PermissionDenied", vec![Value::String(program)])
                            }
                            IoErrorKind::Other(m) => ("Other", vec![Value::String(m)]),
                            other => ("Other", vec![Value::String(format!("{other:?}"))]),
                        };
                        let i = self.variants.get(&("ProcError", variant)).copied().unwrap_or(0);
                        Value::err(Value::variant("ProcError", i, fields))
                    }
                })
            }
            op if op.starts_with("net.") => self.net_call(&op[4..], args),
            other => Err(ice(format!("no host operation `{other}`"))),
        }
    }

    /// A `NetError` value.
    fn net_error(&self, f: crate::NetFail, address: &str) -> Value {
        let (name, fields) = match f {
            crate::NetFail::Refused => ("Refused", vec![Value::String(address.to_string())]),
            crate::NetFail::NotAllowed => ("NotAllowed", vec![Value::String(address.to_string())]),
            crate::NetFail::Closed => ("Closed", Vec::new()),
            crate::NetFail::Other(m) => ("Other", vec![Value::String(m)]),
        };
        let i = self.variants.get(&("NetError", name)).copied().unwrap_or(0);
        Value::variant("NetError", i, fields)
    }

    /// A `Box[dyn T]` of a host socket.
    fn boxed_socket(ty: &str, h: u64) -> Value {
        let ty = sir::Type::Named(ty.into(), Vec::new());
        Value::Struct(Value::adt(
            "Box",
            0,
            vec![Value::Dyn(Box::new(DynVal { ty, value: Value::Socket(h) }))],
        ))
    }

    fn socket_of(&self, v: &Value) -> Result<u64, Unwind> {
        match self.target(v)? {
            Value::Socket(h) => Ok(*h),
            other => Err(ice(format!("expected a socket, found {other:?}"))),
        }
    }

    /// Waits for a socket: the call is tried again a millisecond later on
    /// the host's clock (a poll; the scheduler runs other tasks meanwhile).
    fn wait_io(&self, args: &mut [Value]) -> Unwind {
        let at = self.clock_now(0) + 1_000_000;
        Unwind::Block(Box::new(crate::sched::Blocked {
            args: args.iter_mut().map(|v| std::mem::replace(v, Value::Moved)).collect(),
            wait: crate::sched::Wait::Io,
            keys: Vec::new(),
            timer: Some((0, at)),
        }))
    }

    /// The host's TCP (`HostNet`, `HostStream`, `HostListener`).
    fn net_call(&mut self, op: &str, args: &mut [Value]) -> Result<Value, Unwind> {
        use crate::Poll;
        let port_of = |me: &Self, v: &Value| -> Result<u16, Unwind> {
            let n = me.int(v)?;
            u16::try_from(n)
                .map_err(|_| crate::panic(sisu_diag::msg::PANIC_BAD_PORT, &[("port", n.to_string())]))
        };
        match op {
            "connect" => {
                if self.cancelled_now() {
                    return Ok(self.cancelled_err());
                }
                let host = self.text(&args[1])?.to_string();
                let port = port_of(self, &args[2])?;
                let address = format!("{host}:{port}");
                Ok(match self.host.tcp_connect(&host, port) {
                    Ok(h) => Value::ok(Value::Struct(Value::adt(
                        "Conn",
                        0,
                        vec![Self::boxed_socket("HostStream", h), Value::String(address)],
                    ))),
                    Err(f) => Value::err(self.net_error(f, &address)),
                })
            }
            "listen" => {
                let port = port_of(self, &args[1])?;
                let host = self.text(&args[2])?.to_string();
                let address = format!("{host}:{port}");
                Ok(match self.host.tcp_listen(&host, port) {
                    Ok((h, at)) => Value::ok(Value::Struct(Value::adt(
                        "Listener",
                        0,
                        vec![Self::boxed_socket("HostListener", h), Value::Int(i64::from(at))],
                    ))),
                    Err(f) => Value::err(self.net_error(f, &address)),
                })
            }
            "accept" => {
                if self.cancelled_now() {
                    return Ok(self.cancelled_err());
                }
                let h = self.socket_of(&args[0])?;
                match self.host.tcp_accept(h) {
                    Ok(Poll::Ready((c, peer))) => Ok(Value::ok(Value::Struct(Value::adt(
                        "Conn",
                        0,
                        vec![Self::boxed_socket("HostStream", c), Value::String(peer)],
                    )))),
                    Ok(Poll::Wait) => Err(self.wait_io(args)),
                    Err(f) => Ok(Value::err(self.net_error(f, ""))),
                }
            }
            "read" => {
                if self.cancelled_now() {
                    return Ok(self.cancelled_err());
                }
                let h = self.socket_of(&args[0])?;
                let max = usize::try_from(self.int(&args[1])?).unwrap_or(0);
                match self.host.tcp_read(h, max) {
                    Ok(Poll::Ready(b)) => Ok(Value::ok(Value::Bytes(Rc::from(b)))),
                    Ok(Poll::Wait) => Err(self.wait_io(args)),
                    Err(f) => Ok(Value::err(self.net_error(f, ""))),
                }
            }
            "write" => {
                if self.cancelled_now() {
                    return Ok(self.cancelled_err());
                }
                let h = self.socket_of(&args[0])?;
                let data = match self.target(&args[1])? {
                    Value::Bytes(b) => b.clone(),
                    other => return Err(ice(format!("a write of {other:?}"))),
                };
                match self.host.tcp_write(h, &data) {
                    Ok(Poll::Ready(n)) if n >= data.len() => Ok(Value::ok(Value::Unit)),
                    // What was not sent is sent on the next try.
                    Ok(Poll::Ready(n)) => {
                        args[1] = Value::Bytes(Rc::from(&data[n..]));
                        Err(self.wait_io(args))
                    }
                    Ok(Poll::Wait) => Err(self.wait_io(args)),
                    Err(f) => Ok(Value::err(self.net_error(f, ""))),
                }
            }
            "close" => {
                let h = self.socket_of(&args[0])?;
                self.host.tcp_close(h);
                Ok(Value::Unit)
            }
            other => Err(ice(format!("no network operation `{other}`"))),
        }
    }

    fn unit_result(&self, r: Result<(), IoErrorKind>, path: &str) -> Value {
        match r {
            Ok(()) => Value::ok(Value::Unit),
            Err(k) => Value::err(self.io_error(k, path)),
        }
    }
}

/// Whether a value holds a capability handle or a `Cap` (a `Lent` holds
/// one too: its type says it is not counted).
pub(crate) fn holds_cap(v: &Value) -> bool {
    match v {
        Value::Cap(_) => true,
        Value::Struct(a) | Value::Variant(a) => a.fields.iter().any(holds_cap),
        Value::Tuple(xs) => xs.iter().any(holds_cap),
        // An `own` closure's environment holds copies of what it captured
        // (decision 248).
        Value::Closure(c) => c.env.iter().any(holds_cap),
        _ => false,
    }
}
