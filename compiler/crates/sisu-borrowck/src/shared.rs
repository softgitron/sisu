//! Escape analysis for `Shared` (Q-2; 18, Phase 6: "the `Shared` escape
//! analysis that tags each `Shared(...)` construction site task-local or
//! may-cross"). A `Shared` counts its owners atomically only when one of
//! its owners can reach another task; each construction site (a call of
//! the intrinsic `Shared.new`) is tagged by whether its value may.
//!
//! A program that spawns no task, sends on no channel and makes no
//! generator has only task-local sites. Otherwise, per site, the analysis
//! follows the locals that may hold the value, a copy of it, a reference
//! to it or something holding one (assignments, borrows, aggregates,
//! `.copy()`, closures capturing them, call results), flow-insensitively,
//! and the site may cross when one of them reaches: a `spawn`, a `send`
//! or a generator; a scope's body; a store through a reference; the
//! function's result; or a call of a function that may itself let what it
//! is given cross (one that, directly or through the functions it calls,
//! spawns, sends or makes a generator, or calls a function value or
//! through a trait). Intrinsics other than those keep nothing beyond what
//! their `own` arguments put into their `mut` arguments and what their
//! result holds.
//!
//! A may-cross site is written `call intrinsic Shared.new_atomic`. The
//! dynamic monitor checks the tags: a cell made by a task-local site that
//! another task reaches is a soundness error in this analysis.

use sisu_sir::{Callee, Function, LocalId, Operand, Place, Program, Rvalue, StatementKind};
use std::collections::HashSet;

/// How many construction sites each tag got.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SharedStats {
    pub local: u32,
    pub crossing: u32,
}

/// The intrinsics through which a value reaches another task.
fn crossing_intrinsic(name: &str) -> bool {
    matches!(
        name,
        "Scope.spawn"
            | "Scope.spawn_with"
            | "Sender.send"
            | "Sender.try_send"
            | "gen.new"
            | "task.scope"
            | "task.race"
            | "task.shield"
            // A capability handle may reach every task.
            | "cap.new"
            | "Cap.new"
    )
}

fn root(o: &Operand) -> Option<LocalId> {
    match o {
        Operand::Copy(p) | Operand::Move(p) => Some(p.local),
        Operand::Const(_) => None,
    }
}

/// Tags every `Shared.new` call site of the program; returns the counts.
pub fn tag_sites(p: &mut Program) -> SharedStats {
    let any_crossing = p.functions.iter().any(|f| {
        f.blocks.iter().flat_map(|b| &b.stmts).any(|s| {
            matches!(&s.kind, StatementKind::Call { callee: Callee::Intrinsic(n, _), .. }
                if matches!(n.as_str(), "Scope.spawn" | "Scope.spawn_with" | "Sender.send" | "Sender.try_send" | "gen.new"))
        })
    });
    let crossing_fns = if any_crossing { crossing_functions(p) } else { HashSet::new() };
    let mut stats = SharedStats::default();
    let mut tags: Vec<(usize, usize, usize)> = Vec::new();
    for (fi, f) in p.functions.iter().enumerate() {
        for (b, block) in f.blocks.iter().enumerate() {
            for (si, s) in block.stmts.iter().enumerate() {
                let StatementKind::Call { dest, callee: Callee::Intrinsic(n, _), .. } = &s.kind else {
                    continue;
                };
                if n != "Shared.new" {
                    continue;
                }
                if any_crossing && may_cross(p, f, dest, &crossing_fns) {
                    stats.crossing += 1;
                    tags.push((fi, b, si));
                } else {
                    stats.local += 1;
                }
            }
        }
    }
    for (fi, b, si) in tags {
        if let StatementKind::Call { callee: Callee::Intrinsic(n, _), .. } =
            &mut p.functions[fi].blocks[b].stmts[si].kind
        {
            *n = "Shared.new_atomic".into();
        }
    }
    if stats.crossing > 0
        && !p.intrinsics.iter().any(|i| i.name == "Shared.new_atomic")
        && let Some(mut d) = p.intrinsics.iter().find(|i| i.name == "Shared.new").cloned()
    {
        d.name = "Shared.new_atomic".into();
        p.intrinsics.push(d);
        p.intrinsics.sort_by(|a, b| a.name.cmp(&b.name));
    }
    stats
}

/// The functions that may let what they are given reach another task.
fn crossing_functions(p: &Program) -> HashSet<String> {
    let mut out: HashSet<String> = HashSet::new();
    for f in &p.functions {
        let direct = f.blocks.iter().flat_map(|b| &b.stmts).any(|s| match &s.kind {
            StatementKind::Call { callee: Callee::Intrinsic(n, _), .. } => crossing_intrinsic(n),
            // An unknown callee may do anything.
            StatementKind::Call { callee: Callee::Value(_) | Callee::Trait(..), .. } => true,
            _ => false,
        });
        if direct {
            out.insert(f.name.clone());
        }
    }
    loop {
        let mut changed = false;
        for f in &p.functions {
            if out.contains(&f.name) {
                continue;
            }
            let calls = f.blocks.iter().flat_map(|b| &b.stmts).any(|s| {
                matches!(&s.kind, StatementKind::Call { callee: Callee::Fn(n, _), .. } if out.contains(n))
                    || closure_made(s).is_some_and(|n| out.contains(n))
            });
            if calls {
                out.insert(f.name.clone());
                changed = true;
            }
        }
        if !changed {
            return out;
        }
    }
}

/// The function a statement makes a closure of.
fn closure_made(s: &sisu_sir::Statement) -> Option<&String> {
    match &s.kind {
        StatementKind::Assign(_, Rvalue::Closure(n, _, _)) => Some(n),
        _ => None,
    }
}

/// Whether the value made into `dest` may reach another task.
fn may_cross(p: &Program, f: &Function, dest: &Place, crossing: &HashSet<String>) -> bool {
    let mut carriers: HashSet<u32> = HashSet::new();
    carriers.insert(dest.local.0);
    loop {
        let mut changed = false;
        let mut add = |l: LocalId, carriers: &mut HashSet<u32>| {
            if carriers.insert(l.0) {
                changed = true;
            }
        };
        for s in f.blocks.iter().flat_map(|b| &b.stmts) {
            let held = |o: &Operand, c: &HashSet<u32>| root(o).is_some_and(|l| c.contains(&l.0));
            match &s.kind {
                StatementKind::Assign(d, rv) => {
                    let carries = match rv {
                        Rvalue::Use(o) => held(o, &carriers),
                        Rvalue::Ref(pl, _) | Rvalue::CopyOf(pl) | Rvalue::StrView(pl) => {
                            carriers.contains(&pl.local.0)
                        }
                        Rvalue::Aggregate(_, ops) | Rvalue::Closure(_, _, ops) => {
                            ops.iter().any(|o| held(o, &carriers))
                        }
                        _ => false,
                    };
                    if carries {
                        // A store through a reference reaches memory the
                        // analysis does not follow.
                        if d.proj.iter().any(|x| matches!(x, sisu_sir::Proj::Deref)) {
                            return true;
                        }
                        add(d.local, &mut carriers);
                    }
                }
                StatementKind::Call { dest, callee, args } => {
                    let passed = args.iter().any(|o| held(o, &carriers))
                        || matches!(callee, Callee::Value(o) if held(o, &carriers));
                    if !passed {
                        continue;
                    }
                    match callee {
                        Callee::Intrinsic(n, _) if crossing_intrinsic(n) => return true,
                        Callee::Intrinsic(n, _) => {
                            // What it keeps goes into its `mut` arguments,
                            // and its result may hold what it was given.
                            if let Some(decl) = p.intrinsic(n)
                                && !decl.own.is_empty()
                            {
                                for o in args {
                                    if let Some(l) = root(o) {
                                        add(l, &mut carriers);
                                    }
                                }
                            }
                        }
                        Callee::Fn(n, _) if crossing.contains(n) => return true,
                        Callee::Fn(_, _) => {}
                        Callee::Trait(..) | Callee::Value(_) => return true,
                    }
                    add(dest.local, &mut carriers);
                }
                _ => {}
            }
        }
        if carriers.contains(&0) {
            // The function returns it: the caller may let it cross.
            return true;
        }
        if !changed {
            return false;
        }
    }
}
