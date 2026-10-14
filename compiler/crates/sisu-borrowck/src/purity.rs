//! `pure` on SIR (D-074; 18, pipeline step 6): a pure function calls only
//! pure functions, pure intrinsics and function values of a pure type, and
//! drops only values whose `Drop` (if any) is pure, since dropping runs it.
//! The bodies of the closures a pure function creates are held to the same
//! rule, since the closure runs on the pure function's behalf (the
//! prelude's higher-order functions are pure when their function arguments
//! are). Taking a capability as a parameter, and passing an impure
//! function where a pure one is expected, are type rules checked before
//! SIR (E0334, E0335).

use crate::Finding;
use crate::loans::display_fn;
use crate::types::Types;
use sisu_diag::msg;
use sisu_sir::{Callee, Function, Operand, Program, Rvalue, StatementKind, Type};

pub fn check(p: &Program) -> Vec<Finding> {
    let types = Types::new(p);
    let mut out = Vec::new();
    for f in &p.functions {
        if !f.pure || f.kind == sisu_sir::FnKind::Closure || f.kind == sisu_sir::FnKind::Value {
            continue;
        }
        let name = display_fn(&f.name);
        let mut todo: Vec<&Function> = vec![f];
        let mut seen: Vec<String> = vec![f.name.clone()];
        while let Some(g) = todo.pop() {
            for (b, block) in g.blocks.iter().enumerate() {
                for (si, s) in block.stmts.iter().enumerate() {
                    match &s.kind {
                        StatementKind::Call { callee, .. } => {
                            let impure = match callee {
                                Callee::Fn(n, _) => p.function(n).filter(|c| !c.pure).map(|_| display_fn(n)),
                                Callee::Trait(n, _) => {
                                    p.trait_method(n).filter(|d| !d.pure).map(|_| n.clone())
                                }
                                Callee::Intrinsic(n, _) => p
                                    .intrinsic(n)
                                    .filter(|d| !d.pure)
                                    .map(|_| n.rsplit('.').next().unwrap_or(n).to_string()),
                                Callee::Value(Operand::Copy(pl) | Operand::Move(pl)) => {
                                    match types.place_ty(g, pl) {
                                        Some(Type::Fn(_, _, false)) => {
                                            out.push(
                                                Finding::new(msg::TYPE_IMPURE_VALUE_CALL, g, s.loc)
                                                    .with("name", name.clone())
                                                    .label(
                                                        f.loc,
                                                        msg::TYPE_DECLARED_PURE_HERE,
                                                        pure_here(&name),
                                                    )
                                                    .at(b, si),
                                            );
                                            None
                                        }
                                        _ => None,
                                    }
                                }
                                Callee::Value(Operand::Const(_)) => None,
                            };
                            if let Some(callee) = impure {
                                out.push(
                                    Finding::new(msg::TYPE_IMPURE_CALL, g, s.loc)
                                        .with("callee", callee)
                                        .with("name", name.clone())
                                        .label(f.loc, msg::TYPE_DECLARED_PURE_HERE, pure_here(&name))
                                        .at(b, si),
                                );
                            }
                        }
                        // Dropping a value runs its `Drop`, which must be pure too.
                        StatementKind::Drop { place, .. } if !block.cleanup => {
                            if let Some(t) = types.place_ty(g, place) {
                                for d in types.drop_fns(&t) {
                                    if p.function(&d).is_some_and(|c| !c.pure) {
                                        // `Type.Drop.drop`, the method of `impl Drop for Type`.
                                        let owner = d
                                            .strip_suffix(".Drop.drop")
                                            .or_else(|| d.rsplit_once('.').map(|(o, _)| o))
                                            .unwrap_or(&d)
                                            .to_string();
                                        let value = crate::names::place_name(&types, g, place);
                                        let mut fd = Finding::new(msg::TYPE_IMPURE_DROP, g, s.loc)
                                            .with_arg("value", value.clone())
                                            .with("ty", owner)
                                            .with("name", name.clone())
                                            .at(b, si);
                                        let decl = g.locals.get(place.local.0 as usize).and_then(|l| l.loc);
                                        if decl.is_some() && crate::names::is_named(g, place.local) {
                                            fd = fd.label(
                                                decl,
                                                msg::BORROW_DECLARED_HERE,
                                                vec![("name", value)],
                                            );
                                        }
                                        fd = fd.label(f.loc, msg::TYPE_DECLARED_PURE_HERE, pure_here(&name));
                                        out.push(fd);
                                    }
                                }
                            }
                        }
                        StatementKind::Assign(_, Rvalue::Closure(c, _, _)) => {
                            if !seen.contains(c)
                                && let Some(cf) = p.function(c)
                            {
                                seen.push(c.clone());
                                todo.push(cf);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    out
}

fn pure_here(name: &str) -> Vec<(&'static str, crate::Arg)> {
    vec![("name", crate::Arg::Text(name.to_string()))]
}
