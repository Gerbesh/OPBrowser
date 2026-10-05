use super::inline::{InlineChar, Item, Lines};
use super::*;

pub(super) fn layout(
    document: &Document,
    viewport_width: i32,
    images: &ImageResources,
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
        measurer,
        width: (viewport_width - 64).max(160),
        y: 28,
        text: Vec::new(),
        images_out: Vec::new(),
        order: Vec::new(),
    };
    context.block(root, None, style(""));
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
    measurer: &'m mut dyn TextMeasurer,
    width: i32,
    y: i32,
    text: Vec<TextBox>,
    images_out: Vec<ImageBox>,
    order: Vec<LayoutItem>,
}

#[derive(Clone, Copy)]
struct Style {
    size: i32,
    weight: FontWeight,
    top: i32,
    bottom: i32,
}

impl<'a> Context<'a, '_> {
    fn emit(&mut self, items: &mut Vec<Item<'a>>, style: Style) {
        let lines = Lines::new(
            self.measurer,
            style.size,
            style.weight,
            32,
            self.y,
            self.width,
        )
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
            NodeKind::Text(text) => {
                items.extend(text.chars().map(|ch| Item::Char(InlineChar { ch, href })))
            }
            NodeKind::Element(element) => {
                let tag = element.tag_name.as_str();
                if matches!(
                    tag,
                    "head" | "title" | "style" | "script" | "meta" | "link" | "template"
                ) {
                    return;
                }
                let href = if tag == "a" {
                    attribute(element, "href")
                } else {
                    href
                };
                if is_block(tag) {
                    self.emit(items, inherited);
                    self.block(id, href, style(tag));
                } else if tag == "br" {
                    items.push(Item::Break);
                } else if tag == "img" {
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
                        let mut height = height
                            .unwrap_or_else(|| (width * image.height() / image.width()).max(1));
                        if width > self.width as u32 {
                            height = (u64::from(height) * self.width as u64 / u64::from(width))
                                .max(1) as u32;
                            width = self.width as u32;
                        }
                        if height > op_image::MAX_DIMENSION {
                            width = (u64::from(width) * u64::from(op_image::MAX_DIMENSION)
                                / u64::from(height))
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
                                .map(|ch| Item::Char(InlineChar { ch, href })),
                        );
                    }
                } else {
                    for child in &node.children {
                        self.collect(*child, href, inherited, items);
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

fn style(tag: &str) -> Style {
    match tag {
        "h1" => Style {
            size: 34,
            weight: FontWeight::Bold,
            top: 8,
            bottom: 16,
        },
        "h2" => Style {
            size: 28,
            weight: FontWeight::Bold,
            top: 8,
            bottom: 14,
        },
        "h3" => Style {
            size: 23,
            weight: FontWeight::Bold,
            top: 6,
            bottom: 12,
        },
        "h4" | "h5" | "h6" => Style {
            size: 18,
            weight: FontWeight::Bold,
            top: 6,
            bottom: 12,
        },
        "p" | "li" => Style {
            size: 18,
            weight: FontWeight::Normal,
            top: 0,
            bottom: 12,
        },
        _ => Style {
            size: 18,
            weight: FontWeight::Normal,
            top: 0,
            bottom: 0,
        },
    }
}
