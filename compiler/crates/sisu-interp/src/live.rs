//! Hot replacement (10, "Debugging and profiling"; D-053, D-068, N-35):
//! `sisu run --watch` swaps in the bodies that change on each save, in
//! epochs.
//!
//! Every call goes through the function table, the per-definition
//! indirection point of D-053: SIR names a callee, a function value and a
//! closure by the function's name (its slot), never by its body, and the
//! interpreter resolves the name in the epoch the running task has
//! adopted. An epoch is one such table. The interpreter polls its host
//! for saves at every task start and at `live.boundary()`; a save that
//! checks is compiled again from the top (the queries of the driver), and
//! the new program is compared with everything the run has, of every
//! epoch:
//!
//! - a changed signature of a function (its parameters, result,
//!   conventions, purity or kind), a changed layout of a type, or a
//!   changed impl restarts the program with a message, since an old frame
//!   or value could not run against the new code;
//! - otherwise every function whose body changed, and every new one, is
//!   published as one epoch, with the types and impls new to the run, and
//!   the running task adopts it (a task adopts the newest epoch at its
//!   start and at each `live.boundary()`, and nowhere else, so one
//!   iteration of a loop never mixes an old body with a new helper).
//!
//! A frame in flight keeps the body it started with until it returns (it
//! holds the body, by index, not the slot). A save that fails to check
//! publishes nothing and is reported. A changed `const` or `comptime`
//! result changes the bodies that use it, which are swapped with it: the
//! whole program is compiled again, so the query graph decides what
//! changed. The compiler keeps only what a program reaches, so a save may
//! leave out a function, type or impl the run has; the run keeps it, for
//! the frames and values of earlier epochs. Nothing the runtime persists
//! holds an epoch: a function value holds its name.

use crate::{Interp, Unwind, msg, render};
use sisu_sir as sir;

/// What the driver gives a run under `--watch`: the program again, from
/// the saved files.
pub trait Rebuild: Send {
    /// The program the program's files make with these new texts (each a
    /// file's name, as the program names it, and its text), or the
    /// diagnostics that stop it, rendered for a person.
    fn rebuild(&mut self, edits: &[(String, String)]) -> Result<sir::Program, String>;
}

/// The state of hot replacement in a run under `--watch`.
pub(crate) struct Live<'h> {
    pub rebuild: &'h mut dyn Rebuild,
    /// Set when a save changed what a swap cannot: the program to start
    /// again with.
    pub restart: Option<sir::Program>,
}

/// What a save adds to the run, by index in the new program: the
/// function bodies that changed or are new, and the types and impls new
/// to the run.
struct Additions {
    bodies: Vec<usize>,
    types: Vec<usize>,
    impls: Vec<usize>,
}

/// A function's signature: what its callers and its frames rely on.
type Signature<'a> =
    (&'a [String], sir::FnKind, u32, bool, &'a [u32], &'a [u32], &'a [u32], Vec<&'a sir::Type>);

fn signature(f: &sir::Function) -> Signature<'_> {
    let params: Vec<&sir::Type> = f.locals.iter().take(f.param_count as usize + 1).map(|l| &l.ty).collect();
    (&f.type_params, f.kind, f.param_count, f.pure, &f.from, &f.inner, &f.own, params)
}

/// What names one impl: its trait, the trait's arguments and the type.
fn impl_key(i: &sir::ImplDecl) -> (&str, &[sir::Type], &sir::Type, u32) {
    (&i.trait_name, &i.trait_args, &i.self_ty, i.params)
}

/// Whether a function is the program's own (not the standard library's or
/// a shim the compiler wrote).
fn is_user(p: &sir::Program, f: &sir::Function) -> bool {
    p.sources.get(f.source as usize).is_some_and(|s| !s.starts_with("<std>/") && !s.starts_with("<shims>/"))
}

impl<'p> Interp<'p, '_> {
    /// The epoch the running task has adopted.
    pub(crate) fn task_epoch(&self) -> usize {
        self.sched.tasks.get(self.sched.cur as usize).map(|t| t.epoch).unwrap_or(0)
    }

    /// The newest epoch published.
    pub(crate) fn newest_epoch(&self) -> usize {
        self.epochs.len().saturating_sub(1)
    }

    /// `live.boundary()`: a poll for saves, then the running task adopts
    /// the newest epoch.
    pub(crate) fn live_boundary(&mut self) -> Result<(), Unwind> {
        self.live_poll()?;
        let newest = self.newest_epoch();
        let cur = self.sched.cur as usize;
        if let Some(t) = self.sched.tasks.get_mut(cur) {
            t.epoch = newest;
        }
        Ok(())
    }

    /// `live.epoch()`.
    pub(crate) fn live_epoch(&self) -> u32 {
        self.task_epoch() as u32
    }

    /// Whether the run stopped for a restart (`--watch`).
    pub(crate) fn restarting(&self) -> bool {
        self.live.as_ref().is_some_and(|l| l.restart.is_some())
    }

    /// Polls the host for saves (only under `--watch`) and applies them:
    /// publishes an epoch, reports a save that does not check, or stops
    /// the run for a restart.
    pub(crate) fn live_poll(&mut self) -> Result<(), Unwind> {
        let Some(mut live) = self.live.take() else { return Ok(()) };
        let r = self.apply_edits(&mut live);
        self.live = Some(live);
        r
    }

    fn apply_edits(&mut self, live: &mut Live<'_>) -> Result<(), Unwind> {
        let edits = self.host.poll_edits();
        if edits.is_empty() {
            return Ok(());
        }
        let new = match live.rebuild.rebuild(&edits) {
            Ok(p) => p,
            Err(diagnostics) => {
                let files: Vec<String> = edits.iter().map(|(f, _)| format!("`{f}`")).collect();
                let text = render(
                    msg::RUN_LIVE_REJECTED,
                    &[
                        ("files", files.join(", ")),
                        ("epoch", self.newest_epoch().to_string()),
                        ("diagnostics", diagnostics.trim_end().to_string()),
                    ],
                );
                self.host.stderr(&format!("{text}\n"));
                return Ok(());
            }
        };
        let add = match self.compare_save(&new) {
            Ok(a) => a,
            Err(message) => {
                self.host.stderr(&format!("{message}\n"));
                live.restart = Some(new);
                return Err(Unwind::Abort(String::new()));
            }
        };
        if add.bodies.is_empty() && add.types.is_empty() && add.impls.is_empty() {
            return Ok(());
        }
        let mut shown: Vec<String> = Vec::new();
        for &i in &add.bodies {
            let f = &new.functions[i];
            let name = format!("`{}`", crate::exec::display_fn_name(&f.name, &new));
            if is_user(&new, f) && !shown.contains(&name) {
                shown.push(name);
            }
        }
        self.publish(&new, &add);
        if !shown.is_empty() {
            let text = render(
                msg::RUN_LIVE_EPOCH,
                &[("epoch", self.newest_epoch().to_string()), ("functions", shown.join(", "))],
            );
            self.host.stderr(&format!("{text}\n"));
        }
        Ok(())
    }

    /// What the program a save makes adds to the run, against every body,
    /// type and impl the run has (of any epoch), or the message of the
    /// restart it needs.
    fn compare_save(&self, new: &sir::Program) -> Result<Additions, String> {
        let mut types = Vec::new();
        for (i, t) in new.types.iter().enumerate() {
            match self.types.get(t.name()) {
                Some(old) if *old != t => {
                    return Err(render(msg::RUN_LIVE_RESTART_LAYOUT, &[("ty", t.name().to_string())]));
                }
                Some(_) => {}
                None => types.push(i),
            }
        }
        let mut impls = Vec::new();
        for (i, imp) in new.impls.iter().enumerate() {
            match self.prog.impls.iter().find(|o| impl_key(o) == impl_key(imp)) {
                Some(old) if old != imp => {
                    return Err(render(msg::RUN_LIVE_RESTART_IMPLS, &[("trait", imp.trait_name.clone())]));
                }
                Some(_) => {}
                None => impls.push(i),
            }
        }
        let table = self.epochs.last();
        let mut bodies = Vec::new();
        for (i, f) in new.functions.iter().enumerate() {
            match table.and_then(|t| t.get(f.name.as_str())).and_then(|&k| self.func(k)) {
                Some(old) if signature(old) != signature(f) => {
                    let shown = crate::exec::display_fn_name(&f.name, new);
                    return Err(render(msg::RUN_LIVE_RESTART_SIGNATURE, &[("function", shown)]));
                }
                Some(old) => {
                    // The same body, unless it moved within or to another
                    // file (its locations are part of it).
                    let file =
                        |p: &sir::Program, g: &sir::Function| p.sources.get(g.source as usize).cloned();
                    let same = old.blocks == f.blocks
                        && old.locals == f.locals
                        && old.loc == f.loc
                        && file(self.prog, old) == file(new, f);
                    if !same {
                        bodies.push(i);
                    }
                }
                None => bodies.push(i),
            }
        }
        if new.entry != self.prog.entry {
            let function = new.entry.as_ref().map(|e| e.function.clone()).unwrap_or_else(|| "main".into());
            return Err(render(msg::RUN_LIVE_RESTART_SIGNATURE, &[("function", function)]));
        }
        Ok(Additions { bodies, types, impls })
    }

    /// Publishes a save's additions as the newest epoch. The run's program
    /// becomes the one before with the new types, impls and files added;
    /// the new bodies join the function table. All of it lives as long as
    /// the run: frames of every epoch point into it.
    fn publish(&mut self, new: &sir::Program, add: &Additions) {
        let old = self.prog;
        let mut merged = sir::Program {
            sources: old.sources.clone(),
            entry: old.entry.clone(),
            exports: old.exports.clone(),
            runtime: old.runtime.clone(),
            types: old.types.clone(),
            intrinsics: old.intrinsics.clone(),
            functions: Vec::new(),
            tests: old.tests.clone(),
            test_tags: old.test_tags.clone(),
            benches: old.benches.clone(),
            trait_methods: old.trait_methods.clone(),
            impls: old.impls.clone(),
        };
        for d in &new.intrinsics {
            if !merged.intrinsics.iter().any(|o| o.name == d.name) {
                merged.intrinsics.push(d.clone());
            }
        }
        for d in &new.trait_methods {
            if !merged.trait_methods.iter().any(|o| o.name == d.name) {
                merged.trait_methods.push(d.clone());
            }
        }
        merged.types.extend(add.types.iter().map(|&i| new.types[i].clone()));
        merged.impls.extend(add.impls.iter().map(|&i| new.impls[i].clone()));
        // A new body names its file by its place in the new program's
        // list of files, which may be another in the run's.
        let mut bodies: Vec<sir::Function> = Vec::new();
        for &i in &add.bodies {
            let mut f = new.functions[i].clone();
            let file = new.sources.get(f.source as usize).cloned().unwrap_or_default();
            f.source = match merged.sources.iter().position(|s| *s == file) {
                Some(k) => k as u32,
                None => {
                    merged.sources.push(file);
                    (merged.sources.len() - 1) as u32
                }
            };
            bodies.push(f);
        }
        let merged: &'p sir::Program = Box::leak(Box::new(merged));
        let bodies: &'p [sir::Function] = Box::leak(bodies.into_boxed_slice());
        self.prog = merged;
        let added = merged.types.len() - add.types.len();
        for t in &merged.types[added..] {
            self.types.insert(t.name(), t);
            match t {
                sir::TypeDecl::Enum { name, variants: vs, drop, .. } => {
                    for (k, v) in vs.iter().enumerate() {
                        self.variants.insert((name.as_str(), v.name.as_str()), k as u32);
                    }
                    self.any_drop |= drop.is_some();
                }
                sir::TypeDecl::Struct { drop, .. } => self.any_drop |= drop.is_some(),
            }
        }
        let mut table = self.epochs.last().cloned().unwrap_or_default();
        for f in bodies {
            let k = self.funcs.len();
            self.funcs.push(f);
            table.insert(f.name.as_str(), k);
            self.any_drop |= f.locals.iter().any(|l| crate::mentions_counted(&l.ty));
            if f.kind == sir::FnKind::Glue
                && let Some(ty) = f.name.strip_suffix("#drop")
            {
                self.glue.insert(ty, k);
            }
        }
        self.epochs.push(table);
    }
}
