//! LIR, the backend-neutral low IR that stage 2 (`compiler/stage2`, the
//! lowering of SIR written in Sisu) writes, read from its text form
//! (`compiler/stage2/README.md`, "LIR"; docs/architecture.md, "LIR"). The
//! reader keeps each function's operations in the order they were written,
//! which [`crate::translate`] replays one by one through Cranelift's
//! function builder.

use crate::Error;
use cranelift_codegen::ir::condcodes::{FloatCC, IntCC};
use cranelift_codegen::ir::types;

/// A value of a function: its number, counting the function's parameters
/// first and then each value an operation defines, in order.
pub(crate) type V = u32;

/// A whole LIR program.
pub(crate) struct Lir {
    pub release: bool,
    /// The program's files, as SIR numbers them.
    pub sources: Vec<String>,
    /// The declarations, data and functions, in the order of the object.
    pub items: Vec<Item>,
    /// The descriptions of the named locals' types (`type tN ...`).
    pub types: Vec<DType>,
    pub stats: Stats,
}

/// What `stats` says of the lowering.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Stats {
    pub functions: usize,
    pub statements: usize,
    pub index: usize,
    pub index_proven: usize,
    pub overflow: usize,
    pub overflow_proven: usize,
    pub tests_decided: usize,
}

pub(crate) enum Item {
    /// `data dN import NAME`: one of the runtime's (writable) data symbols.
    Import(String),
    /// `data dN [align A] (bytes|u32|zeros) ...`: read-only data, with the
    /// words that hold functions' addresses.
    Data {
        bytes: Vec<u8>,
        align: Option<u64>,
        funcs: Vec<(u32, String)>,
    },
    Declare {
        symbol: String,
        linkage: cranelift_module::Linkage,
        sig: Sig,
    },
    Function(Box<Func>),
}

/// A signature: Sisu's convention with `N` arguments (`N + 1` addresses
/// and the status), or the types of the parameters and of the results.
#[derive(Clone, Debug)]
pub(crate) enum Sig {
    Sisu(u32),
    Plain(Vec<types::Type>, Vec<types::Type>),
}

/// How a function's frame counts toward the probe reserve (decision 234).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FrameKind {
    Probed,
    Unprobed,
    Outside,
}

pub(crate) struct Func {
    pub symbol: String,
    pub frame: FrameKind,
    pub name: String,
    pub source: Option<u32>,
    pub line: u32,
    /// The size of each slot of the frame, in order.
    pub slots: Vec<u32>,
    pub vars: u32,
    pub locals: Vec<Local>,
    pub ops: Vec<Op>,
    /// Whether it uses `probe_reserve`: it is written once every function
    /// before it is compiled.
    pub reserve: bool,
}

/// A named local, for the debug information.
pub(crate) struct Local {
    pub name: String,
    pub ty: u32,
    pub slot: u32,
    pub indirect: bool,
    pub param: bool,
    pub line: Option<u32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Bin {
    Iadd,
    Isub,
    Imul,
    Sdiv,
    Srem,
    Ishl,
    Sshr,
    Band,
    Bor,
    Bxor,
    BandNot,
    Fadd,
    Fsub,
    Fmul,
    Fdiv,
}

/// An operation with a constant operand (`iadd_imm` and the others).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Imm {
    Iadd,
    Imul,
    Bxor,
    Bor,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Un {
    Ineg,
    Bnot,
    Fneg,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Ovf {
    Sadd,
    Ssub,
    Smul,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Conv {
    Uextend,
    Sextend,
    Ireduce,
    Bitcast,
    Fpromote,
    Fdemote,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Load {
    Word(types::Type),
    U8,
    S8,
}

/// One operation of a function, in the order written.
#[derive(Clone, Debug)]
pub(crate) enum Op {
    Block(u32, bool),
    Loc(u32, u32),
    Iconst(types::Type, i64),
    Binary(Bin, V, V),
    BinaryImm(Imm, V, i64),
    Unary(Un, V),
    Icmp(IntCC, V, V),
    IcmpImm(IntCC, V, i64),
    Fcmp(FloatCC, V, V),
    Overflow(Ovf, V, V),
    Select(V, V, V),
    Convert(Conv, types::Type, V),
    Load(Load, V, i32),
    Store(V, V, i32),
    StackAddr(u32),
    StackLoad(u32),
    StackStore(V, u32),
    SymbolValue(u32),
    FuncAddr(String),
    GetStackPointer,
    /// A call, and how many values its line defines.
    Call(String, Vec<V>, u32),
    CallIndirect(u32, V, Vec<V>, u32),
    Memcpy(V, V, V),
    Use(u32),
    Def(u32, V),
    Param(u32),
    ProbeReserve,
    Jump(u32, Vec<V>),
    Brif(V, u32, Vec<V>, u32, Vec<V>),
    Switch(V, u32, Vec<(u64, u32)>, u32),
    Return(Vec<V>),
    Trap(u8),
}

/// A struct's or a variant's fields as LIR describes them: each one's
/// name, the number of its type's description, and its offset.
pub(crate) type DFields = Vec<(String, u32, u32)>;

/// The description of a Sisu type for the debug information
/// (`compiler/stage2/README.md`, "Debug types").
#[derive(Clone, Debug)]
pub(crate) enum DType {
    Base { name: String, encoding: gimli::DwAte, size: u32 },
    Str,
    Struct { name: String, size: u32, fields: DFields },
    Tuple { name: String, size: u32, items: Vec<(u32, u32, u32)> },
    Pointer { name: String, to: u32 },
    Text { name: String, size: u32 },
    List { name: String, size: u32, slice: String, elem: u32 },
    Enum { name: String, size: u32, tag: u32, head: Vec<(String, u32)>, variants: Vec<(String, DFields)> },
    ErrSet { name: String, size: u32, members: Vec<(u32, String, u32)> },
    Alias(u32),
    Opaque { name: String, size: u32 },
    Word(String),
}

type R<T> = Result<T, String>;

/// A cursor over one line.
struct Cur<'a> {
    s: &'a [u8],
    pos: usize,
}

impl<'a> Cur<'a> {
    fn new(s: &'a str) -> Cur<'a> {
        Cur { s: s.as_bytes(), pos: 0 }
    }

    fn ws(&mut self) {
        while self.pos < self.s.len() && self.s[self.pos] == b' ' {
            self.pos += 1;
        }
    }

    fn at_end(&mut self) -> bool {
        self.ws();
        self.pos >= self.s.len()
    }

    fn rest(&self) -> String {
        String::from_utf8_lossy(&self.s[self.pos.min(self.s.len())..]).into_owned()
    }

    fn peek(&mut self) -> Option<u8> {
        self.ws();
        self.s.get(self.pos).copied()
    }

    fn eat(&mut self, t: &str) -> bool {
        self.ws();
        if self.s[self.pos..].starts_with(t.as_bytes()) {
            self.pos += t.len();
            true
        } else {
            false
        }
    }

    /// A keyword followed by a space or the end of the line.
    fn keyword(&mut self, t: &str) -> bool {
        self.ws();
        let rest = &self.s[self.pos..];
        if rest.starts_with(t.as_bytes()) && rest.get(t.len()).is_none_or(|c| *c == b' ') {
            self.pos += t.len();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, t: &str) -> R<()> {
        if self.eat(t) { Ok(()) } else { Err(format!("expected `{t}` at `{}`", self.rest())) }
    }

    /// A word: letters, digits and `_ $ . # -`, up to a space or a mark.
    fn word(&mut self) -> R<String> {
        self.ws();
        let start = self.pos;
        while self.pos < self.s.len() {
            let c = self.s[self.pos];
            if c.is_ascii_alphanumeric() || matches!(c, b'_' | b'$' | b'.' | b'#' | b'-') {
                self.pos += 1;
            } else {
                break;
            }
        }
        if start == self.pos {
            return Err(format!("expected a word at `{}`", self.rest()));
        }
        Ok(String::from_utf8_lossy(&self.s[start..self.pos]).into_owned())
    }

    fn int(&mut self) -> R<i64> {
        self.ws();
        let start = self.pos;
        if self.pos < self.s.len() && self.s[self.pos] == b'-' {
            self.pos += 1;
        }
        while self.pos < self.s.len() && self.s[self.pos].is_ascii_digit() {
            self.pos += 1;
        }
        std::str::from_utf8(&self.s[start..self.pos])
            .ok()
            .and_then(|t| t.parse().ok())
            .ok_or_else(|| format!("expected an integer at `{}`", self.rest()))
    }

    fn uint(&mut self) -> R<u32> {
        let n = self.int()?;
        u32::try_from(n).map_err(|_| format!("expected a small number, found {n}"))
    }

    /// `PREFIX` and a number: `v12`, `b3`, `s0`.
    fn numbered(&mut self, prefix: u8) -> R<u32> {
        self.ws();
        if self.s.get(self.pos) != Some(&prefix) {
            return Err(format!("expected `{}N` at `{}`", prefix as char, self.rest()));
        }
        self.pos += 1;
        self.uint()
    }

    fn value(&mut self) -> R<V> {
        self.numbered(b'v')
    }

    /// Quoted bytes: `\\`, `\"` and `\xHH` escapes.
    fn bytes(&mut self) -> R<Vec<u8>> {
        self.expect("\"")?;
        let mut out = Vec::new();
        loop {
            let Some(&c) = self.s.get(self.pos) else { return Err("unterminated text".into()) };
            self.pos += 1;
            match c {
                b'"' => return Ok(out),
                b'\\' => {
                    let Some(&e) = self.s.get(self.pos) else { return Err("bad escape".into()) };
                    self.pos += 1;
                    match e {
                        b'\\' | b'"' => out.push(e),
                        b'x' => {
                            let hex = self.s.get(self.pos..self.pos + 2).ok_or("bad escape")?;
                            let hex = std::str::from_utf8(hex).map_err(|_| "bad escape")?;
                            out.push(u8::from_str_radix(hex, 16).map_err(|_| "bad escape")?);
                            self.pos += 2;
                        }
                        other => return Err(format!("unknown escape `\\{}`", other as char)),
                    }
                }
                c => out.push(c),
            }
        }
    }

    fn text(&mut self) -> R<String> {
        let b = self.bytes()?;
        String::from_utf8(b).map_err(|_| "a text that is not UTF-8".to_string())
    }

    /// `(v1, v2)` after a block or a callee, if next.
    fn args(&mut self) -> R<Vec<V>> {
        let mut out = Vec::new();
        if self.s.get(self.pos) != Some(&b'(') {
            return Ok(out);
        }
        self.pos += 1;
        if self.eat(")") {
            return Ok(out);
        }
        loop {
            out.push(self.value()?);
            if self.eat(")") {
                return Ok(out);
            }
            self.expect(",")?;
        }
    }

    /// A block and the values it is given: `b4` or `b4(v1, v2)`.
    fn target(&mut self) -> R<(u32, Vec<V>)> {
        let b = self.numbered(b'b')?;
        Ok((b, self.args()?))
    }

    fn ty(&mut self) -> R<types::Type> {
        let w = self.word()?;
        type_named(&w).ok_or_else(|| format!("unknown type `{w}`"))
    }

    /// `(T, ...)`: types.
    fn type_list(&mut self) -> R<Vec<types::Type>> {
        self.expect("(")?;
        let mut out = Vec::new();
        if self.eat(")") {
            return Ok(out);
        }
        loop {
            out.push(self.ty()?);
            if self.eat(")") {
                return Ok(out);
            }
            self.expect(",")?;
        }
    }
}

fn type_named(w: &str) -> Option<types::Type> {
    Some(match w {
        "i8" => types::I8,
        "i16" => types::I16,
        "i32" => types::I32,
        "i64" => types::I64,
        "f32" => types::F32,
        "f64" => types::F64,
        _ => return None,
    })
}

fn int_cc(w: &str) -> Option<IntCC> {
    Some(match w {
        "eq" => IntCC::Equal,
        "ne" => IntCC::NotEqual,
        "slt" => IntCC::SignedLessThan,
        "sle" => IntCC::SignedLessThanOrEqual,
        "sgt" => IntCC::SignedGreaterThan,
        "sge" => IntCC::SignedGreaterThanOrEqual,
        "ult" => IntCC::UnsignedLessThan,
        "ule" => IntCC::UnsignedLessThanOrEqual,
        "ugt" => IntCC::UnsignedGreaterThan,
        "uge" => IntCC::UnsignedGreaterThanOrEqual,
        _ => return None,
    })
}

fn float_cc(w: &str) -> Option<FloatCC> {
    Some(match w {
        "eq" => FloatCC::Equal,
        "ne" => FloatCC::NotEqual,
        "lt" => FloatCC::LessThan,
        "le" => FloatCC::LessThanOrEqual,
        "gt" => FloatCC::GreaterThan,
        "ge" => FloatCC::GreaterThanOrEqual,
        _ => return None,
    })
}

/// `name.SUFFIX` split at its last dot.
fn suffixed(w: &str) -> (&str, Option<&str>) {
    match w.rsplit_once('.') {
        Some((a, b)) => (a, Some(b)),
        None => (w, None),
    }
}

/// Reads the text form; an error names its line.
pub(crate) fn read(text: &str) -> Result<Lir, Error> {
    let mut r =
        Reader { lir: None, func: None, next_value: 0, data: 0, params: std::collections::HashMap::new() };
    for (i, line) in text.lines().enumerate() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        r.line(t).map_err(|e| Error::internal(format!("LIR line {}: {e}", i + 1)))?;
    }
    if r.func.is_some() {
        return Err(Error::internal("LIR: a function without its `end`".to_string()));
    }
    r.lir.ok_or_else(|| Error::internal("LIR: no `lir 1` header".to_string()))
}

struct Reader {
    lir: Option<Lir>,
    func: Option<Func>,
    /// The number the next value defined must have.
    next_value: u32,
    /// The next data number.
    data: u32,
    /// How many parameters each declared function has: its entry's values.
    params: std::collections::HashMap<String, u32>,
}

impl Reader {
    fn lir(&mut self) -> R<&mut Lir> {
        self.lir.as_mut().ok_or_else(|| "no `lir 1` header".to_string())
    }

    fn line(&mut self, t: &str) -> R<()> {
        let mut c = Cur::new(t);
        if self.func.is_some() {
            return self.in_function(t, &mut c);
        }
        if c.keyword("lir") {
            if c.uint()? != 1 {
                return Err("LIR version is not 1".into());
            }
            self.lir = Some(Lir {
                release: false,
                sources: Vec::new(),
                items: Vec::new(),
                types: Vec::new(),
                stats: Stats::default(),
            });
            return Ok(());
        }
        if c.keyword("profile") {
            let p = c.word()?;
            self.lir()?.release = match p.as_str() {
                "debug" => false,
                "release" => true,
                other => return Err(format!("unknown profile `{other}`")),
            };
            return Ok(());
        }
        if c.keyword("source") {
            let n = c.uint()? as usize;
            let s = c.text()?;
            let lir = self.lir()?;
            if n != lir.sources.len() {
                return Err("sources must be numbered in order".into());
            }
            lir.sources.push(s);
            return Ok(());
        }
        if c.keyword("data") {
            let d = c.numbered(b'd')?;
            if d != self.data {
                return Err(format!("data must be numbered in order: `d{d}`, expected `d{}`", self.data));
            }
            self.data += 1;
            let item = if c.keyword("import") {
                Item::Import(c.word()?)
            } else {
                let align = if c.keyword("align") { Some(c.int()? as u64) } else { None };
                let mut funcs = Vec::new();
                let bytes = if c.keyword("bytes") {
                    c.bytes()?
                } else if c.keyword("u32") {
                    let mut out = Vec::new();
                    while !c.at_end() {
                        out.extend_from_slice(&c.uint()?.to_le_bytes());
                    }
                    out
                } else if c.keyword("zeros") {
                    let n = c.uint()? as usize;
                    while c.keyword("func") {
                        let off = c.uint()?;
                        funcs.push((off, c.word()?));
                    }
                    vec![0u8; n]
                } else {
                    return Err(format!("unknown data `{}`", c.rest()));
                };
                Item::Data { bytes, align, funcs }
            };
            if !c.at_end() {
                return Err(format!("unexpected `{}`", c.rest()));
            }
            self.lir()?.items.push(item);
            return Ok(());
        }
        if c.keyword("declare") {
            let symbol = c.word()?;
            let linkage = match c.word()?.as_str() {
                "local" => cranelift_module::Linkage::Local,
                "import" => cranelift_module::Linkage::Import,
                "export" => cranelift_module::Linkage::Export,
                other => return Err(format!("unknown linkage `{other}`")),
            };
            let sig = if c.keyword("sisu") {
                Sig::Sisu(c.uint()?)
            } else {
                let params = c.type_list()?;
                c.expect("->")?;
                Sig::Plain(params, c.type_list()?)
            };
            let n = match &sig {
                Sig::Sisu(n) => n + 1,
                Sig::Plain(p, _) => p.len() as u32,
            };
            self.params.insert(symbol.clone(), n);
            self.lir()?.items.push(Item::Declare { symbol, linkage, sig });
            return Ok(());
        }
        if c.keyword("function") {
            let symbol = c.word()?;
            let frame = match c.word()?.as_str() {
                "probed" => FrameKind::Probed,
                "unprobed" => FrameKind::Unprobed,
                "outside" => FrameKind::Outside,
                other => return Err(format!("unknown frame `{other}`")),
            };
            c.expect("name")?;
            let name = c.text()?;
            let (mut source, mut line) = (None, 0);
            if c.keyword("source") {
                source = Some(c.uint()?);
                c.expect("line")?;
                line = c.uint()?;
            }
            self.next_value = *self
                .params
                .get(&symbol)
                .ok_or_else(|| format!("`{symbol}` is defined before it is declared"))?;
            self.func = Some(Func {
                symbol,
                frame,
                name,
                source,
                line,
                slots: Vec::new(),
                vars: 0,
                locals: Vec::new(),
                ops: Vec::new(),
                reserve: false,
            });
            return Ok(());
        }
        if c.keyword("types") {
            return Ok(());
        }
        if c.keyword("type") {
            let n = c.numbered(b't')?;
            let ty = dtype(&mut c)?;
            let lir = self.lir()?;
            if n as usize != lir.types.len() {
                return Err("types must be numbered in order".into());
            }
            lir.types.push(ty);
            return Ok(());
        }
        if c.keyword("stats") {
            let mut n = [0usize; 7];
            for x in &mut n {
                *x = c.int()? as usize;
            }
            self.lir()?.stats = Stats {
                functions: n[0],
                statements: n[1],
                index: n[2],
                index_proven: n[3],
                overflow: n[4],
                overflow_proven: n[5],
                tests_decided: n[6],
            };
            return Ok(());
        }
        Err(format!("unknown line `{t}`"))
    }

    fn in_function(&mut self, t: &str, c: &mut Cur<'_>) -> R<()> {
        if t == "end" {
            if let Some(f) = self.func.take() {
                self.lir()?.items.push(Item::Function(Box::new(f)));
            }
            return Ok(());
        }
        let Some(f) = self.func.as_mut() else { return Err("no function".into()) };
        if c.keyword("slot") {
            let s = c.numbered(b's')?;
            if s as usize != f.slots.len() {
                return Err("slots must be numbered in order".into());
            }
            f.slots.push(c.uint()?);
            return Ok(());
        }
        if c.keyword("vars") {
            f.vars = c.uint()?;
            return Ok(());
        }
        if c.keyword("local") {
            let name = c.text()?;
            let ty = c.numbered(b't')?;
            let slot = c.numbered(b's')?;
            let indirect = c.keyword("indirect");
            let param = c.keyword("param");
            let line = if c.keyword("line") { Some(c.uint()?) } else { None };
            f.locals.push(Local { name, ty, slot, indirect, param, line });
            return Ok(());
        }
        if c.keyword("block") {
            let b = c.numbered(b'b')?;
            let cold = c.keyword("cold");
            if f.ops.is_empty() && b != 0 {
                return Err("the first block written must be `b0`, the entry".into());
            }
            f.ops.push(Op::Block(b, cold));
            return Ok(());
        }
        // The values the line defines.
        let mut defined: Vec<u32> = Vec::new();
        if c.peek() == Some(b'v') {
            loop {
                defined.push(c.value()?);
                if c.eat("=") {
                    break;
                }
                c.expect(",")?;
            }
        }
        let op_word = c.word()?;
        let (op, suffix) = suffixed(&op_word);
        let n_defined = defined.len() as u32;
        let op = match (op, suffix) {
            ("loc", None) => {
                let l = c.uint()?;
                Op::Loc(l, c.uint()?)
            }
            ("iconst", Some(t)) => Op::Iconst(type_named(t).ok_or("bad type")?, c.int()?),
            (
                "iadd" | "isub" | "imul" | "sdiv" | "srem" | "ishl" | "sshr" | "band" | "bor" | "bxor"
                | "band_not" | "fadd" | "fsub" | "fmul" | "fdiv",
                None,
            ) => {
                let b = match op {
                    "iadd" => Bin::Iadd,
                    "isub" => Bin::Isub,
                    "imul" => Bin::Imul,
                    "sdiv" => Bin::Sdiv,
                    "srem" => Bin::Srem,
                    "ishl" => Bin::Ishl,
                    "sshr" => Bin::Sshr,
                    "band" => Bin::Band,
                    "bor" => Bin::Bor,
                    "bxor" => Bin::Bxor,
                    "band_not" => Bin::BandNot,
                    "fadd" => Bin::Fadd,
                    "fsub" => Bin::Fsub,
                    "fmul" => Bin::Fmul,
                    _ => Bin::Fdiv,
                };
                let x = c.value()?;
                c.expect(",")?;
                Op::Binary(b, x, c.value()?)
            }
            ("iadd_imm" | "imul_imm" | "bxor_imm" | "bor_imm", None) => {
                let k = match op {
                    "iadd_imm" => Imm::Iadd,
                    "imul_imm" => Imm::Imul,
                    "bxor_imm" => Imm::Bxor,
                    _ => Imm::Bor,
                };
                let x = c.value()?;
                c.expect(",")?;
                Op::BinaryImm(k, x, c.int()?)
            }
            ("ineg", None) => Op::Unary(Un::Ineg, c.value()?),
            ("bnot", None) => Op::Unary(Un::Bnot, c.value()?),
            ("fneg", None) => Op::Unary(Un::Fneg, c.value()?),
            ("icmp", Some(cc)) => {
                let cc = int_cc(cc).ok_or("bad condition")?;
                let x = c.value()?;
                c.expect(",")?;
                Op::Icmp(cc, x, c.value()?)
            }
            ("icmp_imm", Some(cc)) => {
                let cc = int_cc(cc).ok_or("bad condition")?;
                let x = c.value()?;
                c.expect(",")?;
                Op::IcmpImm(cc, x, c.int()?)
            }
            ("fcmp", Some(cc)) => {
                let cc = float_cc(cc).ok_or("bad condition")?;
                let x = c.value()?;
                c.expect(",")?;
                Op::Fcmp(cc, x, c.value()?)
            }
            ("sadd_overflow" | "ssub_overflow" | "smul_overflow", None) => {
                let k = match op {
                    "sadd_overflow" => Ovf::Sadd,
                    "ssub_overflow" => Ovf::Ssub,
                    _ => Ovf::Smul,
                };
                let x = c.value()?;
                c.expect(",")?;
                Op::Overflow(k, x, c.value()?)
            }
            ("select", None) => {
                let a = c.value()?;
                c.expect(",")?;
                let x = c.value()?;
                c.expect(",")?;
                Op::Select(a, x, c.value()?)
            }
            ("uextend" | "sextend" | "ireduce" | "bitcast" | "fpromote" | "fdemote", Some(t)) => {
                let k = match op {
                    "uextend" => Conv::Uextend,
                    "sextend" => Conv::Sextend,
                    "ireduce" => Conv::Ireduce,
                    "bitcast" => Conv::Bitcast,
                    "fpromote" => Conv::Fpromote,
                    _ => Conv::Fdemote,
                };
                Op::Convert(k, type_named(t).ok_or("bad type")?, c.value()?)
            }
            ("load" | "uload8" | "sload8", t) => {
                let k = match (op, t) {
                    ("load", Some(t)) => Load::Word(type_named(t).ok_or("bad type")?),
                    ("uload8", None) => Load::U8,
                    ("sload8", None) => Load::S8,
                    _ => return Err(format!("bad load `{op_word}`")),
                };
                let a = c.value()?;
                c.expect(",")?;
                Op::Load(k, a, c.int()? as i32)
            }
            ("store", None) => {
                let v = c.value()?;
                c.expect(",")?;
                let a = c.value()?;
                c.expect(",")?;
                Op::Store(v, a, c.int()? as i32)
            }
            ("stack_addr", None) => Op::StackAddr(c.numbered(b's')?),
            ("stack_load", None) => Op::StackLoad(c.numbered(b's')?),
            ("stack_store", None) => {
                let v = c.value()?;
                c.expect(",")?;
                Op::StackStore(v, c.numbered(b's')?)
            }
            ("symbol_value", None) => Op::SymbolValue(c.numbered(b'd')?),
            ("func_addr", None) => Op::FuncAddr(c.word()?),
            ("get_stack_pointer", None) => Op::GetStackPointer,
            ("call", None) => {
                let callee = c.word()?;
                let args = c.args()?;
                Op::Call(callee, args, n_defined)
            }
            ("call_indirect", None) => {
                let n = c.uint()?;
                c.expect(",")?;
                let f = c.value()?;
                let args = c.args()?;
                Op::CallIndirect(n, f, args, n_defined)
            }
            ("memcpy", None) => {
                let d = c.value()?;
                c.expect(",")?;
                let s = c.value()?;
                c.expect(",")?;
                Op::Memcpy(d, s, c.value()?)
            }
            ("use", None) => Op::Use(c.numbered(b'x')?),
            ("def", None) => {
                let x = c.numbered(b'x')?;
                c.expect(",")?;
                Op::Def(x, c.value()?)
            }
            ("param", None) => Op::Param(c.numbered(b'b')?),
            ("probe_reserve", None) => {
                f.reserve = true;
                Op::ProbeReserve
            }
            ("jump", None) => {
                let (b, args) = c.target()?;
                Op::Jump(b, args)
            }
            ("brif", None) => {
                let v = c.value()?;
                c.expect(",")?;
                let (a, aargs) = c.target()?;
                c.expect(",")?;
                let (b, bargs) = c.target()?;
                Op::Brif(v, a, aargs, b, bargs)
            }
            ("switch", None) => {
                let v = c.value()?;
                c.expect(",")?;
                let d = c.numbered(b'b')?;
                c.expect(",")?;
                c.expect("[")?;
                let mut cases = Vec::new();
                if !c.eat("]") {
                    loop {
                        let k = c.int()?;
                        cases.push((k as u64, c.numbered(b'b')?));
                        if c.eat("]") {
                            break;
                        }
                        c.expect(",")?;
                    }
                }
                c.expect("made")?;
                Op::Switch(v, d, cases, c.uint()?)
            }
            ("return", None) => {
                let mut vs = Vec::new();
                if !c.at_end() {
                    loop {
                        vs.push(c.value()?);
                        if !c.eat(",") {
                            break;
                        }
                    }
                }
                Op::Return(vs)
            }
            ("trap", None) => Op::Trap(u8::try_from(c.uint()?).map_err(|_| "bad trap code")?),
            _ => return Err(format!("unknown operation `{op_word}`")),
        };
        if !c.at_end() {
            return Err(format!("unexpected `{}`", c.rest()));
        }
        // The values must be numbered in order, after the parameters.
        for d in defined {
            if d != self.next_value {
                return Err(format!("expected `v{}`, found `v{d}`", self.next_value));
            }
            self.next_value += 1;
        }
        f.ops.push(op);
        Ok(())
    }
}

/// A `type tN ...` line after its number.
fn dtype(c: &mut Cur<'_>) -> R<DType> {
    let kind = c.word()?;
    // `{ ITEM, ... }`, each item read by `f`.
    fn list<T>(c: &mut Cur<'_>, mut f: impl FnMut(&mut Cur<'_>) -> R<T>) -> R<Vec<T>> {
        c.expect("{")?;
        let mut out = Vec::new();
        if c.eat("}") {
            return Ok(out);
        }
        loop {
            out.push(f(c)?);
            if c.eat("}") {
                return Ok(out);
            }
            c.expect(",")?;
        }
    }
    fn at(c: &mut Cur<'_>) -> R<u32> {
        c.expect("@")?;
        c.uint()
    }
    fn fields(c: &mut Cur<'_>) -> R<DFields> {
        list(c, |c| {
            let n = c.text()?;
            let t = c.numbered(b't')?;
            Ok((n, t, at(c)?))
        })
    }
    Ok(match kind.as_str() {
        "base" => {
            let name = c.text()?;
            let encoding = match c.word()?.as_str() {
                "signed" => gimli::constants::DW_ATE_signed,
                "unsigned" => gimli::constants::DW_ATE_unsigned,
                "float" => gimli::constants::DW_ATE_float,
                "boolean" => gimli::constants::DW_ATE_boolean,
                "utf" => gimli::constants::DW_ATE_UTF,
                "uchar" => gimli::constants::DW_ATE_unsigned_char,
                other => return Err(format!("unknown encoding `{other}`")),
            };
            DType::Base { name, encoding, size: c.uint()? }
        }
        "str" => DType::Str,
        "struct" => {
            let name = c.text()?;
            let size = c.uint()?;
            DType::Struct { name, size, fields: fields(c)? }
        }
        "tuple" => {
            let name = c.text()?;
            let size = c.uint()?;
            let items = list(c, |c| {
                let k = c.uint()?;
                let t = c.numbered(b't')?;
                Ok((k, t, at(c)?))
            })?;
            DType::Tuple { name, size, items }
        }
        "pointer" => {
            let name = c.text()?;
            DType::Pointer { name, to: c.numbered(b't')? }
        }
        "text" => {
            let name = c.text()?;
            DType::Text { name, size: c.uint()? }
        }
        "list" => {
            let name = c.text()?;
            let size = c.uint()?;
            let slice = c.text()?;
            DType::List { name, size, slice, elem: c.numbered(b't')? }
        }
        "enum" => {
            let name = c.text()?;
            let size = c.uint()?;
            let tag = at(c)?;
            let head = list(c, |c| {
                let n = c.text()?;
                Ok((n, at(c)?))
            })?;
            let variants = list(c, |c| {
                let n = c.text()?;
                Ok((n, fields(c)?))
            })?;
            DType::Enum { name, size, tag, head, variants }
        }
        "errset" => {
            let name = c.text()?;
            let size = c.uint()?;
            let members = list(c, |c| {
                let number = c.uint()?;
                let n = c.text()?;
                Ok((number, n, c.numbered(b't')?))
            })?;
            DType::ErrSet { name, size, members }
        }
        "alias" => DType::Alias(c.numbered(b't')?),
        "opaque" => {
            let name = c.text()?;
            DType::Opaque { name, size: c.uint()? }
        }
        "word" => DType::Word(c.text()?),
        other => return Err(format!("unknown type kind `{other}`")),
    })
}
