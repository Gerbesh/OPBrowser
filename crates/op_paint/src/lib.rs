use op_layout::{FontWeight, LayoutTree};

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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaintCommand {
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
    },
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DisplayList {
    pub commands: Vec<PaintCommand>,
}

pub fn build_display_list(layout: &LayoutTree, viewport_height: i32) -> DisplayList {
    let mut commands = Vec::with_capacity(layout.text_boxes.len() + 1);

    commands.push(PaintCommand::FillRect {
        x: 0,
        y: 0,
        width: layout.viewport_width,
        height: viewport_height.max(layout.content_height),
        color: Color::WHITE,
    });

    for text_box in &layout.text_boxes {
        commands.push(PaintCommand::Text {
            x: text_box.x,
            y: text_box.y,
            text: text_box.text.clone(),
            font_size: text_box.font_size,
            bold: text_box.weight == FontWeight::Bold,
            color: Color::BLACK,
        });
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
            }],
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
            }
        );
    }
}
