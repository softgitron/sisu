//! `Display`, `Debug` and format specs (03, "Strings": "Interpolation
//! `{x}` uses `Display`; `{x:?}` uses `Debug`. Format specs follow
//! Python/Rust").
//!
//! `Debug` renders a value the way it is written: `"text"`, `'c'`,
//! `[1, 2]`, `Point(x=1.0, y=2.0)`, `Err(BadDigit(at=1, found='x'))`, as
//! in 17-testing.md. `Display` is the plain text of strings, characters,
//! numbers and `bool`; every other type displays as its `Debug` form until
//! traits let a type define its own (Phase 4).

use crate::value::{IterState, Value};
use crate::{Interp, Unwind, ice};
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
            Value::DateTime(t) => out.push_str(&strftime(*t, "%Y-%m-%dT%H:%M:%SZ")),
            Value::Ref(p) => match self.read(p) {
                Ok(t) => self.display_into(t, out),
                Err(_) => out.push_str("<dangling>"),
            },
            other => self.debug_into(other, out),
        }
    }

    fn debug_into(&self, v: &Value, out: &mut String) {
        match v {
            Value::Moved => out.push_str("<moved>"),
            Value::Unit => out.push_str("()"),
            Value::Int(i) => out.push_str(&i.to_string()),
            Value::Float(f) => out.push_str(&float_repr(*f)),
            Value::Bool(b) => out.push_str(if *b { "True" } else { "False" }),
            Value::Char(c) => out.push_str(&quote_char(*c)),
            Value::Str(s, _) | Value::Path(s) => out.push_str(&quote_str(s)),
            Value::String(s) => out.push_str(&quote_str(s)),
            Value::DateTime(t) => out.push_str(&strftime(*t, "%Y-%m-%dT%H:%M:%SZ")),
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
            Value::Iter(it) => {
                out.push_str(if matches!(**it, IterState::Done) { "<iter (done)>" } else { "<iter>" })
            }
            Value::Entry(_) => out.push_str("<entry>"),
            Value::Fs(_) => out.push_str("Fs"),
            Value::Env => out.push_str("Env"),
            Value::Clock => out.push_str("Clock"),
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
    pub(crate) fn format(&self, v: &Value, spec: &str, debug: bool) -> Result<String, Unwind> {
        let t = self.target(v)?;
        if spec.is_empty() {
            return Ok(if debug { self.debug_string(t) } else { self.display_string(t) });
        }
        if let (Value::DateTime(secs), false) = (t, debug) {
            return Ok(strftime(*secs, spec));
        }
        let s = Spec::parse(spec).ok_or_else(|| ice(format!("bad format spec `{spec}`")))?;
        let debug = debug || s.ty == Some('?');
        let (body, numeric) = match (t, s.ty) {
            _ if debug => (self.debug_string(t), false),
            (Value::Int(i), None | Some('d')) => (s.int_body(*i, 10, false), true),
            (Value::Int(i), Some('x')) => (s.int_body(*i, 16, false), true),
            (Value::Int(i), Some('X')) => (s.int_body(*i, 16, true), true),
            (Value::Int(i), Some('o')) => (s.int_body(*i, 8, false), true),
            (Value::Int(i), Some('b')) => (s.int_body(*i, 2, false), true),
            (Value::Int(i), Some(c @ ('e' | 'E' | 'f' | 'F' | '%'))) => {
                (s.float_body(*i as f64, Some(c)), true)
            }
            (Value::Float(f), ty) => (s.float_body(*f, ty), true),
            (other, _) => {
                let text = self.display_string(other);
                match s.precision {
                    Some(p) => (text.chars().take(p).collect(), false),
                    None => (text, false),
                }
            }
        };
        Ok(s.pad(body, numeric))
    }

    /// The locations an error passed through, oldest first.
    pub(crate) fn trace_lines(&self, v: &Value) -> Vec<String> {
        match self.target(v) {
            Ok(Value::Variant(a)) => a.trace.iter().map(|l| l.to_string()).collect(),
            _ => Vec::new(),
        }
    }
}

/// A parsed format spec: `[[fill]align][sign][#][0][width][,|_][.precision][type]`.
#[derive(Debug, Default)]
struct Spec {
    fill: char,
    align: Option<char>,
    sign: Option<char>,
    alt: bool,
    zero: bool,
    width: usize,
    grouping: Option<char>,
    precision: Option<usize>,
    ty: Option<char>,
}

impl Spec {
    fn parse(spec: &str) -> Option<Spec> {
        let cs: Vec<char> = spec.chars().collect();
        let mut s = Spec { fill: ' ', ..Spec::default() };
        let mut i = 0;
        let is_align = |c: char| matches!(c, '<' | '>' | '^' | '=');
        if cs.len() >= 2 && is_align(cs[1]) {
            s.fill = cs[0];
            s.align = Some(cs[1]);
            i = 2;
        } else if !cs.is_empty() && is_align(cs[0]) {
            s.align = Some(cs[0]);
            i = 1;
        }
        if let Some(&c) = cs.get(i)
            && matches!(c, '+' | '-' | ' ')
        {
            s.sign = Some(c);
            i += 1;
        }
        if cs.get(i) == Some(&'#') {
            s.alt = true;
            i += 1;
        }
        if cs.get(i) == Some(&'0') {
            s.zero = true;
            i += 1;
        }
        let start = i;
        while cs.get(i).is_some_and(|c| c.is_ascii_digit()) {
            i += 1;
        }
        if i > start {
            s.width = cs[start..i].iter().collect::<String>().parse().ok()?;
        }
        if let Some(&c) = cs.get(i)
            && (c == ',' || c == '_')
        {
            s.grouping = Some(c);
            i += 1;
        }
        if cs.get(i) == Some(&'.') {
            i += 1;
            let start = i;
            while cs.get(i).is_some_and(|c| c.is_ascii_digit()) {
                i += 1;
            }
            s.precision = Some(cs[start..i].iter().collect::<String>().parse().ok()?);
        }
        if let Some(&c) = cs.get(i) {
            if !matches!(c, 'b' | 'o' | 'x' | 'X' | 'd' | 'e' | 'E' | 'f' | 'F' | '%' | 's' | '?') {
                return None;
            }
            s.ty = Some(c);
            i += 1;
        }
        if i != cs.len() {
            return None;
        }
        Some(s)
    }

    fn sign_of(&self, negative: bool) -> &'static str {
        match (negative, self.sign) {
            (true, _) => "-",
            (false, Some('+')) => "+",
            (false, Some(' ')) => " ",
            _ => "",
        }
    }

    fn int_body(&self, i: i64, radix: u32, upper: bool) -> String {
        let mag = i.unsigned_abs();
        let mut digits = match radix {
            16 if upper => format!("{mag:X}"),
            16 => format!("{mag:x}"),
            8 => format!("{mag:o}"),
            2 => format!("{mag:b}"),
            _ => mag.to_string(),
        };
        if let Some(g) = self.grouping {
            digits = group(&digits, g, if radix == 10 { 3 } else { 4 });
        }
        let prefix = match (self.alt, radix) {
            (true, 16) => "0x",
            (true, 8) => "0o",
            (true, 2) => "0b",
            _ => "",
        };
        format!("{}{prefix}{digits}", self.sign_of(i < 0))
    }

    fn float_body(&self, f: f64, ty: Option<char>) -> String {
        if f.is_nan() {
            return "NaN".into();
        }
        let neg = f.is_sign_negative() && f != 0.0;
        let mag = f.abs();
        if mag.is_infinite() {
            return format!("{}inf", self.sign_of(neg));
        }
        let mut body = match ty {
            Some('f' | 'F') => format!("{mag:.*}", self.precision.unwrap_or(6)),
            Some('e' | 'E') => {
                let s = exponent_form(mag, self.precision.unwrap_or(6));
                if ty == Some('E') { s.to_uppercase() } else { s }
            }
            Some('%') => format!("{:.*}%", self.precision.unwrap_or(6), mag * 100.0),
            _ => match self.precision {
                Some(p) => format!("{mag:.p$}"),
                None => float_repr(mag),
            },
        };
        if let Some(g) = self.grouping {
            let (int_part, rest) = match body.find(|c: char| !c.is_ascii_digit()) {
                Some(k) => body.split_at(k),
                None => (body.as_str(), ""),
            };
            body = format!("{}{rest}", group(int_part, g, 3));
        }
        format!("{}{body}", self.sign_of(neg))
    }

    fn pad(&self, body: String, numeric: bool) -> String {
        let len = body.chars().count();
        if len >= self.width {
            return body;
        }
        let gap = self.width - len;
        let (fill, align) = if self.zero && self.align.is_none() && numeric {
            ('0', '=')
        } else {
            (self.fill, self.align.unwrap_or(if numeric { '>' } else { '<' }))
        };
        let fills = |n: usize| std::iter::repeat_n(fill, n).collect::<String>();
        match align {
            '<' => format!("{body}{}", fills(gap)),
            '^' => format!("{}{body}{}", fills(gap / 2), fills(gap - gap / 2)),
            '=' => {
                // Padding goes after the sign and any prefix.
                let mut split = body.chars().take_while(|c| matches!(c, '+' | '-' | ' ')).count();
                if ["0x", "0o", "0b"].iter().any(|p| body[split..].starts_with(p)) {
                    split += 2;
                }
                let (head, tail) = body.split_at(split);
                format!("{head}{}{tail}", fills(gap))
            }
            _ => format!("{}{body}", fills(gap)),
        }
    }
}

fn group(digits: &str, sep: char, every: usize) -> String {
    let cs: Vec<char> = digits.chars().collect();
    let mut out = String::new();
    for (i, c) in cs.iter().enumerate() {
        if i > 0 && (cs.len() - i).is_multiple_of(every) {
            out.push(sep);
        }
        out.push(*c);
    }
    out
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

/// Formats a time with the `strftime` directives `%Y %y %m %d %H %M %S
/// %F %T %%`; other characters are copied.
pub(crate) fn strftime(secs: i64, pattern: &str) -> String {
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
