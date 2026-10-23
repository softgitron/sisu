//! JSON for the language server's protocol (`sisu lsp`): a value, a
//! reader and a writer. Stage 0 takes nothing from a registry, so this is
//! the small part of JSON that JSON-RPC needs, written out: numbers are
//! `f64`, objects keep their keys in order, and strings take every escape
//! of RFC 8259, `\uXXXX` surrogate pairs included.

#[derive(Clone, Debug, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

impl Json {
    /// An object from its fields.
    pub fn obj(fields: Vec<(&str, Json)>) -> Json {
        Json::Obj(fields.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
    }

    pub fn str(s: impl Into<String>) -> Json {
        Json::Str(s.into())
    }

    pub fn num(n: impl Into<f64>) -> Json {
        Json::Num(n.into())
    }

    /// A field of an object.
    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Obj(fields) => fields.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    /// The value at a path of fields.
    pub fn at(&self, path: &[&str]) -> Option<&Json> {
        path.iter().try_fold(self, |v, k| v.get(k))
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Json::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_u64(&self) -> Option<u64> {
        match self {
            Json::Num(n) if *n >= 0.0 && n.fract() == 0.0 => Some(*n as u64),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[Json]> {
        match self {
            Json::Arr(a) => Some(a),
            _ => None,
        }
    }

    /// The text of the value, on one line.
    pub fn write(&self) -> String {
        let mut out = String::new();
        self.write_to(&mut out);
        out
    }

    fn write_to(&self, out: &mut String) {
        match self {
            Json::Null => out.push_str("null"),
            Json::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Json::Num(n) => {
                if n.is_finite() && n.fract() == 0.0 && n.abs() < 1e15 {
                    out.push_str(&format!("{}", *n as i64));
                } else if n.is_finite() {
                    out.push_str(&format!("{n}"));
                } else {
                    out.push_str("null");
                }
            }
            Json::Str(s) => quote(s, out),
            Json::Arr(items) => {
                out.push('[');
                for (i, v) in items.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    v.write_to(out);
                }
                out.push(']');
            }
            Json::Obj(fields) => {
                out.push('{');
                for (i, (k, v)) in fields.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    quote(k, out);
                    out.push(':');
                    v.write_to(out);
                }
                out.push('}');
            }
        }
    }
}

fn quote(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

/// Reads a JSON text: the value, or where it is not JSON.
pub fn parse(text: &str) -> Result<Json, usize> {
    let mut r = Reader { s: text.as_bytes(), at: 0 };
    let v = r.value()?;
    r.space();
    if r.at != r.s.len() {
        return Err(r.at);
    }
    Ok(v)
}

struct Reader<'a> {
    s: &'a [u8],
    at: usize,
}

impl Reader<'_> {
    fn space(&mut self) {
        while self.at < self.s.len() && matches!(self.s[self.at], b' ' | b'\t' | b'\n' | b'\r') {
            self.at += 1;
        }
    }

    fn eat(&mut self, word: &str) -> bool {
        if self.s[self.at..].starts_with(word.as_bytes()) {
            self.at += word.len();
            true
        } else {
            false
        }
    }

    fn value(&mut self) -> Result<Json, usize> {
        self.space();
        let Some(&c) = self.s.get(self.at) else { return Err(self.at) };
        match c {
            b'n' if self.eat("null") => Ok(Json::Null),
            b't' if self.eat("true") => Ok(Json::Bool(true)),
            b'f' if self.eat("false") => Ok(Json::Bool(false)),
            b'"' => self.string().map(Json::Str),
            b'[' => {
                self.at += 1;
                let mut items = Vec::new();
                self.space();
                if self.s.get(self.at) == Some(&b']') {
                    self.at += 1;
                    return Ok(Json::Arr(items));
                }
                loop {
                    items.push(self.value()?);
                    self.space();
                    match self.s.get(self.at) {
                        Some(b',') => self.at += 1,
                        Some(b']') => {
                            self.at += 1;
                            return Ok(Json::Arr(items));
                        }
                        _ => return Err(self.at),
                    }
                }
            }
            b'{' => {
                self.at += 1;
                let mut fields = Vec::new();
                self.space();
                if self.s.get(self.at) == Some(&b'}') {
                    self.at += 1;
                    return Ok(Json::Obj(fields));
                }
                loop {
                    self.space();
                    if self.s.get(self.at) != Some(&b'"') {
                        return Err(self.at);
                    }
                    let k = self.string()?;
                    self.space();
                    if self.s.get(self.at) != Some(&b':') {
                        return Err(self.at);
                    }
                    self.at += 1;
                    let v = self.value()?;
                    fields.push((k, v));
                    self.space();
                    match self.s.get(self.at) {
                        Some(b',') => self.at += 1,
                        Some(b'}') => {
                            self.at += 1;
                            return Ok(Json::Obj(fields));
                        }
                        _ => return Err(self.at),
                    }
                }
            }
            b'-' | b'0'..=b'9' => {
                let start = self.at;
                while self.at < self.s.len()
                    && matches!(self.s[self.at], b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9')
                {
                    self.at += 1;
                }
                let text = std::str::from_utf8(&self.s[start..self.at]).map_err(|_| start)?;
                text.parse::<f64>().map(Json::Num).map_err(|_| start)
            }
            _ => Err(self.at),
        }
    }

    fn hex4(&mut self) -> Result<u32, usize> {
        let digits = self.s.get(self.at..self.at + 4).ok_or(self.at)?;
        if !digits.iter().all(u8::is_ascii_hexdigit) {
            return Err(self.at);
        }
        let text = std::str::from_utf8(digits).map_err(|_| self.at)?;
        let n = u32::from_str_radix(text, 16).map_err(|_| self.at)?;
        self.at += 4;
        Ok(n)
    }

    fn string(&mut self) -> Result<String, usize> {
        self.at += 1;
        let mut out = String::new();
        let mut run = self.at;
        loop {
            let Some(&c) = self.s.get(self.at) else { return Err(self.at) };
            match c {
                b'"' => {
                    out.push_str(std::str::from_utf8(&self.s[run..self.at]).map_err(|_| run)?);
                    self.at += 1;
                    return Ok(out);
                }
                b'\\' => {
                    out.push_str(std::str::from_utf8(&self.s[run..self.at]).map_err(|_| run)?);
                    self.at += 1;
                    let Some(&e) = self.s.get(self.at) else { return Err(self.at) };
                    self.at += 1;
                    match e {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'b' => out.push('\u{8}'),
                        b'f' => out.push('\u{c}'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => {
                            let mut n = self.hex4()?;
                            // A surrogate pair.
                            if (0xD800..0xDC00).contains(&n) && self.eat("\\u") {
                                let low = self.hex4()?;
                                n = 0x10000 + ((n - 0xD800) << 10) + (low.wrapping_sub(0xDC00) & 0x3FF);
                            }
                            out.push(char::from_u32(n).unwrap_or('\u{FFFD}'));
                        }
                        _ => return Err(self.at - 1),
                    }
                    run = self.at;
                }
                _ => self.at += 1,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_what_it_writes() {
        let text = r#"{"a":[1,2.5,-3,true,null],"b":"x\"y\\z\né😀","c":{}}"#;
        let v = parse(text).expect("json");
        assert_eq!(v.at(&["b"]).and_then(Json::as_str), Some("x\"y\\z\né😀"));
        assert_eq!(parse(&v.write()), Ok(v));
    }

    #[test]
    fn says_where_it_is_not_json() {
        assert_eq!(parse("{\"a\" 1}"), Err(5));
        assert!(parse("[1,").is_err());
    }
}
