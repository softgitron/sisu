//! `Display`, `Debug` and format specs (03, "Strings": "Interpolation
//! `{x}` uses `Display`; `{x:?}` uses `Debug`. Format specs follow
//! Python/Rust").
//!
//! `Debug` renders a value the way it is written: `"text"`, `'c'`,
//! `[1, 2]`, `Point(x=1.0, y=2.0)`, `Err(BadDigit(at=1, found='x'))`, as
//! in 17-testing.md. `Display` is the plain text of strings, characters,
//! numbers and `bool`. These are the structural impls of the builtin types
//! (`structural.*`), and what the derived impls compute; a type with an
//! impl of its own is formatted by it, through the `Formatter` protocol.

use crate::Interp;
use crate::value::{IterState, Value};
use sisu_sir as sir;

impl Interp<'_, '_> {
    pub(crate) fn display_string(&self, v: &Value) -> String {
        let mut out = String::new();
        self.display_into(v, &mut out);
        out
    }

    pub(crate) fn debug_string(&self, v: &Value) -> String {
        let mut out = String::new();
        self.debug_into(v, &mut out);
        out
    }

    fn display_into(&self, v: &Value, out: &mut String) {
        match v {
            Value::Str(s, _) | Value::Path(s) => out.push_str(s),
            Value::String(s) => out.push_str(s),
            Value::Char(c) => out.push(*c),
            Value::DateTime(t) => out.push_str(&datetime_text(*t)),
            Value::Ref(p) => match self.read(p) {
                Ok(t) => self.display_into(t, out),
                Err(_) => out.push_str("<dangling>"),
            },
            Value::Dyn(d) => self.display_into(&d.value, out),
            // std's `Contextual` (prelude): `message: cause`.
            Value::Variant(a) if &*a.ty == "Contextual" && a.fields.len() == 2 => {
                self.display_into(&a.fields[0], out);
                out.push_str(": ");
                self.display_into(&a.fields[1], out);
            }
            other => self.debug_into(other, out),
        }
    }

    fn debug_into(&self, v: &Value, out: &mut String) {
        match v {
            Value::Moved => out.push_str("<moved>"),
            Value::Unit => out.push_str("()"),
            Value::Int(i) => out.push_str(&i.to_string()),
            Value::U64(i) => out.push_str(&i.to_string()),
            Value::I128(i) => out.push_str(&i.to_string()),
            Value::U128(i) => out.push_str(&i.to_string()),
            Value::Float(f) => out.push_str(&float_repr(*f)),
            Value::F32(f) => out.push_str(&float32_repr(*f)),
            Value::Bool(b) => out.push_str(if *b { "True" } else { "False" }),
            Value::Char(c) => out.push_str(&quote_char(*c)),
            Value::Str(s, _) | Value::Path(s) => out.push_str(&quote_str(s)),
            Value::String(s) => out.push_str(&quote_str(s)),
            Value::Bytes(b) => out.push_str(&quote_bytes(b)),
            Value::DateTime(t) => out.push_str(&datetime_text(*t)),
            Value::Tuple(xs) => {
                out.push('(');
                self.debug_list(xs, out);
                if xs.len() == 1 {
                    out.push(',');
                }
                out.push(')');
            }
            Value::List(xs) => {
                out.push('[');
                self.debug_list(xs, out);
                out.push(']');
            }
            Value::Set(s) => {
                if s.items.is_empty() {
                    out.push_str("set()");
                } else {
                    out.push('{');
                    self.debug_list(&s.items, out);
                    out.push('}');
                }
            }
            Value::Dict(d) => {
                out.push('{');
                for (i, (k, v)) in d.entries.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    self.debug_into(k, out);
                    out.push_str(": ");
                    self.debug_into(v, out);
                }
                out.push('}');
            }
            Value::Struct(a) => {
                if &*a.ty == "System" {
                    out.push_str("System");
                    return;
                }
                let names = self.field_names(&a.ty);
                let short = a.ty.rsplit('.').next().unwrap_or(&a.ty);
                out.push_str(short);
                out.push('(');
                self.debug_fields(&a.fields, &names, out);
                out.push(')');
            }
            Value::Variant(a) => {
                let (vname, names) = self.variant_info(&a.ty, a.variant);
                out.push_str(&vname);
                if !a.fields.is_empty() {
                    out.push('(');
                    self.debug_fields(&a.fields, &names, out);
                    out.push(')');
                }
            }
            Value::Ref(p) => match self.read(p) {
                Ok(t) => self.debug_into(t, out),
                Err(_) => out.push_str("<dangling>"),
            },
            Value::Fn(n) => {
                out.push_str("<fn ");
                out.push_str(n);
                out.push('>');
            }
            Value::Closure(..) => out.push_str("<closure>"),
            Value::Dyn(d) => self.debug_into(&d.value, out),
            Value::Iter(it) => {
                out.push_str(if matches!(**it, IterState::Done) { "<iter (done)>" } else { "<iter>" })
            }
            Value::Entry(_) => out.push_str("<entry>"),
            Value::Shared(id) => match self.heap.get(*id as usize) {
                Some(c) if c.strong > 0 => self.debug_into(&c.value, out),
                _ => out.push_str("<dropped>"),
            },
            Value::Weak(_) => out.push_str("(Weak)"),
            Value::Cell(v) => {
                out.push_str("Cell(");
                self.debug_into(v, out);
                out.push(')');
            }
            Value::Mutex(m) => {
                out.push_str(if m.rw { "RwLock(" } else { "Mutex(" });
                if m.locked {
                    out.push_str("<locked>");
                } else {
                    self.debug_into(&m.value, out);
                }
                out.push(')');
            }
            Value::Guard(p) | Value::ReadGuard(p) => match self.read(&p.join(crate::value::Step::MutexValue))
            {
                Ok(t) => self.debug_into(t, out),
                Err(_) => out.push_str("<dangling>"),
            },
            Value::Arena(a) => {
                out.push_str("Arena([");
                let live: Vec<&Value> = a.slots.iter().map(|(_, v)| v).filter(|v| !v.is_moved()).collect();
                for (i, v) in live.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    self.debug_into(v, out);
                }
                out.push_str("])");
            }
            Value::Cap(id) => match self.heap.get(*id as usize).and_then(|c| c.label.as_deref()) {
                Some(l) => out.push_str(l),
                None => out.push_str("Cap"),
            },
            Value::Clock(_) => out.push_str("Clock"),
            Value::Socket(h) => out.push_str(&format!("<socket {h}>")),
            Value::Scope(_) => out.push_str("<scope>"),
            Value::Handle(t) => out.push_str(&format!("<task {}>", self.task_path(*t))),
            Value::Cancellation(_) => out.push_str("<cancellation>"),
            Value::Sender(_) => out.push_str("<sender>"),
            Value::Receiver(_) => out.push_str("<receiver>"),
            Value::Timer { .. } => out.push_str("<timer>"),
        }
    }

    fn debug_list(&self, xs: &[Value], out: &mut String) {
        for (i, x) in xs.iter().enumerate() {
            if i > 0 {
                out.push_str(", ");
            }
            self.debug_into(x, out);
        }
    }

    fn debug_fields(&self, xs: &[Value], names: &[Option<String>], out: &mut String) {
        for (i, x) in xs.iter().enumerate() {
            if i > 0 {
                out.push_str(", ");
            }
            if let Some(Some(n)) = names.get(i) {
                out.push_str(n);
                out.push('=');
            }
            self.debug_into(x, out);
        }
    }

    fn field_names(&self, ty: &str) -> Vec<Option<String>> {
        match self.types.get(ty) {
            Some(sir::TypeDecl::Struct { fields, .. }) => {
                fields.iter().map(|f| Some(f.name.clone())).collect()
            }
            _ => Vec::new(),
        }
    }

    /// A variant's name and field names.
    pub(crate) fn variant_info(&self, ty: &str, variant: u32) -> (String, Vec<Option<String>>) {
        let builtin = match (ty, variant) {
            ("Option", 0) => Some("Some"),
            ("Option", _) => Some("None"),
            ("Result", 0) => Some("Ok"),
            ("Result", _) => Some("Err"),
            _ => None,
        };
        if let Some(b) = builtin {
            return (b.to_string(), Vec::new());
        }
        match self.types.get(ty) {
            Some(sir::TypeDecl::Enum { variants, .. }) => match variants.get(variant as usize) {
                Some(v) => (v.name.clone(), v.fields.iter().map(|(n, _)| n.clone()).collect()),
                None => (format!("{ty}#{variant}"), Vec::new()),
            },
            _ => (format!("{ty}#{variant}"), Vec::new()),
        }
    }

    /// The text of `{value:spec}` (`debug` for `{value=}` and `{value:?}`).
    /// The locations an error passed through, oldest first.
    pub(crate) fn trace_lines(&self, v: &Value) -> Vec<String> {
        match self.target(v) {
            Ok(Value::Variant(a)) => a.trace.iter().map(|l| l.to_string()).collect(),
            _ => Vec::new(),
        }
    }
}

/// `fmt.digits`: the digits of `|n|` in `base` (capitals with `upper`).
pub(crate) fn digits(n: i64, base: i64, upper: bool) -> String {
    wide_digits(u128::from(n.unsigned_abs()), base, upper)
}

/// `fmt.wide_digits`: the digits of a magnitude in `base`.
pub(crate) fn wide_digits(mag: u128, base: i64, upper: bool) -> String {
    match base {
        16 if upper => format!("{mag:X}"),
        16 => format!("{mag:x}"),
        8 => format!("{mag:o}"),
        2 => format!("{mag:b}"),
        _ => mag.to_string(),
    }
}

/// `fmt.float_digits`: the text of `x >= 0` by the type letter, with
/// `precision` digits after the point; `inf` for infinity.
pub(crate) fn float_digits(x: f64, kind: Option<char>, precision: Option<usize>) -> String {
    if x.is_infinite() {
        return "inf".into();
    }
    match kind {
        Some('f' | 'F') => format!("{x:.*}", precision.unwrap_or(6)),
        Some('e' | 'E') => {
            let s = exponent_form(x, precision.unwrap_or(6));
            if kind == Some('E') { s.to_uppercase() } else { s }
        }
        Some('%') => format!("{:.*}%", precision.unwrap_or(6), x * 100.0),
        _ => match precision {
            Some(p) => format!("{x:.p$}"),
            None => float_repr(x),
        },
    }
}

/// `fmt.float32_digits`: `float_digits` of the `f64` that holds `x`,
/// except that the shortest form reads back as the same `f32`.
pub(crate) fn float32_digits(x: f32, kind: Option<char>, precision: Option<usize>) -> String {
    match (kind, precision) {
        (Some('f' | 'F' | 'e' | 'E' | '%'), _) | (_, Some(_)) => float_digits(f64::from(x), kind, precision),
        _ if x.is_infinite() => "inf".into(),
        _ => float32_repr(x),
    }
}

/// `1.500000e+00`.
fn exponent_form(x: f64, precision: usize) -> String {
    let s = format!("{x:.precision$e}");
    match s.split_once('e') {
        Some((m, e)) => {
            let (sign, digits) = match e.strip_prefix('-') {
                Some(d) => ('-', d),
                None => ('+', e),
            };
            format!("{m}e{sign}{digits:0>2}")
        }
        None => s,
    }
}

/// The shortest text that reads back as the same `f64`, always with a
/// decimal point or an exponent: `3.0`, `0.1`, `1e100`, `inf`, `NaN`.
pub(crate) fn float_repr(f: f64) -> String {
    format!("{f:?}")
}

/// The shortest text that reads back as the same `f32`, as `float_repr`
/// writes an `f64`: `0.1`, `3.4028235e38`.
pub(crate) fn float32_repr(f: f32) -> String {
    format!("{f:?}")
}

pub(crate) fn quote_str(s: &str) -> String {
    let mut q = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => q.push_str("\\\""),
            '\\' => q.push_str("\\\\"),
            '\n' => q.push_str("\\n"),
            '\t' => q.push_str("\\t"),
            '\r' => q.push_str("\\r"),
            c if c.is_control() => q.push_str(&format!("\\u{{{:x}}}", c as u32)),
            c => q.push(c),
        }
    }
    q.push('"');
    q
}

fn quote_char(c: char) -> String {
    match c {
        '\'' => "'\\''".into(),
        '\\' => "'\\\\'".into(),
        '\n' => "'\\n'".into(),
        '\t' => "'\\t'".into(),
        '\r' => "'\\r'".into(),
        c if c.is_control() => format!("'\\u{{{:x}}}'", c as u32),
        c => format!("'{c}'"),
    }
}

/// The civil date of a day count from 1970-01-01 (H. Hinnant's algorithm).
fn civil(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, m, d)
}

/// The RFC 3339 text of a time in nanoseconds since the epoch, in UTC:
/// `2026-10-11T12:00:00Z`, with the fraction of a second when there is
/// one, without trailing zeros (`12:00:00.5Z`).
pub(crate) fn datetime_text(nanos: i64) -> String {
    let mut s = strftime(nanos, "%Y-%m-%dT%H:%M:%S");
    let frac = nanos.rem_euclid(1_000_000_000);
    if frac != 0 {
        let digits = format!("{frac:09}");
        s.push('.');
        s.push_str(digits.trim_end_matches('0'));
    }
    s.push('Z');
    s
}

/// A time in nanoseconds since the epoch by the `strftime` directives
/// `%Y %y %m %d %H %M %S %F %T %%`, in UTC; other characters are copied.
pub(crate) fn strftime(nanos: i64, pattern: &str) -> String {
    let secs = nanos.div_euclid(1_000_000_000);
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let (y, m, d) = civil(days);
    let (hh, mm, ss) = (rem / 3600, (rem / 60) % 60, rem % 60);
    let mut out = String::new();
    let mut cs = pattern.chars();
    while let Some(c) = cs.next() {
        if c != '%' {
            out.push(c);
            continue;
        }
        match cs.next() {
            Some('Y') => out.push_str(&format!("{y:04}")),
            Some('y') => out.push_str(&format!("{:02}", y.rem_euclid(100))),
            Some('m') => out.push_str(&format!("{m:02}")),
            Some('d') => out.push_str(&format!("{d:02}")),
            Some('H') => out.push_str(&format!("{hh:02}")),
            Some('M') => out.push_str(&format!("{mm:02}")),
            Some('S') => out.push_str(&format!("{ss:02}")),
            Some('F') => out.push_str(&format!("{y:04}-{m:02}-{d:02}")),
            Some('T') => out.push_str(&format!("{hh:02}:{mm:02}:{ss:02}")),
            Some('%') => out.push('%'),
            Some(other) => {
                out.push('%');
                out.push(other);
            }
            None => out.push('%'),
        }
    }
    out
}

/// `bytes` as Sisu writes them: `b"..."`, printable ASCII as itself and
/// every other byte as `\xNN` (with `\n`, `\t`, `\r`, `\\` and `\"`).
pub(crate) fn quote_bytes(b: &[u8]) -> String {
    let mut out = String::from("b\"");
    for &x in b {
        match x {
            b'\n' => out.push_str("\\n"),
            b'\t' => out.push_str("\\t"),
            b'\r' => out.push_str("\\r"),
            b'\\' => out.push_str("\\\\"),
            b'"' => out.push_str("\\\""),
            0x20..=0x7e => out.push(x as char),
            _ => out.push_str(&format!("\\x{x:02x}")),
        }
    }
    out.push('"');
    out
}

/// The values of a failed condition's parts drawn under it (17,
/// "Assertions"), each under its column with a bar above it, as lines
/// starting with a line break:
///
/// ```text
///     parse_int(text).ok() == Some(expected)
///     │         │     │        │
///     │         "4x"  None     Some(42)
///     Err(BadDigit(at=1, found='x'))
/// ```
///
/// Values are placed from the rightmost part to the leftmost, each on the
/// first row below the bars where it does not overlap one already there;
/// every part's bar runs down to its value. Nothing is drawn for a
/// condition on more than one line or with no parts.
pub fn parts_diagram(text: &str, parts: &[(usize, String)]) -> String {
    if parts.is_empty() || text.contains('\n') {
        return String::new();
    }
    let mut parts: Vec<(usize, String)> = parts.to_vec();
    parts.sort_by_key(|p| std::cmp::Reverse(p.0));
    parts.dedup_by_key(|p| p.0);
    // rows[r] holds the values on row r + 1 (row 0 is the bars), as
    // (column, text); depth[i] the row of part i.
    let mut rows: Vec<Vec<(usize, String)>> = Vec::new();
    let mut depth = Vec::new();
    for (c, v) in &parts {
        let len = v.chars().count();
        let fits =
            |row: &Vec<(usize, String)>| row.iter().all(|(k, w)| c + len < *k || *k + w.chars().count() < *c);
        let r = match rows.iter().position(fits) {
            Some(r) => r,
            None => {
                rows.push(Vec::new());
                rows.len() - 1
            }
        };
        rows[r].push((*c, v.clone()));
        depth.push(r);
    }
    let width = parts.iter().map(|(c, v)| c + v.chars().count()).max().unwrap_or(0) + 1;
    let mut out = format!("\n    {text}");
    // The bar row, then each row of values with the bars that pass it.
    for r in 0..=rows.len() {
        let mut line: Vec<char> = vec![' '; width];
        for (i, (c, _)) in parts.iter().enumerate() {
            // A part's bar runs down through the rows above its value.
            if (r == 0 || depth[i] >= r)
                && let Some(slot) = line.get_mut(*c)
            {
                *slot = '│';
            }
        }
        if r > 0 {
            for (c, v) in &rows[r - 1] {
                for (k, ch) in v.chars().enumerate() {
                    if let Some(slot) = line.get_mut(c + k) {
                        *slot = ch;
                    }
                }
            }
        }
        let s: String = line.into_iter().collect();
        out.push_str("\n    ");
        out.push_str(s.trim_end());
    }
    out
}
