//! Code evaluated at compile time by unfolding (08, "Derive" and
//! "Reflection"; 18, Phase 4 and 8): a derive, and a generic function that
//! reflects on its type parameters.
//!
//! A trait's `comptime def derive(T: type) -> impl Trait for T` returns the
//! impl the trait derives for `T`, written `return impl Trait for T:` with
//! methods. A generic function whose body asks about its type parameters
//! (`if comptime reflect.is_struct(T):`, `comptime for f in
//! reflect.fields_of(T):`, `value.[f]`) is made once for each list of type
//! arguments it is called with (an instance). Stage 0 evaluates both by
//! unfolding: the code that runs at compile time (`comptime for`, `comptime
//! if`, `if comptime`, the same two among a `match`'s arms, `comptime
//! panic`, and every expression made only of compile-time values) is
//! evaluated, and the rest is the impl's or the instance's code, written
//! out as source text with the compile-time values in it as literals and
//! each type parameter replaced by its argument. The text is declared
//! beside the type (or the function) and checked like any other code, so
//! an error in it is reported as in any other.
//!
//! The compile-time values are types, descriptors of their fields,
//! variants and attributes, lists and pairs of them, integers, text and
//! booleans. Reflection is the compile-time module `reflect`: `name_of(T)`
//! (the type's name), `is_struct`, `is_enum`, `is_int`, `is_float`,
//! `is_bool`, `is_char`, `is_str` (`str` or `String`), `is_list`,
//! `is_option` and `is_tuple` of a type, `elem_of(T)` (a list's or an
//! option's element type), `fields_of(T)` (a struct's fields),
//! `variants_of(T)` (an enum's variants), `attributes_of(T)` (the
//! attributes written on its declaration, as data) and `implements(T,
//! "Trait")` (whether it has an impl of the trait). A field has `name` (for
//! a positional variant field, its position), `index`, `named`, `ty` (its
//! type, when known) and `default` (whether it has a default); a variant
//! has `name`, `index` and `fields`; an attribute has `name` and `args`
//! (the text of each argument). `x.[field]` is that field of `x`; a
//! variant's field is reachable only inside an arm `case V(..)` that
//! `comptime for V in reflect.variants_of(T)` made for a `match x`, where
//! it is the arm's binding. Compile-time code may also use `enumerate`,
//! `len`, integer arithmetic, comparisons, `and`, `or` and `not`.

use sisu_diag::TextRange;
use sisu_syntax::ast::{
    self, ArmBody, ArmGen, BinOp, CmpOp, CondPart, Expr, ExprKind, Mode, PatKind, Stmt, StmtKind, StrPart,
    UnOp,
};

/// The shape of the type being derived for.
#[derive(Clone)]
pub(crate) enum Body {
    /// Field names.
    Struct(Vec<String>),
    /// Variant names, each with its field names (`None` for positional).
    Enum(Vec<(String, Vec<Option<String>>)>),
}

/// What kind of type a type is, for `reflect.is_...`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    Int,
    Float,
    Bool,
    Char,
    Str,
    List,
    Option,
    Tuple,
    Struct,
    Enum,
    Other,
}

/// What reflection sees of a type: an instance's type argument, the type a
/// derive is for, or a field's type (whose own fields are not looked into).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TypeShape {
    /// `reflect.name_of`: `Point`, `int`, `list[int]`.
    pub name: String,
    /// How the unfolded code writes the type.
    pub written: String,
    pub kind: Kind,
    pub fields: Vec<FieldShape>,
    pub variants: Vec<VariantShape>,
    /// The attributes on the declaration: each name and its arguments' text.
    pub attrs: Vec<Attribute>,
    /// The traits it implements, by name.
    pub impls: Vec<String>,
    /// A list's or an option's element type.
    pub elem: Option<Box<TypeShape>>,
}

/// An attribute as data: its name and the text of each argument.
pub(crate) type Attribute = (String, Vec<String>);

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct FieldShape {
    pub name: String,
    pub ty: Option<Box<TypeShape>>,
    pub default: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct VariantShape {
    pub name: String,
    pub fields: Vec<(Option<String>, Option<Box<TypeShape>>)>,
}

impl TypeShape {
    /// A shape that knows only the type's name.
    pub fn named(name: &str, written: &str, kind: Kind) -> TypeShape {
        TypeShape {
            name: name.to_string(),
            written: written.to_string(),
            kind,
            fields: Vec::new(),
            variants: Vec::new(),
            attrs: Vec::new(),
            impls: Vec::new(),
            elem: None,
        }
    }

    /// The shape of a derive's target, from its declaration's names.
    fn of_target(t: &Target) -> TypeShape {
        let mut s = TypeShape::named(t.name, t.written, Kind::Other);
        match t.body {
            Body::Struct(fields) => {
                s.kind = Kind::Struct;
                s.fields =
                    fields.iter().map(|n| FieldShape { name: n.clone(), ty: None, default: false }).collect();
            }
            Body::Enum(vs) => {
                s.kind = Kind::Enum;
                s.variants = vs
                    .iter()
                    .map(|(n, fs)| VariantShape {
                        name: n.clone(),
                        fields: fs.iter().map(|f| (f.clone(), None)).collect(),
                    })
                    .collect();
            }
        }
        s
    }
}

/// A compile-time value.
#[derive(Clone, Debug, PartialEq)]
enum Value {
    /// A type: a type parameter's argument, or a field's type.
    Type(Box<TypeShape>),
    Field {
        name: String,
        index: usize,
        variant: Option<usize>,
        named: bool,
        ty: Option<Box<TypeShape>>,
        default: bool,
        /// How the unfolded code writes the struct the field is in.
        owner: Option<String>,
    },
    /// An attribute on a declaration: its name and its arguments' text.
    Attr {
        name: String,
        args: Vec<String>,
    },
    Variant {
        name: String,
        index: usize,
        fields: Vec<Value>,
    },
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
#[derive(Clone)]
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
    /// The type parameters (`T`) and their arguments.
    params: Vec<(String, TypeShape)>,
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
pub(crate) fn unfold(
    template: &ast::FnDef,
    source: &str,
    head: &str,
    target: &Target,
) -> Result<String, Failure> {
    let (param, imp) =
        template_parts(template).ok_or((template.name.range, "it is not a derive template".to_string()))?;
    let shape = TypeShape::of_target(target);
    let mut u =
        Unfold { source, params: vec![(param.clone(), shape.clone())], env: Vec::new(), arms: Vec::new() };
    u.env.push((param, Value::Type(Box::new(shape))));
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

/// The source of the instance of the generic function `f` (from the file
/// whose text is `source`) for these type arguments, named `name`: its
/// signature with each type parameter replaced, and its body unfolded.
pub(crate) fn unfold_fn(
    f: &ast::FnDef,
    source: &str,
    params: Vec<(String, TypeShape)>,
    name: &str,
) -> Result<String, Failure> {
    let Some(body) = &f.body else {
        return Err((f.name.range, "a function without a body has no instance".into()));
    };
    let mut u = Unfold { source, params: params.clone(), env: Vec::new(), arms: Vec::new() };
    for (p, shape) in params {
        u.env.push((p, Value::Type(Box::new(shape))));
    }
    // `def name[T: B](params) -> R:` becomes `def NAME(params) -> R:`.
    let header = slice(source, TextRange::new(f.name.range.end, body.range.start)).trim_end();
    let rest = match header.strip_prefix('[') {
        Some(after) => {
            let mut depth = 1;
            let mut cut = after.len();
            for (i, c) in after.char_indices() {
                match c {
                    '[' => depth += 1,
                    ']' => {
                        depth -= 1;
                        if depth == 0 {
                            cut = i + 1;
                            break;
                        }
                    }
                    _ => {}
                }
            }
            &after[cut..]
        }
        None => header,
    };
    // A `where` clause names the parameters, which an instance has not.
    let rest = match rest.find(" where ") {
        Some(i) => format!("{}:", rest[..i].trim_end()),
        None => rest.to_string(),
    };
    let pure = if f.is_pure { "pure " } else { "" };
    let mut out = String::new();
    line(&mut out, 0, &format!("{pure}def {name}{}", u.substitute_param(&rest)));
    if u.stmts(&body.stmts, 1, &mut out)? == 0 {
        line(&mut out, 1, "pass");
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

/// A value as a Sisu literal; a type as its name in the unfolded code.
fn literal(v: &Value) -> Option<String> {
    match v {
        Value::Str(s) => Some(format!("\"{}\"", escape(s))),
        Value::Int(i) => Some(i.to_string()),
        Value::Bool(b) => Some(if *b { "True" } else { "False" }.into()),
        Value::Type(t) => Some(t.written.clone()),
        Value::List(xs) => {
            let parts: Option<Vec<String>> = xs.iter().map(literal).collect();
            Some(format!("[{}]", parts?.join(", ")))
        }
        Value::Tuple(xs) => {
            let parts: Option<Vec<String>> = xs.iter().map(literal).collect();
            let parts = parts?;
            Some(if parts.len() == 1 {
                format!("({},)", parts[0])
            } else {
                format!("({})", parts.join(", "))
            })
        }
        _ => None,
    }
}

/// Whether `e` is marked to be decided at compile time: `comptime c`, or
/// an operator chain that starts with it (`comptime a or b`).
fn marked_comptime(e: &Expr) -> bool {
    let mut x = e;
    loop {
        x = match &x.kind {
            ExprKind::Comptime(_) => return true,
            ExprKind::Binary { lhs, .. } => lhs,
            ExprKind::Compare { first, .. } => first,
            _ => return false,
        };
    }
}

/// The `reflect` functions that make run-time code instead of a
/// compile-time value.
const CODE_MAKERS: [&str; 2] = ["construct", "default_of"];

/// A value as interpolated text shows it: a list as `[a, b]`.
fn written(v: &Value) -> Option<String> {
    match v {
        Value::Str(s) => Some(s.clone()),
        Value::Int(i) => Some(i.to_string()),
        Value::Bool(b) => Some(if *b { "True" } else { "False" }.into()),
        Value::Type(t) => Some(t.name.clone()),
        Value::List(xs) => {
            let parts: Option<Vec<String>> = xs.iter().map(written).collect();
            Some(format!("[{}]", parts?.join(", ")))
        }
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
        Value::Type(_) => "a type",
        Value::Attr { .. } => "an attribute",
        Value::Field { .. } => "a field",
        Value::Variant { .. } => "a variant",
        Value::Str(_) => "text",
        Value::Int(_) => "an integer",
        Value::Bool(_) => "a boolean",
        Value::List(_) => "a list",
        Value::Tuple(_) => "a tuple",
    }
}

impl<'a> Unfold<'a> {
    fn lookup(&self, n: &str) -> Option<&Value> {
        self.env.iter().rev().find(|(k, _)| k == n).map(|(_, v)| v)
    }

    /// Replaces each type parameter by its argument, as a word, outside
    /// string literals.
    fn substitute_param(&self, text: &str) -> String {
        let mut out = String::new();
        let mut word = String::new();
        let flush = |word: &mut String, out: &mut String| {
            match self.params.iter().find(|(p, _)| p == word) {
                Some((_, shape)) => out.push_str(&shape.written),
                None => out.push_str(word),
            }
            word.clear();
        };
        let mut in_str = false;
        let mut escaped = false;
        let mut prev = ' ';
        for c in text.chars() {
            if in_str {
                out.push(c);
                if escaped {
                    escaped = false;
                } else if c == '\\' {
                    escaped = true;
                } else if c == '"' {
                    in_str = false;
                }
                continue;
            }
            if c == '"' && !(word.is_empty() && prev.is_alphanumeric()) {
                flush(&mut word, &mut out);
                in_str = true;
                out.push(c);
            } else if c.is_alphanumeric() || c == '_' {
                word.push(c);
            } else {
                flush(&mut word, &mut out);
                out.push(c);
            }
            prev = c;
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
                StrPart::Interp { expr, spec, debug, .. } => {
                    spec.is_none() && debug.is_none() && self.is_static(expr)
                }
            }),
            // The attributes of descriptors; anything else (a method) runs
            // at run time on the value.
            ExprKind::Field { base, name } => {
                matches!(
                    name.text.as_str(),
                    "name" | "index" | "named" | "fields" | "ty" | "default" | "args"
                ) && self.is_static(base)
            }
            ExprKind::Call { callee, args, trailing: None } => {
                let known = match &callee.kind {
                    ExprKind::Field { base, name } => {
                        matches!(&base.kind, ExprKind::Name(n) if n == "reflect")
                            && !CODE_MAKERS.contains(&name.text.as_str())
                    }
                    ExprKind::Name(n) => {
                        matches!(n.as_str(), "enumerate" | "len") && self.lookup(n).is_none()
                    }
                    _ => false,
                };
                known && args.iter().all(|a| a.name.is_none() && self.is_static(&a.value))
            }
            ExprKind::Binary { lhs, rhs, .. } => self.is_static(lhs) && self.is_static(rhs),
            ExprKind::Compare { first, rest } => {
                self.is_static(first) && rest.iter().all(|(_, x)| self.is_static(x))
            }
            ExprKind::Unary { expr, .. } | ExprKind::Paren(expr) | ExprKind::Comptime(expr) => {
                self.is_static(expr)
            }
            ExprKind::Tuple(xs) | ExprKind::List(xs) => xs.iter().all(|x| self.is_static(x)),
            ExprKind::If { branches, else_ } => {
                branches.iter().all(|(c, v)| self.is_static(c) && self.is_static(v))
                    && else_.as_ref().is_some_and(|x| self.is_static(x))
            }
            // A comprehension over compile-time lists whose element is
            // known: its bindings are compile-time values too.
            ExprKind::Comprehension { kind: ast::CompKind::List, elem, value: None, clauses } => {
                self.envs(clauses).is_ok_and(|es| es.iter().all(|u| u.is_static(elem)))
            }
            _ => false,
        }
    }

    /// The environments a comprehension over compile-time lists makes,
    /// one for each element, in order.
    fn envs(&self, clauses: &[ast::CompClause]) -> R<Vec<Unfold<'a>>> {
        let mut envs = vec![self.fork()];
        for c in clauses {
            let mut next = Vec::new();
            for u in envs {
                match c {
                    ast::CompClause::For { pat, iter } => {
                        if !u.is_static(iter) {
                            return Err((iter.range, "this is not a compile-time list".into()));
                        }
                        for v in u.items(iter)? {
                            let mut w = u.fork();
                            w.bind(pat, v)?;
                            next.push(w);
                        }
                    }
                    ast::CompClause::If(cond) => {
                        if u.truth(cond)? {
                            next.push(u);
                        }
                    }
                }
            }
            envs = next;
        }
        Ok(envs)
    }

    /// A copy of the unfolding's scope, to bind more names in.
    fn fork(&self) -> Unfold<'a> {
        Unfold {
            source: self.source,
            params: self.params.clone(),
            env: self.env.clone(),
            arms: self.arms.clone(),
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
                        StrPart::Interp { expr, .. } => match written(&self.eval(expr)?) {
                            Some(t) => s.push_str(&t),
                            None => {
                                let v = self.eval(expr)?;
                                return fail(format!("{} cannot be written into text", describe(&v)));
                            }
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
                    (Value::Field { ty: Some(t), .. }, "ty") => Some(Value::Type(t.clone())),
                    (Value::Field { default, .. }, "default") => Some(Value::Bool(*default)),
                    (Value::Variant { name, .. }, "name") => Some(Value::Str(name.clone())),
                    (Value::Variant { index, .. }, "index") => Some(Value::Int(*index as i64)),
                    (Value::Variant { fields, .. }, "fields") => Some(Value::List(fields.clone())),
                    (Value::Attr { name, .. }, "name") => Some(Value::Str(name.clone())),
                    (Value::Attr { args, .. }, "args") => {
                        Some(Value::List(args.iter().map(|a| Value::Str(a.clone())).collect()))
                    }
                    _ => None,
                };
                got.map_or_else(|| fail(format!("{} has no `{}`", describe(&b), name.text)), Ok)
            }
            ExprKind::Call { callee, args, .. } => {
                // `len(reflect.variants_of(T))` counts without making the
                // list, which a derive asks for in each arm it writes.
                if let (ExprKind::Name(n), [a]) = (&callee.kind, args.as_slice())
                    && n == "len"
                    && let Some(count) = self.count_of(&a.value)?
                {
                    return Ok(Value::Int(count as i64));
                }
                let vals: Vec<Value> = args.iter().map(|a| self.eval(&a.value)).collect::<R<_>>()?;
                let one = |vals: Vec<Value>| match <[Value; 1]>::try_from(vals) {
                    Ok([v]) => Ok(v),
                    Err(_) => Err((e.range, "this takes one argument".to_string())),
                };
                match &callee.kind {
                    ExprKind::Field { name, .. } => {
                        let Some(Value::Type(t)) = vals.first() else {
                            return fail(format!("`reflect.{}` takes a type", name.text));
                        };
                        if name.text == "implements" {
                            let Some(Value::Str(tr)) = vals.get(1) else {
                                return fail("`reflect.implements` takes a type and a trait's name".into());
                            };
                            return Ok(Value::Bool(t.impls.iter().any(|i| i == tr)));
                        }
                        if vals.len() != 1 {
                            return fail(format!("`reflect.{}` takes one type", name.text));
                        }
                        self.reflect(&name.text, t)
                            .ok_or((e.range, format!("`reflect` has no `{}`", name.text)))
                    }
                    ExprKind::Name(n) if n == "enumerate" => match one(vals)? {
                        Value::List(xs) => Ok(Value::List(
                            xs.into_iter()
                                .enumerate()
                                .map(|(i, x)| Value::Tuple(vec![Value::Int(i as i64), x]))
                                .collect(),
                        )),
                        other => fail(format!("`enumerate` of {}", describe(&other))),
                    },
                    ExprKind::Name(n) if n == "len" => match one(vals)? {
                        Value::List(xs) => Ok(Value::Int(xs.len() as i64)),
                        Value::Str(s) => Ok(Value::Int(s.chars().count() as i64)),
                        other => fail(format!("`len` of {}", describe(&other))),
                    },
                    _ => fail(
                        "only `reflect`, `enumerate` and `len` can be called at compile time here".into(),
                    ),
                }
            }
            ExprKind::Unary { op, expr } => match (op, self.eval(expr)?) {
                (UnOp::Not, Value::Bool(b)) => Ok(Value::Bool(!b)),
                (UnOp::Neg, Value::Int(i)) => {
                    i.checked_neg().map(Value::Int).ok_or((e.range, "overflow".into()))
                }
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
                        _ => {
                            return fail(format!(
                                "`{}` of {} and {}",
                                op.text(),
                                describe(&left),
                                describe(&right)
                            ));
                        }
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
            ExprKind::If { branches, else_ } => {
                for (c, v) in branches {
                    if self.truth(c)? {
                        return self.eval(v);
                    }
                }
                match else_ {
                    Some(x) => self.eval(x),
                    None => fail("an `if` expression needs an `else`".into()),
                }
            }
            ExprKind::Comprehension { kind: ast::CompKind::List, elem, value: None, clauses } => {
                let envs = self.envs(clauses)?;
                Ok(Value::List(envs.iter().map(|u| u.eval(elem)).collect::<R<_>>()?))
            }
            _ => fail("this cannot be evaluated at compile time".into()),
        }
    }

    /// The length of the list `reflect.fields_of(T)`, `variants_of(T)` or
    /// `attributes_of(T)` gives, when `e` is one of them of a type: what
    /// `len` of it is, without making the list.
    fn count_of(&self, e: &Expr) -> R<Option<usize>> {
        let ExprKind::Call { callee, args, .. } = &e.kind else { return Ok(None) };
        let (ExprKind::Field { name, .. }, [a]) = (&callee.kind, args.as_slice()) else { return Ok(None) };
        if !matches!(name.text.as_str(), "fields_of" | "variants_of" | "attributes_of") {
            return Ok(None);
        }
        let Value::Type(t) = self.eval(&a.value)? else { return Ok(None) };
        Ok(Some(match name.text.as_str() {
            "fields_of" => t.fields.len(),
            "variants_of" => t.variants.len(),
            _ => t.attrs.len(),
        }))
    }

    /// `reflect.f(t)`.
    fn reflect(&self, f: &str, t: &TypeShape) -> Option<Value> {
        let is = |k: Kind| Value::Bool(t.kind == k);
        Some(match f {
            "name_of" => Value::Str(t.name.clone()),
            "is_struct" => is(Kind::Struct),
            "is_enum" => is(Kind::Enum),
            "is_int" => is(Kind::Int),
            "is_float" => is(Kind::Float),
            "is_bool" => is(Kind::Bool),
            "is_char" => is(Kind::Char),
            "is_str" => is(Kind::Str),
            "is_list" => is(Kind::List),
            "is_option" => is(Kind::Option),
            "is_tuple" => is(Kind::Tuple),
            "elem_of" => Value::Type(t.elem.clone()?),
            "fields_of" => Value::List(
                t.fields
                    .iter()
                    .enumerate()
                    .map(|(index, f)| Value::Field {
                        name: f.name.clone(),
                        index,
                        variant: None,
                        named: true,
                        ty: f.ty.clone(),
                        default: f.default,
                        owner: Some(t.written.clone()),
                    })
                    .collect(),
            ),
            "variants_of" => Value::List(
                t.variants
                    .iter()
                    .enumerate()
                    .map(|(vi, v)| Value::Variant {
                        name: v.name.clone(),
                        index: vi,
                        fields: v
                            .fields
                            .iter()
                            .enumerate()
                            .map(|(index, (n, ty))| Value::Field {
                                name: n.clone().unwrap_or_else(|| index.to_string()),
                                index,
                                variant: Some(vi),
                                named: n.is_some(),
                                ty: ty.clone(),
                                default: false,
                                owner: None,
                            })
                            .collect(),
                    })
                    .collect(),
            ),
            "attributes_of" => Value::List(
                t.attrs
                    .iter()
                    .map(|(name, args)| Value::Attr { name: name.clone(), args: args.clone() })
                    .collect(),
            ),
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

    /// The source of `range` with `reps` in it and, in what is copied
    /// from the source, each type parameter replaced by its argument.
    fn apply(&self, range: TextRange, mut reps: Vec<(TextRange, String)>) -> String {
        reps.sort_by_key(|(r, _)| r.start);
        let mut out = String::new();
        let mut at = range.start;
        for (r, t) in reps {
            if r.start < at {
                continue;
            }
            out.push_str(&self.substitute_param(slice(self.source, TextRange::new(at, r.start))));
            out.push_str(&t);
            at = r.end;
        }
        out.push_str(&self.substitute_param(slice(self.source, TextRange::new(at, range.end))));
        out
    }

    fn replacements(&self, e: &Expr, out: &mut Vec<(TextRange, String)>) -> R<()> {
        if !self.depends(e) {
            return Ok(());
        }
        if self.is_static(e) {
            let v = self.eval(e)?;
            let lit = literal(&v).ok_or((
                e.range,
                format!("{} is a compile-time value with no run-time form", describe(&v)),
            ))?;
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
                            let Some(t) = written(&v) else {
                                return Err((
                                    expr.range,
                                    format!("{} cannot be written into text", describe(&v)),
                                ));
                            };
                            out.push((*range, escape(&t)));
                        } else {
                            self.replacements(expr, out)?;
                        }
                    }
                }
            }
            // `if comptime c: a else: b` as an expression: the branch the
            // condition picks.
            ExprKind::If { branches, else_ } if branches.first().is_some_and(|(c, _)| marked_comptime(c)) => {
                let mut chosen = None;
                for (c, v) in branches {
                    if !marked_comptime(c) {
                        return Err((
                            c.range,
                            "an `if comptime` chain needs `comptime` in each `elif`".into(),
                        ));
                    }
                    if self.truth(c)? {
                        chosen = Some(v);
                        break;
                    }
                }
                let chosen = match chosen.or(else_.as_deref()) {
                    Some(x) => x,
                    None => return Err((e.range, "an `if` expression needs an `else`".into())),
                };
                out.push((e.range, format!("({})", self.text(chosen)?)));
            }
            // A comprehension over compile-time lists is unrolled into the
            // list, set or dict it makes.
            ExprKind::Comprehension { kind, elem, value, clauses } if self.envs(clauses).is_ok() => {
                let mut parts = Vec::new();
                for u in self.envs(clauses)? {
                    let item = u.text(elem)?;
                    parts.push(match value {
                        Some(v) => format!("{item}: {}", u.text(v)?),
                        None => item,
                    });
                }
                let text = match kind {
                    ast::CompKind::List => format!("[{}]", parts.join(", ")),
                    ast::CompKind::Dict if parts.is_empty() => "dict()".to_string(),
                    ast::CompKind::Set if parts.is_empty() => "set()".to_string(),
                    ast::CompKind::Dict | ast::CompKind::Set => format!("{{{}}}", parts.join(", ")),
                    ast::CompKind::Gen => {
                        return Err((
                            e.range,
                            "a generator over compile-time values is not unrolled; use a list".into(),
                        ));
                    }
                };
                out.push((e.range, text));
            }
            ExprKind::Call { callee, args, trailing: None } if matches!(&callee.kind, ExprKind::Field { base, name } if matches!(&base.kind, ExprKind::Name(n) if n == "reflect") && CODE_MAKERS.contains(&name.text.as_str())) =>
            {
                let ExprKind::Field { name, .. } = &callee.kind else { return Ok(()) };
                let text = match name.text.as_str() {
                    "default_of" => self.default_of(e, args)?,
                    _ => self.construct(e, args)?,
                };
                out.push((e.range, text));
            }
            // In a `match` or a closure, compile-time values are written in
            // where they are used; their statements are copied as they are.
            ExprKind::Match { scrutinee, arms } if arms.iter().all(|a| a.generated.is_none()) => {
                self.replacements(scrutinee, out)?;
                for a in arms {
                    if let Some(g) = &a.guard {
                        self.replacements(g, out)?;
                    }
                    match &a.body {
                        ArmBody::Expr(x) => self.replacements(x, out)?,
                        ArmBody::Block(b) => self.block_replacements(b, out)?,
                    }
                }
            }
            ExprKind::Closure(c) => match &c.body {
                ast::ClosureBody::Expr(x) => self.replacements(x, out)?,
                ast::ClosureBody::Block(b) => self.block_replacements(b, out)?,
            },
            // A comprehension over run-time values with compile-time values
            // in it: they are written in, unless a name it binds hides one.
            ExprKind::Comprehension { elem, value, clauses, .. } => {
                for c in clauses {
                    if let ast::CompClause::For { pat, .. } = c
                        && let Some(n) = bound_names(pat).into_iter().find(|n| self.lookup(n).is_some())
                    {
                        return Err((pat.range, format!("`{n}` here hides the compile-time `{n}`")));
                    }
                }
                self.replacements(elem, out)?;
                if let Some(v) = value {
                    self.replacements(v, out)?;
                }
                for c in clauses {
                    match c {
                        ast::CompClause::For { iter, .. } => self.replacements(iter, out)?,
                        ast::CompClause::If(x) => self.replacements(x, out)?,
                    }
                }
            }
            ExprKind::Match { .. } => {
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

    /// The replacements in a block inside an expression, whose statements
    /// are copied as they are written (none of them runs at compile time).
    fn block_replacements(&self, b: &ast::Block, out: &mut Vec<(TextRange, String)>) -> R<()> {
        for s in &b.stmts {
            match &s.kind {
                StmtKind::Comptime(_) => {
                    return Err((s.range, "a compile-time statement cannot be inside an expression".into()));
                }
                StmtKind::If { branches, else_block } => {
                    if branches.first().is_some_and(|(c, _)| comptime_cond(c).is_some()) {
                        return Err((s.range, "an `if comptime` cannot be inside an expression".into()));
                    }
                    for (c, body) in branches {
                        for p in &c.parts {
                            match p {
                                CondPart::Expr(e) => self.replacements(e, out)?,
                                CondPart::Let { value, .. } => self.replacements(value, out)?,
                            }
                        }
                        self.block_replacements(body, out)?;
                    }
                    if let Some(body) = else_block {
                        self.block_replacements(body, out)?;
                    }
                }
                StmtKind::While { cond, body } => {
                    for p in &cond.parts {
                        match p {
                            CondPart::Expr(e) => self.replacements(e, out)?,
                            CondPart::Let { value, .. } => self.replacements(value, out)?,
                        }
                    }
                    self.block_replacements(body, out)?;
                }
                StmtKind::For { iter, body, .. } => {
                    self.replacements(iter, out)?;
                    self.block_replacements(body, out)?;
                }
                _ => {
                    for e in stmt_exprs(s) {
                        self.replacements(e, out)?;
                    }
                }
            }
        }
        Ok(())
    }

    /// `reflect.default_of(f)`: the default of the struct field `f`,
    /// evaluated where the call is (as an omitted argument's would be).
    fn default_of(&self, e: &Expr, args: &[ast::Arg]) -> R<String> {
        let [a] = args else { return Err((e.range, "`reflect.default_of` takes a field".into())) };
        match self.eval(&a.value)? {
            Value::Field { name, default: true, owner: Some(owner), .. } => {
                Ok(format!("{owner}.__default_{name}()"))
            }
            Value::Field { name, owner: Some(_), .. } => {
                Err((a.value.range, format!("the field `{name}` has no default")))
            }
            Value::Field { name, .. } => {
                Err((a.value.range, format!("`{name}` is a variant's field, which has no default")))
            }
            other => {
                Err((a.value.range, format!("`reflect.default_of` takes a field, not {}", describe(&other))))
            }
        }
    }

    /// `reflect.construct(T, {name: value, ...})`: the struct `T` with
    /// each field named by a key set to its value, the keys known at
    /// compile time (a dict, or a comprehension over compile-time values);
    /// a field that is not named takes its default.
    fn construct(&self, e: &Expr, args: &[ast::Arg]) -> R<String> {
        let [t, d] = args else {
            return Err((e.range, "`reflect.construct` takes a struct type and a dict of its fields".into()));
        };
        let Value::Type(shape) = self.eval(&t.value)? else {
            return Err((t.value.range, "`reflect.construct` takes a type first".into()));
        };
        if shape.kind != Kind::Struct {
            return Err((t.value.range, format!("`{}` is not a struct", shape.name)));
        }
        let mut pairs: Vec<(Value, String, TextRange)> = Vec::new();
        match &d.value.kind {
            ExprKind::Dict(kvs) => {
                for (k, v) in kvs {
                    pairs.push((self.eval(k)?, self.text(v)?, k.range));
                }
            }
            ExprKind::Comprehension { kind: ast::CompKind::Dict, elem, value: Some(v), clauses } => {
                for u in self.envs(clauses)? {
                    pairs.push((u.eval(elem)?, u.text(v)?, elem.range));
                }
            }
            _ => {
                return Err((
                    d.value.range,
                    "`reflect.construct` takes the fields as a dict whose keys are known at compile time"
                        .into(),
                ));
            }
        }
        let mut given: Vec<String> = Vec::new();
        let mut parts = Vec::new();
        for (k, v, at) in pairs {
            let Value::Str(k) = k else {
                return Err((at, format!("a field is named by text, not by {}", describe(&k))));
            };
            if !shape.fields.iter().any(|f| f.name == k) {
                return Err((at, format!("`{}` has no field `{k}`", shape.name)));
            }
            if given.contains(&k) {
                return Err((at, format!("the field `{k}` is given twice")));
            }
            parts.push(format!("{k}={v}"));
            given.push(k);
        }
        if let Some(f) = shape.fields.iter().find(|f| !f.default && !given.contains(&f.name)) {
            return Err((d.value.range, format!("the field `{}` of `{}` is not given", f.name, shape.name)));
        }
        Ok(format!("{}({})", shape.written, parts.join(", ")))
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
            StmtKind::If { branches, else_block }
                if branches.first().is_some_and(|b| comptime_cond(&b.0).is_some()) =>
            {
                for (c, b) in branches {
                    let Some(e) = comptime_cond(c) else {
                        return Err((
                            c.range,
                            "an `if comptime` chain needs `comptime` in each `elif`".into(),
                        ));
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
                Err((cond.range, why))
            }
            // `comptime panic("...")`: a compile error where the code is
            // unfolded (for this type).
            StmtKind::Expr(e) if matches!(&e.kind, ExprKind::Call { callee, .. } if matches!(&callee.kind, ExprKind::Name(n) if n == "panic")) =>
            {
                let ExprKind::Call { args, .. } = &e.kind else { return Ok(0) };
                let why = match args.first().map(|a| self.eval(&a.value)) {
                    Some(Ok(Value::Str(s))) => s,
                    Some(Err(f)) => return Err(f),
                    _ => "`comptime panic`".to_string(),
                };
                Err((e.range, why))
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
                        return Err((
                            a.pat.range,
                            "a variant bound at compile time is matched as `V(..)`".into(),
                        ));
                    }
                    let stem: String = key.chars().filter(|c| c.is_alphanumeric() || *c == '_').collect();
                    let names: Vec<String> = (0..fields.len()).map(|i| format!("__{stem}_{i}")).collect();
                    let pat =
                        if names.is_empty() { name.clone() } else { format!("{name}({})", names.join(", ")) };
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
                branches
                    .iter()
                    .any(|(c, b)| self.cond_depends(c) || b.stmts.iter().any(|s| self.stmt_depends(s)))
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
            ExprKind::Match { scrutinee, arms } => {
                self.depends(scrutinee) || arms.iter().any(|a| self.arm_depends(a))
            }
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

/// The names a pattern binds.
fn bound_names(p: &ast::Pattern) -> Vec<String> {
    let mut out = Vec::new();
    match &p.kind {
        PatKind::Bind(n) => out.push(n.text.clone()),
        PatKind::Tuple(ps) => {
            for q in ps {
                out.extend(bound_names(q));
            }
        }
        _ => {}
    }
    out
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
        ExprKind::Comprehension { elem, value, clauses, .. } => {
            out.push(elem);
            out.extend(value.iter().map(|x| &**x));
            for c in clauses {
                match c {
                    ast::CompClause::For { iter, .. } => out.push(iter),
                    ast::CompClause::If(x) => out.push(x),
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
