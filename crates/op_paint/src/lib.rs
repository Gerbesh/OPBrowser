pub use op_image::RasterImage;
pub use op_layout::LinkSpan;
use op_layout::{BoxDecoration, FontStyle, FontWeight, LayoutItem, LayoutTree, TextColor};
use std::sync::Arc;

/// Shared by the worker's font extent adapter and native painter.
pub const TEXT_FONT_FAMILY: &str = "Segoe UI";

/// Serialize GDI font realization, extents and cleanup across worker/UI threads.
#[cfg(windows)]
pub static GDI_TEXT_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub const WHITE: Self = Self {
        r: 255,
        g: 255,
        b: 255,
    };

    pub const BLACK: Self = Self { r: 0, g: 0, b: 0 };

    pub const LINK: Self = Self {
        r: 0,
        g: 70,
        b: 190,
    };
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaintCommand {
    Image {
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        image: Arc<RasterImage>,
        href: Option<String>,
    },
    FillRect {
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        color: Color,
    },
    Text {
        x: i32,
        y: i32,
        text: String,
        font_size: i32,
        bold: bool,
        italic: bool,
        underline: bool,
        line_through: bool,
        letter_spacing: i32,
        word_spacing: i32,
        color: Color,
        links: Vec<LinkSpan>,
    },
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DisplayList {
    pub commands: Vec<PaintCommand>,
}

pub fn build_display_list(layout: &LayoutTree, viewport_height: i32) -> DisplayList {
    let mut commands = Vec::with_capacity(
        layout.text_boxes.len() + layout.image_boxes.len() + layout.box_decorations.len() * 5 + 1,
    );
    commands.push(PaintCommand::FillRect {
        x: 0,
        y: 0,
        width: layout.viewport_width,
        height: viewport_height.max(layout.content_height),
        color: Color::WHITE,
    });

    // Parent keys identify atomic contexts, even when the parent has no pixels.
    let mut parents = std::collections::HashMap::new();
    for group in &layout.paint_groups {
        parents.insert(group.key, group.parent);
    }
    for key in layout
        .box_decorations
        .iter()
        .filter_map(|box_| box_.paint_key)
        .chain(layout.order.iter().filter_map(|item| match item {
            LayoutItem::PositionedText(_, key) | LayoutItem::PositionedImage(_, key) => Some(*key),
            LayoutItem::Text(_) | LayoutItem::Image(_) => None,
        }))
    {
        parents.entry(key).or_insert(None);
    }
    let mut children: std::collections::HashMap<
        Option<op_layout::PaintKey>,
        Vec<op_layout::PaintKey>,
    > = std::collections::HashMap::new();
    for (&key, &parent) in &parents {
        // Unregistered ancestors in hand-built fixtures fall back to the root.
        let parent = parent.filter(|value| *value != key && parents.contains_key(value));
        children.entry(parent).or_default().push(key);
    }
    for siblings in children.values_mut() {
        siblings.sort_unstable();
    }

    // Iterative context traversal avoids Rust stack overflow on deeply nested markup.
    // Negative children go after the context's block background, before its text.
    let mut pending = vec![(None, false)];
    while let Some((context, foreground)) = pending.pop() {
        if !foreground {
            if context.is_some() {
                emit_backgrounds(
                    layout,
                    &mut commands,
                    op_layout::DecorationPaintLayer::PositionedBlock,
                    context,
                );
            }
            pending.push((context, true));
            if let Some(siblings) = children.get(&context) {
                for &key in siblings.iter().rev().filter(|key| key.z_index < 0) {
                    pending.push((Some(key), false));
                }
            }
        } else {
            if context.is_none() {
                // Root negative contexts belong behind in-flow block backgrounds.
                emit_backgrounds(
                    layout,
                    &mut commands,
                    op_layout::DecorationPaintLayer::Block,
                    None,
                );
            }
            let layer = if context.is_some() {
                op_layout::DecorationPaintLayer::PositionedInline
            } else {
                op_layout::DecorationPaintLayer::Inline
            };
            emit_backgrounds(layout, &mut commands, layer, context);
            emit_foreground(layout, &mut commands, context);
            if let Some(siblings) = children.get(&context) {
                for &key in siblings.iter().rev().filter(|key| key.z_index >= 0) {
                    pending.push((Some(key), false));
                }
            }
        }
    }
    DisplayList { commands }
}

fn emit_backgrounds(
    layout: &LayoutTree,
    commands: &mut Vec<PaintCommand>,
    layer: op_layout::DecorationPaintLayer,
    context: Option<op_layout::PaintKey>,
) {
    for decoration in &layout.box_decorations {
        if decoration.paint_layer == layer && decoration.paint_key == context {
            push_box_decoration(commands, decoration);
        }
    }
}

fn emit_foreground(
    layout: &LayoutTree,
    commands: &mut Vec<PaintCommand>,
    context: Option<op_layout::PaintKey>,
) {
    for item in &layout.order {
        let item_key = match item {
            LayoutItem::Text(_) | LayoutItem::Image(_) => None,
            LayoutItem::PositionedText(_, key) | LayoutItem::PositionedImage(_, key) => Some(*key),
        };
        if item_key != context {
            continue;
        }
        match *item {
            LayoutItem::Text(index) | LayoutItem::PositionedText(index, _) => {
                let Some(text_box) = layout.text_boxes.get(index) else {
                    continue;
                };
                if !text_box.visible {
                    continue;
                }
                commands.push(PaintCommand::Text {
                    x: text_box.x,
                    y: text_box.y,
                    text: text_box.text.clone(),
                    font_size: text_box.font_size,
                    bold: text_box.weight == FontWeight::Bold,
                    italic: text_box.style == FontStyle::Italic,
                    underline: text_box.decoration.underline,
                    line_through: text_box.decoration.line_through,
                    letter_spacing: text_box.letter_spacing,
                    word_spacing: text_box.word_spacing,
                    color: composite_text_color(text_box.color),
                    links: text_box.links.clone(),
                });
            }
            LayoutItem::Image(index) | LayoutItem::PositionedImage(index, _) => {
                let Some(image_box) = layout.image_boxes.get(index) else {
                    continue;
                };
                if !image_box.visible {
                    continue;
                }
                commands.push(PaintCommand::Image {
                    x: image_box.x,
                    y: image_box.y,
                    width: image_box.width,
                    height: image_box.height,
                    image: image_box.image.clone(),
                    href: image_box.href.clone(),
                });
            }
        }
    }
}

fn push_box_decoration(commands: &mut Vec<PaintCommand>, decoration: &BoxDecoration) {
    if decoration.width <= 0 || decoration.height <= 0 {
        return;
    }
    if decoration.background.alpha > 0 {
        commands.push(PaintCommand::FillRect {
            x: decoration.x,
            y: decoration.y,
            width: decoration.width,
            height: decoration.height,
            color: composite_color(decoration.background),
        });
    }

    let top = decoration.border_top.width.clamp(0, decoration.height);
    let bottom = decoration
        .border_bottom
        .width
        .clamp(0, decoration.height.saturating_sub(top));
    let middle_height = decoration.height.saturating_sub(top).saturating_sub(bottom);
    let left = decoration.border_left.width.clamp(0, decoration.width);
    let right = decoration
        .border_right
        .width
        .clamp(0, decoration.width.saturating_sub(left));

    if top > 0 {
        commands.push(PaintCommand::FillRect {
            x: decoration.x,
            y: decoration.y,
            width: decoration.width,
            height: top,
            color: composite_color(decoration.border_top.color),
        });
    }
    if bottom > 0 {
        commands.push(PaintCommand::FillRect {
            x: decoration.x,
            y: decoration.y + decoration.height - bottom,
            width: decoration.width,
            height: bottom,
            color: composite_color(decoration.border_bottom.color),
        });
    }
    if left > 0 && middle_height > 0 {
        commands.push(PaintCommand::FillRect {
            x: decoration.x,
            y: decoration.y + top,
            width: left,
            height: middle_height,
            color: composite_color(decoration.border_left.color),
        });
    }
    if right > 0 && middle_height > 0 {
        commands.push(PaintCommand::FillRect {
            x: decoration.x + decoration.width - right,
            y: decoration.y + top,
            width: right,
            height: middle_height,
            color: composite_color(decoration.border_right.color),
        });
    }
}

fn composite_text_color(color: TextColor) -> Color {
    composite_color(color)
}

fn composite_color(color: TextColor) -> Color {
    fn channel(value: u8, alpha: u8) -> u8 {
        let alpha = u16::from(alpha);
        let value = u16::from(value);
        ((value * alpha + 255 * (255 - alpha) + 127) / 255) as u8
    }

    Color {
        r: channel(color.red, color.alpha),
        g: channel(color.green, color.alpha),
        b: channel(color.blue, color.alpha),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use op_layout::{DecorationBorder, FontStyle, FontWeight, LayoutTree, TextBox, TextDecoration};

    #[test]
    fn positioned_images_paint_after_normal_text_even_when_laid_out_first() {
        let raster =
            Arc::new(RasterImage::from_premultiplied_bgra(1, 1, vec![0, 255, 0, 255]).unwrap());
        let layout = LayoutTree {
            viewport_width: 120,
            content_height: 60,
            box_decorations: vec![],
            text_boxes: vec![TextBox {
                x: 10,
                y: 10,
                width: 60,
                height: 18,
                text: "normal".to_owned(),
                font_size: 16,
                weight: FontWeight::Normal,
                style: FontStyle::Normal,
                decoration: TextDecoration {
                    underline: false,
                    line_through: false,
                },
                letter_spacing: 0,
                word_spacing: 0,
                color: TextColor {
                    red: 0,
                    green: 0,
                    blue: 0,
                    alpha: 255,
                },
                visible: true,
                links: vec![],
            }],
            image_boxes: vec![op_layout::ImageBox {
                x: 10,
                y: 10,
                width: 1,
                height: 1,
                image: raster.clone(),
                visible: true,
                href: None,
            }],
            order: vec![
                LayoutItem::PositionedImage(
                    0,
                    op_layout::PaintKey {
                        z_index: 0,
                        source_order: 1,
                    },
                ),
                LayoutItem::Text(0),
            ],
            paint_groups: vec![],
        };
        let result = build_display_list(&layout, 60);
        assert!(matches!(&result.commands[1], PaintCommand::Text { text, .. } if text == "normal"));
        assert!(
            matches!(&result.commands[2], PaintCommand::Image { image, .. } if Arc::ptr_eq(image, &raster))
        );
    }

    #[test]
    fn positioned_groups_sort_by_z_index_and_stable_source_order() {
        use op_layout::{ImageBox, PaintKey};
        let raster =
            Arc::new(RasterImage::from_premultiplied_bgra(1, 1, vec![0, 0, 0, 255]).unwrap());
        let image = |x| ImageBox {
            x,
            y: 0,
            width: 1,
            height: 1,
            image: raster.clone(),
            visible: true,
            href: None,
        };
        let layout = LayoutTree {
            viewport_width: 100,
            content_height: 20,
            box_decorations: vec![],
            text_boxes: vec![],
            image_boxes: vec![image(10), image(20), image(30), image(40)],
            order: vec![
                LayoutItem::PositionedImage(
                    0,
                    PaintKey {
                        z_index: 5,
                        source_order: 2,
                    },
                ),
                LayoutItem::PositionedImage(
                    1,
                    PaintKey {
                        z_index: -2,
                        source_order: 7,
                    },
                ),
                LayoutItem::PositionedImage(
                    2,
                    PaintKey {
                        z_index: 0,
                        source_order: 1,
                    },
                ),
                LayoutItem::PositionedImage(
                    3,
                    PaintKey {
                        z_index: 5,
                        source_order: 1,
                    },
                ),
            ],
            paint_groups: vec![],
        };
        let ordered_x: Vec<_> = build_display_list(&layout, 20)
            .commands
            .iter()
            .filter_map(|item| match item {
                PaintCommand::Image { x, .. } => Some(*x),
                _ => None,
            })
            .collect();
        assert_eq!(ordered_x, [20, 30, 40, 10]);
    }

    #[test]
    fn inline_backgrounds_paint_above_later_overlapping_block_backgrounds() {
        use op_layout::DecorationPaintLayer;
        let border = DecorationBorder {
            width: 0,
            color: TextColor {
                red: 0,
                green: 0,
                blue: 0,
                alpha: 0,
            },
        };
        let decoration = |layer, y, color| BoxDecoration {
            paint_layer: layer,
            paint_key: None,
            x: 10,
            y,
            width: 40,
            height: 20,
            background: color,
            border_top: border,
            border_right: border,
            border_bottom: border,
            border_left: border,
        };
        let green = TextColor {
            red: 0,
            green: 128,
            blue: 0,
            alpha: 255,
        };
        let red = TextColor {
            red: 255,
            green: 0,
            blue: 0,
            alpha: 255,
        };
        let white = TextColor {
            red: 255,
            green: 255,
            blue: 255,
            alpha: 255,
        };
        let layout = LayoutTree {
            viewport_width: 200,
            content_height: 90,
            // The next block was appended after the inline in layout order.
            box_decorations: vec![
                decoration(DecorationPaintLayer::Inline, 20, green),
                decoration(DecorationPaintLayer::Block, 35, white),
                decoration(DecorationPaintLayer::Inline, 20, red),
            ],
            text_boxes: vec![],
            image_boxes: vec![],
            order: vec![],
            paint_groups: vec![],
        };
        let commands = build_display_list(&layout, 90).commands;
        let backgrounds: Vec<Color> = commands
            .iter()
            .filter_map(|item| match item {
                PaintCommand::FillRect {
                    x: 10,
                    y,
                    width: 40,
                    height: 20,
                    color,
                } if *y == 20 || *y == 35 => Some(*color),
                _ => None,
            })
            .collect();
        assert_eq!(
            backgrounds,
            [
                Color::WHITE,
                Color { r: 0, g: 128, b: 0 },
                Color { r: 255, g: 0, b: 0 }
            ]
        );
    }

    #[test]
    fn composites_css_text_alpha_over_white_page_background() {
        assert_eq!(
            composite_text_color(TextColor {
                red: 0,
                green: 0,
                blue: 0,
                alpha: 0,
            }),
            Color::WHITE
        );
        assert_eq!(
            composite_text_color(TextColor {
                red: 255,
                green: 0,
                blue: 0,
                alpha: 128,
            }),
            Color {
                r: 255,
                g: 127,
                b: 127,
            }
        );
    }

    #[test]
    fn creates_block_background_and_border_fill_commands() {
        let layout = LayoutTree {
            viewport_width: 300,
            content_height: 120,
            box_decorations: vec![BoxDecoration {
                paint_layer: op_layout::DecorationPaintLayer::Block,
                paint_key: None,
                x: 20,
                y: 30,
                width: 200,
                height: 60,
                background: TextColor {
                    red: 240,
                    green: 244,
                    blue: 255,
                    alpha: 255,
                },
                border_top: DecorationBorder {
                    width: 2,
                    color: TextColor {
                        red: 12,
                        green: 34,
                        blue: 56,
                        alpha: 255,
                    },
                },
                border_right: DecorationBorder {
                    width: 3,
                    color: TextColor {
                        red: 34,
                        green: 56,
                        blue: 78,
                        alpha: 255,
                    },
                },
                border_bottom: DecorationBorder {
                    width: 4,
                    color: TextColor {
                        red: 56,
                        green: 78,
                        blue: 90,
                        alpha: 255,
                    },
                },
                border_left: DecorationBorder {
                    width: 5,
                    color: TextColor {
                        red: 78,
                        green: 90,
                        blue: 12,
                        alpha: 255,
                    },
                },
            }],
            text_boxes: vec![],
            image_boxes: vec![],
            order: vec![],
            paint_groups: vec![],
        };

        let display_list = build_display_list(&layout, 120);
        assert_eq!(display_list.commands.len(), 6);
        assert_eq!(
            display_list.commands[1],
            PaintCommand::FillRect {
                x: 20,
                y: 30,
                width: 200,
                height: 60,
                color: Color {
                    r: 240,
                    g: 244,
                    b: 255,
                },
            }
        );
        assert_eq!(
            display_list.commands[2],
            PaintCommand::FillRect {
                x: 20,
                y: 30,
                width: 200,
                height: 2,
                color: Color {
                    r: 12,
                    g: 34,
                    b: 56,
                },
            }
        );
        assert_eq!(
            display_list.commands[3],
            PaintCommand::FillRect {
                x: 20,
                y: 86,
                width: 200,
                height: 4,
                color: Color {
                    r: 56,
                    g: 78,
                    b: 90,
                },
            }
        );
        assert_eq!(
            display_list.commands[4],
            PaintCommand::FillRect {
                x: 20,
                y: 32,
                width: 5,
                height: 54,
                color: Color {
                    r: 78,
                    g: 90,
                    b: 12,
                },
            }
        );
        assert_eq!(
            display_list.commands[5],
            PaintCommand::FillRect {
                x: 217,
                y: 32,
                width: 3,
                height: 54,
                color: Color {
                    r: 34,
                    g: 56,
                    b: 78,
                },
            }
        );
    }

    #[test]
    fn hidden_text_keeps_layout_record_but_emits_no_paint_command() {
        let layout = LayoutTree {
            viewport_width: 320,
            content_height: 40,
            box_decorations: vec![],
            text_boxes: vec![TextBox {
                x: 12,
                y: 8,
                width: 80,
                height: 20,
                text: "hidden".into(),
                font_size: 18,
                weight: FontWeight::Normal,
                style: FontStyle::Normal,
                decoration: TextDecoration {
                    underline: false,
                    line_through: false,
                },
                letter_spacing: 0,
                word_spacing: 0,
                color: TextColor {
                    red: 0,
                    green: 0,
                    blue: 0,
                    alpha: 0,
                },
                visible: false,
                links: Vec::new(),
            }],
            image_boxes: vec![],
            order: vec![LayoutItem::Text(0)],
            paint_groups: vec![],
        };

        let display_list = build_display_list(&layout, 200);
        assert_eq!(display_list.commands.len(), 1);
        assert!(matches!(
            display_list.commands[0],
            PaintCommand::FillRect { .. }
        ));
    }

    #[test]
    fn creates_background_and_text_commands() {
        let layout = LayoutTree {
            viewport_width: 800,
            content_height: 120,
            box_decorations: vec![],
            text_boxes: vec![TextBox {
                x: 32,
                y: 40,
                width: 736,
                height: 32,
                text: "OPBrowser".into(),
                font_size: 24,
                weight: FontWeight::Bold,
                style: FontStyle::Italic,
                decoration: TextDecoration {
                    underline: true,
                    line_through: true,
                },
                letter_spacing: 2,
                word_spacing: 3,
                color: TextColor {
                    red: 12,
                    green: 34,
                    blue: 56,
                    alpha: 255,
                },
                visible: true,
                links: Vec::new(),
            }],
            image_boxes: vec![],
            order: vec![LayoutItem::Text(0)],
            paint_groups: vec![],
        };

        let display_list = build_display_list(&layout, 600);

        assert_eq!(display_list.commands.len(), 2);
        assert!(matches!(
            display_list.commands[0],
            PaintCommand::FillRect {
                width: 800,
                height: 600,
                ..
            }
        ));

        assert_eq!(
            display_list.commands[1],
            PaintCommand::Text {
                x: 32,
                y: 40,
                text: "OPBrowser".into(),
                font_size: 24,
                bold: true,
                italic: true,
                underline: true,
                line_through: true,
                letter_spacing: 2,
                word_spacing: 3,
                color: Color {
                    r: 12,
                    g: 34,
                    b: 56,
                },
                links: Vec::new(),
            }
        );
    }
}
