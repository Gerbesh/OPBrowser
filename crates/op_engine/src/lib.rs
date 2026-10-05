use op_html::parse_document;
use op_layout::layout_document;
use op_paint::{DisplayList, build_display_list};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineState {
    Created,
    Running,
    ShuttingDown,
}

#[derive(Debug)]
pub struct Engine {
    state: EngineState,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            state: EngineState::Created,
        }
    }

    pub fn start(&mut self) {
        self.state = EngineState::Running;
    }

    pub fn state(&self) -> EngineState {
        self.state
    }

    pub fn render_html(
        &self,
        html: &str,
        viewport_width: i32,
        viewport_height: i32,
    ) -> DisplayList {
        let document = parse_document(html);
        let layout = layout_document(&document, viewport_width);
        build_display_list(&layout, viewport_height)
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use op_paint::PaintCommand;

    #[test]
    fn engine_starts_in_created_state() {
        assert_eq!(Engine::new().state(), EngineState::Created);
    }

    #[test]
    fn engine_can_start() {
        let mut engine = Engine::new();
        engine.start();
        assert_eq!(engine.state(), EngineState::Running);
    }

    #[test]
    fn render_html_produces_text_paint_commands() {
        let mut engine = Engine::new();
        engine.start();

        let display_list = engine.render_html(
            "<html><body><h1>OPBrowser</h1><p>Own engine.</p></body></html>",
            800,
            600,
        );

        assert!(display_list.commands.iter().any(|command| {
            matches!(
                command,
                PaintCommand::Text { text, .. } if text == "OPBrowser"
            )
        }));

        assert!(display_list.commands.iter().any(|command| {
            matches!(
                command,
                PaintCommand::Text { text, .. } if text == "Own engine."
            )
        }));
    }
}
