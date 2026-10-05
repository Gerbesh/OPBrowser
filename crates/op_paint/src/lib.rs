pub use op_image::RasterImage;
pub use op_layout::LinkSpan;
use op_layout::{BoxDecoration, FontWeight, LayoutItem, LayoutTree, TextColor};
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

    for decoration in &layout.box_decorations {
        push_box_decoration(&mut commands, decoration);
    }

    for item in &layout.order {
        match *item {
            LayoutItem::Text(index) => {
                let Some(text_box) = layout.text_boxes.get(index) else {
                    continue;
                };
                commands.push(PaintCommand::Text {
                    x: text_box.x,
                    y: text_box.y,
                    text: text_box.text.clone(),
                    font_size: text_box.font_size,
                    bold: text_box.weight == FontWeight::Bold,
                    color: composite_text_color(text_box.color),
                    links: text_box.links.clone(),
                });
            }
            LayoutItem::Image(index) => {
                let Some(image_box) = layout.image_boxes.get(index) else {
                    continue;
                };
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

    DisplayList { commands }
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

    let border = decoration
        .border_width
        .clamp(0, decoration.width.min(decoration.height) / 2);
    if border == 0 {
        return;
    }
    let color = composite_color(decoration.border_color);
    commands.push(PaintCommand::FillRect {
        x: decoration.x,
        y: decoration.y,
        width: decoration.width,
        height: border,
        color,
    });
    commands.push(PaintCommand::FillRect {
        x: decoration.x,
        y: decoration.y + decoration.height - border,
        width: decoration.width,
        height: border,
        color,
    });
    commands.push(PaintCommand::FillRect {
        x: decoration.x,
        y: decoration.y + border,
        width: border,
        height: decoration.height - border * 2,
        color,
    });
    commands.push(PaintCommand::FillRect {
        x: decoration.x + decoration.width - border,
        y: decoration.y + border,
        width: border,
        height: decoration.height - border * 2,
        color,
    });
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
    use op_layout::{FontWeight, LayoutTree, TextBox};

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
                border_width: 2,
                border_color: TextColor {
                    red: 12,
                    green: 34,
                    blue: 56,
                    alpha: 255,
                },
            }],
            text_boxes: vec![],
            image_boxes: vec![],
            order: vec![],
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
        assert!(display_list.commands[2..].iter().all(|command| matches!(
            command,
            PaintCommand::FillRect {
                color: Color {
                    r: 12,
                    g: 34,
                    b: 56
                },
                ..
            }
        )));
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
                color: TextColor {
                    red: 12,
                    green: 34,
                    blue: 56,
                    alpha: 255,
                },
                links: Vec::new(),
            }],
            image_boxes: vec![],
            order: vec![LayoutItem::Text(0)],
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
