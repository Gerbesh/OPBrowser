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
                let text = normalize_whitespace(text);
                if !text.is_empty() {
                    self.emit_block(&text, style_for_tag(""));
                }
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

                let text = visible_text_content(self.document, node_id);
                let text = normalize_whitespace(&text);
                if text.is_empty() {
                    return;
                }

                self.emit_block(&text, style_for_tag(&element.tag_name));
            }
            NodeKind::Document => {
                for child in self.document.children(node_id) {
                    self.layout_top_level(*child);
                }
            }
        }
    }

    fn emit_block(&mut self, text: &str, style: TextStyle) {
        self.cursor_y += style.margin_top;
        let lines = wrap_text(text, self.width, style.font_size);
        let line_height = ((style.font_size as f32) * 1.35).round() as i32;

        for line in lines {
            self.text_boxes.push(TextBox {
                x: self.x,
                y: self.cursor_y,
                width: self.width,
                height: line_height,
                text: line,
                font_size: style.font_size,
                weight: style.weight,
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

fn visible_text_content(document: &Document, node_id: NodeId) -> String {
    let mut output = String::new();
    collect_visible_text(document, node_id, &mut output);
    output
}

fn collect_visible_text(document: &Document, node_id: NodeId, output: &mut String) {
    let Some(node) = document.node(node_id) else {
        return;
    };

    match &node.kind {
        NodeKind::Text(text) => output.push_str(text),
        NodeKind::Element(element) => {
            if is_hidden_tag(&element.tag_name) {
                return;
            }

            if element.tag_name == "br" {
                output.push(' ');
                return;
            }

            for child in &node.children {
                collect_visible_text(document, *child, output);
            }
        }
        NodeKind::Document => {
            for child in &node.children {
                collect_visible_text(document, *child, output);
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

fn normalize_whitespace(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn wrap_text(text: &str, width: i32, font_size: i32) -> Vec<String> {
    let approximate_char_width = ((font_size as f32) * 0.55).max(1.0);
    let max_chars = ((width as f32) / approximate_char_width).floor().max(1.0) as usize;

    let mut lines = Vec::new();
    let mut current = String::new();

    for word in text.split_whitespace() {
        let word_len = word.chars().count();
        let current_len = current.chars().count();
        let separator = usize::from(!current.is_empty());

        if current_len + separator + word_len <= max_chars {
            if !current.is_empty() {
                current.push(' ');
            }
            current.push_str(word);
            continue;
        }

        if !current.is_empty() {
            lines.push(std::mem::take(&mut current));
        }

        if word_len <= max_chars {
            current.push_str(word);
            continue;
        }

        let chars: Vec<char> = word.chars().collect();
        for chunk in chars.chunks(max_chars) {
            let chunk: String = chunk.iter().collect();
            if chunk.chars().count() == max_chars {
                lines.push(chunk);
            } else {
                current = chunk;
            }
        }
    }

    if !current.is_empty() {
        lines.push(current);
    }

    if lines.is_empty() {
        lines.push(String::new());
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
}
