//! The Sisu type checker (proposal/18-next-steps.md, pipeline step 4):
//! a local, bidirectional checker over each body with explicit
//! signatures, error-set subtyping, inferred error sets and `pure`.
//!
//! It never fails: every body yields a typed tree ([`thir::Body`]), with
//! ill-typed subterms typed `Error` and nothing downstream of them
//! reported again (D-065, N-40). The tree is what lowering consumes and
//! what the language server will hover over.

mod call;
mod check;
mod contracts;
mod exhaustive;
mod expr;
mod generic_asserts;
mod infer;
mod memo;
mod pat;
pub mod purity;
pub mod spans;
mod tasks;
pub mod thir;
mod zonk;

use check::{Checker, RetCtx};
pub use memo::BodyCache;
use memo::{Lookup, Session, check_body, memo_body, memo_lookup, memo_store};
pub use pat::pattern_bindings;
use sisu_diag::{Diagnostic, SourceMap, msg};
use sisu_resolve::{Answer, DefId, DefKind, Program, Ty};
use std::collections::HashMap;
pub use thir::*;

/// The type of a parameter as a local of its function: a borrowed
/// parameter is a `ref T` unless `T` is `Copy` (04, rule 2). A borrowed
/// parameter whose type is a generic parameter, or an associated type of
/// one, is always a `ref`: whether its type argument is `Copy` is not known
/// in the body, and every caller passes it the same way.
pub fn param_local_ty(
    prog: &Program,
    ty: &Ty,
    mode: sisu_syntax::ast::Mode,
    env: &[sisu_resolve::Predicate],
) -> Ty {
    use sisu_syntax::ast::Mode;
    match mode {
        Mode::Own => ty.clone(),
        Mode::Mut => Ty::Ref(Box::new(ty.clone()), true),
        Mode::Borrow => {
            // A reference is passed as it is, a `ref mut T` too: the call
            // passes the reference, not a borrow of it. A generic
            // parameter bounded by `Copy` is passed by value, as every type
            // it stands for is.
            if matches!(ty, Ty::Ref(..))
                || (!is_opaque(ty) && prog.is_copy(ty))
                || (is_opaque(ty) && prog.is_copy_in(ty, env))
            {
                ty.clone()
            } else {
                Ty::Ref(Box::new(ty.clone()), false)
            }
        }
    }
}

/// A generic parameter or an associated type of one: a type whose values
/// the body knows only through its bounds.
pub fn is_opaque(ty: &Ty) -> bool {
    matches!(ty, Ty::Param(_) | Ty::Assoc(_))
}

fn file_text(sources: &SourceMap, file: sisu_diag::FileId) -> &str {
    sources.get(file).map(|f| f.text.as_str()).unwrap_or("")
}

/// Checks one function body: the typed body, its diagnostics, the error
/// set its `T !` absorbed, and the inferred error sets it read.
fn check_fn(
    prog: &Program,
    inferred: &HashMap<DefId, Vec<DefId>>,
    def: DefId,
    sources: &SourceMap,
) -> (Body, Vec<Diagnostic>, Vec<DefId>, Vec<DefId>) {
    let empty = Body {
        def,
        kind: BodyKind::Fn,
        params: Vec::new(),
        locals: Vec::new(),
        closures: Vec::new(),
        ret: Ty::Error,
        block: Block::empty(prog.def(def).span),
        span: prog.def(def).span,
        has_errors: false,
        generator: None,
        contracts: None,
        generic_asserts: Vec::new(),
        memo: Memo::default(),
    };
    let Some(f) = prog.fn_info(def) else { return (empty, Vec::new(), Vec::new(), Vec::new()) };
    let Some(body_ast) = &f.body else { return (empty, Vec::new(), Vec::new(), Vec::new()) };
    let mut c = Checker::new(prog, inferred, def, f.file, file_text(sources, f.file));
    c.self_ty = f.owner.clone();
    c.env = prog.elaborate_env(&f.predicates);
    c.generic_names = f.generics.iter().map(|g| g.name.clone()).collect();
    c.const_generics = f.generics.iter().map(|g| g.is_const).collect();
    // A generator: its `return` gives nothing, and calling it makes the
    // `Iter` it declares.
    let generator = if tasks::has_yield(body_ast) {
        let declared = c.fn_ret(def);
        match &declared {
            Ty::Adt(d, a) if Some(*d) == prog.wk.iter && a.len() == 1 => Some(a[0].clone()),
            _ => {
                let span = f.ret_span.unwrap_or(prog.def(def).span);
                let d = Diagnostic::new(msg::TYPE_GENERATOR_RESULT, span)
                    .with("name", prog.def(def).name.clone())
                    .with("ty", prog.display(&declared));
                c.report(d);
                // Checked as a generator of unknown items, so the one
                // mistake is reported once.
                Some(Ty::Error)
            }
        }
    } else {
        None
    };
    c.generator = generator.clone();
    let ret = if generator.is_some() {
        Ty::Unit
    } else if f.err_inferred {
        let ok = match &f.ret {
            Ty::Adt(_, a) => a.first().cloned().unwrap_or(Ty::Unit),
            _ => Ty::Unit,
        };
        let e = c.infer.fresh_absorbing();
        prog.wk.result.map(|r| Ty::Adt(r, vec![ok, e])).unwrap_or(Ty::Error)
    } else {
        c.fn_ret(def)
    };
    c.ret.push(RetCtx { ty: ret.clone(), closure: None, first_local: 0, loops: 0, captures: Vec::new() });
    let def_span = prog.def(def).span;
    let mut params = Vec::new();
    if let Some(mode) = f.self_mode {
        let owner = f.owner.clone().unwrap_or(Ty::Error);
        let ty = param_local_ty(prog, &owner, mode, &c.env);
        let l = c.new_local("self", ty, false, def_span);
        c.declare("self", l, false, def_span.range);
        params.push(l);
    }
    for p in &f.params {
        let ty = param_local_ty(prog, &p.ty, p.mode, &c.env);
        if f.pure && prog.is_capability(&p.ty) {
            let mut d = Diagnostic::new(msg::TYPE_IMPURE_CAPABILITY, p.span)
                .with("name", prog.def(def).name.clone())
                .with("ty", prog.display(&p.ty));
            // `pure ` before `def` on the header's line goes.
            let text = file_text(sources, f.file);
            let name_at = def_span.range.start as usize;
            let line_start = text.get(..name_at).and_then(|t| t.rfind('\n')).map(|i| i + 1).unwrap_or(0);
            if let Some(i) = text.get(line_start..name_at).and_then(|h| h.find("pure ")) {
                let at = (line_start + i) as u32;
                let span = sisu_diag::Span::new(f.file, sisu_diag::TextRange::new(at, at + 5));
                d = d.fix(
                    msg::TYPE_REMOVE_PURE,
                    Vec::new(),
                    vec![sisu_diag::Edit { span, text: String::new() }],
                );
            }
            c.report(d);
        }
        let l = c.new_local(&p.name, ty, false, p.span);
        c.declare(&p.name, l, false, p.span.range);
        params.push(l);
    }
    // The contract: the clauses the body starts with (07).
    let leading = contracts::leading_contracts(body_ast);
    let clauses: Vec<&sisu_syntax::ast::Contract> = body_ast.stmts[..leading]
        .iter()
        .filter_map(|s| match &s.kind {
            sisu_syntax::ast::StmtKind::Contract(k) => Some(k),
            _ => None,
        })
        .collect();
    let contracts = c.check_contracts(f, &clauses, &ret);
    let trimmed;
    let body_ast = if leading > 0 {
        trimmed = sisu_syntax::ast::Block {
            node: body_ast.node,
            range: body_ast.range,
            stmts: body_ast.stmts[leading..].to_vec(),
        };
        &trimmed
    } else {
        body_ast
    };
    let mut block = c.check_block_in_scope(body_ast);
    let r = c.infer.zonk(&ret);
    let ret = if generator.is_some() { c.fn_ret(def) } else { ret };
    // A generator's body becomes a task-like closure that owns the
    // parameters; the function returns the iterator that runs it.
    if generator.is_some() {
        let id = ClosureId(c.closures.len() as u32);
        let captures = params.iter().map(|&l| Capture { local: l, mutable: false, by_value: true }).collect();
        let fn_ty = Ty::Fn(Box::new(sisu_resolve::FnTy { params: Vec::new(), ret: Ty::Unit, pure: f.pure }));
        let span = block.span;
        c.closures.push(Some(Closure {
            params: Vec::new(),
            captures,
            own: true,
            ret: Ty::Unit,
            body: block,
            span,
            fn_ty,
            comptime: false,
            role: ClosureRole::Task,
        }));
        let value = Expr::new(ExprKind::Generator(id), ret.clone(), span);
        block = Block { stmts: vec![Stmt::Return { value: Some(value), span }], span };
    }
    if !matches!(r, Ty::Unit | Ty::Never | Ty::Error) && !f.body_broken && !c.block_diverges(&block) {
        let d = Diagnostic::new(msg::TYPE_MISSING_RETURN, def_span)
            .with("name", prog.def(def).name.clone())
            .with("ty", prog.display(&r))
            .note(msg::TYPE_MISSING_RETURN_END, Vec::new());
        c.report(d);
    }
    c.decide_captures();
    c.check_own_closures();
    check_bounds(&mut c);
    check_comptime_types(&mut c);
    let absorbed = c.infer.absorbed();
    report_uninferred(&mut c);
    let has_errors = c.diags.iter().any(|d| d.is_error());
    let closures = c
        .closures
        .drain(..)
        .map(|x| {
            x.unwrap_or(Closure {
                params: Vec::new(),
                captures: Vec::new(),
                own: false,
                ret: Ty::Error,
                body: Block::empty(def_span),
                span: def_span,
                fn_ty: Ty::Error,
                comptime: false,
                role: ClosureRole::Plain,
            })
        })
        .collect();
    let mut body = Body {
        def,
        kind: BodyKind::Fn,
        params,
        locals: std::mem::take(&mut c.locals),
        closures,
        ret,
        block,
        span: def_span,
        has_errors,
        generator,
        contracts,
        generic_asserts: std::mem::take(&mut c.generic_asserts),
        memo: Memo::default(),
    };
    zonk::Zonker { prog, infer: &c.infer }.body(&mut body);
    // A pure body's calls are checked on SIR (sisu-borrowck's purity pass).
    let diags = std::mem::take(&mut c.diags);
    let reads = c.inferred_reads.take();
    (body, diags, absorbed, reads)
}

/// The `Send` and `Sync` checks of the tasks the body spawns, once types
/// are known (06, "Send, Sync, and shared state"): what a task moves in
/// (or keeps a copy of) must be `Send`, what it shares by reference
/// `Sync`, what it changes through a reference `Send`, and what it
/// returns `Send`. The message names the part of the type in the way.
fn check_spawns(c: &mut Checker) {
    use sisu_resolve::Auto;
    let prog = c.prog;
    for (id, span) in std::mem::take(&mut c.spawn_checks) {
        let Some(Some(cl)) = c.closures.get(id.0 as usize).cloned() else { continue };
        for cap in &cl.captures {
            let ty = prog.normalize(&c.infer.zonk(&c.local_ty(cap.local)), &c.env);
            if c.infer.has_vars(&ty) || ty.contains_error() {
                continue;
            }
            let name = c.locals[cap.local.0 as usize].name.clone();
            let (auto, m) = if cap.by_value {
                (Auto::Send, msg::TASK_NOT_SEND)
            } else if cap.mutable {
                (Auto::Send, msg::TASK_NOT_SEND_MUT)
            } else {
                (Auto::Sync, msg::TASK_NOT_SYNC)
            };
            if let Err(o) = prog.auto_trait(&ty, auto, &c.env) {
                let d = obstacle_diag(c, m, span, &name, &ty, &o);
                c.report(d);
            }
        }
        let ret = prog.normalize(&c.infer.zonk(&cl.ret), &c.env);
        if !c.infer.has_vars(&ret)
            && !ret.contains_error()
            && let Err(o) = prog.auto_trait(&ret, Auto::Send, &c.env)
        {
            let d = obstacle_diag(c, msg::TASK_RESULT_NOT_SEND, span, "", &ret, &o);
            c.report(d);
        }
    }
}

/// The part of a type in the way of `Send` or `Sync`, as the note that
/// names it (`because of `name.field`, a `Cell[int]``); none when it is
/// the type itself.
fn obstacle_note(c: &Checker, d: Diagnostic, name: &str, o: &sisu_resolve::Obstacle) -> Diagnostic {
    if o.path.is_empty() {
        return d;
    }
    let mut p = name.to_string();
    for seg in &o.path {
        if seg == "[]" {
            p.push_str("[...]");
        } else {
            if !p.is_empty() {
                p.push('.');
            }
            p.push_str(seg);
        }
    }
    d.note(msg::TASK_FIELD_PATH, vec![("path", p), ("ty", c.show(&o.ty))])
}

/// A value made into a capability handle must be `Send` and `Sync`: the
/// handle is both, so every task may use what it holds (06).
fn check_capabilities(c: &mut Checker) {
    use sisu_resolve::Auto;
    let prog = c.prog;
    for (ty, cap, span) in std::mem::take(&mut c.cap_checks) {
        let ty = prog.normalize(&c.infer.zonk(&ty), &c.env);
        if c.infer.has_vars(&ty) || ty.contains_error() {
            continue;
        }
        for (auto, name) in [(Auto::Send, "Send"), (Auto::Sync, "Sync")] {
            if let Err(o) = prog.auto_trait(&ty, auto, &c.env) {
                let d = Diagnostic::new(msg::CAP_NOT_SHAREABLE, span)
                    .with("ty", c.show(&ty))
                    .with("cap", cap.clone())
                    .with("auto", name);
                let d = obstacle_note(c, d, "", &o);
                c.report(d);
                break;
            }
        }
    }
}

/// A `Send` or `Sync` diagnostic: the capture, its type, and the part of
/// the type in the way, with the wrapper that would make it shareable.
fn obstacle_diag(
    c: &Checker,
    m: sisu_diag::MsgId,
    span: sisu_diag::Span,
    name: &str,
    ty: &Ty,
    o: &sisu_resolve::Obstacle,
) -> Diagnostic {
    let prog = c.prog;
    let d = Diagnostic::new(m, span).with("name", name.to_string()).with("ty", c.show(ty));
    let mut d = obstacle_note(c, d, name, o);
    // A `Cell` shares mutation through a `Mutex` instead.
    if let Ty::Adt(cd, a) = &o.ty
        && prog.builtin(*cd) == Some("Cell")
    {
        let inner = a.first().map(|t| c.show(t)).unwrap_or_default();
        d = d.note(msg::TASK_WRAP_MUTEX, vec![("ty", inner)]);
    } else if m == msg::TASK_NOT_SYNC {
        d = d.note(msg::TASK_USE_OWN, Vec::new());
    }
    d
}

/// Checks the bounds the body's calls and constructs required, once
/// inference is done: each type must implement its trait (03, "Generic
/// syntax"; a generic body is checked against its own bounds only).
fn check_bounds(c: &mut Checker) {
    let prog = c.prog;
    check_spawns(c);
    check_capabilities(c);
    for (ty, owner, span) in std::mem::take(&mut c.owner_checks) {
        let t = prog.normalize(&c.infer.zonk(&ty), &c.env);
        if c.infer.has_vars(&t) || t.contains_error() {
            continue;
        }
        if prog.can_borrow_in(&t, &c.env) {
            let d =
                Diagnostic::new(msg::BORROW_VIEW_IN_OWNER, span).with("ty", c.show(&t)).with("owner", owner);
            c.report(d);
        } else if must_consume_ty(prog, &t, 0) {
            let d = Diagnostic::new(msg::BORROW_MUST_CONSUME_IN_OWNER, span)
                .with("ty", c.show(&t))
                .with("owner", owner);
            c.report(d);
        }
    }
    let mut seen: Vec<(Ty, sisu_resolve::TraitRef)> = Vec::new();
    let bounds = std::mem::take(&mut c.bounds).into_iter().map(|b| (b, true));
    let impls = std::mem::take(&mut c.impl_bounds).into_iter().map(|b| (b, false));
    for ((ty, bound, span), strict) in bounds.chain(impls).collect::<Vec<_>>() {
        let t = prog.normalize(&c.infer.zonk(&ty), &c.env);
        let b = sisu_resolve::TraitRef {
            def: bound.def,
            args: bound.args.iter().map(|a| prog.normalize(&c.infer.zonk(a), &c.env)).collect(),
        };
        if c.infer.has_vars(&t) || t.contains_error() || b.args.iter().any(|a| a.contains_error()) {
            continue;
        }
        if seen.contains(&(t.clone(), b.clone())) {
            continue;
        }
        let answer = if strict { prog.bound_holds(&t, &b, &c.env) } else { prog.implements(&t, &b, &c.env) };
        if answer == Answer::No {
            seen.push((t.clone(), b.clone()));
            let mut d = Diagnostic::new(msg::TYPE_BOUND_NOT_SATISFIED, span)
                .with("ty", c.show(&t))
                .with("bound", prog.display_trait_in(&b, &c.generic_names));
            // Interpolated without `Display`: its `Debug` form shows it.
            let inside = |at: usize, ch: char| c.source.get(at..at + 1) == Some(ch.encode_utf8(&mut [0; 4]));
            if Some(b.def) == prog.wk.display
                && span.file == c.file
                && span.range.start > 0
                && inside(span.range.start as usize - 1, '{')
                && inside(span.range.end as usize, '}')
            {
                let at = sisu_diag::Span::new(span.file, sisu_diag::TextRange::empty(span.range.end));
                d = d.fix(
                    msg::TYPE_SHOW_DEBUG,
                    Vec::new(),
                    vec![sisu_diag::Edit { span: at, text: ":?".into() }],
                );
            }
            // A type parameter without the bound: bound it.
            if let Ty::Param(i) = t
                && let Some((name, edit)) = c.bound_edit(i, &prog.display_trait_in(&b, &c.generic_names))
            {
                let bound = prog.display_trait_in(&b, &c.generic_names);
                d = d.fix(msg::TYPE_ADD_BOUND, vec![("param", name), ("bound", bound)], vec![edit]);
            }
            let d = c.missing_impl_fix(d, &t, &b);
            c.report(d);
        } else if Some(b.def) == prog.wk.hash && !t.has_params() {
            // The host's `dict` and `set` hash and compare keys
            // structurally: a key type with impls of its own would be found
            // by the wrong rule (port debt; reject when unsure).
            let own_impl =
                [prog.wk.hash, prog.wk.eq].into_iter().flatten().find(|tr| !prog.is_structural(&t, *tr));
            if let Some(tr) = own_impl {
                seen.push((t.clone(), b.clone()));
                let d = Diagnostic::new(msg::TYPE_STRUCTURAL_ONLY, span)
                    .with("callee", "a `dict` or `set` key")
                    .with("trait", prog.def(tr).name.clone())
                    .with("ty", c.show(&t));
                c.report(d);
            }
        }
    }
}

/// Reports locals whose type inference left open, once per body.
fn report_uninferred(c: &mut Checker) {
    if c.diags.iter().any(|d| d.is_error()) {
        return;
    }
    let mut found = None;
    for l in &c.locals {
        if l.name != "?" && !l.name.is_empty() && c.infer.has_vars(&l.ty) {
            found = Some((l.name.clone(), l.span));
            break;
        }
    }
    if let Some((name, span)) = found {
        c.report(Diagnostic::new(msg::TYPE_CANNOT_INFER, span).with("name", name));
    }
}

/// What a `comptime` expression computes cannot depend on the function's
/// type arguments, which are known only when it is called (E0600).
fn check_comptime_types(c: &mut Checker) {
    for (id, range) in std::mem::take(&mut c.comptime_sites) {
        let Some(Some(cl)) = c.closures.get(id.0 as usize) else { continue };
        let mut tys = vec![cl.ret.clone()];
        for st in &cl.body.stmts {
            purity::visit_stmt(st, &mut |e| {
                tys.push(e.ty.clone());
                if let ExprKind::Call { callee: Callee::Fn { type_args, .. }, .. } = &e.kind {
                    tys.extend(type_args.iter().cloned());
                }
            });
        }
        let mut param = None;
        for t in &tys {
            c.infer.zonk(t).walk(&mut |x| {
                if let Ty::Param(i) = x {
                    param.get_or_insert(*i);
                }
            });
        }
        if let Some(i) = param {
            let name = c.generic_names.get(i as usize).cloned().unwrap_or_else(|| "Self".into());
            let d = c.error(msg::COMPTIME_NOT_CONSTANT, range).with("name", name);
            c.report(d);
        }
    }
}

/// Checks a default value or a constant as a body returning it.
#[allow(clippy::too_many_arguments)]
fn check_value(
    prog: &Program,
    inferred: &HashMap<DefId, Vec<DefId>>,
    def: DefId,
    kind: BodyKind,
    expr: &sisu_syntax::ast::Expr,
    ty: &Ty,
    file: sisu_diag::FileId,
    sources: &SourceMap,
) -> (Body, Vec<Diagnostic>, Vec<DefId>) {
    let mut c = Checker::new(prog, inferred, def, file, file_text(sources, file));
    if let Some(f) = prog.fn_info(def) {
        c.env = prog.elaborate_env(&f.predicates);
        c.generic_names = f.generics.iter().map(|g| g.name.clone()).collect();
    }
    if let BodyKind::FieldDefault(s, _) = kind {
        c.field_default_of = Some(s);
    }
    c.ret.push(RetCtx { ty: ty.clone(), closure: None, first_local: 0, loops: 0, captures: Vec::new() });
    let x = c.check_expr(expr, ty);
    c.decide_captures();
    c.check_own_closures();
    check_bounds(&mut c);
    check_comptime_types(&mut c);
    let span = x.span;
    let block = Block { stmts: vec![Stmt::Return { value: Some(x), span }], span };
    let has_errors = c.diags.iter().any(|d| d.is_error());
    let closures = c.closures.drain(..).flatten().collect();
    let mut body = Body {
        def,
        kind,
        params: Vec::new(),
        locals: std::mem::take(&mut c.locals),
        closures,
        ret: ty.clone(),
        block,
        span,
        has_errors,
        generator: None,
        contracts: None,
        generic_asserts: Vec::new(),
        memo: Memo::default(),
    };
    zonk::Zonker { prog, infer: &c.infer }.body(&mut body);
    let reads = c.inferred_reads.take();
    (body, std::mem::take(&mut c.diags), reads)
}

/// Checks every body of a resolved program.
pub fn check(prog: &Program, sources: &SourceMap) -> (Typed, Vec<Diagnostic>) {
    check_with(prog, sources, None)
}

/// [`check`], reusing what `memo` holds: each body is checked again only
/// when its item's text, the program's interface (`iface`, a hash of
/// everything resolution read) or an inferred error set it read changed
/// (docs/architecture.md, "The query layer"). The result is the same as
/// [`check`]'s.
pub fn check_with(
    prog: &Program,
    sources: &SourceMap,
    memo: Option<(&mut BodyCache, u64)>,
) -> (Typed, Vec<Diagnostic>) {
    let mut memo = memo.map(|(cache, iface)| Session { cache, iface });
    // A template is checked in its instances (08, "Reflection").
    let fns: Vec<DefId> = prog
        .functions_with_bodies()
        .into_iter()
        .filter(|d| !prog.fn_info(*d).is_some_and(|f| f.template))
        .collect();
    // Inferred error sets: iterate to a fixpoint over the functions that
    // declare `T !`, each pass using the previous pass's sets (05).
    let inferred_fns: Vec<DefId> =
        fns.iter().copied().filter(|d| prog.fn_info(*d).is_some_and(|f| f.err_inferred)).collect();
    let mut inferred: HashMap<DefId, Vec<DefId>> = inferred_fns.iter().map(|d| (*d, Vec::new())).collect();
    for _ in 0..32 {
        let mut changed = false;
        for d in &inferred_fns {
            let (_, _, absorbed) = memo_body(prog, &inferred, *d, BodyKind::Fn, sources, &mut memo);
            if inferred.get(d) != Some(&absorbed) {
                inferred.insert(*d, absorbed);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    let mut typed = Typed { bodies: Vec::new(), inferred: inferred.clone(), requests: Vec::new() };
    let mut diags = Vec::new();
    // Each body is checked on its own (decision 219): the ones the memo
    // does not have are checked on threads, and taken in order.
    let looked: Vec<Lookup> =
        fns.iter().map(|d| memo_lookup(prog, &inferred, *d, &BodyKind::Fn, sources, &mut memo)).collect();
    let missed: Vec<bool> = looked.iter().map(|l| matches!(l, Lookup::Miss(_))).collect();
    let mut checked =
        each_body(&fns, |i, d| missed[i].then(|| check_body(prog, &inferred, *d, &BodyKind::Fn, sources)))
            .into_iter();
    for (d, look) in fns.iter().zip(looked) {
        let fresh = checked.next().flatten();
        let (body, mut ds, _) = match look {
            Lookup::Hit(found) => found,
            Lookup::Miss(slot) => {
                let fresh = fresh.unwrap_or_else(|| check_body(prog, &inferred, *d, &BodyKind::Fn, sources));
                memo_store(&inferred, *d, BodyKind::Fn, sources, &mut memo, slot, fresh)
            }
        };
        typed.bodies.push(body);
        // A function the compiler wrote from code checked elsewhere too
        // reports nothing of its own.
        if !prog.fn_info(*d).is_some_and(|f| f.quiet) {
            diags.append(&mut ds);
        }
    }
    // Defaults of fields and parameters, and constants.
    for (i, def) in prog.defs.iter().enumerate() {
        let id = DefId(i as u32);
        let kinds: Vec<BodyKind> = match &def.kind {
            DefKind::Struct(s) => (0..s.fields.len() as u32)
                .filter(|fi| s.fields[*fi as usize].default.is_some())
                .map(|fi| BodyKind::FieldDefault(id, fi))
                .collect(),
            DefKind::Fn(f) => (0..f.params.len() as u32)
                .filter(|pi| f.params[*pi as usize].default.is_some())
                .map(|pi| BodyKind::ParamDefault(id, pi))
                .collect(),
            DefKind::Const(c) if c.value.is_some() => vec![BodyKind::Const],
            _ => Vec::new(),
        };
        for kind in kinds {
            let (b, mut ds, _) = memo_body(prog, &inferred, id, kind, sources, &mut memo);
            typed.bodies.push(b);
            diags.append(&mut ds);
        }
    }
    if let Some(main) = prog.main()
        && let Some(f) = prog.fn_info(main)
    {
        let sys = prog.wk.system;
        let params_ok = f.params.is_empty()
            || (f.params.len() == 1 && matches!(&f.params[0].ty, Ty::Adt(d, _) if Some(*d) == sys));
        let ret = prog.display(&f.ret);
        let ret_ok = f.ret == Ty::Unit
            || matches!(&f.ret, Ty::Adt(d, a) if Some(*d) == prog.wk.result && a.first() == Some(&Ty::Unit));
        if !params_ok || !ret_ok || f.self_mode.is_some() {
            let _ = ret;
            diags.push(
                Diagnostic::new(msg::TYPE_MAIN_SIGNATURE, prog.def(main).span)
                    .note(msg::TYPE_MAIN_FORMS, Vec::new()),
            );
        }
    }
    // A test takes nothing, the real `System`, which it then receives, or
    // the capabilities it names, one of each, which it receives as fresh
    // fakes (02; 17, "Isolation and parallelism"; decision 241).
    for &t in &prog.tests {
        let Some(f) = prog.fn_info(t) else { continue };
        let sys = prog.wk.system;
        let real = f.params.len() == 1 && matches!(&f.params[0].ty, Ty::Adt(d, _) if Some(*d) == sys);
        let mut named: Vec<&str> = Vec::new();
        let bad = f.params.iter().find(|p| {
            let fake = match &p.ty {
                Ty::Adt(d, a) if a.is_empty() => prog.builtin(*d).filter(|b| {
                    matches!(*b, "Fs" | "Net" | "Clock" | "TestClock" | "Rand" | "Env" | "Proc" | "Stdin")
                }),
                _ => None,
            };
            match fake {
                Some(b) if !named.contains(&b) && p.mode == sisu_syntax::ast::Mode::Borrow => {
                    named.push(b);
                    false
                }
                _ => true,
            }
        });
        if !real
            && let Some(p) = bad
            && !f.header_broken
        {
            diags.push(Diagnostic::new(msg::TYPE_TEST_SIGNATURE, p.span));
        }
    }
    let (requests, mut ds) = template_requests(prog, &typed.bodies);
    typed.requests = requests;
    diags.append(&mut ds);
    diags.extend(generic_asserts::check_calls(&typed));
    (typed, diags)
}

/// `work` for each of `items`, by index, on as many threads as the machine
/// runs at once (at most 8; this one is one of them, and works alone if no
/// other can be made); the results in the order of `items`.
fn each_body<T: Sync, R: Send>(items: &[T], work: impl Fn(usize, &T) -> Option<R> + Sync) -> Vec<Option<R>> {
    let threads = std::thread::available_parallelism().map_or(1, |n| n.get()).min(8).min(items.len());
    if threads <= 1 {
        return items.iter().enumerate().map(|(i, x)| work(i, x)).collect();
    }
    let next = std::sync::atomic::AtomicUsize::new(0);
    let run = || {
        let mut done = Vec::new();
        loop {
            let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let Some(x) = items.get(i) else { break };
            done.push((i, work(i, x)));
        }
        done
    };
    let mut out: Vec<Option<R>> = std::iter::repeat_with(|| None).take(items.len()).collect();
    std::thread::scope(|s| {
        let others: Vec<_> =
            (1..threads).filter_map(|_| std::thread::Builder::new().spawn_scoped(s, run).ok()).collect();
        let mut done = run();
        for w in others {
            match w.join() {
                Ok(theirs) => done.extend(theirs),
                // A panic in a check is the checker's own: raised again
                // here, as one on this thread would be.
                Err(e) => std::panic::resume_unwind(e),
            }
        }
        for (i, r) in done {
            out[i] = r;
        }
    });
    out
}

/// What the bodies ask of the next resolution (08, "Reflection"): an
/// instance of each template called with concrete type arguments that has
/// none yet, and the promotion of each generic function that calls one
/// with its own type parameters. A template used as a value has no
/// instance to be, and is reported.
fn template_requests(
    prog: &Program,
    bodies: &[std::sync::Arc<Body>],
) -> (Vec<sisu_resolve::Request>, Vec<Diagnostic>) {
    let mut out: Vec<sisu_resolve::Request> = Vec::new();
    let mut diags = Vec::new();
    // A trait's own method that reflects is made in the impls, never for
    // a call, and a generic trait impl's method once, for all its type
    // arguments (decision 251).
    let is_template = |d: &DefId| {
        prog.fn_info(*d)
            .is_some_and(|f| f.template && f.trait_.is_none() && (f.generics.is_empty() || !f.made_once()))
    };
    for b in bodies {
        let mut visit = |e: &thir::Expr| match &e.kind {
            thir::ExprKind::Call { callee: thir::Callee::Fn { def, type_args }, .. } if is_template(def) => {
                let mut param = false;
                let mut unknown = false;
                for t in type_args {
                    t.walk(&mut |x| match x {
                        Ty::Param(_) => param = true,
                        Ty::Var(_) | Ty::Error => unknown = true,
                        _ => {}
                    });
                }
                let q = if param {
                    sisu_resolve::Request::Promote(b.def)
                } else if unknown || prog.instances.contains_key(&(*def, type_args.clone())) {
                    return;
                } else {
                    // Where the program needs it: this call, or the call
                    // that needed the instance this call is in.
                    let site = prog.instance_sites.get(&b.def).copied().unwrap_or(e.span);
                    sisu_resolve::Request::Instance { def: *def, args: type_args.clone(), site }
                };
                if !out.iter().any(|x| x.same(&q)) {
                    out.push(q);
                }
            }
            // A template without type parameters has its one instance,
            // which the value is (decision 244).
            thir::ExprKind::FnRef(def, _)
                if is_template(def) && prog.fn_info(*def).is_some_and(|f| !f.generics.is_empty()) =>
            {
                diags.push(
                    Diagnostic::new(msg::COMPTIME_TEMPLATE_VALUE, e.span)
                        .with("name", prog.def(*def).name.clone()),
                );
            }
            _ => {}
        };
        purity::visit_block(&b.block, &mut visit);
        for c in &b.closures {
            purity::visit_block(&c.body, &mut visit);
        }
    }
    // A template without type parameters (a function or a method that
    // reflects on the program's own types) is made once, whether anything
    // calls it or not: `main`, a method a trait's call reaches, a value;
    // so is a generic trait impl's method, as generic as the impl.
    for d in prog.functions_with_bodies() {
        if prog.fn_info(d).is_some_and(|f| f.made_once() && f.trait_.is_none())
            && !prog.instances.contains_key(&(d, Vec::new()))
        {
            let q = sisu_resolve::Request::Instance { def: d, args: Vec::new(), site: prog.def(d).span };
            if !out.iter().any(|x| x.same(&q)) {
                out.push(q);
            }
        }
    }
    (out, diags)
}

#[cfg(test)]
mod tests;

/// `@must_consume` (D-054), through containers, options and tuples (D-064).
fn must_consume_ty(prog: &Program, t: &Ty, depth: u32) -> bool {
    if depth > 16 {
        return false;
    }
    match t {
        Ty::Tuple(es, _) => es.iter().any(|e| must_consume_ty(prog, e, depth + 1)),
        // A type is must-consume when it is declared so or holds one, in
        // a field or a type argument (D-054's propagation).
        Ty::Adt(d, args) if prog.refinement(*d).is_some() => {
            let base = prog.refinement_base(*d, args).unwrap_or(Ty::Error);
            must_consume_ty(prog, &base, depth + 1)
        }
        Ty::Adt(d, args) => {
            let (flag, fields): (bool, Vec<Ty>) = match &prog.def(*d).kind {
                sisu_resolve::DefKind::Struct(s) => {
                    (s.must_consume, s.fields.iter().map(|f| f.ty.subst(args)).collect())
                }
                sisu_resolve::DefKind::Enum(e) | sisu_resolve::DefKind::ErrorSet(e) => (
                    e.must_consume,
                    e.variants.iter().flat_map(|v| v.fields.iter().map(|(_, t)| t.subst(args))).collect(),
                ),
                _ => (false, Vec::new()),
            };
            flag || args.iter().any(|a| must_consume_ty(prog, a, depth + 1))
                || fields.iter().any(|f| must_consume_ty(prog, f, depth + 1))
        }
        _ => false,
    }
}
