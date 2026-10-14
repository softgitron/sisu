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
mod exhaustive;
mod expr;
mod infer;
mod pat;
pub mod purity;
pub mod thir;
mod zonk;

use check::{Checker, RetCtx};
use sisu_diag::{Diagnostic, SourceMap, msg};
use sisu_resolve::{Answer, DefId, DefKind, Program, Ty};
use std::collections::HashMap;
pub use thir::*;

/// The type of a parameter as a local of its function: a borrowed
/// parameter is a `ref T` unless `T` is `Copy` (04, rule 2). A borrowed
/// parameter whose type is a generic parameter, or an associated type of
/// one, is always a `ref`: whether its type argument is `Copy` is not known
/// in the body, and every caller passes it the same way.
pub fn param_local_ty(prog: &Program, ty: &Ty, mode: sisu_syntax::ast::Mode) -> Ty {
    use sisu_syntax::ast::Mode;
    match mode {
        Mode::Own => ty.clone(),
        Mode::Mut => Ty::Ref(Box::new(ty.clone()), true),
        Mode::Borrow => {
            if !is_opaque(ty) && prog.is_copy(ty) {
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

/// Checks one function body.
fn check_fn(
    prog: &Program,
    inferred: &HashMap<DefId, Vec<DefId>>,
    def: DefId,
    sources: &SourceMap,
) -> (Body, Vec<Diagnostic>, Vec<DefId>) {
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
    };
    let Some(f) = prog.fn_info(def) else { return (empty, Vec::new(), Vec::new()) };
    let Some(body_ast) = &f.body else { return (empty, Vec::new(), Vec::new()) };
    let mut c = Checker::new(prog, inferred, def, f.file, file_text(sources, f.file));
    c.self_ty = f.owner.clone();
    c.env = prog.elaborate_env(&f.predicates);
    c.generic_names = f.generics.iter().map(|g| g.name.clone()).collect();
    let ret = if f.err_inferred {
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
        let ty = param_local_ty(prog, &owner, mode);
        let l = c.new_local("self", ty, false, def_span);
        c.declare("self", l, false, def_span.range);
        params.push(l);
    }
    for p in &f.params {
        let ty = param_local_ty(prog, &p.ty, p.mode);
        if f.pure
            && let Ty::Adt(d, _) = &p.ty
            && matches!(prog.builtin(*d), Some("Fs" | "Env" | "Clock" | "System"))
        {
            let d = Diagnostic::new(msg::TYPE_IMPURE_CAPABILITY, p.span)
                .with("name", prog.def(def).name.clone())
                .with("ty", prog.display(&p.ty));
            c.report(d);
        }
        let l = c.new_local(&p.name, ty, false, p.span);
        c.declare(&p.name, l, false, p.span.range);
        params.push(l);
    }
    let block = c.check_block_in_scope(body_ast);
    let r = c.infer.zonk(&ret);
    if !matches!(r, Ty::Unit | Ty::Never | Ty::Error) && !f.body_broken && !c.block_diverges(&block) {
        let d = Diagnostic::new(msg::TYPE_MISSING_RETURN, def_span)
            .with("name", prog.def(def).name.clone())
            .with("ty", prog.display(&r));
        c.report(d);
    }
    check_bounds(&mut c);
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
                ret: Ty::Error,
                body: Block::empty(def_span),
                span: def_span,
                fn_ty: Ty::Error,
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
    };
    zonk::Zonker { prog, infer: &c.infer }.body(&mut body);
    // A pure body's calls are checked on SIR (sisu-borrowck's purity pass).
    let diags = std::mem::take(&mut c.diags);
    (body, diags, absorbed)
}

/// Checks the bounds the body's calls and constructs required, once
/// inference is done: each type must implement its trait (03, "Generic
/// syntax"; a generic body is checked against its own bounds only).
fn check_bounds(c: &mut Checker) {
    let prog = c.prog;
    let mut seen: Vec<(Ty, sisu_resolve::TraitRef)> = Vec::new();
    for (ty, bound, span) in std::mem::take(&mut c.bounds) {
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
        if prog.implements(&t, &b, &c.env) == Answer::No {
            seen.push((t.clone(), b.clone()));
            let d = Diagnostic::new(msg::TYPE_BOUND_NOT_SATISFIED, span)
                .with("ty", c.show(&t))
                .with("bound", prog.display_trait_in(&b, &c.generic_names));
            let d = c.missing_impl_fix(d, &t, &b);
            c.report(d);
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
) -> (Body, Vec<Diagnostic>) {
    let mut c = Checker::new(prog, inferred, def, file, file_text(sources, file));
    if let Some(f) = prog.fn_info(def) {
        c.env = prog.elaborate_env(&f.predicates);
        c.generic_names = f.generics.iter().map(|g| g.name.clone()).collect();
    }
    c.ret.push(RetCtx { ty: ty.clone(), closure: None, first_local: 0, loops: 0, captures: Vec::new() });
    let x = c.check_expr(expr, ty);
    check_bounds(&mut c);
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
    };
    zonk::Zonker { prog, infer: &c.infer }.body(&mut body);
    (body, std::mem::take(&mut c.diags))
}

/// Checks every body of a resolved program.
pub fn check(prog: &Program, sources: &SourceMap) -> (Typed, Vec<Diagnostic>) {
    let fns = prog.functions_with_bodies();
    // Inferred error sets: iterate to a fixpoint over the functions that
    // declare `T !`, each pass using the previous pass's sets (05).
    let inferred_fns: Vec<DefId> =
        fns.iter().copied().filter(|d| prog.fn_info(*d).is_some_and(|f| f.err_inferred)).collect();
    let mut inferred: HashMap<DefId, Vec<DefId>> = inferred_fns.iter().map(|d| (*d, Vec::new())).collect();
    for _ in 0..32 {
        let mut changed = false;
        for d in &inferred_fns {
            let (_, _, absorbed) = check_fn(prog, &inferred, *d, sources);
            if inferred.get(d) != Some(&absorbed) {
                inferred.insert(*d, absorbed);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    let mut typed = Typed { bodies: Vec::new(), inferred: inferred.clone() };
    let mut diags = Vec::new();
    for d in &fns {
        let (body, mut ds, _) = check_fn(prog, &inferred, *d, sources);
        typed.bodies.push(body);
        diags.append(&mut ds);
    }
    // Defaults of fields and parameters, and constants.
    for (i, def) in prog.defs.iter().enumerate() {
        let id = DefId(i as u32);
        match &def.kind {
            DefKind::Struct(s) => {
                for (fi, f) in s.fields.iter().enumerate() {
                    if let Some(e) = &f.default {
                        let (b, mut ds) = check_value(
                            prog,
                            &inferred,
                            id,
                            BodyKind::FieldDefault(id, fi as u32),
                            e,
                            &f.ty,
                            f.span.file,
                            sources,
                        );
                        typed.bodies.push(b);
                        diags.append(&mut ds);
                    }
                }
            }
            DefKind::Fn(f) => {
                for (pi, p) in f.params.iter().enumerate() {
                    if let Some(e) = &p.default {
                        let (b, mut ds) = check_value(
                            prog,
                            &inferred,
                            id,
                            BodyKind::ParamDefault(id, pi as u32),
                            e,
                            &p.ty,
                            f.file,
                            sources,
                        );
                        typed.bodies.push(b);
                        diags.append(&mut ds);
                    }
                }
            }
            DefKind::Const(c) => {
                if let Some(e) = &c.value {
                    let (b, mut ds) =
                        check_value(prog, &inferred, id, BodyKind::Const, e, &c.ty, c.file, sources);
                    typed.bodies.push(b);
                    diags.append(&mut ds);
                }
            }
            _ => {}
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
            diags.push(Diagnostic::new(msg::TYPE_MAIN_SIGNATURE, prog.def(main).span));
        }
    }
    (typed, diags)
}

#[cfg(test)]
mod tests;
