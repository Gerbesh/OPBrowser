use super::{
    BoxDecoration, DecorationBorder, FontStyle, FontWeight, ImageBox, LayoutItem, LinkSpan,
    TextBox, TextColor, TextDecoration, TextMeasurer, TextMetrics,
};
use op_css::{PseudoElement, TextAlign, TextTransform, WhiteSpace};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct InlineBoxStyle {
    pub node: op_dom::NodeId,
    pub pseudo: Option<PseudoElement>,
    pub padding_top: i32,
    pub padding_right: i32,
    pub padding_bottom: i32,
    pub padding_left: i32,
    pub background: TextColor,
    pub border_top: DecorationBorder,
    pub border_right: DecorationBorder,
    pub border_bottom: DecorationBorder,
    pub border_left: DecorationBorder,
}

impl InlineBoxStyle {
    pub(super) fn left_extra(self) -> i32 {
        self.border_left.width.saturating_add(self.padding_left)
    }

    pub(super) fn right_extra(self) -> i32 {
        self.padding_right.saturating_add(self.border_right.width)
    }

    pub(super) fn top_extra(self) -> i32 {
        self.border_top.width.saturating_add(self.padding_top)
    }

    pub(super) fn bottom_extra(self) -> i32 {
        self.padding_bottom.saturating_add(self.border_bottom.width)
    }
}

#[derive(Default)]
pub(super) struct InlineBoxes {
    nodes: Vec<InlineBoxNode>,
}

struct InlineBoxNode {
    style: InlineBoxStyle,
    parent: Option<usize>,
    depth: usize,
    left: i32,
    right: i32,
    top: i32,
    bottom: i32,
}

impl InlineBoxes {
    pub(super) fn push(&mut self, style: InlineBoxStyle, parent: Option<usize>) -> usize {
        let id = self.nodes.len();
        self.nodes.push(InlineBoxNode {
            style,
            parent,
            depth: parent.map_or(1, |id| self.nodes[id].depth + 1),
            left: self.left(parent).saturating_add(style.left_extra()),
            right: self.right(parent).saturating_add(style.right_extra()),
            top: self.top(parent).saturating_add(style.top_extra()),
            bottom: self.bottom(parent).saturating_add(style.bottom_extra()),
        });
        id
    }

    pub(super) fn style(&self, id: usize) -> InlineBoxStyle {
        self.nodes[id].style
    }
    pub(super) fn parent(&self, id: usize) -> Option<usize> {
        self.nodes[id].parent
    }
    pub(super) fn horizontal(&self, id: Option<usize>) -> i32 {
        self.left(id).saturating_add(self.right(id))
    }
    fn left(&self, id: Option<usize>) -> i32 {
        id.map_or(0, |id| self.nodes[id].left)
    }
    fn right(&self, id: Option<usize>) -> i32 {
        id.map_or(0, |id| self.nodes[id].right)
    }
    fn top(&self, id: Option<usize>) -> i32 {
        id.map_or(0, |id| self.nodes[id].top)
    }
    fn bottom(&self, id: Option<usize>) -> i32 {
        id.map_or(0, |id| self.nodes[id].bottom)
    }
    fn path(&self, mut id: Option<usize>) -> Vec<usize> {
        let mut path = Vec::new();
        while let Some(current) = id {
            path.push(current);
            id = self.parent(current);
        }
        path.reverse();
        path
    }
    fn transition(&self, mut from: Option<usize>, mut to: Option<usize>) -> i32 {
        let mut width: i32 = 0;
        let depth = |id: Option<usize>| id.map_or(0, |id| self.nodes[id].depth);
        while from != to {
            if depth(from) >= depth(to) {
                let id = from.expect("unequal stack has a source node");
                width = width.saturating_add(self.style(id).right_extra());
                from = self.parent(id);
            } else {
                let id = to.expect("unequal stack has a target node");
                width = width.saturating_add(self.style(id).left_extra());
                to = self.parent(id);
            }
        }
        width
    }
    fn contribution(&self, from: Option<usize>, to: Option<usize>, content: i32) -> i32 {
        content
            .saturating_add(self.transition(from, to))
            .saturating_add(self.right(to))
            .saturating_sub(self.right(from))
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct InlineStyle {
    pub font_size: i32,
    pub line_height: i32,
    pub weight: FontWeight,
    pub font_style: FontStyle,
    pub decoration: TextDecoration,
    pub white_space: WhiteSpace,
    pub letter_spacing: i32,
    pub word_spacing: i32,
    pub text_transform: TextTransform,
    pub color: TextColor,
    pub boxes: Option<usize>,
}

#[derive(Clone, Copy)]
pub(super) struct InlineChar<'a> {
    pub ch: char,
    pub href: Option<&'a str>,
    pub style: InlineStyle,
}

pub(super) enum Item<'a> {
    Char(InlineChar<'a>),
    EmptyInline(InlineStyle),
    Image(ImageBox, InlineStyle, Option<InlineBoxStyle>),
    Break,
}

enum BoxItem<'a> {
    Text {
        chars: Vec<InlineChar<'a>>,
        width: i32,
    },
    Image(ImageBox, InlineStyle, Option<InlineBoxStyle>),
    EmptyInline(InlineStyle),
}

struct PreparedText {
    text: String,
    links: Vec<LinkSpan>,
    width: i32,
    style: InlineStyle,
    metrics: TextMetrics,
    ascent: i32,
    descent: i32,
}

enum PreparedBox {
    Text(PreparedText),
    Image(ImageBox, InlineStyle, Option<InlineBoxStyle>),
}

pub(super) struct Lines<'a, 'm> {
    measurer: &'m mut dyn TextMeasurer,
    inline_boxes: &'m InlineBoxes,
    default_style: InlineStyle,
    text_align: TextAlign,
    x: i32,
    y: i32,
    width: i32,
    line_width: i32,
    boxes: Vec<BoxItem<'a>>,
    pending_space: Option<InlineChar<'a>>,
    pub decorations: Vec<BoxDecoration>,
    pub text_boxes: Vec<TextBox>,
    pub image_boxes: Vec<ImageBox>,
    pub order: Vec<LayoutItem>,
}

impl<'a, 'm> Lines<'a, 'm> {
    pub fn new(
        measurer: &'m mut dyn TextMeasurer,
        inline_boxes: &'m InlineBoxes,
        default_style: InlineStyle,
        text_align: TextAlign,
        x: i32,
        y: i32,
        width: i32,
    ) -> Self {
        Self {
            measurer,
            inline_boxes,
            default_style,
            text_align,
            x,
            y,
            width,
            line_width: 0,
            boxes: Vec::new(),
            pending_space: None,
            decorations: Vec::new(),
            text_boxes: Vec::new(),
            image_boxes: Vec::new(),
            order: Vec::new(),
        }
    }

    pub fn layout(mut self, items: Vec<Item<'a>>) -> Self {
        let mut word = Vec::new();
        let mut preserved_cr = false;
        for item in items {
            match item {
                Item::Char(ch) => {
                    let preserve_newline = matches!(
                        ch.style.white_space,
                        WhiteSpace::Pre | WhiteSpace::PreWrap | WhiteSpace::PreLine
                    );
                    if preserve_newline && matches!(ch.ch, '\r' | '\n') {
                        if ch.ch == '\n' && preserved_cr {
                            preserved_cr = false;
                            continue;
                        }
                        preserved_cr = ch.ch == '\r';
                        self.word(std::mem::take(&mut word));
                        self.flush(true);
                        continue;
                    }
                    preserved_cr = false;

                    if matches!(ch.ch, ' ' | '\t' | '\n' | '\r' | '\u{c}') {
                        self.word(std::mem::take(&mut word));
                        if matches!(ch.style.white_space, WhiteSpace::Pre | WhiteSpace::PreWrap) {
                            self.preserved_space(ch);
                        } else if self.pending_space.is_none() {
                            self.pending_space = Some(InlineChar {
                                ch: ' ',
                                href: ch.href,
                                style: ch.style,
                            });
                        }
                    } else {
                        word.push(ch);
                    }
                }
                Item::Image(image, style, own_box) => {
                    preserved_cr = false;
                    self.word(std::mem::take(&mut word));
                    self.image(image, style, own_box);
                }
                Item::EmptyInline(style) => {
                    preserved_cr = false;
                    self.word(std::mem::take(&mut word));
                    self.empty_inline(style);
                }
                Item::Break => {
                    preserved_cr = false;
                    self.word(std::mem::take(&mut word));
                    self.flush(true);
                }
            }
        }
        self.word(word);
        self.flush(false);
        self
    }

    pub fn bottom(&self) -> i32 {
        self.y
    }

    fn content_width(&mut self, chars: &[InlineChar<'a>]) -> i32 {
        let mut width = 0i32;
        let mut start = 0;
        while start < chars.len() {
            let style = chars[start].style;
            let end = chars[start..]
                .iter()
                .position(|ch| ch.style != style)
                .map_or(chars.len(), |n| start + n);
            let text = transformed_text(&chars[start..end], style.text_transform);
            let measured = self
                .measurer
                .measure(&text, style.font_size, style.weight, style.font_style)
                .width;
            width = width.saturating_add(spaced_width(
                measured,
                &text,
                style.letter_spacing,
                style.word_spacing,
            ));
            start = end;
        }
        width
    }

    fn tail(&self) -> Option<usize> {
        self.box_tail(self.boxes.last())
    }

    fn box_tail(&self, item: Option<&BoxItem<'a>>) -> Option<usize> {
        match item {
            Some(BoxItem::Text { chars, .. }) => chars.last().and_then(|ch| ch.style.boxes),
            Some(BoxItem::Image(_, style, _) | BoxItem::EmptyInline(style)) => style.boxes,
            None => None,
        }
    }

    fn text_width(&mut self, chars: &[InlineChar<'a>], prefix: Option<usize>) -> i32 {
        if chars.is_empty() {
            return 0;
        }
        let mut width = self.content_width(chars);
        let mut active = prefix;
        for ch in chars {
            width = width.saturating_add(self.inline_boxes.transition(active, ch.style.boxes));
            active = ch.style.boxes;
        }
        width
            .saturating_add(self.inline_boxes.right(active))
            .saturating_sub(self.inline_boxes.right(prefix))
    }

    fn appended_width(&mut self, chars: &[InlineChar<'a>], space: Option<InlineChar<'a>>) -> i32 {
        let mut combined = match self.boxes.last() {
            Some(BoxItem::Text { chars, .. }) => chars.clone(),
            _ => Vec::new(),
        };
        combined.extend(space);
        combined.extend_from_slice(chars);
        let old = match self.boxes.last() {
            Some(BoxItem::Text { width, .. }) => *width,
            _ => 0,
        };
        let prefix = if matches!(self.boxes.last(), Some(BoxItem::Text { .. })) {
            self.box_tail(
                self.boxes
                    .len()
                    .checked_sub(2)
                    .and_then(|index| self.boxes.get(index)),
            )
        } else {
            self.tail()
        };
        self.line_width
            .saturating_sub(old)
            .saturating_add(self.text_width(&combined, prefix))
    }

    // Exponential probing avoids repeatedly measuring giant remaining suffixes
    // when a long unbroken word must be split across many narrow lines.
    fn fitting_prefix(&mut self, word: &[InlineChar<'a>], space: Option<InlineChar<'a>>) -> usize {
        let (mut low, mut high) = (0, 1.min(word.len()));
        while high > 0 && self.appended_width(&word[..high], space) <= self.width {
            low = high;
            if high == word.len() {
                return high;
            }
            high = high.saturating_mul(2).min(word.len());
        }
        while high > low + 1 {
            let middle = low + (high - low) / 2;
            if self.appended_width(&word[..middle], space) <= self.width {
                low = middle;
            } else {
                high = middle;
            }
        }
        low
    }

    fn append(&mut self, chars: &[InlineChar<'a>], space: Option<InlineChar<'a>>) {
        let mut combined = match self.boxes.pop() {
            Some(BoxItem::Text { chars, width }) => {
                self.line_width -= width;
                chars
            }
            Some(image) => {
                self.boxes.push(image);
                Vec::new()
            }
            None => Vec::new(),
        };
        combined.extend(space);
        combined.extend_from_slice(chars);
        let width = self.text_width(&combined, self.tail());
        self.line_width = self.line_width.saturating_add(width);
        self.boxes.push(BoxItem::Text {
            chars: combined,
            width,
        });
    }

    fn word(&mut self, word: Vec<InlineChar<'a>>) {
        if word.is_empty() {
            return;
        }
        let space = if self.boxes.is_empty() {
            None
        } else {
            self.pending_space.take()
        };
        self.pending_space = None;
        let allows_wrap = word.iter().all(|ch| {
            matches!(
                ch.style.white_space,
                WhiteSpace::Normal | WhiteSpace::PreWrap | WhiteSpace::PreLine
            )
        });
        if !allows_wrap {
            self.append(&word, space);
            return;
        }
        if !self.boxes.is_empty() && self.fitting_prefix(&word, space) < word.len() {
            self.flush(false);
        }
        let mut start = 0;
        while start < word.len() {
            let space = if start == 0 && !self.boxes.is_empty() {
                space
            } else {
                None
            };
            let count = self.fitting_prefix(&word[start..], space).max(1);
            self.append(&word[start..start + count], space);
            start += count;
            if start < word.len() {
                self.flush(false);
            }
        }
    }

    fn preserved_space(&mut self, ch: InlineChar<'a>) {
        self.pending_space = None;
        let count = if ch.ch == '\t' { 4 } else { 1 };
        for _ in 0..count {
            let space = InlineChar { ch: ' ', ..ch };
            if ch.style.white_space == WhiteSpace::PreWrap
                && !self.boxes.is_empty()
                && self.appended_width(&[space], None) > self.width
            {
                self.flush(false);
            }
            self.append(&[space], None);
        }
    }

    fn image(&mut self, image: ImageBox, style: InlineStyle, own_box: Option<InlineBoxStyle>) {
        let space = if self.boxes.is_empty() {
            None
        } else {
            self.pending_space.take()
        };
        self.pending_space = None;
        let extras = own_box.map_or(0, |box_style| {
            box_style
                .left_extra()
                .saturating_add(box_style.right_extra())
        });
        let width = image.width.saturating_add(extras);
        let occupied =
            self.appended_width(&[], space)
                .saturating_add(self.inline_boxes.contribution(
                    space.map_or(self.tail(), |value| value.style.boxes),
                    style.boxes,
                    width,
                ));
        let allows_wrap = matches!(
            style.white_space,
            WhiteSpace::Normal | WhiteSpace::PreWrap | WhiteSpace::PreLine
        );
        if allows_wrap && !self.boxes.is_empty() && occupied > self.width {
            self.flush(false);
        }
        if !self.boxes.is_empty()
            && let Some(space) = space
        {
            self.append(&[], Some(space));
        }
        self.line_width = self
            .line_width
            .saturating_add(
                self.inline_boxes
                    .contribution(self.tail(), style.boxes, width),
            );
        self.boxes.push(BoxItem::Image(image, style, own_box));
    }

    fn empty_inline(&mut self, style: InlineStyle) {
        let Some(_) = style.boxes else {
            return;
        };
        let space = if self.boxes.is_empty() {
            None
        } else {
            self.pending_space.take()
        };
        self.pending_space = None;
        let occupied =
            self.appended_width(&[], space)
                .saturating_add(self.inline_boxes.contribution(
                    space.map_or(self.tail(), |value| value.style.boxes),
                    style.boxes,
                    0,
                ));
        let allows_wrap = matches!(
            style.white_space,
            WhiteSpace::Normal | WhiteSpace::PreWrap | WhiteSpace::PreLine
        );
        if allows_wrap && !self.boxes.is_empty() && occupied > self.width {
            self.flush(false);
        }
        if !self.boxes.is_empty()
            && let Some(space) = space
        {
            self.append(&[], Some(space));
        }
        self.line_width = self
            .line_width
            .saturating_add(self.inline_boxes.contribution(self.tail(), style.boxes, 0));
        self.boxes.push(BoxItem::EmptyInline(style));
    }

    fn metrics_for(&mut self, style: InlineStyle) -> (TextMetrics, i32, i32) {
        let metrics = self
            .measurer
            .measure("", style.font_size, style.weight, style.font_style);
        let line_height = style.line_height.max(0);
        let leading = line_height - metrics.ascent - metrics.descent;
        let ascent = (metrics.ascent + leading / 2).max(0);
        let descent = (line_height - ascent).max(0);
        (metrics, ascent, descent)
    }

    fn prepare_text(&mut self, chars: &[InlineChar<'a>], style: InlineStyle) -> PreparedText {
        let width = self.content_width(chars);
        let mut text = String::new();
        let mut links: Vec<LinkSpan> = Vec::new();
        let mut capitalize_next = true;
        for ch in chars {
            let start = text.len();
            append_transformed_char(&mut text, ch.ch, style.text_transform, &mut capitalize_next);
            if let Some(href) = ch.href {
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
        let (metrics, mut ascent, mut descent) = self.metrics_for(style);
        ascent = ascent.saturating_add(self.inline_boxes.top(style.boxes));
        descent = descent.saturating_add(self.inline_boxes.bottom(style.boxes));
        PreparedText {
            text,
            links,
            width,
            style,
            metrics,
            ascent,
            descent,
        }
    }

    fn flush(&mut self, forced: bool) {
        self.pending_space = None;
        if self.boxes.is_empty() && !forced {
            return;
        }

        let boxes = std::mem::take(&mut self.boxes);
        let mut prepared = Vec::new();
        for item in boxes {
            match item {
                BoxItem::Image(image, style, own_box) => {
                    prepared.push(PreparedBox::Image(image, style, own_box))
                }
                BoxItem::EmptyInline(style) => {
                    prepared.push(PreparedBox::Text(self.prepare_text(&[], style)))
                }
                BoxItem::Text { chars, .. } => {
                    let mut start = 0;
                    while start < chars.len() {
                        let style = chars[start].style;
                        let end = chars[start..]
                            .iter()
                            .position(|ch| ch.style != style)
                            .map_or(chars.len(), |n| start + n);
                        prepared.push(PreparedBox::Text(
                            self.prepare_text(&chars[start..end], style),
                        ));
                        start = end;
                    }
                }
            }
        }

        let (_, default_ascent, default_descent) = self.metrics_for(self.default_style);
        let ascent = prepared
            .iter()
            .map(|item| match item {
                PreparedBox::Image(image, style, own_box) => image
                    .height
                    .saturating_add(own_box.map_or(0, |box_style| {
                        box_style
                            .top_extra()
                            .saturating_add(box_style.bottom_extra())
                    }))
                    .saturating_add(self.inline_boxes.top(style.boxes)),
                PreparedBox::Text(text) => text.ascent,
            })
            .max()
            .unwrap_or(default_ascent)
            .max(default_ascent);
        let descent = prepared
            .iter()
            .map(|item| match item {
                PreparedBox::Text(text) => text.descent,
                PreparedBox::Image(_, style, _) => self.inline_boxes.bottom(style.boxes),
            })
            .max()
            .unwrap_or(default_descent)
            .max(default_descent);

        let baseline = self.y + ascent;
        let remaining = self.width.saturating_sub(self.line_width).max(0);
        let offset = match self.text_align {
            TextAlign::Start | TextAlign::Left => 0,
            TextAlign::End | TextAlign::Right => remaining,
            TextAlign::Center => remaining / 2,
        };
        let mut x = self.x.saturating_add(offset);
        let mut active_fragments: Vec<(usize, i32, usize)> = Vec::new();
        for item in prepared {
            let next_box = match &item {
                PreparedBox::Text(text) => text.style.boxes,
                PreparedBox::Image(_, style, _) => style.boxes,
            };
            let path = self.inline_boxes.path(next_box);
            let common = active_fragments
                .iter()
                .zip(&path)
                .take_while(|((id, _, _), next)| id == *next)
                .count();
            while active_fragments.len() > common {
                let (id, start_x, decoration) =
                    active_fragments.pop().expect("active inline fragment");
                x = x.saturating_add(self.inline_boxes.style(id).right_extra());
                self.decorations[decoration].width = x.saturating_sub(start_x).max(0);
            }
            for id in path.into_iter().skip(common) {
                let style = self.inline_boxes.style(id);
                let parent = self.inline_boxes.parent(id);
                let top = self.inline_boxes.top(parent);
                let bottom = self.inline_boxes.bottom(parent);
                let decoration = self.decorations.len();
                self.decorations.push(BoxDecoration {
                    x,
                    y: self.y.saturating_add(top),
                    width: 0,
                    height: ascent
                        .saturating_add(descent)
                        .saturating_sub(top)
                        .saturating_sub(bottom)
                        .max(0),
                    background: style.background,
                    border_top: style.border_top,
                    border_right: style.border_right,
                    border_bottom: style.border_bottom,
                    border_left: style.border_left,
                });
                active_fragments.push((id, x, decoration));
                x = x.saturating_add(style.left_extra());
            }

            match item {
                PreparedBox::Image(mut image, _, own_box) => {
                    if let Some(box_style) = own_box {
                        let left = box_style.left_extra();
                        let right = box_style.right_extra();
                        let top = box_style.top_extra();
                        let bottom = box_style.bottom_extra();
                        self.decorations.push(BoxDecoration {
                            x,
                            y: baseline - bottom - image.height - top,
                            width: image.width.saturating_add(left).saturating_add(right),
                            height: image.height.saturating_add(top).saturating_add(bottom),
                            background: box_style.background,
                            border_top: box_style.border_top,
                            border_right: box_style.border_right,
                            border_bottom: box_style.border_bottom,
                            border_left: box_style.border_left,
                        });
                        x = x.saturating_add(left);
                    }
                    image.x = x;
                    image.y =
                        baseline - image.height - own_box.map_or(0, InlineBoxStyle::bottom_extra);
                    x += image.width;
                    x = x.saturating_add(own_box.map_or(0, InlineBoxStyle::right_extra));
                    self.order.push(LayoutItem::Image(self.image_boxes.len()));
                    self.image_boxes.push(image);
                }
                PreparedBox::Text(text) => {
                    if text.text.is_empty() {
                        continue;
                    }
                    self.order.push(LayoutItem::Text(self.text_boxes.len()));
                    self.text_boxes.push(TextBox {
                        x,
                        y: baseline - text.metrics.ascent,
                        width: text.width,
                        height: text.metrics.ascent + text.metrics.descent,
                        text: text.text,
                        font_size: text.style.font_size,
                        weight: text.style.weight,
                        style: text.style.font_style,
                        decoration: text.style.decoration,
                        letter_spacing: text.style.letter_spacing,
                        word_spacing: text.style.word_spacing,
                        color: text.style.color,
                        links: text.links,
                    });
                    x += text.width;
                }
            }
        }
        while let Some((id, start_x, decoration)) = active_fragments.pop() {
            x = x.saturating_add(self.inline_boxes.style(id).right_extra());
            self.decorations[decoration].width = x.saturating_sub(start_x).max(0);
        }

        self.y += ascent + descent;
        self.line_width = 0;
    }
}

fn transformed_text(chars: &[InlineChar<'_>], transform: TextTransform) -> String {
    let mut text = String::new();
    let mut capitalize_next = true;
    for ch in chars {
        append_transformed_char(&mut text, ch.ch, transform, &mut capitalize_next);
    }
    text
}

fn append_transformed_char(
    output: &mut String,
    ch: char,
    transform: TextTransform,
    capitalize_next: &mut bool,
) {
    match transform {
        TextTransform::None => output.push(ch),
        TextTransform::Uppercase => output.extend(ch.to_uppercase()),
        TextTransform::Lowercase => output.extend(ch.to_lowercase()),
        TextTransform::Capitalize if *capitalize_next && ch.is_alphabetic() => {
            output.extend(ch.to_uppercase());
        }
        TextTransform::Capitalize => output.push(ch),
    }
    *capitalize_next = ch.is_whitespace();
}

fn spaced_width(base: i32, text: &str, letter_spacing: i32, word_spacing: i32) -> i32 {
    let gaps = text.chars().count().saturating_sub(1) as i32;
    let spaces = text.chars().filter(|ch| *ch == ' ').count() as i32;
    base.saturating_add(letter_spacing.saturating_mul(gaps))
        .saturating_add(word_spacing.saturating_mul(spaces))
        .max(0)
}
