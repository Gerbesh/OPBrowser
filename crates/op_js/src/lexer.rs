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
    InstanceOf,
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
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    LeftBracket,
    RightBracket,
    Dot,
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
}

impl<'a> Lexer<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            source,
            bytes: source.as_bytes(),
            index: 0,
        }
    }

    fn tokenize(mut self) -> Result<Vec<Token>, JsError> {
        let mut tokens = Vec::new();
        while self.index < self.bytes.len() {
            self.skip_space_and_comments()?;
            if self.index >= self.bytes.len() {
                break;
            }
            tokens.push(self.next_token()?);
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
        let start = self.index;
        let byte = self.bytes[start];

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
            b';' => TokenKind::Semicolon,
            b',' => TokenKind::Comma,
            b'&' if self.take(b'&') => TokenKind::AmpAmp,
            b'|' if self.take(b'|') => TokenKind::PipePipe,
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
            "instanceof" => TokenKind::InstanceOf,
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
