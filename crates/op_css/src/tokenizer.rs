use crate::{CssError, Token, TokenKind, TokenizeResult};

pub fn tokenize(input: &str) -> TokenizeResult {
    Tokenizer::new(input).run()
}

struct Tokenizer<'a> {
    input: &'a str,
    pos: usize,
    tokens: Vec<Token>,
    errors: Vec<CssError>,
}

impl<'a> Tokenizer<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input,
            pos: 0,
            tokens: Vec::new(),
            errors: Vec::new(),
        }
    }

    fn run(mut self) -> TokenizeResult {
        while self.pos < self.input.len() {
            if self.starts_with("/*") {
                self.consume_comment();
                continue;
            }

            let start = self.pos;
            let Some(ch) = self.peek_char() else {
                break;
            };

            if is_css_whitespace(ch) {
                self.consume_whitespace();
                self.push(TokenKind::Whitespace, start);
                continue;
            }

            match ch {
                '"' | '\'' => {
                    self.advance_char();
                    let kind = self.consume_string(ch, start);
                    self.push(kind, start);
                }
                '#' if self.would_start_ident_at(self.pos + 1) => {
                    self.advance_char();
                    let name = self.consume_name();
                    self.push(TokenKind::Hash(name), start);
                }
                '@' if self.would_start_ident_at(self.pos + 1) => {
                    self.advance_char();
                    let name = self.consume_name();
                    self.push(TokenKind::AtKeyword(name), start);
                }
                ':' => {
                    self.advance_char();
                    self.push(TokenKind::Colon, start);
                }
                ';' => {
                    self.advance_char();
                    self.push(TokenKind::Semicolon, start);
                }
                ',' => {
                    self.advance_char();
                    self.push(TokenKind::Comma, start);
                }
                '{' => {
                    self.advance_char();
                    self.push(TokenKind::OpenCurly, start);
                }
                '}' => {
                    self.advance_char();
                    self.push(TokenKind::CloseCurly, start);
                }
                '(' => {
                    self.advance_char();
                    self.push(TokenKind::OpenParen, start);
                }
                ')' => {
                    self.advance_char();
                    self.push(TokenKind::CloseParen, start);
                }
                '[' => {
                    self.advance_char();
                    self.push(TokenKind::OpenSquare, start);
                }
                ']' => {
                    self.advance_char();
                    self.push(TokenKind::CloseSquare, start);
                }
                _ if self.would_start_number() => {
                    let number = self.consume_number();
                    if self.would_start_ident_at(self.pos) {
                        let unit = self.consume_name();
                        self.push(TokenKind::Dimension { number, unit }, start);
                    } else if self.peek_char() == Some('%') {
                        self.advance_char();
                        self.push(TokenKind::Percentage(number), start);
                    } else {
                        self.push(TokenKind::Number(number), start);
                    }
                }
                _ if self.would_start_ident_at(self.pos) => {
                    let name = self.consume_name();
                    if self.peek_char() == Some('(') {
                        self.advance_char();
                        self.push(TokenKind::Function(name), start);
                    } else {
                        self.push(TokenKind::Ident(name), start);
                    }
                }
                _ => {
                    self.advance_char();
                    self.push(TokenKind::Delim(ch), start);
                }
            }
        }

        TokenizeResult {
            tokens: self.tokens,
            errors: self.errors,
        }
    }

    fn push(&mut self, kind: TokenKind, start: usize) {
        self.tokens.push(Token {
            kind,
            start,
            end: self.pos,
        });
    }

    fn consume_comment(&mut self) {
        let start = self.pos;
        self.pos += 2;
        if let Some(end) = self.input[self.pos..].find("*/") {
            self.pos += end + 2;
        } else {
            self.pos = self.input.len();
            self.errors.push(CssError {
                offset: start,
                message: "unterminated CSS comment".into(),
            });
        }
    }

    fn consume_whitespace(&mut self) {
        while self.peek_char().is_some_and(is_css_whitespace) {
            self.advance_char();
        }
    }

    fn consume_string(&mut self, quote: char, start: usize) -> TokenKind {
        let mut value = String::new();
        while let Some(ch) = self.peek_char() {
            if ch == quote {
                self.advance_char();
                return TokenKind::String(value);
            }
            if ch == '\n' || ch == '\r' || ch == '\u{000c}' {
                self.errors.push(CssError {
                    offset: start,
                    message: "newline in CSS string".into(),
                });
                return TokenKind::BadString;
            }
            if ch == '\\' {
                self.advance_char();
                if self.peek_char().is_none() {
                    break;
                }
                if matches!(self.peek_char(), Some('\n' | '\r' | '\u{000c}')) {
                    self.consume_escaped_newline();
                } else {
                    value.push(self.consume_escape());
                }
                continue;
            }
            value.push(ch);
            self.advance_char();
        }

        self.errors.push(CssError {
            offset: start,
            message: "unterminated CSS string".into(),
        });
        TokenKind::BadString
    }

    fn consume_escaped_newline(&mut self) {
        if self.starts_with("\r\n") {
            self.pos += 2;
        } else {
            self.advance_char();
        }
    }

    fn consume_name(&mut self) -> String {
        let mut value = String::new();
        while let Some(ch) = self.peek_char() {
            if is_name_char(ch) {
                value.push(ch);
                self.advance_char();
            } else if ch == '\\' && self.valid_escape_at(self.pos) {
                self.advance_char();
                value.push(self.consume_escape());
            } else {
                break;
            }
        }
        value
    }

    fn consume_escape(&mut self) -> char {
        let Some(ch) = self.peek_char() else {
            return '\u{fffd}';
        };
        if ch.is_ascii_hexdigit() {
            let mut value = 0_u32;
            let mut count = 0;
            while count < 6 {
                let Some(next) = self.peek_char() else {
                    break;
                };
                let Some(digit) = next.to_digit(16) else {
                    break;
                };
                value = value.saturating_mul(16).saturating_add(digit);
                self.advance_char();
                count += 1;
            }
            if self.peek_char().is_some_and(is_css_whitespace) {
                self.consume_one_whitespace();
            }
            return char::from_u32(value)
                .filter(|value| *value != '\0')
                .unwrap_or('\u{fffd}');
        }
        self.advance_char();
        if ch == '\0' { '\u{fffd}' } else { ch }
    }

    fn consume_one_whitespace(&mut self) {
        if self.starts_with("\r\n") {
            self.pos += 2;
        } else {
            self.advance_char();
        }
    }

    fn consume_number(&mut self) -> String {
        let start = self.pos;
        if matches!(self.peek_char(), Some('+' | '-')) {
            self.advance_char();
        }
        while self.peek_char().is_some_and(|ch| ch.is_ascii_digit()) {
            self.advance_char();
        }
        if self.peek_char() == Some('.')
            && self
                .char_at(self.pos + 1)
                .is_some_and(|ch| ch.is_ascii_digit())
        {
            self.advance_char();
            while self.peek_char().is_some_and(|ch| ch.is_ascii_digit()) {
                self.advance_char();
            }
        }
        if matches!(self.peek_char(), Some('e' | 'E')) {
            let exponent_start = self.pos;
            self.advance_char();
            if matches!(self.peek_char(), Some('+' | '-')) {
                self.advance_char();
            }
            let digits = self.pos;
            while self.peek_char().is_some_and(|ch| ch.is_ascii_digit()) {
                self.advance_char();
            }
            if self.pos == digits {
                self.pos = exponent_start;
            }
        }
        self.input[start..self.pos].to_owned()
    }

    fn would_start_number(&self) -> bool {
        let first_pos = self.pos;
        let second_pos = self.next_offset(first_pos);
        let third_pos = self.next_offset(second_pos);
        let first = self.char_at(first_pos);
        let second = self.char_at(second_pos);
        let third = self.char_at(third_pos);
        match first {
            Some('+' | '-') => {
                second.is_some_and(|ch| ch.is_ascii_digit())
                    || (second == Some('.') && third.is_some_and(|ch| ch.is_ascii_digit()))
            }
            Some('.') => second.is_some_and(|ch| ch.is_ascii_digit()),
            Some(ch) => ch.is_ascii_digit(),
            None => false,
        }
    }

    fn would_start_ident_at(&self, pos: usize) -> bool {
        let first = self.char_at(pos);
        let second_pos = self.next_offset(pos);
        let second = self.char_at(second_pos);
        match first {
            Some('-') => {
                second.is_some_and(is_name_start)
                    || second == Some('-')
                    || (second == Some('\\') && self.valid_escape_at(second_pos))
            }
            Some(ch) if is_name_start(ch) => true,
            Some('\\') => self.valid_escape_at(pos),
            _ => false,
        }
    }

    fn valid_escape_at(&self, pos: usize) -> bool {
        if self.char_at(pos) != Some('\\') {
            return false;
        }
        !matches!(
            self.char_at(self.next_offset(pos)),
            None | Some('\n' | '\r' | '\u{000c}')
        )
    }

    fn starts_with(&self, value: &str) -> bool {
        self.input[self.pos..].starts_with(value)
    }

    fn peek_char(&self) -> Option<char> {
        self.char_at(self.pos)
    }

    fn char_at(&self, pos: usize) -> Option<char> {
        self.input.get(pos..)?.chars().next()
    }

    fn next_offset(&self, pos: usize) -> usize {
        pos + self.char_at(pos).map_or(0, char::len_utf8)
    }

    fn advance_char(&mut self) {
        self.pos = self.next_offset(self.pos);
    }
}

fn is_css_whitespace(ch: char) -> bool {
    matches!(ch, ' ' | '\t' | '\n' | '\r' | '\u{000c}')
}

fn is_name_start(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphabetic() || !ch.is_ascii()
}

fn is_name_char(ch: char) -> bool {
    is_name_start(ch) || ch.is_ascii_digit() || ch == '-'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenizes_comments_dimensions_percentages_and_functions() {
        let result = tokenize("/*x*/ .card { width: 12.5px; opacity: 50%; color: rgb(1, 2, 3) }");
        assert!(result.errors.is_empty());
        assert!(result.tokens.iter().any(|token| {
            token.kind
                == TokenKind::Dimension {
                    number: "12.5".into(),
                    unit: "px".into(),
                }
        }));
        assert!(
            result
                .tokens
                .iter()
                .any(|token| token.kind == TokenKind::Percentage("50".into()))
        );
        assert!(
            result
                .tokens
                .iter()
                .any(|token| token.kind == TokenKind::Function("rgb".into()))
        );
    }

    #[test]
    fn decodes_identifier_and_string_escapes() {
        let result = tokenize(".caf\\e9 { content: \"A\\26 B\" }");
        assert!(result.errors.is_empty());
        assert!(
            result
                .tokens
                .iter()
                .any(|token| token.kind == TokenKind::Ident("café".into()))
        );
        assert!(
            result
                .tokens
                .iter()
                .any(|token| token.kind == TokenKind::String("A&B".into()))
        );
    }

    #[test]
    fn reports_unterminated_comment_and_string_without_panicking() {
        let comment = tokenize("a{/*");
        assert_eq!(comment.errors.len(), 1);
        let string = tokenize("a{content:\"oops\ncolor:red}");
        assert_eq!(string.errors.len(), 1);
        assert!(
            string
                .tokens
                .iter()
                .any(|token| token.kind == TokenKind::BadString)
        );
    }
}
