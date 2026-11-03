//! The dynamic ownership monitor (18, Phase 3: "every referent carries
//! shared and exclusive borrow counts; use-after-move, aliasing
//! violations, drop-while-borrowed and a view escaping its frame are
//! internal soundness errors"; principle 5, "the checker checks itself").
//!
//! Under `sisu run --monitor` (and by default in `sisu test`) every
//! borrow statement makes a run-time loan, and every reference and `str`
//! view carries the loan it was made by (its *tag*). A loan records the
//! place it borrows, whether it is `mut`, and the loan of the reference it
//! was made through (its parent), so loans form a tree per place.
//!
//! Every access checks two things. First, the reference it goes through
//! must carry a loan that is still active, with every ancestor active; a
//! use of an ended loan is the violation. Second, the access ends the
//! loans it conflicts with: a write (an assignment, a move, a drop, a
//! `mut` borrow) ends every active loan of an overlapping place other than
//! the one it goes through and that loan's ancestors; a read ends the
//! active `mut` loans of an overlapping place likewise. Ending is not yet
//! an error: the conflict is real only if the reference is used later,
//! which is exactly when the static checker's non-lexical rule calls the
//! loan live. So the monitor and the checker agree on what a conflict is,
//! and the report names both halves: the operation that ended the loan and
//! the use that came after.
//!
//! A `ref mut` is two-phase like the checker's: until its first use, its
//! loan survives reads of the place. The end of a local's storage (`dead`)
//! and the return of its frame end every loan of it; a reference into a
//! frame that has returned is the dangling-reference violation, and a read
//! of a moved-from place is the use-after-move one.
//!
//! A violation stops the program (exit status 134) with a report on
//! standard error: the operation, the loan, the value of the borrowed
//! place, the counts of loans still active on it, and the SIR of the
//! function.

use crate::value::{Pointer, Step, Value};
use crate::{Interp, Unwind, render};
use sisu_diag::msg;
use sisu_sir as sir;
use std::rc::Rc;

/// Where something happened: a statement of a function.
#[derive(Clone, Debug, Default)]
pub(crate) struct Site {
    pub func: Option<usize>,
    pub bb: usize,
    pub stmt: Option<usize>,
    pub loc: Option<sir::Loc>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Op {
    Read,
    Write,
    Move,
    Drop,
    Dead,
    /// The return of the frame that owns the place.
    Return,
    MutBorrow,
}

impl Op {
    fn writes(self) -> bool {
        !matches!(self, Op::Read)
    }

    fn msg(self) -> sisu_diag::MsgId {
        match self {
            Op::Read => msg::MONITOR_OP_READ,
            Op::Write => msg::MONITOR_OP_WRITE,
            Op::Move => msg::MONITOR_OP_MOVE,
            Op::Drop => msg::MONITOR_OP_DROP,
            Op::Dead => msg::MONITOR_OP_DEAD,
            Op::Return => msg::MONITOR_OP_RETURN,
            Op::MutBorrow => msg::MONITOR_OP_BORROW_MUT,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Ending {
    pub op: Op,
    pub site: Site,
    pub at: Pointer,
}

#[derive(Clone, Debug)]
pub(crate) struct DynLoan {
    pub at: Pointer,
    pub mutable: bool,
    /// A two-phase `ref mut` not used yet.
    pub reserved: bool,
    /// The loan of the reference this one was made through (0: none).
    pub parent: u32,
    pub ended: Option<Box<Ending>>,
    pub created: Site,
    /// The function whose frame holds the borrowed place, to name it in a
    /// report after the frame has returned.
    pub owner: Option<usize>,
}

/// The active loans of a local, shared and `mut` apart: a read ends no
/// shared loan, so it looks at the `mut` ones only.
#[derive(Default)]
pub(crate) struct Active {
    pub shared: Vec<u32>,
    pub mutable: Vec<u32>,
}

#[derive(Default)]
pub(crate) struct Monitor {
    /// Loan `t` is `loans[t - 1]`.
    pub loans: Vec<DynLoan>,
    /// The active loans of each (frame serial, local).
    pub active: crate::fx::FxMap<(u64, u32), Active>,
    /// How often each source position ran a statement: (function, line,
    /// column).
    pub coverage: Option<crate::fx::FxMap<(usize, u32, u32), u32>>,
    /// Set once a violation was reported.
    pub violated: std::cell::Cell<bool>,
}

/// How many values one use of a reference checks for ended views.
const VIEW_SCAN_BUDGET: u32 = 256;

/// Whether two paths reach overlapping places: one is a prefix of the
/// other, variant payloads being the value itself.
fn overlap(a: &[Step], b: &[Step]) -> bool {
    let place = |s: &&Step| !matches!(s, Step::Payload(_));
    a.iter().filter(place).zip(b.iter().filter(place)).all(|(x, y)| x == y)
}

enum Violation {
    Ended { tag: u32, used: Site },
    Dangling { name: String, used: Site },
    Moved { name: String, used: Site },
    Unconsumed { name: String, ty: String, used: Site },
}

impl Interp<'_, '_> {
    pub(crate) fn monitoring(&self) -> bool {
        self.monitor.is_some()
    }

    /// The statement the top frame is executing.
    pub(crate) fn site(&self) -> Site {
        match self.frames.last() {
            Some(f) => Site { func: f.func, bb: f.bb, stmt: f.next.checked_sub(1), loc: f.loc },
            None => Site::default(),
        }
    }

    fn loan(&self, tag: u32) -> Option<&DynLoan> {
        self.monitor.as_ref()?.loans.get(tag.checked_sub(1)? as usize)
    }

    /// The loan and its ancestors, nearest first.
    fn chain(&self, tag: u32) -> Vec<u32> {
        let mut out = Vec::new();
        let mut t = tag;
        while t != 0 && out.len() < 10_000 {
            out.push(t);
            t = self.loan(t).map(|l| l.parent).unwrap_or(0);
        }
        out
    }

    /// An access to the place at `ptr`, through the loan `ptr.tag`.
    pub(crate) fn mon_access(&mut self, ptr: &Pointer, op: Op) -> Result<(), Unwind> {
        if self.monitor.is_none() {
            return Ok(());
        }
        let chain = self.chain(ptr.tag);
        for t in &chain {
            if self.loan(*t).is_some_and(|l| l.ended.is_some()) {
                return Err(self.violation(Violation::Ended { tag: *t, used: self.site() }));
            }
        }
        // The first use of a two-phase `mut` loan activates it: a write.
        if let Some(&t) = chain.first()
            && self.loan(t).is_some_and(|l| l.reserved)
        {
            if let Some(m) = &mut self.monitor
                && let Some(l) = m.loans.get_mut(t as usize - 1)
            {
                l.reserved = false;
            }
            let at = self.loan(t).map(|l| l.at.clone());
            if let Some(at) = at {
                self.end_conflicting(&at, Op::MutBorrow, &chain);
            }
        }
        self.end_conflicting(ptr, op, &chain);
        Ok(())
    }

    fn end_conflicting(&mut self, ptr: &Pointer, op: Op, through: &[u32]) {
        let Some(m) = &self.monitor else { return };
        let Some(active) = m.active.get(&(ptr.serial, ptr.local)) else { return };
        // A mutex's value changes behind shared references to the mutex
        // (06, "Send, Sync, and shared state"): an access through a guard
        // ends no shared loan of the mutex or of what holds it.
        let mutexes = |p: &[Step]| p.iter().filter(|s| matches!(s, Step::MutexValue)).count();
        let inside = mutexes(&ptr.path);
        let mut ending = Vec::new();
        let shared: &[u32] = if op.writes() { &active.shared } else { &[] };
        for &t in active.mutable.iter().chain(shared) {
            if through.contains(&t) {
                continue;
            }
            let Some(l) = m.loans.get(t as usize - 1) else { continue };
            // A read ends only an active `mut` loan.
            if !(op.writes() || (l.mutable && !l.reserved)) {
                continue;
            }
            if !overlap(&l.at.path, &ptr.path) || (!l.mutable && mutexes(&l.at.path) < inside) {
                continue;
            }
            ending.push(t);
        }
        if ending.is_empty() {
            return;
        }
        let site = self.site();
        let Some(m) = &mut self.monitor else { return };
        for t in &ending {
            if let Some(l) = m.loans.get_mut(*t as usize - 1) {
                l.ended = Some(Box::new(Ending { op, site: site.clone(), at: ptr.clone() }));
            }
        }
        if let Some(active) = m.active.get_mut(&(ptr.serial, ptr.local)) {
            active.mutable.retain(|t| !ending.contains(t));
            active.shared.retain(|t| !ending.contains(t));
        }
    }

    /// A new loan of the place at `ptr`; returns its tag. A `mut` loan is
    /// reserved (two-phase) unless `active`: a reborrow, or a borrow bound
    /// to a name.
    pub(crate) fn mon_borrow(&mut self, ptr: &Pointer, mutable: bool, active: bool) -> Result<u32, Unwind> {
        if self.monitor.is_none() {
            return Ok(0);
        }
        let reborrow = active;
        let op = if mutable && reborrow { Op::MutBorrow } else { Op::Read };
        self.mon_access(ptr, op)?;
        let created = self.site();
        // The pointer knows its frame's index: no walk of a deep stack.
        let owner = self
            .frames_of(ptr.task)
            .get(ptr.frame as usize)
            .filter(|f| f.serial == ptr.serial)
            .and_then(|f| f.func);
        let Some(m) = &mut self.monitor else { return Ok(0) };
        let mut at = ptr.clone();
        at.tag = 0;
        m.loans.push(DynLoan {
            at,
            mutable,
            reserved: mutable && !reborrow,
            parent: ptr.tag,
            ended: None,
            created,
            owner,
        });
        let tag = m.loans.len() as u32;
        let active = m.active.entry((ptr.serial, ptr.local)).or_default();
        if mutable {
            active.mutable.push(tag)
        } else {
            active.shared.push(tag)
        }
        Ok(tag)
    }

    /// A copy of a `ref mut` is a reborrow: a new `mut` loan under the
    /// copied reference's loan.
    #[inline]
    pub(crate) fn mon_copied(&mut self, v: &mut Value) -> Result<(), Unwind> {
        if self.monitor.is_none() {
            return Ok(());
        }
        self.mon_copied_ref(v)
    }

    fn mon_copied_ref(&mut self, v: &mut Value) -> Result<(), Unwind> {
        if let Value::Ref(p) = v
            && p.tag != 0
            && self.loan(p.tag).is_some_and(|l| l.mutable)
        {
            let target = (**p).clone();
            let tag = self.mon_borrow(&target, true, true)?;
            Rc::make_mut(p).tag = tag;
        }
        Ok(())
    }

    /// The end of a local's storage ends every loan of it.
    pub(crate) fn mon_dead(&mut self, serial: u64, local: u32, frame: u32) {
        self.mon_end_local(serial, local, frame, Op::Dead);
    }

    fn mon_end_local(&mut self, serial: u64, local: u32, frame: u32, op: Op) {
        if self.monitor.is_none() {
            return;
        }
        let task = if frame == crate::value::HEAP || frame == crate::value::RAW { 0 } else { self.sched.cur };
        let ptr = Pointer { task, frame, serial, local, path: Vec::new(), tag: 0 };
        self.end_conflicting(&ptr, op, &[]);
    }

    /// A heap cell whose last owner was dropped ends every loan of it.
    pub(crate) fn mon_heap_freed(&mut self, serial: u64, id: u32) {
        if self.monitor.is_none() {
            return;
        }
        self.mon_end_local(serial, id, crate::value::HEAP, Op::Drop);
        if let Some(m) = &mut self.monitor {
            m.active.remove(&(serial, id));
        }
    }

    /// A block of raw memory that was released ends every loan of it
    /// (`rawmem.rs`).
    pub(crate) fn mon_raw_released(&mut self, serial: u64, block: u32) {
        if self.monitor.is_none() {
            return;
        }
        self.mon_end_local(serial, block, crate::value::RAW, Op::Drop);
        if let Some(m) = &mut self.monitor {
            m.active.remove(&(serial, block));
        }
    }

    /// A frame that returns ends every loan of its locals.
    pub(crate) fn mon_frame_gone(&mut self, serial: u64, frame: u32, locals: usize) {
        if self.monitor.is_none() {
            return;
        }
        for l in 0..locals as u32 {
            if self.monitor.as_ref().is_some_and(|m| m.active.contains_key(&(serial, l))) {
                self.mon_end_local(serial, l, frame, Op::Return);
                if let Some(m) = &mut self.monitor {
                    m.active.remove(&(serial, l));
                }
            }
        }
    }

    /// An intrinsic uses its reference arguments: reads through shared
    /// ones and writes through `mut` ones; it reads a `str` view's string.
    pub(crate) fn mon_args(&mut self, vals: &[Value]) -> Result<(), Unwind> {
        if self.monitor.is_none() {
            return Ok(());
        }
        for v in vals {
            self.mon_value_use(v, 0)?;
        }
        Ok(())
    }

    fn mon_value_use(&mut self, v: &Value, depth: u32) -> Result<(), Unwind> {
        if depth > 4 {
            return Ok(());
        }
        match v {
            Value::Ref(p) => {
                let op = if self.loan(p.tag).is_some_and(|l| l.mutable) { Op::Write } else { Op::Read };
                self.mon_access(p, op)?;
                // Reading a value reads the views it holds: a list of `str`
                // whose `String` is gone is a use of ended loans.
                let mut at = (**p).clone();
                at.tag = 0;
                let mut budget = VIEW_SCAN_BUDGET;
                self.mon_held_views(self.read(&at)?, 0, &mut budget)
            }
            Value::Str(_, tag) if *tag != 0 => self.mon_view_use(*tag),
            Value::Entry(e) => self.mon_access(&e.dict, Op::Write),
            Value::Tuple(xs) => {
                for x in xs {
                    self.mon_value_use(x, depth + 1)?;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    /// Checks that every view and reference a value holds still has an
    /// active loan, up to `budget` values (a bound on the monitor's cost
    /// per use; docs/architecture.md lists it among the monitor's gaps).
    fn mon_held_views(&self, v: &Value, depth: u32, budget: &mut u32) -> Result<(), Unwind> {
        if depth > 8 || *budget == 0 {
            return Ok(());
        }
        *budget -= 1;
        match v {
            Value::Str(_, tag) => self.mon_view_ok(*tag),
            Value::Ref(p) => self.mon_view_ok(p.tag),
            Value::Tuple(xs) | Value::List(xs) => {
                xs.iter().try_for_each(|x| self.mon_held_views(x, depth + 1, budget))
            }
            Value::Struct(a) | Value::Variant(a) => {
                a.fields.iter().try_for_each(|x| self.mon_held_views(x, depth + 1, budget))
            }
            Value::Set(s) => s.items.iter().try_for_each(|x| self.mon_held_views(x, depth + 1, budget)),
            Value::Dict(d) => d.entries.iter().try_for_each(|(k, x)| {
                self.mon_held_views(k, depth + 1, budget)?;
                self.mon_held_views(x, depth + 1, budget)
            }),
            Value::Dyn(d) => self.mon_held_views(&d.value, depth + 1, budget),
            Value::Closure(c) => c.env.iter().try_for_each(|x| self.mon_held_views(x, depth + 1, budget)),
            _ => Ok(()),
        }
    }

    /// A value read by an operator: through a reference, a read of what it
    /// points to; a view, a read of its text.
    pub(crate) fn mon_read_value(&mut self, v: &Value) -> Result<(), Unwind> {
        if self.monitor.is_none() {
            return Ok(());
        }
        match v {
            Value::Ref(p) => {
                self.mon_access(p, Op::Read)?;
                let mut at = (**p).clone();
                at.tag = 0;
                let mut budget = VIEW_SCAN_BUDGET;
                self.mon_held_views(self.read(&at)?, 0, &mut budget)
            }
            Value::Str(_, tag) if *tag != 0 => self.mon_view_use(*tag),
            Value::Tuple(xs) => xs.iter().try_for_each(|x| self.mon_read_value(x)),
            _ => Ok(()),
        }
    }

    /// A value passed to a function (or called, a closure): every
    /// reference and view it holds must still have an active loan. The
    /// callee's accesses through them are checked as it makes them.
    pub(crate) fn mon_passed(&self, v: &Value) -> Result<(), Unwind> {
        if self.monitor.is_none() {
            return Ok(());
        }
        let mut budget = VIEW_SCAN_BUDGET;
        match v {
            Value::Closure(c) => c.env.iter().try_for_each(|x| self.mon_held_views(x, 0, &mut budget)),
            _ => self.mon_held_views(v, 0, &mut budget),
        }
    }

    /// A use of a `str` view: a read of the string it views.
    pub(crate) fn mon_view_use(&mut self, tag: u32) -> Result<(), Unwind> {
        let Some(at) = self.loan(tag).map(|l| l.at.clone()) else { return Ok(()) };
        let mut p = at;
        p.tag = tag;
        self.mon_access(&p, Op::Read)
    }

    /// A borrowed iterator reads its collection through its loan.
    pub(crate) fn mon_iter_use(&mut self, base: &Pointer) -> Result<(), Unwind> {
        if self.monitor.is_none() || base.tag == 0 {
            return Ok(());
        }
        let chain = self.chain(base.tag);
        for t in &chain {
            if self.loan(*t).is_some_and(|l| l.ended.is_some()) {
                return Err(self.violation(Violation::Ended { tag: *t, used: self.site() }));
            }
        }
        Ok(())
    }

    /// A drop on a normal path (not unwinding) of a value holding a
    /// must-consume value, outside the methods of that value's type, where
    /// its values end (D-054; the checker's rule, observed).
    pub(crate) fn mon_drop_unconsumed(&self, at: &Pointer, v: &Value) -> Result<(), Unwind> {
        if self.monitor.is_none() {
            return Ok(());
        }
        let Some(fr) = self.frames.last() else { return Ok(()) };
        let Some(f) = fr.func.and_then(|i| self.func(i)) else { return Ok(()) };
        // Unwinding may drop anything; glue drops the parts of a value
        // whose own drop was checked.
        if f.blocks.get(fr.bb).is_some_and(|b| b.cleanup) || f.kind == sir::FnKind::Glue {
            return Ok(());
        }
        let Some(ty) = self.unconsumed(v, 0) else { return Ok(()) };
        if f.name.strip_prefix(&*ty).is_some_and(|rest| rest.starts_with('.')) {
            return Ok(());
        }
        let name = self.pointer_label(at);
        Err(self.violation(Violation::Unconsumed { name, ty: ty.to_string(), used: self.site() }))
    }

    /// The type of a must-consume value that `v` holds, if any.
    fn unconsumed(&self, v: &Value, depth: u32) -> Option<std::rc::Rc<str>> {
        if depth > 64 {
            return None;
        }
        let parts: Vec<&Value> = match v {
            Value::Struct(a) | Value::Variant(a) => {
                let mc = match self.types.get(&*a.ty) {
                    Some(
                        sir::TypeDecl::Struct { must_consume, .. } | sir::TypeDecl::Enum { must_consume, .. },
                    ) => *must_consume,
                    None => false,
                };
                if mc {
                    return Some(a.ty.clone());
                }
                a.fields.iter().collect()
            }
            Value::Tuple(xs) | Value::List(xs) => xs.iter().collect(),
            Value::Set(s) => s.items.iter().collect(),
            Value::Dict(d) => d.entries.iter().flat_map(|e| [&e.0, &e.1]).collect(),
            Value::Iter(it) => match &**it {
                crate::value::IterState::Items(xs) => xs.iter().collect(),
                _ => Vec::new(),
            },
            _ => Vec::new(),
        };
        parts.into_iter().find_map(|x| self.unconsumed(x, depth + 1))
    }

    /// A use of a place that holds no value.
    pub(crate) fn mon_moved_at(&self, p: &Pointer) -> Unwind {
        let name = self.pointer_label(p);
        self.violation(Violation::Moved { name, used: self.site() })
    }

    /// A `Shared` tagged task-local reached by another task: the escape
    /// analysis is wrong (18, Phase 6: a soundness error).
    pub(crate) fn mon_shared_crossed(&self, cell: u32) -> Unwind {
        let mut lines =
            vec![render(msg::MONITOR_HEADER, &[("what", render(msg::MONITOR_SHARED_CROSSED, &[]))])];
        lines.push(format!("  cell {cell}"));
        for fr in self.frames.iter().rev() {
            let Some(f) = fr.func.and_then(|i| self.func(i)) else { continue };
            if f.kind == sir::FnKind::Glue {
                continue;
            }
            let s = Site { func: fr.func, bb: fr.bb, stmt: None, loc: fr.loc };
            lines.push(format!(
                "  {}",
                render(msg::RUN_FRAME, &[("function", f.name.clone()), ("location", self.site_location(&s))])
            ));
        }
        if let Some(m) = &self.monitor {
            m.violated.set(true);
        }
        Unwind::Abort(lines.join("\n"))
    }

    pub(crate) fn mon_dangling(&self, p: &Pointer) -> Unwind {
        // A cell of the heap is named by what holds it (a `Shared`, a
        // capability handle, a `Cap`), when it says.
        let label = (p.frame == crate::value::HEAP)
            .then(|| self.heap.get(p.local as usize).and_then(|c| c.label.clone()))
            .flatten();
        let name = match label {
            Some(l) => l.to_string(),
            None => format!("_{}", p.local),
        };
        self.violation(Violation::Dangling { name, used: self.site() })
    }

    /// Whether a `str` view's loan, and every loan above it, is active.
    pub(crate) fn mon_view_ok(&self, tag: u32) -> Result<(), Unwind> {
        if self.monitor.is_none() || tag == 0 {
            return Ok(());
        }
        for t in self.chain(tag) {
            if self.loan(t).is_some_and(|l| l.ended.is_some()) {
                return Err(self.violation(Violation::Ended { tag: t, used: self.site() }));
            }
        }
        Ok(())
    }

    /// Counts a statement (or terminator) at a position as reached.
    pub(crate) fn mon_cover(&mut self, loc: Option<sir::Loc>) {
        let Some(func) = self.frames.last().and_then(|f| f.func) else { return };
        let Some(l) = loc else { return };
        if let Some(m) = &mut self.monitor
            && let Some(c) = &mut m.coverage
        {
            *c.entry((func, l.line, l.col)).or_default() += 1;
        }
    }

    // ---------------------------------------------------------------- the report

    /// A run-time place: the local's source name and the concrete path,
    /// with field names where the types give them.
    fn pointer_label(&self, p: &Pointer) -> String {
        self.pointer_label_in(p, None)
    }

    /// As [`Self::pointer_label`], naming the local by `owner`'s
    /// declarations when its frame has returned.
    fn pointer_label_in(&self, p: &Pointer, owner: Option<usize>) -> String {
        let frame = self.frames_of(p.task).get(p.frame as usize).filter(|fr| fr.serial == p.serial);
        let f = frame.and_then(|fr| fr.func).or(owner);
        let decl = f.and_then(|i| self.func(i)).and_then(|f| f.locals.get(p.local as usize));
        let mut s = decl.and_then(|l| l.name.clone()).unwrap_or_else(|| format!("_{}", p.local));
        if p.frame == crate::value::RAW {
            s = "raw memory".to_string();
        }
        let mut ty = decl.map(|l| l.ty.clone());
        let mut variant: Option<u32> = None;
        let arg = |t: &Option<sir::Type>, k: usize| match t {
            Some(sir::Type::Named(_, args)) => args.get(k).cloned(),
            _ => None,
        };
        let mut path: &[Step] = &p.path;
        // A closure's capture by the name its code uses (`env(a, b)`).
        if p.local == 1
            && let Some(inner) = s.strip_prefix("env(").and_then(|x| x.strip_suffix(')'))
            && let Some(Step::Field(k)) = path.first()
            && let Some(name) = inner.split(", ").nth(*k as usize)
        {
            let name = name.to_string();
            ty = match ty {
                Some(sir::Type::Tuple(es)) => es.get(*k as usize).cloned(),
                _ => None,
            };
            s = name;
            path = &path[1..];
        }
        for st in path {
            match st {
                Step::Field(i) => {
                    let (name, next) = self.field_label(ty.as_ref(), variant, *i);
                    if let Some(name) = name {
                        s.push('.');
                        s.push_str(&name);
                    }
                    ty = next;
                    variant = None;
                }
                Step::Payload(v) => variant = Some(*v),
                Step::Index(i) => {
                    s.push_str(&format!("[{i}]"));
                    ty = arg(&ty, 0);
                }
                Step::DictKey(i) => {
                    s.push_str(&format!(".keys[{i}]"));
                    ty = arg(&ty, 0);
                }
                Step::DictVal(i) => {
                    s.push_str(&format!(".values[{i}]"));
                    ty = arg(&ty, 1);
                }
                Step::SetItem(i) => {
                    s.push_str(&format!(".items[{i}]"));
                    ty = arg(&ty, 0);
                }
                Step::DynValue => ty = None,
                Step::MutexValue => {
                    s.push_str(".value");
                    ty = arg(&ty, 0);
                }
                Step::Slot(i) => {
                    s.push_str(&format!("[slot {i}]"));
                    ty = arg(&ty, 0);
                }
                Step::Raw(o) => {
                    s.push_str(&format!("[byte {o}]"));
                    ty = None;
                }
            }
        }
        s
    }

    /// The name of field `i` of a value of type `ty` (of variant `variant`
    /// for an enum), and the field's type. The payload of an option or a
    /// result has no name: it is the value inside.
    fn field_label(
        &self,
        ty: Option<&sir::Type>,
        variant: Option<u32>,
        i: u32,
    ) -> (Option<String>, Option<sir::Type>) {
        let index = Some(i.to_string());
        match ty {
            Some(sir::Type::Tuple(ts)) => (index, ts.get(i as usize).cloned()),
            Some(sir::Type::Named(n, args)) => match self.prog.types.iter().find(|t| t.name() == n) {
                Some(sir::TypeDecl::Struct { fields, .. }) => match fields.get(i as usize) {
                    Some(fd) => (Some(fd.name.clone()), Some(fd.ty.clone())),
                    None => (index, None),
                },
                Some(sir::TypeDecl::Enum { variants, .. }) => {
                    match variant
                        .and_then(|v| variants.get(v as usize))
                        .and_then(|v| v.fields.get(i as usize))
                    {
                        Some((name, t)) => (name.clone().or(index), Some(t.clone())),
                        None => (index, None),
                    }
                }
                None if variant.is_some() && matches!(n.as_str(), "Option" | "Result") => {
                    let k = if n == "Result" && variant == Some(1) { 1 } else { 0 };
                    (None, args.get(k).cloned())
                }
                None => (index, None),
            },
            _ => (index, None),
        }
    }

    fn site_location(&self, s: &Site) -> String {
        let f = s.func.and_then(|i| self.func(i));
        let file =
            f.and_then(|f| self.prog.sources.get(f.source as usize)).map(String::as_str).unwrap_or("?");
        match s.loc {
            Some(l) => format!("{file}:{}:{}", l.line, l.col),
            None => file.to_string(),
        }
    }

    fn site_stmt(&self, s: &Site) -> String {
        let Some(f) = s.func.and_then(|i| self.func(i)) else { return String::new() };
        let Some(b) = f.blocks.get(s.bb) else { return String::new() };
        match s.stmt.and_then(|i| b.stmts.get(i)) {
            Some(st) => {
                let mut t = sir::write::stmt(st);
                if let Some(i) = t.find(" @") {
                    t.truncate(i);
                }
                t
            }
            None => sir::write::term(&b.term),
        }
    }

    fn fn_name(&self, s: &Site) -> String {
        s.func.and_then(|i| self.func(i)).map(|f| f.name.clone()).unwrap_or_default()
    }

    fn violation(&self, v: Violation) -> Unwind {
        let mut lines: Vec<String> = Vec::new();
        let used = match &v {
            Violation::Ended { used, .. }
            | Violation::Dangling { used, .. }
            | Violation::Moved { used, .. }
            | Violation::Unconsumed { used, .. } => used.clone(),
        };
        let what = match &v {
            Violation::Ended { .. } => render(msg::MONITOR_USE_ENDED_LOAN, &[]),
            Violation::Dangling { name, .. } => render(msg::MONITOR_USE_DANGLING, &[("name", name.clone())]),
            Violation::Moved { name, .. } => render(msg::MONITOR_USE_MOVED, &[("name", name.clone())]),
            Violation::Unconsumed { name, ty, .. } => {
                render(msg::MONITOR_UNCONSUMED, &[("name", name.clone()), ("ty", ty.clone())])
            }
        };
        lines.push(render(msg::MONITOR_HEADER, &[("what", what)]));
        if let Violation::Ended { tag, .. } = &v
            && let Some(l) = self.loan(*tag).cloned()
        {
            let kind = render(if l.mutable { msg::MONITOR_KIND_MUT } else { msg::MONITOR_KIND_SHARED }, &[]);
            lines.push(format!(
                "  {}",
                render(
                    msg::MONITOR_LOAN,
                    &[
                        ("kind", kind),
                        ("place", self.pointer_label_in(&l.at, l.owner)),
                        ("function", self.fn_name(&l.created)),
                        ("location", self.site_location(&l.created)),
                        ("stmt", self.site_stmt(&l.created)),
                    ],
                )
            ));
            if let Some(e) = &l.ended {
                lines.push(format!(
                    "  {}",
                    render(
                        msg::MONITOR_ENDED_BY,
                        &[
                            ("operation", render(e.op.msg(), &[])),
                            (
                                "place",
                                self.pointer_label_in(&e.at, l.owner.filter(|_| e.at.serial == l.at.serial))
                            ),
                            ("location", self.site_location(&e.site)),
                            ("stmt", self.site_stmt(&e.site)),
                        ],
                    )
                ));
            }
            lines.push(format!(
                "  {}",
                render(
                    msg::MONITOR_USED_AT,
                    &[("location", self.site_location(&used)), ("stmt", self.site_stmt(&used))],
                )
            ));
            let mut at = l.at.clone();
            at.tag = 0;
            if let Ok(val) = self.read(&at) {
                let shown =
                    if val.is_moved() { render(msg::MONITOR_NO_VALUE, &[]) } else { self.debug_string(val) };
                lines.push(format!(
                    "  {}",
                    render(
                        msg::MONITOR_VALUE,
                        &[("place", self.pointer_label_in(&l.at, l.owner)), ("value", shown)]
                    )
                ));
            }
            let (mut shared, mut exclusive) = (0, 0);
            if let Some(m) = &self.monitor
                && let Some(active) = m.active.get(&(l.at.serial, l.at.local))
            {
                for t in active.mutable.iter().chain(&active.shared) {
                    match m.loans.get(*t as usize - 1) {
                        Some(x) if x.mutable => exclusive += 1,
                        Some(_) => shared += 1,
                        None => {}
                    }
                }
            }
            lines.push(format!(
                "  {}",
                render(
                    msg::MONITOR_COUNTS,
                    &[
                        ("place", self.pointer_label_in(&l.at, l.owner)),
                        ("shared", shared.to_string()),
                        ("exclusive", exclusive.to_string()),
                    ],
                )
            ));
        } else {
            lines.push(format!(
                "  {}",
                render(
                    msg::MONITOR_USED_AT,
                    &[("location", self.site_location(&used)), ("stmt", self.site_stmt(&used))],
                )
            ));
        }
        // The stack, innermost first, without drop glue.
        for fr in self.frames.iter().rev() {
            let Some(f) = fr.func.and_then(|i| self.func(i)) else { continue };
            if f.kind == sir::FnKind::Glue {
                continue;
            }
            let s = Site { func: fr.func, bb: fr.bb, stmt: None, loc: fr.loc };
            lines.push(format!(
                "  {}",
                render(msg::RUN_FRAME, &[("function", f.name.clone()), ("location", self.site_location(&s))])
            ));
        }
        if let Some(f) = used.func.and_then(|i| self.func(i)) {
            lines.push(render(msg::MONITOR_SIR_HEADER, &[("function", f.name.clone())]));
            lines.push(sir::write::function(f).trim_end().to_string());
        }
        if let Some(m) = &self.monitor {
            m.violated.set(true);
        }
        Unwind::Abort(lines.join("\n"))
    }
}
