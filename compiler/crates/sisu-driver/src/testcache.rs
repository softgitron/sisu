//! The test cache (17, "Output and caching": results "cached keyed on the
//! test's code and the capabilities it touched"; decision 242). A test
//! that passed is not run again while its key is unchanged: the key
//! hashes the toolchain, the options that decide a run, the program's
//! declarations (types, intrinsics, trait methods and impls), the drops
//! and invariants any test may run, the stored snapshots, and the SIR of
//! every function the test reaches (its calls, its closures, the
//! functions it uses as values, and each impl of a trait method it calls),
//! with source positions and the test's own number left out, so that a
//! comment, a reordering or a new test reruns nothing. A test's
//! capabilities are in its signature: one that takes the real `System`
//! touches the world and is never cached, and one that names capabilities
//! gets fakes, which the key's code covers. A cached test reports `ok` as
//! a run would; nothing printed depends on the cache.

use sisu_sir::{Callee, Const, FnKind, Operand, Program, Rvalue, StatementKind, Terminator};
use std::collections::{BTreeMap, BTreeSet, HashMap};

/// FNV-1a over 64 bits.
fn fnv(h: &mut u64, bytes: &[u8]) {
    for b in bytes {
        *h ^= u64::from(*b);
        *h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    // A separator, so that two parts never run into one another.
    *h ^= 0xff;
    *h = h.wrapping_mul(0x0000_0100_0000_01b3);
}

/// Text without its source positions (`@12:5-12:9`) and its functions'
/// sources (`source 0`), which an edit elsewhere moves.
fn without_positions(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let b = text.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'@' && b.get(i + 1).is_some_and(u8::is_ascii_digit) {
            let mut j = i + 1;
            while j < b.len() && (b[j].is_ascii_digit() || b[j] == b':' || b[j] == b'-') {
                j += 1;
            }
            i = j;
            continue;
        }
        if text[i..].starts_with(" source ") && b.get(i + 8).is_some_and(u8::is_ascii_digit) {
            let mut j = i + 8;
            while j < b.len() && b[j].is_ascii_digit() {
                j += 1;
            }
            i = j;
            continue;
        }
        let c = text[i..].chars().next().unwrap_or(' ');
        out.push(c);
        i += c.len_utf8();
    }
    out
}

/// `name` replaced by `with` wherever it is a whole name (not followed by
/// a digit: `test#3` in `test#3#closure0`, not in `test#30`).
fn renamed(text: &str, name: &str, with: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find(name) {
        let after = &rest[at + name.len()..];
        out.push_str(&rest[..at]);
        if after.starts_with(|c: char| c.is_ascii_digit()) {
            out.push_str(name);
        } else {
            out.push_str(with);
        }
        rest = after;
    }
    out.push_str(rest);
    out
}

/// The functions a function reaches, by name, itself included.
fn reached(prog: &Program, by_name: &HashMap<&str, usize>, root: usize) -> BTreeSet<usize> {
    let mut seen = BTreeSet::from([root]);
    let mut work = vec![root];
    let visit = |name: &str, seen: &mut BTreeSet<usize>, work: &mut Vec<usize>| {
        if let Some(&j) = by_name.get(name)
            && seen.insert(j)
        {
            work.push(j);
        }
    };
    while let Some(i) = work.pop() {
        let f = &prog.functions[i];
        let mut names: Vec<String> = Vec::new();
        let operand = |o: &Operand, names: &mut Vec<String>| {
            if let Operand::Const(Const::Fn(n)) = o {
                names.push(n.clone());
            }
        };
        for b in &f.blocks {
            for s in &b.stmts {
                match &s.kind {
                    StatementKind::Call { callee, args, .. } => {
                        args.iter().for_each(|a| operand(a, &mut names));
                        match callee {
                            Callee::Fn(n, _) => names.push(n.clone()),
                            Callee::Value(o) => operand(o, &mut names),
                            Callee::Trait(method, _) => {
                                // The trait's provided method, and each impl's.
                                names.push(method.clone());
                                let (tr, m) = method.rsplit_once('.').unwrap_or(("", method));
                                for imp in prog.impls.iter().filter(|im| im.trait_name == tr) {
                                    for (mn, target) in &imp.methods {
                                        if let (true, sisu_sir::ImplTarget::Fn(t)) = (mn == m, target) {
                                            names.push(t.clone());
                                        }
                                    }
                                }
                            }
                            Callee::Intrinsic(..) => {}
                        }
                    }
                    StatementKind::Assign(_, rv) => match rv {
                        Rvalue::Use(o) | Rvalue::Unary(_, o) => operand(o, &mut names),
                        Rvalue::Binary(_, a, c) => {
                            operand(a, &mut names);
                            operand(c, &mut names);
                        }
                        Rvalue::Aggregate(_, ops) => ops.iter().for_each(|o| operand(o, &mut names)),
                        Rvalue::Closure(n, _, ops) => {
                            names.push(n.clone());
                            ops.iter().for_each(|o| operand(o, &mut names));
                        }
                        _ => {}
                    },
                    _ => {}
                }
            }
            if let Terminator::Switch { value, .. } = &b.term {
                operand(value, &mut names);
            }
        }
        for n in names {
            visit(&n, &mut seen, &mut work);
        }
    }
    seen
}

/// The cache key of test `index`; `None` for a test that is never cached
/// (one that takes the real `System`). `salt` holds the options that decide
/// a run and the stored snapshots.
pub fn key(prog: &Program, index: usize, salt: &str) -> Option<u64> {
    let (name, function) = prog.tests.get(index)?;
    let by_name: HashMap<&str, usize> =
        prog.functions.iter().enumerate().map(|(i, f)| (f.name.as_str(), i)).collect();
    let &root = by_name.get(function.as_str())?;
    if sisu_sir::takes_real_system(&prog.functions[root]) {
        return None;
    }
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    fnv(&mut h, crate::replay::TOOLCHAIN.as_bytes());
    fnv(&mut h, salt.as_bytes());
    fnv(&mut h, name.as_bytes());
    // The declarations, without the program's entry and list of tests.
    let decls = Program {
        sources: Vec::new(),
        entry: None,
        exports: Vec::new(),
        types: prog.types.clone(),
        intrinsics: prog.intrinsics.clone(),
        functions: Vec::new(),
        tests: Vec::new(),
        test_tags: Vec::new(),
        benches: Vec::new(),
        trait_methods: prog.trait_methods.clone(),
        impls: prog.impls.clone(),
    };
    fnv(&mut h, sisu_sir::write::write(&decls).as_bytes());
    // What runs without a call: drop glue, `Drop`s and invariants, and the
    // fakes a test that names capabilities receives.
    let mut texts: BTreeMap<String, String> = BTreeMap::new();
    let mut roots = vec![root];
    for (i, f) in prog.functions.iter().enumerate() {
        let implicit = matches!(f.kind, FnKind::Glue | FnKind::Drop)
            || f.name.ends_with(".__invariant")
            || f.name.ends_with(".__check_invariant")
            || matches!(f.name.as_str(), "System.fake" | "System.sim" | "Net.fake");
        if implicit {
            roots.push(i);
        }
    }
    for r in roots {
        for i in reached(prog, &by_name, r) {
            let f = &prog.functions[i];
            // Another test's function is not this test's code.
            if f.name.starts_with("test#") && i != root && !f.name.starts_with(&format!("{function}#")) {
                continue;
            }
            let text = renamed(&without_positions(&sisu_sir::write::function(f)), function, "test#");
            texts.insert(renamed(&f.name, function, "test#"), text);
        }
    }
    for (n, t) in &texts {
        fnv(&mut h, n.as_bytes());
        fnv(&mut h, t.as_bytes());
    }
    Some(h)
}

/// A test that passed, as the cache keeps it: what it printed to standard
/// output and to standard error, which a cached test prints again.
pub type Entry = (String, String);

/// The tests that passed, by key, read from the cache file (none when
/// there is none, or it does not read): one JSON object a line.
pub fn read(path: &std::path::Path) -> BTreeMap<u64, Entry> {
    let mut out = BTreeMap::new();
    for line in std::fs::read_to_string(path).unwrap_or_default().lines() {
        let Ok(j) = crate::json::parse(line) else { continue };
        let key = j.get("key").and_then(|k| k.as_str()).and_then(|k| u64::from_str_radix(k, 16).ok());
        let text = |f: &str| j.get(f).and_then(|v| v.as_str()).unwrap_or("").to_string();
        if let Some(key) = key {
            out.insert(key, (text("out"), text("err")));
        }
    }
    out
}

/// Writes the tests that passed; a cache that cannot be written is no
/// cache, and nothing fails for it.
pub fn write(path: &std::path::Path, entries: &BTreeMap<u64, Entry>) {
    use crate::json::Json;
    let mut text = String::new();
    for (key, (out, err)) in entries {
        let j = Json::obj(vec![
            ("key", Json::str(format!("{key:016x}"))),
            ("out", Json::str(out.clone())),
            ("err", Json::str(err.clone())),
        ]);
        text.push_str(&j.write());
        text.push('\n');
    }
    if let Some(dir) = path.parent()
        && !dir.as_os_str().is_empty()
    {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(path, text);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positions_and_numbers_are_left_out() {
        assert_eq!(
            without_positions("fn f(0) source 3 @1:2-3:4 {\n    x @5:6-5:7\n"),
            "fn f(0)  {\n    x \n"
        );
        assert_eq!(
            renamed("test#3 test#3#closure0 test#30", "test#3", "test#"),
            "test# test##closure0 test#30"
        );
    }
}
