//! Loans and the borrow rules (04, "Borrowing rules in detail"; 18,
//! Phase 3, "The borrow checker on SIR").
//!
//! A loan is made by a borrow statement: `ref P`, `ref mut P`, `strview
//! P`, or a copy of a `ref mut` (a reborrow of what it points to). Each
//! parameter that can hold a loan also brings one: what the caller lent.
//!
//! **Which locals hold a loan** is a forward dataflow (union): a borrow's
//! destination holds the new loan and the loans of the borrowed value (a
//! reference is only valid while what it points into is); a copy or move
//! passes the value's loans on; a call's result holds the loans of the
//! arguments its signature says it borrows from (`from`, SIR's record of
//! the `from` clause and the elision rules); an intrinsic that stores an
//! argument through a `mut` argument (`list.push`) adds the argument's
//! loans to what that `mut` argument points to.
//!
//! **Which locals are live** is a backward dataflow: a local is live where
//! it may be read later before being overwritten; dropping a value whose
//! drop glue runs a user `Drop` reads it.
//!
//! A loan is live where a local holding it is live (non-lexical). An
//! access conflicts with a live loan of an overlapping place when the
//! access writes (assignment, move, drop, end of storage, `mut` borrow) or
//! when it reads and the loan is `mut`. A `ref mut` made for an argument
//! is two-phase: until its first use it conflicts only as a shared borrow
//! would, so `xs.push(len(xs))` is accepted; at its first use it must not
//! conflict with any live loan.
//!
//! Escapes: a `return` may carry only loans of parameters in the `from`
//! clause; a value holding a loan may not be stored in a plain struct or
//! enum; and a value holding a loan may be stored through a `mut`
//! parameter only if the loan is that parameter's own.

use crate::bitset::BitSet;
use crate::cfg::{Cfg, targets};
use crate::init::{dest, index_locals};
use crate::names;
use crate::types::Types;
use crate::{Arg, Finding, Options, Rule};
use sisu_diag::{MsgId, msg};
use sisu_sir::{
    Aggregate, Callee, Function, Loc, LocalId, Operand, Place, Program, Proj, Rvalue, Statement,
    StatementKind, Terminator, Type,
};
use std::collections::HashMap;

pub type LoanId = usize;

#[derive(Clone, Debug)]
pub enum LoanKind {
    /// A borrow of a place of this function.
    Local { place: Place, mutable: bool, two_phase: bool },
    /// What the caller lent through argument `i`.
    Param(u32),
}

#[derive(Clone, Debug)]
pub struct Loan {
    pub kind: LoanKind,
    pub loc: Option<Loc>,
}

impl Loan {
    fn place(&self) -> Option<&Place> {
        match &self.kind {
            LoanKind::Local { place, .. } => Some(place),
            LoanKind::Param(_) => None,
        }
    }

    fn mutable(&self) -> bool {
        matches!(self.kind, LoanKind::Local { mutable: true, .. })
    }

    fn two_phase(&self) -> bool {
        matches!(self.kind, LoanKind::Local { two_phase: true, .. })
    }
}

/// What a statement makes: the loan of its rvalue (a borrow) and the
/// reborrows of its `ref mut` operands, by operand index.
#[derive(Clone, Debug, Default)]
struct Made {
    rvalue: Option<LoanId>,
    reborrows: HashMap<usize, LoanId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct State {
    pub holds: Vec<BitSet>,
    /// For a local holding a reference, the loans that are the reference
    /// itself (rather than loans of what it points into), when known.
    pub direct: Vec<Option<BitSet>>,
    /// Two-phase loans whose reference has been used.
    pub activated: BitSet,
}

impl State {
    fn join(&mut self, o: &State) -> bool {
        let mut changed = false;
        for (a, b) in self.holds.iter_mut().zip(&o.holds) {
            changed |= a.union(b);
        }
        for (a, b) in self.direct.iter_mut().zip(&o.direct) {
            let n = match (a.as_ref(), b) {
                (Some(x), Some(y)) => {
                    let mut x = x.clone();
                    x.union(y);
                    Some(x)
                }
                _ => None,
            };
            if *a != n {
                *a = n;
                changed = true;
            }
        }
        changed |= self.activated.union(&o.activated);
        changed
    }
}

pub struct Analysis {
    pub loans: Vec<Loan>,
    made: HashMap<(usize, usize), Made>,
    /// The projected operand places whose type is a reference.
    ref_places: Vec<Place>,
    pub cfg: Cfg,
    pub entry: Vec<Option<State>>,
    pub live_in: Vec<BitSet>,
    nloc: usize,
    nloans: usize,
}

/// One operand's loans: what the value holds, which of those are the
/// reference itself, and what the referent holds.
struct OpLoans {
    holds: BitSet,
    direct: Option<BitSet>,
    inner: BitSet,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Access {
    Read,
    /// The reservation of a two-phase `ref mut`: a read.
    Reserve,
    /// A reborrow of what a `ref mut` points to, or the activation of a
    /// two-phase loan: an exclusive borrow.
    MutBorrow,
    Write,
    Move,
    /// A drop at the end of a scope.
    DropEnd,
    /// A drop that an assignment makes of the old value.
    DropOverwrite,
    Dead,
}

impl Access {
    fn writes(self) -> bool {
        !matches!(self, Access::Read | Access::Reserve)
    }
}

pub fn overlap(a: &Place, b: &Place) -> bool {
    if a.local != b.local {
        return false;
    }
    for (x, y) in a.proj.iter().zip(&b.proj) {
        match (x, y) {
            (Proj::Field(i), Proj::Field(j)) if i != j => return false,
            (Proj::Downcast(i), Proj::Downcast(j)) if i != j => return false,
            (Proj::Deref, Proj::Deref)
            | (Proj::Field(_), Proj::Field(_))
            | (Proj::Downcast(_), Proj::Downcast(_)) => {}
            // Indices and keys are not known statically: any two may be equal.
            (
                Proj::Index(_) | Proj::IndexFromEnd(_) | Proj::Key(_),
                Proj::Index(_) | Proj::IndexFromEnd(_) | Proj::Key(_),
            ) => {}
            _ => {}
        }
    }
    true
}

fn is_ref_mut(t: &Option<Type>) -> bool {
    matches!(t, Some(Type::Ref(_, true)))
}

impl Analysis {
    pub fn run(p: &Program, f: &Function, types: &Types) -> Analysis {
        let cfg = Cfg::new(f);
        let mut loans = Vec::new();
        let mut made: HashMap<(usize, usize), Made> = HashMap::new();
        // The parameters' loans first, then every borrow statement's.
        for i in 0..f.param_count {
            let ty = &f.locals[i as usize + 1].ty;
            if types.may_hold(ty) {
                loans.push(Loan { kind: LoanKind::Param(i), loc: f.locals[i as usize + 1].loc });
            }
        }
        for (b, block) in f.blocks.iter().enumerate() {
            for (si, s) in block.stmts.iter().enumerate() {
                let mut m = Made::default();
                if let StatementKind::Assign(_, rv) = &s.kind {
                    match rv {
                        Rvalue::Ref(pl, mutable) => {
                            // A `mut` borrow into a temporary (a call's
                            // argument or receiver) is two-phase; one bound
                            // to a name is active at once.
                            let dest = crate::init::dest(s).map(|d| d.local);
                            let temp = dest.is_some_and(|d| !names::is_named(f, d));
                            m.rvalue = Some(loans.len());
                            loans.push(Loan {
                                kind: LoanKind::Local {
                                    place: pl.clone(),
                                    mutable: *mutable,
                                    two_phase: *mutable && temp,
                                },
                                loc: s.loc,
                            });
                        }
                        Rvalue::StrView(pl) => {
                            m.rvalue = Some(loans.len());
                            loans.push(Loan {
                                kind: LoanKind::Local { place: pl.clone(), mutable: false, two_phase: false },
                                loc: s.loc,
                            });
                        }
                        _ => {}
                    }
                }
                for (oi, o) in crate::init::operands(s).into_iter().enumerate() {
                    if let Operand::Copy(pl) = o
                        && is_ref_mut(&types.place_ty(f, pl))
                    {
                        m.reborrows.insert(oi, loans.len());
                        loans.push(Loan {
                            kind: LoanKind::Local {
                                place: pl.project(Proj::Deref),
                                mutable: true,
                                two_phase: false,
                            },
                            loc: s.loc,
                        });
                    }
                }
                if m.rvalue.is_some() || !m.reborrows.is_empty() {
                    made.insert((b, si), m);
                }
            }
        }
        let nloc = f.locals.len();
        let nloans = loans.len();
        let mut ref_places = Vec::new();
        for block in &f.blocks {
            for s in &block.stmts {
                for o in crate::init::operands(s) {
                    if let Operand::Copy(pl) = o
                        && !pl.proj.is_empty()
                        && matches!(types.place_ty(f, pl), Some(Type::Ref(..)))
                        && !ref_places.contains(pl)
                    {
                        ref_places.push(pl.clone());
                    }
                }
            }
        }
        let mut a = Analysis {
            loans,
            made,
            ref_places,
            cfg,
            entry: vec![None; f.blocks.len()],
            live_in: vec![BitSet::new(nloc); f.blocks.len()],
            nloc,
            nloans,
        };
        if f.blocks.is_empty() {
            return a;
        }
        // Holds: forward.
        let mut init = a.empty_state();
        for (id, l) in a.loans.iter().enumerate() {
            if let LoanKind::Param(i) = l.kind {
                init.holds[i as usize + 1].insert(id);
                if matches!(f.locals[i as usize + 1].ty, Type::Ref(..)) {
                    let mut d = BitSet::new(nloans);
                    d.insert(id);
                    init.direct[i as usize + 1] = Some(d);
                }
            }
        }
        a.entry[0] = Some(init);
        let mut work: Vec<usize> = a.cfg.order.iter().rev().copied().collect();
        let mut queued = vec![true; f.blocks.len()];
        while let Some(b) = work.pop() {
            queued[b] = false;
            let Some(mut st) = a.entry[b].clone() else { continue };
            let block = &f.blocks[b];
            let mut unwind_states = Vec::new();
            for (si, s) in block.stmts.iter().enumerate() {
                a.transfer(p, f, types, b, si, s, &mut st);
                if block.unwind.is_some() {
                    unwind_states.push(st.clone());
                }
            }
            let mut succ: Vec<(usize, State)> = targets(block).into_iter().map(|t| (t, st.clone())).collect();
            if let Some(u) = block.unwind {
                for s in unwind_states {
                    succ.push((u.0 as usize, s));
                }
                if let Some(e) = a.entry[b].clone() {
                    succ.push((u.0 as usize, e));
                }
            }
            for (t, s) in succ {
                if t >= f.blocks.len() {
                    continue;
                }
                let changed = match &mut a.entry[t] {
                    None => {
                        a.entry[t] = Some(s);
                        true
                    }
                    Some(e) => e.join(&s),
                };
                if changed && !queued[t] {
                    queued[t] = true;
                    work.push(t);
                }
            }
        }
        // Liveness: backward.
        let order: Vec<usize> = a.cfg.order.clone();
        let mut changed = true;
        let mut rounds = 0;
        while changed && rounds < 10_000 {
            changed = false;
            rounds += 1;
            for &b in order.iter().rev() {
                let out = a.live_out(f, b);
                let live = a.live_before_each(f, types, b, &out);
                if let Some(first) = live.first()
                    && a.live_in[b] != *first
                {
                    a.live_in[b] = first.clone();
                    changed = true;
                }
            }
        }
        // The union over a finite set always converges; if the bound is
        // ever reached anyway, every local is live everywhere, which is
        // sound (it only rejects more).
        if changed {
            let mut all = BitSet::new(a.nloc);
            for l in 0..a.nloc {
                all.insert(l);
            }
            for live in &mut a.live_in {
                *live = all.clone();
            }
        }
        a
    }

    fn empty_state(&self) -> State {
        State {
            holds: vec![BitSet::new(self.nloans); self.nloc],
            direct: vec![None; self.nloc],
            activated: BitSet::new(self.nloans),
        }
    }

    // ---------------------------------------------------------------- holds

    /// The loans of the value at a place: through a reference, what the
    /// referent holds, not the reference's own loan.
    fn value_loans(&self, st: &State, p: &Place) -> BitSet {
        let Some(k) = p.proj.iter().rposition(|x| matches!(x, Proj::Deref)) else {
            return st.holds[p.local.0 as usize].clone();
        };
        if k == 0 {
            return self.inner_of(st, p.local);
        }
        st.holds[p.local.0 as usize].clone()
    }

    /// What the referent of a reference local holds.
    fn inner_of(&self, st: &State, r: LocalId) -> BitSet {
        let holds = &st.holds[r.0 as usize];
        let Some(direct) = &st.direct[r.0 as usize] else { return holds.clone() };
        let mut out = holds.clone();
        out.subtract(direct);
        for l in direct.iter() {
            match &self.loans[l].kind {
                LoanKind::Local { place, .. } => out.union(&self.value_loans(st, place)),
                LoanKind::Param(_) => out.insert(l),
            };
        }
        out
    }

    fn ref_typed(&self, p: &Place) -> bool {
        self.ref_places.contains(p)
    }

    fn op_loans(&self, st: &State, made: Option<&Made>, oi: usize, o: &Operand) -> OpLoans {
        let empty = BitSet::new(self.nloans);
        match o {
            Operand::Const(_) => OpLoans { holds: empty.clone(), direct: None, inner: empty },
            Operand::Copy(p) | Operand::Move(p) => {
                let inner =
                    if p.proj.is_empty() { self.inner_of(st, p.local) } else { self.value_loans(st, p) };
                if let Some(&r) = made.and_then(|m| m.reborrows.get(&oi)) {
                    let mut holds = st.holds[p.local.0 as usize].clone();
                    holds.insert(r);
                    let mut d = BitSet::new(self.nloans);
                    d.insert(r);
                    return OpLoans { holds, direct: Some(d), inner };
                }
                let holds = self.value_loans(st, p);
                // A reference read out of a value (an option's payload, a
                // tuple's element) is treated as pointing into every place
                // the value holds a loan of: the referent's loans are then
                // what those places hold, which includes what it holds.
                let direct = if p.proj.is_empty() {
                    st.direct[p.local.0 as usize].clone()
                } else if matches!(o, Operand::Copy(_)) && self.ref_typed(p) {
                    Some(holds.clone())
                } else {
                    None
                };
                OpLoans { holds, direct, inner }
            }
        }
    }

    /// The loans written by an assignment to `d`, with those of the keys
    /// it inserts (`d{k} = v` keeps `k` in the dict).
    fn with_keys(&self, st: &State, d: &Place, mut loans: BitSet) -> BitSet {
        for x in &d.proj {
            if let Proj::Key(k) = x {
                loans.union(&st.holds[k.0 as usize]);
            }
        }
        loans
    }

    /// Stores loans into a place: a whole local is replaced, a part of one
    /// is added to, and a place through a reference adds to what the
    /// reference points to.
    fn write(
        &self,
        f: &Function,
        types: &Types,
        st: &mut State,
        d: &Place,
        loans: BitSet,
        direct: Option<BitSet>,
    ) {
        let l = d.local.0 as usize;
        let filter = |local: usize, mut b: BitSet| {
            if !types.may_hold(&f.locals[local].ty) {
                b.clear();
            }
            b
        };
        match d.proj.iter().position(|x| matches!(x, Proj::Deref)) {
            None if d.proj.is_empty() => {
                st.holds[l] = filter(l, loans);
                st.direct[l] = direct;
            }
            None => {
                let add = filter(l, loans);
                st.holds[l].union(&add);
                st.direct[l] = None;
            }
            Some(k) => {
                let targets: Vec<LoanId> = if k == 0 {
                    match &st.direct[l] {
                        Some(dset) => dset.iter().collect(),
                        None => st.holds[l].iter().collect(),
                    }
                } else {
                    st.holds[l].iter().collect()
                };
                for t in targets {
                    if let LoanKind::Local { place, mutable: true, .. } = &self.loans[t].kind
                        && !place.proj.iter().any(|x| matches!(x, Proj::Deref))
                    {
                        let root = place.local.0 as usize;
                        let add = filter(root, loans.clone());
                        st.holds[root].union(&add);
                        st.direct[root] = None;
                    }
                }
            }
        }
    }

    /// The loans a call's result may hold.
    fn call_result(
        &self,
        p: &Program,
        callee: &Callee,
        ops: &[OpLoans],
        callee_loans: Option<&OpLoans>,
    ) -> BitSet {
        let mut out = BitSet::new(self.nloans);
        match callee {
            Callee::Trait(name, _) => match p.trait_method(name) {
                Some(d) => {
                    for &i in &d.from {
                        if let Some(o) = ops.get(i as usize) {
                            out.union(&o.holds);
                        }
                    }
                    for &i in &d.inner {
                        if let Some(o) = ops.get(i as usize) {
                            out.union(&o.inner);
                        }
                    }
                }
                None => {
                    for o in ops {
                        out.union(&o.holds);
                    }
                }
            },
            Callee::Fn(name, _) => {
                if let Some(g) = p.function(name) {
                    for &i in &g.from {
                        if let Some(o) = ops.get(i as usize) {
                            out.union(&o.holds);
                        }
                    }
                    for &i in &g.inner {
                        if let Some(o) = ops.get(i as usize) {
                            out.union(&o.inner);
                        }
                    }
                } else {
                    for o in ops {
                        out.union(&o.holds);
                    }
                }
            }
            Callee::Intrinsic(name, _) => match p.intrinsic(name) {
                Some(d) => {
                    for &i in &d.from {
                        if let Some(o) = ops.get(i as usize) {
                            out.union(&o.holds);
                        }
                    }
                    for &i in &d.inner {
                        if let Some(o) = ops.get(i as usize) {
                            out.union(&o.inner);
                        }
                    }
                }
                None => {
                    for o in ops {
                        out.union(&o.holds);
                    }
                }
            },
            Callee::Value(_) => {
                for o in ops {
                    out.union(&o.holds);
                }
                if let Some(c) = callee_loans {
                    out.union(&c.holds);
                }
            }
        }
        out
    }

    #[allow(clippy::too_many_arguments)]
    fn transfer(
        &self,
        p: &Program,
        f: &Function,
        types: &Types,
        b: usize,
        si: usize,
        s: &Statement,
        st: &mut State,
    ) {
        let made = self.made.get(&(b, si));
        // A use of a reference activates its two-phase loans.
        for l in self.used_locals(f, types, s) {
            let held = st.holds[l.0 as usize].clone();
            for x in held.iter() {
                if self.loans[x].two_phase() {
                    st.activated.insert(x);
                }
            }
        }
        match &s.kind {
            StatementKind::Assign(d, rv) => {
                let (loans, direct) = match rv {
                    Rvalue::Use(o) => {
                        let ol = self.op_loans(st, made, 0, o);
                        (ol.holds, ol.direct)
                    }
                    Rvalue::Ref(pl, _) | Rvalue::StrView(pl) => {
                        let mut h = st.holds[pl.local.0 as usize].clone();
                        let mut dset = BitSet::new(self.nloans);
                        if let Some(l) = made.and_then(|m| m.rvalue) {
                            h.insert(l);
                            dset.insert(l);
                            // A new borrow is reserved again until its first use.
                            st.activated.remove(l);
                        }
                        (h, Some(dset))
                    }
                    Rvalue::Aggregate(_, ops) | Rvalue::Closure(_, _, ops) => {
                        let mut h = BitSet::new(self.nloans);
                        for (oi, o) in ops.iter().enumerate() {
                            h.union(&self.op_loans(st, made, oi, o).holds);
                        }
                        (h, None)
                    }
                    Rvalue::CopyOf(pl) => (self.value_loans(st, pl), None),
                    Rvalue::Binary(..) | Rvalue::Unary(..) | Rvalue::Discriminant(_) => {
                        (BitSet::new(self.nloans), None)
                    }
                };
                if let Rvalue::Use(Operand::Move(src)) = rv
                    && src.proj.is_empty()
                {
                    st.holds[src.local.0 as usize].clear();
                    st.direct[src.local.0 as usize] = None;
                }
                let loans = self.with_keys(st, d, loans);
                self.write(f, types, st, d, loans, direct);
            }
            StatementKind::Call { dest: d, callee, args } => {
                let ops: Vec<OpLoans> =
                    args.iter().enumerate().map(|(oi, o)| self.op_loans(st, made, oi, o)).collect();
                let callee_loans = match callee {
                    Callee::Value(o) => Some(self.op_loans(st, made, args.len(), o)),
                    _ => None,
                };
                let result = self.call_result(p, callee, &ops, callee_loans.as_ref());
                // A callee may store an argument it takes by value (with
                // `own`, or a view) through a `mut` one.
                let kept = match callee {
                    Callee::Intrinsic(name, _) => Some(kept_args(p, name, ops.len())),
                    Callee::Fn(name, _) => Some(match p.function(name) {
                        Some(g) => (0..ops.len()).filter(|&i| lent_by_value(g, i as u32)).collect(),
                        None => (0..ops.len()).collect(),
                    }),
                    // Any impl may run: any argument may be kept.
                    Callee::Trait(..) => Some((0..ops.len()).collect()),
                    Callee::Value(_) => None,
                };
                if let Some(kept) = kept {
                    for (ai, a) in ops.iter().enumerate() {
                        let mut stored = BitSet::new(self.nloans);
                        for (bi, bo) in ops.iter().enumerate() {
                            if bi != ai && kept.contains(&bi) {
                                stored.union(&bo.holds);
                            }
                        }
                        if stored.is_empty() {
                            continue;
                        }
                        for t in a.holds.iter() {
                            if let LoanKind::Local { place, mutable: true, .. } = &self.loans[t].kind
                                && !place.proj.iter().any(|x| matches!(x, Proj::Deref))
                            {
                                let root = place.local.0 as usize;
                                if types.may_hold(&f.locals[root].ty) {
                                    st.holds[root].union(&stored);
                                    st.direct[root] = None;
                                }
                            }
                        }
                    }
                }
                for o in args {
                    if let Operand::Move(src) = o
                        && src.proj.is_empty()
                    {
                        st.holds[src.local.0 as usize].clear();
                        st.direct[src.local.0 as usize] = None;
                    }
                }
                let result = self.with_keys(st, d, result);
                self.write(f, types, st, d, result, None);
            }
            StatementKind::Drop { place, .. } => {
                if place.proj.is_empty() {
                    st.holds[place.local.0 as usize].clear();
                    st.direct[place.local.0 as usize] = None;
                }
            }
            StatementKind::StorageDead(l) => {
                st.holds[l.0 as usize].clear();
                st.direct[l.0 as usize] = None;
            }
        }
    }

    // ---------------------------------------------------------------- liveness

    /// The locals a statement reads.
    fn used_locals(&self, f: &Function, types: &Types, s: &Statement) -> Vec<LocalId> {
        let mut out = Vec::new();
        let place = |p: &Place, out: &mut Vec<LocalId>| {
            out.push(p.local);
            out.extend(index_locals(p));
        };
        for o in crate::init::operands(s) {
            if let Operand::Copy(p) | Operand::Move(p) = o {
                place(p, &mut out);
            }
        }
        match &s.kind {
            StatementKind::Assign(d, rv) => {
                if let Rvalue::Ref(p, _) | Rvalue::StrView(p) | Rvalue::Discriminant(p) | Rvalue::CopyOf(p) =
                    rv
                {
                    place(p, &mut out);
                }
                out.extend(index_locals(d));
                if d.proj.iter().any(|x| matches!(x, Proj::Deref)) {
                    out.push(d.local);
                }
            }
            StatementKind::Call { dest: d, .. } => {
                out.extend(index_locals(d));
                if d.proj.iter().any(|x| matches!(x, Proj::Deref)) {
                    out.push(d.local);
                }
            }
            StatementKind::Drop { place: p, .. } => {
                // A user `Drop` may read the references the value holds.
                if let Some(t) = types.place_ty(f, p)
                    && types.has_drop_glue(&t)
                {
                    out.push(p.local);
                }
                out.extend(index_locals(p));
            }
            StatementKind::StorageDead(_) => {}
        }
        out
    }

    /// The locals a statement overwrites as a whole.
    fn defined_locals(s: &Statement) -> Option<LocalId> {
        match &s.kind {
            StatementKind::Assign(d, _) | StatementKind::Call { dest: d, .. } if d.proj.is_empty() => {
                Some(d.local)
            }
            StatementKind::Drop { place, .. } if place.proj.is_empty() => Some(place.local),
            StatementKind::StorageDead(l) => Some(*l),
            _ => None,
        }
    }

    fn term_uses(block: &sisu_sir::BasicBlock) -> Vec<LocalId> {
        match &block.term {
            Terminator::Switch { value: Operand::Copy(p) | Operand::Move(p), .. } => vec![p.local],
            Terminator::Return => vec![LocalId(0)],
            _ => Vec::new(),
        }
    }

    fn live_out(&self, f: &Function, b: usize) -> BitSet {
        let mut out = BitSet::new(self.nloc);
        for t in targets(&f.blocks[b]) {
            if let Some(l) = self.live_in.get(t) {
                out.union(l);
            }
        }
        out
    }

    /// Live locals before each statement of a block, and before its
    /// terminator (the last element); `out` is live at the block's end.
    fn live_before_each(&self, f: &Function, types: &Types, b: usize, out: &BitSet) -> Vec<BitSet> {
        let block = &f.blocks[b];
        let unwind_live = block.unwind.and_then(|u| self.live_in.get(u.0 as usize)).cloned();
        let mut live = out.clone();
        for l in Self::term_uses(block) {
            live.insert(l.0 as usize);
        }
        if let Some(u) = &unwind_live {
            live.union(u);
        }
        let mut res = vec![live.clone()];
        for s in block.stmts.iter().rev() {
            if let Some(d) = Self::defined_locals(s) {
                live.remove(d.0 as usize);
            }
            for l in self.used_locals(f, types, s) {
                live.insert(l.0 as usize);
            }
            if let Some(u) = &unwind_live {
                live.union(u);
            }
            res.push(live.clone());
        }
        res.reverse();
        res
    }

    /// The holds state before each statement of a block (and at its end).
    fn states(&self, p: &Program, f: &Function, types: &Types, b: usize) -> Option<Vec<State>> {
        let mut st = self.entry[b].clone()?;
        let mut out = Vec::new();
        for (si, s) in f.blocks[b].stmts.iter().enumerate() {
            out.push(st.clone());
            self.transfer(p, f, types, b, si, s, &mut st);
        }
        out.push(st);
        Some(out)
    }

    fn live_loans(&self, st: &State, live: &BitSet) -> BitSet {
        let mut out = BitSet::new(self.nloans);
        for l in live.iter() {
            if let Some(h) = st.holds.get(l) {
                out.union(h);
            }
        }
        out
    }

    // ---------------------------------------------------------------- checking

    pub fn findings(&self, p: &Program, f: &Function, types: &Types, opts: &Options) -> Vec<Finding> {
        let mut out = Vec::new();
        for b in 0..f.blocks.len() {
            let Some(states) = self.states(p, f, types, b) else { continue };
            let lout = self.live_out(f, b);
            let lives = self.live_before_each(f, types, b, &lout);
            let block = &f.blocks[b];
            // The unwinding path drops in the order of the normal one, so a
            // conflict there is one the normal path reports.
            if block.cleanup {
                continue;
            }
            for (si, s) in block.stmts.iter().enumerate() {
                let before = &states[si];
                let after = &states[si + 1];
                let pre = self.live_loans(before, &lives[si]);
                let post = self.live_loans(after, &lives[si + 1]);
                if opts.on(Rule::Conflicts) {
                    self.check_conflicts(
                        p,
                        f,
                        types,
                        b,
                        si,
                        s,
                        before,
                        after,
                        &pre,
                        &post,
                        &lives[si + 1],
                        &mut out,
                    );
                }
                if opts.on(Rule::Escapes) {
                    self.check_stores(p, f, types, b, si, s, before, &mut out);
                }
            }
            if opts.on(Rule::Escapes)
                && matches!(block.term, Terminator::Return)
                && let Some(st) = states.last()
            {
                self.check_return(f, types, b, block.loc, st, &mut out);
            }
        }
        // A borrow that outlives its place is the mistake; the conflicts
        // the same loan meets afterwards (the place made again in the
        // next iteration) follow from it.
        let gone: Vec<u32> = out
            .iter()
            .filter(|fd| fd.msg == msg::BORROW_DOES_NOT_LIVE_LONG_ENOUGH)
            .filter_map(|fd| fd.loan)
            .collect();
        out.retain(|fd| {
            fd.msg == msg::BORROW_DOES_NOT_LIVE_LONG_ENOUGH || !fd.loan.is_some_and(|l| gone.contains(&l))
        });
        out
    }

    /// The accesses a statement makes, in order: (place, kind, made loan
    /// to exclude, whether it happens after the statement's operands).
    fn accesses(
        &self,
        f: &Function,
        types: &Types,
        b: usize,
        si: usize,
        s: &Statement,
    ) -> Vec<(Place, Access, Option<LoanId>)> {
        let made = self.made.get(&(b, si));
        let mut v = Vec::new();
        for (oi, o) in crate::init::operands(s).into_iter().enumerate() {
            match o {
                Operand::Copy(pl) => {
                    for l in index_locals(pl) {
                        v.push((Place::local(l), Access::Read, None));
                    }
                    if let Some(&r) = made.and_then(|m| m.reborrows.get(&oi)) {
                        v.push((pl.project(Proj::Deref), Access::MutBorrow, Some(r)));
                    } else {
                        v.push((pl.clone(), Access::Read, None));
                    }
                }
                Operand::Move(pl) => {
                    for l in index_locals(pl) {
                        v.push((Place::local(l), Access::Read, None));
                    }
                    v.push((pl.clone(), Access::Move, None));
                }
                Operand::Const(_) => {}
            }
        }
        match &s.kind {
            StatementKind::Assign(_, rv) => match rv {
                Rvalue::Ref(pl, true) => v.push((pl.clone(), Access::Reserve, made.and_then(|m| m.rvalue))),
                Rvalue::Ref(pl, false)
                | Rvalue::StrView(pl)
                | Rvalue::Discriminant(pl)
                | Rvalue::CopyOf(pl) => {
                    for l in index_locals(pl) {
                        v.push((Place::local(l), Access::Read, None));
                    }
                    v.push((pl.clone(), Access::Read, made.and_then(|m| m.rvalue)));
                }
                _ => {}
            },
            StatementKind::Drop { place, .. } => {
                let overwrite = names::is_overwrite(f, b, place);
                v.push((
                    place.clone(),
                    if overwrite { Access::DropOverwrite } else { Access::DropEnd },
                    None,
                ));
            }
            StatementKind::StorageDead(l) => v.push((Place::local(*l), Access::Dead, None)),
            StatementKind::Call { .. } => {}
        }
        let _ = (f, types);
        v
    }

    #[allow(clippy::too_many_arguments)]
    fn check_conflicts(
        &self,
        p: &Program,
        f: &Function,
        types: &Types,
        b: usize,
        si: usize,
        s: &Statement,
        before: &State,
        after: &State,
        pre: &BitSet,
        post: &BitSet,
        post_live: &BitSet,
        out: &mut Vec<Finding>,
    ) {
        let made = self.made.get(&(b, si));
        // Loans made by earlier operands of this statement are live for
        // the later ones (two `mut` reborrows passed to one call).
        let mut pending = BitSet::new(self.nloans);
        let report = |acc: Access, place: &Place, loan: LoanId, out: &mut Vec<Finding>| {
            if let Some(fd) = self.conflict(p, f, types, b, si, s, acc, place, loan) {
                out.push(fd);
            }
        };
        // Activation of two-phase loans at their first use.
        let used = self.used_locals(f, types, s);
        let mut activating = Vec::new();
        for l in &used {
            for x in before.holds[l.0 as usize].iter() {
                if self.loans[x].two_phase() && !before.activated.contains(x) && !activating.contains(&x) {
                    activating.push(x);
                }
            }
        }
        for x in activating {
            let Some(place) = self.loans[x].place().cloned() else { continue };
            for y in pre.iter() {
                if y != x && self.conflicts_with(&place, Access::MutBorrow, y, before) {
                    report(Access::MutBorrow, &place, y, out);
                }
            }
        }
        let after_dest = matches!(s.kind, StatementKind::Drop { .. } | StatementKind::StorageDead(_));
        // A loan only the return value holds is reported at the `return`.
        let post_without_return = {
            let mut lives = BitSet::new(self.nloc);
            for l in post_live.iter() {
                if l != 0 {
                    lives.insert(l);
                }
            }
            self.live_loans(after, &lives)
        };
        for (place, acc, own) in self.accesses(f, types, b, si, s) {
            let live = if after_dest { &post_without_return } else { pre };
            for y in live.iter().chain(pending.iter()) {
                if Some(y) == own {
                    continue;
                }
                if self.conflicts_with(&place, acc, y, before) {
                    report(acc, &place, y, out);
                }
            }
            if let Some(r) = own
                && made.is_some_and(|m| m.reborrows.values().any(|v| *v == r))
            {
                pending.insert(r);
            }
        }
        // The destination is written after the operands are evaluated.
        if let Some(d) = dest(s) {
            for y in post.iter() {
                if made.is_some_and(|m| m.rvalue == Some(y) || m.reborrows.values().any(|v| *v == y)) {
                    continue;
                }
                if self.conflicts_with(d, Access::Write, y, before) {
                    report(Access::Write, d, y, out);
                }
            }
        }
    }

    /// Whether an access to `place` conflicts with live loan `y`.
    fn conflicts_with(&self, place: &Place, acc: Access, y: LoanId, st: &State) -> bool {
        let loan = &self.loans[y];
        let Some(lp) = loan.place() else { return false };
        if !overlap(place, lp) {
            return false;
        }
        if acc.writes() {
            return true;
        }
        // A read conflicts with a `mut` loan, unless it is still reserved.
        loan.mutable() && (!loan.two_phase() || st.activated.contains(y))
    }

    #[allow(clippy::too_many_arguments)]
    fn conflict(
        &self,
        p: &Program,
        f: &Function,
        types: &Types,
        b: usize,
        si: usize,
        s: &Statement,
        acc: Access,
        place: &Place,
        y: LoanId,
    ) -> Option<Finding> {
        let loan = &self.loans[y];
        let lp = loan.place()?;
        let later = self.later_use(p, f, types, b, si, y);
        let later_loc = later.map(|(l, _)| l);
        let line = loan.loc.map(|l| l.line).unwrap_or(0).to_string();
        // Where the borrow ends; for a place that is gone, where the borrow
        // is still used.
        let gone = matches!(acc, Access::DropEnd | Access::Dead);
        let end = if gone { None } else { self.last_live_line(p, f, types, y) }
            .or_else(|| later_loc.flatten().map(|l| l.line))
            .unwrap_or(0)
            .to_string();
        // Name what both sides are about: the longer common prefix of the
        // accessed place and the borrowed one (`xs` for a push while an
        // element of `xs` is borrowed).
        let name_place = match acc {
            Access::DropEnd | Access::Dead => place.clone(),
            _ => common_prefix(place, lp),
        };
        let name = names::place_name(types, f, &shorten(&name_place));
        let m: MsgId = match acc {
            Access::Read => {
                if loan.mutable() {
                    msg::BORROW_USE_WHILE_MUT_BORROWED
                } else {
                    return None;
                }
            }
            Access::Reserve | Access::MutBorrow => msg::BORROW_MUT_WHILE_BORROWED,
            Access::Write | Access::DropOverwrite => msg::BORROW_MODIFY_WHILE_BORROWED,
            Access::Move => msg::BORROW_MOVE_WHILE_BORROWED,
            Access::DropEnd | Access::Dead => msg::BORROW_DOES_NOT_LIVE_LONG_ENOUGH,
        };
        let holder = later.and_then(|(_, h)| h).filter(|h| names::is_named(f, *h));
        let label = match holder {
            Some(h) => (msg::BORROW_BORROWED_BY, vec![("borrower", Arg::Text(names::local_name(f, h)))]),
            None => (msg::BORROW_BORROWED_HERE, Vec::new()),
        };
        let mut fd = Finding::new(m, f, s.loc)
            .with_arg("name", name)
            .with("line", line)
            .with("end", end.clone())
            .label(loan.loc, label.0, label.1)
            .at(b, si);
        fd.loan = Some(y as u32);
        if let Some(Some(l)) = later_loc {
            fd = fd.label(Some(l), msg::BORROW_LATER_USED, Vec::new());
        }
        if !matches!(acc, Access::DropEnd | Access::Dead) {
            fd = fd.note(msg::BORROW_MOVE_AFTER, vec![("end", Arg::Text(end))]);
        } else if names::is_named(f, name_place.local) {
            fd = fd.note(
                msg::BORROW_DECLARE_OUTSIDE,
                vec![("name", names::place_name(types, f, &shorten(&name_place)))],
            );
        } else {
            fd = fd.note(msg::BORROW_BIND_TEMPORARY, Vec::new());
        }
        Some(fd)
    }

    /// The last source line at which loan `y` is live: where the borrow
    /// ends, for the message (in a loop that uses it at its head, the end
    /// of the loop's body).
    fn last_live_line(&self, p: &Program, f: &Function, types: &Types, y: LoanId) -> Option<u32> {
        let mut last: Option<u32> = None;
        let mut see = |l: Option<Loc>| {
            if let Some(l) = l {
                last = Some(last.map_or(l.end_line, |m| m.max(l.end_line)));
            }
        };
        for b in 0..f.blocks.len() {
            let block = &f.blocks[b];
            if block.cleanup {
                continue;
            }
            let Some(states) = self.states(p, f, types, b) else { continue };
            let lout = self.live_out(f, b);
            let lives = self.live_before_each(f, types, b, &lout);
            for (si, s) in block.stmts.iter().enumerate() {
                if self.live_loans(&states[si], &lives[si]).contains(y) {
                    see(s.loc);
                }
            }
            let n = block.stmts.len();
            if self.live_loans(&states[n], &lives[n]).contains(y) {
                see(block.loc);
            }
        }
        last
    }

    /// The first statement after (b, si) that uses a local holding loan
    /// `y`, and that local.
    fn later_use(
        &self,
        p: &Program,
        f: &Function,
        types: &Types,
        b: usize,
        si: usize,
        y: LoanId,
    ) -> Option<(Option<Loc>, Option<LocalId>)> {
        let mut visited = vec![false; f.blocks.len()];
        let mut queue: std::collections::VecDeque<(usize, usize)> = std::collections::VecDeque::new();
        queue.push_back((b, si));
        let mut first = true;
        while let Some((blk, start)) = queue.pop_front() {
            if !first {
                if visited[blk] {
                    continue;
                }
                visited[blk] = true;
            }
            let states = self.states(p, f, types, blk)?;
            let block = &f.blocks[blk];
            let from = if first { start } else { 0 };
            first = false;
            // From the conflicting statement itself: a use of the loan by
            // its own operands counts.
            for (s, st) in block.stmts.iter().zip(&states).skip(from) {
                if s.loc.is_none() {
                    continue;
                }
                for l in self.used_locals(f, types, s) {
                    if st.holds[l.0 as usize].contains(y) {
                        return Some((s.loc, Some(l)));
                    }
                }
            }
            let end = &states[block.stmts.len()];
            if block.loc.is_some() {
                for l in Self::term_uses(block) {
                    if end.holds[l.0 as usize].contains(y) {
                        return Some((block.loc, Some(l)));
                    }
                }
            }
            for t in targets(block) {
                if !visited[t] {
                    queue.push_back((t, 0));
                }
            }
        }
        None
    }

    /// How a label names where a loan comes from: the borrow, or for what
    /// the caller lent, the parameter.
    fn loan_label(&self, f: &Function, l: LoanId) -> (MsgId, Vec<(&'static str, Arg)>) {
        match &self.loans[l].kind {
            LoanKind::Param(i) => {
                (msg::BORROW_DECLARED_HERE, vec![("name", Arg::Text(names::local_name(f, LocalId(i + 1))))])
            }
            LoanKind::Local { .. } => (msg::BORROW_BORROWED_HERE, Vec::new()),
        }
    }

    // ---------------------------------------------------------------- escapes

    /// The `mut` parameter whose referent an operand points into, when the
    /// operand is a `ref mut` that may reach the caller's memory: the
    /// parameter itself, or a borrow through it (followed through the
    /// locals that hold such borrows).
    fn caller_target(
        &self,
        f: &Function,
        types: &Types,
        st: &State,
        o: &Operand,
        ol: &OpLoans,
    ) -> Option<u32> {
        let (Operand::Copy(p) | Operand::Move(p)) = o else { return None };
        // A `mut` reference, or a dict entry, which writes into its dict.
        match types.place_ty(f, p) {
            Some(Type::Ref(_, true)) => {}
            Some(Type::Named(n, _)) if n == "Entry" => {}
            _ => return None,
        }
        self.caller_param(f, st, ol.direct.as_ref().unwrap_or(&ol.holds).iter().collect())
    }

    /// The `mut` parameter whose referent one of the loans `work` points
    /// into, following borrows through the locals that hold them.
    fn caller_param(&self, f: &Function, st: &State, mut work: Vec<LoanId>) -> Option<u32> {
        let is_mut_param =
            |i: u32| i < f.param_count && matches!(f.locals[i as usize + 1].ty, Type::Ref(_, true));
        let mut seen = BitSet::new(self.nloans);
        while let Some(t) = work.pop() {
            if !seen.insert(t) {
                continue;
            }
            match &self.loans[t].kind {
                LoanKind::Param(i) if is_mut_param(*i) => return Some(*i),
                LoanKind::Param(_) => {}
                LoanKind::Local { place, .. } => {
                    // A borrow of a `mut` parameter, or of a place behind
                    // it (`ref mut xs` then `.*.*` reaches the caller too).
                    let l = place.local.0;
                    if l >= 1 && is_mut_param(l - 1) {
                        return Some(l - 1);
                    }
                    let Some(k) = place.proj.iter().position(|x| matches!(x, Proj::Deref)) else { continue };
                    // A borrow through another reference: what that one
                    // points to.
                    let set = match &st.direct[l as usize] {
                        Some(d) if k == 0 => d,
                        _ => &st.holds[l as usize],
                    };
                    work.extend(set.iter());
                }
            }
        }
        None
    }

    #[allow(clippy::too_many_arguments)]
    fn check_stores(
        &self,
        p: &Program,
        f: &Function,
        types: &Types,
        b: usize,
        si: usize,
        s: &Statement,
        st: &State,
        out: &mut Vec<Finding>,
    ) {
        let made = self.made.get(&(b, si));
        // An intrinsic that stores an argument through a `mut` parameter
        // of this function stores it in the caller's memory: only that
        // parameter's own loans may go there, and those of the arguments
        // the caller handed over by value, which a caller assumes may be
        // kept by its `mut` arguments.
        if let StatementKind::Call { callee: Callee::Intrinsic(name, _), args, .. } = &s.kind {
            let ops: Vec<OpLoans> =
                args.iter().enumerate().map(|(oi, o)| self.op_loans(st, made, oi, o)).collect();
            let kept = kept_args(p, name, ops.len());
            for (ai, a) in ops.iter().enumerate() {
                let Some(i) = self.caller_target(f, types, st, &args[ai], a) else { continue };
                let Type::Ref(inner, true) = &f.locals[i as usize + 1].ty else { continue };
                if !types.may_hold(inner) {
                    continue;
                }
                for (bi, bo) in ops.iter().enumerate() {
                    if bi == ai || !kept.contains(&bi) {
                        continue;
                    }
                    let foreign = bo.holds.iter().find(
                        |x| !matches!(self.loans[*x].kind, LoanKind::Param(j) if j == i || lent_by_value(f, j)),
                    );
                    if let Some(x) = foreign {
                        out.push(
                            Finding::new(msg::BORROW_STORE_INTO_CALLER, f, s.loc)
                                .with_arg("name", self.loan_name(f, types, x))
                                .with("param", names::local_name(f, LocalId(i + 1)))
                                .with("function", display_fn(&f.name))
                                .label(self.loans[x].loc, self.loan_label(f, x).0, self.loan_label(f, x).1)
                                .at(b, si),
                        );
                        return;
                    }
                }
            }
        }
        // A plain struct or enum cannot hold a loan.
        if let StatementKind::Assign(
            _,
            Rvalue::Aggregate(agg @ (Aggregate::Struct(n) | Aggregate::Variant(n, _)), ops),
        ) = &s.kind
            && types.is_plain_aggregate(n)
        {
            for (oi, o) in ops.iter().enumerate() {
                // A field whose type is a type parameter holds what the
                // argument holds: the generic type is view-like through
                // its arguments, as `list[Token]` is (04, "Views").
                let variant = match agg {
                    Aggregate::Variant(_, v) => types.variant_index(n, v),
                    _ => None,
                };
                if types.declared_field_ty(n, variant, oi as u32).is_some_and(|t| types.carried_by_params(&t))
                {
                    continue;
                }
                let ol = self.op_loans(st, made, oi, o);
                if let Some(l) = ol.holds.iter().next() {
                    out.push(Self::declare_view(
                        types,
                        n,
                        Finding::new(msg::BORROW_STORE_IN_PLAIN_TYPE, f, s.loc)
                            .with("ty", n.clone())
                            .with_arg("name", self.loan_name(f, types, l))
                            .label(self.loans[l].loc, self.loan_label(f, l).0, self.loan_label(f, l).1)
                            .at(b, si),
                    ));
                    break;
                }
            }
        }
        // A place that cannot hold a loan, or memory of the caller.
        let Some(d) = dest(s) else { return };
        // An assignment to `d{k}` inserts the key when it is missing: the
        // dict keeps the key's loans.
        for (i, x) in d.proj.iter().enumerate() {
            if let Proj::Key(k) = x {
                let keys = st.holds[k.0 as usize].clone();
                let dict = Place { local: d.local, proj: d.proj[..i].to_vec() };
                if !keys.is_empty() && self.store_into(f, types, b, si, s, st, &dict, &keys, out) {
                    return;
                }
            }
        }
        let stored: BitSet = match &s.kind {
            StatementKind::Assign(_, Rvalue::Use(o)) => self.op_loans(st, made, 0, o).holds,
            StatementKind::Assign(_, Rvalue::Aggregate(_, ops) | Rvalue::Closure(_, _, ops)) => {
                let mut h = BitSet::new(self.nloans);
                for (oi, o) in ops.iter().enumerate() {
                    h.union(&self.op_loans(st, made, oi, o).holds);
                }
                h
            }
            _ => BitSet::new(self.nloans),
        };
        if stored.is_empty() || d.proj.is_empty() || !types.place_ty(f, d).is_some_and(|t| types.may_hold(&t))
        {
            return;
        }
        self.store_into(f, types, b, si, s, st, d, &stored, out);
    }

    /// Loans `stored` written into place `d` (a part of a local, or what
    /// a reference points to): reports a store into memory of the caller
    /// other than the `mut` parameter's own loans, or into a value that
    /// cannot hold a loan. Returns whether it reported.
    #[allow(clippy::too_many_arguments)]
    fn store_into(
        &self,
        f: &Function,
        types: &Types,
        b: usize,
        si: usize,
        s: &Statement,
        st: &State,
        d: &Place,
        stored: &BitSet,
        out: &mut Vec<Finding>,
    ) -> bool {
        if let Some(k) = d.proj.iter().position(|x| matches!(x, Proj::Deref)) {
            let r = d.local.0 as usize;
            let targets: Vec<LoanId> = match &st.direct[r] {
                Some(dset) if k == 0 => dset.iter().collect(),
                _ => st.holds[r].iter().collect(),
            };
            let Some(i) = self.caller_param(f, st, targets) else { return false };
            // Only what the caller lent through this parameter may go back,
            // and what it handed over by value.
            let foreign = stored.iter().find(
                |x| !matches!(self.loans[*x].kind, LoanKind::Param(j) if j == i || lent_by_value(f, j)),
            );
            let Some(foreign) = foreign else { return false };
            out.push(
                Finding::new(msg::BORROW_STORE_INTO_CALLER, f, s.loc)
                    .with_arg("name", self.loan_name(f, types, foreign))
                    .with("param", names::local_name(f, LocalId(i + 1)))
                    .with("function", display_fn(&f.name))
                    .label(
                        self.loans[foreign].loc,
                        self.loan_label(f, foreign).0,
                        self.loan_label(f, foreign).1,
                    )
                    .at(b, si),
            );
            return true;
        }
        if !types.may_hold(&f.locals[d.local.0 as usize].ty)
            && let Some(l) = stored.iter().next()
        {
            out.push(Self::declare_view(
                types,
                &sisu_sir::write::ty(&f.locals[d.local.0 as usize].ty),
                Finding::new(msg::BORROW_STORE_IN_PLAIN_TYPE, f, s.loc)
                    .with("ty", sisu_sir::write::ty(&f.locals[d.local.0 as usize].ty))
                    .with_arg("name", self.loan_name(f, types, l))
                    .label(self.loans[l].loc, self.loan_label(f, l).0, self.loan_label(f, l).1)
                    .at(b, si),
            ));
            return true;
        }
        false
    }

    /// The fix for a borrow stored in a plain struct: make it a `view
    /// struct` (an enum has no view form, so it gets no fix).
    fn declare_view(types: &Types, ty: &str, fd: Finding) -> Finding {
        match types.decl(ty) {
            Some(sisu_sir::TypeDecl::Struct { .. }) => {
                fd.fix(msg::BORROW_DECLARE_VIEW, vec![("ty", Arg::Text(ty.to_string()))], None)
            }
            _ => fd,
        }
    }

    fn check_return(
        &self,
        f: &Function,
        types: &Types,
        b: usize,
        loc: Option<Loc>,
        st: &State,
        out: &mut Vec<Finding>,
    ) {
        for l in st.holds[0].iter() {
            let loan = &self.loans[l];
            match &loan.kind {
                LoanKind::Param(i) => {
                    if !f.from.contains(i) && !f.inner.contains(i) {
                        let param = names::local_name(f, LocalId(i + 1));
                        let decl = f.locals.get(*i as usize + 1).and_then(|d| d.loc);
                        out.push(
                            Finding::new(msg::BORROW_RETURN_NOT_FROM, f, loc)
                                .with("param", param.clone())
                                .with("function", display_fn(&f.name))
                                .label(
                                    decl,
                                    msg::BORROW_DECLARED_HERE,
                                    vec![("name", Arg::Text(param.clone()))],
                                )
                                .fix(msg::BORROW_ADD_FROM, vec![("param", Arg::Text(param))], None)
                                .at(b, f.blocks[b].stmts.len()),
                        );
                    }
                }
                LoanKind::Local { place, .. } => {
                    if place.proj.iter().any(|x| matches!(x, Proj::Deref)) {
                        continue;
                    }
                    let note = if matches!(f.locals[0].ty, Type::Fn(..)) {
                        msg::BORROW_USE_OWN_CLOSURE
                    } else {
                        msg::BORROW_RETURN_OWNED
                    };
                    out.push(
                        Finding::new(msg::BORROW_RETURN_LOCAL, f, loc)
                            .with_arg("name", names::place_name(types, f, &shorten(place)))
                            .with("function", display_fn(&f.name))
                            .label(loan.loc, msg::BORROW_BORROWED_HERE, Vec::new())
                            .note(note, Vec::new())
                            .at(b, f.blocks[b].stmts.len()),
                    );
                }
            }
        }
    }

    /// What a loan borrows, in source terms.
    fn loan_name(&self, f: &Function, types: &Types, l: LoanId) -> Arg {
        match &self.loans[l].kind {
            LoanKind::Local { place, .. } => names::place_name(types, f, &shorten(place)),
            LoanKind::Param(i) => Arg::Text(names::local_name(f, LocalId(i + 1))),
        }
    }

    // ---------------------------------------------------------------- explain

    /// For `sisu explain-borrow`: the loans live before each statement at
    /// a line, with the named locals holding them.
    pub fn live_at_line(&self, p: &Program, f: &Function, types: &Types, line: u32) -> Vec<LoanReport> {
        let mut found: Vec<LoanId> = Vec::new();
        for b in 0..f.blocks.len() {
            if f.blocks[b].cleanup {
                continue;
            }
            let Some(states) = self.states(p, f, types, b) else { continue };
            let lout = self.live_out(f, b);
            let lives = self.live_before_each(f, types, b, &lout);
            for (si, s) in f.blocks[b].stmts.iter().enumerate() {
                if s.loc.map(|l| l.line) != Some(line) {
                    continue;
                }
                for l in self
                    .live_loans(&states[si], &lives[si])
                    .iter()
                    .chain(self.live_loans(&states[si + 1], &lives[si + 1]).iter())
                {
                    if !found.contains(&l) && matches!(self.loans[l].kind, LoanKind::Local { .. }) {
                        found.push(l);
                    }
                }
            }
        }
        found.sort_by_key(|l| (self.loans[*l].loc, *l));
        let mut out = Vec::new();
        for l in found {
            // Holders and the last line where the loan is live.
            let mut holders: Vec<String> = Vec::new();
            let mut until: Option<Loc> = None;
            for b in 0..f.blocks.len() {
                if f.blocks[b].cleanup {
                    continue;
                }
                let Some(states) = self.states(p, f, types, b) else { continue };
                let lout = self.live_out(f, b);
                let lives = self.live_before_each(f, types, b, &lout);
                for (si, s) in f.blocks[b].stmts.iter().enumerate() {
                    let st = &states[si];
                    let on_line = s.loc.is_some_and(|loc| loc.line == line);
                    // The holders are the names holding it on the line,
                    // before or after one of its statements.
                    if on_line {
                        for (st2, live2) in [(st, &lives[si]), (&states[si + 1], &lives[si + 1])] {
                            for v in live2.iter() {
                                if st2.holds[v].contains(l) && names::is_named(f, LocalId(v as u32)) {
                                    let n = names::local_name(f, LocalId(v as u32));
                                    if !holders.contains(&n) {
                                        holders.push(n);
                                    }
                                }
                            }
                        }
                    }
                    // The last position where it is live.
                    if lives[si].iter().any(|v| st.holds[v].contains(l))
                        && let Some(loc) = s.loc
                        && until.is_none_or(|u| (loc.line, loc.col) > (u.line, u.col))
                    {
                        until = Some(loc);
                    }
                }
            }
            holders.sort();
            let LoanKind::Local { place, mutable, .. } = &self.loans[l].kind else { continue };
            // A borrow of a temporary that only temporaries hold is the
            // machinery of one expression (a formatting buffer), not a
            // borrow the programmer wrote.
            if !names::is_named(f, place.local) && holders.is_empty() {
                continue;
            }
            out.push(LoanReport {
                place: names::place_name(types, f, &shorten(place)),
                mutable: *mutable,
                start: self.loans[l].loc,
                until,
                holders,
            });
        }
        out
    }
}

/// A loan live at a line, for `sisu explain-borrow`.
#[derive(Clone, Debug)]
pub struct LoanReport {
    pub place: Arg,
    pub mutable: bool,
    pub start: Option<Loc>,
    pub until: Option<Loc>,
    pub holders: Vec<String>,
}

/// The longest common prefix of two places of one local.
fn common_prefix(a: &Place, b: &Place) -> Place {
    let n = a.proj.iter().zip(&b.proj).take_while(|(x, y)| x == y).count();
    Place { local: a.local, proj: a.proj[..n].to_vec() }
}

/// A place without the projections that do not change what the
/// programmer calls it: a trailing dereference of a reference local.
fn shorten(p: &Place) -> Place {
    let mut q = p.clone();
    while matches!(q.proj.last(), Some(Proj::Deref)) {
        q.proj.pop();
    }
    q
}

/// A SIR function name as the programmer knows it: `f`, or the function a
/// closure is written in.
pub fn display_fn(name: &str) -> String {
    let owner = name.split('#').next().unwrap_or(name);
    owner.rsplit('.').next().unwrap_or(owner).to_string()
}

/// The arguments of an intrinsic call it may keep: those it takes with
/// `own`, or every argument of an intrinsic SIR does not declare.
/// Whether a function's parameter `j` is taken with `own`: a caller
/// assumes what it holds may be kept by the call's `mut` arguments (a
/// borrowed one, even a view passed by value, is lent for the call only).
fn lent_by_value(f: &Function, j: u32) -> bool {
    f.own.contains(&j)
}

fn kept_args(p: &Program, name: &str, n: usize) -> Vec<usize> {
    match p.intrinsic(name) {
        Some(d) => d.own.iter().map(|i| *i as usize).collect(),
        None => (0..n).collect(),
    }
}
