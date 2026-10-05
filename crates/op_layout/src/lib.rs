use op_dom::{Document, NodeId, NodeKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontWeight {
    Normal,
    Bold,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextBox {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub text: String,
    pub font_size: i32,
    pub weight: FontWeight,
    pub links: Vec<LinkSpan>,
}

/// UTF-8 byte range within a laid-out line. The painter measures its glyph bounds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkSpan {
    pub start: usize,
    pub end: usize,
    pub href: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayoutTree {
    pub viewport_width: i32,
    pub content_height: i32,
    pub text_boxes: Vec<TextBox>,
}

pub fn layout_document(document: &Document, viewport_width: i32) -> LayoutTree {
    let viewport_width = viewport_width.max(240);
    let margin_x = 32;
    let content_width = (viewport_width - margin_x * 2).max(160);

    let root =
        find_first_element(document, document.root(), "body").unwrap_or_else(|| document.root());

    let mut context = LayoutContext {
        document,
        x: margin_x,
        width: content_width,
        cursor_y: 28,
        text_boxes: Vec::new(),
    };

    for child in document.children(root) {
        context.layout_top_level(*child);
    }

    LayoutTree {
        viewport_width,
        content_height: context.cursor_y + 24,
        text_boxes: context.text_boxes,
    }
}

struct LayoutContext<'a> {
    document: &'a Document,
    x: i32,
    width: i32,
    cursor_y: i32,
    text_boxes: Vec<TextBox>,
}

impl LayoutContext<'_> {
    fn layout_top_level(&mut self, node_id: NodeId) {
        let Some(node) = self.document.node(node_id) else {
            return;
        };

        match &node.kind {
            NodeKind::Text(text) => {
                let chars = text
                    .chars()
                    .map(|ch| InlineChar { ch, href: None })
                    .collect();
                self.emit_block(chars, style_for_tag(""));
            }
            NodeKind::Element(element) => {
                if is_hidden_tag(&element.tag_name) {
                    return;
                }

                // Real documents wrap headings/paragraphs in structural containers.
                // Keep their block defaults instead of flattening an entire div to text.
                if matches!(
                    element.tag_name.as_str(),
                    "html"
                        | "body"
                        | "div"
                        | "main"
                        | "article"
                        | "section"
                        | "nav"
                        | "header"
                        | "footer"
                        | "aside"
                        | "ul"
                        | "ol"
                        | "blockquote"
                ) {
                    for child in &node.children {
                        self.layout_top_level(*child);
                    }
                    return;
                }

                let mut chars = Vec::new();
                collect_inline_text(self.document, node_id, None, &mut chars);
                self.emit_block(chars, style_for_tag(&element.tag_name));
            }
            NodeKind::Document => {
                for child in self.document.children(node_id) {
                    self.layout_top_level(*child);
                }
            }
        }
    }

    fn emit_block(&mut self, chars: Vec<InlineChar<'_>>, style: TextStyle) {
        let lines = wrap_inline_text(chars, self.width, style.font_size);
        if lines.is_empty() {
            return;
        }
        self.cursor_y += style.margin_top;
        let line_height = ((style.font_size as f32) * 1.35).round() as i32;

        for line in lines {
            let mut text = String::new();
            let mut links: Vec<LinkSpan> = Vec::new();
            for character in line {
                let start = text.len();
                text.push(character.ch);
                if let Some(href) = character.href {
                    if let Some(last) = links.last_mut()
                        && last.end == start
                        && last.href == href
                    {
                        last.end = text.len();
                    } else {
                        links.push(LinkSpan {
                            start,
                            end: text.len(),
                            href: href.to_owned(),
                        });
                    }
                }
            }
            self.text_boxes.push(TextBox {
                x: self.x,
                y: self.cursor_y,
                width: self.width,
                height: line_height,
                text,
                font_size: style.font_size,
                weight: style.weight,
                links,
            });
            self.cursor_y += line_height;
        }

        self.cursor_y += style.margin_bottom;
    }
}

#[derive(Debug, Clone, Copy)]
struct TextStyle {
    font_size: i32,
    weight: FontWeight,
    margin_top: i32,
    margin_bottom: i32,
}

fn style_for_tag(tag_name: &str) -> TextStyle {
    match tag_name {
        "h1" => TextStyle {
            font_size: 34,
            weight: FontWeight::Bold,
            margin_top: 8,
            margin_bottom: 16,
        },
        "h2" => TextStyle {
            font_size: 28,
            weight: FontWeight::Bold,
            margin_top: 8,
            margin_bottom: 14,
        },
        "h3" => TextStyle {
            font_size: 23,
            weight: FontWeight::Bold,
            margin_top: 6,
            margin_bottom: 12,
        },
        "p" | "li" => TextStyle {
            font_size: 18,
            weight: FontWeight::Normal,
            margin_top: 0,
            margin_bottom: 12,
        },
        _ => TextStyle {
            font_size: 18,
            weight: FontWeight::Normal,
            margin_top: 0,
            margin_bottom: 10,
        },
    }
}

fn find_first_element(document: &Document, node_id: NodeId, tag_name: &str) -> Option<NodeId> {
    if document
        .element(node_id)
        .is_some_and(|element| element.tag_name == tag_name)
    {
        return Some(node_id);
    }

    for child in document.children(node_id) {
        if let Some(found) = find_first_element(document, *child, tag_name) {
            return Some(found);
        }
    }

    None
}

#[derive(Clone, Copy)]
struct InlineChar<'a> {
    ch: char,
    href: Option<&'a str>,
}

fn collect_inline_text<'a>(
    document: &'a Document,
    node_id: NodeId,
    href: Option<&'a str>,
    output: &mut Vec<InlineChar<'a>>,
) {
    let Some(node) = document.node(node_id) else {
        return;
    };

    match &node.kind {
        NodeKind::Text(text) => output.extend(text.chars().map(|ch| InlineChar { ch, href })),
        NodeKind::Element(element) => {
            if is_hidden_tag(&element.tag_name) {
                return;
            }

            if element.tag_name == "br" {
                output.push(InlineChar { ch: ' ', href });
                return;
            }

            let href = if element.tag_name == "a" {
                element
                    .attributes
                    .iter()
                    .find(|attribute| attribute.name == "href")
                    .map(|attribute| attribute.value.as_str())
            } else {
                href
            };
            for child in &node.children {
                collect_inline_text(document, *child, href, output);
            }
        }
        NodeKind::Document => {
            for child in &node.children {
                collect_inline_text(document, *child, href, output);
            }
        }
    }
}

fn is_hidden_tag(tag_name: &str) -> bool {
    matches!(
        tag_name,
        "head" | "title" | "style" | "script" | "meta" | "link" | "template"
    )
}

fn wrap_inline_text(
    chars: Vec<InlineChar<'_>>,
    width: i32,
    font_size: i32,
) -> Vec<Vec<InlineChar<'_>>> {
    let approximate_char_width = ((font_size as f32) * 0.55).max(1.0);
    let max_chars = ((width as f32) / approximate_char_width).floor().max(1.0) as usize;

    // Normalize in place instead of allocating a second per-character buffer.
    let mut normalized = chars;
    let mut previous_space = true;
    normalized.retain_mut(|character| {
        if character.ch.is_whitespace() {
            if previous_space {
                return false;
            }
            character.ch = ' ';
            previous_space = true;
        } else {
            previous_space = false;
        }
        true
    });
    if normalized.last().is_some_and(|c| c.ch == ' ') {
        normalized.pop();
    }
    let mut lines = Vec::new();
    let mut current = Vec::new();
    let mut index = 0;
    while index < normalized.len() {
        let space = if normalized[index].ch == ' ' {
            let space = normalized[index];
            index += 1;
            Some(space)
        } else {
            None
        };
        let end = normalized[index..]
            .iter()
            .position(|c| c.ch == ' ')
            .map_or(normalized.len(), |offset| index + offset);
        let word = &normalized[index..end];
        if current.len() + usize::from(!current.is_empty()) + word.len() > max_chars
            && !current.is_empty()
        {
            lines.push(std::mem::take(&mut current));
        }
        if !current.is_empty()
            && let Some(space) = space
        {
            current.push(space);
        }
        if word.len() <= max_chars {
            current.extend_from_slice(word);
        } else {
            for chunk in word.chunks(max_chars) {
                if chunk.len() == max_chars {
                    lines.push(chunk.to_vec());
                } else {
                    current.extend_from_slice(chunk);
                }
            }
        }
        index = end;
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use op_dom::Document;

    #[test]
    fn lays_out_heading_and_paragraph_with_distinct_styles() {
        let mut document = Document::new();
        let body = document.create_element("body");
        let h1 = document.create_element("h1");
        let h1_text = document.create_text("OPBrowser");
        let p = document.create_element("p");
        let p_text = document.create_text("First paragraph");

        document.append_child(document.root(), body).unwrap();
        document.append_child(body, h1).unwrap();
        document.append_child(h1, h1_text).unwrap();
        document.append_child(body, p).unwrap();
        document.append_child(p, p_text).unwrap();

        let layout = layout_document(&document, 800);

        assert_eq!(layout.text_boxes.len(), 2);
        assert_eq!(layout.text_boxes[0].text, "OPBrowser");
        assert_eq!(layout.text_boxes[0].font_size, 34);
        assert_eq!(layout.text_boxes[0].weight, FontWeight::Bold);
        assert_eq!(layout.text_boxes[1].font_size, 18);
        assert!(layout.text_boxes[1].y > layout.text_boxes[0].y);
    }

    #[test]
    fn skips_hidden_head_content() {
        let mut document = Document::new();
        let html = document.create_element("html");
        let head = document.create_element("head");
        let title = document.create_element("title");
        let hidden = document.create_text("Do not render me");
        let body = document.create_element("body");
        let visible = document.create_text("Visible");

        document.append_child(document.root(), html).unwrap();
        document.append_child(html, head).unwrap();
        document.append_child(head, title).unwrap();
        document.append_child(title, hidden).unwrap();
        document.append_child(html, body).unwrap();
        document.append_child(body, visible).unwrap();

        let layout = layout_document(&document, 800);

        assert_eq!(layout.text_boxes.len(), 1);
        assert_eq!(layout.text_boxes[0].text, "Visible");
    }

    #[test]
    fn wraps_text_when_the_content_width_is_small() {
        let mut document = Document::new();
        let p = document.create_element("p");
        let text = document
            .create_text("This sentence is deliberately long enough to wrap across multiple lines");

        document.append_child(document.root(), p).unwrap();
        document.append_child(p, text).unwrap();

        let layout = layout_document(&document, 260);

        assert!(layout.text_boxes.len() > 1);
    }

    #[test]
    fn wraps_link_spans_without_losing_unicode_or_nested_labels() {
        let mut document = Document::new();
        let p = document.create_element("p");
        let before = document.create_text("Before ");
        let link = document.create_element_with_attributes(
            "a",
            vec![op_dom::Attribute {
                name: "href".into(),
                value: "../next".into(),
            }],
        );
        let span = document.create_element("span");
        let label = document.create_text("Привет длинная ссылка с переносами");
        let after = document.create_text(" after");
        document.append_child(document.root(), p).unwrap();
        document.append_child(p, before).unwrap();
        document.append_child(p, link).unwrap();
        document.append_child(link, span).unwrap();
        document.append_child(span, label).unwrap();
        document.append_child(p, after).unwrap();
        let layout = layout_document(&document, 260);
        let linked: Vec<&str> = layout
            .text_boxes
            .iter()
            .flat_map(|line| {
                line.links.iter().map(|link| {
                    assert_eq!(link.href, "../next");
                    &line.text[link.start..link.end]
                })
            })
            .collect();
        assert!(linked.len() > 1);
        assert_eq!(
            linked
                .join(" ")
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" "),
            "Привет длинная ссылка с переносами"
        );
        assert!(
            layout
                .text_boxes
                .first()
                .unwrap()
                .text
                .starts_with("Before")
        );
        assert!(layout.text_boxes.last().unwrap().text.ends_with("after"));
    }
}
