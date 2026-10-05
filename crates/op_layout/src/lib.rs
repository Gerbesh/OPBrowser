use op_dom::{Document, NodeId, NodeKind};
use op_image::RasterImage;
use std::collections::HashMap;
use std::sync::Arc;

pub type ImageResources = HashMap<NodeId, Arc<RasterImage>>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageBox {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub image: Arc<RasterImage>,
    pub href: Option<String>,
}

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
pub enum LayoutItem {
    Text(usize),
    Image(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayoutTree {
    pub viewport_width: i32,
    pub content_height: i32,
    pub text_boxes: Vec<TextBox>,
    pub image_boxes: Vec<ImageBox>,
    pub order: Vec<LayoutItem>,
}

pub fn layout_document(document: &Document, viewport_width: i32) -> LayoutTree {
    layout_document_with_images(document, viewport_width, &ImageResources::new())
}

pub fn layout_document_with_images(
    document: &Document,
    viewport_width: i32,
    images: &ImageResources,
) -> LayoutTree {
    layout_document_with_metrics(
        document,
        viewport_width,
        images,
        &mut ApproximateTextMeasurer,
    )
}

pub fn layout_document_with_metrics(
    document: &Document,
    viewport_width: i32,
    images: &ImageResources,
    measurer: &mut dyn TextMeasurer,
) -> LayoutTree {
    flow::layout(document, viewport_width, images, measurer)
}

#[derive(Debug, Clone, Copy)]
pub struct TextMetrics {
    pub width: i32,
    pub ascent: i32,
    pub descent: i32,
}

/// The layout algorithm owns wrapping and placement; this supplies only font extents.
pub trait TextMeasurer {
    fn measure(&mut self, text: &str, font_size: i32, weight: FontWeight) -> TextMetrics;
}

pub struct ApproximateTextMeasurer;
impl TextMeasurer for ApproximateTextMeasurer {
    fn measure(&mut self, text: &str, font_size: i32, _weight: FontWeight) -> TextMetrics {
        TextMetrics {
            width: ((text.chars().count() as f32) * font_size as f32 * 0.55).ceil() as i32,
            ascent: font_size * 4 / 5,
            descent: font_size - font_size * 4 / 5,
        }
    }
}

mod flow;
mod inline;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_image_boxes_preserves_text_order_and_uses_alt_for_failure() {
        let mut document = Document::new();
        let p = document.create_element("p");
        document.append_child(document.root(), p).unwrap();
        let before = document.create_text("before");
        document.append_child(p, before).unwrap();
        let image_node = document.create_element_with_attributes(
            "img",
            vec![op_dom::Attribute {
                name: "width".into(),
                value: "400".into(),
            }],
        );
        document.append_child(p, image_node).unwrap();
        let after = document.create_text("after");
        document.append_child(p, after).unwrap();
        let failed = document.create_element_with_attributes(
            "img",
            vec![op_dom::Attribute {
                name: "alt".into(),
                value: "fallback".into(),
            }],
        );
        document.append_child(p, failed).unwrap();
        let mut images = ImageResources::new();
        images.insert(
            image_node,
            Arc::new(RasterImage::from_premultiplied_bgra(2, 1, vec![255; 8]).unwrap()),
        );
        let layout = layout_document_with_images(&document, 264, &images);
        assert_eq!(
            (layout.image_boxes[0].width, layout.image_boxes[0].height),
            (200, 100)
        );
        assert_eq!(layout.text_boxes[0].text, "before");
        assert!(layout.text_boxes[0].y < layout.image_boxes[0].y);
        assert!(layout.text_boxes[1].y >= layout.image_boxes[0].y + 100);
        assert_eq!(layout.text_boxes[1].text, "afterfallback");
    }

    #[test]
    fn caps_extreme_aspect_ratios_and_zero_sized_fallbacks() {
        let mut document = Document::new();
        let node = document.create_element_with_attributes(
            "img",
            vec![op_dom::Attribute {
                name: "width".into(),
                value: "4096".into(),
            }],
        );
        document.append_child(document.root(), node).unwrap();
        let hidden = document.create_element_with_attributes(
            "img",
            vec![
                op_dom::Attribute {
                    name: "width".into(),
                    value: "0".into(),
                },
                op_dom::Attribute {
                    name: "alt".into(),
                    value: "hidden".into(),
                },
            ],
        );
        document.append_child(document.root(), hidden).unwrap();
        let mut images = ImageResources::new();
        images.insert(
            node,
            Arc::new(RasterImage::from_premultiplied_bgra(1, 4096, vec![255; 4096 * 4]).unwrap()),
        );
        let layout = layout_document_with_images(&document, 800, &images);
        assert_eq!(
            (layout.image_boxes[0].width, layout.image_boxes[0].height),
            (1, 4096)
        );
        assert!(layout.text_boxes.is_empty());
    }
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
