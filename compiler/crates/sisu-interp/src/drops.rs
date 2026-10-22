//! Drops (04, rule 6), run on the run loop.
//!
//! A `drop` statement moves the value out of its place and puts it, with
//! the place's type when SIR gives one, on the frame's stack of pending
//! drops; the run loop takes the next pending drop before the frame's next
//! statement. A value of a declared type is dropped by its glue, the SIR
//! function `T#drop` that lowering made (the type's `Drop`, then its
//! fields in reverse order), which runs as a frame of its own with the
//! type's arguments. A host-backed value is taken apart here and its parts
//! go back on the stack, so the first part is dropped, with everything it
//! owns, before the second: a `list`, `set` or `Arena` first to last, a
//! `dict` entry's value before its key, a tuple last to first. A `Shared`
//! decrements its count and drops the value with its last owner, and a
//! mutex guard unlocks. A drop never nests the host stack, however deep the
//! value, and a `Drop` runs like any other call.
//!
//! Values without a type (a drop inside an intrinsic, a payload of a union
//! of error sets) are dropped by what they are; the glue of a generic type
//! then runs without its type arguments, which only a `Drop` that calls
//! through a trait on them could notice (port-debt.md).

use crate::value::{Pointer, Step, Value};
use crate::{Interp, Unwind, ice};
use sisu_sir as sir;
use sisu_sir::Type;
use std::rc::Rc;

/// A value waiting to be dropped, with its type when the drop knows it.
pub(crate) type PendingDrop = (Value, Option<Type>);

/// What the run loop does when a frame the run time pushed returns.
#[derive(Debug)]
pub(crate) enum After {
    /// The `invariant` check of the value of a mutex whose guard was
    /// dropped by unwinding (D-077): `false` poisons the mutex. Either way
    /// it is unlocked.
    Unlock(Box<Pointer>),
}

/// Whether dropping the value can do anything: values that own nothing a
/// drop could observe are not queued.
fn owns_anything(v: &Value) -> bool {
    matches!(
        v,
        Value::Struct(_)
            | Value::Variant(_)
            | Value::Tuple(_)
            | Value::List(_)
            | Value::Set(_)
            | Value::Dict(_)
            | Value::Iter(_)
            | Value::Entry(_)
            | Value::Dyn(_)
            | Value::Shared(_)
            | Value::Weak(_)
            | Value::Cell(_)
            | Value::Mutex(_)
            | Value::Guard(_)
            | Value::Arena(_)
            | Value::Sender(_)
            | Value::Receiver(_)
            | Value::Cap(_)
    )
}

/// Argument `i` of a named type.
fn arg(ty: &Option<Type>, i: usize) -> Option<Type> {
    match ty {
        Some(Type::Named(_, a)) => a.get(i).cloned(),
        _ => None,
    }
}

impl<'p> Interp<'p, '_> {
    /// Puts a value whose owner is gone on the top frame's stack of
    /// pending drops.
    pub(crate) fn queue_drop(&mut self, v: Value, ty: Option<Type>) {
        self.queue_drops(vec![(v, ty)]);
    }

    /// Queues values to be dropped in the order given.
    fn queue_drops(&mut self, items: Vec<PendingDrop>) {
        if !self.any_drop {
            return;
        }
        if let Some(fr) = self.frames.last_mut() {
            fr.drops.extend(items.into_iter().filter(|(v, _)| owns_anything(v)).rev());
        }
    }

    /// Queues values to be dropped in the order given, after every drop
    /// the top frame has pending.
    pub(crate) fn queue_drops_after(&mut self, items: Vec<PendingDrop>) {
        if !self.any_drop {
            return;
        }
        if let Some(fr) = self.frames.last_mut() {
            let items: Vec<PendingDrop> = items.into_iter().filter(|(v, _)| owns_anything(v)).rev().collect();
            fr.drops.splice(0..0, items);
        }
    }

    /// The type of a place of the running function, with its type
    /// arguments filled in.
    pub(crate) fn place_type(&self, p: &sir::Place) -> Option<Type> {
        let fr = self.frames.last()?;
        let f = self.func(fr.func?)?;
        let t = sir::types::place_ty(&|n| self.types.get(n).copied(), f, p)?;
        Some(self.concrete(&t))
    }

    /// The type a `drop` statement passes on with the value: only values
    /// whose drop may run glue need one.
    pub(crate) fn drop_type(&self, p: &sir::Place, v: &Value) -> Option<Type> {
        if self.any_drop && owns_anything(v) { self.place_type(p) } else { None }
    }

    /// Runs one pending drop of the top frame.
    pub(crate) fn drop_next(&mut self, (v, ty): PendingDrop) -> Result<(), Unwind> {
        match v {
            v @ (Value::Struct(_) | Value::Variant(_)) => {
                let name = match &v {
                    Value::Struct(a) | Value::Variant(a) => a.ty.clone(),
                    _ => return Ok(()),
                };
                if let Some(&fi) = self.glue.get(&*name) {
                    let tys = match ty {
                        Some(Type::Named(n, args)) if *n == *name => args,
                        _ => Vec::new(),
                    };
                    return self.push_frame(fi, vec![v], None, tys);
                }
                let (Value::Struct(a) | Value::Variant(a)) = v else { return Ok(()) };
                // A type without glue drops nothing, except the payload of
                // an option or result, and the handles of a `System`
                // (decision 237).
                let payload = match (&*a.ty, a.variant) {
                    ("Option", 0) | ("Result", 0) => arg(&ty, 0),
                    ("Result", 1) => arg(&ty, 1),
                    ("Option" | "Result", _) => None,
                    ("System", _) => None,
                    _ => return Ok(()),
                };
                let items = a.fields.into_iter().map(|f| (f, payload.clone())).collect();
                self.queue_drops(items);
            }
            Value::Tuple(xs) => {
                let tys: Vec<Option<Type>> = match &ty {
                    Some(Type::Tuple(ts)) => ts.iter().cloned().map(Some).collect(),
                    _ => Vec::new(),
                };
                let items = xs
                    .into_iter()
                    .enumerate()
                    .map(|(i, x)| (x, tys.get(i).cloned().flatten()))
                    .rev()
                    .collect();
                self.queue_drops(items);
            }
            Value::List(xs) => {
                let t = arg(&ty, 0);
                self.queue_drops(xs.into_iter().map(|x| (x, t.clone())).collect());
            }
            Value::Set(s) => {
                let t = arg(&ty, 0);
                self.queue_drops(s.items.into_iter().map(|x| (x, t.clone())).collect());
            }
            Value::Dict(d) => {
                let (k, t) = (arg(&ty, 0), arg(&ty, 1));
                let items = d.entries.into_iter().flat_map(|(key, val)| [(val, t.clone()), (key, k.clone())]);
                self.queue_drops(items.collect());
            }
            Value::Iter(it) => {
                // A generator dropped before its end is closed (decision
                // 236).
                if let crate::value::IterState::Gen(t) = *it {
                    return self.abandon_gen(t);
                }
                let parts = crate::iter::owned_parts(*it);
                self.queue_drops(parts.into_iter().map(|x| (x, None)).collect());
            }
            Value::Entry(e) => self.queue_drop(e.key, arg(&ty, 0)),
            Value::Dyn(d) => self.queue_drop(d.value, Some(d.ty)),
            // The last owner of a `Shared` drops its value.
            Value::Shared(id) => {
                self.note_shared_access(id)?;
                let Some(cell) = self.heap.get_mut(id as usize) else { return Ok(()) };
                cell.strong = cell.strong.saturating_sub(1);
                if cell.strong > 0 {
                    return Ok(());
                }
                let v = std::mem::replace(&mut cell.value, Value::Moved);
                let serial = cell.serial;
                self.mon_heap_freed(serial, id);
                self.queue_drop(v, arg(&ty, 0));
            }
            Value::Weak(id) => {
                if let Some(cell) = self.heap.get_mut(id as usize) {
                    cell.weak = cell.weak.saturating_sub(1);
                }
            }
            Value::Cell(v) => self.queue_drop(*v, arg(&ty, 0)),
            Value::Mutex(m) => self.queue_drop(m.value, arg(&ty, 0)),
            Value::Guard(p) => self.unlock(p, arg(&ty, 0))?,
            Value::Arena(a) => {
                let t = arg(&ty, 0);
                self.queue_drops(a.slots.into_iter().map(|(_, x)| (x, t.clone())).collect());
            }
            v @ (Value::Sender(_) | Value::Receiver(_)) => self.drop_channel_end(&v),
            // A copy of a handle goes: its count with it (decision 237); a
            // `Lent` held none.
            Value::Cap(id) if !matches!(&ty, Some(Type::Named(n, _)) if n == "Lent") => self.release_cap(id),
            _ => {}
        }
        Ok(())
    }

    /// Dropping a mutex guard unlocks the mutex. When a panic is unwinding,
    /// the protected value's type's `invariant` is checked first, as a
    /// frame of its own, and the mutex is poisoned if it fails (D-077); a
    /// type without one unlocks at once.
    fn unlock(&mut self, p: Box<Pointer>, ty: Option<Type>) -> Result<(), Unwind> {
        let unwinding = self.frames.iter().any(|f| f.pending.is_some());
        if unwinding {
            let value = p.join(Step::MutexValue);
            let check = match self.read(&value) {
                Ok(Value::Struct(a) | Value::Variant(a)) => {
                    self.fn_index(&format!("{}.__invariant", a.ty)).ok()
                }
                _ => None,
            };
            if let Some(fi) = check {
                let tys = match ty {
                    Some(Type::Named(_, args)) => args,
                    _ => Vec::new(),
                };
                self.push_frame(fi, vec![Value::Ref(Rc::new(value))], None, tys)?;
                if let Some(fr) = self.frames.last_mut() {
                    fr.after = Some(After::Unlock(p));
                }
                return Ok(());
            }
        }
        self.finish_unlock(&p, true)
    }

    /// Unlocks the mutex a guard held, poisoning it when its invariant did
    /// not hold.
    pub(crate) fn finish_unlock(&mut self, p: &Pointer, holds: bool) -> Result<(), Unwind> {
        match self.write(p)? {
            Value::Mutex(m) => {
                m.poisoned |= !holds;
                m.locked = false;
            }
            other => return Err(ice(format!("a guard on {other:?}"))),
        }
        self.wake_key(crate::sched::Key::Lock);
        Ok(())
    }

    /// What the run loop does with the result of a frame the run time
    /// pushed.
    pub(crate) fn after(&mut self, a: After, result: &Value) -> Result<(), Unwind> {
        match a {
            After::Unlock(p) => self.finish_unlock(&p, matches!(result, Value::Bool(true))),
        }
    }
}
