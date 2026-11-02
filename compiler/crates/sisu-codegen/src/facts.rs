//! What the release profile knows about a function's integers at each
//! point (decision 270): the bounds of the integers kept in variables, the
//! comparisons that hold between them, which local holds the length of a
//! list that cannot change meanwhile, and what a range iterator of the
//! frame yields. The code generator asks it whether an index is in bounds
//! (the check is left out), whether an addition, subtraction or
//! multiplication cannot overflow (the instruction is plain), and whether a
//! test is decided by the type of what it compares or by a test that
//! dominates it (the branch is a jump), which is how a contract clause an
//! earlier test implies costs nothing. Every answer is a proof: where it
//! knows nothing, the check stays.
//!
//! It is a forward analysis over the SIR function, the states joined where
//! paths meet (the hull of the bounds, the facts both paths have), with
//! the bounds at a block widened after a few rounds, so that it ends. Only
//! what cannot change behind its back is tracked: integers and `bool`s in
//! variables (no code borrows one, decision 246), and the lengths of lists
//! reached through a shared reference (which nothing may change while it
//! lives, 04) or held by a local whose mutable borrows go only straight
//! into a call that cannot keep them, each such call counting as a change.

use sisu_rt::abi::Intrinsic;
use sisu_sir as sir;
use sisu_sir::{BinOp, Callee, Const, Operand, Place, Proj, Rvalue, StatementKind, Terminator, Type, UnOp};

const MIN: i64 = i64::MIN;
const MAX: i64 = i64::MAX;

/// How often a block's entry may grow before its bounds are widened.
const WIDEN_AFTER: u32 = 3;

/// How often a block's entry may change at all before the analysis gives
/// up on it, knowing nothing there: widening ends every analysis long
/// before, and this keeps a mistake in that from looping.
const GIVE_UP_AFTER: u32 = 64;

/// A list whose length a fact speaks of: a local, then the referent of a
/// shared reference (first, and only there), fields, and elements at
/// indices held by tracked locals.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Key {
    root: u32,
    steps: Vec<Step>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Step {
    Deref,
    Field(u32),
    Index(u32),
}

impl Key {
    fn mentions(&self, l: u32) -> bool {
        self.root == l || self.steps.contains(&Step::Index(l))
    }

    /// Whether a change of `self` (a write or a mutable borrow of it) may
    /// change the length of the list `other` names: `self` is `other` or
    /// a part it lies in (an element at any index being any element), and
    /// not one of its elements.
    fn contains(&self, other: &Key) -> bool {
        self.root == other.root
            && self.steps.len() <= other.steps.len()
            && self
                .steps
                .iter()
                .zip(&other.steps)
                .all(|(a, b)| a == b || matches!((a, b), (Step::Index(_), Step::Index(_))))
    }
}

/// An integer operand: a tracked local or a constant.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Atom {
    Local(u32),
    Const(i64),
}

/// A comparison of two atoms: `a < b`, `a <= b`, `a == b` or `a != b`
/// (`>` and `>=` with the atoms swapped).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Rel {
    Lt,
    Le,
    Eq,
    Ne,
}

/// Where a range iterator stops: below a constant, below a local's
/// value, or below a list's length.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum End {
    Const(i64),
    Local(u32),
    Len(Key),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Fact {
    /// `a REL b`.
    Holds(Atom, Rel, Atom),
    /// The local holds the list's length.
    Len(u32, Key),
    /// The local is below the list's length.
    Below(u32, Key),
    /// The `bool` local holds whether `a REL b`.
    Cond(u32, Atom, Rel, Atom),
    /// The frame iterator yields integers from the number up and below its
    /// end.
    Range(u32, i64, End),
    /// The option local holds `Iter.next` of the frame iterator.
    Next(u32, u32),
    /// The local holds the discriminant of the option local.
    Disc(u32, u32),
    /// The option local is `Some`.
    IsSome(u32),
    /// The local holds a shared reference to the list or a part of one.
    RefTo(u32, Key),
    /// The local holds a mutable reference to it: each use may change it.
    MutTo(u32, Key),
}

impl Fact {
    fn mentions(&self, l: u32) -> bool {
        let atom = |a: &Atom| *a == Atom::Local(l);
        match self {
            Fact::Holds(a, _, b) => atom(a) || atom(b),
            Fact::Cond(c, a, _, b) => *c == l || atom(a) || atom(b),
            Fact::Len(x, k) | Fact::Below(x, k) | Fact::RefTo(x, k) | Fact::MutTo(x, k) => {
                *x == l || k.mentions(l)
            }
            Fact::Range(r, _, end) => {
                *r == l
                    || match end {
                        End::Const(_) => false,
                        End::Local(e) => *e == l,
                        End::Len(k) => k.mentions(l),
                    }
            }
            Fact::Next(o, r) | Fact::Disc(o, r) => *o == l || *r == l,
            Fact::IsSome(o) => *o == l,
        }
    }

    /// The list a change of which ends the fact: one about its length. A
    /// reference to the list still refers to it after it changes.
    fn key(&self) -> Option<&Key> {
        match self {
            Fact::Len(_, k) | Fact::Below(_, k) | Fact::Range(_, _, End::Len(k)) => Some(k),
            _ => None,
        }
    }

    /// The fact with `to` in the place of `from`, for a copy of an integer
    /// local.
    fn renamed(&self, from: u32, to: u32) -> Option<Fact> {
        let atom = |a: Atom| if a == Atom::Local(from) { Atom::Local(to) } else { a };
        match self {
            Fact::Holds(a, r, b) if *a == Atom::Local(from) || *b == Atom::Local(from) => {
                Some(Fact::Holds(atom(*a), *r, atom(*b)))
            }
            Fact::Len(x, k) if *x == from => Some(Fact::Len(to, k.clone())),
            Fact::Below(x, k) if *x == from => Some(Fact::Below(to, k.clone())),
            Fact::Cond(c, a, r, b) if *c == from => Some(Fact::Cond(to, *a, *r, *b)),
            _ => None,
        }
    }
}

/// What holds at a point: the bounds of the tracked locals that are
/// narrower than their types' (sorted by local), and the facts (sorted).
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct State {
    bounds: Vec<(u32, i64, i64)>,
    facts: Vec<Fact>,
}

impl State {
    fn add(&mut self, f: Fact) {
        if let Err(i) = self.facts.binary_search(&f) {
            self.facts.insert(i, f);
        }
    }

    fn has(&self, f: &Fact) -> bool {
        self.facts.binary_search(f).is_ok()
    }

    fn narrowed(&self, l: u32) -> Option<(i64, i64)> {
        self.bounds.binary_search_by_key(&l, |e| e.0).ok().map(|i| (self.bounds[i].1, self.bounds[i].2))
    }
}

/// The analysis of one function instance.
pub(crate) struct Facts {
    /// Each block's state at its entry; `None` where no path the analysis
    /// follows reaches it.
    entry: Vec<Option<State>>,
    /// Each local's bounds by its type, for the tracked locals.
    tracked: Vec<Option<(i64, i64)>>,
    /// The locals some mutable borrow of which goes where the analysis
    /// cannot follow: no fact speaks of a list in one.
    wild: Vec<bool>,
    /// Each local's type.
    tys: Vec<Type>,
    /// The frame iterators (decision 253).
    frames: Vec<bool>,
    /// The functions the program calls that only return an integer.
    constants: Vec<(String, i64)>,
}

/// The bounds a tracked local's type gives it.
fn type_bounds(t: &Type) -> Option<(i64, i64)> {
    match t {
        Type::Int => Some((MIN, MAX)),
        Type::Bool => Some((0, 1)),
        Type::Named(n, a) if a.is_empty() => match n.as_str() {
            "u8" => Some((0, 255)),
            "i8" => Some((-128, 127)),
            "u16" => Some((0, 65_535)),
            "i16" => Some((-32_768, 32_767)),
            "u32" => Some((0, 4_294_967_295)),
            "i32" => Some((-2_147_483_648, 2_147_483_647)),
            _ => None,
        },
        _ => None,
    }
}

/// Whether a value of the type holds no reference: the result of a call
/// that was given a mutable borrow, for the borrow not to outlive the call.
fn holds_no_reference(t: &Type) -> bool {
    match t {
        Type::Int | Type::Float | Type::Bool | Type::Char | Type::Unit | Type::Never => true,
        Type::Named(n, a) if n == "Option" => a.iter().all(holds_no_reference),
        Type::Named(_, a) if a.is_empty() => type_bounds(t).is_some(),
        _ => false,
    }
}

impl Facts {
    /// The analysis of `f`, whose locals have the concrete types `tys`;
    /// `regs` are the locals in variables, `frames` the frame iterators,
    /// and `constant` the value of a function that only returns an integer.
    pub(crate) fn of(
        f: &sir::Function,
        tys: &[Type],
        regs: &[bool],
        frames: &[bool],
        constant: &dyn Fn(&str) -> Option<i64>,
    ) -> Facts {
        // The result too, which lives in the caller's memory, when the
        // function only assigns it whole (its `ensures` read it).
        let result = whole_only(f, 0);
        let tracked: Vec<Option<(i64, i64)>> = tys
            .iter()
            .zip(regs)
            .enumerate()
            .map(|(i, (t, r))| if *r || (i == 0 && result) { type_bounds(t) } else { None })
            .collect();
        let mut constants: Vec<(String, i64)> = Vec::new();
        for b in &f.blocks {
            for s in &b.stmts {
                if let StatementKind::Call { callee: Callee::Fn(name, targs), args, .. } = &s.kind
                    && targs.is_empty()
                    && args.is_empty()
                    && !constants.iter().any(|(n, _)| n == name)
                    && let Some(c) = constant(name)
                {
                    constants.push((name.clone(), c));
                }
            }
        }
        let mut facts = Facts {
            entry: vec![None; f.blocks.len()],
            tracked,
            wild: wild_locals(f, tys),
            tys: tys.to_vec(),
            frames: frames.to_vec(),
            constants,
        };
        facts.solve(f);
        facts
    }

    /// The state at the entry of block `i`.
    pub(crate) fn entry(&self, i: usize) -> Option<State> {
        self.entry.get(i).cloned().flatten()
    }

    fn solve(&mut self, f: &sir::Function) {
        let n = f.blocks.len();
        let mut visits = vec![0u32; n];
        let mut queued = vec![false; n];
        let mut work: Vec<usize> = Vec::new();
        // The entry, and each cleanup block, which unwinding enters from
        // any statement of its blocks: knowing nothing.
        for (i, b) in f.blocks.iter().enumerate() {
            if i == 0 || b.cleanup {
                self.entry[i] = Some(State::default());
                queued[i] = true;
                work.push(i);
            }
        }
        while let Some(i) = work.pop() {
            queued[i] = false;
            let Some(mut st) = self.entry[i].clone() else { continue };
            let block = &f.blocks[i];
            for s in &block.stmts {
                self.step(&mut st, &s.kind);
            }
            for (t, edge) in successors(&block.term) {
                if f.blocks.get(t).is_none_or(|b| b.cleanup) {
                    continue;
                }
                let mut out = st.clone();
                if !self.edge(&mut out, &block.term, edge) {
                    continue;
                }
                let next = match &self.entry[t] {
                    None => out,
                    Some(old) => {
                        let mut joined = self.join(old, &out);
                        visits[t] += 1;
                        if visits[t] > WIDEN_AFTER {
                            self.widen(old, &mut joined);
                            // What the facts that stay say of the bounds,
                            // which widening may have lost (a loop inside
                            // another widens the outer loop's counter).
                            self.narrow(&mut joined);
                        }
                        if joined == *old {
                            continue;
                        }
                        if visits[t] > GIVE_UP_AFTER { State::default() } else { joined }
                    }
                };
                self.entry[t] = Some(next);
                if !queued[t] {
                    queued[t] = true;
                    work.push(t);
                }
            }
        }
    }

    fn type_of(&self, l: u32) -> Option<(i64, i64)> {
        self.tracked.get(l as usize).copied().flatten()
    }

    fn bound(&self, st: &State, a: Atom) -> (i64, i64) {
        match a {
            Atom::Const(c) => (c, c),
            Atom::Local(l) => st.narrowed(l).or_else(|| self.type_of(l)).unwrap_or((MIN, MAX)),
        }
    }

    fn set_bound(&self, st: &mut State, l: u32, (lo, hi): (i64, i64)) {
        let Some((tlo, thi)) = self.type_of(l) else { return };
        let (lo, hi) = (lo.max(tlo), hi.min(thi));
        let at = st.bounds.binary_search_by_key(&l, |e| e.0);
        // Bounds outside the type say nothing the type does not.
        if (lo, hi) == (tlo, thi) || lo > hi {
            if let Ok(i) = at {
                st.bounds.remove(i);
            }
            return;
        }
        match at {
            Ok(i) => st.bounds[i] = (l, lo, hi),
            Err(i) => st.bounds.insert(i, (l, lo, hi)),
        }
    }

    /// Where two paths meet: the hull of the bounds, the facts both have.
    fn join(&self, old: &State, new: &State) -> State {
        let mut st = State::default();
        for &(l, lo, hi) in &old.bounds {
            if let Some((nlo, nhi)) = new.narrowed(l) {
                self.set_bound(&mut st, l, (lo.min(nlo), hi.max(nhi)));
            }
        }
        st.facts = old.facts.iter().filter(|f| new.has(f)).cloned().collect();
        st
    }

    /// The bounds that keep growing at a block go to their type's ends.
    fn widen(&self, old: &State, joined: &mut State) {
        let grown: Vec<(u32, i64, i64)> = joined.bounds.clone();
        for (l, lo, hi) in grown {
            let (olo, ohi) = self.bound(old, Atom::Local(l));
            let (tlo, thi) = self.type_of(l).unwrap_or((MIN, MAX));
            let lo = if lo < olo { tlo } else { lo };
            let hi = if hi > ohi { thi } else { hi };
            self.set_bound(joined, l, (lo, hi));
        }
    }

    /// Narrows the bounds by the comparisons the state holds.
    fn narrow(&self, st: &mut State) {
        for _ in 0..2 {
            let held: Vec<(Atom, Rel, Atom)> = st
                .facts
                .iter()
                .filter_map(|f| match f {
                    Fact::Holds(a, r, b) => Some((*a, *r, *b)),
                    _ => None,
                })
                .collect();
            for (a, r, b) in held {
                let ((alo, ahi), (blo, bhi)) = (self.bound(st, a), self.bound(st, b));
                let (na, nb) = match r {
                    Rel::Lt => ((alo, ahi.min(bhi.saturating_sub(1))), (blo.max(alo.saturating_add(1)), bhi)),
                    Rel::Le => ((alo, ahi.min(bhi)), (blo.max(alo), bhi)),
                    Rel::Eq => ((alo.max(blo), ahi.min(bhi)), (alo.max(blo), ahi.min(bhi))),
                    Rel::Ne => continue,
                };
                if na.0 > na.1 || nb.0 > nb.1 {
                    continue;
                }
                if let Atom::Local(l) = a {
                    self.set_bound(st, l, na);
                }
                if let Atom::Local(l) = b {
                    self.set_bound(st, l, nb);
                }
            }
        }
    }

    fn is_tracked(&self, l: u32) -> bool {
        self.type_of(l).is_some()
    }

    fn atom(&self, o: &Operand) -> Option<Atom> {
        match o {
            Operand::Const(Const::Int(c)) => Some(Atom::Const(*c)),
            Operand::Copy(p) | Operand::Move(p) if p.proj.is_empty() && self.is_tracked(p.local.0) => {
                Some(Atom::Local(p.local.0))
            }
            _ => None,
        }
    }

    /// The list a place names, as a key: through a shared reference, or in
    /// a local the analysis follows, then fields and elements.
    fn key_of(&self, st: &State, p: &Place) -> Option<Key> {
        let root = p.local.0;
        let mut steps = Vec::new();
        let mut projs = p.proj.as_slice();
        match self.tys.get(root as usize)? {
            Type::Ref(_, false) => {
                if projs.first() != Some(&Proj::Deref) {
                    return None;
                }
                steps.push(Step::Deref);
                projs = &projs[1..];
            }
            Type::Ref(_, true) => return None,
            _ if self.wild.get(root as usize).copied().unwrap_or(true) => return None,
            _ => {}
        }
        for q in projs {
            match q {
                Proj::Field(i) => steps.push(Step::Field(*i)),
                Proj::Index(l) if self.is_tracked(l.0) => steps.push(Step::Index(canonical(st, l.0))),
                _ => return None,
            }
        }
        Some(Key { root, steps })
    }

    /// The list `len` of the operand measures: what a shared reference in
    /// a local refers to, the referent of a shared reference, or the list.
    fn list_of(&self, st: &State, o: &Operand) -> Option<Key> {
        let (Operand::Copy(p) | Operand::Move(p)) = o else { return None };
        if p.proj.is_empty() {
            let l = p.local.0;
            let known = st.facts.iter().find_map(|f| match f {
                Fact::RefTo(x, k) if *x == l => Some(k.clone()),
                _ => None,
            });
            if known.is_some() {
                return known;
            }
            if matches!(self.tys.get(l as usize), Some(Type::Ref(_, false))) {
                let deref = Place { local: p.local, proj: vec![Proj::Deref] };
                return self.key_of(st, &deref);
            }
        }
        self.key_of(st, p)
    }

    /// Forgets what a new value of `l` makes untrue.
    fn kill(&self, st: &mut State, l: u32) {
        st.facts.retain(|f| !f.mentions(l));
        if let Ok(i) = st.bounds.binary_search_by_key(&l, |e| e.0) {
            st.bounds.remove(i);
        }
    }

    /// The part of a list (or of the local holding one) a write to the
    /// place, or a mutable borrow of it, may change.
    fn written(&self, st: &State, p: &Place) -> Key {
        let l = p.local.0;
        // Through a mutable reference the analysis follows: what it refers
        // to, then the rest of the place.
        let via = st.facts.iter().find_map(|f| match f {
            Fact::MutTo(x, k) if *x == l && p.proj.first() == Some(&Proj::Deref) => Some(k.clone()),
            _ => None,
        });
        let (mut key, rest) = match via {
            Some(k) => (k, &p.proj[1..]),
            None => {
                let mut k = Key { root: l, steps: Vec::new() };
                let mut rest = p.proj.as_slice();
                if matches!(self.tys.get(l as usize), Some(Type::Ref(..)))
                    && rest.first() == Some(&Proj::Deref)
                {
                    k.steps.push(Step::Deref);
                    rest = &rest[1..];
                }
                (k, rest)
            }
        };
        for q in rest {
            match q {
                Proj::Field(i) => key.steps.push(Step::Field(*i)),
                Proj::Index(i) => key.steps.push(Step::Index(i.0)),
                // Past a variant or a key: all of what comes before it.
                _ => break,
            }
        }
        key
    }

    /// Forgets the lengths a write into, or a mutable borrow of, the place
    /// may change, and what is known about the option it is.
    fn kill_writes(&self, st: &mut State, p: &Place) {
        let w = self.written(st, p);
        let l = p.local.0;
        st.facts.retain(|f| {
            !f.key().is_some_and(|k| w.contains(k))
                && !matches!(f, Fact::Next(o, _) | Fact::IsSome(o) | Fact::Disc(_, o) if *o == l)
        });
    }

    /// A use of the local: when it holds a mutable reference, what it
    /// refers to may change.
    fn use_local(&self, st: &mut State, l: u32) {
        let targets: Vec<Key> = st
            .facts
            .iter()
            .filter_map(|f| match f {
                Fact::MutTo(x, k) if *x == l => Some(k.clone()),
                _ => None,
            })
            .collect();
        for k in targets {
            st.facts.retain(|f| !f.key().is_some_and(|g| k.contains(g)));
        }
    }

    fn use_operand(&self, st: &mut State, o: &Operand) {
        if let Operand::Copy(p) | Operand::Move(p) = o {
            self.use_local(st, p.local.0);
            for q in &p.proj {
                if let Proj::Index(i) | Proj::IndexFromEnd(i) | Proj::Key(i) = q {
                    self.use_local(st, i.0);
                }
            }
        }
    }

    /// Sets what an assignment to the local `x` makes known.
    fn assign(&self, st: &mut State, x: u32, bound: Option<(i64, i64)>, new: Vec<Fact>) {
        let whole = Place { local: sir::LocalId(x), proj: Vec::new() };
        self.kill(st, x);
        self.kill_writes(st, &whole);
        if let Some(b) = bound {
            self.set_bound(st, x, b);
        }
        for f in new {
            st.add(f);
        }
    }

    /// The state after a statement.
    pub(crate) fn step(&self, st: &mut State, s: &StatementKind) {
        match s {
            StatementKind::Assign(dest, rv) => {
                match rv {
                    Rvalue::Use(o) | Rvalue::Unary(_, o) => self.use_operand(st, o),
                    Rvalue::Binary(_, a, b) => {
                        self.use_operand(st, a);
                        self.use_operand(st, b);
                    }
                    Rvalue::Aggregate(_, os) | Rvalue::Closure(_, _, os) => {
                        os.iter().for_each(|o| self.use_operand(st, o))
                    }
                    Rvalue::Ref(q, true) => self.kill_writes(st, q),
                    _ => {}
                }
                if dest.proj.is_empty() {
                    let (bound, new) = self.value_of(st, dest.local.0, rv);
                    self.assign(st, dest.local.0, bound, new);
                } else {
                    self.kill_writes(st, dest);
                }
            }
            StatementKind::Call { dest, callee, args } => {
                args.iter().for_each(|o| self.use_operand(st, o));
                if let Callee::Value(o) = callee {
                    self.use_operand(st, o);
                }
                // A list given away is gone.
                for o in args {
                    if let Operand::Move(p) = o {
                        self.kill_writes(st, p);
                    }
                }
                if dest.proj.is_empty() {
                    let (bound, new) = self.call_value(st, dest.local.0, callee, args);
                    self.assign(st, dest.local.0, bound, new);
                } else {
                    self.kill_writes(st, dest);
                }
            }
            StatementKind::Drop { place, .. } => self.kill_writes(st, place),
            StatementKind::StorageDead(l) => {
                self.assign(st, l.0, None, Vec::new());
            }
        }
    }

    /// What an assignment of the rvalue to `x` makes known, computed from
    /// the state before it: `x`'s bounds, and facts.
    fn value_of(&self, st: &State, x: u32, rv: &Rvalue) -> (Option<(i64, i64)>, Vec<Fact>) {
        let mut facts = Vec::new();
        match rv {
            Rvalue::Use(o) => {
                if let Some(a) = self.atom(o) {
                    if let Atom::Local(y) = a
                        && y != x
                    {
                        facts.extend(
                            st.facts.iter().filter(|f| !f.mentions(x)).filter_map(|f| f.renamed(y, x)),
                        );
                        // Equal locals, the smaller first: an index held by
                        // either names the same element.
                        facts.push(Fact::Holds(Atom::Local(x.min(y)), Rel::Eq, Atom::Local(x.max(y))));
                    }
                    return (Some(self.bound(st, a)), facts);
                }
                // The payload of `Some` from a range iterator.
                if let Operand::Copy(p) | Operand::Move(p) = o
                    && p.proj == [Proj::Downcast(0), Proj::Field(0)]
                    && self.is_tracked(x)
                {
                    let o = p.local.0;
                    let range = st
                        .facts
                        .iter()
                        .find_map(|f| match f {
                            Fact::Next(q, r) if *q == o => Some(*r),
                            _ => None,
                        })
                        .filter(|_| st.has(&Fact::IsSome(o)))
                        .and_then(|r| {
                            st.facts.iter().find_map(|f| match f {
                                Fact::Range(q, lo, end) if *q == r => Some((*lo, end.clone())),
                                _ => None,
                            })
                        });
                    if let Some((lo, end)) = range {
                        let hi = match end {
                            End::Const(c) => c.saturating_sub(1),
                            End::Local(e) => {
                                if e != x {
                                    facts.push(Fact::Holds(Atom::Local(x), Rel::Lt, Atom::Local(e)));
                                }
                                self.bound(st, Atom::Local(e)).1.saturating_sub(1)
                            }
                            End::Len(k) => {
                                if !k.mentions(x) {
                                    facts.push(Fact::Below(x, k));
                                }
                                MAX - 1
                            }
                        };
                        return (Some((lo, hi)), facts);
                    }
                }
                (None, facts)
            }
            Rvalue::Binary(op, a, b) => {
                let (Some(a), Some(b)) = (self.atom(a), self.atom(b)) else { return (None, facts) };
                let cmp = match op {
                    BinOp::Lt => Some((a, Rel::Lt, b)),
                    BinOp::Le => Some((a, Rel::Le, b)),
                    BinOp::Gt => Some((b, Rel::Lt, a)),
                    BinOp::Ge => Some((b, Rel::Le, a)),
                    BinOp::Eq => Some((a, Rel::Eq, b)),
                    BinOp::Ne => Some((a, Rel::Ne, b)),
                    _ => None,
                };
                if let Some((p, r, q)) = cmp {
                    if p != Atom::Local(x) && q != Atom::Local(x) {
                        facts.push(Fact::Cond(x, p, r, q));
                    }
                    let known = self.decide(st, p, r, q).map(|v| (i64::from(v), i64::from(v)));
                    return (known, facts);
                }
                let (ba, bb) = (self.bound(st, a), self.bound(st, b));
                let bound = match op {
                    BinOp::Add | BinOp::Sub | BinOp::Mul => arith(*op, ba, bb),
                    BinOp::FloorDiv | BinOp::Div if bb.0 >= 1 => Some((
                        quotient(ba.0, if ba.0 >= 0 { bb.1 } else { bb.0 }),
                        quotient(ba.1, if ba.1 >= 0 { bb.0 } else { bb.1 }),
                    )),
                    BinOp::Mod if bb.0 >= 1 && bb.0 == bb.1 => Some((0, bb.0 - 1)),
                    BinOp::BitAnd if bb.0 >= 0 && bb.0 == bb.1 => Some((0, bb.0)),
                    _ => None,
                };
                // `x = y + 1` with `y < e`: `x <= e`.
                if *op == BinOp::Add
                    && let (Atom::Local(y), Atom::Const(1)) = (a, b)
                    && y != x
                {
                    for f in &st.facts {
                        if let Fact::Holds(Atom::Local(q), Rel::Lt, e) = f
                            && *q == y
                            && *e != Atom::Local(x)
                        {
                            facts.push(Fact::Holds(Atom::Local(x), Rel::Le, *e));
                        }
                    }
                }
                (bound, facts)
            }
            Rvalue::Unary(UnOp::Neg, o) => match self.atom(o) {
                Some(a) => {
                    let (lo, hi) = self.bound(st, a);
                    if lo == MIN { (None, facts) } else { (Some((-hi, -lo)), facts) }
                }
                None => (None, facts),
            },
            Rvalue::Discriminant(q) if q.proj.is_empty() && q.local.0 != x => {
                facts.push(Fact::Disc(x, q.local.0));
                (None, facts)
            }
            Rvalue::Ref(q, mutable) => {
                if let Some(k) = self.key_of(st, q).filter(|k| !k.mentions(x)) {
                    facts.push(if *mutable { Fact::MutTo(x, k) } else { Fact::RefTo(x, k) });
                }
                (None, facts)
            }
            _ => (None, facts),
        }
    }

    /// What a call's result in `x` makes known.
    fn call_value(
        &self,
        st: &State,
        x: u32,
        callee: &Callee,
        args: &[Operand],
    ) -> (Option<(i64, i64)>, Vec<Fact>) {
        let mut facts = Vec::new();
        match callee {
            Callee::Intrinsic(name, _) => match Intrinsic::from_name(name) {
                Some(Intrinsic::Len) => {
                    if let Some(k) = args.first().and_then(|o| self.list_of(st, o)).filter(|k| !k.mentions(x))
                    {
                        facts.push(Fact::Len(x, k));
                    }
                    (Some((0, MAX)), facts)
                }
                Some(Intrinsic::IterNext) => {
                    if let [Operand::Copy(p) | Operand::Move(p)] = args
                        && p.proj.is_empty()
                        && let Some(r) = st.facts.iter().find_map(|f| match f {
                            Fact::MutTo(t, k) if *t == p.local.0 && k.steps.is_empty() => Some(k.root),
                            _ => None,
                        })
                        && r != x
                        && self.frames.get(r as usize) == Some(&true)
                    {
                        facts.push(Fact::Next(x, r));
                    }
                    (None, facts)
                }
                Some(Intrinsic::Range) if self.frames.get(x as usize) == Some(&true) => {
                    if let [start, end, step] = args
                        && let (Some(s), Some(t)) = (self.atom(start), self.atom(step))
                        && self.bound(st, t).0 >= 1
                    {
                        let lo = self.bound(st, s).0;
                        let end = match end {
                            Operand::Const(Const::Int(c)) => Some(End::Const(*c)),
                            Operand::Copy(p) | Operand::Move(p) if p.proj.is_empty() && p.local.0 != x => {
                                let e = p.local.0;
                                st.facts
                                    .iter()
                                    .find_map(|f| match f {
                                        Fact::Len(q, k) if *q == e && !k.mentions(x) => {
                                            Some(End::Len(k.clone()))
                                        }
                                        _ => None,
                                    })
                                    .or_else(|| self.is_tracked(e).then_some(End::Local(e)))
                            }
                            _ => None,
                        };
                        if let Some(end) = end {
                            facts.push(Fact::Range(x, lo, end));
                        }
                    }
                    (None, facts)
                }
                _ => (None, facts),
            },
            Callee::Fn(name, targs) if targs.is_empty() && args.is_empty() => {
                let c = self.constants.iter().find(|(n, _)| n == name).map(|(_, c)| *c);
                (c.map(|c| (c, c)), facts)
            }
            _ => (None, facts),
        }
    }

    /// What an edge of the terminator adds; `false` when it cannot be
    /// taken.
    fn edge(&self, st: &mut State, term: &Terminator, edge: Edge) -> bool {
        let Terminator::Switch { value, cases, .. } = term else { return true };
        let (Operand::Copy(p) | Operand::Move(p)) = value else { return true };
        if !p.proj.is_empty() {
            return true;
        }
        let c = p.local.0;
        // `switch c [0: false, else: true]` on a comparison.
        let cond = st.facts.iter().find_map(|f| match f {
            Fact::Cond(x, a, r, b) if *x == c => Some((*a, *r, *b)),
            _ => None,
        });
        if let Some((a, r, b)) = cond
            && let [(0, _)] = cases.as_slice()
        {
            return match edge {
                Edge::Default => self.assume(st, a, r, b),
                Edge::Case(0) => {
                    let (a, r, b) = negate(a, r, b);
                    self.assume(st, a, r, b)
                }
                _ => true,
            };
        }
        // `switch discriminant o [0: some, ...]`.
        if edge == Edge::Case(0)
            && let Some(o) = st.facts.iter().find_map(|f| match f {
                Fact::Disc(d, o) if *d == c => Some(*o),
                _ => None,
            })
        {
            st.add(Fact::IsSome(o));
        }
        true
    }

    /// Adds `a REL b` and narrows the bounds by it; `false` when the bounds
    /// say it cannot hold.
    fn assume(&self, st: &mut State, a: Atom, r: Rel, b: Atom) -> bool {
        if self.decide(st, a, r, b) == Some(false) {
            return false;
        }
        let ((alo, ahi), (blo, bhi)) = (self.bound(st, a), self.bound(st, b));
        let (na, nb) = match r {
            Rel::Lt => ((alo, ahi.min(bhi.saturating_sub(1))), (blo.max(alo.saturating_add(1)), bhi)),
            Rel::Le => ((alo, ahi.min(bhi)), (blo.max(alo), bhi)),
            Rel::Eq => {
                let both = (alo.max(blo), ahi.min(bhi));
                (both, both)
            }
            Rel::Ne => {
                let narrow = |(lo, hi): (i64, i64), (clo, chi): (i64, i64)| {
                    if clo != chi {
                        (lo, hi)
                    } else if lo == clo {
                        (lo.saturating_add(1), hi)
                    } else if hi == clo {
                        (lo, hi.saturating_sub(1))
                    } else {
                        (lo, hi)
                    }
                };
                (narrow((alo, ahi), (blo, bhi)), narrow((blo, bhi), (alo, ahi)))
            }
        };
        if na.0 > na.1 || nb.0 > nb.1 {
            return false;
        }
        if let Atom::Local(l) = a {
            self.set_bound(st, l, na);
        }
        if let Atom::Local(l) = b {
            self.set_bound(st, l, nb);
        }
        if matches!(a, Atom::Local(_)) || matches!(b, Atom::Local(_)) {
            st.add(Fact::Holds(a, r, b));
        }
        // `i < e` with `e` a list's length: `i` is below it.
        if let (Rel::Lt, Atom::Local(i), Atom::Local(e)) = (r, a, b) {
            let below: Vec<Fact> = st
                .facts
                .iter()
                .filter_map(|f| match f {
                    Fact::Len(x, k) if *x == e && !k.mentions(i) => Some(Fact::Below(i, k.clone())),
                    _ => None,
                })
                .collect();
            for f in below {
                st.add(f);
            }
        }
        true
    }

    /// Whether the state decides `a REL b`.
    fn decide(&self, st: &State, a: Atom, r: Rel, b: Atom) -> Option<bool> {
        let ((alo, ahi), (blo, bhi)) = (self.bound(st, a), self.bound(st, b));
        let by_bounds = match r {
            Rel::Lt if ahi < blo => Some(true),
            Rel::Lt if alo >= bhi => Some(false),
            Rel::Le if ahi <= blo => Some(true),
            Rel::Le if alo > bhi => Some(false),
            Rel::Eq if alo == ahi && blo == bhi && alo == blo => Some(true),
            Rel::Eq if ahi < blo || bhi < alo => Some(false),
            Rel::Ne if ahi < blo || bhi < alo => Some(true),
            Rel::Ne if alo == ahi && blo == bhi && alo == blo => Some(false),
            _ => None,
        };
        if by_bounds.is_some() {
            return by_bounds;
        }
        // A comparison of the same atoms that holds.
        let known = |x: Atom, q: Rel, y: Atom| st.has(&Fact::Holds(x, q, y));
        let eq = known(a, Rel::Eq, b) || known(b, Rel::Eq, a);
        let ne = known(a, Rel::Ne, b) || known(b, Rel::Ne, a);
        let (lt, gt) = (known(a, Rel::Lt, b), known(b, Rel::Lt, a));
        let (le, ge) = (known(a, Rel::Le, b), known(b, Rel::Le, a));
        match r {
            Rel::Lt if lt => Some(true),
            Rel::Lt if gt || ge || eq => Some(false),
            Rel::Le if le || lt || eq => Some(true),
            Rel::Le if gt => Some(false),
            Rel::Eq if eq => Some(true),
            Rel::Eq if lt || gt || ne => Some(false),
            Rel::Ne if ne || lt || gt => Some(true),
            Rel::Ne if eq => Some(false),
            _ => None,
        }
    }

    /// Whether the state proves `0 <= i < len(list)`, `list` being the
    /// place before an `Index` projection.
    pub(crate) fn in_bounds(&self, st: &State, list: &Place, i: u32) -> bool {
        if !self.is_tracked(i) || self.bound(st, Atom::Local(i)).0 < 0 {
            return false;
        }
        let Some(k) = self.key_of(st, list) else { return false };
        if st.has(&Fact::Below(i, k.clone())) {
            return true;
        }
        // Below another list's length, which a test found equal to this
        // one's (`requires len(xs) == len(ys)`).
        let lens_of = |key: &Key| -> Vec<u32> {
            st.facts
                .iter()
                .filter_map(|f| match f {
                    Fact::Len(e, g) if g == key => Some(*e),
                    _ => None,
                })
                .collect()
        };
        let mine = lens_of(&k);
        let equal = |a: u32, b: u32| {
            st.has(&Fact::Holds(Atom::Local(a), Rel::Eq, Atom::Local(b)))
                || st.has(&Fact::Holds(Atom::Local(b), Rel::Eq, Atom::Local(a)))
        };
        let below_equal = st.facts.iter().any(|f| match f {
            Fact::Below(x, other) if *x == i && *other != k => {
                lens_of(other).iter().any(|a| mine.iter().any(|b| equal(*a, *b)))
            }
            _ => false,
        });
        if below_equal {
            return true;
        }
        let len = |e: &Atom| matches!(e, Atom::Local(e) if st.has(&Fact::Len(*e, k.clone())));
        st.facts.iter().any(|f| match f {
            Fact::Holds(Atom::Local(a), Rel::Lt, e) if *a == i => len(e),
            Fact::Holds(Atom::Local(a), Rel::Le, m) if *a == i => {
                st.facts.iter().any(|g| matches!(g, Fact::Holds(m2, Rel::Lt, e) if m2 == m && len(e)))
            }
            _ => false,
        })
    }

    /// The value of a `switch` on a `bool` local when the state decides it.
    pub(crate) fn decided(&self, st: &State, value: &Operand) -> Option<bool> {
        let (Operand::Copy(p) | Operand::Move(p)) = value else { return None };
        if !p.proj.is_empty() {
            return None;
        }
        let c = p.local.0;
        st.facts.iter().find_map(|f| match f {
            Fact::Cond(x, a, r, b) if *x == c => self.decide(st, *a, *r, *b),
            _ => None,
        })
    }

    /// Whether `a OP b` on two `int`s cannot overflow.
    pub(crate) fn cannot_overflow(&self, st: &State, op: BinOp, a: &Operand, b: &Operand) -> bool {
        let (Some(a), Some(b)) = (self.atom(a), self.atom(b)) else { return false };
        let int = |x: Atom| match x {
            Atom::Const(_) => true,
            Atom::Local(l) => matches!(self.tys.get(l as usize), Some(Type::Int)),
        };
        int(a) && int(b) && fits(op, self.bound(st, a), self.bound(st, b))
    }
}

/// Which way a terminator leaves: a `switch`'s case or default, or a jump
/// (also a target that several of a `switch`'s values reach).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Edge {
    Case(i64),
    Default,
    Jump,
}

/// Each successor of a terminator, once, and the edge to it.
fn successors(t: &Terminator) -> Vec<(usize, Edge)> {
    match t {
        Terminator::Goto(b) => vec![(b.0 as usize, Edge::Jump)],
        Terminator::Switch { cases, default, .. } => {
            let d = default.0 as usize;
            let mut out: Vec<(usize, Edge)> = Vec::new();
            for (k, b) in cases {
                let t = b.0 as usize;
                let many = t == d || cases.iter().filter(|(_, c)| c.0 as usize == t).count() > 1;
                if !out.iter().any(|(u, _)| *u == t) {
                    out.push((t, if many { Edge::Jump } else { Edge::Case(*k) }));
                }
            }
            if !out.iter().any(|(u, _)| *u == d) {
                out.push((d, Edge::Default));
            }
            out
        }
        _ => Vec::new(),
    }
}

/// Whether the function only reads and writes the local whole: no
/// statement borrows it, projects through it or names a key with it.
fn whole_only(f: &sir::Function, l: u32) -> bool {
    let place = |p: &Place| p.local.0 != l || p.proj.is_empty();
    let operand = |o: &Operand| match o {
        Operand::Copy(p) | Operand::Move(p) => place(p),
        Operand::Const(_) => true,
    };
    let keyless = |p: &Place| {
        p.proj.iter().all(|q| !matches!(q, Proj::Index(k) | Proj::IndexFromEnd(k) | Proj::Key(k) if k.0 == l))
    };
    let operand_keyless = |o: &Operand| match o {
        Operand::Copy(p) | Operand::Move(p) => keyless(p),
        Operand::Const(_) => true,
    };
    f.blocks.iter().all(|b| {
        b.stmts.iter().all(|s| match &s.kind {
            StatementKind::Assign(d, rv) => {
                place(d)
                    && keyless(d)
                    && match rv {
                        Rvalue::Use(o) | Rvalue::Unary(_, o) => operand(o) && operand_keyless(o),
                        Rvalue::Binary(_, a, c) => {
                            operand(a) && operand(c) && operand_keyless(a) && operand_keyless(c)
                        }
                        Rvalue::Aggregate(_, os) | Rvalue::Closure(_, _, os) => {
                            os.iter().all(|o| operand(o) && operand_keyless(o))
                        }
                        Rvalue::Ref(q, _)
                        | Rvalue::Discriminant(q)
                        | Rvalue::CopyOf(q)
                        | Rvalue::StrView(q) => q.local.0 != l && keyless(q),
                    }
            }
            StatementKind::Call { dest, callee, args } => {
                place(dest)
                    && keyless(dest)
                    && args.iter().all(|o| operand(o) && operand_keyless(o))
                    && match callee {
                        Callee::Value(o) => operand(o),
                        _ => true,
                    }
            }
            StatementKind::Drop { place: p, .. } => place(p) && keyless(p),
            StatementKind::StorageDead(_) => true,
        }) && match &b.term {
            Terminator::Switch { value, .. } => operand(value),
            _ => true,
        }
    })
}

/// The locals some mutable borrow of which goes where the analysis cannot
/// follow it: anywhere but whole into the arguments of a call whose result
/// cannot hold a reference. The parameters are left out too.
fn wild_locals(f: &sir::Function, tys: &[Type]) -> Vec<bool> {
    let n = tys.len();
    let mut wild = vec![false; n];
    for w in wild.iter_mut().take(f.param_count as usize + 1) {
        *w = true;
    }
    // Each mutable borrow: the local holding it, and the local borrowed.
    let mut borrows: Vec<(usize, usize)> = Vec::new();
    for b in &f.blocks {
        for s in &b.stmts {
            if let StatementKind::Assign(d, Rvalue::Ref(q, true)) = &s.kind {
                if d.proj.is_empty() {
                    borrows.push((d.local.0 as usize, q.local.0 as usize));
                } else if let Some(w) = wild.get_mut(q.local.0 as usize) {
                    *w = true;
                }
            }
        }
    }
    // The locals used otherwise than whole as such a call's argument.
    let mut loose = vec![false; n];
    let mut mark = |p: &Place, loose: &mut Vec<bool>| {
        if let Some(x) = loose.get_mut(p.local.0 as usize) {
            *x = true;
        }
    };
    let operand = |o: &Operand, loose: &mut Vec<bool>, mark: &mut dyn FnMut(&Place, &mut Vec<bool>)| {
        if let Operand::Copy(p) | Operand::Move(p) = o {
            mark(p, loose);
        }
    };
    for b in &f.blocks {
        for s in &b.stmts {
            match &s.kind {
                StatementKind::Assign(d, rv) => {
                    if !d.proj.is_empty() {
                        mark(d, &mut loose);
                    }
                    match rv {
                        Rvalue::Use(o) | Rvalue::Unary(_, o) => operand(o, &mut loose, &mut mark),
                        Rvalue::Binary(_, x, y) => {
                            operand(x, &mut loose, &mut mark);
                            operand(y, &mut loose, &mut mark);
                        }
                        Rvalue::Aggregate(_, os) | Rvalue::Closure(_, _, os) => {
                            os.iter().for_each(|o| operand(o, &mut loose, &mut mark))
                        }
                        Rvalue::Ref(q, _)
                        | Rvalue::Discriminant(q)
                        | Rvalue::CopyOf(q)
                        | Rvalue::StrView(q) => mark(q, &mut loose),
                    }
                }
                StatementKind::Call { dest, callee, args } => {
                    let result_free = dest.proj.is_empty()
                        && tys.get(dest.local.0 as usize).is_some_and(holds_no_reference);
                    for o in args {
                        match o {
                            Operand::Copy(p) | Operand::Move(p) if p.proj.is_empty() && result_free => {}
                            _ => operand(o, &mut loose, &mut mark),
                        }
                    }
                    if let Callee::Value(o) = callee {
                        operand(o, &mut loose, &mut mark);
                    }
                    if !dest.proj.is_empty() {
                        mark(dest, &mut loose);
                    }
                }
                StatementKind::Drop { place, .. } if !place.proj.is_empty() => mark(place, &mut loose),
                _ => {}
            }
        }
        if let Terminator::Switch { value, .. } = &b.term {
            operand(value, &mut loose, &mut mark);
        }
    }
    for (t, x) in borrows {
        if loose.get(t).copied().unwrap_or(true)
            && let Some(w) = wild.get_mut(x)
        {
            *w = true;
        }
    }
    wild
}

/// The smallest local the state knows equal to `l`, through copies.
fn canonical(st: &State, l: u32) -> u32 {
    let mut c = l;
    loop {
        let smaller = st.facts.iter().find_map(|f| match f {
            Fact::Holds(Atom::Local(a), Rel::Eq, Atom::Local(b)) if *b == c && *a < c => Some(*a),
            _ => None,
        });
        match smaller {
            Some(a) => c = a,
            None => return c,
        }
    }
}

fn negate(a: Atom, r: Rel, b: Atom) -> (Atom, Rel, Atom) {
    match r {
        Rel::Lt => (b, Rel::Le, a),
        Rel::Le => (b, Rel::Lt, a),
        Rel::Eq => (a, Rel::Ne, b),
        Rel::Ne => (a, Rel::Eq, b),
    }
}

/// The values of `a OP b` over the operands' bounds, at the corners.
fn corners(op: BinOp, a: (i64, i64), b: (i64, i64)) -> Option<Vec<i128>> {
    let (a0, a1, b0, b1) = (i128::from(a.0), i128::from(a.1), i128::from(b.0), i128::from(b.1));
    match op {
        BinOp::Add => Some(vec![a0 + b0, a1 + b1]),
        BinOp::Sub => Some(vec![a0 - b1, a1 - b0]),
        BinOp::Mul => Some(vec![a0 * b0, a0 * b1, a1 * b0, a1 * b1]),
        _ => None,
    }
}

/// The bounds of `a OP b` within `int`: what the operation gives when it
/// does not overflow (an overflow panics).
fn arith(op: BinOp, a: (i64, i64), b: (i64, i64)) -> Option<(i64, i64)> {
    let c = corners(op, a, b)?;
    let clamp = |v: i128| v.clamp(i128::from(MIN), i128::from(MAX)) as i64;
    Some((clamp(*c.iter().min()?), clamp(*c.iter().max()?)))
}

/// Whether every `a OP b` over the operands' bounds fits an `int`.
fn fits(op: BinOp, a: (i64, i64), b: (i64, i64)) -> bool {
    corners(op, a, b).is_some_and(|c| c.iter().all(|v| *v >= i128::from(MIN) && *v <= i128::from(MAX)))
}

/// `x // d` rounded down, for a divisor of at least 1. Over divisors from
/// `lo` to `hi`, the smallest quotient of `x` divides by `hi` when `x` is
/// at or above 0 and by `lo` below it, the largest the other way round.
fn quotient(x: i64, d: i64) -> i64 {
    x.div_euclid(d.max(1))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts(n: usize) -> Facts {
        Facts {
            entry: Vec::new(),
            tracked: vec![Some((MIN, MAX)); n],
            wild: vec![false; n],
            tys: vec![Type::Int; n],
            frames: vec![false; n],
            constants: Vec::new(),
        }
    }

    #[test]
    fn bounds_decide_comparisons() {
        let f = facts(2);
        let mut st = State::default();
        f.set_bound(&mut st, 0, (0, 9));
        f.set_bound(&mut st, 1, (10, 20));
        assert_eq!(f.decide(&st, Atom::Local(0), Rel::Lt, Atom::Local(1)), Some(true));
        assert_eq!(f.decide(&st, Atom::Local(1), Rel::Le, Atom::Local(0)), Some(false));
        assert_eq!(f.decide(&st, Atom::Local(0), Rel::Ne, Atom::Const(10)), Some(true));
        assert_eq!(f.decide(&st, Atom::Local(0), Rel::Eq, Atom::Const(5)), None);
    }

    #[test]
    fn assuming_narrows_and_refuses_the_impossible() {
        let f = facts(2);
        let mut st = State::default();
        f.set_bound(&mut st, 1, (0, MAX));
        assert!(f.assume(&mut st, Atom::Local(0), Rel::Lt, Atom::Local(1)));
        assert_eq!(f.bound(&st, Atom::Local(0)), (MIN, MAX - 1));
        assert!(st.has(&Fact::Holds(Atom::Local(0), Rel::Lt, Atom::Local(1))));
        assert!(f.assume(&mut st, Atom::Const(0), Rel::Le, Atom::Local(0)));
        assert_eq!(f.bound(&st, Atom::Local(0)), (0, MAX - 1));
        assert!(!f.assume(&mut st, Atom::Local(0), Rel::Lt, Atom::Const(0)));
        assert_eq!(f.decide(&st, Atom::Local(1), Rel::Le, Atom::Local(0)), Some(false));
    }

    #[test]
    fn a_join_keeps_what_both_paths_know() {
        let f = facts(2);
        let (mut a, mut b) = (State::default(), State::default());
        f.set_bound(&mut a, 0, (0, 0));
        f.set_bound(&mut b, 0, (1, 5));
        a.add(Fact::Holds(Atom::Local(0), Rel::Lt, Atom::Local(1)));
        let j = f.join(&a, &b);
        assert_eq!(f.bound(&j, Atom::Local(0)), (0, 5));
        assert!(j.facts.is_empty());
        let mut grown = j.clone();
        f.set_bound(&mut grown, 0, (0, 6));
        f.widen(&j, &mut grown);
        assert_eq!(f.bound(&grown, Atom::Local(0)), (0, MAX));
    }

    #[test]
    fn arithmetic_bounds_and_overflow() {
        assert_eq!(arith(BinOp::Add, (0, 10), (1, 1)), Some((1, 11)));
        assert!(fits(BinOp::Add, (0, MAX - 1), (1, 1)));
        assert!(!fits(BinOp::Add, (0, MAX), (1, 1)));
        assert!(!fits(BinOp::Mul, (MIN, MAX), (3, 3)));
        assert!(fits(BinOp::Mul, (0, 1_000_000), (3, 3)));
        assert_eq!(quotient(-7, 2), -4);
        assert_eq!(quotient(7, 3), 2);
    }

    #[test]
    fn a_key_contains_its_parts_not_its_elements() {
        let list = Key { root: 1, steps: vec![Step::Field(0)] };
        let whole = Key { root: 1, steps: Vec::new() };
        let element = Key { root: 1, steps: vec![Step::Field(0), Step::Index(2)] };
        let inner = Key { root: 1, steps: vec![Step::Field(0), Step::Index(3), Step::Field(1)] };
        assert!(whole.contains(&list));
        assert!(list.contains(&list));
        assert!(!element.contains(&list));
        assert!(list.contains(&element));
        // An element at another index may be the same element.
        assert!(element.contains(&inner));
    }
}
