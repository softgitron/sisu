//! Iterators: `for` loops, `Iterable[T]` arguments and the `Iter` methods.

use crate::value::{DictPart, IterState, Step, Value};
use crate::{Interp, Unwind, ice, msg, panic};
use sisu_sir as sir;
use std::rc::Rc;

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
            Value::Receiver(c) => IterState::Recv(*c),
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
                self.mon_iter_use(base)?;
                let len = match self.read(base)? {
                    Value::List(xs) => xs.len(),
                    other => return Err(ice(format!("list iterator over {other:?}"))),
                };
                if *idx >= len {
                    return Ok(None);
                }
                let p = base.join(Step::Index(*idx));
                *idx += 1;
                Ok(Some(Value::Ref(Rc::new(p))))
            }
            IterState::SetRef { base, idx } => {
                self.mon_iter_use(base)?;
                let len = match self.read(base)? {
                    Value::Set(s) => s.items.len(),
                    other => return Err(ice(format!("set iterator over {other:?}"))),
                };
                if *idx >= len {
                    return Ok(None);
                }
                let p = base.join(Step::SetItem(*idx));
                *idx += 1;
                Ok(Some(Value::Ref(Rc::new(p))))
            }
            IterState::DictRef { base, idx, part } => {
                self.mon_iter_use(base)?;
                let len = match self.read(base)? {
                    Value::Dict(d) => d.entries.len(),
                    other => return Err(ice(format!("dict iterator over {other:?}"))),
                };
                if *idx >= len {
                    return Ok(None);
                }
                let k = Value::Ref(Rc::new(base.join(Step::DictKey(*idx))));
                let v = Value::Ref(Rc::new(base.join(Step::DictVal(*idx))));
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
                            self.queue_drop(v, None);
                        }
                        if *strict { Err(panic(msg::PANIC_ZIP_LENGTHS, &[])) } else { Ok(None) }
                    }
                }
            }
            IterState::Decay(inner) => match self.iter_next(inner)? {
                Some(x @ Value::Ref(_)) => Ok(Some(self.target(&x)?.clone())),
                other => Ok(other),
            },
            // A receiver inside another iterator (`enumerate(rx)`) takes
            // what has arrived; only `Iter.next` on the receiver's own
            // iterator waits (port-debt.md).
            IterState::Recv(c) => match self.recv_now(*c)? {
                Some(Value::Variant(a)) if &*a.ty == "Result" && a.variant == 0 => {
                    Ok(a.fields.into_iter().next())
                }
                Some(_) => Ok(None),
                None => Err(panic(msg::PANIC_NESTED_RECV, &[])),
            },
            // A generator inside another iterator would have to run its
            // body; it gives what it has yielded already (port-debt.md).
            IterState::Gen(t) => match self.gen_take(*t)? {
                Some(v) => Ok(Some(v)),
                None if self.gen_done(*t) => Ok(None),
                None => Err(panic(msg::PANIC_NESTED_RECV, &[])),
            },
            IterState::Done => Ok(None),
        }
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
        self.queue_drop(Value::Iter(Box::new(st)), None);
        Ok(())
    }
}

/// The values an iterator owns, for drop glue.
pub(crate) fn owned_parts(st: IterState) -> Vec<Value> {
    match st {
        IterState::Items(q) => q.into_iter().collect(),
        IterState::Enumerate { inner, .. } | IterState::Decay(inner) => owned_parts(*inner),
        IterState::Zip { a, b, .. } => {
            let mut v = owned_parts(*a);
            v.extend(owned_parts(*b));
            v
        }
        _ => Vec::new(),
    }
}
