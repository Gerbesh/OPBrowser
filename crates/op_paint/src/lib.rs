pub use op_image::RasterImage;
pub use op_layout::LinkSpan;
use op_layout::{FontWeight, LayoutItem, LayoutTree};
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

    pub const BLACK: Self = Self {
        r: 18,
        g: 18,
        b: 18,
    };

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
    let mut commands = Vec::with_capacity(layout.text_boxes.len() + layout.image_boxes.len() + 1);

    commands.push(PaintCommand::FillRect {
        x: 0,
        y: 0,
        width: layout.viewport_width,
        height: viewport_height.max(layout.content_height),
        color: Color::WHITE,
    });

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
                    color: Color::BLACK,
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

#[cfg(test)]
mod tests {
    use super::*;
    use op_layout::{FontWeight, LayoutTree, TextBox};

    #[test]
    fn creates_background_and_text_commands() {
        let layout = LayoutTree {
            viewport_width: 800,
            content_height: 120,
            text_boxes: vec![TextBox {
                x: 32,
                y: 40,
                width: 736,
                height: 32,
                text: "OPBrowser".into(),
                font_size: 24,
                weight: FontWeight::Bold,
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
                color: Color::BLACK,
                links: Vec::new(),
            }
        );
    }
}
