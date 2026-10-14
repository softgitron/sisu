//! Reads the SIR text form back. A line-oriented reader with a character
//! cursor per line; every error names its line.

use crate::*;

#[derive(Clone, Debug, PartialEq)]
pub struct ReadError {
    pub line: usize,
    pub message: String,
}

type R<T> = Result<T, String>;

struct Cur<'a> {
    s: &'a str,
    pos: usize,
}

impl<'a> Cur<'a> {
    fn new(s: &'a str) -> Cur<'a> {
        Cur { s, pos: 0 }
    }

    fn rest(&self) -> &'a str {
        &self.s[self.pos..]
    }

    fn ws(&mut self) {
        while self.rest().starts_with(' ') {
            self.pos += 1;
        }
    }

    fn at_end(&mut self) -> bool {
        self.ws();
        self.pos >= self.s.len()
    }

    fn eat(&mut self, t: &str) -> bool {
        self.ws();
        if self.rest().starts_with(t) {
            self.pos += t.len();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, t: &str) -> R<()> {
        if self.eat(t) { Ok(()) } else { Err(format!("expected `{t}` at `{}`", self.rest())) }
    }

    fn take_while(&mut self, f: impl Fn(char) -> bool) -> &'a str {
        self.ws();
        let start = self.pos;
        while let Some(c) = self.rest().chars().next() {
            if f(c) {
                self.pos += c.len_utf8();
            } else {
                break;
            }
        }
        &self.s[start..self.pos]
    }

    /// A function, type or intrinsic name.
    fn name(&mut self) -> R<String> {
        let n = self.take_while(|c| c.is_alphanumeric() || c == '_' || c == '.' || c == '#');
        if n.is_empty() { Err(format!("expected a name at `{}`", self.rest())) } else { Ok(n.to_string()) }
    }

    fn type_name(&mut self) -> R<String> {
        let n = self.take_while(|c| c.is_alphanumeric() || c == '_' || c == '.');
        if n.is_empty() {
            Err(format!("expected a type name at `{}`", self.rest()))
        } else {
            Ok(n.to_string())
        }
    }

    fn int(&mut self) -> R<i64> {
        self.ws();
        let start = self.pos;
        if self.rest().starts_with('-') {
            self.pos += 1;
        }
        while self.rest().chars().next().is_some_and(|c| c.is_ascii_digit()) {
            self.pos += 1;
        }
        self.s[start..self.pos].parse().map_err(|_| format!("expected an integer at `{}`", &self.s[start..]))
    }

    fn uint(&mut self) -> R<u32> {
        let n = self.int()?;
        u32::try_from(n).map_err(|_| format!("expected a small non-negative integer, found {n}"))
    }

    fn string(&mut self) -> R<String> {
        self.expect("\"")?;
        let mut out = String::new();
        loop {
            let Some(c) = self.rest().chars().next() else { return Err("unterminated string".into()) };
            self.pos += c.len_utf8();
            match c {
                '"' => return Ok(out),
                '\\' => out.push(self.escape()?),
                c => out.push(c),
            }
        }
    }

    fn escape(&mut self) -> R<char> {
        let Some(c) = self.rest().chars().next() else { return Err("bad escape".into()) };
        self.pos += c.len_utf8();
        Ok(match c {
            'n' => '\n',
            't' => '\t',
            'r' => '\r',
            '0' => '\0',
            '\\' => '\\',
            '"' => '"',
            '\'' => '\'',
            'u' => {
                self.expect("{")?;
                let hex = self.take_while(|c| c.is_ascii_hexdigit());
                let hex = hex.to_string();
                self.expect("}")?;
                u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32).ok_or("bad unicode escape")?
            }
            other => return Err(format!("unknown escape `\\{other}`")),
        })
    }

    fn char_lit(&mut self) -> R<char> {
        self.expect("'")?;
        let Some(c) = self.rest().chars().next() else { return Err("bad char".into()) };
        self.pos += c.len_utf8();
        let ch = if c == '\\' { self.escape()? } else { c };
        if !self.rest().starts_with('\'') {
            return Err("unterminated char".into());
        }
        self.pos += 1;
        Ok(ch)
    }

    fn local(&mut self) -> R<LocalId> {
        self.expect("_")?;
        Ok(LocalId(self.uint()?))
    }

    /// ` word 0 1 2`: indices after a keyword, if the keyword is next.
    fn indices(&mut self, word: &str) -> R<Vec<u32>> {
        let mut out = Vec::new();
        let save = self.pos;
        if !self.eat(word) {
            return Ok(out);
        }
        if !self.rest().starts_with(' ') {
            self.pos = save;
            return Ok(out);
        }
        loop {
            self.ws();
            if !self.rest().starts_with(|c: char| c.is_ascii_digit()) {
                break;
            }
            out.push(self.uint()?);
        }
        Ok(out)
    }

    fn block_id(&mut self) -> R<BlockId> {
        self.expect("bb")?;
        Ok(BlockId(self.uint()?))
    }

    /// `[A, B]` if next, else nothing.
    fn type_args(&mut self) -> R<Vec<Type>> {
        let mut args = Vec::new();
        if self.rest().starts_with('[') {
            self.pos += 1;
            loop {
                args.push(self.ty()?);
                if self.eat("]") {
                    break;
                }
                self.expect(",")?;
            }
        }
        Ok(args)
    }

    /// `[T, U]` after a declaration's name, if next.
    fn param_names(&mut self) -> R<Vec<String>> {
        let mut out = Vec::new();
        if self.rest().starts_with('[') {
            self.pos += 1;
            loop {
                out.push(self.type_name()?);
                if self.eat("]") {
                    break;
                }
                self.expect(",")?;
            }
        }
        Ok(out)
    }

    fn ty(&mut self) -> R<Type> {
        self.ws();
        if self.eat("()") {
            return Ok(Type::Unit);
        }
        if self.eat("$") {
            return Ok(Type::Param(self.uint()?));
        }
        if self.eat("dyn ") {
            let n = self.type_name()?;
            let args = self.type_args()?;
            return Ok(Type::Dyn(n, args));
        }
        if self.eat("<") {
            let t = self.ty()?;
            self.expect("as ")?;
            let tr = self.type_name()?;
            let args = self.type_args()?;
            self.expect(">.")?;
            let n = self.type_name()?;
            return Ok(Type::Assoc(Box::new(t), tr, args, n));
        }
        if self.eat("ref mut ") {
            return Ok(Type::Ref(Box::new(self.ty()?), true));
        }
        if self.eat("ref ") {
            return Ok(Type::Ref(Box::new(self.ty()?), false));
        }
        let pure = self.eat("pure fn(");
        if pure || self.eat("fn(") {
            let mut ps = Vec::new();
            if !self.eat(")") {
                loop {
                    ps.push(self.ty()?);
                    if self.eat(")") {
                        break;
                    }
                    self.expect(",")?;
                }
            }
            self.expect("->")?;
            return Ok(Type::Fn(ps, Box::new(self.ty()?), pure));
        }
        if self.eat("(") {
            let mut es = Vec::new();
            loop {
                es.push(self.ty()?);
                if self.eat(")") {
                    break;
                }
                self.expect(",")?;
                if self.eat(")") {
                    break;
                }
            }
            return Ok(Type::Tuple(es));
        }
        if self.eat("{") {
            let mut ms = Vec::new();
            if !self.eat("}") {
                loop {
                    ms.push(self.type_name()?);
                    if self.eat("}") {
                        break;
                    }
                    self.expect(",")?;
                }
            }
            return Ok(Type::ErrSet(ms));
        }
        let n = self.type_name()?;
        let t = match n.as_str() {
            "int" => Type::Int,
            "f64" => Type::Float,
            "bool" => Type::Bool,
            "char" => Type::Char,
            "str" => Type::Str,
            "Never" => Type::Never,
            _ => {
                let args = self.type_args()?;
                Type::Named(n, args)
            }
        };
        Ok(t)
    }

    fn place(&mut self) -> R<Place> {
        let local = self.local()?;
        let mut proj = Vec::new();
        loop {
            let r = self.rest();
            if r.starts_with(".*") {
                self.pos += 2;
                proj.push(Proj::Deref);
            } else if r.starts_with(".as(") {
                self.pos += 4;
                let v = self.uint()?;
                self.expect(")")?;
                proj.push(Proj::Downcast(v));
            } else if r.starts_with('.') && r[1..].starts_with(|c: char| c.is_ascii_digit()) {
                self.pos += 1;
                proj.push(Proj::Field(self.uint()?));
            } else if r.starts_with("[^") {
                self.pos += 2;
                let l = self.local()?;
                self.expect("]")?;
                proj.push(Proj::IndexFromEnd(l));
            } else if r.starts_with('[') {
                self.pos += 1;
                let l = self.local()?;
                self.expect("]")?;
                proj.push(Proj::Index(l));
            } else if r.starts_with('{') {
                self.pos += 1;
                let l = self.local()?;
                self.expect("}")?;
                proj.push(Proj::Key(l));
            } else {
                break;
            }
        }
        Ok(Place { local, proj })
    }

    fn constant(&mut self) -> R<Const> {
        self.ws();
        let r = self.rest();
        if r.starts_with('"') {
            return Ok(Const::Str(self.string()?));
        }
        if r.starts_with('\'') {
            return Ok(Const::Char(self.char_lit()?));
        }
        if self.eat("()") {
            return Ok(Const::Unit);
        }
        if self.eat("true") {
            return Ok(Const::Bool(true));
        }
        if self.eat("false") {
            return Ok(Const::Bool(false));
        }
        if self.eat("fn ") {
            return Ok(Const::Fn(self.name()?));
        }
        if self.eat("nan") {
            return Ok(Const::Float(f64::NAN));
        }
        if self.eat("inf") {
            return Ok(Const::Float(f64::INFINITY));
        }
        if self.eat("-inf") {
            return Ok(Const::Float(f64::NEG_INFINITY));
        }
        let num = self.take_while(|c| c.is_ascii_digit() || matches!(c, '-' | '+' | '.' | 'e' | 'E'));
        if num.contains('.') || num.contains('e') {
            num.parse().map(Const::Float).map_err(|_| format!("bad float `{num}`"))
        } else {
            num.parse().map(Const::Int).map_err(|_| format!("bad constant `{num}`"))
        }
    }

    fn operand(&mut self) -> R<Operand> {
        if self.eat("copy ") {
            Ok(Operand::Copy(self.place()?))
        } else if self.eat("move ") {
            Ok(Operand::Move(self.place()?))
        } else if self.eat("const ") {
            Ok(Operand::Const(self.constant()?))
        } else {
            Err(format!("expected an operand at `{}`", self.rest()))
        }
    }

    fn operands(&mut self) -> R<Vec<Operand>> {
        self.expect("(")?;
        let mut out = Vec::new();
        if self.eat(")") {
            return Ok(out);
        }
        loop {
            out.push(self.operand()?);
            if self.eat(")") {
                return Ok(out);
            }
            self.expect(",")?;
        }
    }

    fn rvalue(&mut self) -> R<Rvalue> {
        self.ws();
        let r = self.rest();
        if r.starts_with("copy ") || r.starts_with("move ") || r.starts_with("const ") {
            return Ok(Rvalue::Use(self.operand()?));
        }
        if self.eat("ref mut ") {
            return Ok(Rvalue::Ref(self.place()?, true));
        }
        if self.eat("ref ") {
            return Ok(Rvalue::Ref(self.place()?, false));
        }
        if self.eat("tuple") {
            return Ok(Rvalue::Aggregate(Aggregate::Tuple, self.operands()?));
        }
        if self.eat("struct ") {
            let n = self.type_name()?;
            return Ok(Rvalue::Aggregate(Aggregate::Struct(n), self.operands()?));
        }
        if self.eat("variant ") {
            let start = self.pos;
            let idx = self.rest().find("::").ok_or("expected `::` in a variant")?;
            let t = self.s[start..start + idx].to_string();
            self.pos = start + idx + 2;
            let v = self.name()?;
            return Ok(Rvalue::Aggregate(Aggregate::Variant(t, v), self.operands()?));
        }
        if self.eat("list") {
            return Ok(Rvalue::Aggregate(Aggregate::List, self.operands()?));
        }
        if self.eat("set") {
            return Ok(Rvalue::Aggregate(Aggregate::Set, self.operands()?));
        }
        if self.eat("dict") {
            return Ok(Rvalue::Aggregate(Aggregate::Dict, self.operands()?));
        }
        if self.eat("discriminant ") {
            return Ok(Rvalue::Discriminant(self.place()?));
        }
        if self.eat("closure ") {
            let n = self.name()?;
            let tys = self.type_args()?;
            return Ok(Rvalue::Closure(n, tys, self.operands()?));
        }
        if self.eat("dyn ") {
            let t = self.ty()?;
            return Ok(Rvalue::Aggregate(Aggregate::Dyn(t), self.operands()?));
        }
        if self.eat("copyof ") {
            return Ok(Rvalue::CopyOf(self.place()?));
        }
        if self.eat("strview ") {
            return Ok(Rvalue::StrView(self.place()?));
        }
        let word = self.take_while(|c| c.is_ascii_alphanumeric()).to_string();
        if let Some(op) = BinOp::from_name(&word) {
            let a = self.operand()?;
            self.expect(",")?;
            let b = self.operand()?;
            return Ok(Rvalue::Binary(op, a, b));
        }
        if let Some(op) = UnOp::from_name(&word) {
            return Ok(Rvalue::Unary(op, self.operand()?));
        }
        Err(format!("unknown rvalue `{word}`"))
    }
}

/// `L:C` as two numbers.
fn line_col(text: &str) -> Option<(u32, u32)> {
    let (l, c) = text.split_once(':')?;
    if l.is_empty() || c.is_empty() || !l.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    if !c.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    Some((l.parse().ok()?, c.parse().ok()?))
}

/// Splits a trailing ` @L:C` or ` @L:C-L:C` off a line.
fn split_loc(line: &str) -> (&str, Option<Loc>) {
    if let Some(i) = line.rfind(" @") {
        let tail = &line[i + 2..];
        let (start, end) = match tail.split_once('-') {
            Some((a, b)) => (a, Some(b)),
            None => (tail, None),
        };
        if let Some((l, c)) = line_col(start) {
            match end.map(line_col) {
                None => return (&line[..i], Some(Loc::point(l, c))),
                Some(Some((el, ec))) => {
                    return (&line[..i], Some(Loc { line: l, col: c, end_line: el, end_col: ec }));
                }
                Some(None) => {}
            }
        }
    }
    (line, None)
}

pub fn read(text: &str) -> Result<Program, ReadError> {
    let lines: Vec<&str> = text.lines().collect();
    let mut p = Program {
        sources: Vec::new(),
        entry: None,
        types: Vec::new(),
        intrinsics: Vec::new(),
        functions: Vec::new(),
        tests: Vec::new(),
        trait_methods: Vec::new(),
        impls: Vec::new(),
    };
    let mut i = 0;
    let err = |line: usize, message: String| ReadError { line: line + 1, message };
    let mut seen_header = false;
    while i < lines.len() {
        let line = lines[i].trim_end();
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            i += 1;
            continue;
        }
        let mut c = Cur::new(line);
        let res: R<()> = (|| {
            if c.eat("sir ") {
                let v = c.uint()?;
                if v != VERSION {
                    return Err(format!("SIR version {v} is not supported (this reader reads {VERSION})"));
                }
                seen_header = true;
            } else if c.eat("source ") {
                let n = c.uint()? as usize;
                let s = c.string()?;
                if n != p.sources.len() {
                    return Err("sources must be numbered in order".into());
                }
                p.sources.push(s);
            } else if c.eat("entry ") {
                let function = c.name()?;
                let mut e = Entry { function, takes_system: false, returns_result: false };
                while !c.at_end() {
                    if c.eat("system") {
                        e.takes_system = true;
                    } else if c.eat("result") {
                        e.returns_result = true;
                    } else {
                        return Err(format!("unknown entry flag `{}`", c.rest()));
                    }
                }
                p.entry = Some(e);
            } else if c.eat("test ") {
                let name = c.string()?;
                let f = c.name()?;
                p.tests.push((name, f));
            } else if c.eat("intrinsic ") || c.eat("method ") {
                let method = line.trim_start().starts_with("method ");
                let name = c.type_name()?;
                let pure = c.eat("pure");
                let from = c.indices("from")?;
                let inner = c.indices("inner")?;
                let own = c.indices("own")?;
                let keeps = c.indices("keeps")?;
                if !c.at_end() {
                    return Err(format!("unexpected `{}`", c.rest()));
                }
                let d = IntrinsicDecl { name, pure, from, inner, own, keeps };
                if method {
                    p.trait_methods.push(d);
                } else {
                    p.intrinsics.push(d);
                }
            } else if c.eat("impl") {
                let params = if c.rest().starts_with('[') {
                    c.pos += 1;
                    let n = c.uint()?;
                    c.expect("]")?;
                    n
                } else {
                    0
                };
                let trait_name = c.type_name()?;
                let trait_args = c.type_args()?;
                c.expect("for ")?;
                let self_ty = c.ty()?;
                c.expect("{")?;
                let mut methods = Vec::new();
                let mut assoc = Vec::new();
                if !c.eat("}") {
                    loop {
                        if c.eat("type ") {
                            let n = c.name()?;
                            c.expect("=")?;
                            assoc.push((n, c.ty()?));
                        } else {
                            let m = c.name()?;
                            c.expect(":")?;
                            let target = if c.eat("intrinsic ") {
                                ImplTarget::Intrinsic(c.type_name()?)
                            } else {
                                ImplTarget::Fn(c.name()?)
                            };
                            methods.push((m, target));
                        }
                        if c.eat("}") {
                            break;
                        }
                        c.expect(",")?;
                    }
                }
                p.impls.push(ImplDecl { params, trait_name, trait_args, self_ty, methods, assoc });
            } else if c.eat("struct ") || c.eat("view struct ") {
                let view = line.trim_start().starts_with("view ");
                let name = c.type_name()?;
                let params = c.param_names()?;
                let must_consume = c.eat("must_consume");
                let mut drop = None;
                if c.eat("drop ") {
                    drop = Some(c.name()?);
                }
                c.expect("{")?;
                let mut fields = Vec::new();
                if !c.eat("}") {
                    loop {
                        let fname = c.name()?;
                        c.expect(":")?;
                        let ty = c.ty()?;
                        let default = if c.eat("=") { Some(c.name()?) } else { None };
                        fields.push(FieldDecl { name: fname, ty, default });
                        if c.eat("}") {
                            break;
                        }
                        c.expect(",")?;
                    }
                }
                p.types.push(TypeDecl::Struct { name, params, fields, drop, view, must_consume });
            } else if c.eat("enum ") || c.eat("error ") {
                let error = line.trim_start().starts_with("error ");
                let name = c.type_name()?;
                let params = c.param_names()?;
                let open = c.eat("open");
                let must_consume = c.eat("must_consume");
                let mut drop = None;
                if c.eat("drop ") {
                    drop = Some(c.name()?);
                }
                c.expect("{")?;
                let mut variants = Vec::new();
                if !c.eat("}") {
                    loop {
                        let vname = c.name()?;
                        let mut fields = Vec::new();
                        if c.eat("(") {
                            loop {
                                // `name: T` or `T`.
                                let save = c.pos;
                                let n = c.take_while(|ch| ch.is_alphanumeric() || ch == '_').to_string();
                                let named = if !n.is_empty() && c.eat(":") {
                                    Some(n)
                                } else {
                                    c.pos = save;
                                    None
                                };
                                let ty = c.ty()?;
                                fields.push((named, ty));
                                if c.eat(")") {
                                    break;
                                }
                                c.expect(",")?;
                            }
                        }
                        variants.push(VariantDecl { name: vname, fields });
                        if c.eat("}") {
                            break;
                        }
                        c.expect(",")?;
                    }
                }
                p.types.push(TypeDecl::Enum { name, params, variants, drop, error, open, must_consume });
            } else if c.eat("fn ") {
                return Err("FN".into());
            } else {
                return Err(format!("unknown line `{line}`"));
            }
            Ok(())
        })();
        match res {
            Ok(()) => i += 1,
            Err(e) if e == "FN" => {
                let (f, next) = read_function(&lines, i).map_err(|(l, m)| err(l, m))?;
                p.functions.push(f);
                i = next;
            }
            Err(e) => return Err(err(i, e)),
        }
    }
    if !seen_header {
        return Err(ReadError { line: 1, message: "missing `sir 0` header".into() });
    }
    Ok(p)
}

fn read_function(lines: &[&str], start: usize) -> Result<(Function, usize), (usize, String)> {
    let header = lines[start].trim();
    // `fn ... @L:C-L:C {`: the declaration's location comes before the brace.
    let (header, fn_loc) = match header.strip_suffix(" {") {
        Some(h) => {
            let (h, l) = split_loc(h);
            (h, l)
        }
        None => (header, None),
    };
    let mut c = Cur::new(header);
    type Header = (String, Vec<String>, u32, FnKind, bool, u32, Vec<u32>, Vec<u32>, Vec<u32>);
    let parse_header = |c: &mut Cur| -> R<Header> {
        c.expect("fn ")?;
        let name = c.name()?;
        let tparams = c.param_names()?;
        c.expect("(")?;
        let n = c.uint()?;
        c.expect(")")?;
        let kind = if c.eat("closure") {
            FnKind::Closure
        } else if c.eat("drop") {
            FnKind::Drop
        } else if c.eat("glue") {
            FnKind::Glue
        } else if c.eat("comptime ") {
            let n = c.int()?;
            FnKind::Comptime(u64::try_from(n).map_err(|_| format!("a negative budget, {n}"))?)
        } else if c.eat("value") {
            FnKind::Value
        } else if c.eat("test") {
            FnKind::Test
        } else {
            FnKind::Normal
        };
        let pure = c.eat("pure");
        c.expect("source ")?;
        let src = c.uint()?;
        let from = c.indices("from")?;
        let inner = c.indices("inner")?;
        let own = c.indices("own")?;
        if !c.at_end() {
            return Err(format!("expected `{{`, found `{}`", c.rest()));
        }
        Ok((name, tparams, n, kind, pure, src, from, inner, own))
    };
    if !lines[start].trim().ends_with(" {") {
        return Err((start, "expected `{` at the end of the function header".into()));
    }
    let (name, type_params, param_count, kind, pure, source, from, inner, own) =
        parse_header(&mut c).map_err(|e| (start, e))?;
    let mut f = Function {
        name,
        type_params,
        kind,
        param_count,
        pure,
        from,
        inner,
        own,
        locals: Vec::new(),
        blocks: Vec::new(),
        source,
        loc: fn_loc,
    };
    let mut i = start + 1;
    let mut current: Option<BasicBlock> = None;
    while i < lines.len() {
        let raw = lines[i].trim();
        if raw.is_empty() {
            i += 1;
            continue;
        }
        if raw == "}" {
            if current.is_some() {
                return Err((i, "block without a terminator".into()));
            }
            return Ok((f, i + 1));
        }
        let (line, loc) = split_loc(raw);
        let mut c = Cur::new(line);
        let res: R<()> = (|| {
            if c.eat("let ") {
                let l = c.local()?;
                if l.0 as usize != f.locals.len() {
                    return Err("locals must be numbered in order".into());
                }
                c.expect(":")?;
                let ty = c.ty()?;
                let name = if c.rest().trim_start().starts_with('"') { Some(c.string()?) } else { None };
                let once = c.eat("once");
                if !c.at_end() {
                    return Err(format!("unexpected `{}`", c.rest()));
                }
                f.locals.push(LocalDecl { ty, name, loc, once });
                return Ok(());
            }
            if line.starts_with("bb") && line.ends_with(':') {
                if current.is_some() {
                    return Err("block without a terminator".into());
                }
                let id = c.block_id()?;
                if id.0 as usize != f.blocks.len() {
                    return Err("blocks must be numbered in order".into());
                }
                let mut b = BasicBlock {
                    stmts: Vec::new(),
                    term: Terminator::Unreachable,
                    unwind: None,
                    cleanup: false,
                    loc: None,
                };
                if c.eat("(cleanup)") {
                    b.cleanup = true;
                } else if c.eat("(unwind ") {
                    b.unwind = Some(c.block_id()?);
                    c.expect(")")?;
                }
                c.expect(":")?;
                current = Some(b);
                return Ok(());
            }
            let Some(b) = current.as_mut() else { return Err("statement outside a block".into()) };
            // Terminators.
            let term = if c.eat("goto ") {
                Some(Terminator::Goto(c.block_id()?))
            } else if c.eat("switch ") {
                let value = c.operand()?;
                c.expect("[")?;
                let mut cases = Vec::new();
                let default;
                loop {
                    if c.eat("else:") {
                        default = c.block_id()?;
                        c.expect("]")?;
                        break;
                    }
                    let v = c.int()?;
                    c.expect(":")?;
                    cases.push((v, c.block_id()?));
                    c.expect(",")?;
                }
                Some(Terminator::Switch { value, cases, default })
            } else if line == "return" {
                Some(Terminator::Return)
            } else if line == "resume" {
                Some(Terminator::Resume)
            } else if line == "unreachable" {
                Some(Terminator::Unreachable)
            } else {
                None
            };
            if let Some(t) = term {
                b.term = t;
                b.loc = loc;
                if let Some(done) = current.take() {
                    f.blocks.push(done);
                }
                return Ok(());
            }
            let kind = if c.eat("drop ") {
                let place = c.place()?;
                let flag = if c.eat("if ") { Some(c.local()?) } else { None };
                StatementKind::Drop { place, flag }
            } else if c.eat("dead ") {
                StatementKind::StorageDead(c.local()?)
            } else {
                let dest = c.place()?;
                c.expect("=")?;
                if c.eat("call ") {
                    let callee = if c.eat("fn ") {
                        let n = c.name()?;
                        Callee::Fn(n, c.type_args()?)
                    } else if c.eat("intrinsic ") {
                        let n = c.type_name()?;
                        Callee::Intrinsic(n, c.type_args()?)
                    } else if c.eat("trait ") {
                        let n = c.type_name()?;
                        Callee::Trait(n, c.type_args()?)
                    } else {
                        Callee::Value(c.operand()?)
                    };
                    let args = c.operands()?;
                    StatementKind::Call { dest, callee, args }
                } else {
                    StatementKind::Assign(dest, c.rvalue()?)
                }
            };
            if !c.at_end() {
                return Err(format!("unexpected `{}`", c.rest()));
            }
            b.stmts.push(Statement { kind, loc });
            Ok(())
        })();
        res.map_err(|e| (i, e))?;
        i += 1;
    }
    Err((i, "function is never closed".into()))
}
