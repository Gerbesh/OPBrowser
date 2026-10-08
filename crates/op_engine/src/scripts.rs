//! First page-scripting vertical slice: bounded, inline classic scripts and
//! detached DOM text mutations through OPBrowser's own JavaScript VM.
//! This is intentionally not a complete HTML script-processing model.
use op_dom::{Document, NodeId, NodeKind};
use op_js::{DomElementSnapshot, JsRuntime};
use op_net::{NetworkContext, resolve_script_source};
use std::time::{Duration, Instant};

const MAX_NODES: usize = 20_000;
const MAX_ELEMENTS: usize = 4096;
const MAX_SCRIPT_BYTES: usize = 128 * 1024;
const MAX_SCRIPTS: usize = 16;
const MAX_SCRIPT_REQUESTS: usize = 8;
const TOTAL_EXTERNAL_SCRIPT_BUDGET: usize = 512 * 1024;
const SCRIPT_LOAD_DEADLINE: Duration = Duration::from_secs(10);
const MAX_TOTAL_TEXT_BYTES: usize = 512 * 1024;
const JS_INSTRUCTION_BUDGET: usize = 50_000;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ScriptReport {
    pub executed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub mutations: usize,
}

fn collect_text(document: &Document, root: NodeId, budget: usize) -> String {
    let mut result = String::new();
    let mut stack = document
        .children(root)
        .iter()
        .rev()
        .copied()
        .collect::<Vec<_>>();
    let mut visited = 0;
    while let Some(node) = stack.pop() {
        visited += 1;
        if visited > MAX_NODES {
            break;
        }
        if let Some(NodeKind::Text(value)) = document.node(node).map(|node| &node.kind) {
            if result.len().saturating_add(value.len()) > budget {
                break;
            }
            result.push_str(value);
        } else if document
            .element(node)
            .is_some_and(|e| e.tag_name != "script" && e.tag_name != "style")
        {
            stack.extend(document.children(node).iter().rev().copied());
        }
    }
    result
}

#[derive(Debug)]
enum ScriptSource {
    Inline(String),
    External(String),
}

fn snapshot_and_scripts(
    document: &Document,
) -> (Vec<DomElementSnapshot>, Vec<ScriptSource>, usize) {
    let mut elements = Vec::new();
    let mut scripts = Vec::new();
    let mut skipped = 0;
    let mut stack = vec![document.root()];
    let mut visited = 0;
    let mut total_text = 0usize;
    while let Some(node) = stack.pop() {
        visited += 1;
        if visited > MAX_NODES {
            break;
        }
        if let Some(element) = document.element(node) {
            if element.tag_name == "script" {
                let src = element
                    .attributes
                    .iter()
                    .find(|a| a.name == "src")
                    .map(|a| a.value.as_str());
                let unsupported_timing = element
                    .attributes
                    .iter()
                    .any(|a| matches!(a.name.as_str(), "async" | "defer" | "integrity"));
                let type_attr = element
                    .attributes
                    .iter()
                    .find(|a| a.name == "type")
                    .map(|a| a.value.trim().to_ascii_lowercase());
                let classic = type_attr.as_deref().is_none_or(|value| {
                    matches!(value, "" | "text/javascript" | "application/javascript")
                });
                let code = collect_text(document, node, MAX_SCRIPT_BYTES + 1);
                if !classic || scripts.len() >= MAX_SCRIPTS || (src.is_some() && unsupported_timing)
                {
                    skipped += 1;
                } else if let Some(src) = src {
                    if src.trim().is_empty() {
                        skipped += 1;
                    } else {
                        scripts.push(ScriptSource::External(src.to_owned()));
                    }
                } else if code.is_empty() || code.len() > MAX_SCRIPT_BYTES {
                    skipped += 1;
                } else {
                    scripts.push(ScriptSource::Inline(code));
                }
            } else if elements.len() < MAX_ELEMENTS
                && let Some(id) = element
                    .attributes
                    .iter()
                    .find(|a| a.name == "id")
                    .map(|a| a.value.as_str())
                && !id.is_empty()
            {
                let text = collect_text(document, node, MAX_SCRIPT_BYTES);
                if total_text.saturating_add(text.len()) <= MAX_TOTAL_TEXT_BYTES {
                    total_text += text.len();
                    elements.push(DomElementSnapshot {
                        node: node.index(),
                        id: id.to_owned(),
                        text_content: text,
                    });
                }
            }
        }
        stack.extend(document.children(node).iter().rev().copied());
    }
    (elements, scripts, skipped)
}

pub fn execute_inline(document: &mut Document) -> ScriptReport {
    execute_for_page(document, None, None)
}

/// Resolve external classic scripts through the page's filtered network
/// context, preserving DOM script order. This first slice executes after
/// parsing rather than blocking the HTML tokenizer.
pub fn execute_for_page(
    document: &mut Document,
    network: Option<&NetworkContext>,
    page_base: Option<&str>,
) -> ScriptReport {
    execute_retained(document, network, page_base).0
}

/// Return a retained VM so callbacks registered during initial scripts
/// survive until native click events reach the page worker.
pub(crate) fn execute_retained(
    document: &mut Document,
    network: Option<&NetworkContext>,
    page_base: Option<&str>,
) -> (ScriptReport, Option<JsRuntime>) {
    let (elements, scripts, skipped) = snapshot_and_scripts(document);
    let mut report = ScriptReport {
        skipped,
        ..ScriptReport::default()
    };
    if scripts.is_empty() {
        return (report, None);
    }
    let mut runtime = JsRuntime::with_instruction_budget(JS_INSTRUCTION_BUDGET);
    if runtime.install_dom_snapshot(elements).is_err() {
        report.failed = scripts.len();
        return (report, None);
    }
    let started = Instant::now();
    let mut requests = 0usize;
    let mut remaining_bytes = TOTAL_EXTERNAL_SCRIPT_BUDGET;
    for source in scripts {
        let code = match source {
            ScriptSource::Inline(code) => Some(code),
            ScriptSource::External(src) => {
                if let (Some(network), Some(base)) = (network, page_base) {
                    if requests >= MAX_SCRIPT_REQUESTS
                        || remaining_bytes == 0
                        || started.elapsed() > SCRIPT_LOAD_DEADLINE
                    {
                        report.skipped += 1;
                        None
                    } else {
                        requests += 1;
                        match resolve_script_source(base, &src).and_then(|resolved| {
                            network.load_script_for_page(
                                &resolved,
                                base,
                                remaining_bytes.min(MAX_SCRIPT_BYTES),
                            )
                        }) {
                            Ok(code) => {
                                remaining_bytes = remaining_bytes.saturating_sub(code.len());
                                Some(code)
                            }
                            Err(_) => {
                                report.failed += 1;
                                None
                            }
                        }
                    }
                } else {
                    report.skipped += 1;
                    None
                }
            }
        };
        if let Some(code) = code {
            match runtime.eval_script(&code) {
                Ok(_) => report.executed += 1,
                Err(_) => report.failed += 1,
            }
        }
        // Apply completed mutations even when a later statement throws.
        // All DOM changes affect the retained tree before CSS/layout.
        for change in runtime.take_dom_mutations() {
            if let Some(node) = document.node_id(change.node)
                && document.set_text_content(node, &change.text_content)
            {
                report.mutations += 1;
            }
        }
    }
    (report, Some(runtime))
}

/// Dispatch one click only to a still-attached block DOM node.
pub(crate) fn dispatch_click(
    document: &mut Document,
    runtime: &mut JsRuntime,
    node: NodeId,
) -> (bool, usize) {
    if document.element(node).is_none() || !runtime.has_dom_click_listener(node.index()) {
        return (false, 0);
    }
    let handled = runtime.dispatch_dom_click(node.index()).unwrap_or(false);
    let mut changed = 0;
    for mutation in runtime.take_dom_mutations() {
        if let Some(node) = document.node_id(mutation.node)
            && document.set_text_content(node, &mutation.text_content)
        {
            changed += 1;
        }
    }
    (handled, changed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use op_html::parse_document;

    #[test]
    fn executes_script_and_replaces_text_without_reparsing_html() {
        let mut doc = parse_document(
            "<h1 id='title'>Before</h1><script>             document.getElementById('title').textContent = 'After';             </script>",
        );
        let report = execute_inline(&mut doc);
        assert_eq!(
            report,
            ScriptReport {
                executed: 1,
                failed: 0,
                skipped: 0,
                mutations: 1,
            }
        );
        let title = doc
            .children(doc.root())
            .iter()
            .copied()
            .flat_map(|id| doc.children(id).iter().copied())
            .find(|&id| doc.element(id).is_some_and(|e| e.tag_name == "body"));
        let _ = title; // parsed HTML remains a real document.
        let (elements, _, _) = snapshot_and_scripts(&doc);
        assert_eq!(
            elements
                .iter()
                .find(|e| e.id == "title")
                .unwrap()
                .text_content,
            "After"
        );
    }

    #[test]
    fn multiple_classic_scripts_share_vm_but_external_are_not_executed() {
        let mut doc = parse_document(
            "<p id='out'>initial</p><script>var phrase='Hello';</script>             <script src='/external.js'></script>             <script type='module'>throw 'module';</script>             <script>document.getElementById('out').textContent = phrase;</script>",
        );
        let report = execute_inline(&mut doc);
        assert_eq!(report.executed, 2);
        assert_eq!(report.skipped, 2);
        let (elements, _, _) = snapshot_and_scripts(&doc);
        assert_eq!(
            elements
                .iter()
                .find(|e| e.id == "out")
                .unwrap()
                .text_content,
            "Hello"
        );
    }

    #[test]
    fn unknown_id_does_not_silently_insert_new_nodes() {
        let mut doc = parse_document(
            "<p id='out'>unchanged</p><script>             document.getElementById('missing').textContent = 'bad';             </script>",
        );
        let report = execute_inline(&mut doc);
        assert_eq!(report.failed, 1);
        assert_eq!(report.mutations, 0);
        let (elements, _, _) = snapshot_and_scripts(&doc);
        assert_eq!(
            elements
                .iter()
                .find(|e| e.id == "out")
                .unwrap()
                .text_content,
            "unchanged"
        );
    }
}
