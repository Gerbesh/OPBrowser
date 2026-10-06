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
