//! Bounded external author stylesheet discovery/loading for one prepared document.

use op_dom::{Document, NodeId};
use op_net::{NetworkContext, resolve_stylesheet_source};
use std::collections::HashMap;
use std::time::{Duration, Instant};

const MAX_STYLESHEET_NODES: usize = 32;
const MAX_REQUESTS: usize = 8;
const TEXT_BUDGET: usize = 2 * 1024 * 1024;

#[derive(Debug, Default)]
pub(super) struct LoadedStylesheets {
    pub texts: HashMap<NodeId, String>,
    pub addresses: HashMap<NodeId, String>,
}

impl LoadedStylesheets {
    fn insert(&mut self, node: NodeId, sheet: &op_net::LoadedStylesheet) {
        self.texts.insert(node, sheet.text.clone());
        self.addresses.insert(node, sheet.address.clone());
    }
}

/// Load bounded CSS @color-profile resources from inline and linked sheets.
/// URLs are resolved from the stylesheet's own address. No arbitrary file
/// reads bypass the normal network request filter.
pub(super) fn load_color_profiles(
    network: &NetworkContext,
    document: &Document,
    document_address: &str,
    linked: &LoadedStylesheets,
) -> HashMap<String, Vec<u8>> {
    const MAX_PROFILES: usize = 8;
    const MAX_PROFILE_BYTES: usize = 1024 * 1024;
    let mut profiles = HashMap::new();
    let mut remaining = MAX_PROFILE_BYTES;
    let mut requests = 0usize;
    let mut stack = vec![document.root()];
    let mut visited = 0usize;
    while let Some(id) = stack.pop() {
        visited += 1;
        if visited > 20_000 {
            break;
        }
        let css = if let Some(element) = document.element(id) {
            if element.tag_name.eq_ignore_ascii_case("style") {
                let mut text = String::new();
                let mut children = document
                    .children(id)
                    .iter()
                    .copied()
                    .rev()
                    .collect::<Vec<_>>();
                while let Some(child) = children.pop() {
                    if let Some(node) = document.node(child)
                        && let op_dom::NodeKind::Text(value) = &node.kind
                    {
                        text.push_str(value);
                    }
                    children.extend(document.children(child).iter().rev().copied());
                    if text.len() > 1024 * 1024 {
                        break;
                    }
                }
                Some((text, document_address))
            } else {
                linked.texts.get(&id).map(|text| {
                    (
                        text.clone(),
                        linked
                            .addresses
                            .get(&id)
                            .map_or(document_address, String::as_str),
                    )
                })
            }
        } else {
            None
        };
        if let Some((css, stylesheet_base)) = css {
            for (name, source) in op_css::parse_color_profiles(&css) {
                if requests >= MAX_PROFILES || remaining == 0 {
                    break;
                }
                let Ok(url) = op_net::resolve_image_source(Some(stylesheet_base), &source) else {
                    continue;
                };
                requests += 1;
                if let Ok(bytes) = network.load_image_for_page(
                    &url,
                    Some(document_address),
                    remaining.min(MAX_PROFILE_BYTES),
                ) {
                    remaining = remaining.saturating_sub(bytes.len());
                    profiles.insert(name, bytes);
                }
            }
        }
        stack.extend(document.children(id).iter().rev().copied());
    }
    profiles
}

pub(super) fn load(network: &NetworkContext, document: &Document, base: &str) -> LoadedStylesheets {
    let started = Instant::now();
    let mut linked = LoadedStylesheets::default();
    let mut cache: HashMap<String, Option<op_net::LoadedStylesheet>> = HashMap::new();
    let mut nodes = 0usize;
    let mut requests = 0usize;
    let mut text_bytes = 0usize;

    let mut stack = vec![document.root()];
    while let Some(node) = stack.pop() {
        if let Some(element) = document.element(node)
            && element.tag_name.eq_ignore_ascii_case("link")
            && is_active_stylesheet_link(element)
        {
            nodes += 1;
            if nodes > MAX_STYLESHEET_NODES {
                break;
            }

            let Some(href) = attribute(element, "href") else {
                stack.extend(document.children(node).iter().rev());
                continue;
            };
            let Ok(source) = resolve_stylesheet_source(Some(base), href) else {
                stack.extend(document.children(node).iter().rev());
                continue;
            };

            if let Some(Some(css)) = cache.get(&source) {
                linked.insert(node, css);
                stack.extend(document.children(node).iter().rev());
                continue;
            }
            if cache.contains_key(&source)
                || requests >= MAX_REQUESTS
                || text_bytes >= TEXT_BUDGET
                || started.elapsed() > Duration::from_secs(10)
            {
                stack.extend(document.children(node).iter().rev());
                continue;
            }

            requests += 1;
            let css = network
                .load_stylesheet_for_page(&source, Some(base), TEXT_BUDGET - text_bytes)
                .ok()
                .filter(|loaded| {
                    text_bytes = text_bytes.saturating_add(loaded.text.len());
                    text_bytes <= TEXT_BUDGET
                });

            if let Some(css) = &css {
                linked.insert(node, css);
            }
            cache.insert(source, css);
        }

        stack.extend(document.children(node).iter().rev());
    }

    linked
}

fn is_active_stylesheet_link(element: &op_dom::ElementData) -> bool {
    if attribute(element, "disabled").is_some() {
        return false;
    }

    let Some(rel) = attribute(element, "rel") else {
        return false;
    };
    let mut stylesheet = false;
    for token in rel.split_ascii_whitespace() {
        if token.eq_ignore_ascii_case("alternate") {
            return false;
        }
        if token.eq_ignore_ascii_case("stylesheet") {
            stylesheet = true;
        }
    }
    if !stylesheet {
        return false;
    }

    if attribute(element, "type").is_some_and(|value| {
        !value.trim().is_empty() && !value.trim().eq_ignore_ascii_case("text/css")
    }) {
        return false;
    }

    attribute(element, "media").is_none_or(|value| {
        let value = value.trim();
        value.is_empty()
            || value.eq_ignore_ascii_case("all")
            || value.eq_ignore_ascii_case("screen")
    })
}

fn attribute<'a>(element: &'a op_dom::ElementData, name: &str) -> Option<&'a str> {
    element
        .attributes
        .iter()
        .find(|attribute| attribute.name.eq_ignore_ascii_case(name))
        .map(|attribute| attribute.value.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;
    use op_html::parse_document;

    fn first_link(document: &Document) -> NodeId {
        let mut stack = vec![document.root()];
        while let Some(node) = stack.pop() {
            if document
                .element(node)
                .is_some_and(|element| element.tag_name == "link")
            {
                return node;
            }
            stack.extend(document.children(node).iter().rev());
        }
        panic!("expected link");
    }

    #[test]
    fn redirected_stylesheets_retain_effective_bases_through_cache_and_reflow() {
        use std::io::{Read, Write};
        use std::net::TcpListener;
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let address = format!("http://{}", listener.local_addr().unwrap());
        let server = std::thread::spawn(move || {
            let mut paths = Vec::new();
            for index in 0..3 {
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
                paths.push(request.lines().next().unwrap().to_owned());
                if index == 1 {
                    write!(stream, "HTTP/1.1 302 Found\r\nLocation: /assets/theme.css\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").unwrap();
                } else {
                    let (mime, body) = if index == 0 {
                        (
                            "text/html",
                            "<link rel='stylesheet' href='/theme.css'><link rel='stylesheet' href='/theme.css'><p>Styled</p>",
                        )
                    } else {
                        ("text/css", "p { color:red }")
                    };
                    write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: {mime}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
                }
            }
            paths
        });
        let mut engine = crate::Engine::new();
        let original = engine
            .navigate(&format!("{address}/index.html"), 800, 600)
            .unwrap()
            .display_list;
        let prepared = engine.active_document.as_ref().unwrap();
        assert_eq!(prepared.stylesheet_addresses.len(), 2);
        for node in prepared.stylesheet_addresses.keys() {
            assert_eq!(
                engine.active_stylesheet_address(*node),
                Some(format!("{address}/assets/theme.css").as_str())
            );
        }
        engine.reflow(300, 600).unwrap();
        assert_eq!(engine.reflow(800, 600).unwrap().display_list, original);
        assert_eq!(
            server.join().unwrap(),
            [
                "GET /index.html HTTP/1.1",
                "GET /theme.css HTTP/1.1",
                "GET /assets/theme.css HTTP/1.1"
            ]
        );
    }

    #[test]
    fn recognizes_only_active_screen_stylesheet_links() {
        for (attributes, expected) in [
            ("rel='stylesheet' href='a.css'", true),
            ("rel='STYLESHEET' media='screen' href='a.css'", true),
            ("rel='stylesheet' media='all' href='a.css'", true),
            ("rel='alternate stylesheet' href='a.css'", false),
            ("rel='stylesheet' disabled href='a.css'", false),
            ("rel='stylesheet' media='print' href='a.css'", false),
            ("rel='icon' href='a.css'", false),
            ("rel='stylesheet' type='text/plain' href='a.css'", false),
        ] {
            let document = parse_document(&format!("<link {attributes}><p>x</p>"));
            let link = first_link(&document);
            assert_eq!(
                is_active_stylesheet_link(document.element(link).unwrap()),
                expected,
                "{attributes}"
            );
        }
    }
}
