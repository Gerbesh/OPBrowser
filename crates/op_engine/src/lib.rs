use op_css::{
    ComputedStyleMap, StyleCollection, StyleError, StyleMap, collect_author_styles,
    collect_author_styles_with_linked, compute_styles,
};
use op_html::parse_document;
use op_layout::{
    ImageResources, layout_document_with_backgrounds_and_resources,
    layout_document_with_computed_styles_and_viewport_metrics,
};
use op_net::{LoadError, LoadedDocument, NetworkContext, resolve_link};
use op_paint::{DisplayList, build_display_list};
mod images;
mod scripts;
mod styles;
pub use scripts::ScriptReport;
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
    images: images::PageImages,
    style_collection: StyleCollection,
    computed_styles: ComputedStyleMap,
    stylesheet_addresses: std::collections::HashMap<op_dom::NodeId, String>,
    scripts: ScriptReport,
}

impl PreparedDocument {
    fn render(&self, width: i32, height: i32) -> RenderedPage {
        let layout = layout_document_with_backgrounds_and_resources(
            &self.document,
            width,
            height,
            (&self.images.elements, &self.images.backgrounds),
            &self.images.generated,
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
            network: NetworkContext::default(),
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

    pub fn request_filter(&self) -> &op_net::RequestFilter {
        self.network.request_filter()
    }

    pub fn request_filter_mut(&mut self) -> &mut op_net::RequestFilter {
        self.network.request_filter_mut()
    }

    pub fn active_script_report(&self) -> Option<ScriptReport> {
        Some(self.active_document.as_ref()?.scripts)
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

    /// Effective resource base for an active external style/link source node.
    pub fn active_stylesheet_address(&self, style_node: op_dom::NodeId) -> Option<&str> {
        self.active_document
            .as_ref()?
            .stylesheet_addresses
            .get(&style_node)
            .map(String::as_str)
    }

    pub fn render_html(
        &self,
        html: &str,
        viewport_width: i32,
        viewport_height: i32,
    ) -> DisplayList {
        let mut document = parse_document(html);
        let _script_report = scripts::execute_inline(&mut document);
        let style_collection = collect_author_styles(&document);
        let computed_styles = compute_styles(&document, &style_collection.styles);
        let layout = layout_document_with_computed_styles_and_viewport_metrics(
            &document,
            viewport_width,
            viewport_height,
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
        let mut document = parse_document(html);
        let script_report = scripts::execute_inline(&mut document);
        let style_collection = collect_author_styles(&document);
        let computed_styles = compute_styles(&document, &style_collection.styles);
        let prepared = PreparedDocument {
            address: String::new(),
            mime_type: "text/html".into(),
            document,
            images: images::PageImages::default(),
            style_collection,
            computed_styles,
            stylesheet_addresses: std::collections::HashMap::new(),
            scripts: script_report,
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
        let mut document = parse_document(&loaded.text);
        let script_report =
            scripts::execute_for_page(&mut document, Some(&self.network), Some(&loaded.address));
        let linked_stylesheets = styles::load(&self.network, &document, &loaded.address);
        let mut style_collection =
            collect_author_styles_with_linked(&document, &linked_stylesheets.texts);
        let profiles = styles::load_color_profiles(
            &self.network,
            &document,
            &loaded.address,
            &linked_stylesheets,
        );
        if !profiles.is_empty() {
            let mut converted = std::collections::HashMap::new();
            style_collection
                .styles
                .resolve_custom_profile_colors(|name, channels| {
                    let key = (name.to_owned(), channels);
                    if let Some(cached) = converted.get(&key) {
                        return *cached;
                    }
                    let result = profiles
                        .get(name)
                        .and_then(|icc| op_image::convert_icc_rgb(icc, channels).ok());
                    converted.insert(key, result);
                    result
                });
        }
        let computed_styles = compute_styles(&document, &style_collection.styles);
        let images = images::load(
            &self.network,
            &document,
            &loaded.address,
            &computed_styles,
            &linked_stylesheets.addresses,
        );
        Ok(PreparedDocument {
            address: loaded.address,
            mime_type: loaded.mime_type,
            document,
            images,
            style_collection,
            computed_styles,
            stylesheet_addresses: linked_stylesheets.addresses,
            scripts: script_report,
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
    fn inline_page_script_changes_visible_text_and_reflow_keeps_the_mutation() {
        let mut engine = Engine::new();
        let list = engine.set_html_page(
            "<!doctype html><html><body><h1 id='title'>Before JS</h1>\
             <script>document.getElementById('title').textContent = 'JS works';</script>\
             </body></html>",
            800,
            600,
        );
        assert!(contains_text(&list, "JS works"));
        assert!(!contains_text(&list, "Before JS"));
        assert_eq!(engine.active_script_report().unwrap().executed, 1);
        assert_eq!(engine.active_script_report().unwrap().mutations, 1);
        let after_resize = engine.reflow(340, 380).unwrap();
        assert!(contains_text(&after_resize.display_list, "JS works"));
        assert!(!contains_text(&after_resize.display_list, "Before JS"));
    }

    #[test]
    fn inline_page_script_runs_during_real_data_url_navigation() {
        let mut engine = Engine::new();
        let html = "<p id='message'>Initial</p><script>document.getElementById('message').textContent='Updated';</script>";
        let source = format!(
            "data:text/html,{}",
            html.bytes()
                .map(|b| format!("%{b:02X}"))
                .collect::<String>()
        );
        let page = engine.navigate(&source, 800, 600).unwrap();
        assert!(contains_text(&page.display_list, "Updated"));
        assert!(!contains_text(&page.display_list, "Initial"));
        assert_eq!(engine.active_script_report().unwrap().mutations, 1);
    }

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
        assert_eq!(computed.font_size_px, 16.0);
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
    fn functional_colors_reach_text_background_and_border_paint() {
        let display_list = Engine::new().render_html(
            "<div style='color:rgb(255 0 0 / 50%); background-color:hsl(240 100% 50%); border:2px solid rgb(10 20 30); padding:4px'>colors</div>",
            800,
            600,
        );

        assert!(display_list.commands.iter().any(|command| matches!(
            command,
            PaintCommand::Text {
                text,
                color: op_paint::Color { r: 255, g: 127, b: 127 },
                ..
            } if text == "colors"
        )));
        assert!(display_list.commands.iter().any(|command| matches!(
            command,
            PaintCommand::FillRect {
                color: op_paint::Color { r: 0, g: 0, b: 255 },
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
                            r: 10,
                            g: 20,
                            b: 30
                        },
                        ..
                    }
                ))
                .count()
                >= 4
        );
    }

    #[test]
    fn text_alignment_and_line_height_reach_display_list_geometry() {
        let display_list = Engine::new().render_html(
            "<p style='width:300px;text-align:center'>align</p><p style='width:300px;text-align:right'>align</p><p style='line-height:40px'>first<br>second</p>",
            800,
            600,
        );

        let aligned: Vec<i32> = display_list
            .commands
            .iter()
            .filter_map(|command| match command {
                PaintCommand::Text { text, x, .. } if text == "align" => Some(*x),
                _ => None,
            })
            .collect();
        assert_eq!(aligned.len(), 2);
        assert!(aligned[1] > aligned[0]);

        let first_y = display_list
            .commands
            .iter()
            .find_map(|command| match command {
                PaintCommand::Text { text, y, .. } if text == "first" => Some(*y),
                _ => None,
            });
        let second_y = display_list
            .commands
            .iter()
            .find_map(|command| match command {
                PaintCommand::Text { text, y, .. } if text == "second" => Some(*y),
                _ => None,
            });
        assert_eq!(
            second_y.zip(first_y).map(|(second, first)| second - first),
            Some(40)
        );
    }

    #[test]
    fn font_style_decorations_and_white_space_reach_display_list() {
        let display_list = Engine::new().render_html(
            "<p><span style='font-style:italic; text-decoration:underline line-through'>styled</span></p><div style='white-space:pre'>a  b\n c</div>",
            800,
            600,
        );

        assert!(display_list.commands.iter().any(|command| matches!(
            command,
            PaintCommand::Text {
                text,
                italic: true,
                underline: true,
                line_through: true,
                ..
            } if text == "styled"
        )));
        let preserved: Vec<&str> = display_list
            .commands
            .iter()
            .filter_map(|command| match command {
                PaintCommand::Text { text, .. } if text == "a  b" || text == " c" => {
                    Some(text.as_str())
                }
                _ => None,
            })
            .collect();
        assert_eq!(preserved, ["a  b", " c"]);
    }

    #[test]
    fn table_grid_geometry_and_header_weight_reach_display_list() {
        let display_list = Engine::new().render_html(
            "<style>
               table { width:300px; border:2px solid #222 }
               td, th { border:1px solid #000; padding:4px }
             </style>
             <table>
               <caption>Cap</caption>
               <tr><th>A</th><th>B</th></tr>
               <tr><td>one</td><td>two</td></tr>
             </table>",
            800,
            600,
        );

        let find_text = |needle: &str| {
            display_list
                .commands
                .iter()
                .find_map(|command| match command {
                    PaintCommand::Text {
                        x, y, text, bold, ..
                    } if text.contains(needle) => Some((*x, *y, *bold)),
                    _ => None,
                })
                .expect("table text must reach paint")
        };

        let cap = find_text("Cap");
        let a = find_text("A");
        let b = find_text("B");
        let one = find_text("one");
        let two = find_text("two");

        assert!(cap.1 < a.1);
        assert_eq!(a.1, b.1);
        assert_eq!(one.1, two.1);
        assert!(b.0 > a.0 + 100);
        assert!(a.2 && b.2);
        assert!(!one.2 && !two.2);
    }

    #[test]
    fn raw_select_text_does_not_render_as_page_content() {
        let html = "<style>.none{display:none} .red{color:red}</style>          <div>visible before</div><select class='red' size='4'>stray raw text</select>          <select class='red' size='4'><option class='none'>hidden option</option></select>          <div>visible after</div>";
        let page = Engine::new().render_html(html, 800, 600);
        let visible: Vec<&str> = page
            .commands
            .iter()
            .filter_map(|command| match command {
                PaintCommand::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert!(
            visible.iter().any(|text| text.contains("visible before")),
            "{visible:?}"
        );
        assert!(
            visible.iter().any(|text| text.contains("visible after")),
            "{visible:?}"
        );
        assert!(
            !visible.iter().any(|text| text.contains("stray raw text")),
            "{visible:?}"
        );
        assert!(
            !visible.iter().any(|text| text.contains("hidden option")),
            "{visible:?}"
        );
    }

    #[test]
    fn svg_defs_use_and_contents_do_not_leak_hidden_text() {
        let html = "<svg><defs><text id='letter'>S</text></defs>            <text style='display:contents'>FAIL</text>            <svg style='display:contents'><text>P</text></svg>            <g style='display:contents'><text>A</text></g>            <use xlink:href='#letter' style='display:contents'></use>            <text>S</text></svg>            <svg style='display:contents'><text>FAIL</text></svg>";
        let page = Engine::new().render_html(html, 800, 600);
        let text = page
            .commands
            .iter()
            .filter_map(|command| match command {
                PaintCommand::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect::<String>();
        assert_eq!(text.replace(' ', ""), "PASS");
    }

    #[test]
    fn first_line_pseudo_paints_only_first_line_after_explicit_break() {
        let html = "<style>p { color:red;line-height:20px }            p::first-line { color:green;background:#ffc0cb }</style>            <p>alpha<br>beta</p>";
        let commands = Engine::new().render_html(html, 800, 600).commands;
        let text_color = |needle: &str| -> (u8, u8, u8) {
            commands
                .iter()
                .find_map(|command| match command {
                    PaintCommand::Text { text, color, .. } if text.contains(needle) => {
                        Some((color.r, color.g, color.b))
                    }
                    _ => None,
                })
                .expect("expected painted line")
        };
        assert_eq!(text_color("alpha"), (0, 128, 0));
        assert_eq!(text_color("beta"), (255, 0, 0));
        assert!(commands.iter().any(|command| matches!(command,
            PaintCommand::FillRect {color,..}
                if (color.r,color.g,color.b)==(255,192,203)
        )));
    }

    #[test]
    fn first_line_recolors_only_currentcolor_inline_ink() {
        let html = "<style>
            p {color:red}
            p::first-line {color:green}
            #relative span {background:currentcolor;border:2px solid currentcolor}
            #fixed span {background:red;border:2px solid red}
            #blue span {color:blue;background:currentcolor;border:2px solid currentcolor}
            </style>
            <p id='relative'><span>first</span><br><span>second</span></p>
            <p id='fixed'><span>fixed</span></p>
            <p id='blue'><span>blue</span></p>";
        let commands = Engine::new().render_html(html, 800, 600).commands;
        let line_y = |needle: &str| {
            commands
                .iter()
                .find_map(|cmd| match cmd {
                    PaintCommand::Text { text, y, .. } if text == needle => Some(*y),
                    _ => None,
                })
                .expect("text must be rendered")
        };
        let color_near = |y: i32| {
            commands
                .iter()
                .filter_map(|cmd| match cmd {
                    PaintCommand::FillRect {
                        y: top,
                        height,
                        color,
                        ..
                    } if (*top - y).abs() <= 18 && *height >= 2 => {
                        Some((color.r, color.g, color.b))
                    }
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        let first = color_near(line_y("first"));
        let second = color_near(line_y("second"));
        let fixed = color_near(line_y("fixed"));
        let blue = color_near(line_y("blue"));
        assert!(first.contains(&(0, 128, 0)), "first: {first:?}");
        assert!(second.contains(&(255, 0, 0)), "second: {second:?}");
        assert!(fixed.contains(&(255, 0, 0)), "fixed: {fixed:?}");
        assert!(blue.contains(&(0, 0, 255)), "blue: {blue:?}");
        assert!(
            !fixed.contains(&(0, 128, 0)),
            "explicit red must stay red: {fixed:?}"
        );
    }

    #[test]
    fn first_line_background_uses_same_font_metrics_as_inline_background() {
        let html = "<style>p {font-size:10px}            #styled::first-line {background:#ffc0cb}            #control span {background:#ffc0cb}</style>            <p id='styled'>sample<br>plain</p>            <p id='control'><span>sample</span><br>plain</p>";
        let commands = Engine::new().render_html(html, 800, 600).commands;
        let heights: Vec<i32> = commands
            .iter()
            .filter_map(|command| match command {
                PaintCommand::FillRect { height, color, .. }
                    if (color.r, color.g, color.b) == (255, 192, 203) =>
                {
                    Some(*height)
                }
                _ => None,
            })
            .collect();
        assert_eq!(heights.len(), 2, "{heights:?}");
        assert_eq!(heights[0], heights[1]);
    }

    #[test]
    fn intrinsic_table_tracks_and_spacing_reach_display_list() {
        let display_list = Engine::new().render_html(
            "<style>
               table { width:400px; border-spacing:10px 4px }
               td { padding:0 }
               #short { background:#ff0000 }
               #long { background:#0000ff }
             </style>
             <table><tr>
               <td id='short'>x</td>
               <td id='long'>a considerably longer table cell value</td>
             </tr></table>",
            800,
            600,
        );

        let rect = |expected: op_paint::Color| {
            display_list
                .commands
                .iter()
                .find_map(|command| match command {
                    PaintCommand::FillRect {
                        x,
                        y,
                        width,
                        height,
                        color,
                    } if *color == expected => Some((*x, *y, *width, *height)),
                    _ => None,
                })
                .expect("colored table cell must reach paint")
        };

        let short = rect(op_paint::Color { r: 255, g: 0, b: 0 });
        let long = rect(op_paint::Color { r: 0, g: 0, b: 255 });

        assert_eq!(short.1, long.1);
        assert!(long.2 > short.2 * 2);
        assert_eq!(long.0 - (short.0 + short.2), 10);
    }

    #[test]
    fn rowspan_does_not_inflate_height_of_first_row_or_table() {
        let html = r#"<table style="width:100px;border-spacing:0;background:lime">
          <tr><td rowspan="2" style="padding:0;background:red"><div style="height:120px;width:20px"></div></td>
              <td style="padding:0;background:blue"><div style="height:20px;width:20px"></div></td></tr>
          <tr><td style="padding:0;background:yellow"><div style="height:20px;width:20px"></div></td></tr>
        </table>"#;
        let commands = Engine::new().render_html(html, 800, 600).commands;
        let find = |red: u8, green: u8, blue: u8| -> (i32, i32, i32, i32) {
            commands
                .iter()
                .find_map(|command| match command {
                    PaintCommand::FillRect {
                        x,
                        y,
                        width,
                        height,
                        color,
                    } if (color.r, color.g, color.b) == (red, green, blue) => {
                        Some((*x, *y, *width, *height))
                    }
                    _ => None,
                })
                .expect("expected colored table region")
        };
        let table = find(0, 255, 0);
        let span = find(255, 0, 0);
        let first = find(0, 0, 255);
        let second = find(255, 255, 0);
        assert_eq!(table.3, 120);
        assert_eq!(span.3, 120);
        assert_eq!(first.3 + second.3, 120);
        assert_eq!(first.1 + first.3, second.1);
        assert_eq!(span.1, first.1);
        assert_eq!(second.1 + second.3, span.1 + span.3);
    }

    #[test]
    fn overlapping_rowspans_keep_following_cells_at_shared_track_boundaries() {
        let html = r#"<table style="width:120px;border-spacing:0;background:lime">
            <tr><td rowspan="3" style="padding:0;background:red"><div style="height:150px;width:20px"></div></td>
                <td rowspan="2" style="padding:0;background:blue"><div style="height:70px;width:20px"></div></td>
                <td style="padding:0"><div style="height:20px;width:20px"></div></td></tr>
            <tr><td style="padding:0"><div style="height:20px;width:20px"></div></td></tr>
            <tr><td style="padding:0;background:yellow"><div style="height:20px;width:20px"></div></td>
                <td style="padding:0;background:aqua"><div style="height:20px;width:20px"></div></td></tr>
            <tr><td colspan="3" style="padding:0;background:fuchsia"><div style="height:20px;width:20px"></div></td></tr>
          </table>"#;
        let commands = Engine::new().render_html(html, 800, 600).commands;
        let find = |rgb: (u8, u8, u8)| -> (i32, i32) {
            commands
                .iter()
                .find_map(|command| match command {
                    PaintCommand::FillRect {
                        y, height, color, ..
                    } if (color.r, color.g, color.b) == rgb => Some((*y, *height)),
                    _ => None,
                })
                .expect("colored cell")
        };
        let table = find((0, 255, 0));
        let three = find((255, 0, 0));
        let two = find((0, 0, 255));
        let final_span_row = find((255, 255, 0));
        let next_row = find((255, 0, 255));
        assert_eq!(three.1, 150);
        assert_eq!(two.1, 70);
        assert_eq!(final_span_row.0 + final_span_row.1, three.0 + three.1);
        assert_eq!(next_row.0, three.0 + three.1);
        assert_eq!(table.1, 170);
    }

    #[test]
    fn rowspan_respects_vertical_border_spacing_and_following_row_offset() {
        let html = r#"<table style="width:100px;border-spacing:0 5px;background:lime">
            <tr><td rowspan="2" style="padding:0;background:red"><div style="width:20px;height:120px"></div></td>
                <td style="padding:0;background:blue"><div style="width:20px;height:20px"></div></td></tr>
            <tr><td style="padding:0;background:yellow"><div style="width:20px;height:20px"></div></td></tr>
            <tr><td colspan="2" style="padding:0;background:fuchsia"><div style="width:20px;height:20px"></div></td></tr>
          </table>"#;
        let commands = Engine::new().render_html(html, 800, 600).commands;
        let find = |rgb: (u8, u8, u8)| -> (i32, i32) {
            commands
                .iter()
                .find_map(|command| match command {
                    PaintCommand::FillRect {
                        y, height, color, ..
                    } if (color.r, color.g, color.b) == rgb => Some((*y, *height)),
                    _ => None,
                })
                .expect("colored cell")
        };
        let table = find((0, 255, 0));
        let span = find((255, 0, 0));
        let first = find((0, 0, 255));
        let second = find((255, 255, 0));
        let after = find((255, 0, 255));
        assert_eq!(span.1, 120);
        assert_eq!(first.0 + first.1 + 5, second.0);
        assert_eq!(span.0 + span.1 + 5, after.0);
        assert_eq!(table.1, 155);
    }

    #[test]
    fn auto_colspan_intrinsic_width_distributes_to_spanned_columns() {
        let html = r#"<div style="width:300px"><table style="border-spacing:0;background:lime">
          <tr><td colspan="2" style="padding:0;background:red"><div style="height:20px;width:120px"></div></td></tr>
          <tr><td style="padding:0;background:blue"><div style="height:20px;width:20px"></div></td>
              <td style="padding:0;background:yellow"><div style="height:20px;width:20px"></div></td></tr>
        </table></div>"#;
        let commands = Engine::new().render_html(html, 800, 600).commands;
        let width = |rgb: (u8, u8, u8)| -> i32 {
            commands
                .iter()
                .find_map(|command| match command {
                    PaintCommand::FillRect { width, color, .. }
                        if (color.r, color.g, color.b) == rgb =>
                    {
                        Some(*width)
                    }
                    _ => None,
                })
                .expect("expected colored table region")
        };
        assert_eq!(width((0, 255, 0)), 120);
        assert_eq!(width((255, 0, 0)), 120);
        assert_eq!(width((0, 0, 255)) + width((255, 255, 0)), 120);
    }

    #[test]
    fn fixed_table_percent_tracks_and_bottom_caption_reach_paint() {
        let display_list = Engine::new().render_html(
            "<style>
               table { width:400px; table-layout:fixed; border-spacing:0; background:#eeeeee }
               caption { caption-side:bottom; background:#00ff00 }
               #first { width:25% }
               #left { background:#ff0000 }
               #right { background:#0000ff }
             </style>
             <table>
               <caption>bottom</caption>
               <colgroup><col id='first'><col></colgroup>
               <tr><td id='left'>a</td><td id='right'>b</td></tr>
               <tr><td>very very very very long late content</td><td>z</td></tr>
             </table>",
            800,
            600,
        );

        let rect = |expected: op_paint::Color| {
            display_list
                .commands
                .iter()
                .find_map(|command| match command {
                    PaintCommand::FillRect {
                        x,
                        y,
                        width,
                        height,
                        color,
                    } if *color == expected => Some((*x, *y, *width, *height)),
                    _ => None,
                })
                .expect("advanced table rectangle must reach paint")
        };
        let table = rect(op_paint::Color {
            r: 238,
            g: 238,
            b: 238,
        });
        let caption = rect(op_paint::Color { r: 0, g: 255, b: 0 });
        let left = rect(op_paint::Color { r: 255, g: 0, b: 0 });
        let right = rect(op_paint::Color { r: 0, g: 0, b: 255 });

        assert!((left.2 - 100).abs() <= 2);
        assert!((right.2 - 300).abs() <= 2);
        assert_eq!(right.0, left.0 + left.2);
        assert!(caption.1 >= table.1 + table.3);
    }

    #[test]
    fn auto_table_wrapper_shrinks_to_explicit_cell_descendant_width() {
        let html = r#"<div style="width:300px">
          <table style="border-spacing:0;background:lime">
            <tr><td style="padding:0"><div style="width:60px;height:20px;background:blue"></div></td></tr>
          </table>
        </div>"#;
        let page = Engine::new().render_html(html, 800, 600);
        let green = page
            .commands
            .iter()
            .find_map(|item| match item {
                PaintCommand::FillRect {
                    x,
                    y,
                    width,
                    height,
                    color,
                } if (color.r, color.g, color.b) == (0, 255, 0) => Some((*x, *y, *width, *height)),
                _ => None,
            })
            .expect("table background");
        let blue = page
            .commands
            .iter()
            .find_map(|item| match item {
                PaintCommand::FillRect {
                    x,
                    y,
                    width,
                    height,
                    color,
                } if (color.r, color.g, color.b) == (0, 0, 255) => Some((*x, *y, *width, *height)),
                _ => None,
            })
            .expect("cell's sized block");
        assert_eq!(green.2, 60);
        assert_eq!(blue.2, 60);
        assert_eq!((green.0, green.1), (blue.0, blue.1));
    }

    #[test]
    fn auto_table_two_column_width_matches_intrinsic_tracks() {
        let html = r#"<div style="width:300px">
            <table style="border-spacing:0;background:lime">
              <tr><td style="padding:0"><div style="width:40px;height:20px;background:red"></div></td>
                  <td style="padding:0"><div style="width:50px;height:20px;background:blue"></div></td></tr>
            </table>
          </div>"#;
        let page = Engine::new().render_html(html, 800, 600);
        let rect = |color: (u8, u8, u8)| {
            page.commands
                .iter()
                .find_map(|item| match item {
                    PaintCommand::FillRect {
                        x,
                        y,
                        width,
                        height,
                        color: c,
                    } if (c.r, c.g, c.b) == color => Some((*x, *y, *width, *height)),
                    _ => None,
                })
                .expect("paint rectangle")
        };
        let table = rect((0, 255, 0));
        let left = rect((255, 0, 0));
        let right = rect((0, 0, 255));
        assert_eq!(table.2, 90);
        assert_eq!(right.0, left.0 + left.2);
        assert_eq!(table.0 + table.2, right.0 + right.2);
    }

    #[test]
    fn auto_table_width_includes_spacing_padding_and_border() {
        let html = r#"<div style="width:300px">
          <table style="border-spacing:5px 0;padding:0 3px;border:2px solid black;background:lime">
            <tr><td style="padding:0"><div style="width:40px;height:20px;background:red"></div></td>
                <td style="padding:0"><div style="width:50px;height:20px;background:blue"></div></td></tr>
          </table>
        </div>"#;
        let page = Engine::new().render_html(html, 800, 600);
        let rect = |color: (u8, u8, u8)| {
            page.commands
                .iter()
                .find_map(|item| match item {
                    PaintCommand::FillRect {
                        x, width, color: c, ..
                    } if (c.r, c.g, c.b) == color => Some((*x, *width)),
                    _ => None,
                })
                .expect("paint rectangle")
        };
        let table = rect((0, 255, 0));
        let left = rect((255, 0, 0));
        let right = rect((0, 0, 255));
        assert_eq!(table.1, 115); // 40+50 + three 5px spacings + 6px padding + 4px border.
        assert_eq!(left.1, 40);
        assert_eq!(right.1, 50);
        assert_eq!(right.0, left.0 + left.1 + 5);
        assert_eq!(left.0, table.0 + 2 + 3 + 5);
    }

    #[test]
    fn table_explicit_width_remains_authoritative() {
        let html = r#"<div style="width:300px">
          <table style="width:200px;border-spacing:0;background:lime">
            <tr><td style="padding:0"><div style="width:40px;height:20px;background:blue"></div></td></tr>
          </table>
        </div>"#;
        let page = Engine::new().render_html(html, 800, 600);
        let width = page
            .commands
            .iter()
            .find_map(|item| match item {
                PaintCommand::FillRect { width, color, .. }
                    if (color.r, color.g, color.b) == (0, 255, 0) =>
                {
                    Some(*width)
                }
                _ => None,
            })
            .expect("table");
        assert_eq!(width, 200);
    }

    #[test]
    fn fixed_table_layout_with_auto_width_uses_intrinsic_sizing() {
        let html = r#"<div style="width:300px">
          <table style="table-layout:fixed;border-spacing:0;background:lime">
            <tr><td style="padding:0"><div style="width:60px;height:20px"></div></td></tr>
          </table>
        </div>"#;
        let page = Engine::new().render_html(html, 800, 600);
        let width = page
            .commands
            .iter()
            .find_map(|item| match item {
                PaintCommand::FillRect { width, color, .. }
                    if (color.r, color.g, color.b) == (0, 255, 0) =>
                {
                    Some(*width)
                }
                _ => None,
            })
            .expect("table background");
        assert_eq!(width, 60);
    }

    #[test]
    fn auto_table_preserves_min_width_constraint() {
        let html = r#"<div style="width:300px">
          <table style="min-width:100px;border-spacing:0;background:lime">
            <tr><td style="padding:0"><div style="width:60px;height:20px"></div></td></tr>
          </table>
        </div>"#;
        let page = Engine::new().render_html(html, 800, 600);
        let width = page
            .commands
            .iter()
            .find_map(|item| match item {
                PaintCommand::FillRect { width, color, .. }
                    if (color.r, color.g, color.b) == (0, 255, 0) =>
                {
                    Some(*width)
                }
                _ => None,
            })
            .expect("table background");
        assert_eq!(width, 100);
    }

    #[test]
    fn inline_table_reaches_paint_as_an_atomic_inline_context() {
        let display_list = Engine::new().render_html(
            "<style>
               #host { width:420px; font-size:16px; line-height:20px }
               #it { display:inline-table; width:100px; border-spacing:0; background:#ff0000 }
               .row { display:table-row }
               .cell { display:table-cell; padding:0 }
             </style>
             <div id='host'>before <span id='it'><span class='row'><span class='cell'>A</span><span class='cell'>B</span></span></span> after</div>",
            800,
            600,
        );

        let text_position = |needle: &str| {
            display_list
                .commands
                .iter()
                .find_map(|command| match command {
                    PaintCommand::Text { text, x, y, .. } if text.contains(needle) => {
                        Some((*x, *y))
                    }
                    _ => None,
                })
                .expect("inline-table text must reach paint")
        };
        let before = text_position("before");
        let a = text_position("A");
        let after = text_position("after");
        let table = display_list
            .commands
            .iter()
            .find_map(|command| match command {
                PaintCommand::FillRect {
                    x,
                    y,
                    width,
                    height,
                    color,
                } if *color == op_paint::Color { r: 255, g: 0, b: 0 } => {
                    Some((*x, *y, *width, *height))
                }
                _ => None,
            })
            .expect("inline-table background must reach paint");

        assert_eq!(before.1, a.1);
        assert_eq!(before.1, after.1);
        assert!(table.0 >= before.0);
        assert!(after.0 >= table.0 + table.2);
        assert!(table.2 >= 100);
        assert!(table.3 > 0);
    }

    #[test]
    fn orphan_table_cells_are_grouped_before_paint() {
        let display_list = Engine::new().render_html(
            "<style>
               #host { width:360px }
               .cell { display:table-cell; padding:0 }
               #a { background:#ff0000 }
               #b { background:#0000ff }
               #after { display:block; margin:0; background:#00ff00 }
             </style>
             <div id='host'>
               <div id='a' class='cell'>A</div>
               
               <div id='b' class='cell'>B</div>
               <div id='after'>after</div>
             </div>",
            800,
            600,
        );

        let rect = |expected: op_paint::Color| {
            display_list
                .commands
                .iter()
                .find_map(|command| match command {
                    PaintCommand::FillRect {
                        x,
                        y,
                        width,
                        height,
                        color,
                    } if *color == expected => Some((*x, *y, *width, *height)),
                    _ => None,
                })
                .expect("repaired orphan table content must reach paint")
        };

        let a = rect(op_paint::Color { r: 255, g: 0, b: 0 });
        let b = rect(op_paint::Color { r: 0, g: 0, b: 255 });
        let after = rect(op_paint::Color { r: 0, g: 255, b: 0 });

        assert_eq!(a.1, b.1);
        assert_eq!(b.0, a.0 + a.2);
        assert!(after.1 >= a.1 + a.3);
    }

    #[test]
    fn anonymous_table_rows_and_cells_reach_display_list() {
        let display_list = Engine::new().render_html(
            "<style>
               #table { display:table; width:360px; border-spacing:0; color:#ff0000 }
               .cell { display:table-cell; padding:0 }
               #a { background:#0000ff }
               #b { background:#00ff00 }
             </style>
             <div id='table'>
               loose
               <div id='a' class='cell'>A</div>
               <div id='b' class='cell'>B</div>
             </div>",
            800,
            600,
        );

        assert!(display_list.commands.iter().any(|command| matches!(
            command,
            PaintCommand::Text { text, color, .. }
                if text.contains("loose")
                    && *color == op_paint::Color { r: 255, g: 0, b: 0 }
        )));

        let rect = |expected: op_paint::Color| {
            display_list
                .commands
                .iter()
                .find_map(|command| match command {
                    PaintCommand::FillRect {
                        x,
                        y,
                        width,
                        height,
                        color,
                    } if *color == expected => Some((*x, *y, *width, *height)),
                    _ => None,
                })
                .expect("anonymous-table child cell must reach paint")
        };
        let a = rect(op_paint::Color { r: 0, g: 0, b: 255 });
        let b = rect(op_paint::Color { r: 0, g: 255, b: 0 });

        assert_eq!(a.1, b.1);
        assert!(b.0 >= a.0 + a.2);
    }

    #[test]
    fn table_cell_vertical_align_reaches_display_list_geometry() {
        let display_list = Engine::new().render_html(
            "<style>
               table { width:360px; border-spacing:0 }
               td { height:84px; padding:0 }
               #top { vertical-align:top }
               #middle { vertical-align:middle }
               #bottom { vertical-align:bottom }
             </style>
             <table><tr>
               <td id='top'>top</td>
               <td id='middle'>middle</td>
               <td id='bottom'>bottom</td>
             </tr></table>",
            800,
            600,
        );

        let y = |needle: &str| {
            display_list
                .commands
                .iter()
                .find_map(|command| match command {
                    PaintCommand::Text { y, text, .. } if text == needle => Some(*y),
                    _ => None,
                })
                .expect("table cell text must reach paint")
        };

        let top = y("top");
        let middle = y("middle");
        let bottom = y("bottom");
        assert!(top < middle);
        assert!(middle < bottom);
        assert!((middle - top - (bottom - middle)).abs() <= 1);
    }

    #[test]
    fn table_foster_parenting_and_cell_text_reach_display_list() {
        let display_list =
            Engine::new().render_html("<table>outside<tr><td>cell</td></tr></table>tail", 800, 600);

        let painted_text: String = display_list
            .commands
            .iter()
            .filter_map(|command| match command {
                PaintCommand::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect();

        let outside = painted_text
            .find("outside")
            .expect("foster-parented table text must paint");
        let cell = painted_text
            .find("cell")
            .expect("table cell text must paint");
        let tail = painted_text
            .find("tail")
            .expect("text after the table must paint");

        assert!(outside < cell);
        assert!(cell < tail);
    }

    #[test]
    fn misnested_html_formatting_recovery_reaches_display_list() {
        let display_list = Engine::new().render_html("<p>1<b>2<i>3</b>4</i>5</p>", 800, 600);

        assert!(display_list.commands.iter().any(|command| matches!(
            command,
            PaintCommand::Text {
                text,
                bold: true,
                italic: false,
                ..
            } if text == "2"
        )));
        assert!(display_list.commands.iter().any(|command| matches!(
            command,
            PaintCommand::Text {
                text,
                bold: true,
                italic: true,
                ..
            } if text == "3"
        )));
        assert!(display_list.commands.iter().any(|command| matches!(
            command,
            PaintCommand::Text {
                text,
                bold: false,
                italic: true,
                ..
            } if text == "4"
        )));
        assert!(display_list.commands.iter().any(|command| matches!(
            command,
            PaintCommand::Text {
                text,
                bold: false,
                italic: false,
                ..
            } if text == "5"
        )));
    }

    #[test]
    fn text_transform_and_spacing_reach_display_list() {
        let display_list = Engine::new().render_html(
            "<p style='text-transform:uppercase;letter-spacing:3px;word-spacing:6px'><a href='next'>straße test</a></p>",
            800,
            600,
        );

        assert!(display_list.commands.iter().any(|command| matches!(
            command,
            PaintCommand::Text {
                text,
                letter_spacing: 3,
                word_spacing: 6,
                links,
                ..
            } if text == "STRASSE TEST"
                && links.len() == 1
                && &text[links[0].start..links[0].end] == "STRASSE TEST"
        )));
    }

    #[test]
    fn inline_box_fragments_reach_fill_rect_paint_commands() {
        let display_list = Engine::new().render_html(
            "<p>before <span style='padding:3px 6px;background-color:#eef2ff;border:2px solid #4338ca'>boxed <b>inline</b></span> after</p>",
            800,
            600,
        );

        let background = display_list
            .commands
            .iter()
            .find_map(|command| match command {
                PaintCommand::FillRect {
                    x,
                    y,
                    width,
                    height,
                    color,
                } if *color
                    == op_paint::Color {
                        r: 238,
                        g: 242,
                        b: 255,
                    } =>
                {
                    Some((*x, *y, *width, *height))
                }
                _ => None,
            });
        let background = background.expect("inline background must reach the display list");
        assert!(background.2 > 20 && background.3 >= 30, "{background:?}");
        assert!(
            display_list
                .commands
                .iter()
                .filter(|command| matches!(
                    command,
                    PaintCommand::FillRect { color, .. }
                        if *color == op_paint::Color { r: 67, g: 56, b: 202 }
                ))
                .count()
                >= 4
        );
        let boxed_x = display_list
            .commands
            .iter()
            .find_map(|command| match command {
                PaintCommand::Text { x, text, .. } if text.contains("boxed") => Some(*x),
                _ => None,
            })
            .unwrap();
        assert_eq!(boxed_x - background.0, 8);
        assert!(contains_text(&display_list, "inline"));
    }

    #[test]
    fn functional_pseudos_and_background_shorthand_reach_native_display_list() {
        let display_list = Engine::new().render_html(
            "<style>
               .card:is(.hot,.warm):not(.skip):nth-child(odd) { color:#b42318; background:#eef2ff; padding:2px 4px; }
               span:where(#three) { font-weight:bold; }
             </style>
             <main><span class='card hot'>one</span><span class='card hot skip'>two</span><span id='three' class='card warm'>three</span></main>",
            800,
            600,
        );

        let styled_texts: Vec<_> = display_list
            .commands
            .iter()
            .filter_map(|command| match command {
                PaintCommand::Text {
                    text, color, bold, ..
                } if text == "one" || text == "two" || text == "three" => {
                    Some((text.as_str(), *color, *bold))
                }
                _ => None,
            })
            .collect();
        assert!(styled_texts.iter().any(|(text, color, _)| {
            *text == "one"
                && *color
                    == op_paint::Color {
                        r: 180,
                        g: 35,
                        b: 24,
                    }
        }));
        assert!(
            styled_texts
                .iter()
                .any(|(text, color, _)| { *text == "two" && *color == op_paint::Color::BLACK })
        );
        assert!(styled_texts.iter().any(|(text, color, bold)| {
            *text == "three"
                && *color
                    == op_paint::Color {
                        r: 180,
                        g: 35,
                        b: 24,
                    }
                && *bold
        }));
        assert_eq!(
            display_list
                .commands
                .iter()
                .filter(|command| matches!(
                    command,
                    PaintCommand::FillRect { color, .. }
                        if *color == op_paint::Color { r: 238, g: 242, b: 255 }
                ))
                .count(),
            2
        );
    }

    #[test]
    fn generated_before_after_content_reaches_native_display_list() {
        let display_list = Engine::new().render_html(
            "<style>
               #note { color:#123456; font-size:20px }
               #note::before { content:'[GEN] '; color:#b42318; background:#eef2ff; padding:2px 4px; border:1px solid #4338ca }
               #note::after { content:' ✓'; color:#087a35; font-weight:bold }
             </style><p id='note'>Body</p>",
            800,
            600,
        );

        assert!(display_list.commands.iter().any(|command| matches!(
            command,
            PaintCommand::Text { text, .. } if text.contains("[GEN]")
        )));
        assert!(contains_text(&display_list, "Body"));
        assert!(display_list.commands.iter().any(|command| matches!(
            command,
            PaintCommand::Text { text, .. } if text.contains('✓')
        )));
        assert!(display_list.commands.iter().any(|command| matches!(
            command,
            PaintCommand::Text { text, color, .. }
                if text.contains("[GEN]")
                    && *color == op_paint::Color { r: 180, g: 35, b: 24 }
        )));
        assert!(display_list.commands.iter().any(|command| matches!(
            command,
            PaintCommand::Text { text, bold: true, color, .. }
                if text.contains('✓')
                    && *color == op_paint::Color { r: 8, g: 122, b: 53 }
        )));
        assert!(display_list.commands.iter().any(|command| matches!(
            command,
            PaintCommand::FillRect { color, .. }
                if *color == op_paint::Color { r: 238, g: 242, b: 255 }
        )));
    }

    #[test]
    fn generated_attr_and_counters_reach_native_display_list() {
        let display_list = Engine::new().render_html(
            "<style>
               #outline { counter-reset:chapter }
               .item { counter-increment:chapter }
               .item::before { content:attr(data-label) ' ' counter(chapter, upper-roman) ': '; color:#b42318 }
               #outline::after { content:' total=' counter(chapter); color:#087a35; font-weight:bold }
             </style>
             <div id='outline'>
               <p class='item' data-label='Chapter'>One</p>
               <p class='item' data-label='Chapter'>Two</p>
             </div>",
            800,
            600,
        );

        assert!(display_list.commands.iter().any(|command| matches!(
            command,
            PaintCommand::Text { text, color, .. }
                if text.contains("Chapter I:")
                    && *color == op_paint::Color { r: 180, g: 35, b: 24 }
        )));
        assert!(display_list.commands.iter().any(|command| matches!(
            command,
            PaintCommand::Text { text, color, .. }
                if text.contains("Chapter II:")
                    && *color == op_paint::Color { r: 180, g: 35, b: 24 }
        )));
        assert!(display_list.commands.iter().any(|command| matches!(
            command,
            PaintCommand::Text { text, bold: true, color, .. }
                if text.contains("total=2")
                    && *color == op_paint::Color { r: 8, g: 122, b: 53 }
        )));
    }

    #[test]
    fn nested_quotes_reach_paint_links_and_retained_reflow() {
        let mut engine = Engine::new();
        let html = "<style>p { quotes:'«' '»' '‹' '›' }
            q::before { color:red } q::after { color:green }
            </style><p><a href='next.html'><q>outer <q>inner</q> tail</q></a></p>";
        let original = engine.set_html_page(html, 800, 600);
        let text = original
            .commands
            .iter()
            .filter_map(|command| match command {
                PaintCommand::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect::<String>();
        assert_eq!(text, "«outer ‹inner› tail»");
        assert!(original.commands.iter().any(|command| matches!(command,
            PaintCommand::Text { text, links, color, .. }
            if text == "«" && links.iter().any(|link| link.href == "next.html")
                && *color == op_paint::Color { r:255, g:0, b:0 }
        )));
        engine.reflow(180, 600).unwrap();
        assert_eq!(engine.reflow(800, 600).unwrap().display_list, original);
    }

    #[test]
    fn generated_block_geometry_and_empty_boxes_reach_retained_paint() {
        let mut engine = Engine::new();
        let original = engine.set_html_page(
            "<style>a::before { display:block; content:'PRE'; width:50%; margin:7px auto;
                padding:4px; border:2px solid blue; background:red }
             a::after { display:block; content:''; width:80px; height:20px; background:blue }
             </style><a href='next.html'>Body</a>",
            800,
            600,
        );
        let background = original
            .commands
            .iter()
            .find_map(|command| match command {
                PaintCommand::FillRect { x, y, color, .. }
                    if *color == op_paint::Color { r: 255, g: 0, b: 0 } =>
                {
                    Some((*x, *y))
                }
                _ => None,
            })
            .unwrap();
        let pre = original
            .commands
            .iter()
            .find_map(|command| match command {
                PaintCommand::Text {
                    x, y, text, links, ..
                } if text == "PRE" => {
                    assert!(links.iter().any(|link| link.href == "next.html"));
                    Some((*x, *y))
                }
                _ => None,
            })
            .unwrap();
        assert_eq!(pre, (background.0 + 6, background.1 + 6));
        assert!(original.commands.iter().any(|command| matches!(command,
            PaintCommand::FillRect { width:80, height:20, color, .. }
                if *color == op_paint::Color { r:0, g:0, b:255 }
        )));
        let narrow = engine.reflow(400, 600).unwrap();
        assert_ne!(narrow.display_list, original);
        assert_eq!(engine.reflow(800, 600).unwrap().display_list, original);
    }

    #[test]
    fn cyclic_custom_properties_use_consumer_fallbacks_in_paint_and_reflow() {
        let mut engine = Engine::new();
        let original = engine.set_html_page(
            "<style>#item { --self:var(--self,red); --good:green;
                --a:var(--good,var(--b)); --b:var(--a); color:var(--self,blue);
                background:var(--a,#eef2ff); --empty:; }
             #item::before { --label:var(--label,'BAD'); content:var(--label,'GOOD') var(--empty,'BAD'); }
             </style><p id='item'>Body</p>", 800, 600);
        assert!(original.commands.iter().any(|command| matches!(command,
            PaintCommand::Text { text, color, .. } if text.contains("GOOD")
                && *color == op_paint::Color { r:0, g:0, b:255 }
        )));
        assert!(!original.commands.iter().any(|command| matches!(command,
            PaintCommand::Text { text, .. } if text.contains("BAD")
        )));
        engine.reflow(300, 600).unwrap();
        assert_eq!(engine.reflow(800, 600).unwrap().display_list, original);
    }

    #[test]
    fn invalid_computed_var_winners_reach_paint_as_inherited_or_initial_values() {
        let mut engine = Engine::new();
        let original = engine.set_html_page(
            "<style>#parent { color:#123456 }
             #child { --bad:nope; color:red; color:var(--missing); padding:20px; padding:var(--bad);
                 background:red; background:var(--bad); border:4px solid red; border:var(--missing) }
             #child::before { content:'BAD'; content:var(--missing) }
             </style><div id='parent'><div id='child'>Body</div></div>", 800, 600);
        assert!(original.commands.iter().any(|command| matches!(command,
            PaintCommand::Text { x:32, text, color, .. } if text == "Body"
                && *color == op_paint::Color { r:0x12, g:0x34, b:0x56 }
        )));
        assert!(!original.commands.iter().any(|command| matches!(command,
            PaintCommand::Text { text, .. } if text.contains("BAD")
        )));
        assert!(!original.commands.iter().any(|command| matches!(command,
            PaintCommand::FillRect { color, .. } if *color == op_paint::Color { r:255, g:0, b:0 }
        )));
        engine.reflow(300, 600).unwrap();
        assert_eq!(engine.reflow(800, 600).unwrap().display_list, original);
    }

    #[test]
    fn filtered_nth_and_forgiving_selectors_reach_generated_paint_and_reflow() {
        let mut engine = Engine::new();
        let original = engine.set_html_page(
            "<style>:nth-child(2 of .pick)::before { content:'SECOND '; color:red }
             :nth-last-child(1 of .pick)::after { content:' LAST'; color:blue }
             :is(:unsupported, .pick) { font-style:italic }
             </style><div><span class='pick'>One</span><em>Skip</em>
             <span class='pick'>Two</span><span class='pick'>Three</span></div>",
            800,
            600,
        );
        assert!(original.commands.iter().any(|command| matches!(command,
            PaintCommand::Text { text, color, italic:true, .. } if text.contains("SECOND")
                && *color == op_paint::Color { r:255, g:0, b:0 }
        )));
        assert!(original.commands.iter().any(|command| matches!(command,
            PaintCommand::Text { text, color, .. } if text.contains("LAST")
                && *color == op_paint::Color { r:0, g:0, b:255 }
        )));
        engine.reflow(300, 600).unwrap();
        assert_eq!(engine.reflow(800, 600).unwrap().display_list, original);
    }

    #[test]
    fn empty_generated_inline_edges_reach_fill_rects_without_text_commands() {
        let mut engine = Engine::new();
        let original = engine.set_html_page(
            "<style>#host::before { content:''; padding:2px 5px; border:1px solid blue; background:red }
             #host::after { content:''; padding:2px 7px; border:1px solid red; background:blue }
             </style><div id='host'>Body</div>", 800, 600);
        let text: Vec<_> = original
            .commands
            .iter()
            .filter_map(|command| match command {
                PaintCommand::Text { text, x, .. } => Some((text.as_str(), *x)),
                _ => None,
            })
            .collect();
        assert_eq!(text, vec![("Body", 44)]);
        assert!(original.commands.iter().any(|command| matches!(command,
            PaintCommand::FillRect { width:12, color, .. } if *color == op_paint::Color { r:255, g:0, b:0 }
        )));
        assert!(original.commands.iter().any(|command| matches!(command,
            PaintCommand::FillRect { width:16, color, .. } if *color == op_paint::Color { r:0, g:0, b:255 }
        )));
        engine.reflow(300, 600).unwrap();
        assert_eq!(engine.reflow(800, 600).unwrap().display_list, original);
    }

    #[test]
    fn authored_link_presentation_reaches_paint_and_retained_reflow() {
        let mut engine = Engine::new();
        let original = engine.set_html_page(
            "<style>#styled { color:red; text-decoration:none; letter-spacing:2px }
             #styled::before { content:'PRE'; color:green } #styled span { color:blue }</style>
             <p><a href='default.html'>Default</a> <a id='styled' href='next.html'>Styled<span>Nested</span></a></p>",
            800,
            600,
        );
        for (label, expected_color, expected_underline, href) in [
            ("Default", op_paint::Color::LINK, true, "default.html"),
            (
                "Styled",
                op_paint::Color { r: 255, g: 0, b: 0 },
                false,
                "next.html",
            ),
            (
                "PRE",
                op_paint::Color { r: 0, g: 128, b: 0 },
                false,
                "next.html",
            ),
            (
                "Nested",
                op_paint::Color { r: 0, g: 0, b: 255 },
                false,
                "next.html",
            ),
        ] {
            assert!(original.commands.iter().any(|command| matches!(command,
                PaintCommand::Text { text, color, underline, links, .. }
                    if text.contains(label) && *color == expected_color && *underline == expected_underline
                        && links.iter().any(|link| link.href == href)
            )), "missing computed link presentation for {label}");
        }
        engine.reflow(300, 600).unwrap();
        assert_eq!(engine.reflow(800, 600).unwrap().display_list, original);
    }

    #[test]
    fn typed_structural_selectors_reach_pseudo_text_colors_and_retained_reflow() {
        let mut engine = Engine::new();
        let original = engine.set_html_page(
            "<style>span:first-of-type::before { content:'FIRST '; color:red }
             span:last-of-type::after { content:' LAST'; color:blue }
             span:nth-of-type(2):nth-last-of-type(2) { color:green; font-weight:bold }
             </style><p><em>Lead</em><span>One</span><b>Skip</b><span>Two</span>
             <span>Three</span><em>Tail</em></p>",
            800,
            600,
        );
        assert!(original.commands.iter().any(|command| matches!(command,
            PaintCommand::Text { text, color, .. } if text.contains("FIRST")
                && *color == op_paint::Color { r:255, g:0, b:0 }
        )));
        assert!(original.commands.iter().any(|command| matches!(command,
            PaintCommand::Text { text, color, .. } if text.contains("LAST")
                && *color == op_paint::Color { r:0, g:0, b:255 }
        )));
        assert!(original.commands.iter().any(|command| matches!(command,
            PaintCommand::Text { text, color, bold:true, .. } if text.contains("Two")
                && *color == op_paint::Color { r:0, g:128, b:0 }
        )));
        engine.reflow(300, 600).unwrap();
        assert_eq!(engine.reflow(800, 600).unwrap().display_list, original);
    }

    #[test]
    fn custom_properties_and_var_reach_native_display_list() {
        let display_list = Engine::new().render_html(
            "<style>
               #card {
                 --accent:#b42318;
                 --surface:#eef2ff;
                 --edge:#4338ca;
                 --pad:4px 8px;
                 color:var(--accent);
                 background:var(--surface);
                 padding:var(--pad);
                 border:2px solid var(--edge);
               }
               #card::before {
                 --label:'[VAR] ';
                 content:var(--label);
                 color:var(--accent);
               }
             </style><p id='card'>Body</p>",
            800,
            600,
        );

        assert!(display_list.commands.iter().any(|command| matches!(
            command,
            PaintCommand::Text { text, color, .. }
                if text.contains("Body")
                    && *color == op_paint::Color { r: 180, g: 35, b: 24 }
        )));
        assert!(display_list.commands.iter().any(|command| matches!(
            command,
            PaintCommand::Text { text, color, .. }
                if text.contains("[VAR]")
                    && *color == op_paint::Color { r: 180, g: 35, b: 24 }
        )));
        assert!(display_list.commands.iter().any(|command| matches!(
            command,
            PaintCommand::FillRect { color, .. }
                if *color == op_paint::Color { r: 238, g: 242, b: 255 }
        )));
        assert!(display_list.commands.iter().any(|command| matches!(
            command,
            PaintCommand::FillRect { color, .. }
                if *color == op_paint::Color { r: 67, g: 56, b: 202 }
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
    fn hwb_colors_reach_text_background_borders_generated_runs_and_reflow() {
        let mut engine = Engine::new();
        let original = engine.set_html_page("<style>#p { --hue:120;color:hwb(var(--hue) 30 50 / 50%);background:hwb(240 0 0);border:2px solid hwb(45 40% 80%) } #p::before { content:'HWB ';color:hwb(0 0 0) }</style><p id=p>Body</p>", 800, 600);
        assert!(original.commands.iter().any(|command| matches!(command, PaintCommand::Text { text, color, .. } if text == "Body" && *color == (op_paint::Color { r:166, g:191, b:166 }))));
        assert!(original.commands.iter().any(|command| matches!(command, PaintCommand::Text { text, color, .. } if text.contains("HWB") && *color == (op_paint::Color { r:255, g:0, b:0 }))));
        for expected in [
            op_paint::Color { r: 0, g: 0, b: 255 },
            op_paint::Color {
                r: 85,
                g: 85,
                b: 85,
            },
        ] {
            assert!(original.commands.iter().any(|command| matches!(command, PaintCommand::FillRect { color, .. } if *color == expected)));
        }
        engine.reflow(240, 600).unwrap();
        assert_eq!(engine.reflow(800, 600).unwrap().display_list, original);
    }

    #[test]
    fn expanded_named_colors_reach_paint_and_retained_reflow() {
        let mut engine = Engine::new();
        let original = engine.set_html_page("<style>p { --accent:StEeLbLuE;color:var(--accent);background:PapayaWhip;border:2px solid CornFlowerBlue } p::before { content:'[named] ';color:DarkSlateGrey }</style><p>Body</p>", 800, 600);
        assert!(original.commands.iter().any(|command| matches!(command, PaintCommand::Text { text, color, .. } if text == "Body" && *color == (op_paint::Color { r:70, g:130, b:180 }))));
        assert!(original.commands.iter().any(|command| matches!(command, PaintCommand::Text { text, color, .. } if text.contains("[named]") && *color == (op_paint::Color { r:47, g:79, b:79 }))));
        for expected in [
            op_paint::Color {
                r: 255,
                g: 239,
                b: 213,
            },
            op_paint::Color {
                r: 100,
                g: 149,
                b: 237,
            },
        ] {
            assert!(original.commands.iter().any(|command| matches!(command, PaintCommand::FillRect { color, .. } if *color == expected)));
        }
        engine.reflow(240, 600).unwrap();
        assert_eq!(engine.reflow(800, 600).unwrap().display_list, original);
    }

    #[test]
    fn predefined_srgb_spaces_reach_text_box_and_generated_paint() {
        let mut engine = Engine::new();
        let original = engine.set_html_page("<style>p { --half:50%;color:color(srgb .25 .5 .75);background:color(srgb-linear var(--half) 25% 12.5%);border:2px solid color(srgb 1 0 0) } p::before { content:'[linear] ';color:color(srgb-linear .5 .25 .125 / .5) }</style><p>Body</p>",800,600);
        assert!(original.commands.iter().any(|command| matches!(command,PaintCommand::Text { text,color,.. } if text=="Body" && *color==(op_paint::Color { r:64,g:128,b:191 }))));
        assert!(original.commands.iter().any(|command| matches!(command,PaintCommand::Text { text,color,.. } if text.contains("[linear]") && *color==(op_paint::Color { r:221,g:196,b:177 }))));
        assert!(original.commands.iter().any(|command| matches!(command,PaintCommand::FillRect { color,.. } if *color==(op_paint::Color { r:188,g:137,b:99 }))));
        assert!(original.commands.iter().any(|command| matches!(command,PaintCommand::FillRect { color,.. } if *color==(op_paint::Color { r:255,g:0,b:0 }))));
        engine.reflow(240, 600).unwrap();
        assert_eq!(engine.reflow(800, 600).unwrap().display_list, original);
    }

    #[test]
    fn modern_rgb_hsl_missing_components_and_numbers_reach_native_paint() {
        let mut engine = Engine::new();
        let original = engine.set_html_page("<style>p { color:rgb(none 50% 255);background:hsl(none none 50);border:2px solid rgba(255 0% none) } p::before { content:'[modern] ';color:hsla(120 100 50) }</style><p>Body</p>",800,600);
        assert!(original.commands.iter().any(|command| matches!(command,PaintCommand::Text { text,color,.. } if text=="Body" && *color==(op_paint::Color { r:0,g:128,b:255 }))));
        assert!(original.commands.iter().any(|command| matches!(command,PaintCommand::Text { text,color,.. } if text.contains("[modern]") && *color==(op_paint::Color { r:0,g:255,b:0 }))));
        for expected in [
            op_paint::Color {
                r: 128,
                g: 128,
                b: 128,
            },
            op_paint::Color { r: 255, g: 0, b: 0 },
        ] {
            assert!(original.commands.iter().any(
                |command| matches!(command,PaintCommand::FillRect { color,.. } if *color==expected)
            ));
        }
        engine.reflow(240, 600).unwrap();
        assert_eq!(engine.reflow(800, 600).unwrap().display_list, original);
    }

    #[test]
    fn doctypes_and_bogus_declarations_do_not_paint_or_change_reflow() {
        let mut reference = Engine::new();
        let expected = reference.set_html_page("<p>ab</p>", 800, 600);
        for declaration in [
            "<!doctype html>",
            "<!doctype html PUBLIC 'legacy' 'system'>",
            "<!doctype html SYSTEM 'broken>",
            "<!bogus>",
            "<![CDATA[hidden]]>",
        ] {
            let mut engine = Engine::new();
            let source = format!("{declaration}<p>a{declaration}b</p>");
            assert_eq!(engine.set_html_page(&source, 800, 600), expected);
            assert_eq!(
                engine.reflow(240, 600).unwrap().display_list,
                reference.reflow(240, 600).unwrap().display_list
            );
        }
    }

    #[test]
    fn html_comments_do_not_paint_or_change_empty_selector_geometry() {
        let style = "<style>span:empty{padding:2px;border:1px solid red;background:blue}</style>";
        let mut engine = Engine::new();
        let original = engine.set_html_page(&format!("<!--hidden-->{style}<p>a<!--<img src=missing>--><span><!--empty--></span>b</p><!--unfinished"), 800, 600);
        let mut reference = Engine::new();
        assert_eq!(
            original,
            reference.set_html_page(&format!("{style}<p>a<span></span>b</p>"), 800, 600)
        );
        assert!(!contains_text(&original, "hidden"));
        assert!(!contains_text(&original, "empty"));
        for width in [240, 800] {
            assert_eq!(
                engine.reflow(width, 600).unwrap().display_list,
                reference.reflow(width, 600).unwrap().display_list
            );
        }
    }

    #[test]
    fn empty_inline_descendant_frames_paint_without_hidden_text_and_survive_reflow() {
        let mut engine = Engine::new();
        let original = engine.set_html_page("<p><span style='padding:2px;border:1px solid red;background:blue'><em style='display:none'>hidden</em></span>Tail</p><p><span style='padding:2px;border:1px solid green;background:red'> \n </span></p>",800,600);
        assert!(original.commands.iter().any(|command| matches!(command,PaintCommand::FillRect { width:6,color,.. } if *color==(op_paint::Color { r:0,g:0,b:255 }))));
        assert!(original.commands.iter().any(
            |command| matches!(command,PaintCommand::FillRect { width:6,height,.. } if *height>6)
        ));
        assert!(!contains_text(&original, "hidden"));
        assert_eq!(
            original
                .commands
                .iter()
                .filter(|command| matches!(command, PaintCommand::Text { .. }))
                .count(),
            1
        );
        engine.reflow(240, 600).unwrap();
        assert_eq!(engine.reflow(800, 600).unwrap().display_list, original);
    }

    #[test]
    fn positioned_absolute_and_fixed_contents_paint_over_normal_flow() {
        for position in ["absolute", "fixed"] {
            let html = format!(
                "<div style='position:relative;height:90px'>\
                 <div style='position:{position};left:0;top:0;background:lime;width:170px;height:45px'>FRONT</div>\
                 <p>Normal flow underneath</p></div>"
            );
            let page = Engine::new().render_html(&html, 800, 600);
            let commands = &page.commands;
            let normal = commands.iter().position(|cmd| {
                matches!(cmd, PaintCommand::Text { text, .. } if text.contains("Normal flow"))
            }).expect("normal-flow text");
            let background = commands
                .iter()
                .position(|cmd| {
                    matches!(cmd, PaintCommand::FillRect { color, width:170, height:45, .. }
                    if *color == (op_paint::Color { r:0, g:255, b:0 }))
                })
                .expect("positioned background");
            let foreground = commands
                .iter()
                .position(|cmd| matches!(cmd, PaintCommand::Text { text, .. } if text == "FRONT"))
                .expect("positioned foreground text");
            assert!(normal < background && background < foreground, "{position}");
        }
    }

    #[test]
    fn positioned_z_index_changes_actual_background_paint_order() {
        // The source order intentionally contradicts the stacking levels.
        let html = r#"<div style='position:relative;height:90px'>
            <div style='position:absolute;top:0;left:0;width:60px;height:40px;background:red;z-index:8'></div>
            <div style='position:absolute;top:0;left:0;width:60px;height:40px;background:lime;z-index:-2'></div>
            <div style='position:absolute;top:0;left:0;width:60px;height:40px;background:blue;z-index:2'></div>
            </div>"#;
        let mut engine = Engine::new();
        let original = engine.set_html_page(html, 800, 600);
        let order = |commands: &[PaintCommand]| {
            commands
                .iter()
                .filter_map(|command| match command {
                    PaintCommand::FillRect {
                        color,
                        width: 60,
                        height: 40,
                        ..
                    } => Some((color.r, color.g, color.b)),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(
            order(&original.commands),
            [(0, 255, 0), (0, 0, 255), (255, 0, 0)]
        );
        engine.reflow(420, 600).unwrap();
        assert_eq!(
            order(&engine.reflow(800, 600).unwrap().display_list.commands),
            [(0, 255, 0), (0, 0, 255), (255, 0, 0)]
        );
    }

    #[test]
    fn child_high_z_stays_below_later_sibling_context() {
        let html = r#"<div style="position:relative;height:100px">
          <div style="position:relative;z-index:1;background:red;width:70px;height:60px">
            <div style="position:absolute;z-index:999;top:0;left:0;background:lime;width:60px;height:40px"></div>
          </div>
          <div style="position:absolute;z-index:2;top:0;left:0;background:blue;width:55px;height:40px"></div>
        </div>"#;
        let commands = Engine::new().render_html(html, 800, 600).commands;
        let colors: Vec<_> = commands
            .iter()
            .filter_map(|item| match item {
                PaintCommand::FillRect { color, width, .. } if [70, 60, 55].contains(width) => {
                    Some((color.r, color.g, color.b))
                }
                _ => None,
            })
            .collect();
        assert_eq!(colors, [(255, 0, 0), (0, 255, 0), (0, 0, 255)]);
    }

    #[test]
    fn empty_stacking_context_still_contains_high_z_descendant() {
        let html = r#"<div style="position:relative;height:100px">
          <div style="position:relative;z-index:1;width:70px;height:60px">
            <div style="position:absolute;z-index:999;top:0;left:0;background:lime;width:60px;height:40px"></div>
          </div>
          <div style="position:absolute;z-index:2;top:0;left:0;background:blue;width:55px;height:40px"></div>
        </div>"#;
        let commands = Engine::new().render_html(html, 800, 600).commands;
        let colors: Vec<_> = commands
            .iter()
            .filter_map(|item| match item {
                PaintCommand::FillRect { color, width, .. } if [60, 55].contains(width) => {
                    Some((color.r, color.g, color.b))
                }
                _ => None,
            })
            .collect();
        assert_eq!(colors, [(0, 255, 0), (0, 0, 255)]);
    }

    #[test]
    fn negative_positioned_context_paints_beneath_normal_block() {
        let html = r#"<div style="position:relative;height:100px">
          <div style="position:absolute;z-index:-1;top:0;left:0;background:blue;width:55px;height:40px"></div>
          <div style="background:lime;width:60px;height:40px">NORMAL</div>
        </div>"#;
        let commands = Engine::new().render_html(html, 800, 600).commands;
        let negative = commands
            .iter()
            .position(|item| {
                matches!(item,
                    PaintCommand::FillRect { color, width:55, .. }
                        if *color == (op_paint::Color { r:0, g:0, b:255 })
                )
            })
            .unwrap();
        let normal = commands
            .iter()
            .position(|item| {
                matches!(item,
                    PaintCommand::FillRect { color, width:60, .. }
                        if *color == (op_paint::Color { r:0, g:255, b:0 })
                )
            })
            .unwrap();
        assert!(negative < normal);
    }

    #[test]
    fn positioned_same_level_follows_reparented_dom_order_not_creation_order() {
        let html = r#"<table>
          <tr><td><div style="position:absolute;top:0;left:0;z-index:0;background:blue;width:62px;height:40px"></div></td></tr>
          <div style="position:absolute;top:0;left:0;z-index:0;background:red;width:61px;height:40px"></div>
        </table>"#;
        let commands = Engine::new().render_html(html, 800, 600).commands;
        let ordered: Vec<_> = commands
            .iter()
            .filter_map(|item| match item {
                PaintCommand::FillRect { color, width, .. } if [61, 62].contains(width) => {
                    Some((color.r, color.g, color.b))
                }
                _ => None,
            })
            .collect();
        // Foster parenting moves the late div before the table containing blue.
        assert_eq!(ordered, [(255, 0, 0), (0, 0, 255)]);
    }

    #[test]
    fn positioned_inline_background_and_text_obey_explicit_z_index() {
        let html = r#"<div style="position:relative">
            <span style="position:relative;z-index:5;background:red">HIGH</span>
            <span style="position:relative;z-index:1;background:lime">LOW</span>
            <div style="position:absolute;top:0;left:0;z-index:2;background:blue;width:50px;height:30px"></div>
        </div>"#;
        let mut engine = Engine::new();
        let first = engine.set_html_page(html, 800, 600);
        let inspect = |commands: &[PaintCommand]| {
            commands
                .iter()
                .filter_map(|item| match item {
                    PaintCommand::FillRect { color, .. }
                        if *color == (op_paint::Color { r: 255, g: 0, b: 0 }) =>
                    {
                        Some("high-bg")
                    }
                    PaintCommand::FillRect { color, .. }
                        if *color == (op_paint::Color { r: 0, g: 255, b: 0 }) =>
                    {
                        Some("low-bg")
                    }
                    PaintCommand::FillRect { color, .. }
                        if *color == (op_paint::Color { r: 0, g: 0, b: 255 }) =>
                    {
                        Some("middle-bg")
                    }
                    PaintCommand::Text { text, .. } if text.contains("HIGH") => Some("high-text"),
                    PaintCommand::Text { text, .. } if text.contains("LOW") => Some("low-text"),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(
            inspect(&first.commands),
            ["low-bg", "low-text", "middle-bg", "high-bg", "high-text"]
        );
        engine.reflow(330, 600).unwrap();
        assert_eq!(
            inspect(&engine.reflow(800, 600).unwrap().display_list.commands),
            ["low-bg", "low-text", "middle-bg", "high-bg", "high-text"]
        );
    }

    #[test]
    fn positioned_inline_context_keeps_high_child_below_outside_sibling() {
        let html = r#"<div style="position:relative">
            <span style="position:relative;z-index:1;background:red">PARENT
              <span style="position:relative;z-index:999;background:lime">CHILD</span>
            </span>
            <div style="position:absolute;top:0;left:0;z-index:2;background:blue;width:50px;height:30px"></div>
        </div>"#;
        let page = Engine::new().render_html(html, 800, 600);
        let colors: Vec<_> = page
            .commands
            .iter()
            .filter_map(|item| match item {
                PaintCommand::FillRect { color, .. }
                    if (color.r, color.g, color.b) == (255, 0, 0) =>
                {
                    Some("parent")
                }
                PaintCommand::FillRect { color, .. }
                    if (color.r, color.g, color.b) == (0, 255, 0) =>
                {
                    Some("child")
                }
                PaintCommand::FillRect { color, .. }
                    if (color.r, color.g, color.b) == (0, 0, 255) =>
                {
                    Some("sibling")
                }
                _ => None,
            })
            .collect();
        assert_eq!(colors, ["parent", "child", "sibling"]);
    }

    #[test]
    fn inline_z_auto_does_not_trap_explicitly_stacked_descendants() {
        let html = r#"<div style="position:relative">
            <span style="position:relative;z-index:auto;background:red">PARENT
              <span style="position:relative;z-index:99;background:lime">CHILD</span>
            </span>
            <div style="position:absolute;top:0;left:0;z-index:2;background:blue;width:50px;height:30px"></div>
        </div>"#;
        let page = Engine::new().render_html(html, 800, 600);
        let colors: Vec<_> = page
            .commands
            .iter()
            .filter_map(|item| match item {
                PaintCommand::FillRect { color, .. }
                    if (color.r, color.g, color.b) == (255, 0, 0) =>
                {
                    Some("parent")
                }
                PaintCommand::FillRect { color, .. }
                    if (color.r, color.g, color.b) == (0, 255, 0) =>
                {
                    Some("child")
                }
                PaintCommand::FillRect { color, .. }
                    if (color.r, color.g, color.b) == (0, 0, 255) =>
                {
                    Some("sibling")
                }
                _ => None,
            })
            .collect();
        assert_eq!(colors, ["parent", "sibling", "child"]);
    }

    #[test]
    fn wrapped_positioned_inline_keeps_all_fragments_in_one_paint_group() {
        let html = r#"<div style="position:relative;width:180px">
            <span style="position:relative;z-index:4;background:red">
            These many words have to wrap over several lines within a narrow parent and retain their painted fragments together
            </span>
            <div style="position:absolute;z-index:2;top:0;left:0;background:blue;width:38px;height:20px"></div>
        </div>"#;
        let page = Engine::new().render_html(html, 320, 600);
        let blue = page
            .commands
            .iter()
            .position(|command| {
                matches!(
                    command, PaintCommand::FillRect { color, .. }
                    if *color == (op_paint::Color {r:0,g:0,b:255})
                )
            })
            .expect("middle stacking context");
        let red_fragments: Vec<_> = page
            .commands
            .iter()
            .enumerate()
            .filter_map(|(i, command)| {
                matches!(command, PaintCommand::FillRect { color, .. }
                if *color == (op_paint::Color {r:255,g:0,b:0}))
                .then_some(i)
            })
            .collect();
        assert!(red_fragments.len() >= 2, "the inline should have wrapped");
        assert!(red_fragments.iter().all(|i| *i > blue));
    }

    #[test]
    fn atomic_inline_child_uses_parent_inline_stacking_level() {
        let html = r#"<div style="position:relative">
            <span style="position:relative;z-index:5;background:red">
                <span style="display:inline-block;background:lime;padding:3px">ATOMIC</span>
            </span>
            <div style="position:absolute;z-index:2;top:0;left:0;background:blue;width:50px;height:30px"></div>
        </div>"#;
        let page = Engine::new().render_html(html, 800, 600);
        let colors: Vec<_> = page
            .commands
            .iter()
            .filter_map(|command| match command {
                PaintCommand::FillRect { color, .. }
                    if (color.r, color.g, color.b) == (0, 0, 255) =>
                {
                    Some("outside")
                }
                PaintCommand::FillRect { color, .. }
                    if (color.r, color.g, color.b) == (255, 0, 0) =>
                {
                    Some("parent")
                }
                PaintCommand::FillRect { color, .. }
                    if (color.r, color.g, color.b) == (0, 255, 0) =>
                {
                    Some("atomic")
                }
                _ => None,
            })
            .collect();
        assert_eq!(colors, ["outside", "parent", "atomic"]);
    }

    #[test]
    fn relative_auto_block_paints_own_background_in_zero_level_source_order() {
        let html = r#"<div style="position:relative;height:120px">
            <div style="position:absolute;z-index:0;top:0;left:0;width:50px;height:25px;background:blue"></div>
            <div style="position:relative;z-index:auto;background:red;width:70px;height:60px">
              PARENT
              <div style="position:absolute;z-index:0;top:0;left:0;background:lime;width:60px;height:20px"></div>
            </div>
        </div>"#;
        let commands = Engine::new().render_html(html, 800, 600).commands;
        let colors: Vec<_> = commands
            .iter()
            .filter_map(|item| match item {
                PaintCommand::FillRect { color, width, .. } if [50, 70, 60].contains(width) => {
                    Some((color.r, color.g, color.b))
                }
                _ => None,
            })
            .collect();
        assert_eq!(colors, [(0, 0, 255), (255, 0, 0), (0, 255, 0)]);
    }

    #[test]
    fn relative_auto_inline_block_paints_own_background_in_zero_level_source_order() {
        let html = r#"<div style="position:relative;height:120px">
            <div style="position:absolute;z-index:0;top:0;left:0;width:50px;height:25px;background:blue"></div>
            <span style="display:inline-block;position:relative;z-index:auto;background:red;width:70px;height:60px">
              PARENT
              <span style="position:absolute;z-index:0;top:0;left:0;background:lime;width:60px;height:20px"></span>
            </span>
        </div>"#;
        let commands = Engine::new().render_html(html, 800, 600).commands;
        let colors: Vec<_> = commands
            .iter()
            .filter_map(|item| match item {
                PaintCommand::FillRect { color, width, .. } if [50, 70, 60].contains(width) => {
                    Some((color.r, color.g, color.b))
                }
                _ => None,
            })
            .collect();
        assert_eq!(colors, [(0, 0, 255), (255, 0, 0), (0, 255, 0)]);
    }

    #[test]
    fn relative_auto_inline_block_child_stacks_above_outside_higher_z() {
        let html = r#"<div style="position:relative;height:120px">
            <span style="display:inline-block;position:relative;z-index:auto;background:red;width:70px;height:60px">
              <span style="position:absolute;z-index:9;top:0;left:0;background:lime;width:60px;height:20px"></span>
            </span>
            <div style="position:absolute;z-index:2;top:0;left:0;width:50px;height:25px;background:blue"></div>
        </div>"#;
        let commands = Engine::new().render_html(html, 800, 600).commands;
        let colors: Vec<_> = commands
            .iter()
            .filter_map(|item| match item {
                PaintCommand::FillRect { color, width, .. } if [50, 70, 60].contains(width) => {
                    Some((color.r, color.g, color.b))
                }
                _ => None,
            })
            .collect();
        assert_eq!(colors, [(255, 0, 0), (0, 0, 255), (0, 255, 0)]);
    }

    #[test]
    fn inline_block_with_explicit_z_is_atomic_against_outside_sibling() {
        let html = r#"<div style="position:relative;height:120px">
            <span style="display:inline-block;position:relative;z-index:1;background:red;width:70px;height:60px">
              <span style="position:absolute;z-index:99;top:0;left:0;background:lime;width:60px;height:20px"></span>
            </span>
            <div style="position:absolute;z-index:2;top:0;left:0;width:50px;height:25px;background:blue"></div>
        </div>"#;
        let mut engine = Engine::new();
        let first = engine.set_html_page(html, 800, 600);
        let colors = |commands: &[PaintCommand]| -> Vec<_> {
            commands
                .iter()
                .filter_map(|item| match item {
                    PaintCommand::FillRect { color, width, .. } if [50, 70, 60].contains(width) => {
                        Some((color.r, color.g, color.b))
                    }
                    _ => None,
                })
                .collect()
        };
        assert_eq!(
            colors(&first.commands),
            [(255, 0, 0), (0, 255, 0), (0, 0, 255)]
        );
        engine.reflow(440, 600).unwrap();
        assert_eq!(
            colors(&engine.reflow(800, 600).unwrap().display_list.commands),
            [(255, 0, 0), (0, 255, 0), (0, 0, 255)]
        );
    }

    #[test]
    fn relative_auto_inline_block_negative_child_paints_under_own_background() {
        let html = r#"<div style="position:relative;height:120px">
            <span style="display:inline-block;position:relative;z-index:auto;background:red;width:70px;height:60px">
              <span style="position:absolute;z-index:-1;top:0;left:0;background:lime;width:60px;height:20px"></span>
            </span>
        </div>"#;
        let commands = Engine::new().render_html(html, 800, 600).commands;
        let colors: Vec<_> = commands
            .iter()
            .filter_map(|item| match item {
                PaintCommand::FillRect { color, width, .. } if [70, 60].contains(width) => {
                    Some((color.r, color.g, color.b))
                }
                _ => None,
            })
            .collect();
        assert_eq!(colors, [(0, 255, 0), (255, 0, 0)]);
    }

    #[test]
    fn table_row_group_relative_shift_matches_indicator_geometry() {
        let html = r#"<style>
          table { border-collapse:collapse; } td { padding:0; }
          td > div { height:50px;width:50px; }
          .group {display:inline-block;position:relative;width:150px;height:200px;}
          .indicator {position:absolute;background:red;left:100px;height:50px;width:50px;}
          .relative {position:relative;left:100px;background:green;}
        </style>
        <div class="group"><div>
        <div class="indicator"></div>
        <table><tbody class="relative"><tr><td><div></div></td></tr></tbody></table>
        </div></div>"#;
        let page = Engine::new().render_html(html, 800, 600);
        let rectangles: Vec<_> = page
            .commands
            .iter()
            .filter_map(|command| match command {
                PaintCommand::FillRect {
                    x,
                    y,
                    width,
                    height,
                    color,
                } if (color.r == 255 && color.g == 0 && color.b == 0)
                    || (color.r == 0 && color.g == 128 && color.b == 0) =>
                {
                    Some((color.r, color.g, color.b, *x, *y, *width, *height))
                }
                _ => None,
            })
            .collect();
        assert_eq!(rectangles.len(), 2, "{rectangles:?}");
        let red = rectangles[0];
        let green = rectangles[1];
        assert_eq!(
            (green.3, green.4, green.5, green.6),
            (red.3, red.4, red.5, red.6),
            "{rectangles:?}"
        );
    }

    #[test]
    fn relative_table_footer_is_containing_block_for_absolute_cell_child() {
        let html = r#"<style>
          table {border-collapse:collapse} td {padding:0}
          td > div {width:50px;height:50px}
          .group {display:inline-block;position:relative;width:150px;height:200px}
          .relative {position:relative;top:50px;background:white}
          .absolute {position:absolute;top:50px;background:green}
        </style>
        <div class="group">
          <div style="position:absolute;left:0;top:150px;width:50px;height:50px;background:red"></div>
          <table><tbody><tr><td><div></div></td></tr></tbody>
          <tfoot class="relative"><tr><td style="width:50px;height:50px"><div class="absolute"></div></td></tr></tfoot></table>
        </div>"#;
        let page = Engine::new().render_html(html, 800, 600);
        let positions: Vec<_> = page
            .commands
            .iter()
            .filter_map(|command| match command {
                PaintCommand::FillRect {
                    x,
                    y,
                    width: 50,
                    height: 50,
                    color,
                } if (color.r, color.g, color.b) == (255, 0, 0)
                    || (color.r, color.g, color.b) == (0, 128, 0) =>
                {
                    Some((color.r, color.g, color.b, *x, *y))
                }
                _ => None,
            })
            .collect();
        assert_eq!(positions.len(), 2, "{positions:?}");
        assert_eq!(
            (positions[0].3, positions[0].4),
            (positions[1].3, positions[1].4),
            "{positions:?}"
        );
    }

    #[test]
    fn empty_relative_table_section_does_not_paint_a_stray_pixel() {
        let html = r#"<style>
          table {border-collapse:collapse} td {padding:0}
          .group {display:inline-block;position:relative;width:150px;height:200px}
          .relative {position:relative;left:50px;background:green}
          .absolute {position:absolute;left:50px;width:50px;height:50px;background:green}
        </style>
        <div class="group">
          <table><tbody class="relative"><tr><td><div class="absolute"></div></td></tr></tbody></table>
        </div>"#;
        let page = Engine::new().render_html(html, 800, 600);
        let fills: Vec<_> = page
            .commands
            .iter()
            .filter_map(|command| match command {
                PaintCommand::FillRect {
                    x,
                    y,
                    width,
                    height,
                    color,
                } if (color.r, color.g, color.b) == (0, 128, 0) => Some((*x, *y, *width, *height)),
                _ => None,
            })
            .collect();
        assert_eq!(fills.len(), 1, "{fills:?}");
        assert_eq!(fills[0].2, 50);
        assert_eq!(fills[0].3, 50);
    }

    #[test]
    fn relative_table_cell_background_paints_over_earlier_absolute_indicator() {
        let html = r#"<style>
          table {border-collapse:collapse} td {padding:0}
          td > div {width:50px;height:50px}
          .group {display:inline-block;position:relative;width:150px;height:200px}
          .relative {position:relative;top:100px;background:green}
        </style>
        <div class="group">
          <div style="position:absolute;left:0;top:100px;width:50px;height:50px;background:red"></div>
          <table><tbody><tr><td class="relative"><div></div></td></tr></tbody></table>
        </div>"#;
        let page = Engine::new().render_html(html, 800, 600);
        let fills: Vec<_> = page
            .commands
            .iter()
            .filter_map(|command| match command {
                PaintCommand::FillRect {
                    x,
                    y,
                    width: 50,
                    height: 50,
                    color,
                } if (color.r, color.g, color.b) == (255, 0, 0)
                    || (color.r, color.g, color.b) == (0, 128, 0) =>
                {
                    Some((color.r, color.g, color.b, *x, *y))
                }
                _ => None,
            })
            .collect();
        assert_eq!(fills.len(), 2, "{fills:?}");
        assert_eq!((fills[0].3, fills[0].4), (fills[1].3, fills[1].4));
        assert_eq!((fills[1].0, fills[1].1, fills[1].2), (0, 128, 0));
    }

    #[test]
    fn relative_table_caption_overlays_preceding_absolute_indicator() {
        let html = "<div style='display:inline-block;position:relative;height:200px'>\
            <div style='position:absolute;left:0;top:100px;width:50px;height:50px;background:red'></div>\
            <table><caption style='position:relative;top:100px;width:50px;height:50px;background:lime'></caption></table></div>";
        let page = Engine::new().render_html(html, 800, 600);
        let commands = &page.commands;
        let position_of_color = |r: u8, g: u8| {
            commands
                .iter()
                .position(|cmd| {
                    matches!(cmd,
                PaintCommand::FillRect { color, width:50, height:50, .. }
                if color.r == r && color.g == g && color.b == 0)
                })
                .expect("colored positioned rectangle")
        };
        assert!(position_of_color(255, 0) < position_of_color(0, 255));
    }

    #[test]
    fn nested_inline_backgrounds_paint_outer_first_and_survive_reflow() {
        let html = "<p><a href=next style='padding:2px 3px;background:red'>A<span style='padding:4px 5px;background:blue'><b>nested label wraps across lines</b></span>Z</a></p>";
        let mut engine = Engine::new();
        let original = engine.set_html_page(html, 800, 600);
        for width in [180, 800] {
            let page = engine.reflow(width, 600).unwrap();
            let mut outer = None;
            for command in &page.display_list.commands {
                if let PaintCommand::FillRect {
                    x,
                    y,
                    width,
                    height,
                    color,
                } = command
                {
                    if *color == (op_paint::Color { r: 255, g: 0, b: 0 }) {
                        outer = Some((*x, *y, *width, *height));
                    } else if *color == (op_paint::Color { r: 0, g: 0, b: 255 }) {
                        let (left, top, outer_width, outer_height) =
                            outer.expect("outer background paints before inner");
                        // Font-derived inline boxes share the text baseline, not
                        // the outer border top: a deeply padded child can extend
                        // above or below its ancestor without leaving its line.
                        assert!(*x >= left);
                        assert!(*x + *width <= left + outer_width);
                        assert!(*y < top + outer_height);
                        assert!(top < *y + *height);
                    }
                }
            }
            assert!(outer.is_some());
            assert!(page.display_list.commands.iter().any(|command| matches!(command,
                PaintCommand::Text { text, links, bold: true, .. }
                    if text.contains("nested") && links.iter().any(|link| link.href == "next")
            )));
        }
        assert_eq!(engine.reflow(800, 600).unwrap().display_list, original);
    }

    #[test]
    fn unavailable_replaced_boxes_paint_css_without_rasters_and_survive_reflow() {
        let html = "<style>a::before { content:url(missing);display:block;width:80px;height:30px;box-sizing:border-box;padding:4px;border:2px solid red;background:blue;margin:8px auto 12px }</style><a href=next>Alt link</a><img src=missing alt='' style='width:40px;height:20px;padding:2px;border:1px solid green;background:red'><img src=missing alt='Fallback' style='padding:3px;border:1px solid blue'>";
        let mut engine = Engine::new();
        let original = engine.set_html_page(html, 800, 600);
        assert!(
            !original
                .commands
                .iter()
                .any(|command| matches!(command, PaintCommand::Image { .. }))
        );
        assert!(original.commands.iter().any(|command| matches!(command, PaintCommand::FillRect { width:80, height:30, color, .. } if *color == (op_paint::Color { r:0, g:0, b:255 }))));
        assert!(original.commands.iter().any(|command| matches!(command, PaintCommand::FillRect { width:46, height:26, color, .. } if *color == (op_paint::Color { r:255, g:0, b:0 }))));
        assert!(original.commands.iter().any(|command| matches!(command, PaintCommand::Text { text, links, .. } if text == "Alt link" && links[0].href == "next")));
        assert!(contains_text(&original, "Fallback"));
        engine.reflow(240, 600).unwrap();
        assert_eq!(engine.reflow(800, 600).unwrap().display_list, original);
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
