use op_html::parse_document;
use op_layout::layout_document;
use op_net::{LoadError, LoadedDocument, NetworkContext};
use op_paint::{DisplayList, build_display_list};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineState {
    Created,
    Running,
    ShuttingDown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NavigationEntry {
    pub request: String,
    pub address: String,
    pub mime_type: String,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct NavigationState {
    entries: Vec<NavigationEntry>,
    current_index: Option<usize>,
}

impl NavigationState {
    pub fn entries(&self) -> &[NavigationEntry] {
        &self.entries
    }

    pub fn current_index(&self) -> Option<usize> {
        self.current_index
    }

    pub fn current(&self) -> Option<&NavigationEntry> {
        self.current_index.and_then(|index| self.entries.get(index))
    }

    pub fn can_go_back(&self) -> bool {
        self.current_index.is_some_and(|index| index > 0)
    }

    pub fn can_go_forward(&self) -> bool {
        self.current_index
            .is_some_and(|index| index + 1 < self.entries.len())
    }

    fn commit_navigation(&mut self, entry: NavigationEntry) {
        if let Some(index) = self.current_index {
            self.entries.truncate(index + 1);
        } else {
            self.entries.clear();
        }

        self.entries.push(entry);
        self.current_index = Some(self.entries.len() - 1);
    }

    fn back_target(&self) -> Option<(usize, String)> {
        let current = self.current_index?;
        let target = current.checked_sub(1)?;
        Some((target, self.entries.get(target)?.request.clone()))
    }

    fn forward_target(&self) -> Option<(usize, String)> {
        let target = self.current_index?.checked_add(1)?;
        Some((target, self.entries.get(target)?.request.clone()))
    }

    fn current_request(&self) -> Option<String> {
        Some(self.current()?.request.clone())
    }

    fn set_current_index(&mut self, index: usize) {
        debug_assert!(index < self.entries.len());
        self.current_index = Some(index);
    }
}

#[derive(Debug)]
pub struct RenderedPage {
    pub address: String,
    pub mime_type: String,
    pub display_list: DisplayList,
}

#[derive(Debug)]
pub struct Engine {
    state: EngineState,
    network: NetworkContext,
    navigation: NavigationState,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            state: EngineState::Created,
            network: NetworkContext,
            navigation: NavigationState::default(),
        }
    }

    pub fn start(&mut self) {
        self.state = EngineState::Running;
    }

    pub fn state(&self) -> EngineState {
        self.state
    }

    pub fn navigation(&self) -> &NavigationState {
        &self.navigation
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

    pub fn render_source(
        &self,
        source: &str,
        viewport_width: i32,
        viewport_height: i32,
    ) -> Result<RenderedPage, LoadError> {
        let loaded = self.network.load_document(source)?;
        Ok(self.render_loaded_document(loaded, viewport_width, viewport_height))
    }

    pub fn navigate(
        &mut self,
        source: &str,
        viewport_width: i32,
        viewport_height: i32,
    ) -> Result<RenderedPage, LoadError> {
        let page = self.render_source(source, viewport_width, viewport_height)?;
        self.navigation.commit_navigation(NavigationEntry {
            request: source.to_owned(),
            address: page.address.clone(),
            mime_type: page.mime_type.clone(),
        });
        Ok(page)
    }

    pub fn go_back(
        &mut self,
        viewport_width: i32,
        viewport_height: i32,
    ) -> Result<Option<RenderedPage>, LoadError> {
        let Some((target_index, request)) = self.navigation.back_target() else {
            return Ok(None);
        };

        let page = self.render_source(&request, viewport_width, viewport_height)?;
        self.navigation.set_current_index(target_index);
        Ok(Some(page))
    }

    pub fn go_forward(
        &mut self,
        viewport_width: i32,
        viewport_height: i32,
    ) -> Result<Option<RenderedPage>, LoadError> {
        let Some((target_index, request)) = self.navigation.forward_target() else {
            return Ok(None);
        };

        let page = self.render_source(&request, viewport_width, viewport_height)?;
        self.navigation.set_current_index(target_index);
        Ok(Some(page))
    }

    pub fn reload(
        &self,
        viewport_width: i32,
        viewport_height: i32,
    ) -> Result<Option<RenderedPage>, LoadError> {
        let Some(request) = self.navigation.current_request() else {
            return Ok(None);
        };

        self.render_source(&request, viewport_width, viewport_height)
            .map(Some)
    }

    fn render_loaded_document(
        &self,
        loaded: LoadedDocument,
        viewport_width: i32,
        viewport_height: i32,
    ) -> RenderedPage {
        let display_list = self.render_html(&loaded.text, viewport_width, viewport_height);

        RenderedPage {
            address: loaded.address,
            mime_type: loaded.mime_type,
            display_list,
        }
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

        assert!(contains_text(&display_list, "OPBrowser"));
        assert!(contains_text(&display_list, "Own engine."));
    }

    #[test]
    fn render_source_loads_data_html_before_rendering() {
        let engine = Engine::new();
        let page = engine
            .render_source(
                "data:text/html,%3Ch1%3EExternal%20source%3C%2Fh1%3E",
                800,
                600,
            )
            .unwrap();

        assert_eq!(page.address, "data:text/html");
        assert_eq!(page.mime_type, "text/html");
        assert!(contains_text(&page.display_list, "External source"));
        assert!(engine.navigation().entries().is_empty());
    }

    #[test]
    fn navigation_tracks_back_forward_and_reload_without_duplicate_entries() {
        let mut engine = Engine::new();

        engine
            .navigate("data:text/html,%3Ch1%3EOne%3C%2Fh1%3E", 800, 600)
            .unwrap();
        engine
            .navigate("data:text/html,%3Ch1%3ETwo%3C%2Fh1%3E", 800, 600)
            .unwrap();
        engine
            .navigate("data:text/html,%3Ch1%3EThree%3C%2Fh1%3E", 800, 600)
            .unwrap();

        assert_eq!(engine.navigation().entries().len(), 3);
        assert_eq!(engine.navigation().current_index(), Some(2));
        assert!(engine.navigation().can_go_back());
        assert!(!engine.navigation().can_go_forward());

        let back = engine.go_back(800, 600).unwrap().unwrap();
        assert!(contains_text(&back.display_list, "Two"));
        assert_eq!(engine.navigation().current_index(), Some(1));
        assert!(engine.navigation().can_go_forward());

        let reload = engine.reload(800, 600).unwrap().unwrap();
        assert!(contains_text(&reload.display_list, "Two"));
        assert_eq!(engine.navigation().entries().len(), 3);
        assert_eq!(engine.navigation().current_index(), Some(1));

        let forward = engine.go_forward(800, 600).unwrap().unwrap();
        assert!(contains_text(&forward.display_list, "Three"));
        assert_eq!(engine.navigation().current_index(), Some(2));
    }

    #[test]
    fn new_navigation_after_back_discards_forward_history() {
        let mut engine = Engine::new();

        engine
            .navigate("data:text/html,%3Cp%3EOne%3C%2Fp%3E", 800, 600)
            .unwrap();
        engine
            .navigate("data:text/html,%3Cp%3ETwo%3C%2Fp%3E", 800, 600)
            .unwrap();
        engine
            .navigate("data:text/html,%3Cp%3EThree%3C%2Fp%3E", 800, 600)
            .unwrap();

        engine.go_back(800, 600).unwrap().unwrap();
        engine
            .navigate("data:text/html,%3Cp%3EBranch%3C%2Fp%3E", 800, 600)
            .unwrap();

        assert_eq!(engine.navigation().entries().len(), 3);
        assert_eq!(engine.navigation().current_index(), Some(2));
        assert!(!engine.navigation().can_go_forward());
        assert!(
            engine
                .navigation()
                .current()
                .unwrap()
                .request
                .contains("Branch")
        );
    }

    #[test]
    fn failed_navigation_does_not_change_history() {
        let mut engine = Engine::new();

        engine
            .navigate("data:text/html,%3Cp%3EStable%3C%2Fp%3E", 800, 600)
            .unwrap();

        let before = engine.navigation().clone();
        assert!(engine.navigate("https://example.com", 800, 600).is_err());
        assert_eq!(*engine.navigation(), before);
    }

    #[test]
    fn back_and_forward_without_targets_are_noops() {
        let mut engine = Engine::new();

        assert!(engine.go_back(800, 600).unwrap().is_none());
        assert!(engine.go_forward(800, 600).unwrap().is_none());

        engine
            .navigate("data:text/html,%3Cp%3EOnly%3C%2Fp%3E", 800, 600)
            .unwrap();

        assert!(engine.go_back(800, 600).unwrap().is_none());
        assert!(engine.go_forward(800, 600).unwrap().is_none());
    }

    fn contains_text(display_list: &DisplayList, expected: &str) -> bool {
        display_list.commands.iter().any(|command| {
            matches!(
                command,
                PaintCommand::Text { text, .. } if text == expected
            )
        })
    }
}
