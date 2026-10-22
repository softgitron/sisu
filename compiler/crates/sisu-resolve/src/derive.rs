//! `@derive` (08, "Derive"; 18, Phase 4: the built-in derives). A trait
//! derives by its `comptime def derive`, which the compiler runs for the
//! type (`template.rs`): the prelude's `Debug`, `Eq`, `PartialOrd`, `Ord`
//! and `Hash` have one, and so may any trait a program declares. The
//! derived impl is Sisu source, parsed and declared in the module of the
//! type, so it is checked and lowered like any other impl; the derives of
//! the language's traits compute what the interpreter's structural fast
//! paths do (docs/architecture.md, "Traits and generics").
//!
//! Every struct and enum without an explicit `Debug` gets the derived one
//! (11, "Every type has `Debug`"). `@derive(Ord)` derives `PartialOrd`
//! too, from the same rule. `Copy` is a flag on the type, not an impl.
//! Whether each field implements the derived trait is checked once every
//! impl is known, and reported at the attribute.

use crate::collect::Resolver;
use crate::program::*;
use crate::template::{Body, Target};
use crate::ty::Ty;
use sisu_diag::{Diagnostic, Span, TextRange, msg};
use sisu_syntax::ast::{self, ExprKind, ItemKind};

/// The language's derivable traits, besides `Copy`: their derives (in the
/// prelude) compute what the structural fast paths compute, and need the
/// trait of every field. Any other trait with a `comptime def derive` can
/// be derived too.
pub const DERIVABLE: &[&str] = &["Debug", "Eq", "PartialOrd", "Ord", "Hash"];

/// The traits `@delegate` forwards (03, "Coherence": "cheap newtypes and
/// delegation"); the first six return `Self`.
pub const DELEGABLE: &[&str] =
    &["Add", "Sub", "Mul", "Div", "Rem", "Neg", "Eq", "PartialOrd", "Ord", "Hash", "Display", "Debug", "Len"];

/// A derived impl to check once every impl is known.
#[derive(Clone, Debug)]
pub struct DeriveCheck {
    pub ty: DefId,
    pub trait_name: String,
    pub span: Span,
    /// The field a delegated impl forwards to; `None` for a derived impl,
    /// which uses every field.
    pub field: Option<String>,
}

/// The shape of a type, as the derive templates need it.
struct Shape {
    name: String,
    /// Generic parameter names and the source text of their bounds.
    generics: Vec<(String, Vec<String>)>,
    /// Whether each parameter appears in a field's type: one that does not
    /// (`Key[T]`'s `T`) needs no bound for the derived impl.
    used: Vec<bool>,
    body: Body,
}

/// What declaring a module's derived impls does, in the order it does it:
/// a [`Plan`]'s steps.
enum Act {
    Report(Diagnostic),
    /// A derived or delegated impl to check once every impl is known: the
    /// type, the trait, where it was asked for, and the field delegated to.
    Check(String, String, Span, Option<String>),
    /// The text of a delegated impl.
    Delegate(String),
    /// The impl the plan's job `job` unfolds, of the trait for the type,
    /// asked for at `span`.
    Derive { trait_name: String, ty: String, span: Span, job: usize },
}

/// One run of a trait's derive template for a type.
struct Job {
    template: std::rc::Rc<ast::FnDef>,
    /// The module the template is written in, and its source.
    module: ModuleId,
    source: std::rc::Rc<str>,
    head: String,
    name: String,
    written: String,
    body: Body,
}

/// A module's derived and delegated impls, planned without declaring
/// anything: what the templates are run for, then what is done with their
/// texts. Planning reads only the module's items and the templates, which
/// every module's items are declared with, so the plans of all the
/// program's modules are made first and their templates run on threads
/// (decision 228).
pub(crate) struct Plan {
    module: ModuleId,
    acts: Vec<Act>,
    jobs: Vec<Job>,
}

/// The texts each job's template unfolds to, or why it failed.
pub(crate) type Unfolded = Vec<Result<String, crate::template::Failure>>;

impl Plan {
    /// The text of the delegated impls and of the derived ones, given the
    /// plan's unfolded templates.
    fn texts(&self, unfolded: &[Result<String, crate::template::Failure>]) -> (String, String) {
        let mut delegate = String::new();
        let mut text = String::new();
        for act in &self.acts {
            match act {
                Act::Delegate(t) => delegate.push_str(t),
                Act::Derive { job, .. } => {
                    if let Some(Ok(t)) = unfolded.get(*job) {
                        text.push_str(t);
                    }
                }
                Act::Report(_) | Act::Check(..) => {}
            }
        }
        (delegate, text)
    }
}

/// Runs the jobs of the plans, on as many threads as the machine runs at
/// once (at most 8) when there are enough of them; each plan's results in
/// the order of its jobs. A template is a pure function of the type's
/// shape, so the texts are the same however the threads ran.
pub(crate) fn unfold_plans(plans: &[Plan]) -> Vec<Unfolded> {
    // The reference-counted templates stay on this thread; the threads
    // get plain references.
    let work: Vec<(&ast::FnDef, &str, &str, Target<'_>)> = plans
        .iter()
        .flat_map(|p| &p.jobs)
        .map(|j| (&*j.template, &*j.source, j.head.as_str(), Target { name: &j.name, written: &j.written, body: &j.body }))
        .collect();
    let maxj = std::sync::atomic::AtomicU64::new(0);
    let run = |(template, source, head, target): &(&ast::FnDef, &str, &str, Target<'_>)| {
        let t0 = std::time::Instant::now();
        let r = crate::template::unfold(template, source, head, target);
        maxj.fetch_max(t0.elapsed().as_nanos() as u64, std::sync::atomic::Ordering::Relaxed);
        if t0.elapsed().as_millis() > 2 {
            let n = match target.body { Body::Struct(f) => f.len(), Body::Enum(v) => v.len() };
            eprintln!("    slow unfold {} {} ({} parts): {:.1} ms, {} bytes", head.trim().lines().next().unwrap_or(""), target.name, n, t0.elapsed().as_secs_f64() * 1e3, r.as_ref().map_or(0, |t| t.len()));
        }
        r
    };
    let threads = std::thread::available_parallelism().map_or(1, |n| n.get()).min(8).min(work.len() / 8);
    let mut all: Vec<Result<String, crate::template::Failure>> = if threads <= 1 {
        work.iter().map(run).collect()
    } else {
        let next = std::sync::atomic::AtomicUsize::new(0);
        let each = || {
            let mut done = Vec::new();
            loop {
                let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let Some(w) = work.get(i) else { break };
                done.push((i, run(w)));
            }
            done
        };
        let mut out: Vec<Option<Result<String, crate::template::Failure>>> =
            std::iter::repeat_with(|| None).take(work.len()).collect();
        std::thread::scope(|s| {
            let others: Vec<_> =
                (1..threads).filter_map(|_| std::thread::Builder::new().spawn_scoped(s, each).ok()).collect();
            let mut done = each();
            for w in others {
                match w.join() {
                    Ok(theirs) => done.extend(theirs),
                    // A panic in a template is the resolver's own: raised
                    // again here, as one on this thread would be.
                    Err(e) => std::panic::resume_unwind(e),
                }
            }
            for (i, r) in done {
                out[i] = Some(r);
            }
        });
        out.into_iter().map(|r| r.unwrap_or_else(|| Ok(String::new()))).collect()
    };
    eprintln!("  unfold: {} jobs, {} threads, longest {:.1} ms", work.len(), threads, maxj.load(std::sync::atomic::Ordering::Relaxed) as f64 / 1e6);
    let mut per_plan = Vec::with_capacity(plans.len());
    for p in plans.iter().rev() {
        let rest = all.len() - p.jobs.len();
        per_plan.push(all.split_off(rest));
    }
    per_plan.reverse();
    per_plan
}

impl Resolver<'_> {
    /// Declares the derived impls of the structs and enums among `items`.
    pub fn declare_derived(&mut self, module: ModuleId, items: &[ast::Item]) {
        let plans = vec![self.plan_derived(module, items)];
        let unfolded = unfold_plans(&plans);
        for (plan, u) in plans.into_iter().zip(unfolded) {
            self.apply_derived(plan, &u);
        }
    }

    /// The texts a plan declares, parsed ahead (on threads, when there are
    /// many), so that declaring them finds them parsed.
    pub(crate) fn parse_ahead(plans: &[Plan], unfolded: &[Unfolded]) {
        let texts: Vec<String> = plans
            .iter()
            .zip(unfolded)
            .flat_map(|(p, u)| {
                let (delegate, text) = p.texts(u);
                [delegate, text]
            })
            .filter(|t| !t.is_empty())
            .collect();
        let threads = std::thread::available_parallelism().map_or(1, |n| n.get()).min(8).min(texts.len());
        if threads <= 1 {
            return;
        }
        let next = std::sync::atomic::AtomicUsize::new(0);
        let each = || {
            loop {
                let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let Some(t) = texts.get(i) else { break };
                let t0 = std::time::Instant::now();
                let _ = crate::collect::parse_generated(t);
                eprintln!("    parse {} bytes {:.1} ms", t.len(), t0.elapsed().as_secs_f64() * 1e3);
            }
        };
        std::thread::scope(|s| {
            let others: Vec<_> =
                (1..threads).filter_map(|_| std::thread::Builder::new().spawn_scoped(s, each).ok()).collect();
            each();
            for w in others {
                if let Err(e) = w.join() {
                    std::panic::resume_unwind(e);
                }
            }
        });
    }

    /// Plans the derived impls of the structs and enums among `items`.
    pub(crate) fn plan_derived(&self, module: ModuleId, items: &[ast::Item]) -> Plan {
        let file = self.file(module);
        let source = self.sources.get(file).map(|f| f.text.as_str()).unwrap_or_default();
        let mut acts: Vec<Act> = Vec::new();
        let mut jobs: Vec<Job> = Vec::new();
        for item in items {
            let (shape, builtin) = match &item.kind {
                ItemKind::Struct(s) => (
                    Shape {
                        name: s.name.text.clone(),
                        generics: generics_text(&s.generics, source),
                        used: used_params(&s.generics, s.fields.iter().map(|f| &f.ty)),
                        body: Body::Struct(s.fields.iter().map(|f| f.name.text.clone()).collect()),
                    },
                    item.attrs.iter().any(|a| a.name == "builtin"),
                ),
                ItemKind::Enum(e) => (
                    Shape {
                        name: e.name.text.clone(),
                        generics: generics_text(&e.generics, source),
                        used: used_params(
                            &e.generics,
                            e.variants.iter().flat_map(|v| v.fields.iter().map(|f| &f.ty)),
                        ),
                        body: Body::Enum(
                            e.variants
                                .iter()
                                .map(|v| {
                                    (
                                        v.name.text.clone(),
                                        v.fields
                                            .iter()
                                            .map(|f| f.name.as_ref().map(|n| n.text.clone()))
                                            .collect(),
                                    )
                                })
                                .collect(),
                        ),
                    },
                    false,
                ),
                _ => continue,
            };
            // A type with a `const` parameter (E0900) gets no impls.
            let has_const = match &item.kind {
                ItemKind::Struct(s) => s.generics.iter().any(|g| g.is_const),
                ItemKind::Enum(e) => e.generics.iter().any(|g| g.is_const),
                _ => false,
            };
            if shape.name.is_empty() || has_const {
                continue;
            }
            let mut wanted: Vec<(String, Span)> = Vec::new();
            for a in item.attrs.iter().filter(|a| a.name == "derive") {
                for arg in &a.args {
                    let span = self.span(module, arg.range);
                    match &arg.value.kind {
                        // A flag on the type, checked against its fields.
                        ExprKind::Name(n) if n == "Copy" => {
                            acts.push(Act::Check(shape.name.clone(), "Copy".into(), span, None));
                        }
                        ExprKind::Name(n)
                            if DERIVABLE.contains(&n.as_str())
                                || self.derive_template(module, n).is_some() =>
                        {
                            if !wanted.iter().any(|(w, _)| w == n) {
                                wanted.push((n.clone(), span));
                            }
                        }
                        _ => {
                            let name = self.source_of(span);
                            acts.push(Act::Report(
                                Diagnostic::new(msg::RESOLVE_NOT_DERIVABLE, span).with("name", name),
                            ));
                        }
                    }
                }
            }
            // `@delegate(Trait, ..., to="field")`: impls that forward to a field.
            let delegated = self.delegations(module, item, &shape, &mut acts);
            for (t, field, span) in &delegated {
                acts.push(Act::Delegate(delegate_text(&shape, t, field)));
                acts.push(Act::Check(shape.name.clone(), t.clone(), *span, Some(field.clone())));
            }
            // `Ord` brings `PartialOrd`, which it determines.
            if let Some((_, span)) = wanted.iter().find(|(w, _)| w == "Ord").cloned()
                && !wanted.iter().any(|(w, _)| w == "PartialOrd")
                && !has_impl(items, "PartialOrd", &shape.name)
            {
                wanted.push(("PartialOrd".into(), span));
            }
            // Every type has `Debug` (11): the derived one unless it has its own.
            if !builtin
                && !wanted.iter().any(|(w, _)| w == "Debug")
                && !delegated.iter().any(|(w, _, _)| w == "Debug")
                && !has_impl(items, "Debug", &shape.name)
            {
                let span = self.span(module, TextRange::empty(item.range.start));
                wanted.push(("Debug".into(), span));
            }
            for (t, span) in wanted {
                // The trait's own derive, run at compile time; every trait
                // `@derive` knows has one in the prelude.
                let Some((tm, template)) = self.derive_template(module, &t) else { continue };
                let (head, written) = header(&shape, &t);
                let source = self.template_sources.get(&tm).cloned().unwrap_or_else(|| std::rc::Rc::from(""));
                acts.push(Act::Derive { trait_name: t, ty: shape.name.clone(), span, job: jobs.len() });
                jobs.push(Job {
                    template,
                    module: tm,
                    source,
                    head,
                    name: shape.name.clone(),
                    written,
                    body: shape.body.clone(),
                });
            }
        }
        Plan { module, acts, jobs }
    }

    /// Does what a plan says, with the texts its templates unfolded to.
    pub(crate) fn apply_derived(&mut self, plan: Plan, unfolded: &[Result<String, crate::template::Failure>]) {
        let (delegate, text) = plan.texts(unfolded);
        let module = plan.module;
        let mut checks: Vec<(String, String, Span, Option<String>)> = Vec::new();
        for act in plan.acts {
            match act {
                Act::Report(d) => self.report(d),
                Act::Check(ty, t, span, field) => checks.push((ty, t, span, field)),
                Act::Delegate(_) => {}
                Act::Derive { trait_name, ty, span, job } => match unfolded.get(job) {
                    Some(Ok(_)) => checks.push((ty, trait_name, span, None)),
                    Some(Err((range, reason))) => {
                        let tm = plan.jobs.get(job).map_or(module, |j| j.module);
                        let at = self.span(tm, *range);
                        self.report(
                            Diagnostic::new(msg::COMPTIME_DERIVE_FAILED, span)
                                .with("trait", trait_name)
                                .with("ty", ty)
                                .with("reason", reason.clone())
                                .label(at, msg::COMPTIME_DERIVE_HERE, Vec::new()),
                        );
                    }
                    None => {}
                },
            }
        }
        // Delegated impls are ordinary impls, written by the compiler.
        if !delegate.is_empty() {
            self.declare_generated(module, "<delegate>", &delegate);
        }
        if !text.is_empty() {
            self.declare_generated(module, "<derive>", &text);
        }
        for (ty, trait_name, span, field) in checks {
            if let Some(d) = self.prog.lookup_in(module, &ty) {
                self.derive_checks.push(DeriveCheck { ty: d, trait_name, span, field });
            }
        }
    }

    /// The `comptime def derive` of the trait `name` as `module` sees it:
    /// its own trait of that name, or the prelude's; with the module the
    /// template is written in.
    fn derive_template(&self, module: ModuleId, name: &str) -> Option<(ModuleId, std::rc::Rc<ast::FnDef>)> {
        [module, self.prog.prelude]
            .into_iter()
            .find_map(|m| self.derive_templates.get(&(m, name.to_string())).map(|f| (m, f.clone())))
    }

    /// Declares generated impls in a module of their own, with the
    /// generated text as its source file and `module` as its scope.
    fn declare_generated(&mut self, module: ModuleId, dir: &str, text: &str) {
        let _ = self.declare_generated_in(module, dir, text);
    }

    /// [`Resolver::declare_generated`], giving the new module.
    pub(crate) fn declare_generated_in(
        &mut self,
        module: ModuleId,
        dir: &str,
        text: &str,
    ) -> Option<ModuleId> {
        let parent = &self.prog.modules[module.0 as usize];
        let name = format!("{dir}/{}.sisu", parent.name);
        let is_std = parent.is_std;
        let package = parent.package;
        // Code made from the standard library's templates is the standard
        // library's: its file is under `<std>/`, which a run's traces leave
        // out.
        let file_name = if is_std { format!("<std>/{name}") } else { name.clone() };
        let file = self.sources.add(file_name, text);
        let (ast, diags) = crate::collect::parse_generated(text);
        self.diags.extend(diags.iter().cloned().map(|mut d| {
            d.spans_mut().for_each(|s| s.file = file);
            d
        }));
        let id = ModuleId(self.prog.modules.len() as u32);
        self.prog.modules.push(ModuleData {
            name,
            file,
            is_std,
            parent: Some(module),
            package,
            ..ModuleData::default()
        });
        self.declare_items(id, &ast.items, "");
        Some(id)
    }

    /// The traits a struct's `@delegate` forwards, with the field, after
    /// checking the attribute.
    fn delegations(
        &self,
        module: ModuleId,
        item: &ast::Item,
        shape: &Shape,
        acts: &mut Vec<Act>,
    ) -> Vec<(String, String, Span)> {
        let mut out = Vec::new();
        for a in item.attrs.iter().filter(|a| a.name == "delegate") {
            let attr_span = self.span(module, a.range);
            let to = a.args.iter().find(|arg| arg.name.as_ref().is_some_and(|n| n.text == "to"));
            let field = to.and_then(|arg| match &arg.value.kind {
                ExprKind::Str(parts) => Some(ast::build::plain_text(parts)),
                _ => None,
            });
            let fields = match &shape.body {
                Body::Struct(fields) => fields.clone(),
                Body::Enum(_) => Vec::new(),
            };
            let Some(field) = field.filter(|f| fields.contains(f)) else {
                let span = to.map(|arg| self.span(module, arg.range)).unwrap_or(attr_span);
                acts.push(Act::Report(
                    Diagnostic::new(msg::RESOLVE_DELEGATE_TO, span).with("ty", shape.name.clone()),
                ));
                continue;
            };
            for arg in a.args.iter().filter(|arg| arg.name.is_none()) {
                let span = self.span(module, arg.range);
                match &arg.value.kind {
                    ExprKind::Name(n) if DELEGABLE.contains(&n.as_str()) => {
                        let returns_self = DELEGABLE[..6].contains(&n.as_str());
                        if returns_self && fields.len() != 1 {
                            acts.push(Act::Report(
                                Diagnostic::new(msg::RESOLVE_DELEGATE_SELF, span)
                                    .with("trait", n.clone())
                                    .with("ty", shape.name.clone())
                                    .with("field", field.clone()),
                            ));
                            continue;
                        }
                        if !out.iter().any(|(t, _, _): &(String, String, Span)| t == n) {
                            out.push((n.clone(), field.clone(), span));
                        }
                    }
                    _ => {
                        let name = self.source_of(span);
                        acts.push(Act::Report(
                            Diagnostic::new(msg::RESOLVE_NOT_DELEGABLE, span).with("name", name),
                        ));
                    }
                }
            }
        }
        out
    }

    /// Each derived impl's fields implement its trait.
    pub fn check_derives(&mut self) {
        let checks = std::mem::take(&mut self.derive_checks);
        for c in checks {
            if c.trait_name == "Copy" {
                self.check_copy_fields(&c);
                continue;
            }
            let home = self.prog.def(c.ty).module;
            let Some(trait_def) = self
                .prog
                .lookup(home, &c.trait_name)
                .or_else(|| self.prog.lookup(self.prog.prelude, &c.trait_name))
            else {
                continue;
            };
            // A derive of the language's traits needs the trait of every
            // field; another trait's derive needs what its code uses, which
            // checking the derived impl finds.
            let structural = DERIVABLE.contains(&c.trait_name.as_str()) || c.field.is_some();
            let generated = |prog: &Program, i: DefId| {
                let m = &prog.modules[prog.def(i).module.0 as usize].name;
                m.starts_with("<derive>/") || m.starts_with("<delegate>/")
            };
            let Some(imp) = self.prog.impls.iter().copied().find(|&i| {
                self.prog.impl_info(i).is_some_and(|ii| {
                    generated(&self.prog, i)
                        && ii.trait_ref.def == trait_def
                        && matches!(&ii.self_ty, Ty::Adt(d, _) if *d == c.ty)
                })
            }) else {
                continue;
            };
            let Some(info) = self.prog.impl_info(imp).cloned() else { continue };
            let env = self.prog.elaborate_env(&info.predicates);
            let fields: Vec<(String, Ty)> = match &self.prog.def(c.ty).kind {
                DefKind::Struct(s) => s.fields.iter().map(|f| (f.name.clone(), f.ty.clone())).collect(),
                DefKind::Enum(e) => e
                    .variants
                    .iter()
                    .flat_map(|v| {
                        v.fields.iter().enumerate().map(move |(i, (n, t))| {
                            (format!("{}.{}", v.name, n.clone().unwrap_or_else(|| i.to_string())), t.clone())
                        })
                    })
                    .collect(),
                _ => Vec::new(),
            };
            let fields: Vec<(String, Ty)> = match &c.field {
                Some(f) => fields.into_iter().filter(|(n, _)| n == f).collect(),
                None => fields,
            };
            let tr = TraitRef { def: trait_def, args: Vec::new() };
            // The trait's supertraits, which the type implements itself.
            let self_ty = info.self_ty.clone();
            let supers = self.prog.trait_info(trait_def).map(|i| i.supertraits.clone()).unwrap_or_default();
            for st in supers {
                if matches!(self.prog.implements(&self_ty, &st, &env), crate::solve::Answer::No) {
                    let names: Vec<String> = info.generics.iter().map(|g| g.name.clone()).collect();
                    self.report(
                        Diagnostic::new(msg::RESOLVE_SUPERTRAIT_MISSING, c.span)
                            .with("ty", self.prog.display_in(&self_ty, &names))
                            .with("trait", c.trait_name.clone())
                            .with("super", self.prog.display_trait(&st)),
                    );
                }
            }
            for (fname, fty) in fields {
                if fty.contains_error() || !structural {
                    continue;
                }
                if matches!(self.prog.implements(&fty, &tr, &env), crate::solve::Answer::No) {
                    let names: Vec<String> = info.generics.iter().map(|g| g.name.clone()).collect();
                    let id = if c.field.is_some() {
                        msg::RESOLVE_DELEGATE_FIELD
                    } else {
                        msg::RESOLVE_DERIVE_FIELD
                    };
                    self.report(
                        Diagnostic::new(id, c.span)
                            .with("ty", self.prog.def(c.ty).name.clone())
                            .with("trait", c.trait_name.clone())
                            .with("field", fname)
                            .with("fty", self.prog.display_in(&fty, &names)),
                    );
                    // The generated body would only repeat the error.
                    for (_, m) in &info.methods {
                        if let DefKind::Fn(f) = &mut self.prog.defs[m.0 as usize].kind {
                            f.body = None;
                            f.body_broken = true;
                        }
                    }
                    break;
                }
            }
        }
    }
}

impl Resolver<'_> {
    /// `@derive(Copy)` only for a type whose fields are all `Copy` (03,
    /// "Auto traits"); a field of a type parameter's type is, since the
    /// type is `Copy` only for `Copy` arguments.
    fn check_copy_fields(&mut self, c: &DeriveCheck) {
        let fields: Vec<(String, Ty)> = match &self.prog.def(c.ty).kind {
            DefKind::Struct(s) => s.fields.iter().map(|f| (f.name.clone(), f.ty.clone())).collect(),
            DefKind::Enum(e) => e
                .variants
                .iter()
                .flat_map(|v| {
                    v.fields.iter().enumerate().map(move |(i, (n, t))| {
                        (format!("{}.{}", v.name, n.clone().unwrap_or_else(|| i.to_string())), t.clone())
                    })
                })
                .collect(),
            _ => return,
        };
        let generics: Vec<String> = self.prog.type_generics(c.ty).iter().map(|g| g.name.clone()).collect();
        // Each parameter stands for a `Copy` type here.
        let as_copy: Vec<Ty> = (0..generics.len()).map(|_| Ty::Int).collect();
        for (fname, fty) in fields {
            if fty.contains_error() || self.prog.is_copy(&fty.subst(&as_copy)) {
                continue;
            }
            self.report(
                Diagnostic::new(msg::RESOLVE_DERIVE_FIELD, c.span)
                    .with("ty", self.prog.def(c.ty).name.clone())
                    .with("trait", "Copy")
                    .with("field", fname)
                    .with("fty", self.prog.display_in(&fty, &generics)),
            );
            if let DefKind::Struct(s) = &mut self.prog.defs[c.ty.0 as usize].kind {
                s.copy = false;
            } else if let DefKind::Enum(e) = &mut self.prog.defs[c.ty.0 as usize].kind {
                e.copy = false;
            }
            return;
        }
    }
}

/// Whether the module has its own `impl Trait for Type`.
fn has_impl(items: &[ast::Item], trait_name: &str, ty: &str) -> bool {
    items.iter().any(|it| match &it.kind {
        ItemKind::Impl(i) => {
            let names = |t: &Option<ast::TypeExpr>| match t.as_ref().map(|t| &t.kind) {
                Some(ast::TypeKind::Path { segments, .. }) => segments.last().map(|s| s.text.clone()),
                _ => None,
            };
            names(&i.trait_ty).as_deref() == Some(trait_name)
                && names(&Some(i.self_ty.clone())).as_deref() == Some(ty)
        }
        _ => false,
    })
}

/// Whether each generic parameter is named in one of the types.
fn used_params<'a>(gps: &[ast::GenericParam], tys: impl Iterator<Item = &'a ast::TypeExpr>) -> Vec<bool> {
    fn walk(t: &ast::TypeExpr, names: &mut Vec<String>) {
        match &t.kind {
            ast::TypeKind::Path { segments, args } => {
                if let Some(s) = segments.first() {
                    names.push(s.text.clone());
                }
                args.iter().for_each(|a| walk(a, names));
            }
            ast::TypeKind::Option(i) | ast::TypeKind::Dyn(i) => walk(i, names),
            ast::TypeKind::Ref { inner, .. } => walk(inner, names),
            ast::TypeKind::Result { ok, err } => {
                if let Some(o) = ok {
                    walk(o, names);
                }
                if let Some(e) = err {
                    walk(e, names);
                }
            }
            ast::TypeKind::Tuple(es) => es.iter().for_each(|(_, e)| walk(e, names)),
            ast::TypeKind::Fn { params, ret, .. } => {
                params.iter().for_each(|p| walk(p, names));
                walk(ret, names);
            }
            ast::TypeKind::Union(us) => us.iter().for_each(|u| walk(u, names)),
            _ => {}
        }
    }
    let mut names = Vec::new();
    for t in tys {
        walk(t, &mut names);
    }
    gps.iter().map(|g| names.contains(&g.name.text)).collect()
}

fn generics_text(gps: &[ast::GenericParam], source: &str) -> Vec<(String, Vec<String>)> {
    gps.iter()
        .map(|g| {
            let bounds = g
                .bounds
                .iter()
                .map(|b| source.get(b.range.start as usize..b.range.end as usize).unwrap_or("").to_string())
                .filter(|b| !b.is_empty())
                .collect();
            (g.name.text.clone(), bounds)
        })
        .collect()
}

/// `impl[A: Eq] Eq for P[A]:` and the self type `P[A]`.
fn header(shape: &Shape, trait_name: &str) -> (String, String) {
    let ty = if shape.generics.is_empty() {
        shape.name.clone()
    } else {
        format!(
            "{}[{}]",
            shape.name,
            shape.generics.iter().map(|(n, _)| n.clone()).collect::<Vec<_>>().join(", ")
        )
    };
    let params = if shape.generics.is_empty() {
        String::new()
    } else {
        let ps: Vec<String> = shape
            .generics
            .iter()
            .enumerate()
            .map(|(i, (n, bounds))| {
                let mut all = bounds.clone();
                if shape.used.get(i).copied().unwrap_or(true) {
                    all.push(trait_name.to_string());
                }
                if all.is_empty() { n.clone() } else { format!("{n}: {}", all.join(" + ")) }
            })
            .collect();
        format!("[{}]", ps.join(", "))
    };
    (format!("impl{params} {trait_name} for {ty}:\n"), ty)
}

/// The source of one delegated impl: the trait's method on the field, the
/// result wrapped back into the type when the trait returns `Self`.
fn delegate_text(shape: &Shape, trait_name: &str, field: &str) -> String {
    let (head, ty) = header(shape, trait_name);
    let name = &shape.name;
    let v = field;
    let op = |sym: &str, method: &str| {
        format!(
            "    pure def {method}(self, other: {ty}) -> {ty}:\n        return {name}({v}=self.{v} {sym} other.{v})\n"
        )
    };
    let body = match trait_name {
        "Add" => op("+", "add"),
        "Sub" => op("-", "sub"),
        "Mul" => op("*", "mul"),
        "Div" => op("/", "div"),
        "Rem" => op("%", "rem"),
        "Neg" => format!("    pure def neg(self) -> {ty}:\n        return {name}({v}=-self.{v})\n"),
        "Eq" => {
            format!("    pure def eq(self, other: {ty}) -> bool:\n        return self.{v} == other.{v}\n")
        }
        "PartialOrd" => format!(
            "    pure def partial_cmp(self, other: {ty}) -> Ordering?:\n        return self.{v}.partial_cmp(other.{v})\n"
        ),
        "Ord" => format!(
            "    pure def cmp(self, other: {ty}) -> Ordering:\n        return self.{v}.cmp(other.{v})\n"
        ),
        "Hash" => format!("    pure def hash(self, mut h: Hasher):\n        self.{v}.hash(mut h)\n"),
        "Display" => format!("    pure def fmt(self, mut f: Formatter):\n        self.{v}.fmt(mut f)\n"),
        "Debug" => {
            format!("    pure def fmt_debug(self, mut f: Formatter):\n        self.{v}.fmt_debug(mut f)\n")
        }
        _ => format!("    pure def len(self) -> int:\n        return len(self.{v})\n"),
    };
    format!("\n{head}{body}")
}
