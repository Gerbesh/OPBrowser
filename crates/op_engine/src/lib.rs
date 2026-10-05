use op_html::parse_document;
use op_layout::layout_document;
use op_net::{LoadError, LoadedDocument, NetworkContext, resolve_link};
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
    document_address: Option<String>,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            state: EngineState::Created,
            network: NetworkContext,
            navigation: NavigationState::default(),
            document_address: None,
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
        self.document_address = Some(page.address.clone());
        self.navigation.commit_navigation(NavigationEntry {
            request: source.to_owned(),
            address: page.address.clone(),
            mime_type: page.mime_type.clone(),
        });
        Ok(page)
    }

    pub fn follow_link(
        &mut self,
        href: &str,
        viewport_width: i32,
        viewport_height: i32,
    ) -> Result<RenderedPage, LoadError> {
        let source = resolve_link(self.document_address.as_deref(), href)?;
        self.navigate(&source, viewport_width, viewport_height)
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
        self.document_address = Some(page.address.clone());
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
        self.document_address = Some(page.address.clone());
        Ok(Some(page))
    }

    pub fn reload(
        &mut self,
        viewport_width: i32,
        viewport_height: i32,
    ) -> Result<Option<RenderedPage>, LoadError> {
        let Some(request) = self.navigation.current_request() else {
            return Ok(None);
        };

        let page = self.render_source(&request, viewport_width, viewport_height)?;
        self.document_address = Some(page.address.clone());
        Ok(Some(page))
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
    fn nested_site_containers_keep_heading_and_paragraph_blocks() {
        let display_list = Engine::new().render_html(
            "<!doctype html><html><head><style>hidden</style></head><body><main><div><h1>Example Domain</h1><p>Visible text</p></div></main></body></html>", 800, 600,
        );
        assert!(display_list.commands.iter().any(|command| matches!(command, PaintCommand::Text { text, font_size: 34, bold: true, .. } if text == "Example Domain")));
        assert!(contains_text(&display_list, "Visible text"));
        assert!(!contains_text(&display_list, "hidden"));
    }

    #[test]
    fn legacy_document_text_and_entity_decoded_links_reach_paint() {
        let mut engine = Engine::new();
        let page = engine.navigate("data:text/html;charset=windows-1251,%3Ch1%3E%CF%F0%E8%E2%E5%F2%3C%2Fh1%3E%3Cp%3E%26copy%3B%3C%2Fp%3E%3Cp%3E%3Ca%20href%3D%27https%3A%2F%2Fexample.com%2F%3Fa%3D1%26amp%3Bb%3D2%27%3EGo%3C%2Fa%3E%3C%2Fp%3E", 800, 600).unwrap();
        assert!(contains_text(&page.display_list, "Привет"));
        assert!(contains_text(&page.display_list, "©"));
        assert!(page.display_list.commands.iter().any(|command| matches!(command, PaintCommand::Text { links, .. } if links.iter().any(|link| link.href == "https://example.com/?a=1&b=2"))));
        let before = engine.navigation().clone();
        assert!(
            engine
                .navigate("data:text/html;charset=unknown,x", 800, 600)
                .is_err()
        );
        assert_eq!(*engine.navigation(), before);
    }

    #[test]
    fn local_windows1251_fixture_renders_cyrillic_and_decoded_query_link() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/encoding/windows-1251.html");
        let mut engine = Engine::new();
        let page = engine.navigate(path.to_str().unwrap(), 800, 600).unwrap();
        assert!(contains_text(&page.display_list, "Привет, мир!"));
        assert!(contains_text(&page.display_list, "Windows-1251: Ё ё №"));
        assert!(contains_text(&page.display_list, "© 2026 & OPBrowser"));
        assert!(page.display_list.commands.iter().any(|command| matches!(command, PaintCommand::Text { links, .. } if links.iter().any(|link| link.href == "../navigation/index.html?source=encoding&lang=ru"))));
        let next = engine
            .follow_link("../navigation/index.html?source=encoding&lang=ru", 800, 600)
            .unwrap();
        assert!(contains_text(
            &next.display_list,
            "OPBrowser link navigation"
        ));
        assert!(engine.navigation().can_go_back());
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
        assert!(engine.navigate("ftp://example.com", 800, 600).is_err());
        assert_eq!(*engine.navigation(), before);
    }

    #[test]
    #[cfg(windows)]
    fn http_navigation_renders_and_preserves_history_on_failed_operations() {
        use std::io::{Read, Write};
        use std::net::TcpListener;
        use std::time::{Duration, Instant};
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = format!("http://{}", listener.local_addr().unwrap());
        listener.set_nonblocking(true).unwrap();
        let server = std::thread::spawn(move || {
            for (index, status) in [302, 200, 404, 200, 200, 200, 302, 200, 200, 500, 500]
                .into_iter()
                .enumerate()
            {
                let started = Instant::now();
                let mut stream = loop {
                    match listener.accept() {
                        Ok((stream, _)) => break stream,
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            assert!(started.elapsed() < Duration::from_secs(5));
                            std::thread::sleep(Duration::from_millis(5));
                        }
                        Err(error) => panic!("{error}"),
                    }
                };
                stream.set_nonblocking(false).unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(3)))
                    .unwrap();
                let mut request = String::new();
                while !request.ends_with("\r\n\r\n") {
                    let mut buffer = [0; 1024];
                    let count = stream.read(&mut buffer).unwrap();
                    assert!(count > 0 && request.len() < 32 * 1024);
                    request.push_str(std::str::from_utf8(&buffer[..count]).unwrap());
                }
                if status == 302 {
                    let location = if index == 0 {
                        "/nested/one"
                    } else {
                        "/after-reload/two"
                    };
                    write!(stream, "HTTP/1.1 302 Found\r\nLocation: {location}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").unwrap();
                    continue;
                }
                let text = if request.starts_with("GET /nested/one ")
                    || request.starts_with("GET /start ")
                {
                    "One"
                } else {
                    "Two"
                };
                let body = format!("<h1>{text}</h1>");
                write!(stream, "HTTP/1.1 {status} Test\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
            }
        });
        let mut engine = Engine::new();
        let first = engine
            .navigate(&format!("{address}/start"), 800, 600)
            .unwrap();
        assert!(contains_text(&first.display_list, "One"));
        assert_eq!(first.address, format!("{address}/nested/one"));
        let before = engine.navigation().clone();
        assert_eq!(
            engine
                .navigate(&format!("{address}/missing"), 800, 600)
                .unwrap_err(),
            LoadError::HttpStatus(404)
        );
        assert_eq!(*engine.navigation(), before);
        engine.follow_link("two", 800, 600).unwrap();
        assert_eq!(
            engine.navigation().current().unwrap().request,
            format!("{address}/nested/two")
        );
        assert!(contains_text(
            &engine.go_back(800, 600).unwrap().unwrap().display_list,
            "One"
        ));
        assert!(contains_text(
            &engine.go_forward(800, 600).unwrap().unwrap().display_list,
            "Two"
        ));
        assert!(contains_text(
            &engine.reload(800, 600).unwrap().unwrap().display_list,
            "Two"
        ));
        engine.follow_link("next", 800, 600).unwrap();
        assert_eq!(
            engine.navigation().current().unwrap().request,
            format!("{address}/after-reload/next")
        );
        let before = engine.navigation().clone();
        assert_eq!(
            engine.go_back(800, 600).unwrap_err(),
            LoadError::HttpStatus(500)
        );
        assert_eq!(*engine.navigation(), before);
        assert_eq!(
            engine.reload(800, 600).unwrap_err(),
            LoadError::HttpStatus(500)
        );
        assert_eq!(*engine.navigation(), before);
        server.join().unwrap();
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
