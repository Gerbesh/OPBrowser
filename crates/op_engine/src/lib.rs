use op_css::{
    ComputedStyleMap, StyleCollection, StyleError, StyleMap, collect_author_styles,
    collect_author_styles_with_linked, compute_styles,
};
use op_html::parse_document;
use op_layout::{ImageResources, layout_document_with_computed_styles_and_metrics};
use op_net::{LoadError, LoadedDocument, NetworkContext, resolve_link};
use op_paint::{DisplayList, build_display_list};
mod images;
mod styles;
mod text;

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
struct PreparedDocument {
    address: String,
    mime_type: String,
    document: op_dom::Document,
    images: ImageResources,
    style_collection: StyleCollection,
    computed_styles: ComputedStyleMap,
}

impl PreparedDocument {
    fn render(&self, width: i32, height: i32) -> RenderedPage {
        let layout = layout_document_with_computed_styles_and_metrics(
            &self.document,
            width,
            &self.images,
            &self.computed_styles,
            &mut text::Measurer::new(),
        );
        RenderedPage {
            address: self.address.clone(),
            mime_type: self.mime_type.clone(),
            display_list: build_display_list(&layout, height),
        }
    }
}

#[derive(Debug)]
pub struct Engine {
    state: EngineState,
    network: NetworkContext,
    navigation: NavigationState,
    document_address: Option<String>,
    active_document: Option<PreparedDocument>,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            state: EngineState::Created,
            network: NetworkContext,
            navigation: NavigationState::default(),
            document_address: None,
            active_document: None,
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

    pub fn active_styles(&self) -> Option<&StyleMap> {
        Some(&self.active_document.as_ref()?.style_collection.styles)
    }

    pub fn active_style_errors(&self) -> Option<&[StyleError]> {
        Some(&self.active_document.as_ref()?.style_collection.errors)
    }

    pub fn active_computed_styles(&self) -> Option<&ComputedStyleMap> {
        Some(&self.active_document.as_ref()?.computed_styles)
    }

    pub fn render_html(
        &self,
        html: &str,
        viewport_width: i32,
        viewport_height: i32,
    ) -> DisplayList {
        let document = parse_document(html);
        let style_collection = collect_author_styles(&document);
        let computed_styles = compute_styles(&document, &style_collection.styles);
        let layout = layout_document_with_computed_styles_and_metrics(
            &document,
            viewport_width,
            &ImageResources::new(),
            &computed_styles,
            &mut text::Measurer::new(),
        );
        build_display_list(&layout, viewport_height)
    }

    pub fn render_source(
        &self,
        source: &str,
        viewport_width: i32,
        viewport_height: i32,
    ) -> Result<RenderedPage, LoadError> {
        Ok(self
            .prepare_source(source)?
            .render(viewport_width, viewport_height))
    }

    /// Initialize an in-memory page and an empty navigation history for startup.
    pub fn set_html_page(&mut self, html: &str, width: i32, height: i32) -> DisplayList {
        let document = parse_document(html);
        let style_collection = collect_author_styles(&document);
        let computed_styles = compute_styles(&document, &style_collection.styles);
        let prepared = PreparedDocument {
            address: String::new(),
            mime_type: "text/html".into(),
            document,
            images: ImageResources::new(),
            style_collection,
            computed_styles,
        };
        let page = prepared.render(width, height);
        self.active_document = Some(prepared);
        self.document_address = None;
        self.navigation = NavigationState::default();
        page.display_list
    }

    /// Rebuild only layout/paint from the current DOM and shared image pixels.
    pub fn reflow(&self, width: i32, height: i32) -> Option<RenderedPage> {
        Some(self.active_document.as_ref()?.render(width, height))
    }

    fn load_active(
        &mut self,
        source: &str,
        width: i32,
        height: i32,
    ) -> Result<RenderedPage, LoadError> {
        let prepared = self.prepare_source(source)?;
        let page = prepared.render(width, height);
        self.active_document = Some(prepared);
        self.document_address = Some(page.address.clone());
        Ok(page)
    }

    pub fn navigate(
        &mut self,
        source: &str,
        viewport_width: i32,
        viewport_height: i32,
    ) -> Result<RenderedPage, LoadError> {
        let page = self.load_active(source, viewport_width, viewport_height)?;
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

        let page = self.load_active(&request, viewport_width, viewport_height)?;
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

        let page = self.load_active(&request, viewport_width, viewport_height)?;
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

        let page = self.load_active(&request, viewport_width, viewport_height)?;
        self.document_address = Some(page.address.clone());
        Ok(Some(page))
    }

    fn prepare_source(&self, source: &str) -> Result<PreparedDocument, LoadError> {
        let loaded: LoadedDocument = self.network.load_document(source)?;
        let document = parse_document(&loaded.text);
        let linked_stylesheets = styles::load(&self.network, &document, &loaded.address);
        let style_collection = collect_author_styles_with_linked(&document, &linked_stylesheets);
        let computed_styles = compute_styles(&document, &style_collection.styles);
        let images = images::load(&self.network, &document, &loaded.address);
        Ok(PreparedDocument {
            address: loaded.address,
            mime_type: loaded.mime_type,
            document,
            images,
            style_collection,
            computed_styles,
        })
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
    fn retains_author_style_candidates_and_errors_across_reflow() {
        fn find_tag(
            document: &op_dom::Document,
            node: op_dom::NodeId,
            tag: &str,
        ) -> Option<op_dom::NodeId> {
            if document
                .element(node)
                .is_some_and(|element| element.tag_name == tag)
            {
                return Some(node);
            }
            document
                .children(node)
                .iter()
                .find_map(|child| find_tag(document, *child, tag))
        }

        let mut engine = Engine::new();
        engine.set_html_page(
            "<style>.note { color: red } p.note { margin: 1px }</style><p class='note' style='color: blue; broken'>Styled</p>",
            800,
            600,
        );

        let paragraph = {
            let document = &engine.active_document.as_ref().unwrap().document;
            find_tag(document, document.root(), "p").unwrap()
        };
        let declarations = engine.active_styles().unwrap().declarations_for(paragraph);
        assert_eq!(declarations.len(), 3);
        assert!(declarations.iter().any(|matched| {
            matched.declaration.name == "color" && matched.source == op_css::StyleSource::Stylesheet
        }));
        assert!(declarations.iter().any(|matched| {
            matched.declaration.name == "color" && matched.source == op_css::StyleSource::Inline
        }));
        assert_eq!(engine.active_style_errors().unwrap().len(), 1);

        let computed = engine
            .active_computed_styles()
            .unwrap()
            .style_for(paragraph)
            .copied()
            .unwrap();
        assert_eq!(computed.color, op_css::CssColor::BLUE);
        assert_eq!(computed.font_size_px, 18.0);
        assert_eq!(computed.display, op_css::Display::Block);

        let retained = engine.active_styles().unwrap().clone();
        let retained_computed = engine.active_computed_styles().unwrap().clone();
        engine.reflow(320, 300).unwrap();
        assert_eq!(engine.active_styles().unwrap(), &retained);
        assert_eq!(engine.active_computed_styles().unwrap(), &retained_computed);
    }

    #[test]
    fn reflow_keeps_start_page_and_tracks_only_successful_history_changes() {
        let mut engine = Engine::new();
        assert!(engine.reflow(320, 300).is_none());
        engine.set_html_page("<p>Start page</p>", 800, 600);
        assert!(contains_text(
            &engine.reflow(320, 300).unwrap().display_list,
            "Start page"
        ));
        assert!(engine.navigation().entries().is_empty());
        assert!(engine.reload(320, 300).unwrap().is_none());
        let first = "data:text/html,%3Cp%3EFirst%3C%2Fp%3E";
        let second = "data:text/html,%3Cp%3ESecond%3C%2Fp%3E";
        engine.navigate(first, 800, 600).unwrap();
        engine.navigate(second, 800, 600).unwrap();
        engine.go_back(800, 600).unwrap();
        let history = engine.navigation().clone();
        assert!(engine.navigate("unsupported:failure", 320, 300).is_err());
        assert!(contains_text(
            &engine.reflow(320, 300).unwrap().display_list,
            "First"
        ));
        assert_eq!(engine.navigation(), &history);
        engine.go_forward(320, 300).unwrap();
        assert!(contains_text(
            &engine.reflow(800, 600).unwrap().display_list,
            "Second"
        ));
        engine.reload(800, 600).unwrap();
        assert_eq!(engine.navigation().entries().len(), 2);
        assert!(contains_text(
            &engine.reflow(320, 300).unwrap().display_list,
            "Second"
        ));
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
    fn author_css_reaches_layout_and_paint() {
        let display_list = Engine::new().render_html(
            "<style>.accent{color:#123456;font-size:30px;font-weight:bold}.block{display:block;color:red}.hidden{display:none}</style><p>before <span class='accent'>styled</span> after</p><span class='block'>block</span><span class='hidden'>hidden</span>",
            800,
            600,
        );

        let styled = display_list
            .commands
            .iter()
            .find_map(|command| match command {
                PaintCommand::Text {
                    text,
                    font_size,
                    bold,
                    color,
                    ..
                } if text == "styled" => Some((*font_size, *bold, *color)),
                _ => None,
            });
        assert_eq!(
            styled,
            Some((
                30,
                true,
                op_paint::Color {
                    r: 0x12,
                    g: 0x34,
                    b: 0x56,
                },
            ))
        );
        assert!(!contains_text(&display_list, "hidden"));

        let before_y = display_list
            .commands
            .iter()
            .find_map(|command| match command {
                PaintCommand::Text { text, y, .. } if text == "before " => Some(*y),
                _ => None,
            });
        let block_y = display_list
            .commands
            .iter()
            .find_map(|command| match command {
                PaintCommand::Text { text, y, color, .. } if text == "block" => {
                    assert_eq!(*color, op_paint::Color { r: 255, g: 0, b: 0 });
                    Some(*y)
                }
                _ => None,
            });
        assert!(before_y.is_some() && block_y.is_some() && block_y > before_y);
    }

    #[test]
    fn css_block_box_model_reaches_display_list_geometry_and_paint() {
        let display_list = Engine::new().render_html(
            "<style>.card{display:block;margin:10px 20px;padding:8px 12px;background-color:#eef2ff;border:2px solid #4338ca}</style><div class='card'>boxed</div>",
            800,
            600,
        );

        assert!(display_list.commands.iter().any(|command| matches!(
            command,
            PaintCommand::FillRect {
                x: 52,
                width: 696,
                color: op_paint::Color {
                    r: 0xee,
                    g: 0xf2,
                    b: 0xff
                },
                ..
            }
        )));
        assert!(
            display_list
                .commands
                .iter()
                .filter(|command| matches!(
                    command,
                    PaintCommand::FillRect {
                        color: op_paint::Color {
                            r: 0x43,
                            g: 0x38,
                            b: 0xca
                        },
                        ..
                    }
                ))
                .count()
                >= 4
        );
        assert!(display_list.commands.iter().any(|command| matches!(
            command,
            PaintCommand::Text { text, x: 66, .. } if text == "boxed"
        )));
    }

    #[test]
    fn expanded_box_model_reaches_native_display_list() {
        let display_list = Engine::new().render_html(
            "<div style='width:50%; margin:10px auto; padding:10%; box-sizing:border-box; background-color:#eef2ff; border-left:4px solid red; border-right:6px solid blue'>wide</div>",
            800,
            600,
        );

        assert!(display_list.commands.iter().any(|command| matches!(
            command,
            PaintCommand::FillRect {
                x: 216,
                width: 368,
                color: op_paint::Color {
                    r: 0xee,
                    g: 0xf2,
                    b: 0xff
                },
                ..
            }
        )));
        assert!(display_list.commands.iter().any(|command| matches!(
            command,
            PaintCommand::FillRect {
                x: 216,
                width: 4,
                color: op_paint::Color { r: 255, g: 0, b: 0 },
                ..
            }
        )));
        assert!(display_list.commands.iter().any(|command| matches!(
            command,
            PaintCommand::FillRect {
                x: 578,
                width: 6,
                color: op_paint::Color { r: 0, g: 0, b: 255 },
                ..
            }
        )));
        assert!(display_list.commands.iter().any(|command| matches!(
            command,
            PaintCommand::Text { text, x: 294, .. } if text == "wide"
        )));
    }

    #[test]
    fn expanded_selectors_reach_native_display_list() {
        let display_list = Engine::new().render_html(
            "<style>
                section[data-state=ready i] > span:first-child + span[data-kind~=accent] { color:#b42318; font-weight:bold }
                .seed ~ .tail[title$=target] { color:#0369a1; font-weight:bold }
                .blank:empty { min-height:16px; padding:4px; background-color:#f5f3ff; border:2px solid #7c3aed }
             </style>
             <section data-state='READY'><span>first</span><span data-kind='accent featured'>second</span></section>
             <p><span class='seed'>seed</span><span>middle</span><span class='tail' title='final-target'>tail</span></p>
             <div class='blank'></div>",
            800,
            600,
        );

        assert!(display_list.commands.iter().any(|command| matches!(
            command,
            PaintCommand::Text {
                text,
                bold: true,
                color: op_paint::Color { r: 0xb4, g: 0x23, b: 0x18 },
                ..
            } if text == "second"
        )));
        assert!(display_list.commands.iter().any(|command| matches!(
            command,
            PaintCommand::Text {
                text,
                bold: true,
                color: op_paint::Color { r: 0x03, g: 0x69, b: 0xa1 },
                ..
            } if text == "tail"
        )));
        assert!(display_list.commands.iter().any(|command| matches!(
            command,
            PaintCommand::FillRect {
                color: op_paint::Color {
                    r: 0xf5,
                    g: 0xf3,
                    b: 0xff
                },
                ..
            }
        )));
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
    fn full_named_reference_fixture_reaches_paint_links_and_history() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/encoding/named-references.html");
        let mut engine = Engine::new();
        let page = engine.navigate(path.to_str().unwrap(), 800, 600).unwrap();
        assert!(contains_text(
            &page.display_list,
            "Latin: Á ä ß. Greek: α Ω. Cyrillic: Я я."
        ));
        assert!(contains_text(
            &page.display_list,
            "Math: ∉ \u{2242}\u{338}. Ligature: fj. Arrow: →."
        ));
        assert!(contains_text(
            &page.display_list,
            "Legacy without semicolon: ¬in / © 2026."
        ));
        assert!(contains_text(
            &page.display_list,
            "Escaped markup: <b>literal text</b>."
        ));
        let (text, link) = page
            .display_list
            .commands
            .iter()
            .find_map(|command| {
                if let PaintCommand::Text { text, links, .. } = command {
                    links.first().map(|link| (text, link))
                } else {
                    None
                }
            })
            .expect("fixture link must reach paint");
        assert_eq!(&text[link.start..link.end], "Перейти → fj");
        assert_eq!(
            link.href,
            "../navigation/destination.html?source=entities&word=Á"
        );
        let next = engine.follow_link(&link.href, 800, 600).unwrap();
        assert!(contains_text(&next.display_list, "Link destination"));
        assert_eq!(engine.navigation().entries().len(), 2);
    }

    #[test]
    fn local_windows1251_fixture_renders_cyrillic_and_decoded_query_link() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/encoding/windows-1251.html");
        let mut engine = Engine::new();
        let page = engine.navigate(path.to_str().unwrap(), 800, 600).unwrap();
        assert!(contains_text(&page.display_list, "Привет, мир!"));
        assert!(
            contains_text(&page.display_list, "Windows-1251: Ё ё №"),
            "{:?}",
            page.display_list
        );
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
    fn local_linked_stylesheet_reaches_paint_in_document_order() {
        let root =
            std::env::temp_dir().join(format!("opbrowser-linked-css-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join("theme.css"),
            ".target { color: red } .external { color: #123456; font-size: 30px; font-weight: bold }",
        )
        .unwrap();
        std::fs::write(
            root.join("index.html"),
            "<link rel='stylesheet' href='missing.css'><link rel='stylesheet' href='theme.css'><style>.target { color: green }</style><p class='target'>Later embedded</p><p class='external'>External style</p>",
        )
        .unwrap();

        let mut engine = Engine::new();
        let page = engine
            .navigate(root.join("index.html").to_str().unwrap(), 800, 600)
            .unwrap();

        let assert_styles = |page: &RenderedPage| {
            assert!(page.display_list.commands.iter().any(|command| matches!(
                command,
                PaintCommand::Text { text, color, .. }
                    if text == "Later embedded" && *color == op_paint::Color { r: 0, g: 128, b: 0 }
            )));
            assert!(page.display_list.commands.iter().any(|command| matches!(
                command,
                PaintCommand::Text { text, color, font_size: 30, bold: true, .. }
                    if text == "External style" && *color == op_paint::Color { r: 0x12, g: 0x34, b: 0x56 }
            )));
        };
        assert_styles(&page);

        std::fs::remove_dir_all(&root).unwrap();
        let reflowed = engine.reflow(420, 480).unwrap();
        assert_styles(&reflowed);
    }

    #[test]
    #[cfg(windows)]
    fn http_linked_stylesheet_reaches_native_display_list() {
        use std::io::{Read, Write};
        use std::net::TcpListener;
        use std::time::{Duration, Instant};

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = format!("http://{}", listener.local_addr().unwrap());
        listener.set_nonblocking(true).unwrap();
        let server = std::thread::spawn(move || {
            let mut requests = Vec::new();
            for index in 0..2 {
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
                requests.push(request.clone());

                let (content_type, body) = if index == 0 {
                    (
                        "text/html; charset=utf-8",
                        "<link rel='stylesheet' href='/theme.css'><p class='remote'>Remote CSS</p>",
                    )
                } else {
                    (
                        "text/css; charset=utf-8",
                        ".remote { color: #7c3aed; font-size: 31px; font-weight: bold }",
                    )
                };
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                )
                .unwrap();
            }
            requests
        });

        let page = Engine::new()
            .render_source(&format!("{address}/index.html"), 800, 600)
            .unwrap();
        assert!(page.display_list.commands.iter().any(|command| matches!(
            command,
            PaintCommand::Text { text, color, font_size: 31, bold: true, .. }
                if text == "Remote CSS" && *color == op_paint::Color { r: 0x7c, g: 0x3a, b: 0xed }
        )));
        let requests = server.join().unwrap();
        assert!(requests[0].starts_with("GET /index.html "));
        assert!(requests[1].starts_with("GET /theme.css "));
        assert!(requests[1].contains("text/css"));
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
