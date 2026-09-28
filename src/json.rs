use std::collections::BTreeMap;
use std::fmt::Write as _;

#[derive(Clone, Debug)]
pub enum Json {
    Null,
    Bool(bool),
    Number(u64),
    Signed(i64),
    String(String),
    Array(Vec<Json>),
    Object(BTreeMap<String, Json>),
}

impl Json {
    pub fn string(value: impl Into<String>) -> Self {
        Json::String(value.into())
    }

    pub fn opt_string(value: Option<impl Into<String>>) -> Self {
        match value {
            Some(value) => Json::string(value),
            None => Json::Null,
        }
    }

    pub fn object() -> Self {
        Json::Object(BTreeMap::new())
    }

    pub fn put(&mut self, key: impl Into<String>, value: impl IntoJson) -> &mut Self {
        if let Json::Object(map) = self {
            map.insert(key.into(), value.into_json());
        }
        self
    }

    pub fn as_str(&self) -> Option<&str> {
        if let Json::String(value) = self {
            Some(value)
        } else {
            None
        }
    }

    pub fn as_u64(&self) -> Option<u64> {
        match self {
            Json::Number(value) => Some(*value),
            Json::Signed(value) if *value >= 0 => Some(*value as u64),
            _ => None,
        }
    }

    pub fn get(&self, key: &str) -> Option<&Json> {
        if let Json::Object(map) = self {
            map.get(key)
        } else {
            None
        }
    }

    pub fn to_string_compact(&self) -> String {
        let mut output = String::new();
        self.write(&mut output);
        output
    }

    fn write(&self, output: &mut String) {
        match self {
            Json::Null => output.push_str("null"),
            Json::Bool(value) => output.push_str(if *value { "true" } else { "false" }),
            Json::Number(value) => {
                let _ = write!(output, "{value}");
            }
            Json::Signed(value) => {
                let _ = write!(output, "{value}");
            }
            Json::String(value) => write_json_string(output, value),
            Json::Array(values) => {
                output.push('[');
                for (index, value) in values.iter().enumerate() {
                    if index > 0 {
                        output.push(',');
                    }
                    value.write(output);
                }
                output.push(']');
            }
            Json::Object(map) => {
                output.push('{');
                for (index, (key, value)) in map.iter().enumerate() {
                    if index > 0 {
                        output.push(',');
                    }
                    write_json_string(output, key);
                    output.push(':');
                    value.write(output);
                }
                output.push('}');
            }
        }
    }
}

pub trait IntoJson {
    fn into_json(self) -> Json;
}

impl IntoJson for Json {
    fn into_json(self) -> Json {
        self
    }
}

impl IntoJson for &str {
    fn into_json(self) -> Json {
        Json::String(self.to_owned())
    }
}

impl IntoJson for String {
    fn into_json(self) -> Json {
        Json::String(self)
    }
}

impl IntoJson for bool {
    fn into_json(self) -> Json {
        Json::Bool(self)
    }
}

impl IntoJson for u64 {
    fn into_json(self) -> Json {
        Json::Number(self)
    }
}

impl IntoJson for u32 {
    fn into_json(self) -> Json {
        Json::Number(self as u64)
    }
}

impl IntoJson for u16 {
    fn into_json(self) -> Json {
        Json::Number(self as u64)
    }
}

impl IntoJson for usize {
    fn into_json(self) -> Json {
        Json::Number(self as u64)
    }
}

impl IntoJson for i64 {
    fn into_json(self) -> Json {
        Json::Signed(self)
    }
}

impl<T: IntoJson> IntoJson for Vec<T> {
    fn into_json(self) -> Json {
        Json::Array(self.into_iter().map(IntoJson::into_json).collect())
    }
}

impl<T: IntoJson> IntoJson for Option<T> {
    fn into_json(self) -> Json {
        match self {
            Some(value) => value.into_json(),
            None => Json::Null,
        }
    }
}

pub fn write_json_string(output: &mut String, input: &str) {
    output.push('"');
    for ch in input.chars() {
        match ch {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            '\u{08}' => output.push_str("\\b"),
            '\u{0c}' => output.push_str("\\f"),
            ch if (ch as u32) < 0x20 => {
                let _ = write!(output, "\\u{:04x}", ch as u32);
            }
            ch => output.push(ch),
        }
    }
    output.push('"');
}

pub struct Parser<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Parser<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            bytes: input.as_bytes(),
            pos: 0,
        }
    }

    pub fn parse(&mut self) -> Result<Json, String> {
        self.skip_ws();
        let value = self.value()?;
        self.skip_ws();
        if self.pos != self.bytes.len() {
            return Err("trailing JSON".to_owned());
        }
        Ok(value)
    }

    fn value(&mut self) -> Result<Json, String> {
        self.skip_ws();
        match self.peek() {
            Some(b'"') => self.string().map(Json::String),
            Some(b'{') => self.object(),
            Some(b'[') => self.array(),
            Some(b't') | Some(b'f') => self.boolean(),
            Some(b'n') => self.null(),
            Some(ch) if ch == b'-' || ch.is_ascii_digit() => self.number(),
            _ => Err("expected JSON value".to_owned()),
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn skip_ws(&mut self) {
        while let Some(ch) = self.peek() {
            if ch.is_ascii_whitespace() {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    fn literal(&mut self, expected: &[u8]) -> Result<(), String> {
        if self.bytes[self.pos..].starts_with(expected) {
            self.pos += expected.len();
            Ok(())
        } else {
            Err("invalid JSON literal".to_owned())
        }
    }

    fn boolean(&mut self) -> Result<Json, String> {
        if self.bytes[self.pos..].starts_with(b"true") {
            self.pos += 4;
            Ok(Json::Bool(true))
        } else {
            self.literal(b"false")?;
            Ok(Json::Bool(false))
        }
    }

    fn null(&mut self) -> Result<Json, String> {
        self.literal(b"null")?;
        Ok(Json::Null)
    }

    fn number(&mut self) -> Result<Json, String> {
        let start = self.pos;
        if self.peek() == Some(b'-') {
            self.pos += 1;
        }
        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() {
                self.pos += 1;
            } else {
                break;
            }
        }
        let text = std::str::from_utf8(&self.bytes[start..self.pos]).map_err(|e| e.to_string())?;
        text.parse::<i64>()
            .map(Json::Signed)
            .map_err(|_| "unsupported JSON number".to_owned())
    }

    fn string(&mut self) -> Result<String, String> {
        self.pos += 1;
        let mut output = String::new();
        loop {
            let ch = self.bytes.get(self.pos).copied().ok_or("unterminated string")?;
            self.pos += 1;
            match ch {
                b'"' => return Ok(output),
                b'\\' => {
                    let escaped = self.bytes.get(self.pos).copied().ok_or("bad escape")?;
                    self.pos += 1;
                    match escaped {
                        b'"' => output.push('"'),
                        b'\\' => output.push('\\'),
                        b'/' => output.push('/'),
                        b'n' => output.push('\n'),
                        b'r' => output.push('\r'),
                        b't' => output.push('\t'),
                        b'b' => output.push('\u{08}'),
                        b'f' => output.push('\u{0c}'),
                        b'u' => {
                            if self.pos + 4 > self.bytes.len() {
                                return Err("bad unicode escape".to_owned());
                            }
                            let hex = std::str::from_utf8(&self.bytes[self.pos..self.pos + 4])
                                .map_err(|e| e.to_string())?;
                            let code =
                                u32::from_str_radix(hex, 16).map_err(|e| e.to_string())?;
                            self.pos += 4;
                            if (0xd800..=0xdbff).contains(&code) {
                                if self.bytes.get(self.pos..self.pos + 2) != Some(b"\\u".as_slice())
                                {
                                    return Err("expected surrogate pair".to_owned());
                                }
                                self.pos += 2;
                                let low_hex =
                                    std::str::from_utf8(&self.bytes[self.pos..self.pos + 4])
                                        .map_err(|e| e.to_string())?;
                                let low = u32::from_str_radix(low_hex, 16)
                                    .map_err(|e| e.to_string())?;
                                self.pos += 4;
                                let combined = 0x10000
                                    + ((code - 0xd800) << 10)
                                    + (low - 0xdc00);
                                output.push(char::from_u32(combined).ok_or("bad surrogate")?);
                            } else {
                                output.push(char::from_u32(code).ok_or("bad unicode")?);
                            }
                        }
                        _ => return Err("unknown escape".to_owned()),
                    }
                }
                _ => {
                    let start = self.pos - 1;
                    while *self
                        .bytes
                        .get(self.pos)
                        .ok_or("unterminated utf8 string")?
                        != b'"'
                        && *self.bytes.get(self.pos).ok_or("unterminated utf8 string")? != b'\\'
                    {
                        self.pos += 1;
                    }
                    output.push_str(
                        std::str::from_utf8(&self.bytes[start..self.pos])
                            .map_err(|e| e.to_string())?,
                    );
                }
            }
        }
    }

    fn array(&mut self) -> Result<Json, String> {
        self.pos += 1;
        let mut values = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b']') {
            self.pos += 1;
            return Ok(Json::Array(values));
        }
        loop {
            values.push(self.value()?);
            self.skip_ws();
            match self.peek() {
                Some(b',') => {
                    self.pos += 1;
                }
                Some(b']') => {
                    self.pos += 1;
                    return Ok(Json::Array(values));
                }
                _ => return Err("expected array delimiter".to_owned()),
            }
        }
    }

    fn object(&mut self) -> Result<Json, String> {
        self.pos += 1;
        let mut map = BTreeMap::new();
        self.skip_ws();
        if self.peek() == Some(b'}') {
            self.pos += 1;
            return Ok(Json::Object(map));
        }
        loop {
            self.skip_ws();
            let key = self.string()?;
            self.skip_ws();
            if self.peek() != Some(b':') {
                return Err("expected colon".to_owned());
            }
            self.pos += 1;
            let value = self.value()?;
            map.insert(key, value);
            self.skip_ws();
            match self.peek() {
                Some(b',') => {
                    self.pos += 1;
                }
                Some(b'}') => {
                    self.pos += 1;
                    return Ok(Json::Object(map));
                }
                _ => return Err("expected object delimiter".to_owned()),
            }
        }
    }
}

pub fn parse(input: &str) -> Result<Json, String> {
    Parser::new(input).parse()
}
