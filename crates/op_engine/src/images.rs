//! Bounded DOM and CSS-generated image subresources on the navigation worker.

use op_css::{ComputedStyleMap, Display, GeneratedContentItem, PseudoElement};
use op_dom::{Document, NodeId};
use op_image::RasterImage;
use op_layout::{GeneratedImageResources, ImageResources};
use op_net::{NetworkContext, resolve_image_source};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

const MAX_IMAGE_NODES: usize = 32;
const MAX_REQUESTS: usize = 8;
const ENCODED_BUDGET: usize = 8 * 1024 * 1024;
const PIXEL_BUDGET: usize = 32 * 1024 * 1024;

#[derive(Debug, Default)]
pub(super) struct PageImages {
    pub elements: ImageResources,
    pub generated: GeneratedImageResources,
}

pub(super) fn load(
    network: &NetworkContext,
    document: &Document,
    base: &str,
    computed: &ComputedStyleMap,
    stylesheet_addresses: &HashMap<NodeId, String>,
) -> PageImages {
    let mut loader = Loader {
        network,
        page_base: base,
        started: Instant::now(),
        cache: HashMap::new(),
        nodes: 0,
        requests: 0,
        encoded: 0,
        pixels: 0,
    };
    let mut images = PageImages::default();
    let mut search = vec![document.root()];
    let mut root = document.root();
    while let Some(id) = search.pop() {
        if document
            .element(id)
            .is_some_and(|element| element.tag_name == "body")
        {
            root = id;
            break;
        }
        search.extend(document.children(id).iter().rev());
    }
    let mut stack = vec![(root, false)];
    while let Some((id, after)) = stack.pop() {
        if computed
            .style_for(id)
            .is_some_and(|style| style.display == Display::None)
        {
            continue;
        }
        if let Some(element) = document.element(id) {
            if matches!(
                element.tag_name.as_str(),
                "head" | "title" | "style" | "script" | "meta" | "link" | "template"
            ) {
                continue;
            }
            let pseudo = if after {
                PseudoElement::After
            } else {
                PseudoElement::Before
            };
            if let Some(generated) = computed.pseudo_style_for(id, pseudo) {
                for (index, item) in generated.items.iter().enumerate() {
                    if let GeneratedContentItem::Image { url, style_node } = item {
                        let resource_base = stylesheet_addresses
                            .get(style_node)
                            .map_or(base, String::as_str);
                        if let Some(image) = loader.get(resource_base, url) {
                            images.generated.insert((id, pseudo, index), image);
                        }
                    }
                }
            }
            if after {
                continue;
            }
            if element.tag_name == "img" {
                if let Some(src) = element
                    .attributes
                    .iter()
                    .find(|attribute| attribute.name == "src")
                    && let Some(image) = loader.get(base, &src.value)
                {
                    images.elements.insert(id, image);
                }
                continue;
            }
            stack.push((id, true));
        } else if after {
            continue;
        }
        stack.extend(
            document
                .children(id)
                .iter()
                .rev()
                .map(|child| (*child, false)),
        );
    }
    images
}

struct Loader<'a> {
    network: &'a NetworkContext,
    page_base: &'a str,
    started: Instant,
    cache: HashMap<String, Option<Arc<RasterImage>>>,
    nodes: usize,
    requests: usize,
    encoded: usize,
    pixels: usize,
}

impl Loader<'_> {
    fn get(&mut self, base: &str, source: &str) -> Option<Arc<RasterImage>> {
        self.nodes += 1;
        if self.nodes > MAX_IMAGE_NODES {
            return None;
        }
        let source = resolve_image_source(Some(base), source).ok()?;
        if let Some(image) = self.cache.get(&source) {
            return image.clone();
        }
        if self.requests >= MAX_REQUESTS
            || self.encoded >= ENCODED_BUDGET
            || self.pixels >= PIXEL_BUDGET
            || self.started.elapsed() > Duration::from_secs(10)
        {
            return None;
        }
        self.requests += 1;
        let image = self
            .network
            .load_image_for_page(&source, Some(self.page_base), ENCODED_BUDGET - self.encoded)
            .ok()
            .and_then(|bytes| {
                self.encoded += bytes.len();
                op_image::decode(&bytes, PIXEL_BUDGET - self.pixels).ok()
            })
            .map(|image| {
                self.pixels += image.pixels().len();
                Arc::new(image)
            });
        self.cache.insert(source, image.clone());
        image
    }
}
