//! The host intrinsics. Each implements the contract written above its
//! `@intrinsic` declaration in compiler/std/*.sisu; the names lowering
//! introduces itself (`iter.ref`, `fmt.display`, `error.trace_push`, ...)
//! are documented in docs/architecture.md ("Lowering-only intrinsics").
//!
//! Arguments arrive as SIR passes them: a `self` or parameter the
//! prelude takes by borrow is a reference (or a copy, for a `Copy` type);
//! `mut` ones are references; `own` ones are values. The accessors below
//! look through references, so an intrinsic does not care which it got.

use crate::value::{DictPart, EntryVal, IterState, Pointer, Step, Value};
use crate::{Interp, IoErrorKind, Unwind, ice, msg, panic, render};
use sisu_sir as sir;
use std::cmp::Ordering;
use std::rc::Rc;

struct Args(Vec<Value>);

impl Args {
    fn get(&self, i: usize) -> Result<&Value, Unwind> {
        self.0.get(i).ok_or_else(|| ice(format!("missing argument {i}")))
    }

    fn take(&mut self, i: usize) -> Result<Value, Unwind> {
        self.0
            .get_mut(i)
            .map(|v| std::mem::replace(v, Value::Moved))
            .ok_or_else(|| ice(format!("missing argument {i}")))
    }
}

/// A stable merge sort of indices with a comparison that may panic.
fn merge_sort(
    v: &mut Vec<usize>,
    cmp: &mut dyn FnMut(usize, usize) -> Result<Ordering, Unwind>,
) -> Result<(), Unwind> {
    if v.len() <= 1 {
        return Ok(());
    }
    let mut right = v.split_off(v.len() / 2);
    merge_sort(v, cmp)?;
    merge_sort(&mut right, cmp)?;
    let left = std::mem::take(v);
    let (mut i, mut j) = (0, 0);
    while i < left.len() && j < right.len() {
        if cmp(right[j], left[i])? == Ordering::Less {
            v.push(right[j]);
            j += 1;
        } else {
            v.push(left[i]);
            i += 1;
        }
    }
    v.extend_from_slice(&left[i..]);
    v.extend_from_slice(&right[j..]);
    Ok(())
}

/// Pairwise summation in a fixed shape (D-074): halves, recursively.
fn pairwise(xs: &[f64]) -> f64 {
    match xs.len() {
        0 => 0.0,
        1 => xs[0],
        n => pairwise(&xs[..n / 2]) + pairwise(&xs[n / 2..]),
    }
}

/// `pairwise` in `f32`: each sum rounded once to `f32`.
fn pairwise32(xs: &[f32]) -> f32 {
    match xs.len() {
        0 => 0.0,
        1 => xs[0],
        n => pairwise32(&xs[..n / 2]) + pairwise32(&xs[n / 2..]),
    }
}

/// Which dispatcher answers an intrinsic: each takes a name or leaves it
/// to the next, by the name alone, so the first call of a name finds its
/// dispatcher and every later one goes straight to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Route {
    Task,
    Cap,
    Sized,
    F32,
    Bytes,
    /// None of those: the names below.
    Rest,
}

/// Whether the intrinsic reads the type of its call's destination: `sum`,
/// whose result type says whether it adds floats, and a scope, whose result
/// type says which failures it carries (`sched.rs`).
pub(crate) fn needs_return_type(name: &str) -> bool {
    matches!(name, "sum" | "task.scope" | "task.race" | "task.shield")
}

impl<'p> Interp<'p, '_> {
    pub(crate) fn intrinsic(
        &mut self,
        name: &str,
        tys: &[sir::Type],
        args: Vec<Value>,
        ret: Option<&sir::Type>,
    ) -> Result<Value, Unwind> {
        let mut args = args;
        let known = self.routes.get(name).copied();
        let learn = |me: &mut Self, r: Route| {
            if known.is_none() {
                me.routes.insert(name.to_string(), r);
            }
        };
        if matches!(known, None | Some(Route::Task))
            && let Some(r) = self.task_intrinsic(name, tys, &mut args)
        {
            learn(self, Route::Task);
            return r;
        }
        if matches!(known, None | Some(Route::Cap))
            && let Some(r) = self.cap_intrinsic(name, tys, &mut args)
        {
            learn(self, Route::Cap);
            return r;
        }
        if matches!(known, None | Some(Route::Sized))
            && let Some(r) = self.sized_intrinsic(name, &args)
        {
            learn(self, Route::Sized);
            return r;
        }
        if matches!(known, None | Some(Route::F32))
            && let Some(r) = self.f32_intrinsic(name, &args)
        {
            learn(self, Route::F32);
            return r;
        }
        if matches!(known, None | Some(Route::Bytes))
            && let Some(r) = self.bytes_intrinsic(name, &args)
        {
            learn(self, Route::Bytes);
            return r;
        }
        learn(self, Route::Rest);
        if name == "Mutex.lock" {
            return self.lock(&mut args);
        }
        // `Array.filled(value)`: `N` (its second type argument) copies.
        if name == "Array.filled" {
            let n = match tys.get(1) {
                Some(sir::Type::Const(n)) if *n >= 0 => *n as usize,
                other => return Err(ice(format!("an array length that is not a constant: {other:?}"))),
            };
            let value = args.into_iter().next().ok_or_else(|| ice("Array.filled without a value"))?;
            let mut items = Vec::with_capacity(n);
            for _ in 1..n {
                let c = self.copy_value(&value);
                items.push(c);
            }
            if n > 0 {
                items.push(value);
            }
            return Ok(Value::List(items));
        }
        if name == "Array.to_list" {
            let v = args.into_iter().next().ok_or_else(|| ice("Array.to_list without an array"))?;
            let v = self.target(&v)?.clone();
            return Ok(self.copy_value(&v));
        }
        // A `const` generic parameter's value: its type argument.
        if name == "generic.value" {
            return match tys.first() {
                Some(sir::Type::Const(n)) => {
                    i64::try_from(*n).map(Value::Int).map_err(|_| ice("a constant argument out of range"))
                }
                other => Err(ice(format!("a `const` parameter without a constant: {other:?}"))),
            };
        }
        // A receiver's iterator waits for the next value.
        if name == "Iter.next"
            && let Some(v) = args.first()
        {
            let p = self.pointer_of(v)?;
            if let Value::Iter(st) = self.read(&p)? {
                match **st {
                    IterState::Recv(c) => return self.recv_next(c, args),
                    IterState::Gen(t) => return self.gen_next(t, args),
                    _ => {}
                }
            }
        }
        let mut a = Args(args);
        if let Some((owner, method)) = name.split_once('.') {
            return match owner {
                "int" | "f64" | "bool" | "char" => self.scalar_method(owner, method, &a),
                "str" => self.str_method(method, &mut a),
                "String" | "Path" => self.string_method(owner, method, &mut a),
                "list" => self.list_method(method, &mut a),
                "dict" | "Entry" => self.dict_method(owner, method, &mut a),
                "set" => self.set_method(method, &mut a),
                "Shared" | "Weak" => self.shared_method(owner, method, &mut a),
                "Cell" => self.cell_method(method, &mut a),
                "Mutex" => self.mutex_method(method, &mut a),
                "Arena" => self.arena_method(method, &mut a),
                "Iter" => {
                    if method == "next" {
                        let p = self.pointer_of(a.get(0)?)?;
                        let mut st = match std::mem::replace(self.write(&p)?, Value::Moved) {
                            Value::Iter(st) => st,
                            other => return Err(ice(format!("Iter.next on {other:?}"))),
                        };
                        let r = self.iter_next(&mut st);
                        *self.write(&p)? = Value::Iter(st);
                        return Ok(Value::option(r?));
                    }
                    // `iter_of(xs)`: the elements of a list, moved.
                    if method == "of" {
                        let v = a.take(0)?;
                        return Ok(Value::Iter(Box::new(self.iter_own(v)?)));
                    }
                    Err(ice(format!("unknown intrinsic `Iter.{method}`")))
                }
                "Option" | "Result" => self.option_method(owner, method, &mut a),
                "iter" => match method {
                    "ref" | "mut" => Ok(Value::Iter(Box::new(self.iter_borrow(a.get(0)?)?))),
                    _ => {
                        let v = a.take(0)?;
                        Ok(Value::Iter(Box::new(self.iter_own(v)?)))
                    }
                },
                "fmt" => self.fmt(method, &mut a),
                "structural" => self.structural(method, &mut a),
                // The numbers' operator traits (`<std>/builtin_impls.sisu`).
                "op" => {
                    let op = match method {
                        "add" => sisu_sir::BinOp::Add,
                        "sub" => sisu_sir::BinOp::Sub,
                        "mul" => sisu_sir::BinOp::Mul,
                        "div" => sisu_sir::BinOp::Div,
                        "rem" => sisu_sir::BinOp::Mod,
                        "neg" => return self.unary(sisu_sir::UnOp::Neg, a.get(0)?),
                        other => return Err(ice(format!("no operator `{other}`"))),
                    };
                    let (x, y) = (a.get(0)?.clone(), a.get(1)?.clone());
                    self.binary(op, &x, &y)
                }
                "assert" => self.assert_fail(method, &a),
                "contract" if method == "fail" => self.contract_fail(&a),
                "test" => self.test_intrinsic(method, &a),
                "error" => self.error_method(method, tys, &a),
                // Hot replacement's boundaries (std `live`): the task adopts
                // the newest published epoch.
                "live" => match method {
                    "boundary" => {
                        self.live_boundary()?;
                        Ok(Value::Unit)
                    }
                    "epoch" => Ok(Value::Int(self.live_epoch() as i64)),
                    _ => Err(ice(format!("unknown intrinsic `{name}`"))),
                },
                _ => Err(ice(format!("unknown intrinsic `{name}`"))),
            };
        }
        match name {
            "print" | "print_err" => {
                let mut s = self.display_string(self.target(a.get(0)?)?);
                s.push('\n');
                if name == "print" {
                    self.host.stdout(&s);
                } else {
                    self.host.stderr(&s);
                }
                Ok(Value::Unit)
            }
            "len" => {
                let n = match self.target(a.get(0)?)? {
                    Value::Str(s, _) => s.chars().count(),
                    Value::String(s) => s.chars().count(),
                    Value::List(xs) => xs.len(),
                    Value::Bytes(b) => b.len(),
                    Value::Set(s) => s.items.len(),
                    Value::Dict(d) => d.entries.len(),
                    Value::Arena(a) => a.slots.iter().filter(|(_, v)| !v.is_moved()).count(),
                    other => return Err(ice(format!("len of {other:?}"))),
                };
                Ok(Value::Int(n as i64))
            }
            "range" => {
                let (start, stop, step) = (self.int(a.get(0)?)?, self.int(a.get(1)?)?, self.int(a.get(2)?)?);
                if step == 0 {
                    return Err(panic(msg::PANIC_RANGE_STEP, &[]));
                }
                Ok(Value::Iter(Box::new(IterState::Range { cur: start, stop, step })))
            }
            "enumerate" => {
                let inner = self.iterable(a.take(0)?, tys.first())?;
                Ok(Value::Iter(Box::new(IterState::Enumerate { inner: Box::new(inner), n: 0 })))
            }
            "zip" | "zip_shortest" => {
                let x = self.iterable(a.take(0)?, tys.first())?;
                let y = self.iterable(a.take(1)?, tys.get(1))?;
                Ok(Value::Iter(Box::new(IterState::Zip {
                    a: Box::new(x),
                    b: Box::new(y),
                    strict: name == "zip",
                })))
            }
            "reversed" => {
                let mut st = self.iterable(a.take(0)?, tys.first())?;
                let mut items = self.iter_collect(&mut st)?;
                items.reverse();
                Ok(Value::Iter(Box::new(IterState::Items(items.into()))))
            }
            "iterable_list" => {
                let mut st = self.iterable(a.take(0)?, tys.first())?;
                Ok(Value::List(self.iter_collect(&mut st)?))
            }
            "sorted" => {
                let mut st = self.iterable(a.take(0)?, tys.first())?;
                let items = self.iter_collect(&mut st)?;
                let mut idx: Vec<usize> = (0..items.len()).collect();
                merge_sort(&mut idx, &mut |i, j| self.total_compare(&items[i], &items[j]))?;
                let mut slots: Vec<Option<Value>> = items.into_iter().map(Some).collect();
                Ok(Value::List(idx.into_iter().map(|i| slots[i].take().unwrap_or(Value::Moved)).collect()))
            }
            "min" | "max" => {
                // From the body's call (`@structural`), the values come by
                // reference, as a type parameter's do: the result is a copy.
                let (mut x, mut y) = (a.take(0)?, a.take(1)?);
                if matches!(x, Value::Ref(_)) {
                    x = self.target(&x)?.clone();
                }
                if matches!(y, Value::Ref(_)) {
                    y = self.target(&y)?.clone();
                }
                let o = self.total_compare(&x, &y)?;
                let pick_y = if name == "min" { o == Ordering::Greater } else { o == Ordering::Less };
                let (keep, other) = if pick_y { (y, x) } else { (x, y) };
                self.queue_drop(other, None);
                Ok(keep)
            }
            "sum" => {
                let mut st = self.iterable(a.take(0)?, tys.first())?;
                let items = self.iter_collect(&mut st)?;
                let float = matches!(tys.first().or(ret), Some(sir::Type::Float));
                if float {
                    let xs = items.iter().map(|x| self.float(x)).collect::<Result<Vec<_>, _>>()?;
                    return Ok(Value::Float(pairwise(&xs)));
                }
                if matches!(tys.first().or(ret), Some(sir::Type::Named(n, _)) if n == "f32") {
                    let mut xs = Vec::with_capacity(items.len());
                    for x in &items {
                        match self.target(x)? {
                            Value::F32(f) => xs.push(*f),
                            other => return Err(ice(format!("sum of f32 over {other:?}"))),
                        }
                    }
                    return Ok(Value::F32(pairwise32(&xs)));
                }
                let mut total = 0i64;
                for x in &items {
                    let v = self.int(x)?;
                    total = total
                        .checked_add(v)
                        .ok_or_else(|| panic(msg::PANIC_OVERFLOW, &[("expr", format!("{total} + {v}"))]))?;
                }
                Ok(Value::Int(total))
            }
            "any" | "all" => {
                let mut st = self.iterable(a.take(0)?, tys.first())?;
                let want = name == "any";
                while let Some(x) = self.iter_next(&mut st)? {
                    if self.bool(&x)? == want {
                        self.finish_iter(st)?;
                        return Ok(Value::Bool(want));
                    }
                }
                Ok(Value::Bool(!want))
            }
            "abs" => match self.target(a.get(0)?)? {
                Value::Int(i) => i
                    .checked_abs()
                    .map(Value::Int)
                    .ok_or_else(|| panic(msg::PANIC_OVERFLOW, &[("expr", format!("abs({i})"))])),
                Value::Float(f) => Ok(Value::Float(f.abs())),
                Value::F32(f) => Ok(Value::F32(f.abs())),
                other => Err(ice(format!("abs of {other:?}"))),
            },
            "panic" => Err(panic(msg::PANIC_EXPLICIT, &[("message", self.text(a.get(0)?)?.to_string())])),
            "unreachable" => {
                let m = self.text(a.get(0)?)?;
                Err(if m.is_empty() {
                    panic(msg::PANIC_UNREACHABLE_PLAIN, &[])
                } else {
                    panic(msg::PANIC_UNREACHABLE, &[("message", m.to_string())])
                })
            }
            "todo" => {
                let n = self.text(a.get(0)?)?.to_string();
                let shown = if n.is_empty() {
                    "`todo()`".to_string()
                } else {
                    format!("`todo({})`", crate::format::quote_str(&n))
                };
                Err(Unwind::Panic(Box::new(crate::PanicInfo {
                    message: render(msg::PANIC_HOLE, &[("hole", shown)]),
                    stack: Vec::new(),
                    hole: Some(n),
                    refinement: None,
                    too_long: None,
                })))
            }
            _ => Err(ice(format!("unknown intrinsic `{name}`"))),
        }
    }

    fn scalar_method(&mut self, owner: &str, method: &str, a: &Args) -> Result<Value, Unwind> {
        let x = self.target(a.get(0)?)?.clone();
        let y = || a.get(1);
        Ok(match (owner, method) {
            ("int", "cmp") | ("bool", "cmp") | ("char", "cmp") => {
                Value::ordering(self.total_compare(&x, y()?)?)
            }
            ("int", "to_f64") => Value::Float(self.int(&x)? as f64),
            ("int", "pow") => {
                Value::Int(crate::ops::int_op(sir::BinOp::Pow, self.int(&x)?, self.int(y()?)?)?)
            }
            ("f64", "cmp") => {
                Value::ordering(self.compare(&x, y()?)?.ok_or_else(|| panic(msg::PANIC_UNORDERED, &[]))?)
            }
            ("f64", m) => {
                let f = self.float(&x)?;
                match m {
                    "sqrt" => Value::Float(f.sqrt()),
                    "floor" => Value::Float(f.floor()),
                    "ceil" => Value::Float(f.ceil()),
                    "round" => Value::Float(f.round()),
                    "abs" => Value::Float(f.abs()),
                    "is_nan" => Value::Bool(f.is_nan()),
                    // One rounding: the host's fused multiply-add, which
                    // IEEE 754 defines exactly.
                    "mul_add" => Value::Float(f.mul_add(self.float(y()?)?, self.float(a.get(2)?)?)),
                    // How many `f64`s lie between the two, at most `n`.
                    "within_ulps" => {
                        let g = self.float(y()?)?;
                        let n = self.int(a.get(2)?)?;
                        Value::Bool(n >= 0 && ulps_between(f, g).is_some_and(|d| d <= n as u64))
                    }
                    "truncate_to_int" => {
                        let t = f.trunc();
                        if t.is_nan() || !(-9.223_372_036_854_776e18..9.223_372_036_854_776e18).contains(&t) {
                            return Err(panic(
                                msg::PANIC_CONVERSION,
                                &[("value", crate::format::float_repr(f)), ("ty", "int".into())],
                            ));
                        }
                        Value::Int(t as i64)
                    }
                    other => return Err(ice(format!("no f64 method `{other}`"))),
                }
            }
            ("char", "from_code") => {
                let n = self.int(&x)?;
                Value::option(u32::try_from(n).ok().and_then(char::from_u32).map(Value::Char))
            }
            ("char", m) => {
                let c = self.char(&x)?;
                match m {
                    "is_digit" => Value::Bool(c.is_ascii_digit()),
                    "to_digit" => Value::option(c.to_digit(10).map(|d| Value::Int(d as i64))),
                    "is_alphabetic" => Value::Bool(c.is_alphabetic()),
                    "is_whitespace" => Value::Bool(c.is_whitespace()),
                    "is_uppercase" => Value::Bool(c.is_uppercase()),
                    "is_lowercase" => Value::Bool(c.is_lowercase()),
                    "code" => Value::Int(i64::from(u32::from(c))),
                    other => return Err(ice(format!("no char method `{other}`"))),
                }
            }
            (o, m) => return Err(ice(format!("no {o} method `{m}`"))),
        })
    }

    fn str_method(&mut self, method: &str, a: &mut Args) -> Result<Value, Unwind> {
        // The text's bytes (03, "Strings and text"), read in place: the
        // methods std writes in Sisu over them make many calls per text.
        match method {
            "byte_len" => return Ok(Value::Int(self.text(a.get(0)?)?.len() as i64)),
            "byte_at" => {
                let i = self.int(a.get(1)?)?;
                let s = self.text(a.get(0)?)?;
                return match usize::try_from(i).ok().and_then(|i| s.as_bytes().get(i)) {
                    Some(b) => Ok(Value::Int(i64::from(*b))),
                    None => Err(panic(
                        msg::PANIC_INDEX_OUT_OF_BOUNDS,
                        &[("len", s.len().to_string()), ("index", i.to_string())],
                    )),
                };
            }
            "byte_slice" => {
                let (start, end) = (self.int(a.get(1)?)?, self.int(a.get(2)?)?);
                let tag = match self.target(a.get(0)?)? {
                    Value::Str(_, t) => *t,
                    _ => 0,
                };
                let s = self.text(a.get(0)?)?;
                let len = s.len() as i64;
                if start < 0 || end < start || end > len {
                    let bad = if start < 0 || start > len { start } else { end };
                    return Err(panic(
                        msg::PANIC_INDEX_OUT_OF_BOUNDS,
                        &[("len", len.to_string()), ("index", bad.to_string())],
                    ));
                }
                let (start, end) = (start as usize, end as usize);
                for i in [start, end] {
                    if !s.is_char_boundary(i) {
                        return Err(panic(msg::PANIC_CHAR_BOUNDARY, &[("index", i.to_string())]));
                    }
                }
                return Ok(Value::Str(Rc::from(&s[start..end]), tag));
            }
            "bytes" => {
                let s = self.text(a.get(0)?)?;
                let items = s.bytes().map(|b| Value::Int(i64::from(b))).collect();
                return Ok(Value::Iter(Box::new(IterState::Items(items))));
            }
            _ => {}
        }
        let s: Rc<str> = match self.target(a.get(0)?)? {
            Value::Str(s, tag) => {
                self.mon_view_ok(*tag)?;
                s.clone()
            }
            _ => Rc::from(self.text(a.get(0)?)?),
        };
        Ok(match method {
            "cmp" => {
                let o = self.text(a.get(1)?)?;
                Value::ordering((*s).cmp(o))
            }
            "chars" => Value::Iter(Box::new(IterState::Chars { text: s, pos: 0 })),
            "parse_float" => {
                Value::option(decimal(&s).then(|| s.parse::<f64>().ok()).flatten().map(Value::Float))
            }
            // Rounded once from the decimal text, not through an `f64`.
            "parse_f32" => {
                Value::option(decimal(&s).then(|| s.parse::<f32>().ok()).flatten().map(Value::F32))
            }
            "to_lowercase" => Value::String(s.to_lowercase()),
            "to_uppercase" => Value::String(s.to_uppercase()),
            "to_string" => Value::String(s.to_string()),
            other => return Err(ice(format!("no str method `{other}`"))),
        })
    }

    fn string_method(&mut self, owner: &str, method: &str, a: &mut Args) -> Result<Value, Unwind> {
        match (owner, method) {
            ("String", "new") => Ok(Value::String(self.text(a.get(0)?)?.to_string())),
            ("Path", "new") => Ok(Value::Path(Rc::from(self.text(a.get(0)?)?))),
            ("Path", "as_str") => Ok(Value::str(self.text(a.get(0)?)?)),
            ("String", m) => {
                let extra = match m {
                    "push" => self.text(a.get(1)?)?.to_string(),
                    "push_char" => self.char(a.get(1)?)?.to_string(),
                    _ => String::new(),
                };
                let target = self.pointer_of(a.get(0)?)?;
                match self.write(&target)? {
                    Value::String(s) => {
                        if m == "clear" {
                            s.clear();
                        } else {
                            s.push_str(&extra);
                        }
                        Ok(Value::Unit)
                    }
                    other => Err(ice(format!("String.{m} on {other:?}"))),
                }
            }
            (o, m) => Err(ice(format!("no {o} method `{m}`"))),
        }
    }

    fn list_ptr(&self, v: &Value) -> Result<(Pointer, usize), Unwind> {
        let p = self.pointer_of(v)?;
        match self.read(&p)? {
            Value::List(xs) => Ok((p, xs.len())),
            other => Err(ice(format!("expected a list, found {other:?}"))),
        }
    }

    fn with_list<R>(
        &mut self,
        v: &Value,
        f: impl FnOnce(&mut Vec<Value>) -> Result<R, Unwind>,
    ) -> Result<R, Unwind> {
        let p = self.pointer_of(v)?;
        match self.write(&p)? {
            Value::List(xs) => f(xs),
            other => Err(ice(format!("expected a list, found {other:?}"))),
        }
    }

    fn list_method(&mut self, method: &str, a: &mut Args) -> Result<Value, Unwind> {
        match method {
            "new" | "with_capacity" => Ok(Value::List(Vec::new())),
            "push" => {
                let x = a.take(1)?;
                self.with_list(a.get(0)?, |xs| {
                    xs.push(x);
                    Ok(Value::Unit)
                })
            }
            "pop" => self.with_list(a.get(0)?, |xs| Ok(Value::option(xs.pop()))),
            "insert" => {
                let i = self.int(a.get(1)?)?;
                let x = a.take(2)?;
                self.with_list(a.get(0)?, |xs| {
                    if i < 0 || i as usize > xs.len() {
                        return Err(panic(
                            msg::PANIC_INSERT_OUT_OF_BOUNDS,
                            &[("index", i.to_string()), ("len", xs.len().to_string())],
                        ));
                    }
                    xs.insert(i as usize, x);
                    Ok(Value::Unit)
                })
            }
            "remove" => {
                let i = self.int(a.get(1)?)?;
                self.with_list(a.get(0)?, |xs| {
                    if i < 0 || i as usize >= xs.len() {
                        return Err(panic(
                            msg::PANIC_INDEX_OUT_OF_BOUNDS,
                            &[("len", xs.len().to_string()), ("index", i.to_string())],
                        ));
                    }
                    Ok(xs.remove(i as usize))
                })
            }
            "is_empty" => Ok(Value::Bool(self.list_ptr(a.get(0)?)?.1 == 0)),
            "contains" => Ok(Value::Bool(self.contains(a.get(0)?, a.get(1)?)?)),
            "get" | "first" | "last" => {
                let (p, len) = self.list_ptr(a.get(0)?)?;
                let i = match method {
                    "get" => self.int(a.get(1)?)?,
                    "first" => 0,
                    _ => len as i64 - 1,
                };
                Ok(Value::option(
                    (i >= 0 && (i as usize) < len)
                        .then(|| Value::Ref(Rc::new(p.join(Step::Index(i as usize))))),
                ))
            }
            "iter" => {
                let (p, _) = self.list_ptr(a.get(0)?)?;
                Ok(Value::Iter(Box::new(IterState::ListRef { base: p, idx: 0 })))
            }
            // The structural fast path of `list.sort` (its Sisu body is the
            // contract): a stable merge sort by the values' structural order.
            "sort" => {
                let (p, len) = self.list_ptr(a.get(0)?)?;
                let mut idx: Vec<usize> = (0..len).collect();
                merge_sort(&mut idx, &mut |i, j| {
                    let (x, y) = (p.join(Step::Index(i)), p.join(Step::Index(j)));
                    self.total_compare(self.read(&x)?, self.read(&y)?)
                })?;
                match self.write(&p)? {
                    Value::List(xs) => {
                        let mut old: Vec<Option<Value>> = xs.drain(..).map(Some).collect();
                        *xs = idx.into_iter().map(|i| old[i].take().unwrap_or(Value::Moved)).collect();
                        Ok(Value::Unit)
                    }
                    other => Err(ice(format!("sorting {other:?}"))),
                }
            }
            "reverse" => self.with_list(a.get(0)?, |xs| {
                xs.reverse();
                Ok(Value::Unit)
            }),
            "swap" => {
                let (i, j) = (self.int(a.get(1)?)?, self.int(a.get(2)?)?);
                self.with_list(a.get(0)?, |xs| {
                    let n = xs.len();
                    let at = |k: i64| usize::try_from(k).ok().filter(|k| *k < n);
                    match (at(i), at(j)) {
                        (Some(i), Some(j)) => {
                            xs.swap(i, j);
                            Ok(Value::Unit)
                        }
                        _ => Err(crate::panic(
                            msg::PANIC_INDEX_OUT_OF_BOUNDS,
                            &[
                                ("index", (if at(i).is_none() { i } else { j }).to_string()),
                                ("len", n.to_string()),
                            ],
                        )),
                    }
                })
            }
            "clear" => {
                let old = self.with_list(a.get(0)?, |xs| Ok(std::mem::take(xs)))?;
                self.queue_drop(Value::List(old), None);
                Ok(Value::Unit)
            }
            "extend" => {
                let other = match a.take(1)? {
                    Value::List(ys) => ys,
                    other => return Err(ice(format!("extending with {other:?}"))),
                };
                self.with_list(a.get(0)?, |xs| {
                    xs.extend(other);
                    Ok(Value::Unit)
                })
            }
            "join" => {
                let sep = self.text(a.get(1)?)?.to_string();
                let parts: Vec<String> = match self.target(a.get(0)?)? {
                    Value::List(xs) => xs.iter().map(|x| self.display_string(x)).collect(),
                    other => return Err(ice(format!("join on {other:?}"))),
                };
                Ok(Value::String(parts.join(&sep)))
            }
            "slice" => {
                let (lo, hi) = (self.int(a.get(1)?)?, self.int(a.get(2)?)?);
                let xs = match self.target(a.get(0)?)? {
                    Value::List(xs) => xs,
                    other => return Err(ice(format!("slicing {other:?}"))),
                };
                if lo < 0 || hi < 0 {
                    return Err(panic(
                        msg::PANIC_SLICE_BOUNDS,
                        &[("start", lo.to_string()), ("end", hi.to_string())],
                    ));
                }
                let lo = (lo as usize).min(xs.len());
                let hi = (hi as usize).min(xs.len()).max(lo);
                Ok(Value::List(xs[lo..hi].to_vec()))
            }
            "slice_middle" => {
                let (pre, suf) = (self.int(a.get(1)?)? as usize, self.int(a.get(2)?)? as usize);
                let xs = match self.target(a.get(0)?)? {
                    Value::List(xs) => xs,
                    other => return Err(ice(format!("slicing {other:?}"))),
                };
                let hi = xs.len().saturating_sub(suf).max(pre.min(xs.len()));
                Ok(Value::List(xs[pre.min(xs.len())..hi].to_vec()))
            }
            other => Err(ice(format!("no list method `{other}`"))),
        }
    }

    fn dict_ptr(&self, v: &Value) -> Result<Pointer, Unwind> {
        let p = self.pointer_of(v)?;
        match self.read(&p)? {
            Value::Dict(_) => Ok(p),
            other => Err(ice(format!("expected a dict, found {other:?}"))),
        }
    }

    fn dict_position(&self, p: &Pointer, key: &Value) -> Result<Option<usize>, Unwind> {
        let k = self.key_of(key)?;
        match self.read(p)? {
            Value::Dict(d) => Ok(d.position(&k)),
            other => Err(ice(format!("expected a dict, found {other:?}"))),
        }
    }

    fn dict_method(&mut self, owner: &str, method: &str, a: &mut Args) -> Result<Value, Unwind> {
        if owner == "Entry" {
            let Value::Entry(e) = a.take(0)? else { return Err(ice("Entry method on a non-entry")) };
            let value = a.take(1)?;
            let EntryVal { dict, key } = *e;
            let pos = match self.dict_position(&dict, &key)? {
                Some(pos) => {
                    self.queue_drop(value, None);
                    self.queue_drop(key, None);
                    pos
                }
                None => {
                    let k = self.key_of(&key)?;
                    match self.write(&dict)? {
                        Value::Dict(d) => {
                            d.insert(k, key, value);
                            d.entries.len() - 1
                        }
                        other => return Err(ice(format!("entry into {other:?}"))),
                    }
                }
            };
            return Ok(Value::Ref(Rc::new(dict.join(Step::DictVal(pos)))));
        }
        match method {
            "new" => Ok(Value::Dict(Box::default())),
            "get" => {
                let p = self.dict_ptr(a.get(0)?)?;
                let pos = self.dict_position(&p, a.get(1)?)?;
                Ok(Value::option(pos.map(|i| Value::Ref(Rc::new(p.join(Step::DictVal(i)))))))
            }
            "contains" => {
                let p = self.dict_ptr(a.get(0)?)?;
                Ok(Value::Bool(self.dict_position(&p, a.get(1)?)?.is_some()))
            }
            "is_empty" => match self.target(a.get(0)?)? {
                Value::Dict(d) => Ok(Value::Bool(d.entries.is_empty())),
                other => Err(ice(format!("expected a dict, found {other:?}"))),
            },
            "insert" => {
                let p = self.dict_ptr(a.get(0)?)?;
                let (key, value) = (a.take(1)?, a.take(2)?);
                let k = self.key_of(&key)?;
                let key = self.owned_key(&key)?;
                match self.write(&p)? {
                    Value::Dict(d) => Ok(Value::option(d.insert(k, key, value))),
                    other => Err(ice(format!("insert into {other:?}"))),
                }
            }
            "remove" => {
                let p = self.dict_ptr(a.get(0)?)?;
                let k = self.key_of(a.get(1)?)?;
                let removed = match self.write(&p)? {
                    Value::Dict(d) => d.remove(&k),
                    other => return Err(ice(format!("remove from {other:?}"))),
                };
                match removed {
                    Some((key, v)) => {
                        self.queue_drop(key, None);
                        Ok(Value::some(v))
                    }
                    None => Ok(Value::none()),
                }
            }
            "entry" => {
                let p = self.dict_ptr(a.get(0)?)?;
                let key = a.take(1)?;
                let key = self.owned_key(&key)?;
                Ok(Value::Entry(Box::new(EntryVal { dict: p, key })))
            }
            "keys" | "values" | "items" => {
                let p = self.dict_ptr(a.get(0)?)?;
                let part = match method {
                    "keys" => DictPart::Keys,
                    "values" => DictPart::Values,
                    _ => DictPart::Items,
                };
                Ok(Value::Iter(Box::new(IterState::DictRef { base: p, idx: 0, part })))
            }
            other => Err(ice(format!("no dict method `{other}`"))),
        }
    }

    fn set_method(&mut self, method: &str, a: &mut Args) -> Result<Value, Unwind> {
        if method == "new" {
            return Ok(Value::Set(Box::default()));
        }
        let p = self.pointer_of(a.get(0)?)?;
        match method {
            "add" => {
                let x = a.take(1)?;
                let k = self.key_of(&x)?;
                let x = self.owned_key(&x)?;
                match self.write(&p)? {
                    Value::Set(s) => Ok(Value::Bool(s.insert(k, x))),
                    other => Err(ice(format!("add to {other:?}"))),
                }
            }
            "remove" => {
                let k = self.key_of(a.get(1)?)?;
                match self.write(&p)? {
                    Value::Set(s) => Ok(Value::Bool(s.remove(&k))),
                    other => Err(ice(format!("remove from {other:?}"))),
                }
            }
            "contains" => Ok(Value::Bool(self.contains(a.get(0)?, a.get(1)?)?)),
            "is_empty" => match self.read(&p)? {
                Value::Set(s) => Ok(Value::Bool(s.items.is_empty())),
                other => Err(ice(format!("expected a set, found {other:?}"))),
            },
            "iter" => Ok(Value::Iter(Box::new(IterState::SetRef { base: p, idx: 0 }))),
            other => Err(ice(format!("no set method `{other}`"))),
        }
    }

    fn option_method(&mut self, owner: &str, method: &str, a: &mut Args) -> Result<Value, Unwind> {
        let variant = match self.target(a.get(0)?)? {
            Value::Variant(v) => v.variant,
            other => return Err(ice(format!("{owner}.{method} on {other:?}"))),
        };
        let first = variant == 0;
        match method {
            "is_some" | "is_ok" => return Ok(Value::Bool(first)),
            "is_none" | "is_err" => return Ok(Value::Bool(!first)),
            _ => {}
        }
        let payload = match a.take(0)? {
            Value::Variant(v) => v.fields.into_iter().next(),
            Value::Ref(p) => match self.read(&p)? {
                Value::Variant(v) => v.fields.first().cloned(),
                _ => None,
            },
            _ => None,
        };
        match method {
            "expect" => match (first, payload) {
                (true, Some(v)) => Ok(v),
                (_, payload) => {
                    let m = self.text(a.get(1)?)?.to_string();
                    let mut text = match (&payload, m.is_empty()) {
                        (Some(e), true) => {
                            render(msg::PANIC_EXPECT_ERROR, &[("value", self.debug_string(e))])
                        }
                        (Some(e), false) => {
                            render(msg::PANIC_EXPECT, &[("message", m), ("value", self.debug_string(e))])
                        }
                        (None, true) => render(msg::PANIC_EXPECT_NONE, &[]),
                        (None, false) => render(msg::PANIC_EXPLICIT, &[("message", m)]),
                    };
                    if let Some(e) = &payload {
                        let lines = self.trace_lines(e);
                        if !lines.is_empty() {
                            text.push_str("\n  ");
                            text.push_str(&render(msg::RUN_TRACE_HEADER, &[]));
                        }
                        for line in lines {
                            text.push_str("\n    ");
                            text.push_str(&render(msg::RUN_TRACE_LINE, &[("location", line)]));
                        }
                    }
                    Err(panic(msg::PANIC_EXPLICIT, &[("message", text)]))
                }
            },
            "or" => {
                let default = a.take(1)?;
                match (first, payload) {
                    (true, Some(v)) => {
                        self.queue_drop(default, None);
                        Ok(v)
                    }
                    (_, other) => {
                        if let Some(e) = other {
                            self.queue_drop(e, None);
                        }
                        Ok(default)
                    }
                }
            }
            "ok" | "err" => {
                let want = method == "ok";
                match payload {
                    Some(v) if first == want => Ok(Value::some(v)),
                    Some(v) => {
                        self.queue_drop(v, None);
                        Ok(Value::none())
                    }
                    None => Ok(Value::none()),
                }
            }
            other => Err(ice(format!("no {owner} method `{other}`"))),
        }
    }

    pub(crate) fn io_error(&self, kind: IoErrorKind, path: &str) -> Value {
        let (name, fields) = match kind {
            IoErrorKind::NotFound => ("NotFound", vec![Value::Path(Rc::from(path))]),
            IoErrorKind::PermissionDenied => ("PermissionDenied", vec![Value::Path(Rc::from(path))]),
            IoErrorKind::AlreadyExists => ("AlreadyExists", vec![Value::Path(Rc::from(path))]),
            IoErrorKind::InvalidData => ("InvalidData", vec![Value::Path(Rc::from(path))]),
            IoErrorKind::Interrupted => ("Interrupted", Vec::new()),
            IoErrorKind::Other(m) => ("Other", vec![Value::String(m)]),
        };
        let i = self.variants.get(&("IoError", name)).copied().unwrap_or(0);
        Value::variant("IoError", i, fields)
    }

    fn fmt(&mut self, method: &str, a: &mut Args) -> Result<Value, Unwind> {
        if method == "new" {
            return Ok(Value::String(String::new()));
        }
        if method == "formatter" {
            // `Formatter(out="", spec=spec, depth=0)`.
            let spec = self.text(a.get(0)?)?.to_string();
            return Ok(Value::Struct(Value::adt(
                "Formatter",
                0,
                vec![Value::String(String::new()), Value::String(spec), Value::Int(0)],
            )));
        }
        match method {
            "digits" => {
                let (n, base, upper) = (self.int(a.get(0)?)?, self.int(a.get(1)?)?, self.bool(a.get(2)?)?);
                return Ok(Value::String(crate::format::digits(n, base, upper)));
            }
            "wide_digits" => {
                let mag = match self.target(a.get(0)?)? {
                    Value::Int(n) => u128::from(n.unsigned_abs()),
                    Value::U64(n) => u128::from(*n),
                    Value::I128(n) => n.unsigned_abs(),
                    Value::U128(n) => *n,
                    other => return Err(ice(format!("fmt.wide_digits of {other:?}"))),
                };
                let (base, upper) = (self.int(a.get(1)?)?, self.bool(a.get(2)?)?);
                return Ok(Value::String(crate::format::wide_digits(mag, base, upper)));
            }
            "float_digits" => {
                let x = self.float(a.get(0)?)?;
                let kind = self.text(a.get(1)?)?.chars().next();
                let precision = usize::try_from(self.int(a.get(2)?)?).ok();
                return Ok(Value::String(crate::format::float_digits(x, kind, precision)));
            }
            "float32_digits" => {
                let x = match self.target(a.get(0)?)? {
                    Value::F32(f) => *f,
                    other => return Err(ice(format!("fmt.float32_digits of {other:?}"))),
                };
                let kind = self.text(a.get(1)?)?.chars().next();
                let precision = usize::try_from(self.int(a.get(2)?)?).ok();
                return Ok(Value::String(crate::format::float32_digits(x, kind, precision)));
            }
            "strftime" => {
                let t = match self.target(a.get(0)?)? {
                    Value::DateTime(t) => *t,
                    other => return Err(ice(format!("fmt.strftime of {other:?}"))),
                };
                return Ok(Value::String(crate::format::strftime(t, self.text(a.get(1)?)?)));
            }
            _ => {}
        }
        let text = match method {
            "text" => self.text(a.get(1)?)?.to_string(),
            // Without a spec: an interpolation with one formats through the
            // value's impl (std/prelude.sisu, `Spec`).
            "display" => self.display_string(self.target(a.get(1)?)?),
            "debug" => self.debug_string(self.target(a.get(1)?)?),
            other => return Err(ice(format!("no fmt operation `{other}`"))),
        };
        let p = self.pointer_of(a.get(0)?)?;
        match self.write(&p)? {
            Value::String(s) => {
                s.push_str(&text);
                Ok(Value::Unit)
            }
            other => Err(ice(format!("formatting into {other:?}"))),
        }
    }

    /// The `structural.*` intrinsics: the builtin types' impls of
    /// `Display`, `Debug`, `Eq`, `PartialOrd`, `Ord` and `Hash` (their
    /// contracts are in `<std>/builtin_impls.sisu`).
    fn structural(&mut self, method: &str, a: &mut Args) -> Result<Value, Unwind> {
        match method {
            "display" | "debug" => {
                let debug = method == "debug";
                let f = self.pointer_of(a.get(1)?)?;
                // The plain text: a spec is applied by the leaves' impls in
                // Sisu, and padding by `Formatter.finish` (std/prelude.sisu).
                let v = self.target(a.get(0)?)?;
                let text = if debug { self.debug_string(v) } else { self.display_string(v) };
                match self.write(&f)? {
                    Value::Struct(fm) => {
                        if let Some(Value::String(o)) = fm.fields.get_mut(0) {
                            o.push_str(&text);
                        }
                        Ok(Value::Unit)
                    }
                    _ => Err(ice("a Formatter changed while it was written")),
                }
            }
            "eq" => Ok(Value::Bool(self.equal(a.get(0)?, a.get(1)?)?)),
            "partial_cmp" => {
                let o = self.compare(a.get(0)?, a.get(1)?)?;
                Ok(Value::option(o.map(Value::ordering)))
            }
            "cmp" => Ok(Value::ordering(self.total_compare(a.get(0)?, a.get(1)?)?)),
            "hash" => {
                // FNV-1a over the structural key's hash, into the state.
                use std::hash::{Hash, Hasher};
                let k = self.key_of(a.get(0)?)?;
                let mut h = std::collections::hash_map::DefaultHasher::new();
                k.hash(&mut h);
                let mixed = h.finish() as i64;
                let p = self.pointer_of(a.get(1)?)?;
                match self.write(&p)? {
                    Value::Struct(hs) => {
                        if let Some(Value::Int(st)) = hs.fields.get_mut(0) {
                            *st = (*st ^ mixed).wrapping_mul(1_099_511_628_211);
                        }
                        Ok(Value::Unit)
                    }
                    other => Err(ice(format!("hashing into {other:?}"))),
                }
            }
            other => Err(ice(format!("no structural operation `{other}`"))),
        }
    }

    /// The parts a failed check shows: `(column, Debug text)` pairs.
    fn shown_parts(&self, v: &Value) -> Result<Vec<(usize, String)>, Unwind> {
        let Value::List(xs) = self.target(v)? else { return Ok(Vec::new()) };
        let mut out = Vec::new();
        for x in xs {
            if let Value::Tuple(t) = x
                && let [c, s] = t.as_slice()
            {
                out.push((self.int(c)?.max(0) as usize, self.text(s)?.to_string()));
            }
        }
        Ok(out)
    }

    /// The intrinsics `check` runs on (17, "Property-based testing").
    fn test_intrinsic(&mut self, method: &str, a: &Args) -> Result<Value, Unwind> {
        match method {
            "case_seed" => Ok(Value::Int(self.case_seed as i64)),
            "replay_case" => Ok(Value::option(
                self.replay_case.take().map(|c| Value::List(c.into_iter().map(Value::Int).collect())),
            )),
            "record_case" => {
                let mut out = Vec::new();
                if let Value::List(xs) = self.target(a.get(0)?)? {
                    for x in xs {
                        out.push(self.int(x)?);
                    }
                }
                self.recorded_case = Some(out);
                Ok(Value::Unit)
            }
            // `assert_snapshot`: the value's rendering against the stored
            // one (17, "Snapshots and transcripts").
            "snapshot" => {
                let text = self.text(a.get(0)?)?.to_string();
                // One interpreter runs one test: these are its snapshots.
                let k = 1 + self.snapshots_taken.len();
                let key = crate::snapshot_key(&self.test_name, k);
                self.snapshots_taken.push((key.clone(), text.clone()));
                if self.update {
                    return Ok(Value::Unit);
                }
                match self.snapshots.get(&key) {
                    Some(old) if *old == text => Ok(Value::Unit),
                    Some(old) => Err(panic(
                        msg::TEST_SNAPSHOT_CHANGED,
                        &[("name", key), ("diff", crate::diff::lines(old, &text))],
                    )),
                    None => Err(panic(msg::TEST_SNAPSHOT_MISSING, &[("name", key), ("value", text)])),
                }
            }
            _ => Err(ice(format!("unknown intrinsic `test.{method}`"))),
        }
    }

    /// `contract.fail(kind, owner, clause, parts)`: a contract's clause did
    /// not hold (07: "A contract violation is a panic").
    fn contract_fail(&mut self, a: &Args) -> Result<Value, Unwind> {
        let kind = self.int(a.get(0)?)?;
        let owner = self.text(a.get(1)?)?.to_string();
        let clause = self.text(a.get(2)?)?.to_string();
        let parts = self.shown_parts(a.get(3)?)?;
        if kind == 3 {
            let value = parts.first().map(|(_, v)| v.clone()).unwrap_or_default();
            let message = render(
                msg::CONTRACT_REFINEMENT_FAILED,
                &[("value", value.clone()), ("ty", owner.clone()), ("clause", clause.clone())],
            );
            return Err(Unwind::Panic(Box::new(crate::PanicInfo {
                message,
                stack: Vec::new(),
                hole: None,
                refinement: Some((owner, value, clause)),
                too_long: None,
            })));
        }
        let (id, key) = match kind {
            0 => (msg::CONTRACT_REQUIRES_FAILED, "function"),
            1 => (msg::CONTRACT_ENSURES_FAILED, "function"),
            _ => (msg::CONTRACT_INVARIANT_FAILED, "ty"),
        };
        let mut m = render(id, &[(key, owner), ("clause", clause.clone())]);
        m.push_str(&crate::format::parts_diagram(&clause, &parts));
        Err(panic(msg::PANIC_EXPLICIT, &[("message", m)]))
    }

    fn assert_fail(&mut self, method: &str, a: &Args) -> Result<Value, Unwind> {
        let expr = self.text(a.get(0)?)?.to_string();
        let message = self.text(a.get(1)?)?.to_string();
        let mut m = if message.is_empty() {
            render(msg::PANIC_ASSERT, &[("expr", expr.clone())])
        } else {
            render(msg::PANIC_ASSERT_MESSAGE, &[("expr", expr.clone()), ("message", message)])
        };
        if method == "fail_parts" {
            let parts = self.shown_parts(a.get(2)?)?;
            m.push_str(&crate::format::parts_diagram(&expr, &parts));
        }
        if method == "fail_compare" {
            m.push_str("\n  ");
            m.push_str(&render(
                msg::RUN_ASSERT_LEFT,
                &[("value", self.debug_string(self.target(a.get(2)?)?))],
            ));
            m.push_str("\n  ");
            m.push_str(&render(
                msg::RUN_ASSERT_RIGHT,
                &[("value", self.debug_string(self.target(a.get(3)?)?))],
            ));
        }
        Err(panic(msg::PANIC_EXPLICIT, &[("message", m)]))
    }

    fn error_method(&mut self, method: &str, tys: &[sir::Type], a: &Args) -> Result<Value, Unwind> {
        match method {
            "trace_push" => {
                let at: Rc<str> = Rc::from(self.text(a.get(1)?)?);
                let p = self.pointer_of(a.get(0)?)?;
                if let Value::Variant(v) = self.write(&p)? {
                    v.trace.push(at);
                }
                Ok(Value::Unit)
            }
            "trace" => {
                let lines: Vec<String> =
                    self.trace_lines(a.get(0)?).into_iter().map(|l| format!("at {l}")).collect();
                Ok(Value::String(lines.join("\n")))
            }
            "is" => {
                let Some(sir::Type::Named(decl, _)) = tys.first() else {
                    return Err(ice("error.is without a type"));
                };
                Ok(Value::Bool(matches!(self.target(a.get(0)?)?, Value::Variant(v) if *v.ty == **decl)))
            }
            other => Err(ice(format!("no error operation `{other}`"))),
        }
    }
}

impl Interp<'_, '_> {
    /// `Shared` and `Weak` (their contracts are in the prelude).
    fn shared_method(&mut self, owner: &str, method: &str, a: &mut Args) -> Result<Value, Unwind> {
        match (owner, method) {
            // A site the escape analysis tagged may-cross counts atomically.
            ("Shared", "new" | "new_atomic") => {
                let value = a.take(0)?;
                self.serial += 1;
                let id = self.heap.len() as u32;
                let atomic = method == "new_atomic";
                if atomic {
                    self.sched.stats.shared_crossing += 1;
                } else {
                    self.sched.stats.shared_local += 1;
                }
                // The construction site, for `--stats shared`.
                let site = if self.stats_kind == crate::StatsKind::Shared && self.sched.keep_stats {
                    let (function, source) = self
                        .frames
                        .last()
                        .and_then(|f| f.func)
                        .and_then(|fi| self.func(fi))
                        .map(|f| (f.name.clone(), f.source))
                        .unwrap_or_default();
                    let loc = self.frames.last().and_then(|f| f.loc);
                    let sites = &mut self.sched.stats.sites;
                    let i = match sites
                        .iter()
                        .position(|s| s.function == function && s.loc == loc && s.source == source)
                    {
                        Some(i) => i,
                        None => {
                            sites.push(crate::sched::SharedSite {
                                function,
                                loc,
                                source,
                                atomic,
                                ..Default::default()
                            });
                            sites.len() - 1
                        }
                    };
                    sites[i].cells += 1;
                    Some(i as u32)
                } else {
                    None
                };
                self.heap.push(crate::value::HeapCell {
                    value,
                    strong: 1,
                    weak: 0,
                    serial: self.serial,
                    atomic,
                    creator: self.sched.cur,
                    crossed: false,
                    label: None,
                    site,
                });
                Ok(Value::Shared(id))
            }
            ("Shared", "get") => {
                let id = self.shared_id(a.get(0)?)?;
                self.note_shared_access(id)?;
                let serial = self.heap.get(id as usize).map(|c| c.serial).unwrap_or(0);
                let p = crate::value::Pointer {
                    task: 0,
                    frame: crate::value::HEAP,
                    serial,
                    local: id,
                    path: Vec::new(),
                    tag: 0,
                };
                Ok(Value::Ref(Rc::new(p)))
            }
            ("Shared", "downgrade") => {
                let id = self.shared_id(a.get(0)?)?;
                self.note_shared_access(id)?;
                if let Some(c) = self.heap.get_mut(id as usize) {
                    c.weak += 1;
                }
                Ok(Value::Weak(id))
            }
            ("Shared", "count") => {
                let id = self.shared_id(a.get(0)?)?;
                self.note_shared_access(id)?;
                Ok(Value::Int(self.heap.get(id as usize).map(|c| c.strong).unwrap_or(0) as i64))
            }
            ("Weak", "upgrade") => {
                let id = match self.target(a.get(0)?)? {
                    Value::Weak(id) => *id,
                    other => return Err(ice(format!("Weak.upgrade on {other:?}"))),
                };
                self.note_shared_access(id)?;
                match self.heap.get_mut(id as usize) {
                    Some(c) if c.strong > 0 => {
                        c.strong += 1;
                        Ok(Value::some(Value::Shared(id)))
                    }
                    _ => Ok(Value::none()),
                }
            }
            _ => Err(ice(format!("unknown intrinsic `{owner}.{method}`"))),
        }
    }

    fn shared_id(&self, v: &Value) -> Result<u32, Unwind> {
        match self.target(v)? {
            Value::Shared(id) => Ok(*id),
            other => Err(ice(format!("expected a Shared, found {other:?}"))),
        }
    }

    /// `Cell`: the value is replaced through a shared reference, which no
    /// reference into the value can outlive (none is ever given out).
    fn cell_method(&mut self, method: &str, a: &mut Args) -> Result<Value, Unwind> {
        match method {
            "new" => Ok(Value::Cell(Box::new(a.take(0)?))),
            "get" => match self.target(a.get(0)?)? {
                Value::Cell(v) => Ok((**v).clone()),
                other => Err(ice(format!("Cell.get on {other:?}"))),
            },
            "set" | "replace" => {
                let p = self.pointer_of(a.get(0)?)?;
                let new = a.take(1)?;
                let old = match self.write(&p)? {
                    Value::Cell(v) => std::mem::replace(&mut **v, new),
                    other => return Err(ice(format!("Cell.{method} on {other:?}"))),
                };
                if method == "replace" {
                    return Ok(old);
                }
                self.queue_drop(old, None);
                Ok(Value::Unit)
            }
            other => Err(ice(format!("unknown intrinsic `Cell.{other}`"))),
        }
    }

    /// `Mutex` in one task (06; D-077 for a guard dropped by unwinding).
    fn mutex_method(&mut self, method: &str, a: &mut Args) -> Result<Value, Unwind> {
        match method {
            "new" => Ok(Value::Mutex(Box::new(crate::value::MutexVal {
                value: a.take(0)?,
                locked: false,
                holder: 0,
                poisoned: false,
            }))),
            "lock" => {
                let p = self.pointer_of(a.get(0)?)?;
                match self.write(&p)? {
                    Value::Mutex(m) if m.poisoned => Err(panic(msg::PANIC_MUTEX_POISONED, &[])),
                    Value::Mutex(m) if m.locked => Err(panic(msg::PANIC_MUTEX_HELD, &[])),
                    Value::Mutex(m) => {
                        m.locked = true;
                        Ok(Value::Guard(Box::new(p)))
                    }
                    other => Err(ice(format!("Mutex.lock on {other:?}"))),
                }
            }
            "is_poisoned" => match self.target(a.get(0)?)? {
                Value::Mutex(m) => Ok(Value::Bool(m.poisoned)),
                other => Err(ice(format!("Mutex.is_poisoned on {other:?}"))),
            },
            "into_inner" => match a.take(0)? {
                Value::Mutex(m) if m.poisoned => {
                    self.queue_drop(m.value, None);
                    Err(panic(msg::PANIC_INTO_INNER_POISONED, &[]))
                }
                Value::Mutex(m) => Ok(m.value),
                other => Err(ice(format!("Mutex.into_inner on {other:?}"))),
            },
            other => Err(ice(format!("unknown intrinsic `Mutex.{other}`"))),
        }
    }

    /// `Arena` and its generational keys (04).
    fn arena_method(&mut self, method: &str, a: &mut Args) -> Result<Value, Unwind> {
        let key = |index: usize, generation: i64| {
            Value::Struct(Value::adt("Key", 0, vec![Value::Int(index as i64), Value::Int(generation)]))
        };
        match method {
            "new" => Ok(Value::Arena(Box::default())),
            "insert" => {
                let p = self.pointer_of(a.get(0)?)?;
                let value = a.take(1)?;
                match self.write(&p)? {
                    Value::Arena(ar) => {
                        let (index, generation) = match ar.free.pop() {
                            Some(i) => {
                                let slot = &mut ar.slots[i];
                                slot.1 = value;
                                (i, slot.0)
                            }
                            None => {
                                ar.slots.push((0, value));
                                (ar.slots.len() - 1, 0)
                            }
                        };
                        Ok(key(index, generation))
                    }
                    other => Err(ice(format!("Arena.insert on {other:?}"))),
                }
            }
            "get" | "get_mut" | "contains" => {
                let p = self.pointer_of(a.get(0)?)?;
                let (index, generation) = crate::place::arena_key(self.target(a.get(1)?)?)?;
                let live = match self.read(&p)? {
                    Value::Arena(ar) => {
                        ar.slots.get(index).is_some_and(|(g, v)| *g == generation && !v.is_moved())
                    }
                    other => return Err(ice(format!("Arena.{method} on {other:?}"))),
                };
                if method == "contains" {
                    return Ok(Value::Bool(live));
                }
                if !live {
                    return Ok(Value::none());
                }
                Ok(Value::some(Value::Ref(Rc::new(p.join(crate::value::Step::Slot(index))))))
            }
            "remove" | "take" => {
                let p = self.pointer_of(a.get(0)?)?;
                let (index, generation) = crate::place::arena_key(self.target(a.get(1)?)?)?;
                let taken = match self.write(&p)? {
                    Value::Arena(ar) => match ar.slots.get_mut(index) {
                        Some((g, v)) if *g == generation && !v.is_moved() => {
                            // The slot's next value gets the next generation.
                            *g += 1;
                            let v = std::mem::replace(v, Value::Moved);
                            ar.free.push(index);
                            Some(v)
                        }
                        _ => None,
                    },
                    other => return Err(ice(format!("Arena.{method} on {other:?}"))),
                };
                if method == "take" {
                    return Ok(Value::option(taken));
                }
                if let Some(v) = taken {
                    self.queue_drop(v, None);
                }
                Ok(Value::Unit)
            }
            "keys" => {
                let keys: std::collections::VecDeque<Value> = match self.target(a.get(0)?)? {
                    Value::Arena(ar) => ar
                        .slots
                        .iter()
                        .enumerate()
                        .filter(|(_, (_, v))| !v.is_moved())
                        .map(|(i, (g, _))| key(i, *g))
                        .collect(),
                    other => return Err(ice(format!("Arena.keys on {other:?}"))),
                };
                Ok(Value::Iter(Box::new(IterState::Items(keys))))
            }
            other => Err(ice(format!("unknown intrinsic `Arena.{other}`"))),
        }
    }
}

/// Whether `s` is a decimal number as `str.parse_float` reads it: an
/// optional sign, digits with an optional fraction (or a fraction alone),
/// and an optional exponent; not `inf` or `nan`.
fn decimal(s: &str) -> bool {
    let b = s.as_bytes();
    let mut i = 0;
    if matches!(b.first(), Some(b'+' | b'-')) {
        i += 1;
    }
    let int_start = i;
    while b.get(i).is_some_and(u8::is_ascii_digit) {
        i += 1;
    }
    let mut digits = i - int_start;
    if b.get(i) == Some(&b'.') {
        i += 1;
        let frac_start = i;
        while b.get(i).is_some_and(u8::is_ascii_digit) {
            i += 1;
        }
        digits += i - frac_start;
    }
    if digits == 0 {
        return false;
    }
    if matches!(b.get(i), Some(b'e' | b'E')) {
        i += 1;
        if matches!(b.get(i), Some(b'+' | b'-')) {
            i += 1;
        }
        let exp_start = i;
        while b.get(i).is_some_and(u8::is_ascii_digit) {
            i += 1;
        }
        if i == exp_start {
            return false;
        }
    }
    i == b.len()
}

/// The number of representable `f64`s from `a` to `b`, counting `b`
/// (0 for equal values, `0.0` and `-0.0` included); `None` with a NaN.
fn ulps_between(a: f64, b: f64) -> Option<u64> {
    if a.is_nan() || b.is_nan() {
        return None;
    }
    // The bits in an order that follows the values': negatives below
    // positives, both counting away from zero.
    let key = |x: f64| {
        let bits = x.to_bits() as i64;
        if bits < 0 { i64::MIN - bits } else { bits }
    };
    Some(key(a).abs_diff(key(b)))
}
