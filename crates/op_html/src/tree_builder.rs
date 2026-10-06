use op_dom::{
    Attribute as DomAttribute, Document, DocumentMode, DocumentTypeData, NodeId, NodeKind,
};

use crate::{Token, Tokenizer, document_mode};

pub fn parse_document(input: &str) -> Document {
    let mut document = Document::new();
    let mut open_elements: Vec<NodeId> = Vec::new();
    let mut text_buffer = String::new();
    let mut initial_mode = true;

    for token in Tokenizer::new(input).tokenize() {
        let token = if initial_mode {
            match token {
                Token::Character(character) if is_ascii_whitespace(character) => continue,
                Token::Comment(data) => {
                    let root = document.root();
                    append_comment(&mut document, root, data);
                    continue;
                }
                Token::Doctype(doctype) => {
                    document.set_mode(document_mode::classify(&doctype));
                    append_doctype_if_allowed(&mut document, &open_elements, doctype);
                    initial_mode = false;
                    continue;
                }
                Token::Eof => {
                    document.set_mode(DocumentMode::Quirks);
                    break;
                }
                token => {
                    document.set_mode(DocumentMode::Quirks);
                    initial_mode = false;
                    token
                }
            }
        } else {
            token
        };

        match token {
            Token::Character(character) => text_buffer.push(character),
            Token::Comment(data) => {
                flush_text(&mut document, &open_elements, &mut text_buffer);
                let parent = current_parent(&document, &open_elements);
                append_comment(&mut document, parent, data);
            }
            Token::Doctype(_) => {}
            Token::StartTag {
                name,
                attributes,
                self_closing,
            } => {
                flush_text(&mut document, &open_elements, &mut text_buffer);

                let attributes = attributes
                    .into_iter()
                    .map(|attribute| DomAttribute {
                        name: attribute.name,
                        value: attribute.value,
                    })
                    .collect();

                let node = document.create_element_with_attributes(name.clone(), attributes);
                let parent = current_parent(&document, &open_elements);
                document
                    .append_child(parent, node)
                    .expect("tree builder created an invalid parent/child relation");

                if !self_closing && !is_void_element(&name) {
                    open_elements.push(node);
                }
            }
            Token::EndTag { name } => {
                flush_text(&mut document, &open_elements, &mut text_buffer);
                close_matching_element(&document, &mut open_elements, &name);
            }
            Token::Eof => {
                flush_text(&mut document, &open_elements, &mut text_buffer);
                break;
            }
        }
    }

    document
}

fn is_ascii_whitespace(character: char) -> bool {
    matches!(character, '\t' | '\n' | '\x0c' | '\r' | ' ')
}

fn append_comment(document: &mut Document, parent: NodeId, data: String) {
    let node = document.create_comment(data);
    document
        .append_child(parent, node)
        .expect("tree builder created an invalid comment parent");
}

fn append_doctype_if_allowed(
    document: &mut Document,
    open_elements: &[NodeId],
    doctype: crate::Doctype,
) {
    if !open_elements.is_empty() {
        return;
    }

    let root = document.root();
    let root_already_has_element_or_doctype = document.children(root).iter().any(|child| {
        document.node(*child).is_some_and(|node| {
            matches!(node.kind, NodeKind::Element(_) | NodeKind::DocumentType(_))
        })
    });
    if root_already_has_element_or_doctype {
        return;
    }

    let node = document.create_document_type(DocumentTypeData {
        name: doctype.name,
        public_identifier: doctype.public_identifier,
        system_identifier: doctype.system_identifier,
        force_quirks: doctype.force_quirks,
    });
    document
        .append_child(root, node)
        .expect("tree builder created an invalid doctype parent");
}

fn current_parent(document: &Document, open_elements: &[NodeId]) -> NodeId {
    open_elements
        .last()
        .copied()
        .unwrap_or_else(|| document.root())
}

fn flush_text(document: &mut Document, open_elements: &[NodeId], text_buffer: &mut String) {
    if text_buffer.is_empty() {
        return;
    }

    let text = std::mem::take(text_buffer);
    let node = document.create_text(text);
    let parent = current_parent(document, open_elements);
    document
        .append_child(parent, node)
        .expect("tree builder created an invalid text parent");
}

fn close_matching_element(document: &Document, open_elements: &mut Vec<NodeId>, tag_name: &str) {
    let Some(position) = open_elements.iter().rposition(|node_id| {
        document
            .element(*node_id)
            .is_some_and(|element| element.tag_name == tag_name)
    }) else {
        return;
    };

    open_elements.truncate(position);
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
