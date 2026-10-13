//! Iterators: `for` loops, `Iterable[T]` arguments and the `Iter` methods.

use crate::value::{DictPart, IterState, Step, Value};
use crate::{Interp, Unwind, ice, msg, panic};
use sisu_sir as sir;

impl Interp<'_, '_> {
    /// The iterator over a borrowed collection (`iter.ref`, `iter.mut`):
    /// its items are references to the elements; a dict's are pairs of
    /// references to a key and its value.
    pub(crate) fn iter_borrow(&self, v: &Value) -> Result<IterState, Unwind> {
        let p = self.pointer_of(v)?;
        Ok(match self.read(&p)? {
            Value::List(_) => IterState::ListRef { base: p, idx: 0 },
            Value::Set(_) => IterState::SetRef { base: p, idx: 0 },
            Value::Dict(_) => IterState::DictRef { base: p, idx: 0, part: DictPart::Items },
            other => return Err(ice(format!("iterating over a borrowed {other:?}"))),
        })
    }

    /// The iterator that consumes a collection (`iter.own`).
    pub(crate) fn iter_own(&self, v: Value) -> Result<IterState, Unwind> {
        Ok(match v {
            Value::List(xs) => IterState::Items(xs.into()),
            Value::Set(s) => IterState::Items(s.items.into()),
            Value::Dict(d) => {
                IterState::Items(d.entries.into_iter().map(|(k, v)| Value::Tuple(vec![k, v])).collect())
            }
            Value::Iter(it) => *it,
            Value::Ref(_) => self.iter_borrow(&v)?,
            other => return Err(ice(format!("iterating over {other:?}"))),
        })
    }

    /// An `Iterable[T]` argument: a borrowed or owned collection, or an
    /// iterator. When `T` is not a reference type the items are copies of
    /// what the collection holds (the checker's decay of `ref T` to `T`
    /// for a `Copy` type).
    pub(crate) fn iterable(&self, v: Value, item: Option<&sir::Type>) -> Result<IterState, Unwind> {
        let base = match v {
            Value::Ref(_) => self.iter_borrow(&v)?,
            other => self.iter_own(other)?,
        };
        Ok(if matches!(item, Some(sir::Type::Ref(..))) { base } else { IterState::Decay(Box::new(base)) })
    }

    pub(crate) fn iter_next(&mut self, st: &mut IterState) -> Result<Option<Value>, Unwind> {
        match st {
            IterState::Range { cur, stop, step } => {
                let more = if *step > 0 { *cur < *stop } else { *cur > *stop };
                if !more {
                    return Ok(None);
                }
                let v = *cur;
                match cur.checked_add(*step) {
                    Some(n) => *cur = n,
                    None => *st = IterState::Done,
                }
                Ok(Some(Value::Int(v)))
            }
            IterState::ListRef { base, idx } => {
                let len = match self.read(base)? {
                    Value::List(xs) => xs.len(),
                    other => return Err(ice(format!("list iterator over {other:?}"))),
                };
                if *idx >= len {
                    return Ok(None);
                }
                let p = base.join(Step::Index(*idx));
                *idx += 1;
                Ok(Some(Value::Ref(Box::new(p))))
            }
            IterState::SetRef { base, idx } => {
                let len = match self.read(base)? {
                    Value::Set(s) => s.items.len(),
                    other => return Err(ice(format!("set iterator over {other:?}"))),
                };
                if *idx >= len {
                    return Ok(None);
                }
                let p = base.join(Step::SetItem(*idx));
                *idx += 1;
                Ok(Some(Value::Ref(Box::new(p))))
            }
            IterState::DictRef { base, idx, part } => {
                let len = match self.read(base)? {
                    Value::Dict(d) => d.entries.len(),
                    other => return Err(ice(format!("dict iterator over {other:?}"))),
                };
                if *idx >= len {
                    return Ok(None);
                }
                let k = Value::Ref(Box::new(base.join(Step::DictKey(*idx))));
                let v = Value::Ref(Box::new(base.join(Step::DictVal(*idx))));
                *idx += 1;
                Ok(Some(match part {
                    DictPart::Keys => k,
                    DictPart::Values => v,
                    DictPart::Items => Value::Tuple(vec![k, v]),
                }))
            }
            IterState::Items(q) => Ok(q.pop_front()),
            IterState::Chars { text, pos } => {
                let c = text.get(*pos..).and_then(|t| t.chars().next());
                if let Some(c) = c {
                    *pos += c.len_utf8();
                }
                Ok(c.map(Value::Char))
            }
            IterState::Take { inner, left } => {
                if *left <= 0 {
                    return Ok(None);
                }
                *left -= 1;
                self.iter_next(inner)
            }
            IterState::Skip { inner, n } => {
                while *n > 0 {
                    *n -= 1;
                    match self.iter_next(inner)? {
                        Some(x) => self.drop_value(x)?,
                        None => return Ok(None),
                    }
                }
                self.iter_next(inner)
            }
            IterState::Map { inner, f } => match self.iter_next(inner)? {
                Some(x) => Ok(Some(self.call_value(f, vec![x])?)),
                None => Ok(None),
            },
            IterState::Filter { inner, f } => loop {
                let Some(x) = self.iter_next(inner)? else { return Ok(None) };
                let (x, keep) = self.call_with_ref(f, x)?;
                if self.bool(&keep)? {
                    return Ok(Some(x));
                }
                self.drop_value(x)?;
            },
            IterState::Enumerate { inner, n } => match self.iter_next(inner)? {
                Some(x) => {
                    let i = *n;
                    *n += 1;
                    Ok(Some(Value::Tuple(vec![Value::Int(i), x])))
                }
                None => Ok(None),
            },
            IterState::Zip { a, b, strict } => {
                let x = self.iter_next(a)?;
                let y = self.iter_next(b)?;
                match (x, y) {
                    (Some(x), Some(y)) => Ok(Some(Value::Tuple(vec![x, y]))),
                    (None, None) => Ok(None),
                    (x, y) => {
                        for v in x.into_iter().chain(y) {
                            self.drop_value(v)?;
                        }
                        if *strict { Err(panic(msg::PANIC_ZIP_LENGTHS, &[])) } else { Ok(None) }
                    }
                }
            }
            IterState::Decay(inner) => match self.iter_next(inner)? {
                Some(x @ Value::Ref(_)) => Ok(Some(self.target(&x)?.clone())),
                other => Ok(other),
            },
            IterState::Done => Ok(None),
        }
    }

    /// Calls `f` with a reference to `x`, which lives in a scratch frame
    /// for the call; returns `x` and the result. A result that refers into
    /// the scratch frame (a key function returning its argument) is
    /// replaced by a copy of what it refers to before the frame goes.
    pub(crate) fn call_with_ref(&mut self, f: &Value, x: Value) -> Result<(Value, Value), Unwind> {
        let (vals, r) = self.with_scratch(vec![x], |me, ptrs| {
            let scratch = ptrs.first().map(|p| (p.frame, p.serial));
            let args = ptrs.into_iter().map(|p| Value::Ref(Box::new(p))).collect();
            let r = me.call_value(f, args)?;
            match &r {
                Value::Ref(p) if Some((p.frame, p.serial)) == scratch => Ok(me.target(&r)?.clone()),
                _ => Ok(r),
            }
        });
        let r = r?;
        Ok((vals.into_iter().next().unwrap_or(Value::Unit), r))
    }

    /// The remaining items.
    pub(crate) fn iter_collect(&mut self, st: &mut IterState) -> Result<Vec<Value>, Unwind> {
        let mut out = Vec::new();
        while let Some(x) = self.iter_next(st)? {
            out.push(x);
        }
        Ok(out)
    }

    /// Drops what is left in an iterator that is no longer needed.
    pub(crate) fn finish_iter(&mut self, st: IterState) -> Result<(), Unwind> {
        self.drop_value(Value::Iter(Box::new(st)))
    }

    /// The `Iter` methods that consume the iterator (`own self`).
    pub(crate) fn iter_method(
        &mut self,
        name: &str,
        mut st: IterState,
        args: Vec<Value>,
    ) -> Result<Value, Unwind> {
        let mut args = args.into_iter();
        let mut arg = || args.next().unwrap_or(Value::Unit);
        Ok(match name {
            "take" => {
                let n = self.int(&arg())?;
                Value::Iter(Box::new(IterState::Take { inner: Box::new(st), left: n }))
            }
            "skip" => {
                let n = self.int(&arg())?;
                Value::Iter(Box::new(IterState::Skip { inner: Box::new(st), n }))
            }
            "map" => Value::Iter(Box::new(IterState::Map { inner: Box::new(st), f: arg() })),
            "filter" => Value::Iter(Box::new(IterState::Filter { inner: Box::new(st), f: arg() })),
            "enumerate" => Value::Iter(Box::new(IterState::Enumerate { inner: Box::new(st), n: 0 })),
            "count" => {
                let mut n = 0i64;
                while let Some(x) = self.iter_next(&mut st)? {
                    n += 1;
                    self.drop_value(x)?;
                }
                Value::Int(n)
            }
            "collect" => Value::List(self.iter_collect(&mut st)?),
            "find" => {
                let f = arg();
                while let Some(x) = self.iter_next(&mut st)? {
                    let (x, hit) = self.call_with_ref(&f, x)?;
                    if self.bool(&hit)? {
                        self.finish_iter(st)?;
                        return Ok(Value::some(x));
                    }
                    self.drop_value(x)?;
                }
                Value::none()
            }
            "all" | "any" => {
                let f = arg();
                let want = name == "any";
                while let Some(x) = self.iter_next(&mut st)? {
                    let r = self.call_value(&f, vec![x])?;
                    if self.bool(&r)? == want {
                        self.finish_iter(st)?;
                        return Ok(Value::Bool(want));
                    }
                }
                Value::Bool(!want)
            }
            "fold" => {
                let mut acc = arg();
                let f = arg();
                while let Some(x) = self.iter_next(&mut st)? {
                    acc = self.call_value(&f, vec![acc, x])?;
                }
                acc
            }
            "max_by" | "min_by" => {
                let f = arg();
                let mut best: Option<(Value, Value)> = None;
                while let Some(x) = self.iter_next(&mut st)? {
                    let (x, k) = self.call_with_ref(&f, x)?;
                    let better = match &best {
                        None => true,
                        Some((_, bk)) => {
                            let o = self.total_compare(&k, bk)?;
                            if name == "max_by" {
                                o == std::cmp::Ordering::Greater
                            } else {
                                o == std::cmp::Ordering::Less
                            }
                        }
                    };
                    if better {
                        if let Some((old, _)) = best.replace((x, k)) {
                            self.drop_value(old)?;
                        }
                    } else {
                        self.drop_value(x)?;
                    }
                }
                Value::option(best.map(|(x, _)| x))
            }
            other => return Err(ice(format!("no Iter method `{other}`"))),
        })
    }
}

/// The values an iterator owns, for drop glue.
pub(crate) fn owned_parts(st: IterState) -> Vec<Value> {
    match st {
        IterState::Items(q) => q.into_iter().collect(),
        IterState::Take { inner, .. }
        | IterState::Skip { inner, .. }
        | IterState::Map { inner, .. }
        | IterState::Filter { inner, .. }
        | IterState::Enumerate { inner, .. }
        | IterState::Decay(inner) => owned_parts(*inner),
        IterState::Zip { a, b, .. } => {
            let mut v = owned_parts(*a);
            v.extend(owned_parts(*b));
            v
        }
        _ => Vec::new(),
    }
}
