//! Conditional compilation of items (08, "Conditional compilation":
//! "`@cfg` on items and `if comptime` in bodies"; decision 243). An item
//! (a declaration, an import, a test, an item of a suite or of an `extern`
//! block, or a method of an impl or a trait) whose `@cfg(...)` does not
//! hold is left out before the program is resolved, as if it were not
//! written; one whose `@cfg` holds is kept as written. A condition is
//! `test` (the program is built with its tests), `feature="NAME"` (its
//! package turns the feature on), `not` and a condition, or `all(...)` and
//! `any(...)` of conditions (so `not any(feature="tls")` is a feature
//! that is off); several in one `@cfg` must all hold. A program built
//! without its tests leaves out its `test`, `suite` and `bench` items too.

use sisu_diag::TextRange;
use sisu_syntax::ast::{self, Expr, ExprKind, FnDef, Item, ItemKind};

/// A package's features: those its `[features]` declares, and those that
/// are on in this build (09).
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Features {
    pub declared: Vec<String>,
    pub on: Vec<String>,
}

/// A `@cfg` that is not a condition (`E0266`), or a `feature="NAME"`
/// whose package declares no such feature (`W0006`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Problem {
    NotACondition(TextRange),
    UnknownFeature(TextRange, String),
}

struct Filter<'a> {
    test: bool,
    features: Option<&'a Features>,
    problems: Vec<Problem>,
}

impl Filter<'_> {
    fn holds(&mut self, e: &Expr, name: Option<&ast::Name>) -> bool {
        match (name.map(|n| n.text.as_str()), &e.kind) {
            (Some("feature"), ExprKind::Str(parts)) => {
                let f = ast::build::plain_text(parts);
                if !self.features.is_some_and(|fs| fs.declared.contains(&f)) {
                    self.problems.push(Problem::UnknownFeature(e.range, f.clone()));
                }
                self.features.is_some_and(|fs| fs.on.contains(&f))
            }
            (None, ExprKind::Name(n)) if n == "test" => self.test,
            (None, ExprKind::Unary { op: ast::UnOp::Not, expr }) => !self.holds(expr, None),
            (None, ExprKind::Paren(inner)) => self.holds(inner, None),
            (None, ExprKind::Call { callee, args, trailing: None }) => {
                let all = match &callee.kind {
                    ExprKind::Name(n) if n == "all" => true,
                    ExprKind::Name(n) if n == "any" => false,
                    _ => {
                        self.problems.push(Problem::NotACondition(e.range));
                        return true;
                    }
                };
                let each: Vec<bool> = args.iter().map(|a| self.holds(&a.value, a.name.as_ref())).collect();
                if all { each.iter().all(|b| *b) } else { each.iter().any(|b| *b) }
            }
            _ => {
                self.problems.push(Problem::NotACondition(e.range));
                true
            }
        }
    }

    /// Whether every `@cfg` among `attrs` holds. Each condition is looked
    /// at, so every problem is reported.
    fn kept(&mut self, attrs: &[ast::Attr]) -> bool {
        let mut keep = true;
        for a in attrs.iter().filter(|a| a.name == "cfg") {
            if a.args.is_empty() {
                // `@cfg` itself: the attribute's range runs to the next line.
                let at = TextRange::new(a.range.start, a.range.start + 1 + a.name.len() as u32);
                self.problems.push(Problem::NotACondition(at));
            }
            for arg in &a.args {
                keep &= self.holds(&arg.value, arg.name.as_ref());
            }
        }
        keep
    }

    fn methods(&mut self, methods: &mut Vec<FnDef>) {
        let mut out = Vec::new();
        for m in methods.drain(..) {
            if self.kept(&m.attrs) {
                out.push(m);
            }
        }
        *methods = out;
    }

    fn items(&mut self, items: &[Item]) -> Vec<Item> {
        let mut out = Vec::new();
        for i in items {
            if !self.kept(&i.attrs) {
                continue;
            }
            if !self.test && matches!(i.kind, ItemKind::Test(_) | ItemKind::Suite(_) | ItemKind::Bench(_)) {
                continue;
            }
            let mut i = i.clone();
            match &mut i.kind {
                ItemKind::Suite(s) => s.items = self.items(&s.items),
                ItemKind::Extern(xs) => *xs = self.items(xs),
                ItemKind::Impl(imp) => self.methods(&mut imp.methods),
                ItemKind::Trait(t) => self.methods(&mut t.methods),
                _ => {}
            }
            out.push(i);
        }
        out
    }
}

fn has_cfg(attrs: &[ast::Attr]) -> bool {
    attrs.iter().any(|a| a.name == "cfg")
}

/// Whether `items` have anything to leave out or look at: a `@cfg`, or a
/// test when the program is built without its tests.
fn affected(items: &[Item], test: bool) -> bool {
    items.iter().any(|i| {
        has_cfg(&i.attrs)
            || match &i.kind {
                ItemKind::Test(_) | ItemKind::Suite(_) | ItemKind::Bench(_) if !test => true,
                ItemKind::Suite(s) => affected(&s.items, test),
                ItemKind::Extern(xs) => affected(xs, test),
                ItemKind::Impl(imp) => imp.methods.iter().any(|m| has_cfg(&m.attrs)),
                ItemKind::Trait(t) => t.methods.iter().any(|m| has_cfg(&m.attrs)),
                _ => false,
            }
    })
}

/// The module as this build compiles it, or `None` when that is the module
/// as written; and the problems its `@cfg`s have. `test` is whether the
/// program is built with its tests, `features` its package's features.
pub fn filter(
    m: &ast::Module,
    test: bool,
    features: Option<&Features>,
) -> (Option<ast::Module>, Vec<Problem>) {
    if !affected(&m.items, test) {
        return (None, Vec::new());
    }
    let mut f = Filter { test, features, problems: Vec::new() };
    let items = f.items(&m.items);
    (Some(ast::Module { items, docs: m.docs.clone(), broken_names: m.broken_names.clone() }), f.problems)
}
