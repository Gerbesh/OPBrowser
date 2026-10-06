use super::inline::{InlineBoxStyle, InlineBoxes, InlineChar, InlineStyle, Item, Lines};
use super::*;
use op_css::{
    BorderEdges, BorderStyle, BoxSizing, ComputedFontWeight, ComputedLineHeight, ComputedStyle,
    ComputedStyleMap, Display, FontStyle as CssFontStyle, LengthPercentage, MarginEdges,
    MarginValue, PaddingEdges, PseudoElement, TextAlign, TextTransform, WhiteSpace,
};

pub(super) fn layout(
    document: &Document,
    viewport_width: i32,
    images: &ImageResources,
    generated_images: &GeneratedImageResources,
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
        generated_images,
        computed_styles,
        measurer,
        inline_boxes: InlineBoxes::default(),
        width: (viewport_width - 64).max(160),
        y: 28,
        pending_margin: None,
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
        context.block(
            BlockContent::Element(root),
            None,
            root_style,
            32,
            context.width,
        );
        context.flush_pending_margin();
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
    generated_images: &'a GeneratedImageResources,
    computed_styles: &'a ComputedStyleMap,
    measurer: &'m mut dyn TextMeasurer,
    inline_boxes: InlineBoxes,
    width: i32,
    y: i32,
    pending_margin: Option<i32>,
    decorations: Vec<BoxDecoration>,
    text: Vec<TextBox>,
    images_out: Vec<ImageBox>,
    order: Vec<LayoutItem>,
}

/// Generated blocks share ordinary block sizing without adding synthetic DOM nodes.
enum BlockContent {
    Element(NodeId),
    Generated(NodeId, PseudoElement),
}

#[derive(Clone, Copy)]
struct Edges {
    top: i32,
    right: i32,
    bottom: i32,
    left: i32,
}

#[derive(Clone, Copy)]
struct UsedBorderSide {
    width: i32,
    color: TextColor,
}

#[derive(Clone, Copy)]
struct UsedBorderEdges {
    top: UsedBorderSide,
    right: UsedBorderSide,
    bottom: UsedBorderSide,
    left: UsedBorderSide,
}

#[derive(Clone, Copy)]
struct UsedBlockHorizontal {
    margin_left: i32,
    padding: Edges,
    border: UsedBorderEdges,
    content_width: i32,
    border_width: i32,
}

#[derive(Clone, Copy)]
struct Style {
    inline: InlineStyle,
    text_align: TextAlign,
    margin: MarginEdges,
    padding: PaddingEdges,
    background: TextColor,
    border: BorderEdges,
    width: Option<LengthPercentage>,
    min_width: LengthPercentage,
    max_width: Option<LengthPercentage>,
    height: Option<LengthPercentage>,
    min_height: LengthPercentage,
    max_height: Option<LengthPercentage>,
    box_sizing: BoxSizing,
}

impl<'a> Context<'a, '_> {
    fn emit(&mut self, items: &mut Vec<Item<'a>>, style: Style, x: i32, width: i32) {
        if items.iter().all(|item| {
            matches!(
                item,
                Item::Char(ch)
                    if matches!(ch.ch, ' ' | '\t' | '\n' | '\r' | '\u{c}')
                        && matches!(ch.style.white_space, WhiteSpace::Normal | WhiteSpace::NoWrap)
            )
        }) {
            items.clear();
            return;
        }
        self.flush_pending_margin();
        let lines = Lines::new(
            self.measurer,
            &self.inline_boxes,
            style.inline,
            style.text_align,
            x,
            self.y,
            width.max(1),
        )
        .layout(std::mem::take(items));
        self.y = lines.bottom();
        self.order
            .extend(lines.order.into_iter().map(|item| match item {
                LayoutItem::Text(index) => LayoutItem::Text(index + self.text.len()),
                LayoutItem::Image(index) => LayoutItem::Image(index + self.images_out.len()),
            }));
        self.decorations.extend(lines.decorations);
        self.text.extend(lines.text_boxes);
        self.images_out.extend(lines.image_boxes);
    }

    fn block(
        &mut self,
        content: BlockContent,
        href: Option<&'a str>,
        style: Style,
        containing_x: i32,
        containing_width: i32,
    ) {
        let used = resolve_block_horizontal(style, containing_width);
        let margin_top = resolve_vertical_margin(style.margin.top, containing_width);
        let margin_bottom = resolve_vertical_margin(style.margin.bottom, containing_width);
        let padding_top = used.padding.top;
        let padding_bottom = used.padding.bottom;

        self.apply_collapsed_margin(margin_top);
        let border_x = containing_x.saturating_add(used.margin_left);
        let border_y = self.y;
        let content_x = border_x
            .saturating_add(used.border.left.width)
            .saturating_add(used.padding.left);
        let content_width = used.content_width.max(1);

        let has_border = used.border.top.width > 0
            || used.border.right.width > 0
            || used.border.bottom.width > 0
            || used.border.left.width > 0;
        let decoration = if style.background.alpha > 0 || has_border {
            let index = self.decorations.len();
            self.decorations.push(BoxDecoration {
                x: border_x,
                y: border_y,
                width: used.border_width,
                height: 0,
                background: style.background,
                border_top: DecorationBorder {
                    width: used.border.top.width,
                    color: used.border.top.color,
                },
                border_right: DecorationBorder {
                    width: used.border.right.width,
                    color: used.border.right.color,
                },
                border_bottom: DecorationBorder {
                    width: used.border.bottom.width,
                    color: used.border.bottom.color,
                },
                border_left: DecorationBorder {
                    width: used.border.left.width,
                    color: used.border.left.color,
                },
            });
            Some(index)
        } else {
            None
        };

        self.y = self
            .y
            .saturating_add(used.border.top.width)
            .saturating_add(padding_top);
        let content_top = self.y;

        let mut items = Vec::new();
        match content {
            BlockContent::Element(id) => {
                self.collect_generated(
                    id,
                    PseudoElement::Before,
                    href,
                    style,
                    (content_x, content_width),
                    &mut items,
                );
                for child in self.document.children(id) {
                    self.collect(*child, href, style, content_x, content_width, &mut items);
                }
                self.collect_generated(
                    id,
                    PseudoElement::After,
                    href,
                    style,
                    (content_x, content_width),
                    &mut items,
                );
            }
            BlockContent::Generated(id, pseudo) => {
                self.collect_generated_items((id, pseudo), href, style, content_width, &mut items)
            }
        }
        self.emit(&mut items, style, content_x, content_width);
        // Parent/child margin collapse is intentionally deferred; consume the final
        // child margin before this block's padding/border boundary.
        self.flush_pending_margin();

        let natural_content_height = self.y.saturating_sub(content_top).max(0);
        let target_content_height = resolve_block_content_height(
            style,
            natural_content_height,
            padding_top,
            padding_bottom,
            used.border,
        );
        // Definite height/min/max controls the box even when its text overflows.
        self.y = content_top.saturating_add(target_content_height);

        self.y = self
            .y
            .saturating_add(padding_bottom)
            .saturating_add(used.border.bottom.width);

        if let Some(index) = decoration {
            self.decorations[index].height = self.y.saturating_sub(border_y).max(0);
        }
        self.pending_margin = Some(margin_bottom);
    }

    fn apply_collapsed_margin(&mut self, next: i32) {
        let used = self
            .pending_margin
            .take()
            .map_or(next, |previous| collapse_margins(previous, next));
        self.y = self.y.saturating_add(used);
    }

    fn flush_pending_margin(&mut self) {
        if let Some(margin) = self.pending_margin.take() {
            self.y = self.y.saturating_add(margin);
        }
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
                let mut current = self.element_style(id, tag, inherited);
                if display == Display::Inline {
                    current.inline.boxes = if tag == "img" {
                        inherited.inline.boxes
                    } else {
                        resolve_inline_box_style(id, None, current, containing_width)
                            .map(|box_style| {
                                self.inline_boxes.push(box_style, inherited.inline.boxes)
                            })
                            .or(inherited.inline.boxes)
                    };
                }
                let href = if tag == "a" {
                    attribute(element, "href")
                } else {
                    href
                };

                if tag == "img" {
                    if display == Display::Block {
                        self.emit(items, inherited, containing_x, containing_width);
                        if let Some(image) = self.images.get(&id).cloned() {
                            self.block_image(
                                (id, None),
                                href,
                                current,
                                &image,
                                (containing_x, containing_width),
                            );
                            return;
                        }
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
                    self.block(
                        BlockContent::Element(id),
                        href,
                        current,
                        containing_x,
                        containing_width,
                    );
                } else {
                    self.collect_generated(
                        id,
                        PseudoElement::Before,
                        href,
                        current,
                        (containing_x, containing_width),
                        items,
                    );
                    if node.children.is_empty()
                        && self
                            .computed_styles
                            .pseudo_style_for(id, PseudoElement::Before)
                            .is_none()
                        && self
                            .computed_styles
                            .pseudo_style_for(id, PseudoElement::After)
                            .is_none()
                        && current.inline.boxes.is_some_and(|box_id| {
                            let box_style = self.inline_boxes.style(box_id);
                            box_style.node == id && box_style.pseudo.is_none()
                        })
                    {
                        items.push(Item::EmptyInline(current.inline));
                    }
                    for child in &node.children {
                        self.collect(*child, href, current, containing_x, containing_width, items);
                    }
                    self.collect_generated(
                        id,
                        PseudoElement::After,
                        href,
                        current,
                        (containing_x, containing_width),
                        items,
                    );
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

    fn collect_generated(
        &mut self,
        id: NodeId,
        pseudo: PseudoElement,
        href: Option<&'a str>,
        host_style: Style,
        containing: (i32, i32),
        items: &mut Vec<Item<'a>>,
    ) {
        let (containing_x, containing_width) = containing;
        let Some(generated) = self.computed_styles.pseudo_style_for(id, pseudo) else {
            return;
        };
        if generated.style.display == Display::None {
            return;
        }

        let mut style = computed_style(generated.style);
        if generated.replaced_image
            && generated.style.display == Display::Block
            && let Some(image) = self.generated_images.get(&(id, pseudo, 0)).cloned()
        {
            self.emit(items, host_style, containing_x, containing_width);
            self.block_image((id, Some(pseudo)), href, style, &image, containing);
            return;
        }
        if generated.style.display == Display::Block {
            self.emit(items, host_style, containing_x, containing_width);
            self.block(
                BlockContent::Generated(id, pseudo),
                href,
                style,
                containing_x,
                containing_width,
            );
            return;
        }
        style.inline.boxes = resolve_inline_box_style(id, Some(pseudo), style, containing_width)
            .map(|box_style| self.inline_boxes.push(box_style, host_style.inline.boxes))
            .or(host_style.inline.boxes);
        if generated.items.is_empty() {
            if style.inline.boxes.is_some() {
                items.push(Item::EmptyInline(style.inline));
            }
            return;
        }
        self.collect_generated_items((id, pseudo), href, style, containing_width, items);
    }

    fn collect_generated_items(
        &self,
        target: (NodeId, PseudoElement),
        href: Option<&'a str>,
        style: Style,
        containing_width: i32,
        items: &mut Vec<Item<'a>>,
    ) {
        let Some(generated) = self.computed_styles.pseudo_style_for(target.0, target.1) else {
            return;
        };
        for (index, item) in generated.items.iter().enumerate() {
            match item {
                op_css::GeneratedContentItem::Text(text) => items.extend(text.chars().map(|ch| {
                    Item::Char(InlineChar {
                        ch,
                        href,
                        style: style.inline,
                    })
                })),
                op_css::GeneratedContentItem::Image { .. } => {
                    if let Some(image) = self.generated_images.get(&(target.0, target.1, index)) {
                        let mut image_style = style.inline;
                        let mut own_box = None;
                        let sizing = if generated.replaced_image
                            && generated.style.display == Display::Inline
                        {
                            own_box = resolve_inline_box_style(
                                target.0,
                                Some(target.1),
                                style,
                                containing_width,
                            );
                            if own_box.is_some()
                                && let Some(box_id) = image_style.boxes
                            {
                                image_style.boxes = self.inline_boxes.parent(box_id);
                            }
                            style
                        } else {
                            default_style()
                        };
                        let Some((width, height)) = resolve_image_size(
                            sizing,
                            image,
                            own_box,
                            containing_width,
                            containing_width
                                .saturating_sub(self.inline_boxes.horizontal(image_style.boxes)),
                        ) else {
                            continue;
                        };
                        items.push(Item::Image(
                            ImageBox {
                                x: 0,
                                y: 0,
                                width,
                                height,
                                image: image.clone(),
                                href: href.map(str::to_owned),
                            },
                            image_style,
                            own_box,
                        ));
                    }
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
        if let Some(image) = self.images.get(&id) {
            let image_style = style.inline;
            let own_box = resolve_inline_box_style(id, None, style, available_width);
            let Some((width, height)) = resolve_image_size(
                style,
                image,
                own_box,
                available_width,
                available_width.saturating_sub(self.inline_boxes.horizontal(image_style.boxes)),
            ) else {
                return;
            };
            items.push(Item::Image(
                ImageBox {
                    x: 0,
                    y: 0,
                    width,
                    height,
                    image: image.clone(),
                    href: href.map(str::to_owned),
                },
                image_style,
                own_box,
            ));
        } else {
            if style
                .width
                .is_some_and(|value| resolve_length(value, available_width) == 0)
                || matches!(style.height, Some(LengthPercentage::Px(0.0)))
            {
                return;
            }
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

    fn block_image(
        &mut self,
        target: (NodeId, Option<PseudoElement>),
        href: Option<&str>,
        style: Style,
        image: &Arc<RasterImage>,
        containing: (i32, i32),
    ) {
        let (containing_x, containing_width) = containing;
        let box_style = resolve_inline_box_style(target.0, target.1, style, containing_width);
        let left = box_style.map_or(0, InlineBoxStyle::left_extra);
        let right = box_style.map_or(0, InlineBoxStyle::right_extra);
        let top = box_style.map_or(0, InlineBoxStyle::top_extra);
        let bottom = box_style.map_or(0, InlineBoxStyle::bottom_extra);
        let margin_left = resolve_margin(style.margin.left, containing_width);
        let margin_right = resolve_margin(style.margin.right, containing_width);
        let fit_width = containing_width
            .saturating_sub(margin_left.unwrap_or(0))
            .saturating_sub(margin_right.unwrap_or(0));
        let Some((width, height)) =
            resolve_image_size(style, image, box_style, containing_width, fit_width)
        else {
            return;
        };
        let outer_width = width.saturating_add(left).saturating_add(right);
        let outer_height = height.saturating_add(top).saturating_add(bottom);
        let remaining = containing_width.saturating_sub(outer_width);
        let used_left = match (margin_left, margin_right) {
            (None, None) => remaining.max(0) / 2,
            (None, Some(right)) => remaining.saturating_sub(right).max(0),
            (Some(left), _) => left,
        };
        self.apply_collapsed_margin(resolve_vertical_margin(style.margin.top, containing_width));
        let x = containing_x.saturating_add(used_left);
        if let Some(box_style) = box_style {
            self.decorations.push(BoxDecoration {
                x,
                y: self.y,
                width: outer_width,
                height: outer_height,
                background: box_style.background,
                border_top: box_style.border_top,
                border_right: box_style.border_right,
                border_bottom: box_style.border_bottom,
                border_left: box_style.border_left,
            });
        }
        self.order.push(LayoutItem::Image(self.images_out.len()));
        self.images_out.push(ImageBox {
            x: x.saturating_add(left),
            y: self.y.saturating_add(top),
            width,
            height,
            image: image.clone(),
            href: href.map(str::to_owned),
        });
        self.y = self.y.saturating_add(outer_height);
        self.pending_margin = Some(resolve_vertical_margin(
            style.margin.bottom,
            containing_width,
        ));
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

fn resolve_image_size(
    style: Style,
    image: &RasterImage,
    box_style: Option<InlineBoxStyle>,
    available_width: i32,
    fit_width: i32,
) -> Option<(i32, i32)> {
    let horizontal_extras = box_style.map_or(0, |value| {
        value.left_extra().saturating_add(value.right_extra())
    });
    let vertical_extras = box_style.map_or(0, |value| {
        value.top_extra().saturating_add(value.bottom_extra())
    });
    let width_value =
        |value| size_to_content_width(value, style.box_sizing, available_width, horizontal_extras);
    let height_value = |value| match value {
        LengthPercentage::Px(_) => Some(size_to_content_width(
            value,
            style.box_sizing,
            0,
            vertical_extras,
        )),
        LengthPercentage::Percent(_) => None,
    };
    let (width, height) = super::replaced::dimensions(
        (image.width(), image.height()),
        style.width.map(width_value),
        style.height.and_then(height_value),
        (
            width_value(style.min_width),
            style.max_width.map(width_value),
        ),
        (
            height_value(style.min_height).unwrap_or(0),
            style.max_height.and_then(height_value),
        ),
    );
    if width == 0 || height == 0 {
        return None;
    }
    let (mut width, mut height) = (width as u32, height as u32);
    let available_width = fit_width.saturating_sub(horizontal_extras).max(1) as u32;
    if width > available_width {
        height = (u64::from(height) * u64::from(available_width) / u64::from(width)).max(1) as u32;
        width = available_width;
    }
    if height > op_image::MAX_DIMENSION {
        width = (u64::from(width) * u64::from(op_image::MAX_DIMENSION) / u64::from(height)).max(1)
            as u32;
        height = op_image::MAX_DIMENSION;
    }
    Some((width as i32, height as i32))
}

fn resolve_inline_box_style(
    node: NodeId,
    pseudo: Option<PseudoElement>,
    style: Style,
    containing_width: i32,
) -> Option<InlineBoxStyle> {
    let padding_top = resolve_length(style.padding.top, containing_width).max(0);
    let padding_right = resolve_length(style.padding.right, containing_width).max(0);
    let padding_bottom = resolve_length(style.padding.bottom, containing_width).max(0);
    let padding_left = resolve_length(style.padding.left, containing_width).max(0);
    let border = resolve_border_edges(style.border);
    let visible = style.background.alpha > 0
        || padding_top > 0
        || padding_right > 0
        || padding_bottom > 0
        || padding_left > 0
        || border.top.width > 0
        || border.right.width > 0
        || border.bottom.width > 0
        || border.left.width > 0;
    visible.then_some(InlineBoxStyle {
        node,
        pseudo,
        padding_top,
        padding_right,
        padding_bottom,
        padding_left,
        background: style.background,
        border_top: DecorationBorder {
            width: border.top.width,
            color: border.top.color,
        },
        border_right: DecorationBorder {
            width: border.right.width,
            color: border.right.color,
        },
        border_bottom: DecorationBorder {
            width: border.bottom.width,
            color: border.bottom.color,
        },
        border_left: DecorationBorder {
            width: border.left.width,
            color: border.left.color,
        },
    })
}

fn resolve_block_horizontal(style: Style, containing_width: i32) -> UsedBlockHorizontal {
    let containing_width = containing_width.max(1);
    let padding = Edges {
        top: resolve_length(style.padding.top, containing_width).max(0),
        right: resolve_length(style.padding.right, containing_width).max(0),
        bottom: resolve_length(style.padding.bottom, containing_width).max(0),
        left: resolve_length(style.padding.left, containing_width).max(0),
    };
    let border = resolve_border_edges(style.border);
    let horizontal_extras = padding
        .left
        .saturating_add(padding.right)
        .saturating_add(border.left.width)
        .saturating_add(border.right.width);

    let margin_left = resolve_margin(style.margin.left, containing_width);
    let margin_right = resolve_margin(style.margin.right, containing_width);
    let specified_width = style.width.map(|value| {
        size_to_content_width(value, style.box_sizing, containing_width, horizontal_extras)
    });

    let mut content_width = specified_width.unwrap_or_else(|| {
        containing_width
            .saturating_sub(margin_left.unwrap_or(0))
            .saturating_sub(margin_right.unwrap_or(0))
            .saturating_sub(horizontal_extras)
    });
    let min_width = size_to_content_width(
        style.min_width,
        style.box_sizing,
        containing_width,
        horizontal_extras,
    );
    content_width = content_width.max(min_width);
    if let Some(max_width) = style.max_width {
        let max_width = size_to_content_width(
            max_width,
            style.box_sizing,
            containing_width,
            horizontal_extras,
        );
        content_width = content_width.min(max_width.max(0));
    }
    content_width = content_width.max(0);

    let border_width = content_width.saturating_add(horizontal_extras).max(1);
    let (margin_left, margin_right) = if specified_width.is_none() {
        (margin_left.unwrap_or(0), margin_right.unwrap_or(0))
    } else {
        distribute_auto_margins(
            margin_left,
            margin_right,
            containing_width.saturating_sub(border_width),
        )
    };

    let _ = margin_right;
    UsedBlockHorizontal {
        margin_left,
        padding,
        border,
        content_width,
        border_width,
    }
}

fn distribute_auto_margins(
    left: Option<i32>,
    right: Option<i32>,
    available_for_margins: i32,
) -> (i32, i32) {
    match (left, right) {
        (None, None) => {
            let left = available_for_margins / 2;
            (left, available_for_margins - left)
        }
        (None, Some(right)) => (available_for_margins.saturating_sub(right), right),
        (Some(left), None) => (left, available_for_margins.saturating_sub(left)),
        (Some(left), Some(right)) => (left, right),
    }
}

fn resolve_margin(value: MarginValue, basis: i32) -> Option<i32> {
    match value {
        MarginValue::Auto => None,
        MarginValue::Length(value) => Some(resolve_length(value, basis)),
    }
}

fn resolve_vertical_margin(value: MarginValue, basis: i32) -> i32 {
    resolve_margin(value, basis).unwrap_or(0)
}

fn resolve_length(value: LengthPercentage, basis: i32) -> i32 {
    match value {
        LengthPercentage::Px(value) => bounded_round(value),
        LengthPercentage::Percent(value) => bounded_round(value * basis as f32),
    }
}

fn bounded_round(value: f32) -> i32 {
    value.round().clamp(-1_000_000.0, 1_000_000.0) as i32
}

pub(super) fn collapse_margins(first: i32, second: i32) -> i32 {
    let positive = first.max(second).max(0);
    let negative = first.min(second).min(0);
    positive.saturating_add(negative)
}

fn size_to_content_width(
    value: LengthPercentage,
    box_sizing: BoxSizing,
    basis: i32,
    horizontal_extras: i32,
) -> i32 {
    let resolved = resolve_length(value, basis).max(0);
    match box_sizing {
        BoxSizing::ContentBox => resolved,
        BoxSizing::BorderBox => resolved.saturating_sub(horizontal_extras).max(0),
    }
}

fn resolve_border_edges(border: BorderEdges) -> UsedBorderEdges {
    UsedBorderEdges {
        top: resolve_border_side(border.top),
        right: resolve_border_side(border.right),
        bottom: resolve_border_side(border.bottom),
        left: resolve_border_side(border.left),
    }
}

fn resolve_border_side(border: op_css::ComputedBorder) -> UsedBorderSide {
    UsedBorderSide {
        width: if border.style == BorderStyle::Solid {
            bounded_round(border.width_px).clamp(0, 4096)
        } else {
            0
        },
        color: border.color.into(),
    }
}

fn resolve_block_content_height(
    style: Style,
    natural_height: i32,
    padding_top: i32,
    padding_bottom: i32,
    border: UsedBorderEdges,
) -> i32 {
    let vertical_extras = padding_top
        .saturating_add(padding_bottom)
        .saturating_add(border.top.width)
        .saturating_add(border.bottom.width);
    let to_content = |value: LengthPercentage| -> Option<i32> {
        let LengthPercentage::Px(value) = value else {
            return None;
        };
        let resolved = bounded_round(value).max(0);
        Some(match style.box_sizing {
            BoxSizing::ContentBox => resolved,
            BoxSizing::BorderBox => resolved.saturating_sub(vertical_extras).max(0),
        })
    };

    let mut target = style.height.and_then(to_content).unwrap_or(natural_height);
    if let Some(min_height) = to_content(style.min_height) {
        target = target.max(min_height);
    }
    if let Some(max_height) = style.max_height.and_then(to_content) {
        target = target.min(max_height);
    }
    target.max(0)
}

fn computed_style(style: ComputedStyle) -> Style {
    let font_size = style.font_size_px.round().clamp(1.0, 4096.0) as i32;
    Style {
        inline: InlineStyle {
            font_size,
            line_height: used_line_height(style.line_height, font_size),
            weight: match style.font_weight {
                ComputedFontWeight::Normal => FontWeight::Normal,
                ComputedFontWeight::Bold => FontWeight::Bold,
            },
            font_style: match style.font_style {
                CssFontStyle::Normal => FontStyle::Normal,
                CssFontStyle::Italic | CssFontStyle::Oblique => FontStyle::Italic,
            },
            decoration: TextDecoration {
                underline: style.text_decoration_line.underline,
                line_through: style.text_decoration_line.line_through,
            },
            white_space: style.white_space,
            letter_spacing: style.letter_spacing_px.round().clamp(-4096.0, 4096.0) as i32,
            word_spacing: style.word_spacing_px.round().clamp(-4096.0, 4096.0) as i32,
            text_transform: style.text_transform,
            color: style.color.into(),
            boxes: None,
        },
        text_align: style.text_align,
        margin: style.margin,
        padding: style.padding,
        background: style.background_color.into(),
        border: style.border,
        width: style.width,
        min_width: style.min_width,
        max_width: style.max_width,
        height: style.height,
        min_height: style.min_height,
        max_height: style.max_height,
        box_sizing: style.box_sizing,
    }
}

fn used_line_height(value: ComputedLineHeight, font_size: i32) -> i32 {
    let value = match value {
        ComputedLineHeight::Normal => font_size as f32 * 1.35,
        ComputedLineHeight::Number(multiplier) => font_size as f32 * multiplier,
        ComputedLineHeight::Px(value) => value,
    };
    value.round().clamp(0.0, 100_000.0) as i32
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
            line_height: 24,
            weight: FontWeight::Normal,
            font_style: FontStyle::Normal,
            decoration: TextDecoration::NONE,
            white_space: WhiteSpace::Normal,
            letter_spacing: 0,
            word_spacing: 0,
            text_transform: TextTransform::None,
            color: TextColor {
                red: 0,
                green: 0,
                blue: 0,
                alpha: 255,
            },
            boxes: None,
        },
        text_align: TextAlign::Start,
        margin: MarginEdges::ZERO,
        padding: PaddingEdges::ZERO,
        background: TextColor {
            red: 0,
            green: 0,
            blue: 0,
            alpha: 0,
        },
        border: BorderEdges::NONE,
        width: None,
        min_width: LengthPercentage::ZERO,
        max_width: None,
        height: None,
        min_height: LengthPercentage::ZERO,
        max_height: None,
        box_sizing: BoxSizing::ContentBox,
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
        text_align: inherited.text_align,
        margin: MarginEdges {
            top: MarginValue::Length(LengthPercentage::Px(top as f32)),
            right: MarginValue::ZERO,
            bottom: MarginValue::Length(LengthPercentage::Px(bottom as f32)),
            left: MarginValue::ZERO,
        },
        padding: PaddingEdges::ZERO,
        background: default_style().background,
        border: BorderEdges::NONE,
        width: None,
        min_width: LengthPercentage::ZERO,
        max_width: None,
        height: None,
        min_height: LengthPercentage::ZERO,
        max_height: None,
        box_sizing: BoxSizing::ContentBox,
    }
}

fn attribute<'a>(element: &'a op_dom::ElementData, name: &str) -> Option<&'a str> {
    element
        .attributes
        .iter()
        .find(|a| a.name == name)
        .map(|a| a.value.as_str())
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
