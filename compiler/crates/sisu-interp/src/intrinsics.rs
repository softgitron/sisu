//! The host intrinsics. Each implements the contract written above its
//! `@intrinsic` declaration in compiler/std/*.sisu; the names lowering
//! introduces itself (`iter.ref`, `fmt.display`, `error.trace_push`, ...)
//! are documented in docs/architecture.md ("Lowering-only intrinsics").
//!
//! Arguments arrive as SIR passes them: a `self` or parameter the
//! prelude takes by borrow is a reference (or a copy, for a `Copy` type);
//! `mut` ones are references; `own` ones are values. The accessors below
//! look through references, so an intrinsic does not care which it got.

use crate::value::{DictPart, EntryVal, FsCap, FsStore, IterState, Pointer, Step, Value};
use crate::{Interp, IoErrorKind, Unwind, ice, msg, panic, render};
use sisu_sir as sir;
use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::BTreeMap;
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

fn ordering_of(v: &Value) -> Result<Ordering, Unwind> {
    match v {
        Value::Variant(a) if &*a.ty == "Ordering" => Ok(match a.variant {
            0 => Ordering::Less,
            1 => Ordering::Equal,
            _ => Ordering::Greater,
        }),
        other => Err(ice(format!("expected an Ordering, found {other:?}"))),
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

impl<'p> Interp<'p, '_> {
    pub(crate) fn intrinsic(
        &mut self,
        name: &str,
        tys: &[sir::Type],
        args: Vec<Value>,
        ret: Option<&sir::Type>,
    ) -> Result<Value, Unwind> {
        let mut a = Args(args);
        if let Some((owner, method)) = name.split_once('.') {
            return match owner {
                "int" | "f64" | "bool" | "char" => self.scalar_method(owner, method, &a),
                "str" => self.str_method(method, &mut a),
                "String" | "Path" => self.string_method(owner, method, &mut a),
                "list" => self.list_method(method, &mut a),
                "dict" | "Entry" => self.dict_method(owner, method, &mut a),
                "set" => self.set_method(method, &mut a),
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
                    let st = match a.take(0)? {
                        Value::Iter(st) => *st,
                        other => return Err(ice(format!("Iter.{method} on {other:?}"))),
                    };
                    let rest = a.0.into_iter().skip(1).collect();
                    self.iter_method(method, st, rest)
                }
                "Option" | "Result" => self.option_method(owner, method, &mut a),
                "Fs" | "Env" | "Clock" => self.capability(owner, method, &mut a),
                "iter" => match method {
                    "ref" | "mut" => Ok(Value::Iter(Box::new(self.iter_borrow(a.get(0)?)?))),
                    _ => {
                        let v = a.take(0)?;
                        Ok(Value::Iter(Box::new(self.iter_own(v)?)))
                    }
                },
                "fmt" => self.fmt(method, &mut a),
                "assert" => self.assert_fail(method, &a),
                "error" => self.error_method(method, tys, &a),
                "cli" if method == "parse" => self.cli_parse(tys.first(), a.get(0)?),
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
                    Value::Str(s) => s.chars().count(),
                    Value::String(s) => s.chars().count(),
                    Value::List(xs) => xs.len(),
                    Value::Set(s) => s.items.len(),
                    Value::Dict(d) => d.entries.len(),
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
            "sorted" => {
                let mut st = self.iterable(a.take(0)?, tys.first())?;
                let items = self.iter_collect(&mut st)?;
                let mut idx: Vec<usize> = (0..items.len()).collect();
                merge_sort(&mut idx, &mut |i, j| self.total_compare(&items[i], &items[j]))?;
                let mut slots: Vec<Option<Value>> = items.into_iter().map(Some).collect();
                Ok(Value::List(idx.into_iter().map(|i| slots[i].take().unwrap_or(Value::Moved)).collect()))
            }
            "min" | "max" => {
                let (x, y) = (a.take(0)?, a.take(1)?);
                let o = self.total_compare(&x, &y)?;
                let pick_y = if name == "min" { o == Ordering::Greater } else { o == Ordering::Less };
                let (keep, other) = if pick_y { (y, x) } else { (x, y) };
                self.drop_value(other)?;
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
            ("char", m) => {
                let c = self.char(&x)?;
                match m {
                    "is_digit" => Value::Bool(c.is_ascii_digit()),
                    "to_digit" => Value::option(c.to_digit(10).map(|d| Value::Int(d as i64))),
                    "is_alphabetic" => Value::Bool(c.is_alphabetic()),
                    "is_whitespace" => Value::Bool(c.is_whitespace()),
                    "is_uppercase" => Value::Bool(c.is_uppercase()),
                    "is_lowercase" => Value::Bool(c.is_lowercase()),
                    other => return Err(ice(format!("no char method `{other}`"))),
                }
            }
            (o, m) => return Err(ice(format!("no {o} method `{m}`"))),
        })
    }

    fn str_method(&mut self, method: &str, a: &mut Args) -> Result<Value, Unwind> {
        let s: Rc<str> = Rc::from(self.text(a.get(0)?)?);
        let arg_text =
            |me: &Self, i: usize| -> Result<String, Unwind> { Ok(me.text(a.get(i)?)?.to_string()) };
        let pieces =
            |v: Vec<&str>| Value::Iter(Box::new(IterState::Items(v.into_iter().map(Value::str).collect())));
        Ok(match method {
            "cmp" => {
                let o = arg_text(self, 1)?;
                Value::ordering((*s).cmp(o.as_str()))
            }
            "is_empty" => Value::Bool(s.is_empty()),
            "chars" => Value::Iter(Box::new(IterState::Chars { text: s, pos: 0 })),
            "lines" => pieces(s.lines().collect()),
            "split_whitespace" => pieces(s.split_whitespace().collect()),
            "split" => {
                let sep = arg_text(self, 1)?;
                if sep.is_empty() {
                    return Err(panic(
                        msg::PANIC_EMPTY_ARGUMENT,
                        &[("operation", "str.split".into()), ("what", "separator".into())],
                    ));
                }
                pieces(s.split(sep.as_str()).collect())
            }
            "split_once" => {
                let sep = arg_text(self, 1)?;
                Value::option(
                    s.split_once(sep.as_str()).map(|(x, y)| Value::Tuple(vec![Value::str(x), Value::str(y)])),
                )
            }
            "trim" => Value::str(s.trim()),
            "trim_start" => Value::str(s.trim_start()),
            "trim_end" => Value::str(s.trim_end()),
            "starts_with" => Value::Bool(s.starts_with(arg_text(self, 1)?.as_str())),
            "ends_with" => Value::Bool(s.ends_with(arg_text(self, 1)?.as_str())),
            "contains" => Value::Bool(s.contains(arg_text(self, 1)?.as_str())),
            "strip_prefix" => {
                let p = arg_text(self, 1)?;
                Value::str(s.strip_prefix(p.as_str()).unwrap_or(&s))
            }
            "strip_suffix" => {
                let p = arg_text(self, 1)?;
                Value::str(s.strip_suffix(p.as_str()).unwrap_or(&s))
            }
            "find" => {
                let p = arg_text(self, 1)?;
                Value::option(s.find(p.as_str()).map(|b| Value::Int(s[..b].chars().count() as i64)))
            }
            "replace" => {
                let (old, new) = (arg_text(self, 1)?, arg_text(self, 2)?);
                if old.is_empty() {
                    return Err(panic(
                        msg::PANIC_EMPTY_ARGUMENT,
                        &[("operation", "str.replace".into()), ("what", "text to replace".into())],
                    ));
                }
                Value::String(s.replace(old.as_str(), &new))
            }
            "to_lowercase" => Value::String(s.to_lowercase()),
            "to_uppercase" => Value::String(s.to_uppercase()),
            "to_string" => Value::String(s.to_string()),
            "parse_int" => Value::option(parse_int(&s).map(Value::Int)),
            "repeat" => {
                let n = self.int(a.get(1)?)?;
                if n < 0 {
                    return Err(panic(
                        msg::PANIC_NEGATIVE_COUNT,
                        &[("operation", "str.repeat".into()), ("count", n.to_string())],
                    ));
                }
                Value::String(s.repeat(n as usize))
            }
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
                        .then(|| Value::Ref(Box::new(p.join(Step::Index(i as usize))))),
                ))
            }
            "iter" => {
                let (p, _) = self.list_ptr(a.get(0)?)?;
                Ok(Value::Iter(Box::new(IterState::ListRef { base: p, idx: 0 })))
            }
            "sort" | "sort_by" => {
                let (p, len) = self.list_ptr(a.get(0)?)?;
                let f = if method == "sort_by" { Some(a.take(1)?) } else { None };
                let mut idx: Vec<usize> = (0..len).collect();
                merge_sort(&mut idx, &mut |i, j| {
                    let (x, y) = (p.join(Step::Index(i)), p.join(Step::Index(j)));
                    match &f {
                        Some(f) => {
                            let r =
                                self.call_value(f, vec![Value::Ref(Box::new(x)), Value::Ref(Box::new(y))])?;
                            ordering_of(&r)
                        }
                        None => self.total_compare(self.read(&x)?, self.read(&y)?),
                    }
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
            "clear" => {
                let old = self.with_list(a.get(0)?, |xs| Ok(std::mem::take(xs)))?;
                self.drop_value(Value::List(old))?;
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
                    self.drop_value(value)?;
                    self.drop_value(key)?;
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
            return Ok(Value::Ref(Box::new(dict.join(Step::DictVal(pos)))));
        }
        match method {
            "new" => Ok(Value::Dict(Box::default())),
            "get" => {
                let p = self.dict_ptr(a.get(0)?)?;
                let pos = self.dict_position(&p, a.get(1)?)?;
                Ok(Value::option(pos.map(|i| Value::Ref(Box::new(p.join(Step::DictVal(i)))))))
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
                        self.drop_value(key)?;
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
                        self.drop_value(default)?;
                        Ok(v)
                    }
                    (_, other) => {
                        if let Some(e) = other {
                            self.drop_value(e)?;
                        }
                        Ok(default)
                    }
                }
            }
            "map" => match (first, payload) {
                (true, Some(v)) => {
                    let f = a.take(1)?;
                    Ok(Value::some(self.call_value(&f, vec![v])?))
                }
                _ => Ok(Value::none()),
            },
            "ok" | "err" => {
                let want = method == "ok";
                match payload {
                    Some(v) if first == want => Ok(Value::some(v)),
                    Some(v) => {
                        self.drop_value(v)?;
                        Ok(Value::none())
                    }
                    None => Ok(Value::none()),
                }
            }
            other => Err(ice(format!("no {owner} method `{other}`"))),
        }
    }

    fn io_error(&self, kind: IoErrorKind, path: &str) -> Value {
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

    /// The path an `Fs` operation uses, or the error for one outside its root.
    fn fs_path(cap: &FsCap, path: &str) -> Result<String, IoErrorKind> {
        match &cap.root {
            None => Ok(path.to_string()),
            Some(root) => {
                let p = std::path::Path::new(path);
                if p.is_absolute() || p.components().any(|c| matches!(c, std::path::Component::ParentDir)) {
                    return Err(IoErrorKind::PermissionDenied);
                }
                Ok(format!("{}/{path}", root.trim_end_matches('/')))
            }
        }
    }

    fn capability(&mut self, owner: &str, method: &str, a: &mut Args) -> Result<Value, Unwind> {
        if (owner, method) == ("Fs", "in_memory") {
            let mut files = BTreeMap::new();
            if let Value::Dict(d) = self.target(a.get(0)?)? {
                for (k, v) in &d.entries {
                    files.insert(self.text(k)?.to_string(), self.text(v)?.to_string());
                }
            }
            return Ok(Value::Fs(Rc::new(FsCap {
                store: FsStore::Memory(Rc::new(RefCell::new(files))),
                read_only: false,
                root: None,
            })));
        }
        match (owner, method) {
            ("Env", "args") => Ok(Value::List(self.host.args().iter().map(|s| Value::str(s)).collect())),
            ("Env", "get") => {
                let name = self.text(a.get(1)?)?.to_string();
                Ok(Value::option(self.host.env_var(&name).map(Value::String)))
            }
            ("Clock", "now") => Ok(Value::DateTime(self.host.now())),
            ("Fs", m) => {
                let cap = match self.target(a.get(0)?)? {
                    Value::Fs(c) => c.clone(),
                    other => return Err(ice(format!("Fs.{m} on {other:?}"))),
                };
                match m {
                    "read_only" => Ok(Value::Fs(Rc::new(FsCap { read_only: true, ..(*cap).clone() }))),
                    "restrict_to" => {
                        let dir = self.text(a.get(1)?)?.to_string();
                        let root = match Self::fs_path(&cap, &dir) {
                            Ok(r) => r,
                            Err(_) => dir,
                        };
                        Ok(Value::Fs(Rc::new(FsCap { root: Some(root), ..(*cap).clone() })))
                    }
                    "read_to_string" | "write_text" | "exists" => {
                        let path = self.text(a.get(1)?)?.to_string();
                        let full = match Self::fs_path(&cap, &path) {
                            Ok(f) => f,
                            Err(k) if m == "exists" => {
                                let _ = k;
                                return Ok(Value::Bool(false));
                            }
                            Err(k) => return Ok(Value::err(self.io_error(k, &path))),
                        };
                        match m {
                            "exists" => Ok(Value::Bool(match &cap.store {
                                FsStore::Host => self.host.exists(&full),
                                FsStore::Memory(files) => files.borrow().contains_key(&full),
                            })),
                            "read_to_string" => {
                                let r = match &cap.store {
                                    FsStore::Host => self.host.read_file(&full),
                                    FsStore::Memory(files) => {
                                        files.borrow().get(&full).cloned().ok_or(IoErrorKind::NotFound)
                                    }
                                };
                                Ok(match r {
                                    Ok(text) => Value::ok(Value::String(text)),
                                    Err(k) => Value::err(self.io_error(k, &path)),
                                })
                            }
                            _ => {
                                if cap.read_only {
                                    return Ok(Value::err(
                                        self.io_error(IoErrorKind::PermissionDenied, &path),
                                    ));
                                }
                                let text = self.text(a.get(2)?)?.to_string();
                                let r = match &cap.store {
                                    FsStore::Host => self.host.write_file(&full, &text),
                                    FsStore::Memory(files) => {
                                        files.borrow_mut().insert(full, text);
                                        Ok(())
                                    }
                                };
                                Ok(match r {
                                    Ok(()) => Value::ok(Value::Unit),
                                    Err(k) => Value::err(self.io_error(k, &path)),
                                })
                            }
                        }
                    }
                    other => Err(ice(format!("no Fs method `{other}`"))),
                }
            }
            (o, m) => Err(ice(format!("no {o} method `{m}`"))),
        }
    }

    fn fmt(&mut self, method: &str, a: &mut Args) -> Result<Value, Unwind> {
        if method == "new" {
            return Ok(Value::String(String::new()));
        }
        let text = match method {
            "text" => self.text(a.get(1)?)?.to_string(),
            "display" | "debug" => {
                let spec = self.text(a.get(2)?)?.to_string();
                self.format(a.get(1)?, &spec, method == "debug")?
            }
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

    fn assert_fail(&mut self, method: &str, a: &Args) -> Result<Value, Unwind> {
        let expr = self.text(a.get(0)?)?.to_string();
        let message = self.text(a.get(1)?)?.to_string();
        let mut m = if message.is_empty() {
            render(msg::PANIC_ASSERT, &[("expr", expr)])
        } else {
            render(msg::PANIC_ASSERT_MESSAGE, &[("expr", expr), ("message", message)])
        };
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

/// `str.parse_int`: an optional sign and decimal digits, nothing else.
fn parse_int(s: &str) -> Option<i64> {
    let digits = s.strip_prefix(['+', '-']).unwrap_or(s);
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    s.parse().ok()
}
