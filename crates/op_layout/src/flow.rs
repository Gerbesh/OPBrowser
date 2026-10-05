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
        decorations: Vec::new(),
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
        context.block(root, None, root_style, 32, context.width);
    }

    LayoutTree {
        viewport_width,
        content_height: context.y + 24,
        box_decorations: context.decorations,
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
    decorations: Vec<BoxDecoration>,
    text: Vec<TextBox>,
    images_out: Vec<ImageBox>,
    order: Vec<LayoutItem>,
}

#[derive(Clone, Copy)]
struct Edges {
    top: i32,
    right: i32,
    bottom: i32,
    left: i32,
}

impl Edges {
    const ZERO: Self = Self {
        top: 0,
        right: 0,
        bottom: 0,
        left: 0,
    };
}

#[derive(Clone, Copy)]
struct Style {
    inline: InlineStyle,
    margin: Edges,
    padding: Edges,
    background: TextColor,
    border_width: i32,
    border_color: TextColor,
}

impl<'a> Context<'a, '_> {
    fn emit(&mut self, items: &mut Vec<Item<'a>>, style: Style, x: i32, width: i32) {
        let lines = Lines::new(self.measurer, style.inline, x, self.y, width.max(1))
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

    fn block(
        &mut self,
        id: NodeId,
        href: Option<&'a str>,
        style: Style,
        containing_x: i32,
        containing_width: i32,
    ) {
        self.y = self.y.saturating_add(style.margin.top);
        let border_x = containing_x.saturating_add(style.margin.left);
        let border_width = containing_width
            .saturating_sub(style.margin.left)
            .saturating_sub(style.margin.right)
            .max(1);
        let border = style.border_width.clamp(0, border_width / 2);
        let border_y = self.y;
        let content_x = border_x
            .saturating_add(border)
            .saturating_add(style.padding.left);
        let content_width = border_width
            .saturating_sub(border.saturating_mul(2))
            .saturating_sub(style.padding.left)
            .saturating_sub(style.padding.right)
            .max(1);

        let decoration = if style.background.alpha > 0 || border > 0 {
            let index = self.decorations.len();
            self.decorations.push(BoxDecoration {
                x: border_x,
                y: border_y,
                width: border_width,
                height: 0,
                background: style.background,
                border_width: border,
                border_color: style.border_color,
            });
            Some(index)
        } else {
            None
        };

        self.y = self
            .y
            .saturating_add(border)
            .saturating_add(style.padding.top);
        let mut items = Vec::new();
        for child in self.document.children(id) {
            self.collect(*child, href, style, content_x, content_width, &mut items);
        }
        self.emit(&mut items, style, content_x, content_width);
        self.y = self
            .y
            .saturating_add(style.padding.bottom)
            .saturating_add(border);

        if let Some(index) = decoration {
            self.decorations[index].height = self.y.saturating_sub(border_y).max(0);
        }
        self.y = self.y.saturating_add(style.margin.bottom);
    }

    fn collect(
        &mut self,
        id: NodeId,
        href: Option<&'a str>,
        inherited: Style,
        containing_x: i32,
        containing_width: i32,
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
                        self.emit(items, inherited, containing_x, containing_width);
                        let mut image_items = Vec::new();
                        self.collect_image(
                            id,
                            element,
                            href,
                            current,
                            containing_width,
                            &mut image_items,
                        );
                        self.emit(&mut image_items, current, containing_x, containing_width);
                    } else {
                        self.collect_image(id, element, href, current, containing_width, items);
                    }
                } else if tag == "br" {
                    if display == Display::Block {
                        self.emit(items, inherited, containing_x, containing_width);
                        let mut line = vec![Item::Break];
                        self.emit(&mut line, current, containing_x, containing_width);
                    } else {
                        items.push(Item::Break);
                    }
                } else if display == Display::Block {
                    self.emit(items, inherited, containing_x, containing_width);
                    self.block(id, href, current, containing_x, containing_width);
                } else {
                    for child in &node.children {
                        self.collect(*child, href, current, containing_x, containing_width, items);
                    }
                }
            }
            NodeKind::Document => {
                for child in &node.children {
                    self.collect(
                        *child,
                        href,
                        inherited,
                        containing_x,
                        containing_width,
                        items,
                    );
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
        available_width: i32,
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
            let available_width = available_width.max(1) as u32;
            if width > available_width {
                height = (u64::from(height) * u64::from(available_width) / u64::from(width)).max(1)
                    as u32;
                width = available_width;
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
        self.computed_styles
            .style_for(id)
            .copied()
            .map(computed_style)
            .unwrap_or_else(|| fallback_style(tag, inherited))
    }
}

fn computed_style(style: ComputedStyle) -> Style {
    Style {
        inline: InlineStyle {
            font_size: style.font_size_px.round().clamp(1.0, 4096.0) as i32,
            weight: match style.font_weight {
                ComputedFontWeight::Normal => FontWeight::Normal,
                ComputedFontWeight::Bold => FontWeight::Bold,
            },
            color: style.color.into(),
        },
        margin: computed_edges(style.margin),
        padding: computed_edges(style.padding),
        background: style.background_color.into(),
        border_width: if style.border.solid {
            style.border.width_px.round().clamp(0.0, 4096.0) as i32
        } else {
            0
        },
        border_color: style.border.color.into(),
    }
}

fn computed_edges(edges: op_css::BoxEdges) -> Edges {
    let px = |value: f32| value.round().clamp(0.0, 4096.0) as i32;
    Edges {
        top: px(edges.top),
        right: px(edges.right),
        bottom: px(edges.bottom),
        left: px(edges.left),
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
        margin: Edges::ZERO,
        padding: Edges::ZERO,
        background: TextColor {
            red: 0,
            green: 0,
            blue: 0,
            alpha: 0,
        },
        border_width: 0,
        border_color: TextColor {
            red: 0,
            green: 0,
            blue: 0,
            alpha: 255,
        },
    }
}

fn fallback_style(tag: &str, inherited: Style) -> Style {
    let (top, bottom) = match tag {
        "h1" => (8, 16),
        "h2" => (8, 14),
        "h3" => (6, 12),
        "h4" | "h5" | "h6" => (6, 12),
        "p" | "li" => (0, 12),
        _ => (0, 0),
    };
    Style {
        inline: fallback_inline_style(tag, inherited.inline),
        margin: Edges {
            top,
            right: 0,
            bottom,
            left: 0,
        },
        padding: Edges::ZERO,
        background: default_style().background,
        border_width: 0,
        border_color: default_style().border_color,
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
