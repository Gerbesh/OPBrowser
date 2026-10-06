use crate::{Doctype, Tokenizer};

#[derive(Clone, Copy)]
enum DoctypeState {
    BeforeName,
    Name,
    AfterName,
    BeforeIdentifier(bool),
    QuotedIdentifier(bool, char),
    AfterPublic,
    AfterSystem,
    Bogus,
}

fn whitespace(ch: char) -> bool {
    matches!(ch, '\t' | '\n' | '\x0c' | '\r' | ' ')
}

fn normalized(ch: char) -> char {
    if ch == '\0' { '\u{fffd}' } else { ch }
}

impl Tokenizer {
    pub(super) fn consume_keyword(&mut self, keyword: &str) -> bool {
        let Some(candidate) = self.input.get(self.cursor..self.cursor + keyword.len()) else {
            return false;
        };
        if !candidate
            .iter()
            .zip(keyword.chars())
            .all(|(a, b)| a.eq_ignore_ascii_case(&b))
        {
            return false;
        }
        self.cursor += keyword.len();
        true
    }

    pub(super) fn consume_bogus_comment(&mut self) -> String {
        let mut data = String::new();
        while let Some(ch) = self.next_char() {
            if ch == '>' {
                break;
            }
            data.push(normalized(ch));
        }
        data
    }

    // Equivalent token-data states are shared where only parse-error reporting differs.
    pub(super) fn consume_doctype(&mut self) -> Doctype {
        use DoctypeState::*;
        let mut state = BeforeName;
        let mut doctype = Doctype::default();
        loop {
            let Some(ch) = self.next_char() else {
                if !matches!(state, Bogus) {
                    doctype.force_quirks = true;
                }
                break;
            };
            match state {
                BeforeName => {
                    if whitespace(ch) {
                        continue;
                    }
                    if ch == '>' {
                        doctype.force_quirks = true;
                        break;
                    }
                    doctype.name = Some(normalized(ch).to_ascii_lowercase().to_string());
                    state = Name;
                }
                Name => {
                    if whitespace(ch) {
                        state = AfterName;
                    } else if ch == '>' {
                        break;
                    } else {
                        doctype
                            .name
                            .as_mut()
                            .unwrap()
                            .push(normalized(ch).to_ascii_lowercase());
                    }
                }
                AfterName => {
                    if whitespace(ch) {
                        continue;
                    }
                    if ch == '>' {
                        break;
                    }
                    self.reconsume();
                    if self.consume_keyword("PUBLIC") {
                        state = BeforeIdentifier(true);
                    } else if self.consume_keyword("SYSTEM") {
                        state = BeforeIdentifier(false);
                    } else {
                        doctype.force_quirks = true;
                        state = Bogus;
                    }
                }
                BeforeIdentifier(public) => {
                    if whitespace(ch) {
                        continue;
                    }
                    if matches!(ch, '\'' | '"') {
                        let identifier = if public {
                            &mut doctype.public_identifier
                        } else {
                            &mut doctype.system_identifier
                        };
                        *identifier = Some(String::new());
                        state = QuotedIdentifier(public, ch);
                    } else {
                        doctype.force_quirks = true;
                        if ch == '>' {
                            break;
                        }
                        state = Bogus;
                    }
                }
                QuotedIdentifier(public, quote) => {
                    if ch == quote {
                        state = if public { AfterPublic } else { AfterSystem };
                    } else if ch == '>' {
                        doctype.force_quirks = true;
                        break;
                    } else {
                        let identifier = if public {
                            &mut doctype.public_identifier
                        } else {
                            &mut doctype.system_identifier
                        };
                        identifier.as_mut().unwrap().push(normalized(ch));
                    }
                }
                AfterPublic => {
                    if whitespace(ch) {
                        continue;
                    }
                    if ch == '>' {
                        break;
                    }
                    self.reconsume();
                    state = BeforeIdentifier(false);
                }
                AfterSystem => {
                    if whitespace(ch) {
                        continue;
                    }
                    if ch == '>' {
                        break;
                    }
                    // Junk after a complete system identifier does not force quirks.
                    state = Bogus;
                }
                Bogus => {
                    if ch == '>' {
                        break;
                    }
                }
            }
        }
        doctype
    }
}
