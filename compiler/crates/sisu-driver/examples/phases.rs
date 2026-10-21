#![allow(clippy::unwrap_used, clippy::expect_used)]
use sisu_driver::Db;
use std::time::Instant;

fn edit_first_body(text: &str, k: usize) -> Option<String> {
    let parse = sisu_parser::parse(sisu_diag::FileId(0), text);
    let module = sisu_syntax::ast::build::module(&parse.tree);
    let mut starts = Vec::new();
    for item in &module.items {
        if let sisu_syntax::ast::ItemKind::Fn(f) = &item.kind
            && let Some(b) = &f.body
            && let Some(s) = b.stmts.first()
        {
            starts.push(s.range.start as usize);
        }
    }
    let at = *starts.get(k)?;
    let line_start = text[..at].rfind('\n').map(|i| i + 1).unwrap_or(0);
    let indent = &text[line_start..at];
    Some(format!("{}{indent}pass\n{}", &text[..line_start], &text[line_start..]))
}

fn main() {
    let path = std::env::args().nth(1).expect("file");
    let text = std::fs::read_to_string(&path).unwrap();
    let mut db = Db::new();
    db.set_file(&path, &text);
    let t = Instant::now();
    let d0 = db.diagnostics(&path);
    println!("cold: {:?} ({} diagnostics) {:?}", t.elapsed(), d0.len(), db.counters);
    if std::env::var("WARM").is_ok() {
        for round in 0..10 {
            let Some(edited) = edit_first_body(&text, round % 2) else { break };
            db.set_file(&path, &edited);
            let t = Instant::now();
            let _ = db.diagnostics(&path);
            println!("warm {round}: {:?} {:?}", t.elapsed(), db.counters);
        }
        return;
    }
    for k in 0..4 {
        let Some(edited) = edit_first_body(&text, k) else { break };
        db.set_file(&path, &edited);
        let t = Instant::now();
        let d1 = db.diagnostics(&path);
        let warm = t.elapsed();
        let sir1 = db.sir(&path).map(|s| sisu_sir::write::write(&s));
        let mut fresh = Db::new();
        fresh.set_file(&path, &edited);
        let t = Instant::now();
        let d2 = fresh.diagnostics(&path);
        let cold = t.elapsed();
        let sir2 = fresh.sir(&path).map(|s| sisu_sir::write::write(&s));
        println!(
            "edit {k}: warm {:?} cold {:?} same diags {} same sir {} {:?}",
            warm,
            cold,
            d1 == d2,
            sir1 == sir2,
            db.counters
        );
    }
}
