use super::{
    BoxDecoration, DecorationBorder, FontStyle, FontWeight, ImageBox, LayoutItem, LinkSpan,
    TextBox, TextColor, TextDecoration, TextMeasurer, TextMetrics,
};
use op_css::{TextAlign, TextTransform, WhiteSpace};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct InlineBoxStyle {
    pub node: op_dom::NodeId,
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
    fn left_extra(self) -> i32 {
        self.border_left.width.saturating_add(self.padding_left)
    }

    fn right_extra(self) -> i32 {
        self.padding_right.saturating_add(self.border_right.width)
    }

    fn top_extra(self) -> i32 {
        self.border_top.width.saturating_add(self.padding_top)
    }

    fn bottom_extra(self) -> i32 {
        self.padding_bottom.saturating_add(self.border_bottom.width)
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
    pub box_style: Option<InlineBoxStyle>,
}

#[derive(Clone, Copy)]
pub(super) struct InlineChar<'a> {
    pub ch: char,
    pub href: Option<&'a str>,
    pub style: InlineStyle,
}

pub(super) enum Item<'a> {
    Char(InlineChar<'a>),
    Image(ImageBox),
    Break,
}

enum BoxItem<'a> {
    Text {
        chars: Vec<InlineChar<'a>>,
        width: i32,
    },
    Image(ImageBox),
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
    Image(ImageBox),
}

pub(super) struct Lines<'a, 'm> {
    measurer: &'m mut dyn TextMeasurer,
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
        default_style: InlineStyle,
        text_align: TextAlign,
        x: i32,
        y: i32,
        width: i32,
    ) -> Self {
        Self {
            measurer,
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
                Item::Image(image) => {
                    preserved_cr = false;
                    self.word(std::mem::take(&mut word));
                    self.image(image);
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

    fn text_width(&mut self, chars: &[InlineChar<'a>]) -> i32 {
        if chars.is_empty() {
            return 0;
        }
        let mut width = self.content_width(chars);
        let mut active: Option<InlineBoxStyle> = None;
        for ch in chars {
            if ch.style.box_style != active {
                if let Some(previous) = active {
                    width = width.saturating_add(previous.right_extra());
                }
                if let Some(next) = ch.style.box_style {
                    width = width.saturating_add(next.left_extra());
                }
                active = ch.style.box_style;
            }
        }
        if let Some(active) = active {
            width = width.saturating_add(active.right_extra());
        }
        width
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
        self.line_width
            .saturating_sub(old)
            .saturating_add(self.text_width(&combined))
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
        let width = self.text_width(&combined);
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

    fn image(&mut self, image: ImageBox) {
        let space = if self.boxes.is_empty() {
            None
        } else {
            self.pending_space.take()
        };
        self.pending_space = None;
        let occupied = self.appended_width(&[], space).saturating_add(image.width);
        if !self.boxes.is_empty() && occupied > self.width {
            self.flush(false);
        }
        if !self.boxes.is_empty()
            && let Some(space) = space
        {
            self.append(&[], Some(space));
        }
        self.line_width = self.line_width.saturating_add(image.width);
        self.boxes.push(BoxItem::Image(image));
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
        if let Some(box_style) = style.box_style {
            ascent = ascent.saturating_add(box_style.top_extra());
            descent = descent.saturating_add(box_style.bottom_extra());
        }
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
                BoxItem::Image(image) => prepared.push(PreparedBox::Image(image)),
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
                PreparedBox::Image(image) => image.height,
                PreparedBox::Text(text) => text.ascent,
            })
            .max()
            .unwrap_or(default_ascent)
            .max(default_ascent);
        let descent = prepared
            .iter()
            .filter_map(|item| match item {
                PreparedBox::Text(text) => Some(text.descent),
                PreparedBox::Image(_) => None,
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
        let mut active_fragment: Option<(InlineBoxStyle, i32)> = None;
        for item in prepared {
            let next_box = match &item {
                PreparedBox::Text(text) => text.style.box_style,
                PreparedBox::Image(_) => None,
            };
            if active_fragment.map(|(style, _)| style) != next_box {
                if let Some((style, start_x)) = active_fragment.take() {
                    x = x.saturating_add(style.right_extra());
                    self.decorations.push(BoxDecoration {
                        x: start_x,
                        y: self.y,
                        width: x.saturating_sub(start_x).max(0),
                        height: ascent.saturating_add(descent),
                        background: style.background,
                        border_top: style.border_top,
                        border_right: style.border_right,
                        border_bottom: style.border_bottom,
                        border_left: style.border_left,
                    });
                }
                if let Some(style) = next_box {
                    let start_x = x;
                    x = x.saturating_add(style.left_extra());
                    active_fragment = Some((style, start_x));
                }
            }

            match item {
                PreparedBox::Image(mut image) => {
                    image.x = x;
                    image.y = baseline - image.height;
                    x += image.width;
                    self.order.push(LayoutItem::Image(self.image_boxes.len()));
                    self.image_boxes.push(image);
                }
                PreparedBox::Text(text) => {
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
        if let Some((style, start_x)) = active_fragment.take() {
            x = x.saturating_add(style.right_extra());
            self.decorations.push(BoxDecoration {
                x: start_x,
                y: self.y,
                width: x.saturating_sub(start_x).max(0),
                height: ascent.saturating_add(descent),
                background: style.background,
                border_top: style.border_top,
                border_right: style.border_right,
                border_bottom: style.border_bottom,
                border_left: style.border_left,
            });
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
