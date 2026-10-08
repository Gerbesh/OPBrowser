//! Strict, bounded JSON tokenizer/parser for OPBrowser's own JavaScript runtime.
//! JSON syntax is never evaluated as executable JavaScript.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum JsonValue {
    Null,
    Boolean(bool),
    Number(f64),
    String(String),
    Array(Vec<JsonValue>),
    Object(Vec<(String, JsonValue)>),
}

const MAX_BYTES: usize = 64 * 1024;
const MAX_DEPTH: usize = 64;
const MAX_ELEMENTS: usize = 4096;

pub(crate) fn parse(source: &str) -> Result<JsonValue, String> {
    if source.len() > MAX_BYTES {
        return Err("JSON input exceeds 64 KiB".into());
    }
    let mut parser = Parser {
        input: source,
        offset: 0,
        elements: 0,
    };
    let value = parser.value(0)?;
    parser.whitespace();
    if parser.offset != source.len() {
        return Err(format!("unexpected content at byte {}", parser.offset));
    }
    Ok(value)
}

struct Parser<'a> {
    input: &'a str,
    offset: usize,
    elements: usize,
}

impl Parser<'_> {
    fn byte(&self) -> Option<u8> {
        self.input.as_bytes().get(self.offset).copied()
    }

    fn advance(&mut self, expected: u8) -> bool {
        if self.byte() == Some(expected) {
            self.offset += 1;
            true
        } else {
            false
        }
    }

    fn whitespace(&mut self) {
        while matches!(self.byte(), Some(b' ' | b'\t' | b'\r' | b'\n')) {
            self.offset += 1;
        }
    }

    fn literal(&mut self, expected: &str, value: JsonValue) -> Result<JsonValue, String> {
        if self.input[self.offset..].starts_with(expected) {
            self.offset += expected.len();
            Ok(value)
        } else {
            Err(format!("invalid literal at byte {}", self.offset))
        }
    }

    fn element(&mut self) -> Result<(), String> {
        self.elements += 1;
        if self.elements > MAX_ELEMENTS {
            return Err("JSON node budget exceeded".into());
        }
        Ok(())
    }

    fn value(&mut self, depth: usize) -> Result<JsonValue, String> {
        if depth > MAX_DEPTH {
            return Err("JSON nesting limit exceeded".into());
        }
        self.whitespace();
        self.element()?;
        match self.byte() {
            Some(b'n') => self.literal("null", JsonValue::Null),
            Some(b't') => self.literal("true", JsonValue::Boolean(true)),
            Some(b'f') => self.literal("false", JsonValue::Boolean(false)),
            Some(b'"') => self.string().map(JsonValue::String),
            Some(b'[') => self.array(depth + 1),
            Some(b'{') => self.object(depth + 1),
            Some(b'-' | b'0'..=b'9') => self.number().map(JsonValue::Number),
            _ => Err(format!("invalid JSON value at byte {}", self.offset)),
        }
    }

    fn array(&mut self, depth: usize) -> Result<JsonValue, String> {
        self.offset += 1;
        self.whitespace();
        let mut values = Vec::new();
        if self.advance(b']') {
            return Ok(JsonValue::Array(values));
        }
        loop {
            values.push(self.value(depth)?);
            self.whitespace();
            if self.advance(b']') {
                return Ok(JsonValue::Array(values));
            }
            if !self.advance(b',') {
                return Err(format!("expected comma or ] at byte {}", self.offset));
            }
        }
    }

    fn object(&mut self, depth: usize) -> Result<JsonValue, String> {
        self.offset += 1;
        self.whitespace();
        let mut values = Vec::new();
        if self.advance(b'}') {
            return Ok(JsonValue::Object(values));
        }
        loop {
            self.whitespace();
            if self.byte() != Some(b'"') {
                return Err(format!("expected JSON key at byte {}", self.offset));
            }
            let key = self.string()?;
            self.whitespace();
            if !self.advance(b':') {
                return Err(format!("expected colon at byte {}", self.offset));
            }
            let value = self.value(depth)?;
            values.push((key, value));
            self.whitespace();
            if self.advance(b'}') {
                return Ok(JsonValue::Object(values));
            }
            if !self.advance(b',') {
                return Err(format!("expected comma or }} at byte {}", self.offset));
            }
        }
    }

    fn number(&mut self) -> Result<f64, String> {
        let start = self.offset;
        self.advance(b'-');
        if !self.advance(b'0') {
            self.digits(true)?;
        } else if matches!(self.byte(), Some(b'0'..=b'9')) {
            return Err(format!("leading zero at byte {}", self.offset));
        }
        if self.advance(b'.') {
            self.digits(true)?;
        }
        if self.advance(b'e') || self.advance(b'E') {
            if !self.advance(b'+') {
                self.advance(b'-');
            }
            self.digits(true)?;
        }
        self.input[start..self.offset]
            .parse::<f64>()
            .map_err(|_| format!("invalid JSON number at byte {start}"))
    }

    fn digits(&mut self, required: bool) -> Result<(), String> {
        let start = self.offset;
        while matches!(self.byte(), Some(b'0'..=b'9')) {
            self.offset += 1;
        }
        if required && start == self.offset {
            return Err(format!("missing JSON digits at byte {}", self.offset));
        }
        Ok(())
    }

    fn hex4(&mut self) -> Result<u16, String> {
        let mut number = 0u16;
        for _ in 0..4 {
            let Some(digit) = self.byte().and_then(|b| (b as char).to_digit(16)) else {
                return Err(format!(
                    "invalid JSON Unicode escape at byte {}",
                    self.offset
                ));
            };
            self.offset += 1;
            number = (number << 4) | digit as u16;
        }
        Ok(number)
    }

    fn string(&mut self) -> Result<String, String> {
        self.offset += 1; // opening quote was validated by caller
        let mut result = String::new();
        loop {
            match self.byte() {
                None => return Err("unterminated JSON string".into()),
                Some(b'"') => {
                    self.offset += 1;
                    return Ok(result);
                }
                Some(b'\\') => {
                    self.offset += 1;
                    let escaped = self.byte().ok_or("unterminated JSON escape")?;
                    self.offset += 1;
                    match escaped {
                        b'"' => result.push('"'),
                        b'\\' => result.push('\\'),
                        b'/' => result.push('/'),
                        b'b' => result.push('\u{0008}'),
                        b'f' => result.push('\u{000c}'),
                        b'n' => result.push('\n'),
                        b'r' => result.push('\r'),
                        b't' => result.push('\t'),
                        b'u' => {
                            let high = self.hex4()?;
                            if (0xD800..=0xDBFF).contains(&high) {
                                if !self.advance(b'\\') || !self.advance(b'u') {
                                    return Err("unpaired high surrogate".into());
                                }
                                let low = self.hex4()?;
                                if !(0xDC00..=0xDFFF).contains(&low) {
                                    return Err("unpaired high surrogate".into());
                                }
                                let scalar = 0x10000
                                    + ((u32::from(high) - 0xD800) << 10)
                                    + (u32::from(low) - 0xDC00);
                                result.push(char::from_u32(scalar).expect("valid surrogate pair"));
                            } else if (0xDC00..=0xDFFF).contains(&high) {
                                return Err("unpaired low surrogate".into());
                            } else {
                                result.push(char::from_u32(high as u32).expect("non-surrogate"));
                            }
                        }
                        _ => return Err(format!("unknown JSON escape at byte {}", self.offset)),
                    }
                }
                Some(byte) if byte < 32 => {
                    return Err(format!(
                        "control character in JSON string at byte {}",
                        self.offset
                    ));
                }
                _ => {
                    let ch = self.input[self.offset..]
                        .chars()
                        .next()
                        .expect("valid UTF-8");
                    result.push(ch);
                    self.offset += ch.len_utf8();
                }
            }
        }
    }
}

pub(crate) fn quote(value: &str) -> String {
    let mut output = String::from("\"");
    for ch in value.chars() {
        match ch {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\u{0008}' => output.push_str("\\b"),
            '\u{000c}' => output.push_str("\\f"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            ch if ch < '\u{0020}' => {
                use std::fmt::Write;
                write!(output, "\\u{:04x}", ch as u32).expect("write to String");
            }
            _ => output.push(ch),
        }
    }
    output.push('"');
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strict_literals_numbers_and_surrogate_pairs() {
        assert_eq!(
            parse(" [null,true,-1.5e2,\"\\uD83D\\uDE80\"] ").unwrap(),
            JsonValue::Array(vec![
                JsonValue::Null,
                JsonValue::Boolean(true),
                JsonValue::Number(-150.0),
                JsonValue::String("🚀".into()),
            ])
        );
        assert_eq!(
            parse("{\"x\":1,\"x\":2}").unwrap(),
            JsonValue::Object(vec![
                ("x".into(), JsonValue::Number(1.0)),
                ("x".into(), JsonValue::Number(2.0))
            ])
        );
    }

    #[test]
    fn malformed_json_is_never_executable() {
        for bad in [
            "{a:1}",
            "[1,]",
            "{\"x\":}",
            "01",
            "+1",
            "1.",
            "1e",
            "NaN",
            "undefined",
            "'hi'",
            "\"\n\"",
            "\"\\uD800\"",
            "\"\\uDC00\"",
            "\"\\uD83D\\u0041\"",
            "false;",
            "1 2",
            "{\"constructor\":function(){}}",
        ] {
            assert!(parse(bad).is_err(), "accepted {bad:?}");
        }
    }

    #[test]
    fn string_encoding_roundtrips_control_and_unicode() {
        let source = "\u{0000}\nПривет \"🌐\\";
        assert_eq!(
            parse(&quote(source)).unwrap(),
            JsonValue::String(source.into())
        );
    }
}
