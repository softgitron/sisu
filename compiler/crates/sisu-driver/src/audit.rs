//! `sisu audit --ownership` (18, Phase 3): how often each file reaches for
//! the escape hatches of the ownership model, for the corpus study's
//! measures of `unsafe` use and of escape-hatch density (18, T1.1 and
//! T1.2).
//!
//! The counts are of tokens, so they mean the same to any implementation:
//! the names `Shared`, `Weak`, `Cell`, `Arena` and `Key` where they mean
//! std's types (not after a `.`, and not in a file that declares the name
//! itself: a type, trait, function, constant, alias, variant or imported
//! name), the method calls `.copy()`, and the keyword `unsafe`. `lines`
//! counts the lines of code: those on which a token starts that is not
//! trivia (a comment, a blank) or layout.
//!
//! The output is data, one line per file: the file's name, then
//! `name=count` for `lines` and each column in [`COLUMNS`]' order, separated
//! by single spaces.

use sisu_syntax::SyntaxKind;
use sisu_syntax::ast::{self, ItemKind};
use sisu_syntax::tree::Token;
use std::collections::HashSet;

/// The counted names and constructs, in output order.
pub const COLUMNS: [&str; 7] = ["Shared", "Weak", "Cell", "Arena", "Key", "copy", "unsafe"];

/// The counts for one file.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Counts {
    pub lines: usize,
    /// In [`COLUMNS`]' order.
    pub counts: [usize; COLUMNS.len()],
}

impl Counts {
    /// The file's line of output, without the line break.
    pub fn line(&self, file: &str) -> String {
        let mut out = format!("{file} lines={}", self.lines);
        for (name, n) in COLUMNS.iter().zip(self.counts) {
            out.push_str(&format!(" {name}={n}"));
        }
        out
    }
}

/// Counts one file from its tokens and its module.
pub fn ownership(source: &str, tokens: &[Token], module: &ast::Module) -> Counts {
    let mut declared = HashSet::new();
    declared_names(&module.items, &mut declared);
    let text = |t: &Token| source.get(t.range.start as usize..t.range.end as usize).unwrap_or("");
    // The tokens the counts look at: no trivia, no layout.
    let code: Vec<&Token> = tokens
        .iter()
        .filter(|t| {
            !t.kind.is_trivia()
                && !matches!(
                    t.kind,
                    SyntaxKind::NEWLINE | SyntaxKind::INDENT | SyntaxKind::DEDENT | SyntaxKind::EOF
                )
        })
        .collect();
    // The offset after each line break: a token at offset `o` is on line
    // `breaks.partition_point(|b| *b <= o)`.
    let breaks: Vec<usize> = source.match_indices('\n').map(|(i, _)| i + 1).collect();
    let mut c = Counts::default();
    let mut lines = HashSet::new();
    for (i, t) in code.iter().enumerate() {
        lines.insert(breaks.partition_point(|b| *b <= t.range.start as usize));
        let after_dot = i > 0 && code[i - 1].kind == SyntaxKind::DOT;
        let column = match t.kind {
            SyntaxKind::UNSAFE_KW => Some(6),
            SyntaxKind::IDENT if after_dot => {
                let call = code.get(i + 1).is_some_and(|n| n.kind == SyntaxKind::L_PAREN);
                (text(t) == "copy" && call).then_some(5)
            }
            SyntaxKind::IDENT if !declared.contains(text(t)) => {
                COLUMNS[..5].iter().position(|n| *n == text(t))
            }
            _ => None,
        };
        if let Some(k) = column {
            c.counts[k] += 1;
        }
    }
    c.lines = lines.len();
    c
}

/// The names a file declares at its top level, which then mean its own
/// declarations rather than std's.
fn declared_names(items: &[ast::Item], out: &mut HashSet<String>) {
    for item in items {
        match &item.kind {
            ItemKind::Fn(f) => {
                out.insert(f.name.text.clone());
            }
            ItemKind::Struct(s) => {
                out.insert(s.name.text.clone());
            }
            ItemKind::Enum(e) | ItemKind::ErrorSet(e) => {
                out.insert(e.name.text.clone());
                out.extend(e.variants.iter().map(|v| v.name.text.clone()));
            }
            ItemKind::Trait(t) => {
                out.insert(t.name.text.clone());
            }
            ItemKind::Const(k) => {
                out.insert(k.name.text.clone());
            }
            ItemKind::TypeAlias(a) => {
                out.insert(a.name.text.clone());
            }
            ItemKind::FromImport(f) => {
                out.extend(f.names.iter().map(|(n, alias)| alias.as_ref().unwrap_or(n).text.clone()));
            }
            ItemKind::Extern(inner) => declared_names(inner, out),
            _ => {}
        }
    }
}

/// The capability types of `sisu audit --caps`: the handles of a
/// `System`, the `System`, a test clock, and a `Lent` of a handle.
const CAPABILITIES: [&str; 10] =
    ["Fs", "Net", "Clock", "Rand", "Env", "Proc", "Stdin", "System", "TestClock", "Lent"];

/// The operations traits, whose methods a capability's calls are.
const OPS_TRAITS: [&str; 7] = ["FsOps", "NetOps", "ClockOps", "RandOps", "EnvOps", "ProcOps", "StdinOps"];

/// The capability types a type holds, by name: itself (through
/// references), or the fields of a struct or enum of the program, its
/// tuples, options and collections (decision 240).
fn capability_types(prog: &sisu_sir::Program, t: &sisu_sir::Type, out: &mut Vec<String>, depth: u32) {
    use sisu_sir::{Type, TypeDecl};
    if depth > 8 {
        return;
    }
    match t {
        Type::Ref(inner, _) => capability_types(prog, inner, out, depth + 1),
        Type::Tuple(es) => es.iter().for_each(|e| capability_types(prog, e, out, depth + 1)),
        Type::Named(n, args) if CAPABILITIES.contains(&n.as_str()) => {
            // A `Lent` is its handle's capability.
            let name = match (n.as_str(), args.first()) {
                ("Lent", Some(Type::Named(h, _))) => h.clone(),
                _ => n.clone(),
            };
            if !out.contains(&name) {
                out.push(name);
            }
        }
        Type::Named(n, args) => {
            args.iter().for_each(|a| capability_types(prog, a, out, depth + 1));
            match prog.types.iter().find(|d| d.name() == n.as_str()) {
                Some(TypeDecl::Struct { fields, .. }) => {
                    fields.iter().for_each(|f| capability_types(prog, &f.ty.subst(args), out, depth + 1))
                }
                Some(TypeDecl::Enum { variants, .. }) => variants
                    .iter()
                    .flat_map(|v| v.fields.iter())
                    .for_each(|(_, ft)| capability_types(prog, &ft.subst(args), out, depth + 1)),
                None => {}
            }
        }
        _ => {}
    }
}

/// `sisu audit --caps` (18, Phase 6; T2.1, T2.3 and the threading depth of
/// 20's pre-registration), computed from the program's SIR: for each
/// function of the program that does I/O (calls an operation of a
/// capability, or a function that does), the number of its parameters
/// that hold a capability; for each such call (a leaf use), the
/// threading depth, the fewest calls from `main` to the function that
/// makes it; and the structs of the program that bundle three or more
/// capability types. A function's own lines come first, by name, then
/// the uses, then the structs, then a summary; every line starts with
/// the file's name (decision 240).
pub fn caps(file: &str, prog: &sisu_sir::Program, own_structs: &[String]) -> String {
    use sisu_sir::{Callee, Operand, Rvalue, StatementKind, Type};
    use std::collections::{BTreeMap, VecDeque};
    let own =
        |f: &sisu_sir::Function| prog.sources.get(f.source as usize).is_some_and(|s| !s.starts_with('<'));
    let index: BTreeMap<&str, usize> = prog
        .functions
        .iter()
        .enumerate()
        .filter(|(_, f)| own(f))
        .map(|(i, f)| (f.name.as_str(), i))
        .collect();
    // Each function's calls of the program's functions (a closure counts
    // as called where it is made), and its uses of capabilities.
    let mut edges: Vec<Vec<usize>> = vec![Vec::new(); prog.functions.len()];
    let mut uses: Vec<(usize, String)> = Vec::new();
    let mut generic_caps: Vec<Vec<u32>> = vec![Vec::new(); prog.functions.len()];
    for &i in index.values() {
        let f = &prog.functions[i];
        for s in f.blocks.iter().flat_map(|b| b.stmts.iter()) {
            match &s.kind {
                StatementKind::Call { callee, args, .. } => {
                    let first_ty = args.first().and_then(|a| match a {
                        Operand::Copy(p) | Operand::Move(p) => place_type(prog, f, p),
                        Operand::Const(_) => None,
                    });
                    // A call on a capability itself (not on a struct
                    // that holds one) is a use of it.
                    let on_capability = first_ty.as_ref().is_some_and(|t| {
                        let mut base = t;
                        while let Type::Ref(inner, _) = base {
                            base = inner;
                        }
                        matches!(base, Type::Named(n, _) if CAPABILITIES.contains(&n.as_str()))
                    });
                    match callee {
                        Callee::Fn(target, _) => {
                            if let Some(&j) = index.get(target.as_str()) {
                                edges[i].push(j);
                            } else if on_capability {
                                uses.push((i, target.clone()));
                            }
                        }
                        Callee::Trait(method, tys) => {
                            let tr = method.split('.').next().unwrap_or("");
                            if OPS_TRAITS.contains(&tr) {
                                uses.push((i, method.clone()));
                                // A type parameter that a capability's
                                // operation is called on is a capability.
                                if let Some(Type::Param(k)) = tys.first().map(strip_refs)
                                    && !generic_caps[i].contains(k)
                                {
                                    generic_caps[i].push(*k);
                                }
                            } else {
                                // The impls of the program it may run.
                                let m = method.rsplit('.').next().unwrap_or("");
                                for imp in prog.impls.iter().filter(|im| im.trait_name == tr) {
                                    for (mn, target) in &imp.methods {
                                        if let (true, sisu_sir::ImplTarget::Fn(t)) = (mn == m, target)
                                            && let Some(&j) = index.get(t.as_str())
                                        {
                                            edges[i].push(j);
                                        }
                                    }
                                }
                            }
                        }
                        // Lending a capability passes it on; it does no I/O.
                        Callee::Intrinsic(name, _)
                            if on_capability && !name.starts_with("cap.") && !name.starts_with("Lent.") =>
                        {
                            uses.push((i, name.clone()))
                        }
                        _ => {}
                    }
                }
                StatementKind::Assign(_, Rvalue::Closure(name, _, _)) => {
                    if let Some(&j) = index.get(name.as_str()) {
                        edges[i].push(j);
                    }
                }
                _ => {}
            }
        }
    }
    // The fewest calls from `main` to each function.
    let mut depth: Vec<Option<usize>> = vec![None; prog.functions.len()];
    if let Some(&m) = prog.entry.as_ref().and_then(|e| index.get(e.function.as_str())) {
        depth[m] = Some(0);
        let mut queue = VecDeque::from([m]);
        while let Some(i) = queue.pop_front() {
            for &j in &edges[i] {
                if depth[j].is_none() {
                    depth[j] = depth[i].map(|d| d + 1);
                    queue.push_back(j);
                }
            }
        }
    }
    // The functions that do I/O: those with a use, and their callers.
    let mut io: Vec<Option<&str>> = vec![None; prog.functions.len()];
    for (i, _) in &uses {
        io[*i] = Some("direct");
    }
    loop {
        let mut more = false;
        for &i in index.values() {
            if io[i].is_none() && edges[i].iter().any(|&j| io[j].is_some()) {
                io[i] = Some("calls");
                more = true;
            }
        }
        if !more {
            break;
        }
    }
    let shown = |d: Option<usize>| d.map_or("-".to_string(), |d| d.to_string());
    let mut out = String::new();
    let mut counts = Vec::new();
    for (&name, &i) in &index {
        let Some(kind) = io[i] else { continue };
        let f = &prog.functions[i];
        let params = (1..=f.param_count as usize)
            .filter(|k| {
                let ty = &f.locals[*k].ty;
                let mut found = Vec::new();
                capability_types(prog, ty, &mut found, 0);
                !found.is_empty() || matches!(strip_refs(ty), Type::Param(p) if generic_caps[i].contains(p))
            })
            .count();
        counts.push(params);
        out.push_str(&format!("{file} function {name} caps={params} io={kind} depth={}\n", shown(depth[i])));
    }
    uses.sort_by(|a, b| (&prog.functions[a.0].name, &a.1).cmp(&(&prog.functions[b.0].name, &b.1)));
    uses.dedup();
    for (i, op) in &uses {
        out.push_str(&format!("{file} use {} {op} depth={}\n", prog.functions[*i].name, shown(depth[*i])));
    }
    let mut bundles = 0;
    for d in &prog.types {
        let sisu_sir::TypeDecl::Struct { name, fields, .. } = d else { continue };
        if !own_structs.contains(name) {
            continue;
        }
        let mut found = Vec::new();
        for f in fields {
            capability_types(prog, &f.ty, &mut found, 0);
        }
        // A `System` bundles all of them, by the language's design.
        found.retain(|c| c != "System");
        if found.len() >= 3 {
            bundles += 1;
            out.push_str(&format!("{file} struct {name} caps={} {}\n", found.len(), found.join(",")));
        }
    }
    counts.sort_unstable();
    let median = match counts.len() {
        0 => "0".to_string(),
        n if n % 2 == 1 => counts[n / 2].to_string(),
        n => {
            let twice = counts[n / 2 - 1] + counts[n / 2];
            if twice % 2 == 0 { (twice / 2).to_string() } else { format!("{}.5", twice / 2) }
        }
    };
    let deepest = uses.iter().filter_map(|(i, _)| depth[*i]).max();
    out.push_str(&format!(
        "{file} summary functions={} median={median} max={} uses={} deepest={} structs={bundles}\n",
        counts.len(),
        counts.last().copied().unwrap_or(0),
        uses.len(),
        shown(deepest),
    ));
    out
}

/// A type without the references around it.
fn strip_refs(t: &sisu_sir::Type) -> &sisu_sir::Type {
    match t {
        sisu_sir::Type::Ref(inner, _) => strip_refs(inner),
        other => other,
    }
}

/// The type of a place, a `System`'s fields being its handles.
fn place_type(
    prog: &sisu_sir::Program,
    f: &sisu_sir::Function,
    p: &sisu_sir::Place,
) -> Option<sisu_sir::Type> {
    use sisu_sir::{Proj, Type};
    if let Some(t) = sisu_sir::types::place_ty(&|n| prog.types.iter().find(|d| d.name() == n), f, p) {
        return Some(t);
    }
    // `sys.fs`: the `System` is a builtin, its fields the handles in the
    // order `System.assemble` takes them.
    const HANDLES: [&str; 7] = ["Fs", "Net", "Clock", "Rand", "Env", "Proc", "Stdin"];
    let base = f.locals.get(p.local.0 as usize)?;
    let mut projs = p.proj.iter().skip_while(|x| matches!(x, Proj::Deref));
    match (strip_refs(&base.ty), projs.next(), projs.next()) {
        (Type::Named(n, _), Some(Proj::Field(i)), None) if n == "System" => {
            HANDLES.get(*i as usize).map(|h| Type::Named(h.to_string(), Vec::new()))
        }
        _ => None,
    }
}

/// The structs the program declares, by their SIR names (std's and the
/// dependencies' are not the program's design).
pub fn own_structs(p: &sisu_resolve::Program) -> Vec<String> {
    (0..p.defs.len())
        .map(|i| sisu_resolve::DefId(i as u32))
        .filter(|d| {
            let def = p.def(*d);
            matches!(def.kind, sisu_resolve::program::DefKind::Struct(_))
                && p.modules
                    .get(def.module.0 as usize)
                    .is_some_and(|m| m.package == sisu_resolve::program::ROOT_PACKAGE)
        })
        .map(|d| p.qualified_name(d))
        .collect()
}
