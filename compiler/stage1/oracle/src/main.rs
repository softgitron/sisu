//! `stage1-oracle ast OUT LIST`: for the `n`th file (from 1) that LIST
//! names, one per line, writes `OUT/n.ast`: the module's docs, its broken
//! names, and each item of stage 0's AST (`ast::build::module`) on a line
//! of its own, in Rust's `Debug` notation, which stage 1's
//! `syntax.ast.dump` writes too.

use std::error::Error;
use std::fmt::Write as _;

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    let [_, command, out_dir, list] = args.as_slice() else {
        return Err("usage: stage1-oracle ast OUT LIST".into());
    };
    if command != "ast" {
        return Err(format!("unknown command `{command}`").into());
    }
    let list = std::fs::read_to_string(list)?;
    for (i, path) in list.lines().filter(|l| !l.is_empty()).enumerate() {
        let text = std::fs::read_to_string(path)?;
        let parsed = sisu_parser::parse(sisu_diag::FileId(0), &text);
        let module = sisu_syntax::ast::build::module(&parsed.tree);
        let mut out = String::new();
        writeln!(out, "docs {:?}", module.docs)?;
        writeln!(out, "broken {:?}", module.broken_names)?;
        for item in &module.items {
            writeln!(out, "{item:?}")?;
        }
        std::fs::write(format!("{out_dir}/{}.ast", i + 1), out)?;
    }
    Ok(())
}
