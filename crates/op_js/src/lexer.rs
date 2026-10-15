use crate::JsError;

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Number(f64),
    String(String),
    TemplateNoSub(String),
    TemplateHead(String),
    TemplateMiddle(String),
    TemplateTail(String),
    TemplateExprEnd,
    Identifier(String),
    Let,
    Const,
    Var,
    If,
    Else,
    While,
    For,
    Do,
    Switch,
    Case,
    Default,
    Break,
    Continue,
    Function,
    Return,
    This,
    New,
    Void,
    Typeof,
    InstanceOf,
    In,
    Throw,
    Try,
    Catch,
    Finally,
    True,
    False,
    Null,
    Undefined,
    Plus,
    PlusPlus,
    Minus,
    MinusMinus,
    Star,
    Slash,
    Percent,
    Bang,
    Equal,
    EqualEqual,
    EqualEqualEqual,
    BangEqual,
    BangEqualEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    AmpAmp,
    PipePipe,
    Amp,
    Pipe,
    Caret,
    Tilde,
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    LeftBracket,
    RightBracket,
    Dot,
    Question,
    Colon,
    Semicolon,
    Comma,
    Eof,
}

pub fn tokenize(source: &str) -> Result<Vec<Token>, JsError> {
    Lexer::new(source).tokenize()
}

struct Lexer<'a> {
    source: &'a str,
    bytes: &'a [u8],
    index: usize,
    templates: Vec<TemplateFrame>,
}

struct TemplateFrame {
    brace_depth: usize,
    awaiting_chunk: bool,
}

impl<'a> Lexer<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            source,
            bytes: source.as_bytes(),
            index: 0,
            templates: Vec::new(),
        }
    }

    fn tokenize(mut self) -> Result<Vec<Token>, JsError> {
        let mut tokens = Vec::new();
        while self.index < self.bytes.len() {
            if !self
                .templates
                .last()
                .is_some_and(|frame| frame.awaiting_chunk)
            {
                self.skip_space_and_comments()?;
            }
            if self.index >= self.bytes.len() {
                break;
            }
            tokens.push(self.next_token()?);
        }
        if !self.templates.is_empty() {
            return Err(JsError::syntax(self.index, "unterminated template literal"));
        }
        tokens.push(Token {
            kind: TokenKind::Eof,
            start: self.bytes.len(),
            end: self.bytes.len(),
        });
        Ok(tokens)
    }

    fn skip_space_and_comments(&mut self) -> Result<(), JsError> {
        loop {
            while self
                .bytes
                .get(self.index)
                .is_some_and(|byte| byte.is_ascii_whitespace())
            {
                self.index += 1;
            }

            if self.bytes.get(self.index..self.index + 2) == Some(b"//") {
                self.index += 2;
                while self
                    .bytes
                    .get(self.index)
                    .is_some_and(|byte| !matches!(byte, b'\n' | b'\r'))
                {
                    self.index += 1;
                }
                continue;
            }

            if self.bytes.get(self.index..self.index + 2) == Some(b"/*") {
                let start = self.index;
                self.index += 2;
                while self.index + 1 < self.bytes.len()
                    && self.bytes.get(self.index..self.index + 2) != Some(b"*/")
                {
                    self.index += 1;
                }
                if self.index + 1 >= self.bytes.len() {
                    return Err(JsError::syntax(start, "unterminated block comment"));
                }
                self.index += 2;
                continue;
            }

            return Ok(());
        }
    }

    fn next_token(&mut self) -> Result<Token, JsError> {
        if self
            .templates
            .last()
            .is_some_and(|frame| frame.awaiting_chunk)
        {
            return self.template_chunk(false);
        }
        let start = self.index;
        let byte = self.bytes[start];
        if byte == b'`' {
            self.index += 1;
            return self.template_chunk(true);
        }
        if byte == b'}'
            && self
                .templates
                .last()
                .is_some_and(|frame| frame.brace_depth == 0)
        {
            self.index += 1;
            self.templates
                .last_mut()
                .expect("template frame exists")
                .awaiting_chunk = true;
            return Ok(Token {
                kind: TokenKind::TemplateExprEnd,
                start,
                end: self.index,
            });
        }
        if byte == b'{' {
            if let Some(frame) = self.templates.last_mut() {
                frame.brace_depth += 1;
            }
        } else if byte == b'}'
            && let Some(frame) = self.templates.last_mut()
        {
            frame.brace_depth -= 1;
        }

        if byte.is_ascii_digit()
            || (byte == b'.' && self.bytes.get(start + 1).is_some_and(u8::is_ascii_digit))
        {
            return self.number();
        }

        if is_identifier_start(byte) {
            return Ok(self.identifier());
        }

        if matches!(byte, b'\'' | b'"') {
            return self.string();
        }

        self.index += 1;
        let kind = match byte {
            b'+' if self.take(b'+') => TokenKind::PlusPlus,
            b'+' => TokenKind::Plus,
            b'-' if self.take(b'-') => TokenKind::MinusMinus,
            b'-' => TokenKind::Minus,
            b'*' => TokenKind::Star,
            b'/' => TokenKind::Slash,
            b'%' => TokenKind::Percent,
            b'(' => TokenKind::LeftParen,
            b')' => TokenKind::RightParen,
            b'{' => TokenKind::LeftBrace,
            b'}' => TokenKind::RightBrace,
            b'[' => TokenKind::LeftBracket,
            b']' => TokenKind::RightBracket,
            b'.' => TokenKind::Dot,
            b':' => TokenKind::Colon,
            b'?' => TokenKind::Question,
            b';' => TokenKind::Semicolon,
            b',' => TokenKind::Comma,
            b'&' if self.take(b'&') => TokenKind::AmpAmp,
            b'&' => TokenKind::Amp,
            b'|' if self.take(b'|') => TokenKind::PipePipe,
            b'|' => TokenKind::Pipe,
            b'^' => TokenKind::Caret,
            b'~' => TokenKind::Tilde,
            b'=' if self.take(b'=') => {
                if self.take(b'=') {
                    TokenKind::EqualEqualEqual
                } else {
                    TokenKind::EqualEqual
                }
            }
            b'=' => TokenKind::Equal,
            b'!' if self.take(b'=') => {
                if self.take(b'=') {
                    TokenKind::BangEqualEqual
                } else {
                    TokenKind::BangEqual
                }
            }
            b'!' => TokenKind::Bang,
            b'<' if self.take(b'=') => TokenKind::LessEqual,
            b'<' => TokenKind::Less,
            b'>' if self.take(b'=') => TokenKind::GreaterEqual,
            b'>' => TokenKind::Greater,
            _ => {
                return Err(JsError::syntax(
                    start,
                    format!("unsupported character {:?}", byte as char),
                ));
            }
        };

        Ok(Token {
            kind,
            start,
            end: self.index,
        })
    }

    fn take(&mut self, expected: u8) -> bool {
        if self.bytes.get(self.index) == Some(&expected) {
            self.index += 1;
            true
        } else {
            false
        }
    }

    fn number(&mut self) -> Result<Token, JsError> {
        let start = self.index;
        let mut saw_dot = false;

        if self.bytes.get(self.index) == Some(&b'.') {
            saw_dot = true;
            self.index += 1;
        }
        while self.bytes.get(self.index).is_some_and(u8::is_ascii_digit) {
            self.index += 1;
        }
        if !saw_dot && self.bytes.get(self.index) == Some(&b'.') {
            self.index += 1;
            while self.bytes.get(self.index).is_some_and(u8::is_ascii_digit) {
                self.index += 1;
            }
        }

        if matches!(self.bytes.get(self.index), Some(b'e' | b'E')) {
            self.index += 1;
            if matches!(self.bytes.get(self.index), Some(b'+' | b'-')) {
                self.index += 1;
            }
            let exponent_start = self.index;
            while self.bytes.get(self.index).is_some_and(u8::is_ascii_digit) {
                self.index += 1;
            }
            if exponent_start == self.index {
                return Err(JsError::syntax(start, "invalid numeric exponent"));
            }
        }

        let text = &self.source[start..self.index];
        let value = text
            .parse::<f64>()
            .map_err(|_| JsError::syntax(start, "invalid numeric literal"))?;

        Ok(Token {
            kind: TokenKind::Number(value),
            start,
            end: self.index,
        })
    }

    fn identifier(&mut self) -> Token {
        let start = self.index;
        self.index += 1;
        while self
            .bytes
            .get(self.index)
            .is_some_and(|byte| is_identifier_continue(*byte))
        {
            self.index += 1;
        }
        let text = &self.source[start..self.index];
        let kind = match text {
            "let" => TokenKind::Let,
            "const" => TokenKind::Const,
            "var" => TokenKind::Var,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "while" => TokenKind::While,
            "for" => TokenKind::For,
            "do" => TokenKind::Do,
            "switch" => TokenKind::Switch,
            "case" => TokenKind::Case,
            "default" => TokenKind::Default,
            "break" => TokenKind::Break,
            "continue" => TokenKind::Continue,
            "function" => TokenKind::Function,
            "return" => TokenKind::Return,
            "this" => TokenKind::This,
            "new" => TokenKind::New,
            "void" => TokenKind::Void,
            "typeof" => TokenKind::Typeof,
            "instanceof" => TokenKind::InstanceOf,
            "in" => TokenKind::In,
            "throw" => TokenKind::Throw,
            "try" => TokenKind::Try,
            "catch" => TokenKind::Catch,
            "finally" => TokenKind::Finally,
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            "null" => TokenKind::Null,
            "undefined" => TokenKind::Undefined,
            _ => TokenKind::Identifier(text.into()),
        };
        Token {
            kind,
            start,
            end: self.index,
        }
    }

    /// Scan either the opening quasi or the raw continuation after an
    /// interpolation. A template frame stays active until its closing `.
    fn template_chunk(&mut self, opening: bool) -> Result<Token, JsError> {
        let start = if opening { self.index - 1 } else { self.index };
        let mut cooked = String::new();
        while let Some(&byte) = self.bytes.get(self.index) {
            if byte == b'`' {
                self.index += 1;
                if !opening {
                    self.templates.pop();
                }
                return Ok(Token {
                    kind: if opening {
                        TokenKind::TemplateNoSub(cooked)
                    } else {
                        TokenKind::TemplateTail(cooked)
                    },
                    start,
                    end: self.index,
                });
            }
            if byte == b'$' && self.bytes.get(self.index + 1) == Some(&b'{') {
                self.index += 2;
                if opening {
                    if self.templates.len() >= 32 {
                        return Err(JsError::syntax(start, "template nesting budget exceeded"));
                    }
                    self.templates.push(TemplateFrame {
                        brace_depth: 0,
                        awaiting_chunk: false,
                    });
                } else {
                    self.templates
                        .last_mut()
                        .expect("template continuation frame")
                        .awaiting_chunk = false;
                }
                return Ok(Token {
                    kind: if opening {
                        TokenKind::TemplateHead(cooked)
                    } else {
                        TokenKind::TemplateMiddle(cooked)
                    },
                    start,
                    end: self.index,
                });
            }
            if byte == b'\\' {
                self.index += 1;
                let Some(&escaped) = self.bytes.get(self.index) else {
                    return Err(JsError::syntax(start, "unterminated template escape"));
                };
                self.index += 1;
                match escaped {
                    b'\n' => continue,
                    b'\r' => {
                        if self.bytes.get(self.index) == Some(&b'\n') {
                            self.index += 1;
                        }
                        continue;
                    }
                    b'n' => cooked.push('\n'),
                    b'r' => cooked.push('\r'),
                    b't' => cooked.push('\t'),
                    b'b' => cooked.push('\u{0008}'),
                    b'f' => cooked.push('\u{000c}'),
                    b'v' => cooked.push('\u{000b}'),
                    b'0' => cooked.push('\0'),
                    b'\\' => cooked.push('\\'),
                    b'`' => cooked.push('`'),
                    b'$' => cooked.push('$'),
                    b'x' => cooked.push(self.template_hex_escape(2, start)?),
                    b'u' => {
                        if self.bytes.get(self.index) == Some(&b'{') {
                            self.index += 1;
                            let mut number = 0u32;
                            let mut digits = 0;
                            while self.bytes.get(self.index) != Some(&b'}') {
                                let Some(&next) = self.bytes.get(self.index) else {
                                    return Err(JsError::syntax(
                                        start,
                                        "unterminated unicode escape",
                                    ));
                                };
                                let digit = (next as char).to_digit(16).ok_or_else(|| {
                                    JsError::syntax(start, "invalid unicode escape")
                                })?;
                                number = number.saturating_mul(16).saturating_add(digit);
                                digits += 1;
                                if digits > 6 {
                                    return Err(JsError::syntax(start, "unicode escape too long"));
                                }
                                self.index += 1;
                            }
                            if digits == 0 {
                                return Err(JsError::syntax(start, "empty unicode escape"));
                            }
                            self.index += 1;
                            cooked.push(
                                char::from_u32(number).ok_or_else(|| {
                                    JsError::syntax(start, "invalid unicode scalar")
                                })?,
                            );
                        } else {
                            cooked.push(self.template_hex_escape(4, start)?);
                        }
                    }
                    b'1'..=b'9' => {
                        return Err(JsError::syntax(start, "invalid template escape"));
                    }
                    other if other.is_ascii() => cooked.push(other as char),
                    _ => return Err(JsError::syntax(start, "invalid template escape")),
                }
                continue;
            }
            if byte == b'\r' {
                self.index += 1;
                if self.bytes.get(self.index) == Some(&b'\n') {
                    self.index += 1;
                }
                cooked.push('\n');
                continue;
            }
            let ch = self.source[self.index..]
                .chars()
                .next()
                .expect("valid UTF-8");
            cooked.push(ch);
            self.index += ch.len_utf8();
        }
        Err(JsError::syntax(start, "unterminated template literal"))
    }

    fn template_hex_escape(&mut self, digits: usize, start: usize) -> Result<char, JsError> {
        let mut number = 0u32;
        for _ in 0..digits {
            let Some(&byte) = self.bytes.get(self.index) else {
                return Err(JsError::syntax(start, "unterminated hex escape"));
            };
            let digit = (byte as char)
                .to_digit(16)
                .ok_or_else(|| JsError::syntax(start, "invalid hex escape"))?;
            number = number * 16 + digit;
            self.index += 1;
        }
        char::from_u32(number).ok_or_else(|| JsError::syntax(start, "invalid unicode scalar"))
    }

    fn string(&mut self) -> Result<Token, JsError> {
        let start = self.index;
        let quote = self.bytes[self.index];
        self.index += 1;
        let mut value = String::new();

        while let Some(&byte) = self.bytes.get(self.index) {
            if byte == quote {
                self.index += 1;
                return Ok(Token {
                    kind: TokenKind::String(value),
                    start,
                    end: self.index,
                });
            }
            if matches!(byte, b'\n' | b'\r') {
                return Err(JsError::syntax(start, "unterminated string literal"));
            }
            if byte == b'\\' {
                self.index += 1;
                let Some(&escaped) = self.bytes.get(self.index) else {
                    return Err(JsError::syntax(start, "unterminated string escape"));
                };
                let ch = match escaped {
                    b'n' => '\n',
                    b'r' => '\r',
                    b't' => '\t',
                    b'\\' => '\\',
                    b'\'' => '\'',
                    b'"' => '"',
                    b'0' => '\0',
                    other => other as char,
                };
                value.push(ch);
                self.index += 1;
                continue;
            }

            if byte.is_ascii() {
                value.push(byte as char);
                self.index += 1;
            } else {
                let remainder = &self.source[self.index..];
                let ch = remainder
                    .chars()
                    .next()
                    .ok_or_else(|| JsError::syntax(self.index, "invalid UTF-8 in string"))?;
                value.push(ch);
                self.index += ch.len_utf8();
            }
        }

        Err(JsError::syntax(start, "unterminated string literal"))
    }
}

fn is_identifier_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || matches!(byte, b'_' | b'$')
}

fn is_identifier_continue(byte: u8) -> bool {
    is_identifier_start(byte) || byte.is_ascii_digit()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn template_lexer_preserves_interpolation_braces_and_nested_templates() {
        let tokens =
            tokenize(r#"const x = `outer ${({a: [1, 2]}).a[0]} and ${`inner ${3}`}`;"#).unwrap();
        assert!(
            tokens
                .iter()
                .any(|t| matches!(&t.kind, TokenKind::TemplateHead(s) if s == "outer "))
        );
        assert!(
            tokens
                .iter()
                .any(|t| matches!(&t.kind, TokenKind::TemplateMiddle(s) if s == " and "))
        );
        assert!(
            tokens
                .iter()
                .any(|t| matches!(&t.kind, TokenKind::TemplateHead(s) if s == "inner "))
        );
        assert_eq!(
            tokens
                .iter()
                .filter(|t| matches!(t.kind, TokenKind::TemplateExprEnd))
                .count(),
            3
        );
        assert_eq!(
            tokens
                .iter()
                .filter(|t| matches!(t.kind, TokenKind::TemplateTail(_)))
                .count(),
            2
        );
    }

    #[test]
    fn template_lexer_distinguishes_cooked_literal_text_from_js() {
        let tokens = tokenize(
            r#"`literal // text ${2}`
            `one\n\r\t\``
            `escaped \${notAnExpression}`"#,
        )
        .unwrap();
        assert!(matches!(&tokens[0].kind, TokenKind::TemplateHead(s) if s == "literal // text "));
        assert!(matches!(&tokens[2].kind, TokenKind::TemplateExprEnd));
        assert!(matches!(&tokens[3].kind, TokenKind::TemplateTail(s) if s.is_empty()));
        assert!(
            tokens
                .iter()
                .any(|t| matches!(&t.kind, TokenKind::TemplateNoSub(s) if s == "one\n\r\t`"))
        );
        assert!(tokens.iter().any(
            |t| matches!(&t.kind, TokenKind::TemplateNoSub(s) if s == "escaped ${notAnExpression}")
        ));
    }

    #[test]
    fn rejects_unterminated_template_and_bad_cooked_escapes() {
        for source in [
            r"`unclosed",
            r"`${1",
            r"`x \xQ1`",
            r"`x \uD800`",
            r"`x \u{}`",
            r"`x \8`",
        ] {
            assert!(tokenize(source).is_err(), "{source}");
        }
    }

    #[test]
    fn tokenizes_comments_literals_and_operators() {
        let tokens = tokenize(
            "let answer = 4.2e1 + 'x'; // tail\n if (answer !== 0 && true) { answer = 1 || 2 }",
        )
        .unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Let));
        assert!(matches!(tokens[3].kind, TokenKind::Number(value) if value == 42.0));
        assert!(matches!(tokens[5].kind, TokenKind::String(ref value) if value == "x"));
        assert!(
            tokens
                .iter()
                .any(|token| matches!(token.kind, TokenKind::BangEqualEqual))
        );
        assert!(
            tokens
                .iter()
                .any(|token| matches!(token.kind, TokenKind::If))
        );
        let function_tokens = tokenize("function add(a) { return this; } new add()").unwrap();
        assert!(matches!(function_tokens[0].kind, TokenKind::Function));
        assert!(
            function_tokens
                .iter()
                .any(|token| matches!(token.kind, TokenKind::This))
        );
        assert!(
            function_tokens
                .iter()
                .any(|token| matches!(token.kind, TokenKind::New))
        );
        assert!(
            function_tokens
                .iter()
                .any(|token| matches!(token.kind, TokenKind::Return))
        );
        assert!(
            tokens
                .iter()
                .any(|token| matches!(token.kind, TokenKind::AmpAmp))
        );
        assert!(
            tokens
                .iter()
                .any(|token| matches!(token.kind, TokenKind::PipePipe))
        );
        assert!(
            tokens
                .iter()
                .any(|token| matches!(token.kind, TokenKind::LeftBrace))
        );
    }

    #[test]
    fn rejects_unterminated_comment_and_string() {
        assert!(tokenize("/* nope").is_err());
        assert!(tokenize("'nope").is_err());
    }
}
