use op_css::{
    ComputedStyleMap, StyleCollection, StyleError, StyleMap, collect_author_styles,
    collect_author_styles_with_linked, compute_styles,
};
use op_html::parse_document;
use op_layout::{
    ImageResources, layout_document_with_computed_styles_and_metrics,
    layout_document_with_resources_and_metrics,
};
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
    images: images::PageImages,
    style_collection: StyleCollection,
    computed_styles: ComputedStyleMap,
    stylesheet_addresses: std::collections::HashMap<op_dom::NodeId, String>,
}

impl PreparedDocument {
    fn render(&self, width: i32, height: i32) -> RenderedPage {
        let layout = layout_document_with_resources_and_metrics(
            &self.document,
            width,
            &self.images.elements,
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
            images: images::PageImages::default(),
            style_collection,
            computed_styles,
            stylesheet_addresses: std::collections::HashMap::new(),
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
        let style_collection =
            collect_author_styles_with_linked(&document, &linked_stylesheets.texts);
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
        assert!(background.2 > 20 && background.3 >= 34);
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
                        assert!(*x >= left && *y >= top);
                        assert!(*x + *width <= left + outer_width);
                        assert!(*y + *height <= top + outer_height);
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
