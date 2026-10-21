//! What a test declaration declares (17): a parametrised test is a test
//! for each case, and a test's `@tag`s say which runs select it.
//!
//! `test "name" for x, y in [(1, 2), (3, 4)]:` over a list written out is
//! one test per element, named after it (`name (1, 2)`), whose body
//! starts by binding the pattern to the element; each case is reported,
//! filtered and run on its own. Over a list computed some other way, it is
//! one test that runs the body for each element in turn.

use crate::collect::Resolver;
use crate::program::{DefId, ModuleId};
use sisu_syntax::ast::{self, ExprKind, Stmt, StmtKind};

/// The tests a declaration named `name` declares, with their names.
pub(crate) fn cases(name: &str, t: &ast::TestDef, source: &str) -> Vec<(String, ast::TestDef)> {
    let Some((pat, list)) = &t.cases else { return vec![(name.to_string(), t.clone())] };
    let with_body = |name: String, first: Stmt| {
        let body = t.body.as_ref().map(|b| {
            let mut stmts = vec![first];
            stmts.extend(b.stmts.iter().cloned());
            std::sync::Arc::new(ast::Block { node: b.node, range: b.range, stmts })
        });
        (
            name,
            ast::TestDef {
                name: t.name.clone(),
                name_range: t.name_range,
                cases: None,
                params: t.params.clone(),
                body,
            },
        )
    };
    match &list.kind {
        ExprKind::List(elems) => elems
            .iter()
            .map(|e| {
                let shown = source.get(e.range.start as usize..e.range.end as usize).unwrap_or("?");
                let bind = Stmt {
                    node: pat.node,
                    range: e.range,
                    kind: StmtKind::Let {
                        mutable: false,
                        pat: pat.clone(),
                        ty: None,
                        init: Some(e.clone()),
                        else_: None,
                    },
                };
                with_body(format!("{name} {shown}"), bind)
            })
            .collect(),
        _ => {
            // `for PAT in LIST:` around the body.
            let body = t.body.as_deref().cloned().unwrap_or(ast::Block {
                node: list.node,
                range: list.range,
                stmts: Vec::new(),
            });
            let looped = Stmt {
                node: list.node,
                range: body.range,
                kind: StmtKind::For { pat: pat.clone(), mode: ast::Mode::Borrow, iter: list.clone(), body },
            };
            let mut one = with_body(name.to_string(), looped);
            if let Some(b) = &mut one.1.body {
                std::sync::Arc::make_mut(b).stmts.truncate(1);
            }
            vec![one]
        }
    }
}

impl Resolver<'_> {
    /// Records the `@tag("...")`s on a test.
    pub(crate) fn test_tags(&mut self, def: DefId, attrs: &[ast::Attr], module: ModuleId) {
        let source = self.sources.get(self.file(module)).map(|f| f.text.clone()).unwrap_or_default();
        let mut tags = Vec::new();
        for a in attrs.iter().filter(|a| a.name == "tag") {
            for x in &a.args {
                let text = source.get(x.range.start as usize..x.range.end as usize).unwrap_or("");
                tags.push(text.trim_matches('"').to_string());
            }
        }
        if !tags.is_empty() {
            self.prog.test_tags.insert(def, tags);
        }
    }
}
