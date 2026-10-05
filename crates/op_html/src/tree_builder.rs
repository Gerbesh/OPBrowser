use op_dom::{Attribute as DomAttribute, Document, NodeId};

use crate::{Token, Tokenizer};

pub fn parse_document(input: &str) -> Document {
    let mut document = Document::new();
    let mut open_elements: Vec<NodeId> = Vec::new();
    let mut text_buffer = String::new();

    for token in Tokenizer::new(input).tokenize() {
        match token {
            Token::Character(character) => text_buffer.push(character),
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
