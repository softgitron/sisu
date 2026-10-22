//! The TOML that `sisu.toml` and `sisu.lock` are written in (09, "Package
//! manifest": the manifest stays data, D-063). A reader of the subset the
//! manifest needs, keeping every key's and value's position for
//! diagnostics, and a writer of the same subset.
//!
//! Accepted: `#` comments; `[table]` and `[a.b]` headers and `[[array]]`
//! headers; `key = value` with bare, quoted and dotted keys; basic strings
//! (`"..."` with the escapes `\" \\ \n \t \r \uXXXX \UXXXXXXXX`), literal
//! strings (`'...'`), multi-line basic strings (`"""..."""`); integers with
//! `_` separators; floats; `true` and `false`; arrays (over several lines,
//! with a trailing comma and comments); inline tables (`{ k = v, ... }`).
//! Dates and times are not part of the subset: nothing in the manifest
//! takes one.

use sisu_diag::TextRange;

/// A value with the range of its text.
#[derive(Clone, Debug, PartialEq)]
pub struct Spanned<T> {
    pub value: T,
    pub range: TextRange,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Str(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    Array(Vec<Spanned<Value>>),
    Table(Table),
}

impl Value {
    /// How the value is described in a diagnostic.
    pub fn describe(&self) -> &'static str {
        match self {
            Value::Str(_) => "a string",
            Value::Int(_) => "an integer",
            Value::Float(_) => "a float",
            Value::Bool(_) => "a boolean",
            Value::Array(_) => "an array",
            Value::Table(_) => "a table",
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_table(&self) -> Option<&Table> {
        match self {
            Value::Table(t) => Some(t),
            _ => None,
        }
    }
}

/// A table: its entries in the order written. A table made by a header
/// records where the header is.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Table {
    pub entries: Vec<(Spanned<String>, Spanned<Value>)>,
    /// Defined by a `[header]` (rather than implicitly by a dotted key or
    /// a longer header), so a second `[header]` is a duplicate.
    pub explicit: bool,
}

impl Table {
    pub fn get(&self, key: &str) -> Option<&Spanned<Value>> {
        self.entries.iter().find(|(k, _)| k.value == key).map(|(_, v)| v)
    }

    fn get_mut(&mut self, key: &str) -> Option<&mut Spanned<Value>> {
        self.entries.iter_mut().find(|(k, _)| k.value == key).map(|(_, v)| v)
    }
}

/// A syntax error: where, and what was expected and found (as the
/// parameters of `E1000`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error {
    pub range: TextRange,
    pub expected: String,
    pub found: String,
}

/// A key given twice: its second occurrence and the table's name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Duplicate {
    pub range: TextRange,
    pub key: String,
    pub table: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Problem {
    Syntax(Error),
    Duplicate(Duplicate),
}

struct Reader<'a> {
    text: &'a str,
    pos: usize,
}

impl<'a> Reader<'a> {
    fn peek(&self) -> Option<char> {
        self.text[self.pos..].chars().next()
    }

    fn peek_at(&self, n: usize) -> Option<char> {
        self.text[self.pos..].chars().nth(n)
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += c.len_utf8();
        Some(c)
    }

    fn starts_with(&self, s: &str) -> bool {
        self.text[self.pos..].starts_with(s)
    }

    fn here(&self) -> TextRange {
        TextRange::new(self.pos as u32, self.pos as u32)
    }

    fn range_from(&self, start: usize) -> TextRange {
        TextRange::new(start as u32, self.pos as u32)
    }

    /// What is at the position, for an error.
    fn found(&self) -> String {
        match self.peek() {
            None => "the end of the file".into(),
            Some('\n') | Some('\r') => "the end of the line".into(),
            Some(c) => format!("`{c}`"),
        }
    }

    fn error(&self, expected: &str) -> Problem {
        // A line break is found where the line ends, an empty range.
        let width = match self.peek() {
            None | Some('\n') | Some('\r') => 0,
            Some(c) => c.len_utf8(),
        };
        let end = self.pos + width;
        Problem::Syntax(Error {
            range: TextRange::new(self.pos as u32, end as u32),
            expected: expected.into(),
            found: self.found(),
        })
    }

    /// Spaces and tabs.
    fn skip_blank(&mut self) {
        while matches!(self.peek(), Some(' ') | Some('\t')) {
            self.pos += 1;
        }
    }

    /// Spaces, tabs, comments and line breaks.
    fn skip_space(&mut self) {
        loop {
            self.skip_blank();
            match self.peek() {
                Some('#') => self.skip_comment(),
                Some('\n') => self.pos += 1,
                Some('\r') if self.peek_at(1) == Some('\n') => self.pos += 2,
                _ => return,
            }
        }
    }

    fn skip_comment(&mut self) {
        while let Some(c) = self.peek() {
            if c == '\n' {
                break;
            }
            self.pos += c.len_utf8();
        }
    }

    /// The end of a line: blanks, an optional comment, then a line break or
    /// the end of the file.
    fn end_of_line(&mut self) -> Result<(), Problem> {
        self.skip_blank();
        if self.peek() == Some('#') {
            self.skip_comment();
        }
        match self.peek() {
            None => Ok(()),
            Some('\n') => {
                self.pos += 1;
                Ok(())
            }
            Some('\r') if self.peek_at(1) == Some('\n') => {
                self.pos += 2;
                Ok(())
            }
            _ => Err(self.error("the end of the line")),
        }
    }

    fn key_part(&mut self) -> Result<Spanned<String>, Problem> {
        let start = self.pos;
        match self.peek() {
            Some('"') => {
                let s = self.basic_string()?;
                Ok(Spanned { value: s, range: self.range_from(start) })
            }
            Some('\'') => {
                let s = self.literal_string()?;
                Ok(Spanned { value: s, range: self.range_from(start) })
            }
            Some(c) if is_bare(c) => {
                while self.peek().is_some_and(is_bare) {
                    self.pos += 1;
                }
                Ok(Spanned { value: self.text[start..self.pos].to_string(), range: self.range_from(start) })
            }
            _ => Err(self.error("a key")),
        }
    }

    /// `a.b.c`, as its parts.
    fn key(&mut self) -> Result<Vec<Spanned<String>>, Problem> {
        let mut parts = vec![self.key_part()?];
        loop {
            self.skip_blank();
            if self.peek() != Some('.') {
                return Ok(parts);
            }
            self.pos += 1;
            self.skip_blank();
            parts.push(self.key_part()?);
        }
    }

    fn basic_string(&mut self) -> Result<String, Problem> {
        if self.starts_with("\"\"\"") {
            return self.multiline_string();
        }
        self.pos += 1;
        let mut out = String::new();
        loop {
            match self.peek() {
                None | Some('\n') => return Err(self.error("`\"`, the end of the string")),
                Some('"') => {
                    self.pos += 1;
                    return Ok(out);
                }
                Some('\\') => out.push(self.escape()?),
                Some(c) => {
                    self.pos += c.len_utf8();
                    out.push(c);
                }
            }
        }
    }

    fn multiline_string(&mut self) -> Result<String, Problem> {
        self.pos += 3;
        // A line break right after the opening quotes is not part of it.
        if self.peek() == Some('\n') {
            self.pos += 1;
        } else if self.starts_with("\r\n") {
            self.pos += 2;
        }
        let mut out = String::new();
        loop {
            if self.starts_with("\"\"\"") {
                self.pos += 3;
                return Ok(out);
            }
            match self.peek() {
                None => return Err(self.error("`\"\"\"`, the end of the string")),
                Some('\\') => out.push(self.escape()?),
                Some(c) => {
                    self.pos += c.len_utf8();
                    out.push(c);
                }
            }
        }
    }

    fn escape(&mut self) -> Result<char, Problem> {
        let start = self.pos;
        self.pos += 1;
        let c = match self.bump() {
            Some('"') => '"',
            Some('\\') => '\\',
            Some('n') => '\n',
            Some('t') => '\t',
            Some('r') => '\r',
            Some('b') => '\u{8}',
            Some('f') => '\u{c}',
            Some(u @ ('u' | 'U')) => {
                let n = if u == 'u' { 4 } else { 8 };
                let digits = self.text.get(self.pos..self.pos + n).unwrap_or("");
                // Hex digits only: `u32::from_str_radix` would take a sign.
                let code = Some(digits)
                    .filter(|d| d.bytes().all(|b| b.is_ascii_hexdigit()))
                    .and_then(|d| u32::from_str_radix(d, 16).ok())
                    .and_then(char::from_u32);
                match code {
                    Some(c) if digits.len() == n => {
                        self.pos += n;
                        c
                    }
                    _ => {
                        self.pos = start;
                        return Err(self.error("an escape: `\\\"`, `\\\\`, `\\n`, `\\t`, `\\r` or `\\uXXXX`"));
                    }
                }
            }
            _ => {
                self.pos = start;
                return Err(self.error("an escape: `\\\"`, `\\\\`, `\\n`, `\\t`, `\\r` or `\\uXXXX`"));
            }
        };
        Ok(c)
    }

    fn literal_string(&mut self) -> Result<String, Problem> {
        self.pos += 1;
        let start = self.pos;
        loop {
            match self.peek() {
                None | Some('\n') => return Err(self.error("`'`, the end of the string")),
                Some('\'') => {
                    let s = self.text[start..self.pos].to_string();
                    self.pos += 1;
                    return Ok(s);
                }
                Some(c) => self.pos += c.len_utf8(),
            }
        }
    }

    fn value(&mut self) -> Result<Spanned<Value>, Problem> {
        let start = self.pos;
        let value = match self.peek() {
            Some('"') => Value::Str(self.basic_string()?),
            Some('\'') => Value::Str(self.literal_string()?),
            Some('[') => {
                self.pos += 1;
                let mut items = Vec::new();
                loop {
                    self.skip_space();
                    if self.peek() == Some(']') {
                        self.pos += 1;
                        break;
                    }
                    items.push(self.value()?);
                    self.skip_space();
                    match self.peek() {
                        Some(',') => self.pos += 1,
                        Some(']') => {
                            self.pos += 1;
                            break;
                        }
                        _ => return Err(self.error("`,` or `]`")),
                    }
                }
                Value::Array(items)
            }
            Some('{') => {
                self.pos += 1;
                let mut table = Table::default();
                self.skip_blank();
                if self.peek() == Some('}') {
                    self.pos += 1;
                } else {
                    loop {
                        self.skip_blank();
                        let key = self.key()?;
                        self.skip_blank();
                        if self.peek() != Some('=') {
                            return Err(self.error("`=`"));
                        }
                        self.pos += 1;
                        self.skip_blank();
                        let v = self.value()?;
                        insert(&mut table, &key, v, "")?;
                        self.skip_blank();
                        match self.peek() {
                            Some(',') => self.pos += 1,
                            Some('}') => {
                                self.pos += 1;
                                break;
                            }
                            _ => return Err(self.error("`,` or `}`")),
                        }
                    }
                }
                Value::Table(table)
            }
            Some(c) if c.is_ascii_digit() || c == '-' || c == '+' => {
                while self.peek().is_some_and(|c| c.is_ascii_alphanumeric() || "+-._".contains(c)) {
                    self.pos += 1;
                }
                let text: String = self.text[start..self.pos].chars().filter(|c| *c != '_').collect();
                if let Ok(n) = text.parse::<i64>() {
                    Value::Int(n)
                } else if let Ok(f) = text.parse::<f64>()
                    && text.chars().any(|c| c.is_ascii_digit())
                {
                    Value::Float(f)
                } else {
                    self.pos = start;
                    return Err(self.error("a number"));
                }
            }
            Some(c) if c.is_ascii_alphabetic() => {
                while self.peek().is_some_and(|c| c.is_ascii_alphanumeric()) {
                    self.pos += 1;
                }
                match &self.text[start..self.pos] {
                    "true" => Value::Bool(true),
                    "false" => Value::Bool(false),
                    _ => {
                        self.pos = start;
                        return Err(
                            self.error("a value: a string, a number, `true`, `false`, an array or a table")
                        );
                    }
                }
            }
            _ => return Err(self.error("a value: a string, a number, `true`, `false`, an array or a table")),
        };
        Ok(Spanned { value, range: self.range_from(start) })
    }
}

fn is_bare(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '-'
}

/// Inserts `value` at the dotted `key` of `table`, making the tables on the
/// way; `name` is the table's own name, for a duplicate's report.
fn insert(
    table: &mut Table,
    key: &[Spanned<String>],
    value: Spanned<Value>,
    name: &str,
) -> Result<(), Problem> {
    let Some((last, path)) = key.split_last() else { return Ok(()) };
    let mut t = table;
    let mut full = name.to_string();
    for part in path {
        if !full.is_empty() {
            full.push('.');
        }
        full.push_str(&part.value);
        if t.get(&part.value).is_none() {
            t.entries
                .push((part.clone(), Spanned { value: Value::Table(Table::default()), range: part.range }));
        }
        match t.get_mut(&part.value) {
            Some(Spanned { value: Value::Table(inner), .. }) => t = inner,
            _ => {
                return Err(Problem::Duplicate(Duplicate {
                    range: part.range,
                    key: part.value.clone(),
                    table: full,
                }));
            }
        }
    }
    if t.get(&last.value).is_some() {
        return Err(Problem::Duplicate(Duplicate {
            range: last.range,
            key: last.value.clone(),
            table: full,
        }));
    }
    t.entries.push((last.clone(), value));
    Ok(())
}

/// The table a `[header]` names, made when missing; for `[[header]]`, a
/// new table appended to the array at the header.
fn header_table<'t>(
    root: &'t mut Table,
    key: &[Spanned<String>],
    array: bool,
) -> Result<&'t mut Table, Problem> {
    let mut t = root;
    let mut full = String::new();
    for (i, part) in key.iter().enumerate() {
        let last = i + 1 == key.len();
        if !full.is_empty() {
            full.push('.');
        }
        full.push_str(&part.value);
        if t.get(&part.value).is_none() {
            let v = if last && array { Value::Array(Vec::new()) } else { Value::Table(Table::default()) };
            t.entries.push((part.clone(), Spanned { value: v, range: part.range }));
        }
        let dup = || {
            Problem::Duplicate(Duplicate { range: part.range, key: part.value.clone(), table: full.clone() })
        };
        let Some(slot) = t.get_mut(&part.value) else { return Err(dup()) };
        t = match &mut slot.value {
            Value::Table(inner) => {
                if last {
                    if array || inner.explicit {
                        return Err(dup());
                    }
                    inner.explicit = true;
                }
                inner
            }
            Value::Array(items) => {
                if last && !array {
                    return Err(dup());
                }
                if last {
                    items.push(Spanned {
                        value: Value::Table(Table { entries: Vec::new(), explicit: true }),
                        range: part.range,
                    });
                }
                match items.last_mut() {
                    Some(Spanned { value: Value::Table(inner), .. }) => inner,
                    _ => return Err(dup()),
                }
            }
            _ => return Err(dup()),
        };
    }
    Ok(t)
}

/// Reads a TOML document.
pub fn parse(text: &str) -> Result<Table, Problem> {
    let mut r = Reader { text, pos: 0 };
    let mut root = Table { entries: Vec::new(), explicit: true };
    // The header the following keys belong to.
    let mut current: Vec<Spanned<String>> = Vec::new();
    loop {
        r.skip_space();
        let Some(c) = r.peek() else { break };
        if c == '[' {
            let array = r.starts_with("[[");
            r.pos += if array { 2 } else { 1 };
            r.skip_blank();
            let key = r.key()?;
            r.skip_blank();
            let close = if array { "]]" } else { "]" };
            if !r.starts_with(close) {
                return Err(r.error(if array { "`]]`" } else { "`]`" }));
            }
            r.pos += close.len();
            r.end_of_line()?;
            header_table(&mut root, &key, array)?;
            current = key;
            continue;
        }
        let key = r.key()?;
        r.skip_blank();
        if r.peek() != Some('=') {
            return Err(r.error("`=`"));
        }
        r.pos += 1;
        r.skip_blank();
        if r.peek().is_none() || r.peek() == Some('\n') {
            return Err(r.error("a value: a string, a number, `true`, `false`, an array or a table"));
        }
        let value = r.value()?;
        r.end_of_line()?;
        let name: Vec<&str> = current.iter().map(|k| k.value.as_str()).collect();
        let name = name.join(".");
        match current_table(&mut root, &current) {
            Some(table) => insert(table, &key, value, &name)?,
            None => insert(&mut root, &key, value, &name)?,
        }
    }
    let _ = r.here();
    Ok(root)
}

/// The table the keys after a header go into: the header made it, so the
/// path is there (what is missing is made, as the header would).
fn current_table<'t>(t: &'t mut Table, key: &[Spanned<String>]) -> Option<&'t mut Table> {
    let Some((first, rest)) = key.split_first() else { return Some(t) };
    let idx = match t.entries.iter().position(|(k, _)| k.value == first.value) {
        Some(i) => i,
        None => {
            let v = Spanned { value: Value::Table(Table::default()), range: first.range };
            t.entries.push((first.clone(), v));
            t.entries.len() - 1
        }
    };
    let slot = &mut t.entries[idx].1.value;
    match &mut *slot {
        Value::Array(items) => {
            if !matches!(items.last(), Some(Spanned { value: Value::Table(_), .. })) {
                items.push(Spanned { value: Value::Table(Table::default()), range: first.range });
            }
        }
        Value::Table(_) => {}
        other => *other = Value::Table(Table::default()),
    }
    match slot {
        Value::Array(items) => match items.last_mut() {
            Some(Spanned { value: Value::Table(inner), .. }) => current_table(inner, rest),
            _ => None,
        },
        Value::Table(inner) => current_table(inner, rest),
        _ => None,
    }
}

/// A string as TOML writes it: a basic string with the escapes it needs.
pub fn quote(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04X}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &Spanned<Value>) -> &str {
        v.value.as_str().unwrap_or("")
    }

    #[test]
    fn reads_a_manifest() {
        let t = parse(
            "# a comment\n[package]\nname = \"weather\"   # trailing\nversion = '0.3.1'\n\n[dependencies]\nmylib = { path = \"../mylib\" }\nwinapi = { path = \"w\", when = { os = \"windows\" } }\n\n[trust]\nunsafe = [\n  \"mylib\",  # ok\n]\n",
        )
        .unwrap();
        let p = t.get("package").unwrap().value.as_table().unwrap();
        assert_eq!(s(p.get("name").unwrap()), "weather");
        assert_eq!(s(p.get("version").unwrap()), "0.3.1");
        let d = t.get("dependencies").unwrap().value.as_table().unwrap();
        let w = d.get("winapi").unwrap().value.as_table().unwrap();
        let when = w.get("when").unwrap().value.as_table().unwrap();
        assert_eq!(s(when.get("os").unwrap()), "windows");
        let trust = t.get("trust").unwrap().value.as_table().unwrap();
        assert!(matches!(&trust.get("unsafe").unwrap().value, Value::Array(a) if a.len() == 1));
    }

    #[test]
    fn dotted_keys_and_arrays_of_tables() {
        let t = parse("a.b = 1\n[[package]]\nname = \"x\"\n[[package]]\nname = \"y\"\n").unwrap();
        let a = t.get("a").unwrap().value.as_table().unwrap();
        assert_eq!(a.get("b").unwrap().value, Value::Int(1));
        let Value::Array(ps) = &t.get("package").unwrap().value else { panic!() };
        assert_eq!(ps.len(), 2);
    }

    #[test]
    fn reports_where() {
        let e = parse("[package]\nname \"x\"\n").unwrap_err();
        let Problem::Syntax(e) = e else { panic!() };
        assert_eq!(e.range.start, 15);
        assert_eq!(e.expected, "`=`");
        let d = parse("[package]\nname = \"x\"\nname = \"y\"\n").unwrap_err();
        assert!(matches!(d, Problem::Duplicate(Duplicate { ref key, .. }) if key == "name"));
        assert!(parse("[a]\n[a]\n").is_err());
        assert!(parse("x = \"open\n").is_err());
    }

    #[test]
    fn escapes_and_quoting_round_trip() {
        let t = parse("x = \"a\\\"b\\\\c\\n\\u00e9\"\n").unwrap();
        assert_eq!(s(t.get("x").unwrap()), "a\"b\\c\né");
        let q = quote("a\"b\\c\n");
        let t = parse(&format!("y = {q}\n")).unwrap();
        assert_eq!(s(t.get("y").unwrap()), "a\"b\\c\n");
    }
}
