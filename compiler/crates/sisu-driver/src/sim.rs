//! The guarantee of the `sim` profile (17, "Simulation and replay"): a
//! `sim` build fails to compile if reachable code could escape the
//! simulation. Only a foreign call can (or `unsafe` code in a `[trust]`ed
//! package, which stage 0 does not have): the world reaches a program
//! through a capability, and every root capability of `System.sim` is
//! simulated. A foreign call lowers to the intrinsic `extern.LIBRARY.NAME`
//! (Phase 8, foreign functions); one with a simulation model is allowed.
//!
//! Stage 0 has no foreign functions, so no program it accepts reaches
//! one; the check runs on every `sisu test --sim` all the same, so the
//! guarantee holds from the first program that can.

use sisu_sir as sir;
use std::collections::HashSet;

/// The prefix of the intrinsics foreign calls lower to.
pub const EXTERN_PREFIX: &str = "extern.";

/// A foreign call reachable from a root: the function that makes it, the
/// callee (`sqlite.exec`), and where the call is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Escape {
    pub function: String,
    pub callee: String,
    pub loc: Option<sir::Loc>,
}

/// The foreign calls with no simulation model in `models` that the
/// program makes. Lowering keeps exactly the functions the entry, the
/// tests and the root module's functions reach (through calls, function
/// values and the impls of the traits they call through), so every
/// function of the SIR is reachable code.
pub fn escapes(prog: &sir::Program, models: &HashSet<String>) -> Vec<Escape> {
    let mut found = Vec::new();
    for f in &prog.functions {
        for b in &f.blocks {
            for s in &b.stmts {
                if let sir::StatementKind::Call { callee: sir::Callee::Intrinsic(n, _), .. } = &s.kind
                    && let Some(c) = n.strip_prefix(EXTERN_PREFIX)
                    && !models.contains(c)
                {
                    found.push(Escape { function: f.name.clone(), callee: c.to_string(), loc: s.loc });
                }
            }
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROGRAM: &str = r#"sir 0
source 0 "db.sisu"

intrinsic extern.sqlite.exec
intrinsic extern.clock.ticks

fn query(1) source 0 @1:5-1:10 {
    let _0: int
    let _1: str "sql" @1:11-1:19
  bb0:
    _0 = call intrinsic extern.sqlite.exec(copy _1) @2:12-2:30
    return
}

fn ticks(0) source 0 @4:5-4:10 {
    let _0: int
  bb0:
    _0 = call intrinsic extern.clock.ticks() @5:12-5:30
    return
}

fn test#0(0) test source 0 @7:1-7:5 {
    let _0: ()
    let _1: int
    let _2: str
    let _3: int
  bb0:
    _2 = const "select 1"
    _1 = call fn query(copy _2) @8:9-8:25
    _3 = call fn ticks() @9:9-9:16
    return
}
"#;

    #[test]
    fn a_foreign_call_without_a_model_escapes_the_simulation() {
        let prog = sisu_sir::read(PROGRAM).expect("the program reads");
        let none = escapes(&prog, &HashSet::new());
        let callees: Vec<&str> = none.iter().map(|e| e.callee.as_str()).collect();
        assert_eq!(callees, ["sqlite.exec", "clock.ticks"]);
        assert_eq!(none[0].function, "query");
        let models: HashSet<String> = ["clock.ticks".to_string()].into();
        let one = escapes(&prog, &models);
        assert_eq!(one.len(), 1);
        assert_eq!(one[0].callee, "sqlite.exec");
        let all: HashSet<String> = ["clock.ticks".to_string(), "sqlite.exec".to_string()].into();
        assert!(escapes(&prog, &all).is_empty());
    }
}
