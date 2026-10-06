use op_css::{ComputedStyleMap, CssColor, StyleMap, compute_styles};
use op_dom::{Document, NodeId, NodeKind};
use op_image::RasterImage;
use std::collections::HashMap;
use std::sync::Arc;

pub type ImageResources = HashMap<NodeId, Arc<RasterImage>>;
pub type GeneratedImageResources =
    HashMap<(NodeId, op_css::PseudoElement, usize), Arc<RasterImage>>;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontStyle {
    Normal,
    Italic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextDecoration {
    pub underline: bool,
    pub line_through: bool,
}

impl TextDecoration {
    pub const NONE: Self = Self {
        underline: false,
        line_through: false,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextColor {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
}

impl From<CssColor> for TextColor {
    fn from(color: CssColor) -> Self {
        Self {
            red: color.red,
            green: color.green,
            blue: color.blue,
            alpha: color.alpha,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecorationBorder {
    pub width: i32,
    pub color: TextColor,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoxDecoration {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub background: TextColor,
    pub border_top: DecorationBorder,
    pub border_right: DecorationBorder,
    pub border_bottom: DecorationBorder,
    pub border_left: DecorationBorder,
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
    pub style: FontStyle,
    pub decoration: TextDecoration,
    pub letter_spacing: i32,
    pub word_spacing: i32,
    pub color: TextColor,
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
    pub box_decorations: Vec<BoxDecoration>,
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
    let computed = compute_styles(document, &StyleMap::default());
    layout_document_with_computed_styles_and_metrics(
        document,
        viewport_width,
        images,
        &computed,
        measurer,
    )
}

pub fn layout_document_with_computed_styles_and_metrics(
    document: &Document,
    viewport_width: i32,
    images: &ImageResources,
    computed_styles: &ComputedStyleMap,
    measurer: &mut dyn TextMeasurer,
) -> LayoutTree {
    layout_document_with_resources_and_metrics(
        document,
        viewport_width,
        images,
        &GeneratedImageResources::new(),
        computed_styles,
        measurer,
    )
}

pub fn layout_document_with_resources_and_metrics(
    document: &Document,
    viewport_width: i32,
    images: &ImageResources,
    generated_images: &GeneratedImageResources,
    computed_styles: &ComputedStyleMap,
    measurer: &mut dyn TextMeasurer,
) -> LayoutTree {
    flow::layout(
        document,
        viewport_width,
        images,
        generated_images,
        computed_styles,
        measurer,
    )
}

#[derive(Debug, Clone, Copy)]
pub struct TextMetrics {
    pub width: i32,
    pub ascent: i32,
    pub descent: i32,
}

/// The layout algorithm owns wrapping and placement; this supplies only font extents.
pub trait TextMeasurer {
    fn measure(
        &mut self,
        text: &str,
        font_size: i32,
        weight: FontWeight,
        style: FontStyle,
    ) -> TextMetrics;
}

pub struct ApproximateTextMeasurer;
impl TextMeasurer for ApproximateTextMeasurer {
    fn measure(
        &mut self,
        text: &str,
        font_size: i32,
        _weight: FontWeight,
        _style: FontStyle,
    ) -> TextMetrics {
        TextMetrics {
            width: ((text.chars().count() as f32) * font_size as f32 * 0.55).ceil() as i32,
            ascent: font_size * 4 / 5,
            descent: font_size - font_size * 4 / 5,
        }
    }
}

mod flow;
mod inline;
mod replaced;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_dom_images_use_intrinsic_width_auto_margins_and_collapsed_vertical_margins() {
        let document = op_html::parse_document(
            "<style>p { width:200px } img { display:block; padding:4px; border:2px solid red; background:blue } #first { margin:0 auto 20px } #second { width:50%; height:40px; box-sizing:border-box; margin:12px 10px 5px auto }</style><p><a href=next.html><img id=first></a><img id=second>Tail</p>",
        );
        let computed = compute_styles(&document, &op_css::collect_author_styles(&document).styles);
        let image = Arc::new(
            RasterImage::from_premultiplied_bgra(100, 50, vec![255; 100 * 50 * 4]).unwrap(),
        );
        let mut images = ImageResources::new();
        let mut nodes = vec![document.root()];
        while let Some(node) = nodes.pop() {
            if document
                .element(node)
                .is_some_and(|element| element.tag_name == "img")
            {
                images.insert(node, image.clone());
            }
            nodes.extend(document.children(node));
        }
        let page = layout_document_with_computed_styles_and_metrics(
            &document,
            800,
            &images,
            &computed,
            &mut ApproximateTextMeasurer,
        );
        assert_eq!(page.image_boxes.len(), 2);
        let boxes = page
            .box_decorations
            .iter()
            .filter(|decoration| decoration.border_left.width == 2)
            .collect::<Vec<_>>();
        assert_eq!((boxes[0].width, boxes[0].height), (112, 62));
        assert_eq!((boxes[0].x, boxes[0].y), (76, 28));
        assert_eq!((page.image_boxes[0].x, page.image_boxes[0].y), (82, 34));
        assert_eq!(page.image_boxes[0].href.as_deref(), Some("next.html"));
        assert_eq!((boxes[1].width, boxes[1].height), (100, 40));
        assert_eq!(boxes[1].x, 122);
        assert_eq!(boxes[1].y, boxes[0].y + boxes[0].height + 20);
        assert_eq!(
            (page.image_boxes[1].width, page.image_boxes[1].height),
            (88, 28)
        );
        assert!(page.text_boxes[0].y >= boxes[1].y + boxes[1].height + 5);
    }

    #[test]
    fn generated_block_image_replacements_share_size_margin_and_decoration_geometry() {
        let document = op_html::parse_document(
            "<style>p { width:200px } a::before { display:block; content:url(icon.png); width:100px; height:40px; box-sizing:border-box; padding:4px; border:2px solid red; background:blue; margin:8px auto 12px }</style><p><a href=next.html>Body</a></p>",
        );
        let computed = compute_styles(&document, &op_css::collect_author_styles(&document).styles);
        let image = Arc::new(
            RasterImage::from_premultiplied_bgra(100, 50, vec![255; 100 * 50 * 4]).unwrap(),
        );
        let mut generated = GeneratedImageResources::new();
        let mut nodes = vec![document.root()];
        while let Some(node) = nodes.pop() {
            if computed
                .pseudo_style_for(node, op_css::PseudoElement::Before)
                .is_some()
            {
                generated.insert((node, op_css::PseudoElement::Before, 0), image.clone());
            }
            nodes.extend(document.children(node));
        }
        let page = layout_document_with_resources_and_metrics(
            &document,
            800,
            &ImageResources::new(),
            &generated,
            &computed,
            &mut ApproximateTextMeasurer,
        );
        let image = &page.image_boxes[0];
        assert_eq!((image.width, image.height), (88, 28));
        let decoration = page
            .box_decorations
            .iter()
            .find(|decoration| decoration.border_left.width == 2)
            .unwrap();
        assert_eq!(
            (
                decoration.x,
                decoration.y,
                decoration.width,
                decoration.height
            ),
            (82, 36, 100, 40)
        );
        assert_eq!((image.x, image.y), (88, 42));
        assert_eq!(image.href.as_deref(), Some("next.html"));
        assert!(page.text_boxes[0].y >= 88);
    }

    #[test]
    fn sole_inline_generated_images_use_css_sizes_and_own_decorated_boxes() {
        let document = op_html::parse_document(
            "<style>a::before { content:url(icon.png); width:64px; height:44px; box-sizing:border-box; padding:4px; border:2px solid red; background:blue } a::after { content:'' url(icon.png); width:200px; height:100px }</style><p><a href=next.html>Body</a></p>",
        );
        let computed = compute_styles(&document, &op_css::collect_author_styles(&document).styles);
        let mut generated = GeneratedImageResources::new();
        let image =
            Arc::new(RasterImage::from_premultiplied_bgra(80, 32, vec![255; 80 * 32 * 4]).unwrap());
        let mut nodes = vec![document.root()];
        while let Some(node) = nodes.pop() {
            for pseudo in [op_css::PseudoElement::Before, op_css::PseudoElement::After] {
                if computed.pseudo_style_for(node, pseudo).is_some() {
                    generated.insert((node, pseudo, 0), image.clone());
                }
            }
            nodes.extend(document.children(node));
        }
        let page = layout_document_with_resources_and_metrics(
            &document,
            800,
            &ImageResources::new(),
            &generated,
            &computed,
            &mut ApproximateTextMeasurer,
        );
        assert_eq!(page.image_boxes.len(), 2);
        let before = &page.image_boxes[0];
        assert_eq!((before.width, before.height), (52, 32));
        let decoration = page
            .box_decorations
            .iter()
            .find(|decoration| decoration.border_left.width == 2)
            .unwrap();
        assert_eq!((decoration.width, decoration.height), (64, 44));
        assert_eq!((before.x, before.y), (decoration.x + 6, decoration.y + 6));
        assert_eq!(before.href.as_deref(), Some("next.html"));
        assert_eq!(
            (page.image_boxes[1].width, page.image_boxes[1].height),
            (80, 32)
        );
        let body = &page.text_boxes[0];
        assert_eq!(body.x, decoration.x + decoration.width);
    }

    #[test]
    fn css_image_sizes_override_attributes_and_use_containing_width_box_sizing_and_limits() {
        for (attributes, css, expected) in [
            ("width=300 height=150", "width:80px;height:auto", (80, 40)),
            ("width=300 height=150", "width:auto;height:auto", (100, 50)),
            ("", "height:30px", (60, 30)),
            ("", "width:50%", (100, 50)),
            ("", "font-size:20px;width:2em", (40, 20)),
            (
                "",
                "width:80px;height:50px;padding:6px;border:2px solid red;box-sizing:border-box",
                (64, 34),
            ),
            ("", "width:auto;height:auto;max-width:60px", (60, 30)),
            ("", "width:auto;height:auto;min-width:140px", (140, 70)),
            ("", "width:80px;min-height:60px", (80, 60)),
            ("", "width:80px;height:50%", (80, 40)),
            (
                "",
                "width:auto;height:auto;min-width:100px;max-width:50px",
                (100, 50),
            ),
            ("", "width:0;min-width:40px", (40, 20)),
        ] {
            let document = op_html::parse_document(&format!(
                "<style>p {{ width:200px }} img {{ {css} }}</style><p><img {attributes}></p>"
            ));
            let computed =
                compute_styles(&document, &op_css::collect_author_styles(&document).styles);
            let mut images = ImageResources::new();
            let image = Arc::new(
                RasterImage::from_premultiplied_bgra(100, 50, vec![255; 100 * 50 * 4]).unwrap(),
            );
            let mut nodes = vec![document.root()];
            while let Some(node) = nodes.pop() {
                if document
                    .element(node)
                    .is_some_and(|element| element.tag_name == "img")
                {
                    images.insert(node, image.clone());
                }
                nodes.extend(document.children(node));
            }
            let page = layout_document_with_computed_styles_and_metrics(
                &document,
                800,
                &images,
                &computed,
                &mut ApproximateTextMeasurer,
            );
            assert_eq!(page.image_boxes.len(), 1, "{css}");
            assert_eq!(
                (page.image_boxes[0].width, page.image_boxes[0].height),
                expected,
                "{attributes}: {css}"
            );
        }
    }

    #[test]
    fn image_padding_borders_and_background_form_atomic_inline_boxes() {
        let document = op_html::parse_document(
            "<style>p { width:176px } img { padding:3px 5px; border:2px solid red; background:blue }</style><p>X<a href='next.html'><img width=100 height=50></a><img width=100 height=50>Y</p>",
        );
        let computed = compute_styles(&document, &op_css::collect_author_styles(&document).styles);
        let mut images = ImageResources::new();
        let image = Arc::new(RasterImage::from_premultiplied_bgra(2, 2, vec![255; 16]).unwrap());
        let mut nodes = vec![document.root()];
        while let Some(node) = nodes.pop() {
            if document
                .element(node)
                .is_some_and(|element| element.tag_name == "img")
            {
                images.insert(node, image.clone());
            }
            nodes.extend(document.children(node));
        }
        let page = layout_document_with_computed_styles_and_metrics(
            &document,
            240,
            &images,
            &computed,
            &mut ApproximateTextMeasurer,
        );
        assert_eq!(page.image_boxes.len(), 2);
        let boxes = page
            .box_decorations
            .iter()
            .filter(|decoration| decoration.border_left.width == 2)
            .collect::<Vec<_>>();
        assert_eq!(boxes.len(), 2);
        for (image, decoration) in page.image_boxes.iter().zip(&boxes) {
            assert_eq!((image.width, image.height), (100, 50));
            assert_eq!((decoration.width, decoration.height), (114, 60));
            assert_eq!((image.x, image.y), (decoration.x + 7, decoration.y + 5));
            assert_eq!(decoration.background, CssColor::BLUE.into());
            assert_eq!(decoration.border_top.color, CssColor::RED.into());
        }
        assert_eq!(page.image_boxes[0].href.as_deref(), Some("next.html"));
        assert!(boxes[1].y >= boxes[0].y + boxes[0].height);
        let before = page
            .text_boxes
            .iter()
            .find(|text| text.text == "X")
            .unwrap();
        assert_eq!(boxes[0].x, before.x + before.width);
        let after = page
            .text_boxes
            .iter()
            .find(|text| text.text == "Y")
            .unwrap();
        assert_eq!(after.x, boxes[1].x + boxes[1].width);
    }

    #[test]
    fn decorated_images_fit_content_width_and_respect_nowrap() {
        for nowrap in [false, true] {
            let document = op_html::parse_document(&format!(
                "<style>p {{ width:176px; white-space:{} }} img {{ padding:4px; border:2px solid red }}</style><p><img width=500 height=250><img width=100 height=50></p>",
                if nowrap { "nowrap" } else { "normal" }
            ));
            let computed =
                compute_styles(&document, &op_css::collect_author_styles(&document).styles);
            let mut images = ImageResources::new();
            let image =
                Arc::new(RasterImage::from_premultiplied_bgra(2, 2, vec![255; 16]).unwrap());
            let mut nodes = vec![document.root()];
            while let Some(node) = nodes.pop() {
                if document
                    .element(node)
                    .is_some_and(|element| element.tag_name == "img")
                {
                    images.insert(node, image.clone());
                }
                nodes.extend(document.children(node));
            }
            let page = layout_document_with_computed_styles_and_metrics(
                &document,
                240,
                &images,
                &computed,
                &mut ApproximateTextMeasurer,
            );
            assert_eq!(
                (page.image_boxes[0].width, page.image_boxes[0].height),
                (164, 82)
            );
            let boxes = page
                .box_decorations
                .iter()
                .filter(|decoration| decoration.border_left.width == 2)
                .collect::<Vec<_>>();
            assert_eq!(boxes[0].width, 176);
            if nowrap {
                assert_eq!(boxes[0].y + boxes[0].height, boxes[1].y + boxes[1].height);
                assert_eq!(boxes[1].x, boxes[0].x + boxes[0].width);
            } else {
                assert!(boxes[1].y >= boxes[0].y + boxes[0].height);
            }
        }
    }

    #[test]
    fn generated_images_share_inline_baselines_order_wrapping_and_block_flow() {
        let document = op_html::parse_document(
            "<style>a::before { content:'Before' url(icon.png) 'After' } p::after { content:url(icon.png); display:block; margin-top:8px }</style><p><a href='next.html'>Body</a></p><p>Tail</p>",
        );
        let computed = compute_styles(&document, &op_css::collect_author_styles(&document).styles);
        let mut generated = GeneratedImageResources::new();
        let mut nodes = vec![document.root()];
        let image =
            Arc::new(RasterImage::from_premultiplied_bgra(80, 32, vec![255; 80 * 32 * 4]).unwrap());
        while let Some(node) = nodes.pop() {
            for pseudo in [op_css::PseudoElement::Before, op_css::PseudoElement::After] {
                if let Some(style) = computed.pseudo_style_for(node, pseudo) {
                    for (index, item) in style.items.iter().enumerate() {
                        if matches!(item, op_css::GeneratedContentItem::Image { .. }) {
                            generated.insert((node, pseudo, index), image.clone());
                        }
                    }
                }
            }
            nodes.extend(document.children(node));
        }
        let page = layout_document_with_resources_and_metrics(
            &document,
            800,
            &ImageResources::new(),
            &generated,
            &computed,
            &mut ApproximateTextMeasurer,
        );
        assert_eq!(page.image_boxes.len(), 3);
        let before = page
            .text_boxes
            .iter()
            .find(|text| text.text.contains("Before"))
            .unwrap();
        let after = page
            .text_boxes
            .iter()
            .find(|text| text.text.contains("After"))
            .unwrap();
        let inline_image = &page.image_boxes[0];
        assert_eq!(inline_image.href.as_deref(), Some("next.html"));
        assert_eq!(inline_image.x, before.x + before.width);
        assert_eq!(after.x, inline_image.x + inline_image.width);
        assert_eq!(
            inline_image.y + inline_image.height,
            before.y + before.font_size * 4 / 5
        );
        assert!(page.image_boxes[1].y > inline_image.y);
        let tail = page
            .text_boxes
            .iter()
            .find(|text| text.text == "Tail")
            .unwrap();
        assert!(tail.y >= page.image_boxes[1].y + page.image_boxes[1].height);
        let narrow = layout_document_with_resources_and_metrics(
            &document,
            240,
            &ImageResources::new(),
            &generated,
            &computed,
            &mut ApproximateTextMeasurer,
        );
        assert!(narrow.content_height > page.content_height);
        assert!(
            narrow
                .image_boxes
                .iter()
                .all(|image| image.x + image.width <= 208)
        );
    }

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
    fn block_box_model_changes_content_geometry_and_emits_decoration() {
        let document = op_html::parse_document(
            "<style>.box { margin:10px 20px; padding:8px 12px; background-color:#eef2ff; border:2px solid #4338ca }</style>
             <div class='box'>inside</div><p>after</p>",
        );
        let author = op_css::collect_author_styles(&document);
        let computed = op_css::compute_styles(&document, &author.styles);
        let layout = layout_document_with_computed_styles_and_metrics(
            &document,
            800,
            &ImageResources::new(),
            &computed,
            &mut ApproximateTextMeasurer,
        );

        assert_eq!(layout.box_decorations.len(), 1);
        let decoration = &layout.box_decorations[0];
        assert_eq!(decoration.x, 52);
        assert_eq!(decoration.width, 696);
        assert_eq!(decoration.border_top.width, 2);
        assert_eq!(decoration.border_right.width, 2);
        assert_eq!(decoration.border_bottom.width, 2);
        assert_eq!(decoration.border_left.width, 2);
        assert_eq!(
            decoration.background,
            TextColor {
                red: 0xee,
                green: 0xf2,
                blue: 0xff,
                alpha: 255,
            }
        );
        let inside = layout
            .text_boxes
            .iter()
            .find(|text| text.text == "inside")
            .unwrap();
        let after = layout
            .text_boxes
            .iter()
            .find(|text| text.text == "after")
            .unwrap();
        assert_eq!(inside.x, 66);
        assert!(inside.y >= decoration.y + 10);
        assert!(after.y >= decoration.y + decoration.height + 10);
    }

    #[test]
    fn resolves_percent_width_auto_margins_box_sizing_and_per_side_borders() {
        let document = op_html::parse_document(
            "<style>
                .card {
                    width:50%; min-width:300px; max-width:500px;
                    margin:10px auto; padding:10%;
                    box-sizing:border-box;
                    background-color:#eee;
                    border-left:4px solid red;
                    border-right:6px solid blue;
                }
             </style>
             <div class='card'>centered</div>",
        );
        let author = op_css::collect_author_styles(&document);
        let computed = op_css::compute_styles(&document, &author.styles);
        let layout = layout_document_with_computed_styles_and_metrics(
            &document,
            800,
            &ImageResources::new(),
            &computed,
            &mut ApproximateTextMeasurer,
        );

        let decoration = &layout.box_decorations[0];
        assert_eq!(decoration.x, 216);
        assert_eq!(decoration.width, 368);
        assert_eq!(decoration.border_left.width, 4);
        assert_eq!(decoration.border_right.width, 6);
        assert_eq!(decoration.border_top.width, 0);
        let text = layout
            .text_boxes
            .iter()
            .find(|text| text.text == "centered")
            .unwrap();
        assert_eq!(text.x, 294);
    }

    #[test]
    fn border_box_height_includes_padding_and_border() {
        let document = op_html::parse_document(
            "<div style='height:100px; padding:10px; border:2px solid red; box-sizing:border-box; background-color:white'>height</div>",
        );
        let author = op_css::collect_author_styles(&document);
        let computed = op_css::compute_styles(&document, &author.styles);
        let layout = layout_document_with_computed_styles_and_metrics(
            &document,
            800,
            &ImageResources::new(),
            &computed,
            &mut ApproximateTextMeasurer,
        );

        assert_eq!(layout.box_decorations[0].height, 100);
    }

    #[test]
    fn collapses_adjacent_sibling_vertical_margins() {
        assert_eq!(flow::collapse_margins(20, 12), 20);
        assert_eq!(flow::collapse_margins(20, -8), 12);
        assert_eq!(flow::collapse_margins(-20, -8), -20);

        let document = op_html::parse_document(
            "<div style='height:20px; margin-bottom:30px; background-color:red'></div>
             <div style='height:20px; margin-top:10px; background-color:blue'></div>",
        );
        let author = op_css::collect_author_styles(&document);
        let computed = op_css::compute_styles(&document, &author.styles);
        let layout = layout_document_with_computed_styles_and_metrics(
            &document,
            800,
            &ImageResources::new(),
            &computed,
            &mut ApproximateTextMeasurer,
        );

        assert_eq!(layout.box_decorations.len(), 2);
        let first = &layout.box_decorations[0];
        let second = &layout.box_decorations[1];
        assert_eq!(second.y - (first.y + first.height), 30);
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
    fn text_align_and_line_height_change_real_line_geometry() {
        let document = op_html::parse_document(
            "<p style='text-align:center'>center</p>
             <p style='text-align:right'>right</p>
             <p style='line-height:40px'>first<br>second</p>",
        );
        let author = op_css::collect_author_styles(&document);
        let computed = op_css::compute_styles(&document, &author.styles);
        let layout = layout_document_with_computed_styles_and_metrics(
            &document,
            400,
            &ImageResources::new(),
            &computed,
            &mut ApproximateTextMeasurer,
        );

        let center = layout
            .text_boxes
            .iter()
            .find(|line| line.text == "center")
            .unwrap();
        let right = layout
            .text_boxes
            .iter()
            .find(|line| line.text == "right")
            .unwrap();
        let first = layout
            .text_boxes
            .iter()
            .find(|line| line.text == "first")
            .unwrap();
        let second = layout
            .text_boxes
            .iter()
            .find(|line| line.text == "second")
            .unwrap();
        let content_x = 32;
        let content_width = 336;

        assert_eq!(center.x, content_x + (content_width - center.width) / 2);
        assert_eq!(right.x, content_x + content_width - right.width);
        assert_eq!(second.y - first.y, 40);
    }

    #[test]
    fn white_space_modes_and_inline_text_styles_reach_real_layout() {
        let document = op_html::parse_document(
            "<div id='normal' style='width:90px'>a   b ccc ddd</div>
             <div id='nowrap' style='width:90px; white-space:nowrap'>a   b ccc ddd</div>
             <div id='pre' style='white-space:pre'>a  b\n c</div>
             <div id='preline' style='white-space:pre-line'>a   b\n c</div>
             <p><span style='font-style:italic; text-decoration:underline line-through'>styled</span></p>",
        );
        let author = op_css::collect_author_styles(&document);
        let computed = op_css::compute_styles(&document, &author.styles);
        let layout = layout_document_with_computed_styles_and_metrics(
            &document,
            400,
            &ImageResources::new(),
            &computed,
            &mut ApproximateTextMeasurer,
        );

        let texts: Vec<&str> = layout
            .text_boxes
            .iter()
            .map(|line| line.text.as_str())
            .collect();
        assert!(
            texts
                .iter()
                .filter(|text| text.contains("ccc") || text.contains("ddd"))
                .count()
                >= 3
        );
        assert!(texts.contains(&"a  b"));
        assert!(texts.contains(&" c"));
        assert!(texts.contains(&"a b"));

        let nowrap_lines = layout
            .text_boxes
            .iter()
            .filter(|line| line.text == "a b ccc ddd")
            .count();
        assert_eq!(nowrap_lines, 1);
        let styled = layout
            .text_boxes
            .iter()
            .find(|line| line.text == "styled")
            .unwrap();
        assert_eq!(styled.style, FontStyle::Italic);
        assert!(styled.decoration.underline);
        assert!(styled.decoration.line_through);
    }

    #[test]
    fn text_transform_and_spacing_change_output_width_and_link_ranges() {
        let document = op_html::parse_document(
            "<div style='text-transform:uppercase; letter-spacing:3px; word-spacing:7px'><a href='next'>straße test</a></div>
             <div style='text-transform:capitalize'>hello world</div>",
        );
        let author = op_css::collect_author_styles(&document);
        let computed = op_css::compute_styles(&document, &author.styles);
        let layout = layout_document_with_computed_styles_and_metrics(
            &document,
            500,
            &ImageResources::new(),
            &computed,
            &mut ApproximateTextMeasurer,
        );

        let transformed = layout
            .text_boxes
            .iter()
            .find(|line| line.text == "STRASSE TEST")
            .unwrap();
        assert_eq!(transformed.letter_spacing, 3);
        assert_eq!(transformed.word_spacing, 7);
        assert!(transformed.width > 120);
        assert_eq!(transformed.links.len(), 1);
        assert_eq!(
            &transformed.text[transformed.links[0].start..transformed.links[0].end],
            "STRASSE TEST"
        );
        assert_eq!(transformed.links[0].href, "next");
        assert!(
            layout
                .text_boxes
                .iter()
                .any(|line| line.text == "Hello World")
        );
    }

    #[test]
    fn inline_padding_background_and_borders_form_real_wrapping_fragments() {
        let document = op_html::parse_document(
            "<p>A <span style='padding:2px 5px;background-color:#eef2ff;border:2px solid #4338ca'>boxed <b>bold</b> content that wraps</span> Z</p>",
        );
        let author = op_css::collect_author_styles(&document);
        let computed = op_css::compute_styles(&document, &author.styles);
        let layout = layout_document_with_computed_styles_and_metrics(
            &document,
            220,
            &ImageResources::new(),
            &computed,
            &mut ApproximateTextMeasurer,
        );

        let fragments: Vec<&BoxDecoration> = layout
            .box_decorations
            .iter()
            .filter(|fragment| {
                fragment.background
                    == TextColor {
                        red: 238,
                        green: 242,
                        blue: 255,
                        alpha: 255,
                    }
            })
            .collect();
        assert!(
            fragments.len() >= 2,
            "the inline span should fragment across wrapped lines"
        );
        assert!(fragments.iter().all(|fragment| {
            fragment.border_top.width == 2
                && fragment.border_right.width == 2
                && fragment.border_bottom.width == 2
                && fragment.border_left.width == 2
                && fragment.height >= 32
        }));

        let boxed = layout
            .text_boxes
            .iter()
            .find(|text| text.text.contains("boxed"))
            .unwrap();
        let first_fragment = fragments
            .iter()
            .find(|fragment| boxed.x >= fragment.x && boxed.x < fragment.x + fragment.width)
            .unwrap();
        assert_eq!(boxed.x - first_fragment.x, 7);
        assert!(
            layout
                .text_boxes
                .iter()
                .any(|text| text.text.contains("bold") && text.weight == FontWeight::Bold)
        );
    }

    #[test]
    fn before_and_after_generated_text_join_inline_layout_with_own_styles() {
        let document = op_html::parse_document(
            "<style>
               #note { color:#123456; font-size:20px }
               #note::before { content:'['; color:#b42318; background:#eef2ff; padding:2px 4px; border:1px solid #4338ca }
               #note::after { content:']'; color:#087a35; font-weight:bold }
             </style><p id='note'>Body</p>",
        );
        let author = op_css::collect_author_styles(&document);
        let computed = op_css::compute_styles(&document, &author.styles);
        let layout = layout_document_with_computed_styles_and_metrics(
            &document,
            500,
            &ImageResources::new(),
            &computed,
            &mut ApproximateTextMeasurer,
        );

        let before = layout
            .text_boxes
            .iter()
            .find(|text| text.text == "[")
            .unwrap();
        let body = layout
            .text_boxes
            .iter()
            .find(|text| text.text == "Body")
            .unwrap();
        let after = layout
            .text_boxes
            .iter()
            .find(|text| text.text == "]")
            .unwrap();
        assert!(before.x < body.x && body.x < after.x);
        assert_eq!(
            before.color,
            TextColor {
                red: 180,
                green: 35,
                blue: 24,
                alpha: 255,
            }
        );
        assert_eq!(
            body.color,
            TextColor {
                red: 0x12,
                green: 0x34,
                blue: 0x56,
                alpha: 255,
            }
        );
        assert_eq!(after.weight, FontWeight::Bold);
        let fragment = layout
            .box_decorations
            .iter()
            .find(|fragment| fragment.background.red == 238 && fragment.border_left.width == 1)
            .unwrap();
        assert_eq!(before.x - fragment.x, 5);
        assert!(fragment.x + fragment.width <= body.x);
    }

    #[test]
    fn adjacent_equal_inline_boxes_stay_separate() {
        let document = op_html::parse_document(
            "<p><span style='padding:1px 4px;background-color:#eef2ff;border:1px solid #4338ca'>one</span><span style='padding:1px 4px;background-color:#eef2ff;border:1px solid #4338ca'>two</span></p>",
        );
        let author = op_css::collect_author_styles(&document);
        let computed = op_css::compute_styles(&document, &author.styles);
        let layout = layout_document_with_computed_styles_and_metrics(
            &document,
            500,
            &ImageResources::new(),
            &computed,
            &mut ApproximateTextMeasurer,
        );
        let fragments: Vec<&BoxDecoration> = layout
            .box_decorations
            .iter()
            .filter(|fragment| fragment.background.red == 238 && fragment.border_left.width == 1)
            .collect();
        assert_eq!(fragments.len(), 2);
        assert!(fragments[0].x + fragments[0].width <= fragments[1].x);
    }

    #[test]
    fn generated_blocks_share_width_height_padding_borders_and_auto_margins() {
        let document = op_html::parse_document(
            "<style>
             #host { padding:10px; border:2px solid black }
             #host::before { display:block; content:'PRE'; width:50%; height:60px;
                 box-sizing:border-box; margin:10px auto 7px; padding:4px;
                 border:2px solid blue; background:red }
             #host::after { display:block; content:''; width:80px; min-height:20px;
                 box-sizing:border-box; margin:5px auto 0; padding:2px;
                 border:1px solid red; background:blue }
             </style><div id='host'>Body</div>",
        );
        let computed =
            op_css::compute_styles(&document, &op_css::collect_author_styles(&document).styles);
        let layout = layout_document_with_computed_styles_and_metrics(
            &document,
            500,
            &ImageResources::new(),
            &computed,
            &mut ApproximateTextMeasurer,
        );
        let before = layout
            .box_decorations
            .iter()
            .find(|d| d.background.red == 255 && d.background.blue == 0)
            .unwrap();
        assert_eq!(
            (before.x, before.y, before.width, before.height),
            (147, 50, 206, 60)
        );
        let pre = layout
            .text_boxes
            .iter()
            .find(|text| text.text == "PRE")
            .unwrap();
        // Approximate metrics center 18px glyph extents inside a 24px line box.
        assert_eq!((pre.x, pre.y), (before.x + 6, before.y + 9));
        let body = layout
            .text_boxes
            .iter()
            .find(|text| text.text == "Body")
            .unwrap();
        assert_eq!(body.x, 44);
        assert_eq!(body.y, before.y + before.height + 10);
        let after = layout
            .box_decorations
            .iter()
            .find(|d| d.background.blue == 255)
            .unwrap();
        assert_eq!((after.x, after.width, after.height), (210, 80, 20));
        assert!(after.y >= body.y + body.height + 5);
        assert_eq!(
            layout.text_boxes.len(),
            2,
            "empty pseudo box must not invent glyphs"
        );
    }

    #[test]
    fn adjacent_empty_generated_blocks_collapse_margins_and_obey_min_max_sizes() {
        let document = op_html::parse_document(
            "<style>
             #host::before { display:block; content:''; width:200px; max-width:80px;
                 height:2px; min-height:10px; background:red; margin-bottom:12px }
             #host::after { display:block; content:''; width:10px; min-width:60px;
                 height:90px; max-height:15px; background:blue; margin-top:20px }
             </style><span id='host'></span>",
        );
        let computed =
            op_css::compute_styles(&document, &op_css::collect_author_styles(&document).styles);
        let layout = layout_document_with_computed_styles_and_metrics(
            &document,
            500,
            &ImageResources::new(),
            &computed,
            &mut ApproximateTextMeasurer,
        );
        assert!(layout.text_boxes.is_empty());
        let before = &layout.box_decorations[0];
        let after = &layout.box_decorations[1];
        assert_eq!((before.width, before.height), (80, 10));
        assert_eq!((after.width, after.height), (60, 15));
        assert_eq!(after.y - before.y - before.height, 20);
    }

    #[test]
    fn empty_inline_pseudos_reserve_edges_align_and_paint_without_glyphs() {
        let document = op_html::parse_document(
            "<style>#host { text-align:center }
             #host::before { content:''; padding:2px 5px; border:1px solid blue; background:red }
             #host::after { content:''; padding:3px 7px; border:2px solid red; background:blue }
             </style><div id='host'>Body</div>",
        );
        let computed =
            op_css::compute_styles(&document, &op_css::collect_author_styles(&document).styles);
        let layout = layout_document_with_computed_styles_and_metrics(
            &document,
            500,
            &ImageResources::new(),
            &computed,
            &mut ApproximateTextMeasurer,
        );
        assert_eq!(layout.text_boxes.len(), 1);
        assert_eq!(layout.text_boxes[0].text, "Body");
        assert_eq!(layout.order, vec![LayoutItem::Text(0)]);
        assert_eq!(layout.box_decorations.len(), 2);
        let before = &layout.box_decorations[0];
        let after = &layout.box_decorations[1];
        let body = &layout.text_boxes[0];
        assert_eq!(before.width, 12);
        assert_eq!(after.width, 18);
        assert_eq!(before.x + before.width, body.x);
        assert_eq!(body.x + body.width, after.x);
        assert_eq!(before.y, after.y);
        assert_eq!(before.height, 34);
        let total = before.width + body.width + after.width;
        assert_eq!(before.x, 32 + (436 - total) / 2);
    }

    #[test]
    fn empty_inline_pseudos_wrap_atomically_and_honor_nowrap() {
        for (white_space, same_line) in [("normal", false), ("nowrap", true)] {
            let document = op_html::parse_document(&format!(
                "<style>#host {{ width:80px; white-space:{white_space} }}
                 #host::before {{ content:''; padding:0 30px; background:red }}
                 #host::after {{ content:''; padding:0 30px; background:blue }}
                 </style><div id='host'>A</div>"
            ));
            let computed =
                op_css::compute_styles(&document, &op_css::collect_author_styles(&document).styles);
            let layout = layout_document_with_computed_styles_and_metrics(
                &document,
                500,
                &ImageResources::new(),
                &computed,
                &mut ApproximateTextMeasurer,
            );
            assert_eq!(layout.box_decorations.len(), 2);
            let before = &layout.box_decorations[0];
            let after = &layout.box_decorations[1];
            assert_eq!(before.y == after.y, same_line, "{white_space}");
            assert_eq!(before.width, 60);
            assert_eq!(after.width, 60);
            assert_eq!(layout.text_boxes.len(), 1);
            assert_eq!(layout.text_boxes[0].text, "A");
        }
    }

    #[test]
    fn standalone_empty_inline_pseudo_creates_only_a_decorated_line() {
        let document = op_html::parse_document(
            "<style>#host::before { content:''; padding:3px 5px; border:1px solid blue; background:red }
             #host::after { content:'' }
             </style><div id='host'></div>",
        );
        let computed =
            op_css::compute_styles(&document, &op_css::collect_author_styles(&document).styles);
        let layout = layout_document_with_computed_styles_and_metrics(
            &document,
            500,
            &ImageResources::new(),
            &computed,
            &mut ApproximateTextMeasurer,
        );
        assert!(layout.text_boxes.is_empty());
        assert!(layout.order.is_empty());
        assert_eq!(layout.box_decorations.len(), 1);
        assert_eq!(
            (
                layout.box_decorations[0].width,
                layout.box_decorations[0].height
            ),
            (12, 32)
        );
    }

    #[test]
    fn empty_dom_inline_boxes_use_the_same_edges_without_duplicating_inherited_boxes() {
        let document = op_html::parse_document(
            "<p>A<span style='padding:2px 5px;border:1px solid blue;background:red'></span>B
             <span style='padding:2px 5px;border:1px solid blue;background:red'>C<b></b>D</span></p>",
        );
        let computed =
            op_css::compute_styles(&document, &op_css::collect_author_styles(&document).styles);
        let layout = layout_document_with_computed_styles_and_metrics(
            &document,
            500,
            &ImageResources::new(),
            &computed,
            &mut ApproximateTextMeasurer,
        );
        assert_eq!(layout.box_decorations.len(), 2);
        assert_eq!(layout.box_decorations[0].width, 12);
        let a = &layout.text_boxes[0];
        let b = &layout.text_boxes[1];
        assert_eq!(b.x - a.x - a.width, 12);
        let text = layout
            .text_boxes
            .iter()
            .map(|text| text.text.as_str())
            .collect::<String>();
        assert_eq!(text, "AB CD");
        assert_eq!(
            layout.box_decorations[1].width,
            12 + layout.text_boxes.last().unwrap().width
        );
    }

    #[test]
    fn definite_block_height_keeps_overflow_outside_following_flow_geometry() {
        let document = op_html::parse_document(
            "<style>#small { width:80px; height:5px; border:1px solid red; background:blue }
             </style><div id='small'>many words wrapping onto multiple overflowing lines</div><div>Next</div>",
        );
        let computed =
            op_css::compute_styles(&document, &op_css::collect_author_styles(&document).styles);
        let layout = layout_document_with_computed_styles_and_metrics(
            &document,
            500,
            &ImageResources::new(),
            &computed,
            &mut ApproximateTextMeasurer,
        );
        let small = &layout.box_decorations[0];
        assert_eq!(small.height, 7);
        let next = layout
            .text_boxes
            .iter()
            .find(|text| text.text == "Next")
            .unwrap();
        assert_eq!(next.y, small.y + small.height + 3);
        assert!(layout.text_boxes.iter().any(|text| text.y > next.y));
    }

    #[test]
    fn html_table_layout_places_rows_and_columns_in_a_real_grid() {
        let document = op_html::parse_document(
            "<style>
               table { width:300px; background:#eeeeee; border:2px solid #222 }
               td, th { border:1px solid #000; padding:4px }
               #left { background:#ff0000 }
               #right { background:#0000ff }
             </style>
             <table>
               <caption>Title</caption>
               <tr><th>A</th><th>B</th></tr>
               <tr><td id='left'>one</td><td id='right'>two</td></tr>
             </table>
             <p>after</p>",
        );
        let author = op_css::collect_author_styles(&document);
        let computed = compute_styles(&document, &author.styles);
        let layout = layout_document_with_computed_styles_and_metrics(
            &document,
            800,
            &ImageResources::new(),
            &computed,
            &mut ApproximateTextMeasurer,
        );

        let text = |value: &str| {
            layout
                .text_boxes
                .iter()
                .find(|line| line.text.contains(value))
                .unwrap()
        };
        let title = text("Title");
        let a = text("A");
        let b = text("B");
        let one = text("one");
        let two = text("two");
        let after = text("after");

        assert!(title.y < a.y);
        assert_eq!(a.y, b.y);
        assert_eq!(one.y, two.y);
        assert!(b.x > a.x + 100);
        assert!((one.x - a.x).abs() <= 2);
        assert!((two.x - b.x).abs() <= 2);
        assert!(after.y > one.y);

        let red = TextColor {
            red: 255,
            green: 0,
            blue: 0,
            alpha: 255,
        };
        let blue = TextColor {
            red: 0,
            green: 0,
            blue: 255,
            alpha: 255,
        };
        let left = layout
            .box_decorations
            .iter()
            .find(|decoration| decoration.background == red)
            .unwrap();
        let right = layout
            .box_decorations
            .iter()
            .find(|decoration| decoration.background == blue)
            .unwrap();
        assert_eq!(left.y, right.y);
        assert_eq!(left.width, right.width);
        assert!(right.x > left.x);
    }

    #[test]
    fn table_colspan_and_rowspan_affect_cell_geometry() {
        let document = op_html::parse_document(
            "<style>
               table { width:300px }
               td { border:1px solid #111; padding:3px }
               #wide { background:#ff0000 }
               #span { background:#0000ff }
               #right { background:#00ff00 }
               #bottom { background:#ffff00 }
             </style>
             <table>
               <tr><td id='wide' colspan='2'>wide</td></tr>
               <tr><td id='span' rowspan='2'>span</td><td id='right'>right</td></tr>
               <tr><td id='bottom'>bottom</td></tr>
             </table>",
        );
        let author = op_css::collect_author_styles(&document);
        let computed = compute_styles(&document, &author.styles);
        let layout = layout_document_with_computed_styles_and_metrics(
            &document,
            800,
            &ImageResources::new(),
            &computed,
            &mut ApproximateTextMeasurer,
        );

        let decoration = |color: TextColor| {
            layout
                .box_decorations
                .iter()
                .find(|decoration| decoration.background == color)
                .unwrap()
        };
        let wide = decoration(TextColor {
            red: 255,
            green: 0,
            blue: 0,
            alpha: 255,
        });
        let span = decoration(TextColor {
            red: 0,
            green: 0,
            blue: 255,
            alpha: 255,
        });
        let right = decoration(TextColor {
            red: 0,
            green: 255,
            blue: 0,
            alpha: 255,
        });
        let bottom = decoration(TextColor {
            red: 255,
            green: 255,
            blue: 0,
            alpha: 255,
        });

        assert!(wide.width > right.width);
        assert!(span.height > right.height);
        assert_eq!(right.x, bottom.x);
        assert!(bottom.y > right.y);
        assert_eq!(span.x, wide.x);
    }

    #[test]
    fn caption_side_bottom_places_caption_after_table_box() {
        let document = op_html::parse_document(
            "<style>
               table { width:220px; border-spacing:0; background:#0000ff }
               caption { caption-side:bottom; background:#ff0000; margin:0 }
               td { background:#00ff00; padding:0 }
             </style>
             <table>
               <caption>Bottom caption</caption>
               <tr><td>cell</td></tr>
             </table>
             <p>after</p>",
        );
        let computed = compute_styles(&document, &op_css::collect_author_styles(&document).styles);
        let layout = layout_document_with_computed_styles_and_metrics(
            &document,
            800,
            &ImageResources::new(),
            &computed,
            &mut ApproximateTextMeasurer,
        );

        let decoration = |color: TextColor| {
            layout
                .box_decorations
                .iter()
                .find(|decoration| decoration.background == color)
                .unwrap()
        };
        let table = decoration(TextColor {
            red: 0,
            green: 0,
            blue: 255,
            alpha: 255,
        });
        let caption = decoration(TextColor {
            red: 255,
            green: 0,
            blue: 0,
            alpha: 255,
        });
        let cell = decoration(TextColor {
            red: 0,
            green: 255,
            blue: 0,
            alpha: 255,
        });
        let after = layout
            .text_boxes
            .iter()
            .find(|text| text.text.contains("after"))
            .unwrap();

        assert!(cell.y >= table.y);
        assert!(cell.y < table.y + table.height);
        assert!(caption.y >= table.y + table.height);
        assert!(after.y >= caption.y + caption.height);
    }

    #[test]
    fn auto_table_percentage_column_reserves_its_share_before_auto_tracks_expand() {
        let document = op_html::parse_document(
            "<style>
               table { width:400px; border-spacing:0 }
               #quarter { width:25% }
               td { padding:0 }
               #left { background:#ff0000 }
               #right { background:#0000ff }
             </style>
             <table>
               <colgroup><col id='quarter'><col></colgroup>
               <tr><td id='left'>a</td><td id='right'>b</td></tr>
             </table>",
        );
        let computed = compute_styles(&document, &op_css::collect_author_styles(&document).styles);
        let layout = layout_document_with_computed_styles_and_metrics(
            &document,
            800,
            &ImageResources::new(),
            &computed,
            &mut ApproximateTextMeasurer,
        );

        let decoration = |color: TextColor| {
            layout
                .box_decorations
                .iter()
                .find(|decoration| decoration.background == color)
                .unwrap()
        };
        let left = decoration(TextColor {
            red: 255,
            green: 0,
            blue: 0,
            alpha: 255,
        });
        let right = decoration(TextColor {
            red: 0,
            green: 0,
            blue: 255,
            alpha: 255,
        });

        assert!((left.width - 100).abs() <= 2, "{left:?}");
        assert!((right.width - 300).abs() <= 2, "{right:?}");
        assert_eq!(right.x, left.x + left.width);
    }

    #[test]
    fn fixed_table_uses_column_and_first_row_widths_not_late_content() {
        let document = op_html::parse_document(
            "<style>
               table { width:400px; table-layout:fixed; border-spacing:0 }
               #first-col { width:100px }
               td { padding:0 }
               #a { background:#ff0000 }
               #b { background:#0000ff }
               #late-a { background:#00ff00 }
               #late-b { background:#ffff00 }
             </style>
             <table>
               <colgroup><col id='first-col'><col></colgroup>
               <tr><td id='a'>a</td><td id='b'>b</td></tr>
               <tr>
                 <td id='late-a'>this late cell contains an absurdly long unbrokenwordunbrokenwordunbrokenword</td>
                 <td id='late-b'>z</td>
               </tr>
             </table>",
        );
        let computed = compute_styles(&document, &op_css::collect_author_styles(&document).styles);
        let layout = layout_document_with_computed_styles_and_metrics(
            &document,
            800,
            &ImageResources::new(),
            &computed,
            &mut ApproximateTextMeasurer,
        );

        let decoration = |color: TextColor| {
            layout
                .box_decorations
                .iter()
                .find(|decoration| decoration.background == color)
                .unwrap()
        };
        let a = decoration(TextColor {
            red: 255,
            green: 0,
            blue: 0,
            alpha: 255,
        });
        let b = decoration(TextColor {
            red: 0,
            green: 0,
            blue: 255,
            alpha: 255,
        });
        let late_a = decoration(TextColor {
            red: 0,
            green: 255,
            blue: 0,
            alpha: 255,
        });
        let late_b = decoration(TextColor {
            red: 255,
            green: 255,
            blue: 0,
            alpha: 255,
        });

        assert!((a.width - 100).abs() <= 2, "{a:?}");
        assert!((b.width - 300).abs() <= 2, "{b:?}");
        assert_eq!(late_a.width, a.width);
        assert_eq!(late_b.width, b.width);
        assert_eq!(b.x, a.x + a.width);
    }

    #[test]
    fn fixed_table_first_row_percentage_width_drives_unspecified_tracks() {
        let document = op_html::parse_document(
            "<style>
               table { width:400px; table-layout:fixed; border-spacing:0 }
               td { padding:0 }
               #first { width:30%; background:#ff0000 }
               #second { background:#0000ff }
             </style>
             <table>
               <tr><td id='first'>a</td><td id='second'>b</td></tr>
               <tr><td>tiny</td><td>later content is intentionally much much much longer</td></tr>
             </table>",
        );
        let computed = compute_styles(&document, &op_css::collect_author_styles(&document).styles);
        let layout = layout_document_with_computed_styles_and_metrics(
            &document,
            800,
            &ImageResources::new(),
            &computed,
            &mut ApproximateTextMeasurer,
        );

        let decoration = |color: TextColor| {
            layout
                .box_decorations
                .iter()
                .find(|decoration| decoration.background == color)
                .unwrap()
        };
        let first = decoration(TextColor {
            red: 255,
            green: 0,
            blue: 0,
            alpha: 255,
        });
        let second = decoration(TextColor {
            red: 0,
            green: 0,
            blue: 255,
            alpha: 255,
        });

        assert!((first.width - 120).abs() <= 2, "{first:?}");
        assert!((second.width - 280).abs() <= 2, "{second:?}");
    }

    #[test]
    fn table_intrinsic_content_and_col_hints_drive_track_widths() {
        let document = op_html::parse_document(
            "<style>
               table { width:420px; border-spacing:4px }
               td { padding:2px }
               #short { background:#ff0000 }
               #long { background:#0000ff }
             </style>
             <table>
               <tr>
                 <td id='short'>x</td>
                 <td id='long'>a much much longer table cell value</td>
               </tr>
             </table>
             <table style='width:420px'>
               <colgroup><col style='width:260px'><col></colgroup>
               <tr>
                 <td id='hinted' style='background:#00ff00'>a</td>
                 <td id='rest' style='background:#ffff00'>b</td>
               </tr>
             </table>",
        );
        let author = op_css::collect_author_styles(&document);
        let computed = compute_styles(&document, &author.styles);
        let layout = layout_document_with_computed_styles_and_metrics(
            &document,
            800,
            &ImageResources::new(),
            &computed,
            &mut ApproximateTextMeasurer,
        );

        let decoration = |color: TextColor| {
            layout
                .box_decorations
                .iter()
                .find(|decoration| decoration.background == color)
                .unwrap()
        };
        let short = decoration(TextColor {
            red: 255,
            green: 0,
            blue: 0,
            alpha: 255,
        });
        let long = decoration(TextColor {
            red: 0,
            green: 0,
            blue: 255,
            alpha: 255,
        });
        let hinted = decoration(TextColor {
            red: 0,
            green: 255,
            blue: 0,
            alpha: 255,
        });
        let rest = decoration(TextColor {
            red: 255,
            green: 255,
            blue: 0,
            alpha: 255,
        });

        assert!(long.width > short.width * 2);
        assert!(hinted.width > rest.width);
        assert!(hinted.width >= 250);
    }

    #[test]
    fn display_contents_rows_flatten_into_one_anonymous_table_row() {
        let document = op_html::parse_document(
            "<style>
               #table { display:table; width:320px; border-spacing:0 }
               .row { display:contents }
               .cell { display:table-cell; padding:0 }
               #a { background:#ff0000 }
               #b { background:#0000ff }
             </style>
             <div id='table'>
               <div class='row'><div id='a' class='cell'>PA</div></div>
               <div class='row'><div id='b' class='cell'>SS</div></div>
             </div>",
        );
        let computed = compute_styles(&document, &op_css::collect_author_styles(&document).styles);
        let layout = layout_document_with_computed_styles_and_metrics(
            &document,
            800,
            &ImageResources::new(),
            &computed,
            &mut ApproximateTextMeasurer,
        );

        let decoration = |color: TextColor| {
            layout
                .box_decorations
                .iter()
                .find(|decoration| decoration.background == color)
                .unwrap()
        };
        let a = decoration(TextColor {
            red: 255,
            green: 0,
            blue: 0,
            alpha: 255,
        });
        let b = decoration(TextColor {
            red: 0,
            green: 0,
            blue: 255,
            alpha: 255,
        });

        assert_eq!(a.y, b.y);
        assert_eq!(b.x, a.x + a.width);
    }

    #[test]
    fn nested_display_contents_are_transparent_to_table_row_fixup() {
        let document = op_html::parse_document(
            "<style>
               #table { display:table; width:320px; border-spacing:0 }
               #outer, #inner { display:contents }
               #row { display:table-row }
               .cell { display:table-cell; padding:0 }
               #a { background:#00ff00 }
               #b { background:#ffff00 }
             </style>
             <div id='table'>
               <div id='outer'>
                 <div id='row'>
                   <div id='inner'>
                     <div id='a' class='cell'>A</div>
                     <div id='b' class='cell'>B</div>
                   </div>
                 </div>
               </div>
             </div>",
        );
        let computed = compute_styles(&document, &op_css::collect_author_styles(&document).styles);
        let layout = layout_document_with_computed_styles_and_metrics(
            &document,
            800,
            &ImageResources::new(),
            &computed,
            &mut ApproximateTextMeasurer,
        );

        let decoration = |color: TextColor| {
            layout
                .box_decorations
                .iter()
                .find(|decoration| decoration.background == color)
                .unwrap()
        };
        let a = decoration(TextColor {
            red: 0,
            green: 255,
            blue: 0,
            alpha: 255,
        });
        let b = decoration(TextColor {
            red: 255,
            green: 255,
            blue: 0,
            alpha: 255,
        });

        assert_eq!(a.y, b.y);
        assert_eq!(b.x, a.x + a.width);
    }

    #[test]
    fn css_table_fixup_generates_missing_rows_for_direct_cells_and_row_groups() {
        let document = op_html::parse_document(
            "<style>
               #table { display:table; width:360px; border-spacing:0 }
               #group { display:table-row-group }
               .cell { display:table-cell; padding:0 }
               #a { background:#ff0000 }
               #b { background:#0000ff }
               #c { background:#00ff00 }
               #d { background:#ffff00 }
             </style>
             <div id='table'>
               <div id='a' class='cell'>A</div>
               <div id='b' class='cell'>B</div>
               <div id='group'>
                 <div id='c' class='cell'>C</div>
                 <div id='d' class='cell'>D</div>
               </div>
             </div>",
        );
        let computed = compute_styles(&document, &op_css::collect_author_styles(&document).styles);
        let layout = layout_document_with_computed_styles_and_metrics(
            &document,
            800,
            &ImageResources::new(),
            &computed,
            &mut ApproximateTextMeasurer,
        );

        let decoration = |color: TextColor| {
            layout
                .box_decorations
                .iter()
                .find(|decoration| decoration.background == color)
                .unwrap()
        };
        let a = decoration(TextColor {
            red: 255,
            green: 0,
            blue: 0,
            alpha: 255,
        });
        let b = decoration(TextColor {
            red: 0,
            green: 0,
            blue: 255,
            alpha: 255,
        });
        let c = decoration(TextColor {
            red: 0,
            green: 255,
            blue: 0,
            alpha: 255,
        });
        let d = decoration(TextColor {
            red: 255,
            green: 255,
            blue: 0,
            alpha: 255,
        });

        assert_eq!(a.y, b.y);
        assert_eq!(c.y, d.y);
        assert!(c.y > a.y);
        assert_eq!(b.x, a.x + a.width);
        assert_eq!(d.x, c.x + c.width);
    }

    #[test]
    fn css_table_fixup_wraps_non_cell_row_children_in_one_anonymous_cell() {
        let document = op_html::parse_document(
            "<style>
               #table { display:table; width:280px; border-spacing:0 }
               #row { display:table-row }
               #first, #second { display:block; margin:0; padding:0 }
               #first { background:#ff0000 }
               #second { background:#0000ff }
             </style>
             <div id='table'>
               <div id='row'>
                 <div id='first'>first</div>
                 <div id='second'>second</div>
               </div>
             </div>",
        );
        let computed = compute_styles(&document, &op_css::collect_author_styles(&document).styles);
        let layout = layout_document_with_computed_styles_and_metrics(
            &document,
            800,
            &ImageResources::new(),
            &computed,
            &mut ApproximateTextMeasurer,
        );

        let decoration = |color: TextColor| {
            layout
                .box_decorations
                .iter()
                .find(|decoration| decoration.background == color)
                .unwrap()
        };
        let first = decoration(TextColor {
            red: 255,
            green: 0,
            blue: 0,
            alpha: 255,
        });
        let second = decoration(TextColor {
            red: 0,
            green: 0,
            blue: 255,
            alpha: 255,
        });

        assert_eq!(first.x, second.x);
        assert_eq!(first.width, second.width);
        assert!(second.y >= first.y + first.height);
    }

    #[test]
    fn css_table_fixup_wraps_orphan_cells_in_one_anonymous_table_row() {
        let document = op_html::parse_document(
            "<style>
               #host { width:360px }
               .cell { display:table-cell; padding:0 }
               #a { background:#ff0000 }
               #b { background:#0000ff }
               #after { display:block; margin:0; background:#00ff00 }
             </style>
             <div id='host'>
               <div id='a' class='cell'>A</div>
               
               <div id='b' class='cell'>B</div>
               <div id='after'>after</div>
             </div>",
        );
        let computed = compute_styles(&document, &op_css::collect_author_styles(&document).styles);
        let layout = layout_document_with_computed_styles_and_metrics(
            &document,
            800,
            &ImageResources::new(),
            &computed,
            &mut ApproximateTextMeasurer,
        );

        let decoration = |color: TextColor| {
            layout
                .box_decorations
                .iter()
                .find(|decoration| decoration.background == color)
                .unwrap()
        };
        let a = decoration(TextColor {
            red: 255,
            green: 0,
            blue: 0,
            alpha: 255,
        });
        let b = decoration(TextColor {
            red: 0,
            green: 0,
            blue: 255,
            alpha: 255,
        });
        let after = decoration(TextColor {
            red: 0,
            green: 255,
            blue: 0,
            alpha: 255,
        });

        assert_eq!(a.y, b.y);
        assert_eq!(b.x, a.x + a.width);
        assert!(after.y >= a.y + a.height);
    }

    #[test]
    fn css_table_fixup_wraps_orphan_rows_in_one_anonymous_table() {
        let document = op_html::parse_document(
            "<style>
               #host { width:320px }
               .row { display:table-row }
               .cell { display:table-cell; padding:0 }
               #a { background:#ff0000 }
               #b { background:#0000ff }
             </style>
             <div id='host'>
               <div class='row'><div id='a' class='cell'>A</div></div>
               <div class='row'><div id='b' class='cell'>B</div></div>
             </div>",
        );
        let computed = compute_styles(&document, &op_css::collect_author_styles(&document).styles);
        let layout = layout_document_with_computed_styles_and_metrics(
            &document,
            800,
            &ImageResources::new(),
            &computed,
            &mut ApproximateTextMeasurer,
        );

        let decoration = |color: TextColor| {
            layout
                .box_decorations
                .iter()
                .find(|decoration| decoration.background == color)
                .unwrap()
        };
        let a = decoration(TextColor {
            red: 255,
            green: 0,
            blue: 0,
            alpha: 255,
        });
        let b = decoration(TextColor {
            red: 0,
            green: 0,
            blue: 255,
            alpha: 255,
        });

        assert_eq!(a.x, b.x);
        assert_eq!(a.width, b.width);
        assert!(b.y >= a.y + a.height);
    }

    #[test]
    fn orphan_table_row_still_gets_an_anonymous_cell_for_normal_children() {
        let document = op_html::parse_document(
            "<style>
               #host { width:300px; color:#ff0000 }
               #row { display:table-row }
               #first, #second { display:block; margin:0; padding:0 }
               #first { background:#0000ff }
               #second { background:#00ff00 }
             </style>
             <div id='host'>
               <div id='row'>
                 <div id='first'>first</div>
                 <div id='second'>second</div>
               </div>
             </div>",
        );
        let computed = compute_styles(&document, &op_css::collect_author_styles(&document).styles);
        let layout = layout_document_with_computed_styles_and_metrics(
            &document,
            800,
            &ImageResources::new(),
            &computed,
            &mut ApproximateTextMeasurer,
        );

        let decoration = |color: TextColor| {
            layout
                .box_decorations
                .iter()
                .find(|decoration| decoration.background == color)
                .unwrap()
        };
        let first = decoration(TextColor {
            red: 0,
            green: 0,
            blue: 255,
            alpha: 255,
        });
        let second = decoration(TextColor {
            red: 0,
            green: 255,
            blue: 0,
            alpha: 255,
        });

        assert_eq!(first.x, second.x);
        assert_eq!(first.width, second.width);
        assert!(second.y >= first.y + first.height);
        assert!(layout.text_boxes.iter().any(|text| {
            text.text == "first"
                && text.color
                    == TextColor {
                        red: 255,
                        green: 0,
                        blue: 0,
                        alpha: 255,
                    }
        }));
    }

    #[test]
    fn orphan_caption_and_cells_share_one_anonymous_table_wrapper() {
        let document = op_html::parse_document(
            "<style>
               #host { width:360px }
               #caption { display:table-caption; background:#ff0000; margin:0 }
               .cell { display:table-cell; padding:0 }
               #a { background:#0000ff }
               #b { background:#00ff00 }
             </style>
             <div id='host'>
               <div id='caption'>Caption</div>
               <div id='a' class='cell'>A</div>
               <div id='b' class='cell'>B</div>
             </div>",
        );
        let computed = compute_styles(&document, &op_css::collect_author_styles(&document).styles);
        let layout = layout_document_with_computed_styles_and_metrics(
            &document,
            800,
            &ImageResources::new(),
            &computed,
            &mut ApproximateTextMeasurer,
        );

        let decoration = |color: TextColor| {
            layout
                .box_decorations
                .iter()
                .find(|decoration| decoration.background == color)
                .unwrap()
        };
        let caption = decoration(TextColor {
            red: 255,
            green: 0,
            blue: 0,
            alpha: 255,
        });
        let a = decoration(TextColor {
            red: 0,
            green: 0,
            blue: 255,
            alpha: 255,
        });
        let b = decoration(TextColor {
            red: 0,
            green: 255,
            blue: 0,
            alpha: 255,
        });

        assert!(caption.y < a.y);
        assert_eq!(a.y, b.y);
        assert_eq!(b.x, a.x + a.width);
    }

    #[test]
    fn orphan_columns_feed_width_hints_into_the_repaired_anonymous_table() {
        let document = op_html::parse_document(
            "<style>
               #host { width:420px }
               #col { display:table-column; width:240px }
               .cell { display:table-cell; padding:0 }
               #a { background:#ff0000 }
               #b { background:#0000ff }
             </style>
             <div id='host'>
               <div id='col'></div>
               <div id='a' class='cell'>A</div>
               <div id='b' class='cell'>B</div>
             </div>",
        );
        let computed = compute_styles(&document, &op_css::collect_author_styles(&document).styles);
        let layout = layout_document_with_computed_styles_and_metrics(
            &document,
            800,
            &ImageResources::new(),
            &computed,
            &mut ApproximateTextMeasurer,
        );

        let decoration = |color: TextColor| {
            layout
                .box_decorations
                .iter()
                .find(|decoration| decoration.background == color)
                .unwrap()
        };
        let a = decoration(TextColor {
            red: 255,
            green: 0,
            blue: 0,
            alpha: 255,
        });
        let b = decoration(TextColor {
            red: 0,
            green: 0,
            blue: 255,
            alpha: 255,
        });

        assert!(a.width >= 240);
        assert!(a.width > b.width);
        assert_eq!(b.x, a.x + a.width);
    }

    #[test]
    fn inline_table_vertical_align_top_middle_and_bottom_move_atomic_box_in_line() {
        let document = op_html::parse_document(
            "<style>
               .host { width:300px; font-size:40px; line-height:60px; margin:0 }
               .it { display:inline-table; width:40px; border-spacing:0; font-size:10px; line-height:10px }
               .cell { display:table-cell; padding:0 }
               #top { vertical-align:top; background:#ff0000 }
               #middle { vertical-align:middle; background:#00ff00 }
               #bottom { vertical-align:bottom; background:#0000ff }
             </style>
             <div class='host'>X<span id='top' class='it'><span class='cell'>a</span></span>X</div>
             <div class='host'>X<span id='middle' class='it'><span class='cell'>a</span></span>X</div>
             <div class='host'>X<span id='bottom' class='it'><span class='cell'>a</span></span>X</div>",
        );
        let computed = compute_styles(&document, &op_css::collect_author_styles(&document).styles);
        let layout = layout_document_with_computed_styles_and_metrics(
            &document,
            800,
            &ImageResources::new(),
            &computed,
            &mut ApproximateTextMeasurer,
        );

        let decoration = |color: TextColor| {
            layout
                .box_decorations
                .iter()
                .find(|decoration| decoration.background == color)
                .unwrap()
        };
        let top = decoration(TextColor {
            red: 255,
            green: 0,
            blue: 0,
            alpha: 255,
        });
        let middle = decoration(TextColor {
            red: 0,
            green: 255,
            blue: 0,
            alpha: 255,
        });
        let bottom = decoration(TextColor {
            red: 0,
            green: 0,
            blue: 255,
            alpha: 255,
        });

        let line_tops: Vec<i32> = layout
            .text_boxes
            .iter()
            .filter(|text| text.text == "X")
            .map(|text| text.y)
            .collect();
        assert!(line_tops.len() >= 6);

        let top_relative = top.y - line_tops[0];
        let middle_relative = middle.y - line_tops[2];
        let bottom_relative = bottom.y - line_tops[4];

        assert!(
            top_relative < middle_relative,
            "{top_relative} !< {middle_relative}"
        );
        assert!(
            middle_relative < bottom_relative,
            "{middle_relative} !< {bottom_relative}"
        );
    }

    #[test]
    fn inline_table_box_model_and_margins_contribute_to_atomic_width() {
        let document = op_html::parse_document(
            "<style>
               #host { width:420px; font-size:16px; line-height:20px }
               #it {
                 display:inline-table; width:100px; box-sizing:content-box;
                 padding:4px 10px; border:2px solid #ff0000;
                 margin:3px 7px 5px 6px; border-spacing:0; background:#0000ff
               }
               .cell { display:table-cell; padding:0 }
             </style>
             <div id='host'>left <span id='it'><span class='cell'>A</span></span> right</div>",
        );
        let computed = compute_styles(&document, &op_css::collect_author_styles(&document).styles);
        let layout = layout_document_with_computed_styles_and_metrics(
            &document,
            800,
            &ImageResources::new(),
            &computed,
            &mut ApproximateTextMeasurer,
        );

        let table = layout
            .box_decorations
            .iter()
            .find(|decoration| {
                decoration.background
                    == TextColor {
                        red: 0,
                        green: 0,
                        blue: 255,
                        alpha: 255,
                    }
                    && decoration.border_left.width == 2
            })
            .unwrap();
        let right = layout
            .text_boxes
            .iter()
            .find(|text| text.text.contains("right"))
            .unwrap();

        assert_eq!(table.width, 124);
        assert!(table.height > 8);
        assert!(right.x >= table.x + table.width + 7);
    }

    #[test]
    fn inline_table_is_atomic_and_stays_between_surrounding_text() {
        let document = op_html::parse_document(
            "<style>
               #host { width:420px; font-size:16px; line-height:20px }
               #it { display:inline-table; width:100px; border-spacing:0; background:#ff0000 }
               .row { display:table-row }
               .cell { display:table-cell; padding:0 }
             </style>
             <div id='host'>before <span id='it'><span class='row'><span class='cell'>A</span><span class='cell'>B</span></span></span> after</div>",
        );
        let computed = compute_styles(&document, &op_css::collect_author_styles(&document).styles);
        let layout = layout_document_with_computed_styles_and_metrics(
            &document,
            800,
            &ImageResources::new(),
            &computed,
            &mut ApproximateTextMeasurer,
        );

        let before = layout
            .text_boxes
            .iter()
            .find(|text| text.text.contains("before"))
            .unwrap();
        let a = layout
            .text_boxes
            .iter()
            .find(|text| text.text == "A")
            .unwrap();
        let after = layout
            .text_boxes
            .iter()
            .find(|text| text.text.contains("after"))
            .unwrap();
        let table = layout
            .box_decorations
            .iter()
            .find(|decoration| {
                decoration.background
                    == TextColor {
                        red: 255,
                        green: 0,
                        blue: 0,
                        alpha: 255,
                    }
            })
            .unwrap();

        assert_eq!(before.y, a.y);
        assert_eq!(before.y, after.y);
        assert!(table.x >= before.x + before.width);
        assert!(after.x >= table.x + table.width);
        assert!(table.width >= 100);
    }

    #[test]
    fn inline_table_wraps_as_one_atomic_unit_and_keeps_anchor_linkage() {
        let document = op_html::parse_document(
            "<style>
               #host { width:120px; font-size:16px; line-height:20px }
               #it { display:inline-table; width:80px; border-spacing:0; background:#00ff00 }
               .cell { display:table-cell; padding:0 }
             </style>
             <div id='host'>abcdefgh <a href='next.html'><span id='it'><span class='cell'>cell</span></span></a> tail</div>",
        );
        let computed = compute_styles(&document, &op_css::collect_author_styles(&document).styles);
        let layout = layout_document_with_computed_styles_and_metrics(
            &document,
            800,
            &ImageResources::new(),
            &computed,
            &mut ApproximateTextMeasurer,
        );

        let prefix = layout
            .text_boxes
            .iter()
            .find(|text| text.text.contains("abcdefgh"))
            .unwrap();
        let cell = layout
            .text_boxes
            .iter()
            .find(|text| text.text == "cell")
            .unwrap();
        let table = layout
            .box_decorations
            .iter()
            .find(|decoration| {
                decoration.background
                    == TextColor {
                        red: 0,
                        green: 255,
                        blue: 0,
                        alpha: 255,
                    }
            })
            .unwrap();

        assert!(cell.y > prefix.y);
        assert!(table.y > prefix.y);
        assert_eq!(cell.links.len(), 1);
        assert_eq!(cell.links[0].href, "next.html");
        assert_eq!(cell.links[0].start, 0);
        assert_eq!(cell.links[0].end, cell.text.len());
    }

    #[test]
    fn inline_table_transfers_nested_images_and_outer_anchor_to_final_layout() {
        let document = op_html::parse_document(
            "<style>
               #host { width:300px }
               #it { display:inline-table; width:90px; border-spacing:0; background:#ffff00 }
               .cell { display:table-cell; padding:0 }
               img { width:20px; height:10px }
             </style>
             <div id='host'>x <a href='next.html'><span id='it'><span class='cell'><img id='pic'></span></span></a> y</div>",
        );
        let computed = compute_styles(&document, &op_css::collect_author_styles(&document).styles);
        let image =
            Arc::new(RasterImage::from_premultiplied_bgra(20, 10, vec![255; 20 * 10 * 4]).unwrap());
        let mut images = ImageResources::new();
        let mut nodes = vec![document.root()];
        while let Some(node) = nodes.pop() {
            if document.element(node).is_some_and(|element| {
                element
                    .attributes
                    .iter()
                    .any(|attr| attr.name == "id" && attr.value == "pic")
            }) {
                images.insert(node, image.clone());
            }
            nodes.extend(document.children(node));
        }

        let layout = layout_document_with_computed_styles_and_metrics(
            &document,
            800,
            &images,
            &computed,
            &mut ApproximateTextMeasurer,
        );
        let table = layout
            .box_decorations
            .iter()
            .find(|decoration| {
                decoration.background
                    == TextColor {
                        red: 255,
                        green: 255,
                        blue: 0,
                        alpha: 255,
                    }
            })
            .unwrap();
        assert_eq!(layout.image_boxes.len(), 1);
        let image = &layout.image_boxes[0];
        assert_eq!(image.href.as_deref(), Some("next.html"));
        assert!(image.x >= table.x);
        assert!(image.x + image.width <= table.x + table.width);
        assert!(image.y >= table.y);
        assert!(image.y + image.height <= table.y + table.height);
    }

    #[test]
    fn inline_table_auto_width_shrinks_to_content_instead_of_filling_the_line() {
        let document = op_html::parse_document(
            "<style>
               #host { width:500px; font-size:16px; line-height:20px }
               #it { display:inline-table; border-spacing:0; background:#0000ff }
               .cell { display:table-cell; padding:0 }
             </style>
             <div id='host'>left <span id='it'><span class='cell'>tiny</span></span> right</div>",
        );
        let computed = compute_styles(&document, &op_css::collect_author_styles(&document).styles);
        let layout = layout_document_with_computed_styles_and_metrics(
            &document,
            800,
            &ImageResources::new(),
            &computed,
            &mut ApproximateTextMeasurer,
        );

        let left = layout
            .text_boxes
            .iter()
            .find(|text| text.text.contains("left"))
            .unwrap();
        let right = layout
            .text_boxes
            .iter()
            .find(|text| text.text.contains("right"))
            .unwrap();
        let table = layout
            .box_decorations
            .iter()
            .find(|decoration| {
                decoration.background
                    == TextColor {
                        red: 0,
                        green: 0,
                        blue: 255,
                        alpha: 255,
                    }
            })
            .unwrap();

        assert_eq!(left.y, right.y);
        assert!(
            table.width < 200,
            "auto inline-table should shrink: {table:?}"
        );
        assert!(right.x >= table.x + table.width);
    }

    #[test]
    fn table_border_spacing_controls_real_horizontal_and_vertical_gaps() {
        let document = op_html::parse_document(
            "<style>
               table { width:320px; border-spacing:12px 7px }
               td { padding:0; background:#ff0000 }
               #b { background:#0000ff }
               #c { background:#00ff00 }
             </style>
             <table>
               <tr><td id='a'>A</td><td id='b'>B</td></tr>
               <tr><td id='c'>C</td><td>D</td></tr>
             </table>",
        );
        let computed = compute_styles(&document, &op_css::collect_author_styles(&document).styles);
        let layout = layout_document_with_computed_styles_and_metrics(
            &document,
            800,
            &ImageResources::new(),
            &computed,
            &mut ApproximateTextMeasurer,
        );

        let decoration = |color: TextColor| {
            layout
                .box_decorations
                .iter()
                .find(|decoration| decoration.background == color)
                .unwrap()
        };
        let a = decoration(TextColor {
            red: 255,
            green: 0,
            blue: 0,
            alpha: 255,
        });
        let b = decoration(TextColor {
            red: 0,
            green: 0,
            blue: 255,
            alpha: 255,
        });
        let c = decoration(TextColor {
            red: 0,
            green: 255,
            blue: 0,
            alpha: 255,
        });

        assert_eq!(b.x - (a.x + a.width), 12);
        assert_eq!(c.y - (a.y + a.height), 7);
    }

    #[test]
    fn collapsed_table_mode_removes_separate_border_spacing() {
        let document = op_html::parse_document(
            "<style>
               table { width:320px; border-collapse:collapse; border-spacing:30px 20px }
               td { padding:0; background:#ff0000 }
               #b { background:#0000ff }
               #c { background:#00ff00 }
             </style>
             <table>
               <tr><td id='a'>A</td><td id='b'>B</td></tr>
               <tr><td id='c'>C</td><td>D</td></tr>
             </table>",
        );
        let computed = compute_styles(&document, &op_css::collect_author_styles(&document).styles);
        let layout = layout_document_with_computed_styles_and_metrics(
            &document,
            800,
            &ImageResources::new(),
            &computed,
            &mut ApproximateTextMeasurer,
        );

        let decoration = |color: TextColor| {
            layout
                .box_decorations
                .iter()
                .find(|decoration| decoration.background == color)
                .unwrap()
        };
        let a = decoration(TextColor {
            red: 255,
            green: 0,
            blue: 0,
            alpha: 255,
        });
        let b = decoration(TextColor {
            red: 0,
            green: 0,
            blue: 255,
            alpha: 255,
        });
        let c = decoration(TextColor {
            red: 0,
            green: 255,
            blue: 0,
            alpha: 255,
        });

        assert_eq!(b.x, a.x + a.width);
        assert_eq!(c.y, a.y + a.height);
    }

    #[test]
    fn collapsed_table_borders_choose_one_winning_shared_edge() {
        let document = op_html::parse_document(
            "<style>
               table { width:320px; border-collapse:collapse }
               td { padding:0 }
               #a { background:#ff0000; border-right:2px solid #ff0000; border-bottom:3px solid #ff0000 }
               #b { background:#0000ff; border-left:5px solid #0000ff }
               #c { background:#00ff00; border-top:6px solid #00ff00 }
             </style>
             <table>
               <tr><td id='a'>A</td><td id='b'>B</td></tr>
               <tr><td id='c'>C</td><td>D</td></tr>
             </table>",
        );
        let computed = compute_styles(&document, &op_css::collect_author_styles(&document).styles);
        let layout = layout_document_with_computed_styles_and_metrics(
            &document,
            800,
            &ImageResources::new(),
            &computed,
            &mut ApproximateTextMeasurer,
        );

        let decoration = |color: TextColor| {
            layout
                .box_decorations
                .iter()
                .find(|decoration| decoration.background == color)
                .unwrap()
        };
        let a = decoration(TextColor {
            red: 255,
            green: 0,
            blue: 0,
            alpha: 255,
        });
        let b = decoration(TextColor {
            red: 0,
            green: 0,
            blue: 255,
            alpha: 255,
        });
        let c = decoration(TextColor {
            red: 0,
            green: 255,
            blue: 0,
            alpha: 255,
        });

        assert_eq!(a.border_right.width, 0);
        assert_eq!(b.border_left.width, 5);
        assert_eq!(
            b.border_left.color,
            TextColor {
                red: 0,
                green: 0,
                blue: 255,
                alpha: 255,
            }
        );
        assert_eq!(a.border_bottom.width, 0);
        assert_eq!(c.border_top.width, 6);
        assert_eq!(
            c.border_top.color,
            TextColor {
                red: 0,
                green: 255,
                blue: 0,
                alpha: 255,
            }
        );
    }

    #[test]
    fn table_cell_vertical_align_moves_content_inside_shared_row_height() {
        let document = op_html::parse_document(
            "<style>
               table { width:420px; border-spacing:0 }
               td { height:90px; padding:0 }
               #top { vertical-align:top; background:#ff0000 }
               #middle { vertical-align:middle; background:#0000ff }
               #bottom { vertical-align:bottom; background:#00ff00 }
               #inner { background:#ffff00; padding:1px }
             </style>
             <table><tr>
               <td id='top'>top</td>
               <td id='middle'>middle</td>
               <td id='bottom'><span id='inner'>bottom</span></td>
             </tr></table>",
        );
        let computed = compute_styles(&document, &op_css::collect_author_styles(&document).styles);
        let layout = layout_document_with_computed_styles_and_metrics(
            &document,
            800,
            &ImageResources::new(),
            &computed,
            &mut ApproximateTextMeasurer,
        );

        let text = |value: &str| {
            layout
                .text_boxes
                .iter()
                .find(|text| text.text == value)
                .unwrap()
        };
        let top = text("top");
        let middle = text("middle");
        let bottom = text("bottom");

        assert!(top.y < middle.y);
        assert!(middle.y < bottom.y);
        assert!((middle.y - top.y - (bottom.y - middle.y)).abs() <= 1);

        let cell = |color: TextColor| {
            layout
                .box_decorations
                .iter()
                .find(|decoration| decoration.background == color)
                .unwrap()
        };
        let top_cell = cell(TextColor {
            red: 255,
            green: 0,
            blue: 0,
            alpha: 255,
        });
        let middle_cell = cell(TextColor {
            red: 0,
            green: 0,
            blue: 255,
            alpha: 255,
        });
        let bottom_cell = cell(TextColor {
            red: 0,
            green: 255,
            blue: 0,
            alpha: 255,
        });
        let inner = cell(TextColor {
            red: 255,
            green: 255,
            blue: 0,
            alpha: 255,
        });

        assert_eq!(top_cell.y, middle_cell.y);
        assert_eq!(middle_cell.y, bottom_cell.y);
        assert_eq!(top_cell.height, middle_cell.height);
        assert_eq!(middle_cell.height, bottom_cell.height);
        assert!(inner.y > bottom_cell.y + bottom_cell.height / 2);
    }

    #[test]
    fn table_cell_baseline_aligns_first_line_across_font_sizes() {
        let document = op_html::parse_document(
            "<style>
               table { width:320px; border-spacing:0 }
               td { padding:0; vertical-align:baseline }
               #big { font-size:40px }
               #small { font-size:16px }
             </style>
             <table><tr>
               <td id='big'>BIG</td>
               <td id='small'>small</td>
             </tr></table>",
        );
        let computed = compute_styles(&document, &op_css::collect_author_styles(&document).styles);
        let layout = layout_document_with_computed_styles_and_metrics(
            &document,
            800,
            &ImageResources::new(),
            &computed,
            &mut ApproximateTextMeasurer,
        );

        let big = layout
            .text_boxes
            .iter()
            .find(|text| text.text == "BIG")
            .unwrap();
        let small = layout
            .text_boxes
            .iter()
            .find(|text| text.text == "small")
            .unwrap();

        let big_baseline = big.y + big.font_size * 4 / 5;
        let small_baseline = small.y + small.font_size * 4 / 5;
        assert_eq!(big_baseline, small_baseline);
        assert!(small.y > big.y);
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
