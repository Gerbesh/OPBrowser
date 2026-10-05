use super::{FontWeight, ImageBox, LayoutItem, LinkSpan, TextBox, TextMeasurer};

#[derive(Clone, Copy)]
pub(super) struct InlineChar<'a> {
    pub ch: char,
    pub href: Option<&'a str>,
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

pub(super) struct Lines<'a, 'm> {
    measurer: &'m mut dyn TextMeasurer,
    font_size: i32,
    weight: FontWeight,
    x: i32,
    y: i32,
    width: i32,
    line_width: i32,
    boxes: Vec<BoxItem<'a>>,
    pending_space: Option<InlineChar<'a>>,
    pub text_boxes: Vec<TextBox>,
    pub image_boxes: Vec<ImageBox>,
    pub order: Vec<LayoutItem>,
}

impl<'a, 'm> Lines<'a, 'm> {
    pub fn new(
        measurer: &'m mut dyn TextMeasurer,
        font_size: i32,
        weight: FontWeight,
        x: i32,
        y: i32,
        width: i32,
    ) -> Self {
        Self {
            measurer,
            font_size,
            weight,
            x,
            y,
            width,
            line_width: 0,
            boxes: Vec::new(),
            pending_space: None,
            text_boxes: Vec::new(),
            image_boxes: Vec::new(),
            order: Vec::new(),
        }
    }

    pub fn layout(mut self, items: Vec<Item<'a>>) -> Self {
        let mut word = Vec::new();
        for item in items {
            match item {
                Item::Char(ch) if matches!(ch.ch, ' ' | '\t' | '\n' | '\r' | '\u{c}') => {
                    self.word(std::mem::take(&mut word));
                    if self.pending_space.is_none() {
                        self.pending_space = Some(InlineChar {
                            ch: ' ',
                            href: ch.href,
                        });
                    }
                }
                Item::Char(ch) => word.push(ch),
                Item::Image(image) => {
                    self.word(std::mem::take(&mut word));
                    self.image(image);
                }
                Item::Break => {
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

    fn text_width(&mut self, chars: &[InlineChar<'a>]) -> i32 {
        let mut width = 0i32;
        let mut start = 0;
        while start < chars.len() {
            let href = chars[start].href;
            let end = chars[start..]
                .iter()
                .position(|ch| ch.href != href)
                .map_or(chars.len(), |n| start + n);
            let text: String = chars[start..end].iter().map(|ch| ch.ch).collect();
            width = width.saturating_add(
                self.measurer
                    .measure(&text, self.font_size, self.weight)
                    .width
                    .max(0),
            );
            start = end;
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

    fn flush(&mut self, forced: bool) {
        self.pending_space = None;
        if self.boxes.is_empty() && !forced {
            return;
        }
        let metrics = self.measurer.measure("", self.font_size, self.weight);
        let leading =
            (((self.font_size as f32) * 1.35).round() as i32 - metrics.ascent - metrics.descent)
                .max(0);
        let descent = metrics.descent + leading - leading / 2;
        let ascent = self
            .boxes
            .iter()
            .filter_map(|item| match item {
                BoxItem::Image(image) => Some(image.height),
                _ => None,
            })
            .max()
            .unwrap_or(0)
            .max(metrics.ascent + leading / 2);
        let baseline = self.y + ascent;
        let mut x = self.x;
        for item in std::mem::take(&mut self.boxes) {
            match item {
                BoxItem::Image(mut image) => {
                    image.x = x;
                    image.y = baseline - image.height;
                    x += image.width;
                    self.order.push(LayoutItem::Image(self.image_boxes.len()));
                    self.image_boxes.push(image);
                }
                BoxItem::Text { chars, width } => {
                    let mut text = String::new();
                    let mut links: Vec<LinkSpan> = Vec::new();
                    for ch in chars {
                        let start = text.len();
                        text.push(ch.ch);
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
                    self.order.push(LayoutItem::Text(self.text_boxes.len()));
                    self.text_boxes.push(TextBox {
                        x,
                        y: baseline - metrics.ascent,
                        width,
                        height: metrics.ascent + metrics.descent,
                        text,
                        font_size: self.font_size,
                        weight: self.weight,
                        links,
                    });
                    x += width;
                }
            }
        }
        self.y += ascent + descent;
        self.line_width = 0;
    }
}
