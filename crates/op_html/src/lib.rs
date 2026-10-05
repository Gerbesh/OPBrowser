mod references;
mod tree_builder;
pub use tree_builder::parse_document;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attribute {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    StartTag {
        name: String,
        attributes: Vec<Attribute>,
        self_closing: bool,
    },
    EndTag {
        name: String,
    },
    Character(char),
    Eof,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Data,
    TagOpen,
    EndTagOpen,
    TagName,
    BeforeAttributeName,
    AttributeName,
    AfterAttributeName,
    BeforeAttributeValue,
    AttributeValueDoubleQuoted,
    AttributeValueSingleQuoted,
    AttributeValueUnquoted,
    AfterAttributeValueQuoted,
    SelfClosingStartTag,
}

#[derive(Debug)]
pub struct Tokenizer {
    input: Vec<char>,
    cursor: usize,
    state: State,
    current_tag_name: String,
    current_attributes: Vec<Attribute>,
    current_attribute: Option<Attribute>,
    current_is_end_tag: bool,
    text_mode: Option<(String, bool)>,
}

impl Tokenizer {
    pub fn new(input: &str) -> Self {
        Self {
            input: input.chars().collect(),
            cursor: 0,
            state: State::Data,
            current_tag_name: String::new(),
            current_attributes: Vec::new(),
            current_attribute: None,
            current_is_end_tag: false,
            text_mode: None,
        }
    }

    pub fn tokenize(mut self) -> Vec<Token> {
        let mut output = Vec::new();

        loop {
            // Initial raw-text/RCDATA handling keeps escaped markup as text and
            // avoids decoding references inside script/style source.
            if self.state == State::Data
                && let Some((tag, decode_references)) = &self.text_mode
            {
                let remaining = &self.input[self.cursor..];
                let is_end = remaining.starts_with(&['<', '/'])
                    && remaining.len() >= tag.len() + 2
                    && remaining[2..]
                        .iter()
                        .copied()
                        .zip(tag.chars())
                        .all(|(a, b)| a.eq_ignore_ascii_case(&b))
                    && remaining
                        .get(tag.len() + 2)
                        .is_some_and(|ch| ch.is_ascii_whitespace() || matches!(ch, '/' | '>'));
                if is_end {
                    self.text_mode = None;
                } else {
                    let decode_references = *decode_references;
                    match self.next_char() {
                        Some('&') if decode_references => {
                            output.extend(
                                self.character_reference(false).iter().map(Token::Character),
                            );
                        }
                        Some(ch) => output.push(Token::Character(ch)),
                        None => {
                            output.push(Token::Eof);
                            break;
                        }
                    }
                    continue;
                }
            }
            match self.state {
                State::Data => match self.next_char() {
                    Some('<') => self.state = State::TagOpen,
                    Some('&') => {
                        output.extend(self.character_reference(false).iter().map(Token::Character));
                    }
                    Some(character) => output.push(Token::Character(character)),
                    None => {
                        output.push(Token::Eof);
                        break;
                    }
                },
                State::TagOpen => match self.next_char() {
                    Some('/') => self.state = State::EndTagOpen,
                    Some(character) if character.is_ascii_alphabetic() => {
                        self.begin_tag(false);
                        self.reconsume();
                        self.state = State::TagName;
                    }
                    Some(_) => {
                        output.push(Token::Character('<'));
                        self.reconsume();
                        self.state = State::Data;
                    }
                    None => {
                        output.push(Token::Character('<'));
                        output.push(Token::Eof);
                        break;
                    }
                },
                State::EndTagOpen => match self.next_char() {
                    Some(character) if character.is_ascii_alphabetic() => {
                        self.begin_tag(true);
                        self.reconsume();
                        self.state = State::TagName;
                    }
                    Some('>') => self.state = State::Data,
                    Some(_) => {
                        output.push(Token::Character('<'));
                        output.push(Token::Character('/'));
                        self.reconsume();
                        self.state = State::Data;
                    }
                    None => {
                        output.push(Token::Character('<'));
                        output.push(Token::Character('/'));
                        output.push(Token::Eof);
                        break;
                    }
                },
                State::TagName => match self.next_char() {
                    Some(character) if character.is_ascii_whitespace() => {
                        self.state = State::BeforeAttributeName;
                    }
                    Some('/') if !self.current_is_end_tag => {
                        self.state = State::SelfClosingStartTag;
                    }
                    Some('>') => {
                        self.finish_attribute();
                        output.push(self.emit_current_tag(false));
                        self.state = State::Data;
                    }
                    Some('\0') => self.current_tag_name.push('\u{fffd}'),
                    Some(character) => self.current_tag_name.push(character.to_ascii_lowercase()),
                    None => {
                        output.push(Token::Eof);
                        break;
                    }
                },
                State::BeforeAttributeName => match self.next_char() {
                    Some(character) if character.is_ascii_whitespace() => {}
                    Some('/') if !self.current_is_end_tag => {
                        self.finish_attribute();
                        self.state = State::SelfClosingStartTag;
                    }
                    Some('>') => {
                        self.finish_attribute();
                        output.push(self.emit_current_tag(false));
                        self.state = State::Data;
                    }
                    Some(_) => {
                        self.start_attribute();
                        self.reconsume();
                        self.state = State::AttributeName;
                    }
                    None => {
                        output.push(Token::Eof);
                        break;
                    }
                },
                State::AttributeName => match self.next_char() {
                    Some(character) if character.is_ascii_whitespace() => {
                        self.state = State::AfterAttributeName;
                    }
                    Some('/') if !self.current_is_end_tag => {
                        self.finish_attribute();
                        self.state = State::SelfClosingStartTag;
                    }
                    Some('=') => self.state = State::BeforeAttributeValue,
                    Some('>') => {
                        self.finish_attribute();
                        output.push(self.emit_current_tag(false));
                        self.state = State::Data;
                    }
                    Some('\0') => self.push_attribute_name('\u{fffd}'),
                    Some(character) => self.push_attribute_name(character.to_ascii_lowercase()),
                    None => {
                        output.push(Token::Eof);
                        break;
                    }
                },
                State::AfterAttributeName => match self.next_char() {
                    Some(character) if character.is_ascii_whitespace() => {}
                    Some('/') if !self.current_is_end_tag => {
                        self.finish_attribute();
                        self.state = State::SelfClosingStartTag;
                    }
                    Some('=') => self.state = State::BeforeAttributeValue,
                    Some('>') => {
                        self.finish_attribute();
                        output.push(self.emit_current_tag(false));
                        self.state = State::Data;
                    }
                    Some(_) => {
                        self.finish_attribute();
                        self.start_attribute();
                        self.reconsume();
                        self.state = State::AttributeName;
                    }
                    None => {
                        output.push(Token::Eof);
                        break;
                    }
                },
                State::BeforeAttributeValue => match self.next_char() {
                    Some(character) if character.is_ascii_whitespace() => {}
                    Some('"') => self.state = State::AttributeValueDoubleQuoted,
                    Some('\'') => self.state = State::AttributeValueSingleQuoted,
                    Some('>') => {
                        self.finish_attribute();
                        output.push(self.emit_current_tag(false));
                        self.state = State::Data;
                    }
                    Some(_) => {
                        self.reconsume();
                        self.state = State::AttributeValueUnquoted;
                    }
                    None => {
                        output.push(Token::Eof);
                        break;
                    }
                },
                State::AttributeValueDoubleQuoted => match self.next_char() {
                    Some('"') => self.state = State::AfterAttributeValueQuoted,
                    Some('&') => self.attribute_reference(),
                    Some('\0') => self.push_attribute_value('\u{fffd}'),
                    Some(character) => self.push_attribute_value(character),
                    None => {
                        output.push(Token::Eof);
                        break;
                    }
                },
                State::AttributeValueSingleQuoted => match self.next_char() {
                    Some('\'') => self.state = State::AfterAttributeValueQuoted,
                    Some('&') => self.attribute_reference(),
                    Some('\0') => self.push_attribute_value('\u{fffd}'),
                    Some(character) => self.push_attribute_value(character),
                    None => {
                        output.push(Token::Eof);
                        break;
                    }
                },
                State::AttributeValueUnquoted => match self.next_char() {
                    Some('&') => self.attribute_reference(),
                    Some(character) if character.is_ascii_whitespace() => {
                        self.finish_attribute();
                        self.state = State::BeforeAttributeName;
                    }
                    Some('>') => {
                        self.finish_attribute();
                        output.push(self.emit_current_tag(false));
                        self.state = State::Data;
                    }
                    Some('\0') => self.push_attribute_value('\u{fffd}'),
                    Some(character) => self.push_attribute_value(character),
                    None => {
                        output.push(Token::Eof);
                        break;
                    }
                },
                State::AfterAttributeValueQuoted => match self.next_char() {
                    Some(character) if character.is_ascii_whitespace() => {
                        self.finish_attribute();
                        self.state = State::BeforeAttributeName;
                    }
                    Some('/') if !self.current_is_end_tag => {
                        self.finish_attribute();
                        self.state = State::SelfClosingStartTag;
                    }
                    Some('>') => {
                        self.finish_attribute();
                        output.push(self.emit_current_tag(false));
                        self.state = State::Data;
                    }
                    Some(_) => {
                        self.finish_attribute();
                        self.reconsume();
                        self.state = State::BeforeAttributeName;
                    }
                    None => {
                        output.push(Token::Eof);
                        break;
                    }
                },
                State::SelfClosingStartTag => match self.next_char() {
                    Some('>') => {
                        self.finish_attribute();
                        output.push(self.emit_current_tag(true));
                        self.state = State::Data;
                    }
                    Some(_) => {
                        self.reconsume();
                        self.state = State::BeforeAttributeName;
                    }
                    None => {
                        output.push(Token::Eof);
                        break;
                    }
                },
            }
        }

        output
    }

    fn next_char(&mut self) -> Option<char> {
        let character = self.input.get(self.cursor).copied();
        if character.is_some() {
            self.cursor += 1;
        }
        character
    }

    fn reconsume(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    fn begin_tag(&mut self, is_end_tag: bool) {
        self.current_tag_name.clear();
        self.current_attributes.clear();
        self.current_attribute = None;
        self.current_is_end_tag = is_end_tag;
    }

    fn start_attribute(&mut self) {
        self.current_attribute = Some(Attribute {
            name: String::new(),
            value: String::new(),
        });
    }

    fn push_attribute_name(&mut self, character: char) {
        if let Some(attribute) = &mut self.current_attribute {
            attribute.name.push(character);
        }
    }

    fn push_attribute_value(&mut self, character: char) {
        if let Some(attribute) = &mut self.current_attribute {
            attribute.value.push(character);
        }
    }

    fn character_reference(&mut self, attribute: bool) -> references::Characters {
        if let Some((characters, consumed)) =
            references::consume(&self.input[self.cursor..], attribute)
        {
            self.cursor += consumed;
            characters
        } else {
            references::Characters::single('&')
        }
    }

    fn attribute_reference(&mut self) {
        for character in self.character_reference(true).iter() {
            self.push_attribute_value(character);
        }
    }

    fn finish_attribute(&mut self) {
        if self.current_is_end_tag {
            self.current_attribute = None;
            return;
        }

        if let Some(attribute) = self.current_attribute.take()
            && !self
                .current_attributes
                .iter()
                .any(|existing| existing.name == attribute.name)
        {
            self.current_attributes.push(attribute);
        }
    }

    fn emit_current_tag(&mut self, self_closing: bool) -> Token {
        if self.current_is_end_tag {
            Token::EndTag {
                name: std::mem::take(&mut self.current_tag_name),
            }
        } else {
            let tag = self.current_tag_name.as_str();
            if matches!(
                tag,
                "script"
                    | "style"
                    | "xmp"
                    | "iframe"
                    | "noembed"
                    | "noframes"
                    | "title"
                    | "textarea"
            ) {
                self.text_mode = Some((tag.to_owned(), matches!(tag, "title" | "textarea")));
            }
            Token::StartTag {
                name: std::mem::take(&mut self.current_tag_name),
                attributes: std::mem::take(&mut self.current_attributes),
                self_closing,
            }
        }
    }
}
