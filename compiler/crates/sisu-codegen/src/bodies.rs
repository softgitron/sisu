//! The bodies the native backend compiles in place of a standard library
//! function's SIR (decision 333): std.par's `map`, whose bodies the runtime
//! runs on its workers, is a call of the runtime's parallel map
//! (`par.map`, an intrinsic no SIR names) and the drop of the function it
//! owns. The interpreter runs the SIR as written, the elision; stage 2's
//! lowering gives the function the same body (`stage2/src/gen.sisu`,
//! `native_bodies`).

use sisu_sir as sir;
use std::borrow::Cow;

/// The program with std.par's `map` given its native body, or the program
/// itself when it has no such function.
pub(crate) fn native_bodies(prog: &sir::Program) -> Cow<'_, sir::Program> {
    let Some(i) = prog.functions.iter().position(|f| is_par_map(prog, f)) else {
        return Cow::Borrowed(prog);
    };
    let mut p = prog.clone();
    p.functions[i] = par_map(&prog.functions[i]);
    Cow::Owned(p)
}

/// Whether `f` is std.par's `map[T, U](xs, f)`.
fn is_par_map(prog: &sir::Program, f: &sir::Function) -> bool {
    f.name == "par.map"
        && f.param_count == 2
        && f.type_params.len() == 2
        && f.locals.len() >= 3
        && prog.sources.get(f.source as usize).is_some_and(|s| s == "<std>/par.sisu")
}

/// `_0 = call intrinsic par.map[$0, $1](copy _1, copy _2)`, then the drop
/// of `_2`, the function `map` owns, on the way out and on the way
/// unwinding.
fn par_map(f: &sir::Function) -> sir::Function {
    use sir::{BasicBlock, BlockId, Callee, LocalId, Operand, Place, Statement, StatementKind, Terminator, Type};
    let at = |kind: StatementKind| Statement { kind, loc: f.loc };
    let call = at(StatementKind::Call {
        dest: Place::local(LocalId(0)),
        callee: Callee::Intrinsic("par.map".to_string(), vec![Type::Param(0), Type::Param(1)]),
        args: vec![Operand::Copy(Place::local(LocalId(1))), Operand::Copy(Place::local(LocalId(2)))],
    });
    let drop_f = || at(StatementKind::Drop { place: Place::local(LocalId(2)), flag: None });
    let block = |stmts: Vec<Statement>, term: Terminator, unwind: Option<BlockId>, cleanup: bool| BasicBlock {
        stmts,
        term,
        unwind,
        cleanup,
        loc: None,
    };
    sir::Function {
        name: f.name.clone(),
        type_params: f.type_params.clone(),
        kind: f.kind.clone(),
        param_count: f.param_count,
        pure: f.pure,
        from: f.from.clone(),
        inner: f.inner.clone(),
        own: f.own.clone(),
        locals: f.locals[..3].to_vec(),
        blocks: vec![
            block(vec![call], Terminator::Goto(BlockId(1)), Some(BlockId(2)), false),
            block(vec![drop_f()], Terminator::Return, None, false),
            block(vec![drop_f()], Terminator::Resume, None, true),
        ],
        loc: f.loc,
        source: f.source,
    }
}
