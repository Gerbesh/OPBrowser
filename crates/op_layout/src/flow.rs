use super::inline::{InlineChar, InlineStyle, Item, Lines};
use super::*;
use op_css::{ComputedFontWeight, ComputedStyle, ComputedStyleMap, Display};

pub(super) fn layout(
    document: &Document,
    viewport_width: i32,
    images: &ImageResources,
    computed_styles: &ComputedStyleMap,
    measurer: &mut dyn TextMeasurer,
) -> LayoutTree {
    let viewport_width = viewport_width.max(240);
    let mut stack = vec![document.root()];
    let mut root = document.root();
    while let Some(id) = stack.pop() {
        if document.element(id).is_some_and(|e| e.tag_name == "body") {
            root = id;
            break;
        }
        stack.extend(document.children(id).iter().rev());
    }

    let mut context = Context {
        document,
        images,
        computed_styles,
        measurer,
        width: (viewport_width - 64).max(160),
        y: 28,
        text: Vec::new(),
        images_out: Vec::new(),
        order: Vec::new(),
    };

    let root_style = document
        .element(root)
        .map_or_else(default_style, |element| {
            context.element_style(root, element.tag_name.as_str(), default_style())
        });
    if document.element(root).is_none_or(|element| {
        context.element_display(root, element.tag_name.as_str()) != Display::None
    }) {
        context.block(root, None, root_style);
    }

    LayoutTree {
        viewport_width,
        content_height: context.y + 24,
        text_boxes: context.text,
        image_boxes: context.images_out,
        order: context.order,
    }
}

struct Context<'a, 'm> {
    document: &'a Document,
    images: &'a ImageResources,
    computed_styles: &'a ComputedStyleMap,
    measurer: &'m mut dyn TextMeasurer,
    width: i32,
    y: i32,
    text: Vec<TextBox>,
    images_out: Vec<ImageBox>,
    order: Vec<LayoutItem>,
}

#[derive(Clone, Copy)]
struct Style {
    inline: InlineStyle,
    top: i32,
    bottom: i32,
}

impl<'a> Context<'a, '_> {
    fn emit(&mut self, items: &mut Vec<Item<'a>>, style: Style) {
        let lines = Lines::new(self.measurer, style.inline, 32, self.y, self.width)
            .layout(std::mem::take(items));
        self.y = lines.bottom();
        self.order
            .extend(lines.order.into_iter().map(|item| match item {
                LayoutItem::Text(index) => LayoutItem::Text(index + self.text.len()),
                LayoutItem::Image(index) => LayoutItem::Image(index + self.images_out.len()),
            }));
        self.text.extend(lines.text_boxes);
        self.images_out.extend(lines.image_boxes);
    }

    fn block(&mut self, id: NodeId, href: Option<&'a str>, style: Style) {
        let start = self.y;
        self.y += style.top;
        let mut items = Vec::new();
        for child in self.document.children(id) {
            self.collect(*child, href, style, &mut items);
        }
        self.emit(&mut items, style);
        if self.y > start + style.top {
            self.y += style.bottom;
        } else {
            self.y = start;
        }
    }

    fn collect(
        &mut self,
        id: NodeId,
        href: Option<&'a str>,
        inherited: Style,
        items: &mut Vec<Item<'a>>,
    ) {
        let Some(node) = self.document.node(id) else {
            return;
        };
        match &node.kind {
            NodeKind::Text(text) => items.extend(text.chars().map(|ch| {
                Item::Char(InlineChar {
                    ch,
                    href,
                    style: inherited.inline,
                })
            })),
            NodeKind::Element(element) => {
                let tag = element.tag_name.as_str();
                let display = self.element_display(id, tag);
                if display == Display::None {
                    return;
                }
                let current = self.element_style(id, tag, inherited);
                let href = if tag == "a" {
                    attribute(element, "href")
                } else {
                    href
                };

                if tag == "img" {
                    if display == Display::Block {
                        self.emit(items, inherited);
                        let mut image_items = Vec::new();
                        self.collect_image(id, element, href, current, &mut image_items);
                        self.emit(&mut image_items, current);
                    } else {
                        self.collect_image(id, element, href, current, items);
                    }
                } else if tag == "br" {
                    if display == Display::Block {
                        self.emit(items, inherited);
                        let mut line = vec![Item::Break];
                        self.emit(&mut line, current);
                    } else {
                        items.push(Item::Break);
                    }
                } else if display == Display::Block {
                    self.emit(items, inherited);
                    self.block(id, href, current);
                } else {
                    for child in &node.children {
                        self.collect(*child, href, current, items);
                    }
                }
            }
            NodeKind::Document => {
                for child in &node.children {
                    self.collect(*child, href, inherited, items);
                }
            }
        }
    }

    fn collect_image(
        &mut self,
        id: NodeId,
        element: &'a op_dom::ElementData,
        href: Option<&'a str>,
        style: Style,
        items: &mut Vec<Item<'a>>,
    ) {
        let width = dimension(element, "width");
        let height = dimension(element, "height");
        if width == Some(0) || height == Some(0) {
            return;
        }
        if let Some(image) = self.images.get(&id) {
            let mut width = width.unwrap_or_else(|| {
                height.map_or(image.width(), |h| {
                    (h * image.width() / image.height()).max(1)
                })
            });
            let mut height =
                height.unwrap_or_else(|| (width * image.height() / image.width()).max(1));
            if width > self.width as u32 {
                height = (u64::from(height) * self.width as u64 / u64::from(width)).max(1) as u32;
                width = self.width as u32;
            }
            if height > op_image::MAX_DIMENSION {
                width = (u64::from(width) * u64::from(op_image::MAX_DIMENSION) / u64::from(height))
                    .max(1) as u32;
                height = op_image::MAX_DIMENSION;
            }
            items.push(Item::Image(ImageBox {
                x: 0,
                y: 0,
                width: width as i32,
                height: height as i32,
                image: image.clone(),
                href: href.map(str::to_owned),
            }));
        } else {
            items.extend(
                attribute(element, "alt")
                    .unwrap_or("[image]")
                    .chars()
                    .map(|ch| {
                        Item::Char(InlineChar {
                            ch,
                            href,
                            style: style.inline,
                        })
                    }),
            );
        }
    }

    fn element_display(&self, id: NodeId, tag: &str) -> Display {
        self.computed_styles
            .style_for(id)
            .map_or_else(|| fallback_display(tag), |style| style.display)
    }

    fn element_style(&self, id: NodeId, tag: &str, inherited: Style) -> Style {
        let (top, bottom) = spacing(tag);
        let inline = self
            .computed_styles
            .style_for(id)
            .copied()
            .map(computed_inline_style)
            .unwrap_or_else(|| fallback_inline_style(tag, inherited.inline));
        Style {
            inline,
            top,
            bottom,
        }
    }
}

fn computed_inline_style(style: ComputedStyle) -> InlineStyle {
    InlineStyle {
        font_size: style.font_size_px.round().clamp(1.0, 4096.0) as i32,
        weight: match style.font_weight {
            ComputedFontWeight::Normal => FontWeight::Normal,
            ComputedFontWeight::Bold => FontWeight::Bold,
        },
        color: style.color.into(),
    }
}

fn fallback_inline_style(tag: &str, inherited: InlineStyle) -> InlineStyle {
    match tag {
        "h1" => InlineStyle {
            font_size: 34,
            weight: FontWeight::Bold,
            ..inherited
        },
        "h2" => InlineStyle {
            font_size: 28,
            weight: FontWeight::Bold,
            ..inherited
        },
        "h3" => InlineStyle {
            font_size: 23,
            weight: FontWeight::Bold,
            ..inherited
        },
        "h4" | "h5" | "h6" => InlineStyle {
            font_size: 18,
            weight: FontWeight::Bold,
            ..inherited
        },
        _ => inherited,
    }
}

fn fallback_display(tag: &str) -> Display {
    if hidden_tag(tag) {
        Display::None
    } else if is_block(tag) {
        Display::Block
    } else {
        Display::Inline
    }
}

fn default_style() -> Style {
    Style {
        inline: InlineStyle {
            font_size: 18,
            weight: FontWeight::Normal,
            color: TextColor {
                red: 0,
                green: 0,
                blue: 0,
                alpha: 255,
            },
        },
        top: 0,
        bottom: 0,
    }
}

fn spacing(tag: &str) -> (i32, i32) {
    match tag {
        "h1" => (8, 16),
        "h2" => (8, 14),
        "h3" => (6, 12),
        "h4" | "h5" | "h6" => (6, 12),
        "p" | "li" => (0, 12),
        _ => (0, 0),
    }
}

fn attribute<'a>(element: &'a op_dom::ElementData, name: &str) -> Option<&'a str> {
    element
        .attributes
        .iter()
        .find(|a| a.name == name)
        .map(|a| a.value.as_str())
}

fn dimension(element: &op_dom::ElementData, name: &str) -> Option<u32> {
    attribute(element, name)?
        .trim()
        .parse::<u32>()
        .ok()
        .filter(|n| *n <= op_image::MAX_DIMENSION)
}

fn hidden_tag(tag: &str) -> bool {
    matches!(
        tag,
        "head" | "title" | "style" | "script" | "meta" | "link" | "template"
    )
}

fn is_block(tag: &str) -> bool {
    matches!(
        tag,
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
            | "p"
            | "li"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "pre"
            | "address"
            | "dl"
            | "dt"
            | "dd"
            | "figure"
            | "figcaption"
    )
}
