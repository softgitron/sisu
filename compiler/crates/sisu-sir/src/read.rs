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

    fn block_id(&mut self) -> R<BlockId> {
        self.expect("bb")?;
        Ok(BlockId(self.uint()?))
    }

    fn ty(&mut self) -> R<Type> {
        self.ws();
        if self.eat("()") {
            return Ok(Type::Unit);
        }
        if self.eat("ref mut ") {
            return Ok(Type::Ref(Box::new(self.ty()?), true));
        }
        if self.eat("ref ") {
            return Ok(Type::Ref(Box::new(self.ty()?), false));
        }
        if self.eat("fn(") {
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
            return Ok(Type::Fn(ps, Box::new(self.ty()?)));
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
            return Ok(Rvalue::Closure(n, self.operands()?));
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

/// Splits a trailing ` @L:C` off a line.
fn split_loc(line: &str) -> (&str, Option<Loc>) {
    if let Some(i) = line.rfind(" @") {
        let tail = &line[i + 2..];
        if let Some((l, c)) = tail.split_once(':')
            && let (Ok(line_no), Ok(col)) = (l.parse(), c.parse())
            && !l.is_empty()
            && !c.is_empty()
            && tail.chars().all(|ch| ch.is_ascii_digit() || ch == ':')
        {
            return (&line[..i], Some(Loc { line: line_no, col }));
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
        functions: Vec::new(),
        tests: Vec::new(),
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
            } else if c.eat("struct ") {
                let name = c.type_name()?;
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
                p.types.push(TypeDecl::Struct { name, fields, drop });
            } else if c.eat("enum ") || c.eat("error ") {
                let error = line.trim_start().starts_with("error ");
                let name = c.type_name()?;
                let open = c.eat("open");
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
                p.types.push(TypeDecl::Enum { name, variants, drop, error, open });
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
    let mut c = Cur::new(header);
    let parse_header = |c: &mut Cur| -> R<(String, u32, FnKind, u32)> {
        c.expect("fn ")?;
        let name = c.name()?;
        c.expect("(")?;
        let n = c.uint()?;
        c.expect(")")?;
        let kind = if c.eat("closure") {
            FnKind::Closure
        } else if c.eat("drop") {
            FnKind::Drop
        } else if c.eat("value") {
            FnKind::Value
        } else if c.eat("test") {
            FnKind::Test
        } else {
            FnKind::Normal
        };
        c.expect("source ")?;
        let src = c.uint()?;
        c.expect("{")?;
        Ok((name, n, kind, src))
    };
    let (name, param_count, kind, source) = parse_header(&mut c).map_err(|e| (start, e))?;
    let mut f = Function { name, kind, param_count, locals: Vec::new(), blocks: Vec::new(), source };
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
                f.locals.push(LocalDecl { ty, name });
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
            } else {
                let dest = c.place()?;
                c.expect("=")?;
                if c.eat("call ") {
                    let callee = if c.eat("fn ") {
                        Callee::Fn(c.name()?)
                    } else if c.eat("intrinsic ") {
                        let n = c.type_name()?;
                        let mut tys = Vec::new();
                        if c.rest().starts_with('[') {
                            c.pos += 1;
                            loop {
                                tys.push(c.ty()?);
                                if c.eat("]") {
                                    break;
                                }
                                c.expect(",")?;
                            }
                        }
                        Callee::Intrinsic(n, tys)
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
