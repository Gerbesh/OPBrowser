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
    InTable,
    InTableText,
    InCaption,
    InColumnGroup,
    InTableBody,
    InRow,
    InCell,
    AfterBody,
    AfterAfterBody,
    Text,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TokenAction {
    Consumed,
    Reprocess,
    Stop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScopeKind {
    Normal,
    ListItem,
    Button,
    Table,
}

#[derive(Debug, Clone)]
struct FormattingElement {
    node: NodeId,
    tag_name: String,
    attributes: Vec<Attribute>,
}

#[derive(Debug, Clone)]
enum ActiveFormattingEntry {
    Marker,
    Element(FormattingElement),
}

pub fn parse_document(input: &str) -> Document {
    parse_document_with_script_hook(input, |_, _| {})
}

/// Pause tree construction after closing a script element. The hook sees
/// only nodes inserted so far and may apply DOM mutations before parsing
/// continues. Tokenization is still eager; document.write is unsupported.
pub fn parse_document_with_script_hook(
    input: &str,
    mut on_script: impl FnMut(&mut Document, NodeId),
) -> Document {
    let mut builder = TreeBuilder::new();

    for token in Tokenizer::new(input).tokenize() {
        let script = if builder.mode == InsertionMode::Text
            && matches!(&token, Token::EndTag { name } if name == "script")
        {
            builder.open_elements.last().copied().filter(|node| {
                builder
                    .document
                    .element(*node)
                    .is_some_and(|element| element.tag_name == "script")
            })
        } else {
            None
        };
        let stopped = builder.process(token);
        if let Some(script) = script {
            on_script(&mut builder.document, script);
        }
        if stopped {
            break;
        }
    }

    builder.finish()
}

struct TreeBuilder {
    document: Document,
    open_elements: Vec<NodeId>,
    active_formatting: Vec<ActiveFormattingEntry>,
    mode: InsertionMode,
    html_element: Option<NodeId>,
    head_element: Option<NodeId>,
    body_element: Option<NodeId>,
    original_mode: InsertionMode,
    text_buffer: String,
    pending_table_text: String,
    foster_parenting: bool,
}

impl TreeBuilder {
    fn new() -> Self {
        Self {
            document: Document::new(),
            open_elements: Vec::new(),
            active_formatting: Vec::new(),
            mode: InsertionMode::Initial,
            html_element: None,
            head_element: None,
            body_element: None,
            original_mode: InsertionMode::InBody,
            text_buffer: String::new(),
            pending_table_text: String::new(),
            foster_parenting: false,
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
                InsertionMode::InTable => self.handle_in_table(&token),
                InsertionMode::InTableText => self.handle_in_table_text(&token),
                InsertionMode::InCaption => self.handle_in_caption(&token),
                InsertionMode::InColumnGroup => self.handle_in_column_group(&token),
                InsertionMode::InTableBody => self.handle_in_table_body(&token),
                InsertionMode::InRow => self.handle_in_row(&token),
                InsertionMode::InCell => self.handle_in_cell(&token),
                InsertionMode::AfterBody => self.handle_after_body(&token),
                InsertionMode::AfterAfterBody => self.handle_after_after_body(&token),
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
            Token::Character('\0') => TokenAction::Consumed,
            Token::Character(character) => {
                self.reconstruct_active_formatting();
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
            } if is_head_token(name) => {
                self.flush_text();
                self.process_head_token(name, attributes, InsertionMode::InBody);
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
            } if name == "table" => {
                self.flush_text();
                self.close_p_if_in_button_scope();
                self.insert_element(name, attributes, None, false);
                self.mode = InsertionMode::InTable;
                TokenAction::Consumed
            }
            Token::StartTag { name, .. }
                if matches!(
                    name.as_str(),
                    "caption"
                        | "col"
                        | "colgroup"
                        | "tbody"
                        | "td"
                        | "tfoot"
                        | "th"
                        | "thead"
                        | "tr"
                ) =>
            {
                TokenAction::Consumed
            }
            Token::StartTag {
                name, attributes, ..
            } if is_p_closing_block_start(name) => {
                self.flush_text();
                self.close_p_if_in_button_scope();
                self.insert_element(name, attributes, None, false);
                TokenAction::Consumed
            }
            Token::StartTag {
                name, attributes, ..
            } if is_heading(name) => {
                self.flush_text();
                self.close_p_if_in_button_scope();
                if self
                    .open_elements
                    .last()
                    .and_then(|node| self.document.element(*node))
                    .is_some_and(|element| is_heading(&element.tag_name))
                {
                    self.open_elements.pop();
                }
                self.insert_element(name, attributes, None, false);
                TokenAction::Consumed
            }
            Token::StartTag {
                name, attributes, ..
            } if name == "li" => {
                self.flush_text();
                self.close_previous_list_item();
                self.close_p_if_in_button_scope();
                self.insert_element(name, attributes, None, false);
                TokenAction::Consumed
            }
            Token::StartTag {
                name, attributes, ..
            } if matches!(name.as_str(), "dd" | "dt") => {
                self.flush_text();
                self.close_previous_description_item();
                self.close_p_if_in_button_scope();
                self.insert_element(name, attributes, None, false);
                TokenAction::Consumed
            }
            Token::StartTag {
                name, attributes, ..
            } if name == "button" => {
                self.flush_text();
                if self.has_in_scope("button", ScopeKind::Normal) {
                    self.generate_implied_end_tags(None);
                    self.pop_until("button");
                }
                self.reconstruct_active_formatting();
                self.insert_element(name, attributes, None, false);
                TokenAction::Consumed
            }
            Token::StartTag {
                name, attributes, ..
            } if name == "a" => {
                self.flush_text();
                if let Some(existing) = self.last_active_formatting_node("a") {
                    self.adoption_agency("a");
                    self.remove_active_formatting_node(existing);
                    self.remove_open_element(existing);
                }
                self.reconstruct_active_formatting();
                let node = self.insert_element(name, attributes, None, false);
                self.push_active_formatting(node, name, attributes);
                TokenAction::Consumed
            }
            Token::StartTag {
                name, attributes, ..
            } if is_formatting_element(name) && name != "nobr" => {
                self.flush_text();
                self.reconstruct_active_formatting();
                let node = self.insert_element(name, attributes, None, false);
                self.push_active_formatting(node, name, attributes);
                TokenAction::Consumed
            }
            Token::StartTag {
                name, attributes, ..
            } if name == "nobr" => {
                self.flush_text();
                self.reconstruct_active_formatting();
                if self.has_in_scope("nobr", ScopeKind::Normal) {
                    self.adoption_agency("nobr");
                    self.reconstruct_active_formatting();
                }
                let node = self.insert_element(name, attributes, None, false);
                self.push_active_formatting(node, name, attributes);
                TokenAction::Consumed
            }
            Token::StartTag {
                name, attributes, ..
            } if is_formatting_marker_container(name) => {
                self.flush_text();
                self.reconstruct_active_formatting();
                self.insert_element(name, attributes, None, false);
                self.active_formatting.push(ActiveFormattingEntry::Marker);
                TokenAction::Consumed
            }
            Token::StartTag {
                name, attributes, ..
            } if name == "image" => {
                self.flush_text();
                self.reconstruct_active_formatting();
                self.insert_element("img", attributes, None, true);
                TokenAction::Consumed
            }
            Token::StartTag {
                name, attributes, ..
            } => {
                self.flush_text();
                self.reconstruct_active_formatting();
                self.insert_element(name, attributes, None, false);
                TokenAction::Consumed
            }
            Token::EndTag { name } if name == "body" => {
                self.flush_text();
                if self.has_in_scope("body", ScopeKind::Normal) {
                    self.mode = InsertionMode::AfterBody;
                }
                TokenAction::Consumed
            }
            Token::EndTag { name } if name == "html" => {
                self.flush_text();
                if self.has_in_scope("body", ScopeKind::Normal) {
                    self.mode = InsertionMode::AfterBody;
                    TokenAction::Reprocess
                } else {
                    TokenAction::Consumed
                }
            }
            Token::EndTag { name } if is_formatting_element(name) => {
                self.flush_text();
                self.adoption_agency(name);
                TokenAction::Consumed
            }
            Token::EndTag { name } if is_formatting_marker_container(name) => {
                self.flush_text();
                if self.has_in_scope(name, ScopeKind::Normal) {
                    self.generate_implied_end_tags(None);
                    self.pop_until(name);
                    self.clear_active_formatting_to_marker();
                }
                TokenAction::Consumed
            }
            Token::EndTag { name } if is_scoped_block_end(name) => {
                self.flush_text();
                if self.has_in_scope(name, ScopeKind::Normal) {
                    self.generate_implied_end_tags(None);
                    self.pop_until(name);
                }
                TokenAction::Consumed
            }
            Token::EndTag { name } if name == "p" => {
                self.flush_text();
                if !self.has_in_scope("p", ScopeKind::Button) {
                    self.insert_element("p", &[], None, false);
                }
                self.close_p();
                TokenAction::Consumed
            }
            Token::EndTag { name } if name == "li" => {
                self.flush_text();
                if self.has_in_scope("li", ScopeKind::ListItem) {
                    self.generate_implied_end_tags(Some("li"));
                    self.pop_until("li");
                }
                TokenAction::Consumed
            }
            Token::EndTag { name } if matches!(name.as_str(), "dd" | "dt") => {
                self.flush_text();
                if self.has_in_scope(name, ScopeKind::Normal) {
                    self.generate_implied_end_tags(Some(name));
                    self.pop_until(name);
                }
                TokenAction::Consumed
            }
            Token::EndTag { name } if is_heading(name) => {
                self.flush_text();
                if self.has_heading_in_scope() {
                    self.generate_implied_end_tags(None);
                    self.pop_until_heading();
                }
                TokenAction::Consumed
            }
            Token::EndTag { name } if name == "br" => {
                self.flush_text();
                self.reconstruct_active_formatting();
                self.insert_element("br", &[], None, true);
                TokenAction::Consumed
            }
            Token::EndTag { name } => {
                self.flush_text();
                self.close_generic_end_tag(name);
                TokenAction::Consumed
            }
            Token::Eof => {
                self.flush_text();
                TokenAction::Stop
            }
        }
    }

    fn handle_in_table(&mut self, token: &Token) -> TokenAction {
        if !matches!(token, Token::Character(_)) {
            self.flush_text();
        }

        match token {
            Token::Character(_) if self.current_tag_name().is_some_and(is_table_text_context) => {
                self.pending_table_text.clear();
                self.original_mode = self.mode;
                self.mode = InsertionMode::InTableText;
                TokenAction::Reprocess
            }
            Token::Character(_) => self.process_in_body_with_foster_parenting(token),
            Token::Comment(data) => {
                let parent = self.current_parent();
                self.append_comment(parent, data.clone());
                TokenAction::Consumed
            }
            Token::Doctype(_) => TokenAction::Consumed,
            Token::StartTag {
                name, attributes, ..
            } if name == "caption" => {
                self.clear_stack_back_to_table_context();
                self.active_formatting.push(ActiveFormattingEntry::Marker);
                self.insert_element(name, attributes, None, false);
                self.mode = InsertionMode::InCaption;
                TokenAction::Consumed
            }
            Token::StartTag {
                name, attributes, ..
            } if name == "colgroup" => {
                self.clear_stack_back_to_table_context();
                self.insert_element(name, attributes, None, false);
                self.mode = InsertionMode::InColumnGroup;
                TokenAction::Consumed
            }
            Token::StartTag { name, .. } if name == "col" => {
                self.clear_stack_back_to_table_context();
                self.insert_element("colgroup", &[], None, false);
                self.mode = InsertionMode::InColumnGroup;
                TokenAction::Reprocess
            }
            Token::StartTag {
                name, attributes, ..
            } if matches!(name.as_str(), "tbody" | "tfoot" | "thead") => {
                self.clear_stack_back_to_table_context();
                self.insert_element(name, attributes, None, false);
                self.mode = InsertionMode::InTableBody;
                TokenAction::Consumed
            }
            Token::StartTag { name, .. } if matches!(name.as_str(), "td" | "th" | "tr") => {
                self.clear_stack_back_to_table_context();
                self.insert_element("tbody", &[], None, false);
                self.mode = InsertionMode::InTableBody;
                TokenAction::Reprocess
            }
            Token::StartTag { name, .. } if name == "table" => {
                if self.has_in_scope("table", ScopeKind::Table) {
                    self.pop_until("table");
                    self.reset_insertion_mode_appropriately();
                    TokenAction::Reprocess
                } else {
                    TokenAction::Consumed
                }
            }
            Token::EndTag { name } if name == "table" => {
                if self.has_in_scope("table", ScopeKind::Table) {
                    self.pop_until("table");
                    self.reset_insertion_mode_appropriately();
                }
                TokenAction::Consumed
            }
            Token::EndTag { name }
                if matches!(
                    name.as_str(),
                    "body"
                        | "caption"
                        | "col"
                        | "colgroup"
                        | "html"
                        | "tbody"
                        | "td"
                        | "tfoot"
                        | "th"
                        | "thead"
                        | "tr"
                ) =>
            {
                TokenAction::Consumed
            }
            Token::StartTag {
                name, attributes, ..
            } if is_head_token(name) => {
                let return_mode = self.mode;
                self.process_head_token(name, attributes, return_mode);
                TokenAction::Consumed
            }
            Token::Eof => self.handle_in_body(token),
            _ => self.process_in_body_with_foster_parenting(token),
        }
    }

    fn handle_in_table_text(&mut self, token: &Token) -> TokenAction {
        match token {
            Token::Character('\0') => TokenAction::Consumed,
            Token::Character(character) => {
                self.pending_table_text.push(*character);
                TokenAction::Consumed
            }
            _ => {
                self.flush_pending_table_text();
                self.mode = self.original_mode;
                TokenAction::Reprocess
            }
        }
    }

    fn handle_in_caption(&mut self, token: &Token) -> TokenAction {
        match token {
            Token::EndTag { name } if name == "caption" => {
                self.flush_text();
                if self.has_in_scope("caption", ScopeKind::Table) {
                    self.generate_implied_end_tags(None);
                    self.pop_until("caption");
                    self.clear_active_formatting_to_marker();
                    self.mode = InsertionMode::InTable;
                }
                TokenAction::Consumed
            }
            Token::StartTag { name, .. }
                if matches!(
                    name.as_str(),
                    "caption"
                        | "col"
                        | "colgroup"
                        | "tbody"
                        | "td"
                        | "tfoot"
                        | "th"
                        | "thead"
                        | "tr"
                ) =>
            {
                if self.close_caption_if_in_scope() {
                    TokenAction::Reprocess
                } else {
                    TokenAction::Consumed
                }
            }
            Token::EndTag { name } if name == "table" => {
                if self.close_caption_if_in_scope() {
                    TokenAction::Reprocess
                } else {
                    TokenAction::Consumed
                }
            }
            Token::EndTag { name }
                if matches!(
                    name.as_str(),
                    "body"
                        | "col"
                        | "colgroup"
                        | "html"
                        | "tbody"
                        | "td"
                        | "tfoot"
                        | "th"
                        | "thead"
                        | "tr"
                ) =>
            {
                TokenAction::Consumed
            }
            _ => self.handle_in_body(token),
        }
    }

    fn handle_in_column_group(&mut self, token: &Token) -> TokenAction {
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
            Token::StartTag { name, .. } if name == "html" => self.handle_in_body(token),
            Token::StartTag {
                name, attributes, ..
            } if name == "col" => {
                self.flush_text();
                self.insert_element(name, attributes, None, true);
                TokenAction::Consumed
            }
            Token::EndTag { name } if name == "colgroup" => {
                self.flush_text();
                if self.current_tag_name() == Some("colgroup") {
                    self.open_elements.pop();
                    self.mode = InsertionMode::InTable;
                }
                TokenAction::Consumed
            }
            Token::EndTag { name } if name == "col" => TokenAction::Consumed,
            Token::Eof => self.handle_in_body(token),
            _ => {
                self.flush_text();
                if self.current_tag_name() == Some("colgroup") {
                    self.open_elements.pop();
                    self.mode = InsertionMode::InTable;
                    TokenAction::Reprocess
                } else {
                    TokenAction::Consumed
                }
            }
        }
    }

    fn handle_in_table_body(&mut self, token: &Token) -> TokenAction {
        match token {
            Token::StartTag {
                name, attributes, ..
            } if name == "tr" => {
                self.flush_text();
                self.clear_stack_back_to_table_body_context();
                self.insert_element(name, attributes, None, false);
                self.mode = InsertionMode::InRow;
                TokenAction::Consumed
            }
            Token::StartTag { name, .. } if matches!(name.as_str(), "td" | "th") => {
                self.flush_text();
                self.clear_stack_back_to_table_body_context();
                self.insert_element("tr", &[], None, false);
                self.mode = InsertionMode::InRow;
                TokenAction::Reprocess
            }
            Token::EndTag { name } if matches!(name.as_str(), "tbody" | "tfoot" | "thead") => {
                self.flush_text();
                if self.has_in_scope(name, ScopeKind::Table) {
                    self.clear_stack_back_to_table_body_context();
                    self.open_elements.pop();
                    self.mode = InsertionMode::InTable;
                }
                TokenAction::Consumed
            }
            Token::StartTag { name, .. }
                if matches!(
                    name.as_str(),
                    "caption" | "col" | "colgroup" | "tbody" | "tfoot" | "thead"
                ) =>
            {
                if self.close_table_body_if_in_scope() {
                    TokenAction::Reprocess
                } else {
                    TokenAction::Consumed
                }
            }
            Token::EndTag { name } if name == "table" => {
                if self.close_table_body_if_in_scope() {
                    TokenAction::Reprocess
                } else {
                    TokenAction::Consumed
                }
            }
            Token::EndTag { name }
                if matches!(
                    name.as_str(),
                    "body" | "caption" | "col" | "colgroup" | "html" | "td" | "th" | "tr"
                ) =>
            {
                TokenAction::Consumed
            }
            _ => self.handle_in_table(token),
        }
    }

    fn handle_in_row(&mut self, token: &Token) -> TokenAction {
        match token {
            Token::StartTag {
                name, attributes, ..
            } if matches!(name.as_str(), "td" | "th") => {
                self.flush_text();
                self.clear_stack_back_to_table_row_context();
                self.insert_element(name, attributes, None, false);
                self.mode = InsertionMode::InCell;
                self.active_formatting.push(ActiveFormattingEntry::Marker);
                TokenAction::Consumed
            }
            Token::EndTag { name } if name == "tr" => {
                self.flush_text();
                if self.has_in_scope("tr", ScopeKind::Table) {
                    self.clear_stack_back_to_table_row_context();
                    self.open_elements.pop();
                    self.mode = InsertionMode::InTableBody;
                }
                TokenAction::Consumed
            }
            Token::StartTag { name, .. }
                if matches!(
                    name.as_str(),
                    "caption" | "col" | "colgroup" | "tbody" | "tfoot" | "thead" | "tr"
                ) =>
            {
                if self.close_row_if_in_scope() {
                    TokenAction::Reprocess
                } else {
                    TokenAction::Consumed
                }
            }
            Token::EndTag { name } if name == "table" => {
                if self.close_row_if_in_scope() {
                    TokenAction::Reprocess
                } else {
                    TokenAction::Consumed
                }
            }
            Token::EndTag { name } if matches!(name.as_str(), "tbody" | "tfoot" | "thead") => {
                if self.has_in_scope(name, ScopeKind::Table) && self.close_row_if_in_scope() {
                    TokenAction::Reprocess
                } else {
                    TokenAction::Consumed
                }
            }
            Token::EndTag { name }
                if matches!(
                    name.as_str(),
                    "body" | "caption" | "col" | "colgroup" | "html" | "td" | "th"
                ) =>
            {
                TokenAction::Consumed
            }
            _ => self.handle_in_table(token),
        }
    }

    fn handle_in_cell(&mut self, token: &Token) -> TokenAction {
        match token {
            Token::EndTag { name } if matches!(name.as_str(), "td" | "th") => {
                self.flush_text();
                if self.has_in_scope(name, ScopeKind::Table) {
                    self.generate_implied_end_tags(None);
                    self.pop_until(name);
                    self.clear_active_formatting_to_marker();
                    self.mode = InsertionMode::InRow;
                }
                TokenAction::Consumed
            }
            Token::StartTag { name, .. }
                if matches!(
                    name.as_str(),
                    "caption"
                        | "col"
                        | "colgroup"
                        | "tbody"
                        | "td"
                        | "tfoot"
                        | "th"
                        | "thead"
                        | "tr"
                ) =>
            {
                if self.close_cell_if_in_scope() {
                    TokenAction::Reprocess
                } else {
                    TokenAction::Consumed
                }
            }
            Token::EndTag { name }
                if matches!(name.as_str(), "table" | "tbody" | "tfoot" | "thead" | "tr") =>
            {
                if self.has_in_scope(name, ScopeKind::Table) && self.close_cell_if_in_scope() {
                    TokenAction::Reprocess
                } else {
                    TokenAction::Consumed
                }
            }
            Token::EndTag { name }
                if matches!(
                    name.as_str(),
                    "body" | "caption" | "col" | "colgroup" | "html"
                ) =>
            {
                TokenAction::Consumed
            }
            _ => self.handle_in_body(token),
        }
    }

    fn handle_after_body(&mut self, token: &Token) -> TokenAction {
        match token {
            Token::Character(character) if is_ascii_whitespace(*character) => {
                self.handle_in_body(token)
            }
            Token::Comment(data) => {
                self.flush_text();
                if let Some(html) = self.html_element {
                    self.append_comment(html, data.clone());
                }
                TokenAction::Consumed
            }
            Token::Doctype(_) => TokenAction::Consumed,
            Token::StartTag { name, .. } if name == "html" => self.handle_in_body(token),
            Token::EndTag { name } if name == "html" => {
                self.flush_text();
                self.mode = InsertionMode::AfterAfterBody;
                TokenAction::Consumed
            }
            Token::Eof => {
                self.flush_text();
                TokenAction::Stop
            }
            _ => {
                self.mode = InsertionMode::InBody;
                TokenAction::Reprocess
            }
        }
    }

    fn handle_after_after_body(&mut self, token: &Token) -> TokenAction {
        match token {
            Token::Comment(data) => {
                self.flush_text();
                let root = self.document.root();
                self.append_comment(root, data.clone());
                TokenAction::Consumed
            }
            Token::Doctype(_) => self.handle_in_body(token),
            Token::Character(character) if is_ascii_whitespace(*character) => {
                self.handle_in_body(token)
            }
            Token::StartTag { name, .. } if name == "html" => self.handle_in_body(token),
            Token::Eof => {
                self.flush_text();
                TokenAction::Stop
            }
            _ => {
                self.mode = InsertionMode::InBody;
                TokenAction::Reprocess
            }
        }
    }

    fn current_tag_name(&self) -> Option<&str> {
        self.open_elements
            .last()
            .and_then(|node| self.document.element(*node))
            .map(|element| element.tag_name.as_str())
    }

    fn process_in_body_with_foster_parenting(&mut self, token: &Token) -> TokenAction {
        let previous = self.foster_parenting;
        self.foster_parenting = true;
        let action = self.handle_in_body(token);
        self.foster_parenting = previous;
        action
    }

    fn flush_pending_table_text(&mut self) {
        if self.pending_table_text.is_empty() {
            return;
        }

        let pending = std::mem::take(&mut self.pending_table_text);
        if pending.chars().all(is_ascii_whitespace) {
            let node = self.document.create_text(pending);
            let parent = self.current_parent();
            self.document
                .append_child(parent, node)
                .expect("tree builder created an invalid table-whitespace parent");
            return;
        }

        let previous = self.foster_parenting;
        self.foster_parenting = true;
        for character in pending.chars() {
            let token = Token::Character(character);
            let _ = self.handle_in_body(&token);
        }
        self.flush_text();
        self.foster_parenting = previous;
    }

    fn close_caption_if_in_scope(&mut self) -> bool {
        self.flush_text();
        if !self.has_in_scope("caption", ScopeKind::Table) {
            return false;
        }
        self.generate_implied_end_tags(None);
        self.pop_until("caption");
        self.clear_active_formatting_to_marker();
        self.mode = InsertionMode::InTable;
        true
    }

    fn close_table_body_if_in_scope(&mut self) -> bool {
        self.flush_text();
        let Some(target) = self.find_in_table_scope(&["tbody", "tfoot", "thead"]) else {
            return false;
        };
        self.clear_stack_back_to_table_body_context();
        if self.current_tag_name() == Some(target.as_str()) {
            self.open_elements.pop();
        } else {
            self.pop_until(&target);
        }
        self.mode = InsertionMode::InTable;
        true
    }

    fn close_row_if_in_scope(&mut self) -> bool {
        self.flush_text();
        if !self.has_in_scope("tr", ScopeKind::Table) {
            return false;
        }
        self.clear_stack_back_to_table_row_context();
        self.open_elements.pop();
        self.mode = InsertionMode::InTableBody;
        true
    }

    fn close_cell_if_in_scope(&mut self) -> bool {
        self.flush_text();
        let Some(target) = self.find_in_table_scope(&["td", "th"]) else {
            return false;
        };
        self.generate_implied_end_tags(None);
        self.pop_until(&target);
        self.clear_active_formatting_to_marker();
        self.mode = InsertionMode::InRow;
        true
    }

    fn find_in_table_scope(&self, targets: &[&str]) -> Option<String> {
        for node in self.open_elements.iter().rev() {
            let Some(element) = self.document.element(*node) else {
                continue;
            };
            if targets.contains(&element.tag_name.as_str()) {
                return Some(element.tag_name.clone());
            }
            if is_scope_boundary(&element.tag_name, ScopeKind::Table) {
                return None;
            }
        }
        None
    }

    fn clear_stack_back_to_table_context(&mut self) {
        while self
            .current_tag_name()
            .is_some_and(|name| !matches!(name, "table" | "template" | "html"))
        {
            self.open_elements.pop();
        }
    }

    fn clear_stack_back_to_table_body_context(&mut self) {
        while self
            .current_tag_name()
            .is_some_and(|name| !matches!(name, "tbody" | "tfoot" | "thead" | "template" | "html"))
        {
            self.open_elements.pop();
        }
    }

    fn clear_stack_back_to_table_row_context(&mut self) {
        while self
            .current_tag_name()
            .is_some_and(|name| !matches!(name, "tr" | "template" | "html"))
        {
            self.open_elements.pop();
        }
    }

    fn reset_insertion_mode_appropriately(&mut self) {
        for node in self.open_elements.iter().rev() {
            let Some(element) = self.document.element(*node) else {
                continue;
            };
            self.mode = match element.tag_name.as_str() {
                "td" | "th" => InsertionMode::InCell,
                "tr" => InsertionMode::InRow,
                "tbody" | "tfoot" | "thead" => InsertionMode::InTableBody,
                "caption" => InsertionMode::InCaption,
                "colgroup" => InsertionMode::InColumnGroup,
                "table" => InsertionMode::InTable,
                "body" => InsertionMode::InBody,
                "html" => InsertionMode::AfterHead,
                _ => continue,
            };
            return;
        }
        self.mode = InsertionMode::InBody;
    }

    fn push_active_formatting(&mut self, node: NodeId, tag_name: &str, attributes: &[Attribute]) {
        let start = self
            .active_formatting
            .iter()
            .rposition(|entry| matches!(entry, ActiveFormattingEntry::Marker))
            .map_or(0, |index| index + 1);

        let mut matches = Vec::new();
        for index in start..self.active_formatting.len() {
            if let ActiveFormattingEntry::Element(entry) = &self.active_formatting[index]
                && entry.tag_name == tag_name
                && same_formatting_attributes(&entry.attributes, attributes)
            {
                matches.push(index);
            }
        }

        if matches.len() >= 3 {
            self.active_formatting.remove(matches[0]);
        }

        self.active_formatting
            .push(ActiveFormattingEntry::Element(FormattingElement {
                node,
                tag_name: tag_name.to_owned(),
                attributes: attributes.to_vec(),
            }));
    }

    fn reconstruct_active_formatting(&mut self) {
        let Some(last_index) = self.active_formatting.len().checked_sub(1) else {
            return;
        };

        match &self.active_formatting[last_index] {
            ActiveFormattingEntry::Marker => return,
            ActiveFormattingEntry::Element(entry) if self.open_elements.contains(&entry.node) => {
                return;
            }
            ActiveFormattingEntry::Element(_) => {}
        }

        self.flush_text();

        let mut first_index = last_index;
        while first_index > 0 {
            match &self.active_formatting[first_index - 1] {
                ActiveFormattingEntry::Marker => break,
                ActiveFormattingEntry::Element(entry)
                    if self.open_elements.contains(&entry.node) =>
                {
                    break;
                }
                ActiveFormattingEntry::Element(_) => first_index -= 1,
            }
        }

        for index in first_index..self.active_formatting.len() {
            let ActiveFormattingEntry::Element(entry) = self.active_formatting[index].clone()
            else {
                continue;
            };
            let node = self.insert_element(&entry.tag_name, &entry.attributes, None, false);
            self.active_formatting[index] = ActiveFormattingEntry::Element(FormattingElement {
                node,
                tag_name: entry.tag_name,
                attributes: entry.attributes,
            });
        }
    }

    fn last_active_formatting_node(&self, tag_name: &str) -> Option<NodeId> {
        for entry in self.active_formatting.iter().rev() {
            match entry {
                ActiveFormattingEntry::Marker => return None,
                ActiveFormattingEntry::Element(entry) if entry.tag_name == tag_name => {
                    return Some(entry.node);
                }
                ActiveFormattingEntry::Element(_) => {}
            }
        }
        None
    }

    fn active_formatting_index_for_node(&self, node: NodeId) -> Option<usize> {
        self.active_formatting.iter().rposition(
            |entry| matches!(entry, ActiveFormattingEntry::Element(entry) if entry.node == node),
        )
    }

    fn remove_active_formatting_node(&mut self, node: NodeId) {
        if let Some(index) = self.active_formatting_index_for_node(node) {
            self.active_formatting.remove(index);
        }
    }

    fn remove_open_element(&mut self, node: NodeId) {
        if let Some(index) = self
            .open_elements
            .iter()
            .position(|candidate| *candidate == node)
        {
            self.open_elements.remove(index);
        }
    }

    fn clear_active_formatting_to_marker(&mut self) {
        while let Some(entry) = self.active_formatting.pop() {
            if matches!(entry, ActiveFormattingEntry::Marker) {
                break;
            }
        }
    }

    fn node_in_scope(&self, target: NodeId, kind: ScopeKind) -> bool {
        for node in self.open_elements.iter().rev() {
            if *node == target {
                return true;
            }
            if self
                .document
                .element(*node)
                .is_some_and(|element| is_scope_boundary(&element.tag_name, kind))
            {
                return false;
            }
        }
        false
    }

    fn adoption_agency(&mut self, subject: &str) {
        if let Some(current) = self.open_elements.last().copied()
            && self
                .document
                .element(current)
                .is_some_and(|element| element.tag_name == subject)
            && self.active_formatting_index_for_node(current).is_none()
        {
            self.open_elements.pop();
            return;
        }

        for _ in 0..8 {
            let Some(formatting_list_index) =
                self.active_formatting
                    .iter()
                    .rposition(|entry| match entry {
                        ActiveFormattingEntry::Marker => false,
                        ActiveFormattingEntry::Element(entry) => entry.tag_name == subject,
                    })
            else {
                self.close_generic_end_tag(subject);
                return;
            };

            if self.active_formatting[formatting_list_index..]
                .iter()
                .any(|entry| matches!(entry, ActiveFormattingEntry::Marker))
            {
                self.close_generic_end_tag(subject);
                return;
            }

            let ActiveFormattingEntry::Element(formatting) =
                self.active_formatting[formatting_list_index].clone()
            else {
                return;
            };

            let Some(formatting_stack_index) = self
                .open_elements
                .iter()
                .position(|node| *node == formatting.node)
            else {
                self.active_formatting.remove(formatting_list_index);
                return;
            };

            if !self.node_in_scope(formatting.node, ScopeKind::Normal) {
                return;
            }

            let furthest_stack_index = ((formatting_stack_index + 1)..self.open_elements.len())
                .find(|index| {
                    self.document
                        .element(self.open_elements[*index])
                        .is_some_and(|element| is_special_element(&element.tag_name))
                });

            let Some(furthest_stack_index) = furthest_stack_index else {
                self.open_elements.truncate(formatting_stack_index);
                self.remove_active_formatting_node(formatting.node);
                return;
            };

            let Some(common_ancestor) = formatting_stack_index
                .checked_sub(1)
                .and_then(|index| self.open_elements.get(index))
                .copied()
            else {
                return;
            };
            let furthest_block = self.open_elements[furthest_stack_index];
            let mut bookmark = formatting_list_index;
            let mut stack_cursor = furthest_stack_index;
            let mut last_node = furthest_block;
            let mut inner_loop_counter = 0usize;

            loop {
                inner_loop_counter += 1;
                let Some(previous_index) = stack_cursor.checked_sub(1) else {
                    return;
                };
                stack_cursor = previous_index;
                let node = self.open_elements[stack_cursor];

                if node == formatting.node {
                    break;
                }

                if inner_loop_counter > 3
                    && let Some(active_index) = self.active_formatting_index_for_node(node)
                {
                    self.active_formatting.remove(active_index);
                    if active_index < bookmark {
                        bookmark = bookmark.saturating_sub(1);
                    }
                }

                let Some(active_index) = self.active_formatting_index_for_node(node) else {
                    self.open_elements.remove(stack_cursor);
                    continue;
                };

                let ActiveFormattingEntry::Element(entry) =
                    self.active_formatting[active_index].clone()
                else {
                    self.open_elements.remove(stack_cursor);
                    continue;
                };

                let new_node = self.create_element_node(&entry.tag_name, &entry.attributes);
                self.active_formatting[active_index] =
                    ActiveFormattingEntry::Element(FormattingElement {
                        node: new_node,
                        tag_name: entry.tag_name,
                        attributes: entry.attributes,
                    });
                self.open_elements[stack_cursor] = new_node;

                if last_node == furthest_block {
                    bookmark = active_index + 1;
                }

                self.document
                    .append_child(new_node, last_node)
                    .expect("adoption agency created an invalid inner reparent");
                last_node = new_node;
            }

            self.document
                .append_child(common_ancestor, last_node)
                .expect("adoption agency created an invalid common-ancestor reparent");

            let new_formatting =
                self.create_element_node(&formatting.tag_name, &formatting.attributes);
            let furthest_children = self.document.children(furthest_block).to_vec();
            for child in furthest_children {
                self.document
                    .append_child(new_formatting, child)
                    .expect("adoption agency created an invalid formatting child");
            }
            self.document
                .append_child(furthest_block, new_formatting)
                .expect("adoption agency created an invalid furthest-block child");

            if let Some(old_index) = self.active_formatting_index_for_node(formatting.node) {
                self.active_formatting.remove(old_index);
                if old_index < bookmark {
                    bookmark = bookmark.saturating_sub(1);
                }
            }
            bookmark = bookmark.min(self.active_formatting.len());
            self.active_formatting.insert(
                bookmark,
                ActiveFormattingEntry::Element(FormattingElement {
                    node: new_formatting,
                    tag_name: formatting.tag_name,
                    attributes: formatting.attributes,
                }),
            );

            self.remove_open_element(formatting.node);
            let Some(furthest_index) = self
                .open_elements
                .iter()
                .position(|node| *node == furthest_block)
            else {
                return;
            };
            self.open_elements
                .insert(furthest_index + 1, new_formatting);
        }
    }

    fn process_head_token(
        &mut self,
        name: &str,
        attributes: &[Attribute],
        return_mode: InsertionMode,
    ) {
        let Some(head) = self.head_element else {
            return;
        };

        if is_head_void_element(name) {
            self.insert_element(name, attributes, Some(head), true);
        } else if is_head_text_element(name) {
            self.insert_element(name, attributes, Some(head), false);
            self.original_mode = return_mode;
            self.mode = InsertionMode::Text;
        }
    }

    fn has_in_scope(&self, target: &str, kind: ScopeKind) -> bool {
        for node in self.open_elements.iter().rev() {
            let Some(element) = self.document.element(*node) else {
                continue;
            };
            if element.tag_name == target {
                return true;
            }
            if is_scope_boundary(&element.tag_name, kind) {
                return false;
            }
        }
        false
    }

    fn has_heading_in_scope(&self) -> bool {
        for node in self.open_elements.iter().rev() {
            let Some(element) = self.document.element(*node) else {
                continue;
            };
            if is_heading(&element.tag_name) {
                return true;
            }
            if is_scope_boundary(&element.tag_name, ScopeKind::Normal) {
                return false;
            }
        }
        false
    }

    fn generate_implied_end_tags(&mut self, except: Option<&str>) {
        while let Some(name) = self
            .open_elements
            .last()
            .and_then(|node| self.document.element(*node))
            .map(|element| element.tag_name.as_str())
        {
            if !is_implied_end_tag(name) || except == Some(name) {
                break;
            }
            self.open_elements.pop();
        }
    }

    fn close_p_if_in_button_scope(&mut self) {
        if self.has_in_scope("p", ScopeKind::Button) {
            self.close_p();
        }
    }

    fn close_p(&mut self) {
        self.generate_implied_end_tags(Some("p"));
        self.pop_until("p");
    }

    fn close_previous_list_item(&mut self) {
        let mut found = false;
        for node in self.open_elements.iter().rev() {
            let Some(element) = self.document.element(*node) else {
                continue;
            };
            if element.tag_name == "li" {
                found = true;
                break;
            }
            if is_special_element(&element.tag_name)
                && !matches!(element.tag_name.as_str(), "address" | "div" | "p")
            {
                break;
            }
        }

        if found {
            self.generate_implied_end_tags(Some("li"));
            self.pop_until("li");
        }
    }

    fn close_previous_description_item(&mut self) {
        let mut target = None;
        for node in self.open_elements.iter().rev() {
            let Some(element) = self.document.element(*node) else {
                continue;
            };
            if matches!(element.tag_name.as_str(), "dd" | "dt") {
                target = Some(element.tag_name.clone());
                break;
            }
            if is_special_element(&element.tag_name)
                && !matches!(element.tag_name.as_str(), "address" | "div" | "p")
            {
                break;
            }
        }

        if let Some(target) = target {
            self.generate_implied_end_tags(Some(&target));
            self.pop_until(&target);
        }
    }

    fn pop_until(&mut self, tag_name: &str) {
        if let Some(position) = self.open_elements.iter().rposition(|node| {
            self.document
                .element(*node)
                .is_some_and(|element| element.tag_name == tag_name)
        }) {
            self.open_elements.truncate(position);
        }
    }

    fn pop_until_heading(&mut self) {
        if let Some(position) = self.open_elements.iter().rposition(|node| {
            self.document
                .element(*node)
                .is_some_and(|element| is_heading(&element.tag_name))
        }) {
            self.open_elements.truncate(position);
        }
    }

    fn close_generic_end_tag(&mut self, tag_name: &str) {
        for (position, node) in self.open_elements.iter().enumerate().rev() {
            let Some(element) = self.document.element(*node) else {
                continue;
            };
            if element.tag_name == tag_name {
                self.generate_implied_end_tags(Some(tag_name));
                self.open_elements.truncate(position);
                return;
            }
            if is_special_element(&element.tag_name) {
                return;
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

    fn create_element_node(&mut self, name: &str, attributes: &[Attribute]) -> NodeId {
        let attributes = attributes
            .iter()
            .cloned()
            .map(|attribute| DomAttribute {
                name: attribute.name,
                value: attribute.value,
            })
            .collect();

        self.document
            .create_element_with_attributes(name.to_owned(), attributes)
    }

    fn insert_element(
        &mut self,
        name: &str,
        attributes: &[Attribute],
        parent: Option<NodeId>,
        force_no_push: bool,
    ) -> NodeId {
        let node = self.create_element_node(name, attributes);
        self.insert_node_at_appropriate_location(node, parent);

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

    fn appropriate_insertion_location(
        &self,
        override_parent: Option<NodeId>,
    ) -> (NodeId, Option<NodeId>) {
        let target = override_parent.unwrap_or_else(|| self.current_parent());
        if !self.foster_parenting
            || !self
                .document
                .element(target)
                .is_some_and(|element| is_foster_parenting_target(&element.tag_name))
        {
            return (target, None);
        }

        let Some((table_index, table)) =
            self.open_elements
                .iter()
                .enumerate()
                .rev()
                .find_map(|(index, node)| {
                    self.document
                        .element(*node)
                        .is_some_and(|element| element.tag_name == "table")
                        .then_some((index, *node))
                })
        else {
            return (
                self.open_elements
                    .first()
                    .copied()
                    .unwrap_or_else(|| self.document.root()),
                None,
            );
        };

        if let Some(parent) = self.document.node(table).and_then(|node| node.parent) {
            return (parent, Some(table));
        }

        if table_index > 0 {
            return (self.open_elements[table_index - 1], None);
        }

        (target, None)
    }

    fn insert_node_at_appropriate_location(
        &mut self,
        node: NodeId,
        override_parent: Option<NodeId>,
    ) {
        let (parent, reference) = self.appropriate_insertion_location(override_parent);
        self.document
            .insert_before(parent, node, reference)
            .expect("tree builder created an invalid parent/child relation");
    }

    fn flush_text(&mut self) {
        if self.text_buffer.is_empty() {
            return;
        }

        let text = std::mem::take(&mut self.text_buffer);
        let node = self.document.create_text(text);
        self.insert_node_at_appropriate_location(node, None);
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

fn same_formatting_attributes(left: &[Attribute], right: &[Attribute]) -> bool {
    left.len() == right.len()
        && left.iter().all(|attribute| {
            right.iter().any(|candidate| {
                candidate.name == attribute.name && candidate.value == attribute.value
            })
        })
}

fn is_formatting_element(tag_name: &str) -> bool {
    matches!(
        tag_name,
        "a" | "b"
            | "big"
            | "code"
            | "em"
            | "font"
            | "i"
            | "nobr"
            | "s"
            | "small"
            | "strike"
            | "strong"
            | "tt"
            | "u"
    )
}

fn is_formatting_marker_container(tag_name: &str) -> bool {
    matches!(tag_name, "applet" | "marquee" | "object")
}

fn is_table_text_context(tag_name: &str) -> bool {
    matches!(tag_name, "table" | "tbody" | "tfoot" | "thead" | "tr")
}

fn is_foster_parenting_target(tag_name: &str) -> bool {
    matches!(tag_name, "table" | "tbody" | "tfoot" | "thead" | "tr")
}

fn is_ascii_whitespace(character: char) -> bool {
    matches!(character, '\t' | '\n' | '\x0c' | '\r' | ' ')
}

fn is_head_token(tag_name: &str) -> bool {
    is_head_void_element(tag_name) || is_head_text_element(tag_name)
}

fn is_head_void_element(tag_name: &str) -> bool {
    matches!(tag_name, "base" | "basefont" | "bgsound" | "link" | "meta")
}

fn is_head_text_element(tag_name: &str) -> bool {
    matches!(tag_name, "title" | "style" | "script" | "noframes")
}

fn is_p_closing_block_start(tag_name: &str) -> bool {
    matches!(
        tag_name,
        "address"
            | "article"
            | "aside"
            | "blockquote"
            | "center"
            | "details"
            | "dialog"
            | "dir"
            | "div"
            | "dl"
            | "fieldset"
            | "figcaption"
            | "figure"
            | "footer"
            | "header"
            | "hgroup"
            | "main"
            | "menu"
            | "nav"
            | "ol"
            | "p"
            | "search"
            | "section"
            | "summary"
            | "ul"
    )
}

fn is_scoped_block_end(tag_name: &str) -> bool {
    matches!(
        tag_name,
        "address"
            | "article"
            | "aside"
            | "blockquote"
            | "button"
            | "center"
            | "details"
            | "dialog"
            | "dir"
            | "div"
            | "dl"
            | "fieldset"
            | "figcaption"
            | "figure"
            | "footer"
            | "header"
            | "hgroup"
            | "listing"
            | "main"
            | "menu"
            | "nav"
            | "ol"
            | "pre"
            | "search"
            | "section"
            | "select"
            | "summary"
            | "ul"
    )
}

fn is_heading(tag_name: &str) -> bool {
    matches!(tag_name, "h1" | "h2" | "h3" | "h4" | "h5" | "h6")
}

fn is_implied_end_tag(tag_name: &str) -> bool {
    matches!(
        tag_name,
        "dd" | "dt" | "li" | "optgroup" | "option" | "p" | "rb" | "rp" | "rt" | "rtc"
    )
}

fn is_scope_boundary(tag_name: &str, kind: ScopeKind) -> bool {
    if matches!(kind, ScopeKind::Table) {
        return matches!(tag_name, "html" | "table" | "template");
    }

    let normal = matches!(
        tag_name,
        "applet"
            | "caption"
            | "html"
            | "table"
            | "td"
            | "th"
            | "marquee"
            | "object"
            | "select"
            | "template"
    );

    normal
        || matches!(kind, ScopeKind::ListItem) && matches!(tag_name, "ol" | "ul")
        || matches!(kind, ScopeKind::Button) && tag_name == "button"
}

fn is_special_element(tag_name: &str) -> bool {
    matches!(
        tag_name,
        "address"
            | "applet"
            | "area"
            | "article"
            | "aside"
            | "base"
            | "basefont"
            | "bgsound"
            | "blockquote"
            | "body"
            | "br"
            | "button"
            | "caption"
            | "center"
            | "col"
            | "colgroup"
            | "dd"
            | "details"
            | "dir"
            | "div"
            | "dl"
            | "dt"
            | "embed"
            | "fieldset"
            | "figcaption"
            | "figure"
            | "footer"
            | "form"
            | "frame"
            | "frameset"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "head"
            | "header"
            | "hgroup"
            | "hr"
            | "html"
            | "iframe"
            | "img"
            | "input"
            | "keygen"
            | "li"
            | "link"
            | "listing"
            | "main"
            | "marquee"
            | "menu"
            | "meta"
            | "nav"
            | "noembed"
            | "noframes"
            | "noscript"
            | "object"
            | "ol"
            | "p"
            | "param"
            | "plaintext"
            | "pre"
            | "script"
            | "search"
            | "section"
            | "select"
            | "source"
            | "style"
            | "summary"
            | "table"
            | "tbody"
            | "td"
            | "template"
            | "textarea"
            | "tfoot"
            | "th"
            | "thead"
            | "title"
            | "tr"
            | "track"
            | "ul"
            | "wbr"
            | "xmp"
    )
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noahs_ark_clause_keeps_at_most_three_matching_formatting_entries() {
        let mut builder = TreeBuilder::new();
        let attributes = vec![Attribute {
            name: "class".to_owned(),
            value: "same".to_owned(),
        }];

        for _ in 0..4 {
            let node = builder.create_element_node("b", &attributes);
            builder.push_active_formatting(node, "b", &attributes);
        }

        let matching = builder
            .active_formatting
            .iter()
            .filter(|entry| {
                matches!(
                    entry,
                    ActiveFormattingEntry::Element(entry)
                        if entry.tag_name == "b"
                            && same_formatting_attributes(&entry.attributes, &attributes)
                )
            })
            .count();
        assert_eq!(matching, 3);
    }
}
