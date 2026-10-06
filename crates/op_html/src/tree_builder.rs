use op_dom::{Attribute as DomAttribute, Document, DocumentMode, DocumentTypeData, NodeId};

use crate::{Attribute, Token, Tokenizer, document_mode};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InsertionMode {
    Initial,
    BeforeHtml,
    BeforeHead,
    InHead,
    AfterHead,
    InBody,
    Text,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TokenAction {
    Consumed,
    Reprocess,
    Stop,
}

pub fn parse_document(input: &str) -> Document {
    let mut builder = TreeBuilder::new();

    for token in Tokenizer::new(input).tokenize() {
        if builder.process(token) {
            break;
        }
    }

    builder.finish()
}

struct TreeBuilder {
    document: Document,
    open_elements: Vec<NodeId>,
    mode: InsertionMode,
    html_element: Option<NodeId>,
    head_element: Option<NodeId>,
    body_element: Option<NodeId>,
    original_mode: InsertionMode,
    text_buffer: String,
}

impl TreeBuilder {
    fn new() -> Self {
        Self {
            document: Document::new(),
            open_elements: Vec::new(),
            mode: InsertionMode::Initial,
            html_element: None,
            head_element: None,
            body_element: None,
            original_mode: InsertionMode::InBody,
            text_buffer: String::new(),
        }
    }

    fn finish(mut self) -> Document {
        self.flush_text();
        self.document
    }

    fn process(&mut self, token: Token) -> bool {
        loop {
            let action = match self.mode {
                InsertionMode::Initial => self.handle_initial(&token),
                InsertionMode::BeforeHtml => self.handle_before_html(&token),
                InsertionMode::BeforeHead => self.handle_before_head(&token),
                InsertionMode::InHead => self.handle_in_head(&token),
                InsertionMode::AfterHead => self.handle_after_head(&token),
                InsertionMode::InBody => self.handle_in_body(&token),
                InsertionMode::Text => self.handle_text(&token),
            };

            match action {
                TokenAction::Consumed => return false,
                TokenAction::Reprocess => {}
                TokenAction::Stop => return true,
            }
        }
    }

    fn handle_initial(&mut self, token: &Token) -> TokenAction {
        match token {
            Token::Character(character) if is_ascii_whitespace(*character) => TokenAction::Consumed,
            Token::Comment(data) => {
                let root = self.document.root();
                self.append_comment(root, data.clone());
                TokenAction::Consumed
            }
            Token::Doctype(doctype) => {
                self.document.set_mode(document_mode::classify(doctype));
                self.append_doctype(doctype);
                self.mode = InsertionMode::BeforeHtml;
                TokenAction::Consumed
            }
            _ => {
                self.document.set_mode(DocumentMode::Quirks);
                self.mode = InsertionMode::BeforeHtml;
                TokenAction::Reprocess
            }
        }
    }

    fn handle_before_html(&mut self, token: &Token) -> TokenAction {
        match token {
            Token::Doctype(_) => TokenAction::Consumed,
            Token::Comment(data) => {
                let root = self.document.root();
                self.append_comment(root, data.clone());
                TokenAction::Consumed
            }
            Token::Character(character) if is_ascii_whitespace(*character) => TokenAction::Consumed,
            Token::StartTag {
                name, attributes, ..
            } if name == "html" => {
                let root = self.document.root();
                let html = self.insert_element(name, attributes, Some(root), false);
                self.html_element = Some(html);
                self.mode = InsertionMode::BeforeHead;
                TokenAction::Consumed
            }
            Token::EndTag { name } if !matches!(name.as_str(), "head" | "body" | "html" | "br") => {
                TokenAction::Consumed
            }
            _ => {
                self.ensure_html();
                self.mode = InsertionMode::BeforeHead;
                TokenAction::Reprocess
            }
        }
    }

    fn handle_before_head(&mut self, token: &Token) -> TokenAction {
        match token {
            Token::Character(character) if is_ascii_whitespace(*character) => TokenAction::Consumed,
            Token::Comment(data) => {
                let parent = self.current_parent();
                self.append_comment(parent, data.clone());
                TokenAction::Consumed
            }
            Token::Doctype(_) => TokenAction::Consumed,
            Token::StartTag {
                name, attributes, ..
            } if name == "html" => {
                self.merge_html_attributes(attributes);
                TokenAction::Consumed
            }
            Token::StartTag {
                name, attributes, ..
            } if name == "head" => {
                let head = self.insert_element(name, attributes, None, false);
                self.head_element = Some(head);
                self.mode = InsertionMode::InHead;
                TokenAction::Consumed
            }
            Token::EndTag { name } if !matches!(name.as_str(), "head" | "body" | "html" | "br") => {
                TokenAction::Consumed
            }
            _ => {
                self.ensure_head();
                self.mode = InsertionMode::InHead;
                TokenAction::Reprocess
            }
        }
    }

    fn handle_in_head(&mut self, token: &Token) -> TokenAction {
        match token {
            Token::Character(character) if is_ascii_whitespace(*character) => {
                self.text_buffer.push(*character);
                TokenAction::Consumed
            }
            Token::Comment(data) => {
                self.flush_text();
                let parent = self.current_parent();
                self.append_comment(parent, data.clone());
                TokenAction::Consumed
            }
            Token::Doctype(_) => TokenAction::Consumed,
            Token::StartTag {
                name, attributes, ..
            } if name == "html" => {
                self.flush_text();
                self.merge_html_attributes(attributes);
                TokenAction::Consumed
            }
            Token::StartTag {
                name, attributes, ..
            } if is_head_void_element(name) => {
                self.flush_text();
                self.insert_element(name, attributes, None, true);
                TokenAction::Consumed
            }
            Token::StartTag {
                name, attributes, ..
            } if is_head_text_element(name) => {
                self.flush_text();
                self.insert_element(name, attributes, None, false);
                self.original_mode = InsertionMode::InHead;
                self.mode = InsertionMode::Text;
                TokenAction::Consumed
            }
            Token::EndTag { name } if name == "head" => {
                self.flush_text();
                self.close_matching("head");
                self.mode = InsertionMode::AfterHead;
                TokenAction::Consumed
            }
            Token::EndTag { name } if is_head_text_element(name) => TokenAction::Consumed,
            Token::EndTag { name } if matches!(name.as_str(), "body" | "html" | "br") => {
                self.flush_text();
                self.close_matching("head");
                self.mode = InsertionMode::AfterHead;
                TokenAction::Reprocess
            }
            Token::StartTag { name, .. } if name == "head" => TokenAction::Consumed,
            Token::EndTag { .. } => TokenAction::Consumed,
            _ => {
                self.flush_text();
                self.close_matching("head");
                self.mode = InsertionMode::AfterHead;
                TokenAction::Reprocess
            }
        }
    }

    fn handle_after_head(&mut self, token: &Token) -> TokenAction {
        match token {
            Token::Character(character) if is_ascii_whitespace(*character) => {
                self.text_buffer.push(*character);
                TokenAction::Consumed
            }
            Token::Comment(data) => {
                self.flush_text();
                let parent = self.current_parent();
                self.append_comment(parent, data.clone());
                TokenAction::Consumed
            }
            Token::Doctype(_) => TokenAction::Consumed,
            Token::StartTag {
                name, attributes, ..
            } if name == "html" => {
                self.flush_text();
                self.merge_html_attributes(attributes);
                TokenAction::Consumed
            }
            Token::StartTag {
                name, attributes, ..
            } if name == "body" => {
                self.flush_text();
                let body = self.insert_element(name, attributes, None, false);
                self.body_element = Some(body);
                self.mode = InsertionMode::InBody;
                TokenAction::Consumed
            }
            Token::StartTag {
                name, attributes, ..
            } if is_head_void_element(name) => {
                self.flush_text();
                if let Some(head) = self.head_element {
                    self.insert_element(name, attributes, Some(head), true);
                }
                TokenAction::Consumed
            }
            Token::StartTag {
                name, attributes, ..
            } if is_head_text_element(name) => {
                self.flush_text();
                if let Some(head) = self.head_element {
                    self.insert_element(name, attributes, Some(head), false);
                    self.original_mode = InsertionMode::AfterHead;
                    self.mode = InsertionMode::Text;
                }
                TokenAction::Consumed
            }
            Token::StartTag { name, .. } if name == "head" => TokenAction::Consumed,
            Token::EndTag { name } if matches!(name.as_str(), "body" | "html" | "br") => {
                self.flush_text();
                self.ensure_body();
                self.mode = InsertionMode::InBody;
                TokenAction::Reprocess
            }
            Token::EndTag { .. } => TokenAction::Consumed,
            _ => {
                self.flush_text();
                self.ensure_body();
                self.mode = InsertionMode::InBody;
                TokenAction::Reprocess
            }
        }
    }

    fn handle_text(&mut self, token: &Token) -> TokenAction {
        match token {
            Token::Character(character) => {
                self.text_buffer.push(*character);
                TokenAction::Consumed
            }
            Token::EndTag { name }
                if self
                    .open_elements
                    .last()
                    .and_then(|node| self.document.element(*node))
                    .is_some_and(|element| element.tag_name == *name) =>
            {
                self.flush_text();
                self.close_matching(name);
                self.mode = self.original_mode;
                TokenAction::Consumed
            }
            Token::Eof => {
                self.flush_text();
                self.open_elements.pop();
                TokenAction::Stop
            }
            _ => TokenAction::Consumed,
        }
    }

    fn handle_in_body(&mut self, token: &Token) -> TokenAction {
        match token {
            Token::Character(character) => {
                self.text_buffer.push(*character);
                TokenAction::Consumed
            }
            Token::Comment(data) => {
                self.flush_text();
                let parent = self.current_parent();
                self.append_comment(parent, data.clone());
                TokenAction::Consumed
            }
            Token::Doctype(_) => TokenAction::Consumed,
            Token::StartTag {
                name, attributes, ..
            } if name == "html" => {
                self.flush_text();
                self.merge_html_attributes(attributes);
                TokenAction::Consumed
            }
            Token::StartTag { name, .. } if name == "head" => TokenAction::Consumed,
            Token::StartTag {
                name, attributes, ..
            } if name == "body" => {
                self.flush_text();
                if let Some(body) = self.body_element {
                    self.merge_attributes(body, attributes);
                }
                TokenAction::Consumed
            }
            Token::StartTag {
                name, attributes, ..
            } => {
                self.flush_text();
                self.insert_element(name, attributes, None, false);
                TokenAction::Consumed
            }
            Token::EndTag { name } => {
                self.flush_text();
                self.close_matching(name);
                TokenAction::Consumed
            }
            Token::Eof => {
                self.flush_text();
                TokenAction::Stop
            }
        }
    }

    fn ensure_html(&mut self) -> NodeId {
        if let Some(html) = self.html_element {
            return html;
        }

        let root = self.document.root();
        let html = self.insert_element("html", &[], Some(root), false);
        self.html_element = Some(html);
        html
    }

    fn ensure_head(&mut self) -> NodeId {
        if let Some(head) = self.head_element {
            return head;
        }

        self.ensure_html();
        let head = self.insert_element("head", &[], None, false);
        self.head_element = Some(head);
        head
    }

    fn ensure_body(&mut self) -> NodeId {
        if let Some(body) = self.body_element {
            return body;
        }

        self.ensure_html();
        let body = self.insert_element("body", &[], None, false);
        self.body_element = Some(body);
        body
    }

    fn insert_element(
        &mut self,
        name: &str,
        attributes: &[Attribute],
        parent: Option<NodeId>,
        force_no_push: bool,
    ) -> NodeId {
        let attributes = attributes
            .iter()
            .cloned()
            .map(|attribute| DomAttribute {
                name: attribute.name,
                value: attribute.value,
            })
            .collect();

        let node = self
            .document
            .create_element_with_attributes(name.to_owned(), attributes);
        let parent = parent.unwrap_or_else(|| self.current_parent());
        self.document
            .append_child(parent, node)
            .expect("tree builder created an invalid parent/child relation");

        if !force_no_push && !is_void_element(name) {
            self.open_elements.push(node);
        }

        node
    }

    fn append_comment(&mut self, parent: NodeId, data: String) {
        let node = self.document.create_comment(data);
        self.document
            .append_child(parent, node)
            .expect("tree builder created an invalid comment parent");
    }

    fn append_doctype(&mut self, doctype: &crate::Doctype) {
        let node = self.document.create_document_type(DocumentTypeData {
            name: doctype.name.clone(),
            public_identifier: doctype.public_identifier.clone(),
            system_identifier: doctype.system_identifier.clone(),
            force_quirks: doctype.force_quirks,
        });
        let root = self.document.root();
        self.document
            .append_child(root, node)
            .expect("tree builder created an invalid doctype parent");
    }

    fn current_parent(&self) -> NodeId {
        self.open_elements
            .last()
            .copied()
            .unwrap_or_else(|| self.document.root())
    }

    fn flush_text(&mut self) {
        if self.text_buffer.is_empty() {
            return;
        }

        let text = std::mem::take(&mut self.text_buffer);
        let node = self.document.create_text(text);
        let parent = self.current_parent();
        self.document
            .append_child(parent, node)
            .expect("tree builder created an invalid text parent");
    }

    fn close_matching(&mut self, tag_name: &str) -> Option<NodeId> {
        let position = self.open_elements.iter().rposition(|node_id| {
            self.document
                .element(*node_id)
                .is_some_and(|element| element.tag_name == tag_name)
        })?;
        let closed = self.open_elements[position];
        self.open_elements.truncate(position);
        Some(closed)
    }

    fn merge_html_attributes(&mut self, attributes: &[Attribute]) {
        if let Some(html) = self.html_element {
            self.merge_attributes(html, attributes);
        }
    }

    fn merge_attributes(&mut self, node: NodeId, attributes: &[Attribute]) {
        let Some(element) = self.document.element_mut(node) else {
            return;
        };

        for attribute in attributes {
            if !element
                .attributes
                .iter()
                .any(|existing| existing.name == attribute.name)
            {
                element.attributes.push(DomAttribute {
                    name: attribute.name.clone(),
                    value: attribute.value.clone(),
                });
            }
        }
    }
}

fn is_ascii_whitespace(character: char) -> bool {
    matches!(character, '\t' | '\n' | '\x0c' | '\r' | ' ')
}

fn is_head_void_element(tag_name: &str) -> bool {
    matches!(tag_name, "base" | "basefont" | "bgsound" | "link" | "meta")
}

fn is_head_text_element(tag_name: &str) -> bool {
    matches!(tag_name, "title" | "style" | "script" | "noframes")
}

fn is_void_element(tag_name: &str) -> bool {
    matches!(
        tag_name,
        "area"
            | "base"
            | "br"
            | "col"
            | "embed"
            | "hr"
            | "img"
            | "input"
            | "link"
            | "meta"
            | "param"
            | "source"
            | "track"
            | "wbr"
    )
}
