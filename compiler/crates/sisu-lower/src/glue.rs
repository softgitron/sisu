//! Drop glue as SIR functions (04, rule 6; 18, Phase 4).
//!
//! For each declared type whose drop can run code or change what the
//! program observes, lowering emits `T#drop`, a function of kind `glue`
//! that takes the value with `own` and drops it: the type's own `Drop`
//! first, with a reference to the whole value, then the fields in reverse
//! declaration order (of the active variant, for an enum), each by its own
//! `drop`. A panic in one of them still drops the rest, in cleanup blocks
//! (the drop ladder), where a second panic aborts.
//!
//! A field whose type drops nothing is left out, and a type all of whose
//! fields drop nothing and that has no `Drop` has no glue. A generic
//! parameter may stand for anything, so a field of that type is dropped.
//! The host-backed types have no glue in SIR: the run time drops a `list`,
//! `set` or `Arena` element by element, first to last; a `dict` entry's
//! value before its key; a tuple's elements last to first; a `Shared` or
//! `Weak` by its count, and a mutex guard by unlocking (their contracts in
//! `std/prelude.sisu`).

use sisu_sir as sir;
use sisu_sir::{
    BasicBlock, BlockId, Callee, FnKind, Function, LocalDecl, LocalId, Operand, Place, Proj, Rvalue,
    Statement, StatementKind, Terminator, Type, TypeDecl,
};
use std::collections::HashMap;

/// The host-backed types that have declarations in SIR (the others are
/// [`sir::validate::BUILTIN_TYPES`]).
const HOST_DECLARED: &[&str] = &["Shared", "Weak", "Cell", "Mutex", "MutexGuard", "Arena"];

/// The glue functions of the declared types that need one.
pub fn glue_functions(types: &[TypeDecl]) -> Vec<Function> {
    let needs = needs_glue(types);
    types.iter().filter(|t| needs.get(t.name()).copied().unwrap_or(false)).map(|t| glue(t, &needs)).collect()
}

/// Which declared types need glue: the least fixpoint of "has a `Drop`, or
/// a field whose type drops something".
fn needs_glue(types: &[TypeDecl]) -> HashMap<&str, bool> {
    let mut needs: HashMap<&str, bool> =
        types.iter().filter(|t| !HOST_DECLARED.contains(&t.name())).map(|t| (t.name(), false)).collect();
    loop {
        let mut changed = false;
        for t in types {
            if needs.get(t.name()) != Some(&false) {
                continue;
            }
            let yes = match t {
                TypeDecl::Struct { drop, fields, .. } => {
                    drop.is_some() || fields.iter().any(|f| drops_something(&f.ty, &needs))
                }
                TypeDecl::Enum { drop, variants, .. } => {
                    drop.is_some()
                        || variants.iter().any(|v| v.fields.iter().any(|(_, t)| drops_something(t, &needs)))
                }
            };
            if yes {
                needs.insert(t.name(), true);
                changed = true;
            }
        }
        if !changed {
            return needs;
        }
    }
}

/// Whether dropping a value of the type can do anything: run a `Drop`,
/// change a count, unlock a mutex.
pub fn drops_something(t: &Type, needs: &HashMap<&str, bool>) -> bool {
    match t {
        Type::Int | Type::Float | Type::Bool | Type::Char | Type::Str | Type::Unit | Type::Never => false,
        // A reference owns nothing; a closure holds references, or copies
        // of `Copy` values (an `own` closure).
        Type::Ref(..) | Type::Fn(..) => false,
        Type::Tuple(es) => es.iter().any(|e| drops_something(e, needs)),
        Type::Param(_) | Type::Assoc(..) | Type::Dyn(..) => true,
        Type::ErrSet(ms) => ms.iter().any(|m| needs.get(m.as_str()).copied().unwrap_or(true)),
        Type::Named(n, args) => match n.as_str() {
            "Shared" | "Weak" | "MutexGuard" => true,
            "String" | "Path" | "DateTime" | "Fs" | "Env" | "Clock" | "System" => false,
            "list" | "dict" | "set" | "Iter" | "Entry" | "Iterable" | "Option" | "Result" | "Cell"
            | "Mutex" | "Arena" => args.iter().any(|a| drops_something(a, needs)),
            other => needs.get(other).copied().unwrap_or(true),
        },
    }
}

/// Builds blocks in order.
struct Builder {
    blocks: Vec<BasicBlock>,
}

impl Builder {
    fn add(
        &mut self,
        stmts: Vec<Statement>,
        term: Terminator,
        unwind: Option<BlockId>,
        cleanup: bool,
    ) -> BlockId {
        self.blocks.push(BasicBlock { stmts, term, unwind, cleanup, loc: None });
        BlockId(self.blocks.len() as u32 - 1)
    }

    /// Drops `parts` in order, each in a block of its own, and returns
    /// after the last (so that the run time can drop the last part in the
    /// caller's place, keeping a chain of values at one frame). The cleanup
    /// ladder beside them drops what is left when one panics: rung `i`
    /// drops part `i` and climbs on, the last one resumes. Returns the
    /// first block and, when `whole` asks for it, the rung that drops every
    /// part (for a panic before the first).
    fn drops(&mut self, parts: &[Place], resume: BlockId, whole: bool) -> (BlockId, BlockId) {
        let mut rungs = vec![resume; parts.len() + 1];
        let lowest = if whole { 0 } else { 1 };
        for (i, p) in parts.iter().enumerate().skip(lowest).rev() {
            rungs[i] = self.add(vec![drop(p.clone())], Terminator::Goto(rungs[i + 1]), None, true);
        }
        let mut next = None;
        for (i, p) in parts.iter().enumerate().rev() {
            let unwind = Some(rungs[i + 1]).filter(|r| *r != resume);
            let term = next.map_or(Terminator::Return, Terminator::Goto);
            next = Some(self.add(vec![drop(p.clone())], term, unwind, false));
        }
        (next.unwrap_or(resume), rungs[0])
    }
}

fn drop(place: Place) -> Statement {
    Statement { kind: StatementKind::Drop { place, flag: None }, loc: None }
}

fn glue(t: &TypeDecl, needs: &HashMap<&str, bool>) -> Function {
    let params: Vec<String> = t.params().to_vec();
    let self_ty = Type::Named(t.name().to_string(), (0..params.len() as u32).map(Type::Param).collect());
    let value = Place::local(LocalId(1));
    let mut locals =
        vec![LocalDecl::new(Type::Unit, None), LocalDecl::new(self_ty.clone(), Some("self".into()))];
    // The parts to drop, in order, for the struct (`None`) or each variant.
    let arms: Vec<(Option<u32>, Vec<Place>)> = match t {
        TypeDecl::Struct { fields, .. } => {
            let parts: Vec<Place> = (0..fields.len())
                .rev()
                .filter(|i| drops_something(&fields[*i].ty, needs))
                .map(|i| value.project(Proj::Field(i as u32)))
                .collect();
            if parts.is_empty() { Vec::new() } else { vec![(None, parts)] }
        }
        TypeDecl::Enum { variants, .. } => variants
            .iter()
            .enumerate()
            .map(|(v, var)| {
                let parts: Vec<Place> = (0..var.fields.len())
                    .rev()
                    .filter(|i| drops_something(&var.fields[*i].1, needs))
                    .map(|i| value.project(Proj::Downcast(v as u32)).project(Proj::Field(i as u32)))
                    .collect();
                (Some(v as u32), parts)
            })
            .filter(|(_, parts)| !parts.is_empty())
            .collect(),
    };
    let drop_fn = match t {
        TypeDecl::Struct { drop, .. } | TypeDecl::Enum { drop, .. } => drop.clone(),
    };
    let mut b = Builder { blocks: Vec::new() };
    // bb0 is the entry, filled in last.
    let entry = b.add(Vec::new(), Terminator::Unreachable, None, false);
    let ret = b.add(Vec::new(), Terminator::Return, None, false);
    // Cleanup is needed when a panic can leave parts behind: in the type's
    // `Drop`, or in a part that is not the last.
    let unwinds = arms.iter().any(|(_, parts)| drop_fn.is_some() || parts.len() > 1);
    let resume = if unwinds { b.add(Vec::new(), Terminator::Resume, None, true) } else { ret };
    let mut normal: Vec<(i64, BlockId)> = Vec::new();
    let mut unwinding: Vec<(i64, BlockId)> = Vec::new();
    for (v, parts) in &arms {
        let (first, rung) = b.drops(parts, resume, drop_fn.is_some());
        normal.push((v.unwrap_or(0) as i64, first));
        unwinding.push((v.unwrap_or(0) as i64, rung));
    }
    // Where the fields' drops start, and where they start when the type's
    // `Drop` panicked.
    let (body, cleanup) = match t {
        _ if arms.is_empty() => (ret, None),
        TypeDecl::Struct { .. } => (normal[0].1, Some(unwinding[0].1)),
        TypeDecl::Enum { .. } => {
            let disc = LocalId(locals.len() as u32);
            locals.push(LocalDecl::new(Type::Int, None));
            let read = Statement {
                kind: StatementKind::Assign(Place::local(disc), Rvalue::Discriminant(value.clone())),
                loc: None,
            };
            let switch = |cases: Vec<(i64, BlockId)>, default: BlockId| Terminator::Switch {
                value: Operand::Copy(Place::local(disc)),
                cases,
                default,
            };
            let body = b.add(vec![read.clone()], switch(normal, ret), None, false);
            let cleanup = drop_fn.is_some().then(|| b.add(vec![read], switch(unwinding, resume), None, true));
            (body, cleanup)
        }
    };
    let mut first = BasicBlock {
        stmts: Vec::new(),
        term: Terminator::Goto(body),
        unwind: None,
        cleanup: false,
        loc: None,
    };
    // The type's own `Drop` runs first, on the whole value.
    if let Some(d) = drop_fn {
        let r = LocalId(locals.len() as u32);
        locals.push(LocalDecl::new(Type::Ref(Box::new(self_ty), true), None));
        first.stmts = vec![
            Statement { kind: StatementKind::Assign(Place::local(r), Rvalue::Ref(value, true)), loc: None },
            Statement {
                kind: StatementKind::Call {
                    dest: Place::local(LocalId(0)),
                    callee: Callee::Fn(d, (0..params.len() as u32).map(Type::Param).collect()),
                    args: vec![Operand::Move(Place::local(r))],
                },
                loc: None,
            },
        ];
        first.unwind = cleanup;
    }
    b.blocks[entry.0 as usize] = first;
    Function {
        name: sir::types::glue_name(t.name()),
        type_params: params,
        kind: FnKind::Glue,
        param_count: 1,
        pure: false,
        from: Vec::new(),
        inner: Vec::new(),
        own: vec![0],
        locals,
        blocks: b.blocks,
        loc: None,
        source: 0,
    }
}
