//! The standard-library modules of stage 0, embedded at build time. They
//! are Sisu source and go through the same front end as user code.

use std::fmt::Write as _;
use std::sync::OnceLock;

pub const PRELUDE: &str = include_str!("../../../std/prelude.sisu");
pub const CLI: &str = include_str!("../../../std/cli.sisu");
pub const MATH: &str = include_str!("../../../std/math.sisu");
pub const TASK: &str = include_str!("../../../std/task.sisu");
pub const LOG: &str = include_str!("../../../std/log.sisu");
pub const CAP: &str = include_str!("../../../std/cap.sisu");
pub const NET: &str = include_str!("../../../std/net.sisu");
pub const TIME: &str = include_str!("../../../std/time.sisu");
pub const HTTP: &str = include_str!("../../../std/http.sisu");
pub const SIM: &str = include_str!("../../../std/sim.sisu");
pub const JSON: &str = include_str!("../../../std/json.sisu");
pub const TEST: &str = include_str!("../../../std/test.sisu");
pub const COLLECTIONS: &str = include_str!("../../../std/collections.sisu");
pub const NUM: &str = include_str!("../../../std/num.sisu");
pub const LIVE: &str = include_str!("../../../std/live.sisu");

/// `target` (08, "Conditional compilation"): the build target's facts as
/// constants. Stage 0's target is the interpreter on the host, so they are
/// the host's operating system and processor; `if comptime` on them is
/// decided when the program is compiled, and both branches are checked.
pub fn target() -> &'static str {
    static TEXT: OnceLock<String> = OnceLock::new();
    TEXT.get_or_init(|| {
        format!(
            "##! The build target (08, \"Conditional compilation\").\n\n## The operating system: `linux`, `macos`, `windows`, ...\nconst os: str = \"{}\"\n\n## The processor: `x86_64`, `aarch64`, ...\nconst arch: str = \"{}\"\n\n## How the target runs a program: `interpreter` in stage 0.\nconst runtime: str = \"interpreter\"\n",
            std::env::consts::OS,
            std::env::consts::ARCH
        )
    })
}

/// The std modules every module sees by name without an import, as the
/// prelude's own items: `task.scope` is ambient (Q-6), and so is the `log`
/// facade (Q-5). `cap` (std.cap) grants nothing by itself: its fakes,
/// attenuations and interceptors work on capabilities they are given.
pub const AMBIENT: &[(&str, &str)] = &[("task", TASK), ("log", LOG), ("cap", CAP), ("live", LIVE)];

/// The std module an import path names, as (module name, source).
pub fn module_for(path: &[&str]) -> Option<(&'static str, &'static str)> {
    match path {
        ["cli"] | ["std", "x", "cli"] => Some(("cli", CLI)),
        ["std", "math"] | ["math"] => Some(("math", MATH)),
        ["std", "task"] | ["task"] => Some(("task", TASK)),
        ["std", "log"] | ["log"] => Some(("log", LOG)),
        ["std", "cap"] | ["cap"] => Some(("cap", CAP)),
        ["std", "net"] | ["net"] => Some(("net", NET)),
        ["std", "time"] | ["time"] => Some(("time", TIME)),
        ["std", "x", "http"] | ["http"] => Some(("http", HTTP)),
        ["std", "sim"] | ["sim"] => Some(("sim", SIM)),
        ["std", "json"] | ["json"] => Some(("json", JSON)),
        ["std", "test"] => Some(("test", TEST)),
        ["std", "collections"] => Some(("collections", COLLECTIONS)),
        ["std", "num"] => Some(("num", NUM)),
        ["std", "live"] => Some(("live", LIVE)),
        _ => None,
    }
}

/// The std modules whose doc examples `sisu test --std` runs, besides the
/// prelude's.
pub const DOCUMENTED: &[(&str, &str)] = &[
    ("cap", CAP),
    ("cli", CLI),
    ("collections", COLLECTIONS),
    ("live", LIVE),
    ("num", NUM),
    ("http", HTTP),
    ("json", JSON),
    ("log", LOG),
    ("math", MATH),
    ("net", NET),
    ("sim", SIM),
    ("task", TASK),
    ("test", TEST),
    ("time", TIME),
];

/// A std module's path as an import names it: `std.x.http` for `http`.
pub fn display_path(name: &str) -> String {
    match name {
        "prelude" => "std.core".into(),
        "cli" | "http" => format!("std.x.{name}"),
        other => format!("std.{other}"),
    }
}

/// The prelude's names a std module names too (`from std.time import
/// DateTime`).
pub fn reexports(module: &str) -> &'static [&'static str] {
    match module {
        "time" => &["DateTime", "Duration", "Timer", "TimeError", "TestClock", "Clock"],
        "net" => &["Net", "NetOps", "NetError", "Conn", "Listener", "Stream", "Accept"],
        "cap" => &[
            "System",
            "Fs",
            "Net",
            "Clock",
            "Rand",
            "Env",
            "Proc",
            "Stdin",
            "FsOps",
            "NetOps",
            "ClockOps",
            "RandOps",
            "EnvOps",
            "ProcOps",
            "StdinOps",
            "Lent",
            "Cap",
            "Revoker",
            "TestClock",
        ],
        "task" => &["Scope", "Handle", "TaskError", "Cancellation", "channel", "Sender", "Receiver", "Mutex"],
        "test" => &["check", "arbitrary", "exhaustive", "Gen", "Draws"],
        "collections" => &["list", "dict", "set", "Arena", "Key", "Slice", "Iter", "Entry"],
        "num" => &[
            "Num", "u8", "u16", "u32", "u64", "u128", "i8", "i16", "i32", "i128", "bytes", "abs", "min",
            "max", "sum",
        ],
        _ => &[],
    }
}

/// The leaf types and the traits each implements structurally.
const LEAVES: &[(&str, &[&str])] = &[
    ("int", &["Display", "Debug", "Eq", "PartialOrd", "Ord", "Hash"]),
    ("f64", &["Display", "Debug", "Eq", "PartialOrd"]),
    ("bool", &["Display", "Debug", "Eq", "PartialOrd", "Ord", "Hash"]),
    ("char", &["Display", "Debug", "Eq", "PartialOrd", "Ord", "Hash"]),
    ("str", &["Display", "Debug", "Eq", "PartialOrd", "Ord", "Hash", "Len"]),
    ("String", &["Display", "Debug", "Eq", "PartialOrd", "Ord", "Hash", "Len"]),
    ("Path", &["Display", "Debug", "Eq", "PartialOrd", "Ord", "Hash"]),
    ("DateTime", &["Display", "Debug", "Eq", "PartialOrd", "Ord", "Hash"]),
    ("()", &["Display", "Debug", "Eq", "PartialOrd", "Ord", "Hash"]),
    ("bytes", &["Display", "Debug", "Eq", "PartialOrd", "Ord", "Hash"]),
    ("u8", &["Display", "Debug", "Eq", "PartialOrd", "Ord", "Hash"]),
    ("u16", &["Display", "Debug", "Eq", "PartialOrd", "Ord", "Hash"]),
    ("u32", &["Display", "Debug", "Eq", "PartialOrd", "Ord", "Hash"]),
    ("u64", &["Display", "Debug", "Eq", "PartialOrd", "Ord", "Hash"]),
    ("i8", &["Display", "Debug", "Eq", "PartialOrd", "Ord", "Hash"]),
    ("i16", &["Display", "Debug", "Eq", "PartialOrd", "Ord", "Hash"]),
    ("i32", &["Display", "Debug", "Eq", "PartialOrd", "Ord", "Hash"]),
    ("u128", &["Display", "Debug", "Eq", "PartialOrd", "Ord", "Hash"]),
    ("i128", &["Display", "Debug", "Eq", "PartialOrd", "Ord", "Hash"]),
    ("Fs", &["Display", "Debug"]),
    ("Net", &["Display", "Debug"]),
    ("Clock", &["Display", "Debug"]),
    ("Rand", &["Display", "Debug"]),
    ("Env", &["Display", "Debug"]),
    ("Proc", &["Display", "Debug"]),
    ("Stdin", &["Display", "Debug"]),
    ("System", &["Display", "Debug"]),
    ("TestClock", &["Display", "Debug"]),
    ("HostFs", &["Debug"]),
    ("HostNet", &["Debug"]),
    ("HostClock", &["Debug"]),
    ("HostRand", &["Debug"]),
    ("HostEnv", &["Debug"]),
    ("HostProc", &["Debug"]),
    ("HostStdin", &["Debug"]),
];

/// The largest tuple the generated impls cover.
pub const TUPLE_ARITY: usize = 6;

/// `<std>/builtin_impls.sisu`: the impls of the leaf types and of tuples,
/// which are the same text for each type and so are written by a loop
/// rather than by hand. The prelude names every trait they implement.
pub fn builtin_impls() -> &'static str {
    static TEXT: OnceLock<String> = OnceLock::new();
    TEXT.get_or_init(|| {
        let mut s = String::new();
        s.push_str(
            "##! Generated by compiler/crates/sisu-resolve/src/std_sources.rs: the\n\
             ##! impls of the leaf types and of tuples of up to six elements.\n\
             ##!\n\
             ##! A leaf's `Display` and `Debug` apply the format spec when the leaf is\n\
             ##! the value being rendered (`Spec` in the prelude) and otherwise write\n\
             ##! its plain text, which `\"{self}\"` gives without a spec.\n\
             ##! The intrinsics `structural.*` implement a trait the way the\n\
             ##! interpreter compares, orders, hashes and prints values:\n\
             ##! `structural.display` writes the plain text of a value (numbers,\n\
             ##! text, characters and paths as they read); `structural.debug` writes a value as it\n\
             ##! is written in Sisu (`\"text\"`, `'c'`, `1.0`); `structural.eq` is\n\
             ##! equality of the values (`str` and `String` with the same text are\n\
             ##! equal); `structural.partial_cmp` is the order of numbers, of text by\n\
             ##! code points, `False < True`, and `None` for NaN; `structural.hash`\n\
             ##! mixes the value into the hasher. `op.add`, `op.sub`, `op.mul`,\n\
             ##! `op.div`, `op.rem` and `op.neg` are the operators `+`, `-`, `*`, `/`,\n\
             ##! `%` and unary `-` on numbers, with their overflow and division\n\
             ##! panics.\n",
        );
        for (ty, traits) in LEAVES {
            for t in *traits {
                let _ = writeln!(s, "\nimpl {t} for {ty}:");
                if let Some(body) = leaf_format(t, ty) {
                    s.push_str(&body);
                    continue;
                }
                let (name, sig) = leaf_method(t, ty);
                let _ = writeln!(s, "    @intrinsic(\"{name}\")\n    pure def {sig}");
            }
        }
        sized_impls(&mut s);
        // The operators of the numbers, as traits for generic code; `a + b`
        // on numbers stays the language's own operation.
        for ty in ["int", "f64"] {
            for (t, m) in [("Add", "add"), ("Sub", "sub"), ("Mul", "mul"), ("Div", "div"), ("Rem", "rem")] {
                let _ = writeln!(s, "\nimpl {t} for {ty}:");
                let _ =
                    writeln!(s, "    @intrinsic(\"op.{m}\")\n    pure def {m}(self, other: {ty}) -> {ty}");
            }
            let _ = writeln!(s, "\nimpl Neg for {ty}:");
            let _ = writeln!(s, "    @intrinsic(\"op.neg\")\n    pure def neg(self) -> {ty}");
        }
        for n in 1..=TUPLE_ARITY {
            tuple_impls(&mut s, n);
        }
        s
    })
}

/// `Display` and `Debug` of the values a format spec applies to, in Sisu:
/// the leaf applies the spec when it is the value being rendered (`Spec`
/// in std/prelude.sisu), and otherwise writes its plain text, which the
/// interpolation `"{self}"` gives without a spec (the host's structural
/// text).
fn leaf_format(t: &str, ty: &str) -> Option<String> {
    if is_handle(ty) {
        return None;
    }
    let (sig, plain, with_spec) = match (t, ty) {
        ("Display", "DateTime") => {
            return Some(
                "    pure def fmt(self, mut f: Formatter):\n        \
                 if f.depth > 0 or f.spec.is_empty():\n            f.write(\"{self}\")\n        \
                 else:\n            let pattern = f.spec.to_string()\n            f.spec = String.new(\"\")\n            \
                 f.write(Spec.strftime(self, pattern))\n"
                    .to_string(),
            );
        }
        ("Display", "int") => ("fmt", "\"{self}\"", "f.write(s.int(self))".to_string()),
        // A sized integer formats as the `int` it equals; a `u64`, `u128`
        // or `i128` beyond `int` as text.
        ("Display", "u64" | "u128" | "i128") => (
            "fmt",
            "\"{self}\"",
            format!(
                "match self.to_int():\n                    case Some(n):\n                        f.write(s.int(n))\n                    \
                 case None:\n                        f.write(s.wide(self, {}))",
                if ty == "i128" { "self < 0" } else { "False" }
            ),
        ),
        ("Display", t) if SIZED.contains(&t) => {
            ("fmt", "\"{self}\"", "f.write(s.int(self.to_int()))".to_string())
        }
        ("Display", "f64") => ("fmt", "\"{self}\"", "f.write(s.float(self))".to_string()),
        // `?` as the spec's type asks for the `Debug` text.
        ("Display", _) => (
            "fmt",
            "\"{self}\"",
            "if s.kind == Some('?'):\n                    f.write(s.pad(\"{self:?}\"))\n                \
             else:\n                    f.write(s.text(\"{self}\"))"
                .to_string(),
        ),
        ("Debug", _) => ("fmt_debug", "\"{self:?}\"", "f.write(s.pad(\"{self:?}\"))".to_string()),
        _ => return None,
    };
    Some(format!(
        "    pure def {sig}(self, mut f: Formatter):\n        match f.take_spec():\n            case Some(s):\n                \
         {with_spec}\n            case None:\n                f.write({plain})\n"
    ))
}

fn leaf_method(t: &str, ty: &str) -> (&'static str, String) {
    match t {
        "Display" => ("structural.display", "fmt(self, mut f: Formatter)".into()),
        "Debug" => ("structural.debug", "fmt_debug(self, mut f: Formatter)".into()),
        "Eq" => ("structural.eq", format!("eq(self, other: {ty}) -> bool")),
        "PartialOrd" => ("structural.partial_cmp", format!("partial_cmp(self, other: {ty}) -> Ordering?")),
        "Ord" => ("structural.cmp", format!("cmp(self, other: {ty}) -> Ordering")),
        "Hash" => ("structural.hash", "hash(self, mut h: Hasher)".into()),
        _ => ("len", "len(self) -> int".into()),
    }
}

fn tuple_impls(s: &mut String, n: usize) {
    let names: Vec<String> = (0..n).map(|i| format!("T{i}")).collect();
    let ty = if n == 1 { format!("({},)", names[0]) } else { format!("({})", names.join(", ")) };
    let params = |bound: &str| names.iter().map(|x| format!("{x}: {bound}")).collect::<Vec<_>>().join(", ");
    // Debug and Display: `(1, "a")`, `(1,)`.
    let _ = writeln!(s, "\nimpl[{}] Debug for {ty}:", params("Debug"));
    let _ = writeln!(s, "    pure def fmt_debug(self, mut f: Formatter):");
    let _ = writeln!(s, "        f.write(\"(\")");
    let _ = writeln!(s, "        f.depth += 1");
    for i in 0..n {
        if i > 0 {
            let _ = writeln!(s, "        f.write(\", \")");
        }
        let _ = writeln!(s, "        self.{i}.fmt_debug(mut f)");
    }
    let _ = writeln!(s, "        f.depth -= 1");
    let _ = writeln!(s, "        f.write(\"{})\")", if n == 1 { "," } else { "" });
    let _ = writeln!(s, "\nimpl[{}] Display for {ty}:", params("Debug"));
    let _ = writeln!(s, "    pure def fmt(self, mut f: Formatter):");
    let _ = writeln!(s, "        self.fmt_debug(mut f)");
    // Eq: element by element.
    let _ = writeln!(s, "\nimpl[{}] Eq for {ty}:", params("Eq"));
    let _ = writeln!(s, "    pure def eq(self, other: {ty}) -> bool:");
    let parts: Vec<String> = (0..n).map(|i| format!("self.{i} == other.{i}")).collect();
    let _ = writeln!(s, "        return {}", parts.join(" and "));
    // PartialOrd and Ord: lexicographic.
    for (tr, method, eq, wrap) in
        [("PartialOrd", "partial_cmp", "Some(Equal)", "Ordering?"), ("Ord", "cmp", "Equal", "Ordering")]
    {
        let _ = writeln!(s, "\nimpl[{}] {tr} for {ty}:", params(tr));
        let _ = writeln!(s, "    pure def {method}(self, other: {ty}) -> {wrap}:");
        for i in 0..n {
            if i + 1 == n {
                let _ = writeln!(s, "        return self.{i}.{method}(other.{i})");
            } else {
                let _ = writeln!(s, "        let c{i} = self.{i}.{method}(other.{i})");
                let _ = writeln!(s, "        if c{i} != {eq}:");
                let _ = writeln!(s, "            return c{i}");
            }
        }
    }
    let _ = writeln!(s, "\nimpl[{}] Hash for {ty}:", params("Hash"));
    let _ = writeln!(s, "    pure def hash(self, mut h: Hasher):");
    for i in 0..n {
        let _ = writeln!(s, "        self.{i}.hash(mut h)");
    }
}

/// The capability handles and host objects among the leaves: they print
/// as their type's name and take no format spec.
fn is_handle(ty: &str) -> bool {
    matches!(
        ty,
        "Fs" | "Net"
            | "Clock"
            | "Rand"
            | "Env"
            | "Proc"
            | "Stdin"
            | "System"
            | "TestClock"
            | "HostFs"
            | "HostNet"
            | "HostClock"
            | "HostRand"
            | "HostEnv"
            | "HostProc"
            | "HostStdin"
    )
}

/// The sized integer types (03, "Numbers"), narrowest first in each kind.
pub const SIZED: &[&str] = &["u8", "u16", "u32", "u64", "u128", "i8", "i16", "i32", "i128"];

/// The sized integer types some of whose values `int` cannot hold: a
/// value of one is a kind of its own at run time, made by an intrinsic
/// (`u64.from_bits`, `i128.widen`, `u128.from_text`) rather than an `int`
/// constant.
pub fn own_kind(ty: &str) -> bool {
    matches!(ty, "u64" | "u128" | "i128")
}

/// The bits of a sized integer type and whether it is signed; `int` is
/// `i64`.
pub fn int_width(ty: &str) -> Option<(u32, bool)> {
    Some(match ty {
        "u8" => (8, false),
        "u16" => (16, false),
        "u32" => (32, false),
        "u64" => (64, false),
        "u128" => (128, false),
        "i8" => (8, true),
        "i16" => (16, true),
        "i32" => (32, true),
        "i128" => (128, true),
        "int" => (64, true),
        _ => return None,
    })
}

/// Whether every value of integer type `from` is one of `to` (the implicit
/// widening of 03).
pub fn widens(from: &str, to: &str) -> bool {
    let (Some((fw, fs)), Some((tw, ts))) = (int_width(from), int_width(to)) else { return false };
    from != to && if fs == ts { tw >= fw } else { !fs && ts && tw > fw }
}

/// The arithmetic of the sized integers: the operator traits for generic
/// code, the conversions to and from `int`, and `Into` for each widening.
fn sized_impls(s: &mut String) {
    for ty in SIZED {
        let signed = int_width(ty).is_some_and(|(_, sg)| sg);
        for (t, m) in [("Add", "add"), ("Sub", "sub"), ("Mul", "mul"), ("Rem", "rem")] {
            let _ = writeln!(s, "\nimpl {t} for {ty}:");
            let _ = writeln!(s, "    @intrinsic(\"{ty}.{m}\")\n    pure def {m}(self, other: {ty}) -> {ty}");
        }
        if signed {
            let _ = writeln!(s, "\nimpl Neg for {ty}:");
            let _ = writeln!(s, "    @intrinsic(\"{ty}.neg\")\n    pure def neg(self) -> {ty}");
        }
        let _ = writeln!(s, "\nimpl {ty}:");
        if own_kind(ty) {
            let _ = writeln!(s, "    ## The value as an `int`, or `None` outside `int`'s range.");
            let _ = writeln!(s, "    @intrinsic(\"{ty}.to_int\")\n    pure def to_int(self) -> int?");
        } else {
            let _ = writeln!(s, "    ## The value as an `int`, which holds every one.");
            let _ = writeln!(s, "    @intrinsic(\"{ty}.to_int\")\n    pure def to_int(self) -> int");
        }
        let _ = writeln!(s, "    @intrinsic(\"{ty}.cmp\")\n    pure def cmp(self, other: {ty}) -> Ordering");
        let _ = writeln!(s, "\nimpl Integer for {ty}:");
        let _ =
            writeln!(s, "    @intrinsic(\"{ty}.wrapping_from\")\n    pure def wrapping_from(n: int) -> {ty}");
        let _ = writeln!(s, "\nimpl FromLiteral[int] for {ty}:");
        let _ = writeln!(
            s,
            "    @intrinsic(\"{ty}.from_literal\")\n    pure def from_literal(literal: int) -> {ty}"
        );
        for to in SIZED.iter().chain(std::iter::once(&"int")) {
            if widens(ty, to) {
                let _ = writeln!(s, "\nimpl Into[{to}] for {ty}:");
                let _ = writeln!(s, "    @intrinsic(\"{to}.widen\")\n    pure def into(self) -> {to}");
            }
        }
    }
    let _ = writeln!(s, "\nimpl Integer for int:");
    let _ = writeln!(s, "    pure def wrapping_from(n: int) -> int:\n        return n");
    let _ = writeln!(s, "\nimpl int:");
    let _ = writeln!(
        s,
        "    ## The value reduced to `T`'s width, as two's complement wrapping does:\n    \
         ## `300.truncate_to[u8]()` is `44`, `-1.truncate_to[u32]()` is `4294967295`.\n    \
         pure def truncate_to[T: Integer](self) -> T:\n        return T.wrapping_from(self)"
    );
    for ty in SIZED {
        let _ = writeln!(s, "    ## The value as a `{ty}`, or `None` if it does not fit.");
        let _ = writeln!(s, "    @intrinsic(\"int.to_{ty}\")\n    pure def to_{ty}(self) -> {ty}?");
    }
}
