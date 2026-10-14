//! Places: resolving a SIR place to a [`Pointer`], and reading and writing
//! through pointers.

use crate::value::{Pointer, Step, Value};
use crate::{Interp, Unwind, ice, msg, panic};
use sisu_sir::{Place, Proj};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Read,
    /// A missing dict key at the end of the place is inserted.
    Write,
    /// A missing dict key means there is nothing to drop.
    Drop,
}

impl Interp<'_, '_> {
    pub(crate) fn resolve(&mut self, p: &Place) -> Result<Pointer, Unwind> {
        self.resolve_mode(p, Mode::Read)?.ok_or_else(|| ice("place did not resolve"))
    }

    pub(crate) fn resolve_for_write(&mut self, p: &Place) -> Result<Pointer, Unwind> {
        self.resolve_mode(p, Mode::Write)?.ok_or_else(|| ice("place did not resolve"))
    }

    pub(crate) fn resolve_for_drop(&mut self, p: &Place) -> Result<Option<Pointer>, Unwind> {
        self.resolve_mode(p, Mode::Drop)
    }

    fn resolve_mode(&mut self, p: &Place, mode: Mode) -> Result<Option<Pointer>, Unwind> {
        let top = self.frames.len().checked_sub(1).ok_or_else(|| ice("no frame"))?;
        let serial = self.frames[top].serial;
        let mut ptr = Pointer { frame: top as u32, serial, local: p.local.0, path: Vec::new(), tag: 0 };
        let n = p.proj.len();
        for (idx, proj) in p.proj.iter().enumerate() {
            match proj {
                Proj::Deref => match self.read(&ptr)? {
                    Value::Ref(q) => ptr = (**q).clone(),
                    Value::Moved => return Err(self.moved_at(&ptr)),
                    other => return Err(ice(format!("dereferencing {other:?}"))),
                },
                Proj::Field(i) => ptr.path.push(Step::Field(*i)),
                Proj::Downcast(v) => {
                    // Dropping the payload of a variant the value does not
                    // hold drops nothing (the drop elaboration relies on it).
                    if mode == Mode::Drop
                        && !matches!(self.target(self.read(&ptr)?)?, Value::Variant(a) if a.variant == *v)
                    {
                        return Ok(None);
                    }
                    ptr.path.push(Step::Payload(*v))
                }
                Proj::Index(l) | Proj::IndexFromEnd(l) => {
                    let i = match self.target(self.local(*l)?)? {
                        Value::Int(i) => *i,
                        other => return Err(ice(format!("index {other:?}"))),
                    };
                    let len = match self.read(&ptr)? {
                        Value::List(xs) => xs.len() as i64,
                        other => return Err(ice(format!("indexing {other:?}"))),
                    };
                    let at = if matches!(proj, Proj::IndexFromEnd(_)) { len - i } else { i };
                    if i < 0 && matches!(proj, Proj::Index(_)) {
                        return Err(panic(msg::PANIC_NEGATIVE_INDEX, &[("index", i.to_string())]));
                    }
                    if i < 0 || at < 0 || at >= len {
                        let shown = if matches!(proj, Proj::IndexFromEnd(_)) {
                            format!("^{i}")
                        } else {
                            i.to_string()
                        };
                        return Err(panic(
                            msg::PANIC_INDEX_OUT_OF_BOUNDS,
                            &[("len", len.to_string()), ("index", shown)],
                        ));
                    }
                    ptr.path.push(Step::Index(at as usize));
                }
                Proj::Key(l) => {
                    let key = self.local(*l)?.clone();
                    let k = self.key_of(&key)?;
                    let pos = match self.read(&ptr)? {
                        Value::Dict(d) => d.position(&k),
                        other => return Err(ice(format!("key lookup in {other:?}"))),
                    };
                    let pos = match pos {
                        Some(pos) => pos,
                        None if mode == Mode::Write && idx + 1 == n => {
                            let key = self.owned_key(&key)?;
                            match self.write(&ptr)? {
                                Value::Dict(d) => {
                                    d.insert(k, key, Value::Moved);
                                    d.entries.len() - 1
                                }
                                _ => return Err(ice("dict changed during a write")),
                            }
                        }
                        None if mode == Mode::Drop => return Ok(None),
                        None => {
                            let shown = self.debug_string(&key);
                            return Err(panic(msg::PANIC_MISSING_KEY, &[("key", shown)]));
                        }
                    };
                    ptr.path.push(Step::DictVal(pos));
                }
            }
        }
        Ok(Some(ptr))
    }

    /// The value at a pointer.
    pub(crate) fn read(&self, p: &Pointer) -> Result<&Value, Unwind> {
        let frame = self.frames.get(p.frame as usize).filter(|f| f.serial == p.serial);
        let frame = match frame {
            Some(f) => f,
            None if self.monitoring() => return Err(self.mon_dangling(p)),
            None => return Err(dangling()),
        };
        let mut v = frame.locals.get(p.local as usize).ok_or_else(|| ice("pointer to a missing local"))?;
        for s in &p.path {
            if v.is_moved() {
                return Err(self.moved_at(p));
            }
            v = step(v, *s)?;
        }
        Ok(v)
    }

    /// A read of a place that holds no value (moved out, or never
    /// assigned): a panic, or under the monitor a use-after-move violation.
    pub(crate) fn moved_at(&self, p: &Pointer) -> Unwind {
        if self.monitoring() { self.mon_moved_at(p) } else { panic(msg::PANIC_MOVED_VALUE, &[]) }
    }

    pub(crate) fn write(&mut self, p: &Pointer) -> Result<&mut Value, Unwind> {
        if !self.frames.get(p.frame as usize).is_some_and(|f| f.serial == p.serial) {
            return Err(if self.monitoring() { self.mon_dangling(p) } else { dangling() });
        }
        // A write into a part of a moved value is a use after move.
        if let Some((_, init)) = p.path.split_last()
            && self.read(&Pointer { path: init.to_vec(), ..p.clone() })?.is_moved()
        {
            return Err(self.moved_at(p));
        }
        let frame =
            self.frames.get_mut(p.frame as usize).filter(|f| f.serial == p.serial).ok_or_else(dangling)?;
        let root = frame.locals.get_mut(p.local as usize).ok_or_else(|| ice("pointer to a missing local"))?;
        walk_mut(root, &p.path)
    }

    /// Follows references to the value they point at.
    pub(crate) fn target<'v>(&'v self, mut v: &'v Value) -> Result<&'v Value, Unwind> {
        while let Value::Ref(p) = v {
            v = self.read(p)?;
        }
        Ok(v)
    }

    /// The pointer to the non-reference value a reference leads to.
    pub(crate) fn pointer_of(&self, v: &Value) -> Result<Pointer, Unwind> {
        let Value::Ref(p) = v else { return Err(ice(format!("expected a reference, found {v:?}"))) };
        let mut p = (**p).clone();
        loop {
            match self.read(&p)? {
                Value::Ref(q) => p = (**q).clone(),
                _ => return Ok(p),
            }
        }
    }

    /// The text of a `str`, `String` or `Path`, through references.
    pub(crate) fn text<'v>(&'v self, v: &'v Value) -> Result<&'v str, Unwind> {
        match self.target(v)? {
            Value::Str(s, tag) => {
                self.mon_view_ok(*tag)?;
                Ok(s)
            }
            Value::Path(s) => Ok(s),
            Value::String(s) => Ok(s),
            other => Err(ice(format!("expected text, found {other:?}"))),
        }
    }

    pub(crate) fn int(&self, v: &Value) -> Result<i64, Unwind> {
        match self.target(v)? {
            Value::Int(i) => Ok(*i),
            other => Err(ice(format!("expected an int, found {other:?}"))),
        }
    }

    pub(crate) fn float(&self, v: &Value) -> Result<f64, Unwind> {
        match self.target(v)? {
            Value::Float(f) => Ok(*f),
            other => Err(ice(format!("expected an f64, found {other:?}"))),
        }
    }

    pub(crate) fn bool(&self, v: &Value) -> Result<bool, Unwind> {
        match self.target(v)? {
            Value::Bool(b) => Ok(*b),
            other => Err(ice(format!("expected a bool, found {other:?}"))),
        }
    }

    pub(crate) fn char(&self, v: &Value) -> Result<char, Unwind> {
        match self.target(v)? {
            Value::Char(c) => Ok(*c),
            other => Err(ice(format!("expected a char, found {other:?}"))),
        }
    }

    /// A key value as the dict stores it: references are replaced by
    /// copies of what they point at.
    pub(crate) fn owned_key(&self, v: &Value) -> Result<Value, Unwind> {
        Ok(self.target(v)?.clone())
    }
}

fn dangling() -> Unwind {
    panic(msg::PANIC_DANGLING, &[])
}

fn step(v: &Value, s: Step) -> Result<&Value, Unwind> {
    let r = match (s, v) {
        (Step::Field(i), Value::Tuple(xs)) => xs.get(i as usize),
        (Step::Field(i), Value::Struct(a) | Value::Variant(a)) => a.fields.get(i as usize),
        (Step::Payload(k), Value::Variant(a)) if a.variant == k => Some(v),
        (Step::Index(i), Value::List(xs)) => {
            return xs.get(i).ok_or_else(|| {
                panic(
                    msg::PANIC_INDEX_OUT_OF_BOUNDS,
                    &[("len", xs.len().to_string()), ("index", i.to_string())],
                )
            });
        }
        (Step::DictKey(i), Value::Dict(d)) => d.entries.get(i).map(|e| &e.0),
        (Step::DictVal(i), Value::Dict(d)) => d.entries.get(i).map(|e| &e.1),
        (Step::SetItem(i), Value::Set(s)) => s.items.get(i),
        _ => None,
    };
    r.ok_or_else(|| ice(format!("{s:?} does not apply to {v:?}")))
}

fn walk_mut<'v>(mut v: &'v mut Value, path: &[Step]) -> Result<&'v mut Value, Unwind> {
    for &s in path {
        v = match (s, v) {
            (Step::Field(i), Value::Tuple(xs)) => xs.get_mut(i as usize),
            (Step::Field(i), Value::Struct(a) | Value::Variant(a)) => a.fields.get_mut(i as usize),
            (Step::Payload(k), v) => {
                let ok = matches!(&*v, Value::Variant(a) if a.variant == k);
                if ok { Some(v) } else { None }
            }
            (Step::Index(i), Value::List(xs)) => {
                let len = xs.len();
                Some(xs.get_mut(i).ok_or_else(|| {
                    panic(
                        msg::PANIC_INDEX_OUT_OF_BOUNDS,
                        &[("len", len.to_string()), ("index", i.to_string())],
                    )
                })?)
            }
            (Step::DictKey(i), Value::Dict(d)) => d.entries.get_mut(i).map(|e| &mut e.0),
            (Step::DictVal(i), Value::Dict(d)) => d.entries.get_mut(i).map(|e| &mut e.1),
            (Step::SetItem(i), Value::Set(s)) => s.items.get_mut(i),
            (s, _) => return Err(ice(format!("{s:?} does not apply to the value"))),
        }
        .ok_or_else(|| ice(format!("{s:?} is out of range")))?;
    }
    Ok(v)
}
