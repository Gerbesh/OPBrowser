//! Initial serial image subresources on the navigation worker; failures use alt.

use op_dom::Document;
use op_image::RasterImage;
use op_layout::ImageResources;
use op_net::{NetworkContext, resolve_image_source};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

const MAX_IMAGE_NODES: usize = 32;
const MAX_REQUESTS: usize = 8;
const ENCODED_BUDGET: usize = 8 * 1024 * 1024;
const PIXEL_BUDGET: usize = 32 * 1024 * 1024;

pub(super) fn load(network: &NetworkContext, document: &Document, base: &str) -> ImageResources {
    let started = Instant::now();
    let mut images = ImageResources::new();
    let mut cache: HashMap<String, Option<Arc<RasterImage>>> = HashMap::new();
    let (mut nodes, mut requests, mut encoded, mut pixels) = (0, 0, 0, 0);
    let mut stack = vec![document.root()];
    let mut root = document.root();
    while let Some(id) = stack.pop() {
        if document.element(id).is_some_and(|e| e.tag_name == "body") {
            root = id;
            break;
        }
        stack.extend(document.children(id).iter().rev());
    }
    stack.clear();
    stack.push(root);
    while let Some(id) = stack.pop() {
        if let Some(element) = document.element(id) {
            if matches!(
                element.tag_name.as_str(),
                "head" | "title" | "style" | "script" | "meta" | "link" | "template"
            ) {
                continue;
            }
            if element.tag_name == "img" {
                nodes += 1;
                if nodes > MAX_IMAGE_NODES {
                    break;
                }
                let Some(src) = element
                    .attributes
                    .iter()
                    .find(|a| a.name == "src")
                    .map(|a| a.value.as_str())
                else {
                    continue;
                };
                let Ok(source) = resolve_image_source(Some(base), src) else {
                    continue;
                };
                if let Some(Some(image)) = cache.get(&source) {
                    images.insert(id, image.clone());
                    continue;
                }
                if cache.contains_key(&source)
                    || requests >= MAX_REQUESTS
                    || encoded >= ENCODED_BUDGET
                    || pixels >= PIXEL_BUDGET
                    || started.elapsed() > Duration::from_secs(10)
                {
                    continue;
                }
                requests += 1;
                let image = network
                    .load_image(&source, ENCODED_BUDGET - encoded)
                    .ok()
                    .and_then(|bytes| {
                        encoded += bytes.len();
                        op_image::decode(&bytes, PIXEL_BUDGET - pixels).ok()
                    })
                    .map(|image| {
                        pixels += image.pixels().len();
                        Arc::new(image)
                    });
                if let Some(image) = &image {
                    images.insert(id, image.clone());
                }
                cache.insert(source, image);
                continue;
            }
        }
        stack.extend(document.children(id).iter().rev());
    }
    images
}
