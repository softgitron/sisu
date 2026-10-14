//! `sisu explain-borrow FILE:LINE` (10, "Language server": the "explain
//! this borrow" command): every loan live on a line, with where it starts,
//! the named locals holding it, and where it is last used.

use crate::loans::{self, LoanReport};
use crate::types::Types;
use sisu_sir::Program;

/// The loans live on a line of one function.
#[derive(Clone, Debug)]
pub struct FunctionLoans {
    pub function: String,
    pub source: u32,
    pub loans: Vec<LoanReport>,
}

/// Every function with a statement on `line` of source `source`, and the
/// loans live there. `p` is the checked (elaborated) program.
pub fn live_at(p: &Program, source: u32, line: u32) -> Vec<FunctionLoans> {
    let types = Types::new(p);
    let mut out = Vec::new();
    for f in &p.functions {
        if f.source != source {
            continue;
        }
        let on_line = f
            .blocks
            .iter()
            .any(|b| !b.cleanup && b.stmts.iter().any(|s| s.loc.is_some_and(|l| l.line == line)));
        if !on_line {
            continue;
        }
        let a = loans::Analysis::run(p, f, &types);
        let loans = a.live_at_line(p, f, &types, line);
        out.push(FunctionLoans { function: loans::display_fn(&f.name), source: f.source, loans });
    }
    out
}
