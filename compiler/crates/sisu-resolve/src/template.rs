//! A derive evaluated at compile time (08, "Derive"; 18, Phase 4).
//!
//! A trait's `comptime def derive(T: type) -> impl Trait for T` returns the
//! impl the trait derives for `T`, written `return impl Trait for T:` with
//! methods. Stage 0 evaluates such a function by unfolding it: the code
//! that runs at compile time (`comptime for`, `comptime if`, `if comptime`,
//! the same two among a `match`'s arms, and every expression made only of
//! compile-time values) is evaluated, and the rest is the impl's code,
//! written out as source text with the compile-time values in it as
//! literals. The text is declared beside the type and checked like any
//! impl, so an error in the derived code is reported as in any other.
//!
//! The compile-time values are the type `T`, descriptors of its fields and
//! variants, lists and pairs of them, integers, text and booleans.
//! Reflection is the compile-time module `reflect`: `name_of(T)` (the
//! type's name), `is_struct(T)`, `is_enum(T)`, `fields_of(T)` (a struct's
//! fields) and `variants_of(T)` (an enum's variants). A field has `name`
//! (for a positional variant field, its position), `index` and `named`; a
//! variant has `name`, `index` and `fields`. `x.[field]` is that field of
//! `x`; a variant's field is reachable only inside an arm `case V(..)` that
//! `comptime for V in reflect.variants_of(T)` made for a `match x`, where
//! it is the arm's binding. Compile-time code may also use `enumerate`,
//! `len`, integer arithmetic, comparisons, `and`, `or` and `not`.

use sisu_diag::TextRange;
use sisu_syntax::ast::{
    self, ArmBody, ArmGen, BinOp, CmpOp, CondPart, Expr, ExprKind, Mode, PatKind, Stmt, StmtKind, StrPart,
    UnOp,
};

/// The shape of the type being derived for.
pub(crate) enum Body {
    /// Field names.
    Struct(Vec<String>),
    /// Variant names, each with its field names (`None` for positional).
    Enum(Vec<(String, Vec<Option<String>>)>),
}

/// A compile-time value.
#[derive(Clone, Debug, PartialEq)]
enum Value {
    /// The type the derive is for.
    Type,
    Field { name: String, index: usize, variant: Option<usize>, named: bool },
    Variant { name: String, index: usize, fields: Vec<Value> },
    Str(String),
    Int(i64),
    Bool(bool),
    List(Vec<Value>),
    Tuple(Vec<Value>),
}

/// Why the unfolding stopped, and where in the template.
pub(crate) type Failure = (TextRange, String);

type R<T> = Result<T, Failure>;

/// The bindings of an arm `case V(..)` made for a `match` on `key`.
struct ArmScope {
    key: String,
    variant: usize,
    names: Vec<String>,
}

/// The type a derive is for.
pub(crate) struct Target<'a> {
    pub name: &'a str,
    /// The type as the impl names it (`Pair[A, B]`).
    pub written: &'a str,
    pub body: &'a Body,
}

struct Unfold<'a> {
    source: &'a str,
    target: &'a Target<'a>,
    /// The template's type parameter (`T`).
    param: String,
    env: Vec<(String, Value)>,
    arms: Vec<ArmScope>,
}

/// Checks that `f` is a trait's derive template: `comptime def derive(T:
/// type) -> impl Trait for T` whose body is `return impl Trait for T:`.
/// Returns the type parameter's name and the impl.
pub(crate) fn template_parts(f: &ast::FnDef) -> Option<(String, &ast::ImplDef)> {
    let [p] = f.params.as_slice() else { return None };
    let ast::ParamKind::Named { name, ty, .. } = &p.kind else { return None };
    let ast::TypeKind::Path { segments, args } = &ty.kind else { return None };
    if !args.is_empty() || segments.len() != 1 || segments[0].text != "type" {
        return None;
    }
    let body = f.body.as_ref()?;
    let mut imp = None;
    for s in &body.stmts {
        match &s.kind {
            StmtKind::ReturnImpl(i) if imp.is_none() => imp = Some(&**i),
            StmtKind::Pass => {}
            _ => return None,
        }
    }
    Some((name.text.clone(), imp?))
}

/// The source of the impl that `template` (from the file whose text is
/// `source`) derives for `target`, under the impl header `head`.
pub(crate) fn unfold(template: &ast::FnDef, source: &str, head: &str, target: &Target) -> Result<String, Failure> {
    let (param, imp) =
        template_parts(template).ok_or((template.name.range, "it is not a derive template".to_string()))?;
    let mut u = Unfold { source, target, param, env: Vec::new(), arms: Vec::new() };
    u.env.push((u.param.clone(), Value::Type));
    let mut out = format!("\n{head}");
    if !imp.assoc_types.is_empty() {
        let r = imp.assoc_types[0].range;
        return Err((r, "a derived impl cannot bind associated types in stage 0".into()));
    }
    if imp.methods.is_empty() {
        out.push_str("    pass\n");
    }
    for m in &imp.methods {
        let Some(body) = &m.body else {
            return Err((m.name.range, "a derived method needs a body".into()));
        };
        let sig = slice(source, TextRange::new(m.range.start, body.range.start)).trim_end();
        line(&mut out, 1, &u.substitute_param(sig));
        let n = u.stmts(&body.stmts, 2, &mut out)?;
        if n == 0 {
            line(&mut out, 2, "pass");
        }
    }
    Ok(out)
}

fn slice(source: &str, r: TextRange) -> &str {
    source.get(r.start as usize..r.end as usize).unwrap_or("")
}

fn line(out: &mut String, depth: usize, text: &str) {
    for _ in 0..depth {
        out.push_str("    ");
    }
    out.push_str(text);
    out.push('\n');
}

/// A value as a Sisu literal.
fn literal(v: &Value) -> Option<String> {
    match v {
        Value::Str(s) => Some(format!("\"{}\"", escape(s))),
        Value::Int(i) => Some(i.to_string()),
        Value::Bool(b) => Some(if *b { "True" } else { "False" }.into()),
        _ => None,
    }
}

/// Text as it is written inside a string literal.
fn escape(s: &str) -> String {
    let mut q = String::new();
    for c in s.chars() {
        match c {
            '"' => q.push_str("\\\""),
            '\\' => q.push_str("\\\\"),
            '{' => q.push_str("{{"),
            '}' => q.push_str("}}"),
            '\n' => q.push_str("\\n"),
            c => q.push(c),
        }
    }
    q
}

fn describe(v: &Value) -> &'static str {
    match v {
        Value::Type => "a type",
        Value::Field { .. } => "a field",
        Value::Variant { .. } => "a variant",
        Value::Str(_) => "text",
        Value::Int(_) => "an integer",
        Value::Bool(_) => "a boolean",
        Value::List(_) => "a list",
        Value::Tuple(_) => "a tuple",
    }
}

impl Unfold<'_> {
    fn lookup(&self, n: &str) -> Option<&Value> {
        self.env.iter().rev().find(|(k, _)| k == n).map(|(_, v)| v)
    }

    /// Replaces the template's type parameter by the type, as a word.
    fn substitute_param(&self, text: &str) -> String {
        let mut out = String::new();
        let mut word = String::new();
        let flush = |word: &mut String, out: &mut String| {
            if *word == self.param {
                out.push_str(self.target.written);
            } else {
                out.push_str(word);
            }
            word.clear();
        };
        for c in text.chars() {
            if c.is_alphanumeric() || c == '_' {
                word.push(c);
            } else {
                flush(&mut word, &mut out);
                out.push(c);
            }
        }
        flush(&mut word, &mut out);
        out
    }

    // ------------------------------------------------------------ values

    /// Whether the expression mentions anything evaluated at compile time.
    fn depends(&self, e: &Expr) -> bool {
        let mut found = false;
        walk(e, &mut |x| match &x.kind {
            ExprKind::Name(n) => found |= self.lookup(n).is_some() || n == "reflect",
            ExprKind::FieldOf { .. } | ExprKind::Comptime(_) => found = true,
            _ => {}
        });
        found
    }

    /// Whether the expression is made only of compile-time values.
    fn is_static(&self, e: &Expr) -> bool {
        match &e.kind {
            ExprKind::Name(n) => self.lookup(n).is_some() || n == "True" || n == "False",
            ExprKind::Int { .. } => true,
            ExprKind::Str(parts) => parts.iter().all(|p| match p {
                StrPart::Text(_) => true,
                StrPart::Interp { expr, spec, debug, .. } => spec.is_none() && debug.is_none() && self.is_static(expr),
            }),
            ExprKind::Field { base, .. } => self.is_static(base),
            ExprKind::Call { callee, args, trailing: None } => {
                let known = match &callee.kind {
                    ExprKind::Field { base, .. } => matches!(&base.kind, ExprKind::Name(n) if n == "reflect"),
                    ExprKind::Name(n) => matches!(n.as_str(), "enumerate" | "len") && self.lookup(n).is_none(),
                    _ => false,
                };
                known && args.iter().all(|a| a.name.is_none() && self.is_static(&a.value))
            }
            ExprKind::Binary { lhs, rhs, .. } => self.is_static(lhs) && self.is_static(rhs),
            ExprKind::Compare { first, rest } => self.is_static(first) && rest.iter().all(|(_, x)| self.is_static(x)),
            ExprKind::Unary { expr, .. } | ExprKind::Paren(expr) | ExprKind::Comptime(expr) => self.is_static(expr),
            ExprKind::Tuple(xs) | ExprKind::List(xs) => xs.iter().all(|x| self.is_static(x)),
            _ => false,
        }
    }

    fn eval(&self, e: &Expr) -> R<Value> {
        let fail = |why: String| Err((e.range, why));
        match &e.kind {
            ExprKind::Name(n) => match self.lookup(n) {
                Some(v) => Ok(v.clone()),
                None if n == "True" => Ok(Value::Bool(true)),
                None if n == "False" => Ok(Value::Bool(false)),
                None => fail(format!("`{n}` is not known at compile time")),
            },
            ExprKind::Int { value: Some(v), .. } => match i64::try_from(*v) {
                Ok(v) => Ok(Value::Int(v)),
                Err(_) => fail("the integer does not fit in `int`".into()),
            },
            ExprKind::Str(parts) => {
                let mut s = String::new();
                for p in parts {
                    match p {
                        StrPart::Text(t) => s.push_str(t),
                        StrPart::Interp { expr, .. } => match self.eval(expr)? {
                            Value::Str(t) => s.push_str(&t),
                            Value::Int(i) => s.push_str(&i.to_string()),
                            Value::Bool(b) => s.push_str(if b { "True" } else { "False" }),
                            other => return fail(format!("{} cannot be written into text", describe(&other))),
                        },
                    }
                }
                Ok(Value::Str(s))
            }
            ExprKind::Paren(x) | ExprKind::Comptime(x) => self.eval(x),
            ExprKind::Field { base, name } => {
                let b = self.eval(base)?;
                let got = match (&b, name.text.as_str()) {
                    (Value::Field { name, .. }, "name") => Some(Value::Str(name.clone())),
                    (Value::Field { index, .. }, "index") => Some(Value::Int(*index as i64)),
                    (Value::Field { named, .. }, "named") => Some(Value::Bool(*named)),
                    (Value::Variant { name, .. }, "name") => Some(Value::Str(name.clone())),
                    (Value::Variant { index, .. }, "index") => Some(Value::Int(*index as i64)),
                    (Value::Variant { fields, .. }, "fields") => Some(Value::List(fields.clone())),
                    _ => None,
                };
                got.map_or_else(|| fail(format!("{} has no `{}`", describe(&b), name.text)), Ok)
            }
            ExprKind::Call { callee, args, .. } => {
                let vals: Vec<Value> = args.iter().map(|a| self.eval(&a.value)).collect::<R<_>>()?;
                let one = || match vals.as_slice() {
                    [v] => Ok(v.clone()),
                    _ => Err((e.range, "this takes one argument".to_string())),
                };
                match &callee.kind {
                    ExprKind::Field { name, .. } => {
                        if one()? != Value::Type {
                            return fail(format!("`reflect.{}` takes the type `{}`", name.text, self.param));
                        }
                        self.reflect(&name.text).ok_or((e.range, format!("`reflect` has no `{}`", name.text)))
                    }
                    ExprKind::Name(n) if n == "enumerate" => match one()? {
                        Value::List(xs) => Ok(Value::List(
                            xs.into_iter().enumerate().map(|(i, x)| Value::Tuple(vec![Value::Int(i as i64), x])).collect(),
                        )),
                        other => fail(format!("`enumerate` of {}", describe(&other))),
                    },
                    ExprKind::Name(n) if n == "len" => match one()? {
                        Value::List(xs) => Ok(Value::Int(xs.len() as i64)),
                        Value::Str(s) => Ok(Value::Int(s.chars().count() as i64)),
                        other => fail(format!("`len` of {}", describe(&other))),
                    },
                    _ => fail("only `reflect`, `enumerate` and `len` can be called at compile time here".into()),
                }
            }
            ExprKind::Unary { op, expr } => match (op, self.eval(expr)?) {
                (UnOp::Not, Value::Bool(b)) => Ok(Value::Bool(!b)),
                (UnOp::Neg, Value::Int(i)) => i.checked_neg().map(Value::Int).ok_or((e.range, "overflow".into())),
                (_, v) => fail(format!("`{}` of {}", op.text(), describe(&v))),
            },
            ExprKind::Binary { op, lhs, rhs } => {
                let (a, b) = (self.eval(lhs)?, self.eval(rhs)?);
                let int = |r: Option<i64>| r.map(Value::Int).ok_or((e.range, "overflow".to_string()));
                match (op, a, b) {
                    (BinOp::And, Value::Bool(x), Value::Bool(y)) => Ok(Value::Bool(x && y)),
                    (BinOp::Or, Value::Bool(x), Value::Bool(y)) => Ok(Value::Bool(x || y)),
                    (BinOp::Add, Value::Int(x), Value::Int(y)) => int(x.checked_add(y)),
                    (BinOp::Sub, Value::Int(x), Value::Int(y)) => int(x.checked_sub(y)),
                    (BinOp::Mul, Value::Int(x), Value::Int(y)) => int(x.checked_mul(y)),
                    (BinOp::Add, Value::Str(x), Value::Str(y)) => Ok(Value::Str(x + &y)),
                    (op, a, b) => fail(format!("`{}` of {} and {}", op.text(), describe(&a), describe(&b))),
                }
            }
            ExprKind::Compare { first, rest } => {
                let mut left = self.eval(first)?;
                for (op, x) in rest {
                    let right = self.eval(x)?;
                    let holds = match (op, &left, &right) {
                        (CmpOp::Eq, a, b) => a == b,
                        (CmpOp::Ne, a, b) => a != b,
                        (CmpOp::Lt, Value::Int(a), Value::Int(b)) => a < b,
                        (CmpOp::Le, Value::Int(a), Value::Int(b)) => a <= b,
                        (CmpOp::Gt, Value::Int(a), Value::Int(b)) => a > b,
                        (CmpOp::Ge, Value::Int(a), Value::Int(b)) => a >= b,
                        _ => return fail(format!("`{}` of {} and {}", op.text(), describe(&left), describe(&right))),
                    };
                    if !holds {
                        return Ok(Value::Bool(false));
                    }
                    left = right;
                }
                Ok(Value::Bool(true))
            }
            ExprKind::Tuple(xs) => Ok(Value::Tuple(xs.iter().map(|x| self.eval(x)).collect::<R<_>>()?)),
            ExprKind::List(xs) => Ok(Value::List(xs.iter().map(|x| self.eval(x)).collect::<R<_>>()?)),
            _ => fail("this cannot be evaluated at compile time".into()),
        }
    }

    /// `reflect.f(T)`.
    fn reflect(&self, f: &str) -> Option<Value> {
        let t = self.target;
        Some(match (f, t.body) {
            ("name_of", _) => Value::Str(t.name.to_string()),
            ("is_struct", b) => Value::Bool(matches!(b, Body::Struct(_))),
            ("is_enum", b) => Value::Bool(matches!(b, Body::Enum(_))),
            ("fields_of", Body::Struct(fields)) => Value::List(
                fields
                    .iter()
                    .enumerate()
                    .map(|(index, n)| Value::Field { name: n.clone(), index, variant: None, named: true })
                    .collect(),
            ),
            ("fields_of", Body::Enum(_)) => Value::List(Vec::new()),
            ("variants_of", Body::Enum(vs)) => Value::List(
                vs.iter()
                    .enumerate()
                    .map(|(vi, (name, fields))| Value::Variant {
                        name: name.clone(),
                        index: vi,
                        fields: fields
                            .iter()
                            .enumerate()
                            .map(|(index, n)| Value::Field {
                                name: n.clone().unwrap_or_else(|| index.to_string()),
                                index,
                                variant: Some(vi),
                                named: n.is_some(),
                            })
                            .collect(),
                    })
                    .collect(),
            ),
            ("variants_of", Body::Struct(_)) => Value::List(Vec::new()),
            _ => return None,
        })
    }

    fn truth(&self, e: &Expr) -> R<bool> {
        match self.eval(e)? {
            Value::Bool(b) => Ok(b),
            other => Err((e.range, format!("a condition must be a boolean, not {}", describe(&other)))),
        }
    }

    fn bind(&mut self, p: &ast::Pattern, v: Value) -> R<usize> {
        match (&p.kind, v) {
            (PatKind::Bind(n), v) => {
                self.env.push((n.text.clone(), v));
                Ok(1)
            }
            (PatKind::Ctor { path, args: None }, v) if path.len() == 1 => {
                self.env.push((path[0].text.clone(), v));
                Ok(1)
            }
            (PatKind::Wildcard, _) => Ok(0),
            (PatKind::Tuple(ps), Value::Tuple(vs)) if ps.len() == vs.len() => {
                let mut n = 0;
                for (p, v) in ps.iter().zip(vs) {
                    n += self.bind(p, v)?;
                }
                Ok(n)
            }
            (_, v) => Err((p.range, format!("this pattern cannot take {}", describe(&v)))),
        }
    }

    fn items(&self, e: &Expr) -> R<Vec<Value>> {
        match self.eval(e)? {
            Value::List(xs) => Ok(xs),
            other => Err((e.range, format!("`comptime for` needs a list, not {}", describe(&other)))),
        }
    }

    // ------------------------------------------------------------ text

    /// The text of `e` with its compile-time parts replaced.
    fn text(&self, e: &Expr) -> R<String> {
        let mut reps = Vec::new();
        self.replacements(e, &mut reps)?;
        Ok(self.apply(e.range, reps))
    }

    fn apply(&self, range: TextRange, mut reps: Vec<(TextRange, String)>) -> String {
        reps.sort_by_key(|(r, _)| r.start);
        let mut out = String::new();
        let mut at = range.start;
        for (r, t) in reps {
            if r.start < at {
                continue;
            }
            out.push_str(slice(self.source, TextRange::new(at, r.start)));
            out.push_str(&t);
            at = r.end;
        }
        out.push_str(slice(self.source, TextRange::new(at, range.end)));
        out
    }

    fn replacements(&self, e: &Expr, out: &mut Vec<(TextRange, String)>) -> R<()> {
        if !self.depends(e) {
            return Ok(());
        }
        if self.is_static(e) {
            let v = self.eval(e)?;
            let lit = literal(&v)
                .ok_or((e.range, format!("{} is a compile-time value with no run-time form", describe(&v))))?;
            out.push((e.range, lit));
            return Ok(());
        }
        match &e.kind {
            ExprKind::FieldOf { base, field } => {
                let f = self.eval(field)?;
                let base_text = self.text(base)?;
                let Value::Field { name, index, variant, .. } = f else {
                    return Err((field.range, format!("`x.[f]` needs a field, not {}", describe(&f))));
                };
                let text = match variant {
                    None => format!("{base_text}.{name}"),
                    Some(v) => match self.arms.iter().rev().find(|a| a.key == base_text && a.variant == v) {
                        Some(a) => a.names.get(index).cloned().unwrap_or_default(),
                        None => {
                            return Err((
                                e.range,
                                format!(
                                    "`{base_text}.[{}]` is a variant's field, reachable only in an arm `case V(..)` of a `match {base_text}`",
                                    slice(self.source, field.range)
                                ),
                            ));
                        }
                    },
                };
                out.push((e.range, text));
            }
            ExprKind::Str(parts) => {
                for p in parts {
                    if let StrPart::Interp { expr, spec, debug, range } = p
                        && self.depends(expr)
                    {
                        if spec.is_none() && debug.is_none() && self.is_static(expr) {
                            let v = self.eval(expr)?;
                            let t = match v {
                                Value::Str(s) => s,
                                Value::Int(i) => i.to_string(),
                                Value::Bool(b) => if b { "True" } else { "False" }.to_string(),
                                other => {
                                    return Err((
                                        expr.range,
                                        format!("{} cannot be written into text", describe(&other)),
                                    ));
                                }
                            };
                            out.push((*range, escape(&t)));
                        } else {
                            self.replacements(expr, out)?;
                        }
                    }
                }
            }
            ExprKind::Closure(_) | ExprKind::Match { .. } | ExprKind::Comprehension { .. } => {
                return Err((e.range, "compile-time values cannot be used inside this expression".into()));
            }
            _ => {
                let mut kids = Vec::new();
                children(e, &mut kids);
                for k in kids {
                    self.replacements(k, out)?;
                }
            }
        }
        Ok(())
    }

    /// A statement's own text, re-indented to `depth`.
    fn copy(&self, range: TextRange, reps: Vec<(TextRange, String)>, depth: usize, out: &mut String) {
        let col = self.column(range.start);
        let text = self.apply(range, reps);
        for (i, l) in text.trim_end().lines().enumerate() {
            if i == 0 {
                line(out, depth, l.trim_start());
            } else {
                let lead = l.len() - l.trim_start().len();
                let mut s = " ".repeat(lead.saturating_sub(col));
                s.push_str(l.trim_start());
                line(out, depth, &s);
            }
        }
    }

    fn column(&self, at: u32) -> usize {
        let before = self.source.get(..at as usize).unwrap_or("");
        before.len() - before.rfind('\n').map(|i| i + 1).unwrap_or(0)
    }

    fn cond_text(&self, c: &ast::Cond) -> R<String> {
        let mut reps = Vec::new();
        for p in &c.parts {
            match p {
                CondPart::Expr(e) => self.replacements(e, &mut reps)?,
                CondPart::Let { value, .. } => self.replacements(value, &mut reps)?,
            }
        }
        Ok(self.apply(c.range, reps))
    }

    // ------------------------------------------------------------ statements

    /// Unfolds statements at `depth`; returns how many it wrote.
    fn stmts(&mut self, stmts: &[Stmt], depth: usize, out: &mut String) -> R<usize> {
        let mut n = 0;
        for s in stmts {
            n += self.stmt(s, depth, out)?;
        }
        Ok(n)
    }

    fn block(&mut self, b: &ast::Block, depth: usize, out: &mut String) -> R<()> {
        if self.stmts(&b.stmts, depth, out)? == 0 {
            line(out, depth, "pass");
        }
        Ok(())
    }

    fn stmt(&mut self, s: &Stmt, depth: usize, out: &mut String) -> R<usize> {
        match &s.kind {
            StmtKind::Comptime(inner) => self.comptime_stmt(inner, depth, out),
            StmtKind::If { branches, else_block } if branches.first().is_some_and(|b| comptime_cond(&b.0).is_some()) => {
                for (c, b) in branches {
                    let Some(e) = comptime_cond(c) else {
                        return Err((c.range, "an `if comptime` chain needs `comptime` in each `elif`".into()));
                    };
                    if self.truth(e)? {
                        return self.stmts(&b.stmts, depth, out);
                    }
                }
                match else_block {
                    Some(b) => self.stmts(&b.stmts, depth, out),
                    None => Ok(0),
                }
            }
            _ if !self.stmt_depends(s) => {
                self.copy(s.range, Vec::new(), depth, out);
                Ok(1)
            }
            StmtKind::If { branches, else_block } => {
                for (i, (c, b)) in branches.iter().enumerate() {
                    let kw = if i == 0 { "if" } else { "elif" };
                    line(out, depth, &format!("{kw} {}:", self.cond_text(c)?));
                    self.block(b, depth + 1, out)?;
                }
                if let Some(b) = else_block {
                    line(out, depth, "else:");
                    self.block(b, depth + 1, out)?;
                }
                Ok(1)
            }
            StmtKind::While { cond, body } => {
                line(out, depth, &format!("while {}:", self.cond_text(cond)?));
                self.block(body, depth + 1, out)?;
                Ok(1)
            }
            StmtKind::For { pat, mode, iter, body } => {
                let m = match mode {
                    Mode::Own => "own ",
                    Mode::Mut => "mut ",
                    Mode::Borrow => "",
                };
                let header = format!("for {} in {m}{}:", slice(self.source, pat.range), self.text(iter)?);
                line(out, depth, &header);
                self.block(body, depth + 1, out)?;
                Ok(1)
            }
            StmtKind::Expr(e) if matches!(e.kind, ExprKind::Match { .. }) => {
                let ExprKind::Match { scrutinee, arms } = &e.kind else { return Ok(0) };
                let key = self.text(scrutinee)?;
                line(out, depth, &format!("match {key}:"));
                if self.arms(&key, arms, depth + 1, out)? == 0 {
                    return Err((e.range, "this `match` has no arms for the type".into()));
                }
                Ok(1)
            }
            StmtKind::Expr(_)
            | StmtKind::Let { .. }
            | StmtKind::Assign { .. }
            | StmtKind::Return(_)
            | StmtKind::Assert { .. }
            | StmtKind::Expect(_) => {
                let mut reps = Vec::new();
                for e in stmt_exprs(s) {
                    self.replacements(e, &mut reps)?;
                }
                self.copy(s.range, reps, depth, out);
                Ok(1)
            }
            _ => Err((s.range, "compile-time values cannot be used in this statement".into())),
        }
    }

    fn comptime_stmt(&mut self, inner: &Stmt, depth: usize, out: &mut String) -> R<usize> {
        match &inner.kind {
            StmtKind::For { pat, iter, body, .. } => {
                let mut n = 0;
                for v in self.items(iter)? {
                    let bound = self.bind(pat, v)?;
                    let r = self.stmts(&body.stmts, depth, out);
                    self.env.truncate(self.env.len() - bound);
                    n += r?;
                }
                Ok(n)
            }
            StmtKind::If { branches, else_block } => {
                for (c, b) in branches {
                    let [CondPart::Expr(e)] = c.parts.as_slice() else {
                        return Err((c.range, "a `comptime if` condition is one expression".into()));
                    };
                    if self.truth(e)? {
                        return self.stmts(&b.stmts, depth, out);
                    }
                }
                match else_block {
                    Some(b) => self.stmts(&b.stmts, depth, out),
                    None => Ok(0),
                }
            }
            StmtKind::Assert { cond, message } => {
                if self.truth(cond)? {
                    return Ok(0);
                }
                let why = match message.as_ref().map(|m| self.eval(m)) {
                    Some(Ok(Value::Str(s))) => s,
                    _ => "a compile-time assertion failed".to_string(),
                };
                Err((inner.range, why))
            }
            StmtKind::Expr(e) => {
                self.eval(e)?;
                Ok(0)
            }
            _ => Err((inner.range, "this statement cannot run at compile time".into())),
        }
    }

    /// The arms of a `match` on `key`, those made at compile time unfolded.
    fn arms(&mut self, key: &str, arms: &[ast::Arm], depth: usize, out: &mut String) -> R<usize> {
        let mut n = 0;
        for a in arms {
            match a.generated.as_deref() {
                Some(ArmGen::For { pat, iter, arms }) => {
                    for v in self.items(iter)? {
                        let bound = self.bind(pat, v)?;
                        let r = self.arms(key, arms, depth, out);
                        self.env.truncate(self.env.len() - bound);
                        n += r?;
                    }
                }
                Some(ArmGen::If { cond, then, else_ }) => {
                    let chosen = if self.truth(cond)? { then } else { else_ };
                    n += self.arms(key, chosen, depth, out)?;
                }
                None => {
                    self.arm(key, a, depth, out)?;
                    n += 1;
                }
            }
        }
        Ok(n)
    }

    fn arm(&mut self, key: &str, a: &ast::Arm, depth: usize, out: &mut String) -> R<()> {
        // `case V(..)` for a variant `V` bound at compile time.
        let made = match &a.pat.kind {
            PatKind::Ctor { path, args } if path.len() == 1 => match self.lookup(&path[0].text) {
                Some(Value::Variant { name, index, fields }) => {
                    if !matches!(args.as_deref(), None | Some([ast::PatArg::Rest])) {
                        return Err((a.pat.range, "a variant bound at compile time is matched as `V(..)`".into()));
                    }
                    let stem: String = key.chars().filter(|c| c.is_alphanumeric() || *c == '_').collect();
                    let names: Vec<String> = (0..fields.len()).map(|i| format!("__{stem}_{i}")).collect();
                    let pat = if names.is_empty() { name.clone() } else { format!("{name}({})", names.join(", ")) };
                    Some((pat, ArmScope { key: key.to_string(), variant: *index, names }))
                }
                Some(other) => return Err((a.pat.range, format!("{} is not a variant", describe(other)))),
                None => None,
            },
            _ => None,
        };
        let pat = match &made {
            Some((p, _)) => p.clone(),
            None => slice(self.source, a.pat.range).to_string(),
        };
        let guard = match &a.guard {
            Some(g) => format!(" if {}", self.text(g)?),
            None => String::new(),
        };
        let pushed = made.is_some();
        if let Some((_, scope)) = made {
            self.arms.push(scope);
        }
        let r = match &a.body {
            ArmBody::Block(b) => {
                line(out, depth, &format!("case {pat}{guard}:"));
                self.block(b, depth + 1, out)
            }
            ArmBody::Expr(e) => self.text(e).map(|t| line(out, depth, &format!("case {pat}{guard}: {t}"))),
        };
        if pushed {
            self.arms.pop();
        }
        r
    }

    /// Whether anything in the statement is evaluated at compile time.
    fn stmt_depends(&self, s: &Stmt) -> bool {
        match &s.kind {
            StmtKind::Comptime(_) => true,
            StmtKind::If { branches, else_block } => {
                branches.iter().any(|(c, b)| self.cond_depends(c) || b.stmts.iter().any(|s| self.stmt_depends(s)))
                    || else_block.as_ref().is_some_and(|b| b.stmts.iter().any(|s| self.stmt_depends(s)))
            }
            StmtKind::While { cond, body } => {
                self.cond_depends(cond) || body.stmts.iter().any(|s| self.stmt_depends(s))
            }
            StmtKind::For { iter, body, .. } => {
                self.depends(iter) || body.stmts.iter().any(|s| self.stmt_depends(s))
            }
            StmtKind::Expr(e) => self.expr_stmt_depends(e),
            _ => stmt_exprs(s).iter().any(|e| self.depends(e)),
        }
    }

    fn expr_stmt_depends(&self, e: &Expr) -> bool {
        match &e.kind {
            ExprKind::Match { scrutinee, arms } => self.depends(scrutinee) || arms.iter().any(|a| self.arm_depends(a)),
            _ => self.depends(e),
        }
    }

    fn arm_depends(&self, a: &ast::Arm) -> bool {
        a.generated.is_some()
            || a.guard.as_ref().is_some_and(|g| self.depends(g))
            || match &a.body {
                ArmBody::Expr(e) => self.depends(e),
                ArmBody::Block(b) => b.stmts.iter().any(|s| self.stmt_depends(s)),
            }
    }

    fn cond_depends(&self, c: &ast::Cond) -> bool {
        c.parts.iter().any(|p| match p {
            CondPart::Expr(e) => self.depends(e),
            CondPart::Let { value, .. } => self.depends(value),
        })
    }
}

/// The condition of `if comptime e:`: `comptime` before a condition makes
/// all of it compile-time (`if comptime a or b:`), though as an operator
/// it applies to the operand that follows it.
fn comptime_cond(c: &ast::Cond) -> Option<&Expr> {
    let [CondPart::Expr(e)] = c.parts.as_slice() else { return None };
    let mut x = e;
    loop {
        x = match &x.kind {
            ExprKind::Comptime(_) => return Some(e),
            ExprKind::Binary { lhs, .. } => lhs,
            ExprKind::Compare { first, .. } => first,
            _ => return None,
        };
    }
}

/// The expressions a simple statement holds.
fn stmt_exprs(s: &Stmt) -> Vec<&Expr> {
    match &s.kind {
        StmtKind::Let { init, .. } => init.iter().collect(),
        StmtKind::Assign { target, value, .. } => vec![target, value],
        StmtKind::Expr(e) | StmtKind::Expect(e) | StmtKind::Return(Some(e)) => vec![e],
        StmtKind::Assert { cond, message } => std::iter::once(cond).chain(message.iter()).collect(),
        _ => Vec::new(),
    }
}

/// The direct subexpressions of an expression.
fn children<'e>(e: &'e Expr, out: &mut Vec<&'e Expr>) {
    match &e.kind {
        ExprKind::Paren(x)
        | ExprKind::Unary { expr: x, .. }
        | ExprKind::Ref { expr: x, .. }
        | ExprKind::FromEnd(x)
        | ExprKind::Try(x)
        | ExprKind::Comptime(x) => out.push(x),
        ExprKind::Field { base, .. } | ExprKind::OptField { base, .. } => out.push(base),
        ExprKind::FieldOf { base, field } => {
            out.push(base);
            out.push(field);
        }
        ExprKind::Binary { lhs, rhs, .. } => {
            out.push(lhs);
            out.push(rhs);
        }
        ExprKind::Compare { first, rest } => {
            out.push(first);
            out.extend(rest.iter().map(|(_, x)| x));
        }
        ExprKind::Call { callee, args, .. } => {
            out.push(callee);
            out.extend(args.iter().map(|a| &a.value));
        }
        ExprKind::Index { base, args } => {
            out.push(base);
            for a in args {
                match a {
                    ast::IndexArg::Expr(x) => out.push(x),
                    ast::IndexArg::Slice { lo, hi } => out.extend(lo.iter().chain(hi.iter())),
                }
            }
        }
        ExprKind::Tuple(xs) | ExprKind::List(xs) | ExprKind::Set(xs) => out.extend(xs.iter()),
        ExprKind::Dict(kvs) => {
            for (k, v) in kvs {
                out.push(k);
                out.push(v);
            }
        }
        ExprKind::If { branches, else_ } => {
            for (c, v) in branches {
                out.push(c);
                out.push(v);
            }
            out.extend(else_.iter().map(|x| &**x));
        }
        ExprKind::Str(parts) => {
            for p in parts {
                if let StrPart::Interp { expr, .. } = p {
                    out.push(expr);
                }
            }
        }
        _ => {}
    }
}

/// Visits an expression and everything in it.
fn walk(e: &Expr, f: &mut dyn FnMut(&Expr)) {
    f(e);
    let mut kids = Vec::new();
    children(e, &mut kids);
    for k in kids {
        walk(k, f);
    }
    match &e.kind {
        ExprKind::Closure(c) => match &c.body {
            ast::ClosureBody::Expr(x) => walk(x, f),
            ast::ClosureBody::Block(b) => walk_block(b, f),
        },
        ExprKind::Match { scrutinee, arms } => {
            walk(scrutinee, f);
            for a in arms {
                match &a.body {
                    ArmBody::Expr(x) => walk(x, f),
                    ArmBody::Block(b) => walk_block(b, f),
                }
            }
        }
        _ => {}
    }
}

fn walk_block(b: &ast::Block, f: &mut dyn FnMut(&Expr)) {
    for s in &b.stmts {
        for e in stmt_exprs(s) {
            walk(e, f);
        }
    }
}
