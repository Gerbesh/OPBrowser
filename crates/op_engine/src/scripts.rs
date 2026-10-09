//! First page-scripting vertical slice: bounded, inline classic scripts and
//! detached DOM text mutations through OPBrowser's own JavaScript VM.
//! This is intentionally not a complete HTML script-processing model.
use op_dom::{Attribute, Document, NodeId, NodeKind};
use op_js::{DomElementSnapshot, DomOperation, JsRuntime};
use op_net::{NetworkContext, resolve_script_source};
use std::collections::HashMap;
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

// Retained for isolated legacy post-parse compatibility tests.
#[cfg_attr(not(test), allow(dead_code))]
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

fn find_body(document: &Document) -> Option<NodeId> {
    let mut stack = vec![document.root()];
    let mut count = 0usize;
    while let Some(node) = stack.pop() {
        count += 1;
        if count > MAX_NODES {
            return None;
        }
        if document
            .element(node)
            .is_some_and(|element| element.tag_name == "body")
        {
            return Some(node);
        }
        stack.extend(document.children(node).iter().rev().copied());
    }
    None
}

/// Commit ordered JS mutations to the authoritative DOM tree. Synthetic
/// handles are resolved through the page-owned VM, never exposed as raw
/// op_dom NodeIds. This routine is shared by parser scripts, clicks and timers.
pub(crate) fn apply_dom_operations(document: &mut Document, runtime: &mut JsRuntime) -> usize {
    let mut changed = 0;
    for op in runtime.take_dom_operations() {
        match op {
            DomOperation::CreateElement { node, tag } => {
                if document.len() >= MAX_NODES {
                    continue;
                }
                let actual = document.create_element(tag);
                runtime.bind_dom_node(node, actual.index());
                // Detached creation by itself does not change visible layout.
            }
            DomOperation::AppendChild { parent, child } => {
                let parent = document.node_id(runtime.resolve_dom_node(parent));
                let child = document.node_id(runtime.resolve_dom_node(child));
                if let (Some(parent), Some(child)) = (parent, child)
                    && document.element(parent).is_some()
                    && document.element(child).is_some()
                    && document.append_child(parent, child).is_ok()
                {
                    changed += 1;
                }
            }
            DomOperation::SetId { node, id } => {
                if let Some(node) = document.node_id(runtime.resolve_dom_node(node))
                    && let Some(element) = document.element_mut(node)
                {
                    element.attributes.retain(|a| a.name != "id");
                    if !id.is_empty() {
                        element.attributes.push(Attribute {
                            name: "id".into(),
                            value: id,
                        });
                    }
                    changed += 1;
                }
            }
            DomOperation::SetText(mutation) => {
                if let Some(node) = document.node_id(runtime.resolve_dom_node(mutation.node))
                    && document.set_text_content(node, &mutation.text_content)
                {
                    changed += 1;
                }
            }
        }
    }
    changed
}

/// Execute blocking scripts at tree-builder pauses. External async/defer
/// scripts fetch on bounded scoped workers; completions are handled at
/// subsequent parser script boundaries or after DOM construction.
/// Tokenization is eager; there is no interactive browser event loop yet.
pub(crate) fn parse_and_execute(
    html: &str,
    network: Option<&NetworkContext>,
    page_base: Option<&str>,
) -> (Document, ScriptReport, Option<JsRuntime>) {
    let mut runner = ParserScriptRunner {
        runtime: None,
        report: ScriptReport::default(),
        accepted: 0,
        requests: 0,
        remaining_bytes: TOTAL_EXTERNAL_SCRIPT_BUDGET,
        started: Instant::now(),
        network,
        page_base,
        next_request: 0,
        deferred_order: Vec::new(),
        deferred_ready: HashMap::new(),
        async_pending: 0,
        ready_state: "loading",
    };

    let document = std::thread::scope(|scope| {
        let (sender, receiver) = std::sync::mpsc::channel::<CompletedScript>();
        let mut document = op_html::parse_document_with_script_hook(html, |document, script| {
            runner.drain_ready(document, &receiver);
            if let Some(request) = runner.execute(document, script) {
                let network = network.expect("scheduled script requires network context");
                let base = page_base
                    .expect("scheduled script requires page base")
                    .to_owned();
                let send = sender.clone();
                scope.spawn(move || {
                    let result =
                        network.load_script_for_page(&request.url, &base, request.reserved_bytes);
                    let _ = send.send(CompletedScript {
                        kind: request.kind,
                        index: request.index,
                        reserved_bytes: request.reserved_bytes,
                        result,
                    });
                });
            }
            runner.drain_ready(document, &receiver);
        });
        runner.drain_ready(&mut document, &receiver);
        runner.advance_state(&mut document, "interactive");

        // Deferred classic scripts run after DOM parsing, in document order.
        // Async completions arriving while we wait can still run immediately.
        for index in std::mem::take(&mut runner.deferred_order) {
            while !runner.deferred_ready.contains_key(&index) {
                match receiver.recv() {
                    Ok(completion) => runner.complete(&mut document, completion),
                    Err(_) => {
                        runner.report.failed += 1;
                        break;
                    }
                }
            }
            if let Some(result) = runner.deferred_ready.remove(&index) {
                runner.evaluate_loaded(&mut document, result);
            }
            runner.drain_ready(&mut document, &receiver);
        }

        runner.dispatch_lifecycle(&mut document, "DOMContentLoaded");

        // Initial navigation owns the page VM; do not let background workers
        // outlive that page or mutate it on another thread.
        while runner.async_pending > 0 {
            match receiver.recv() {
                Ok(completion) => runner.complete(&mut document, completion),
                Err(_) => {
                    runner.report.failed += runner.async_pending;
                    runner.async_pending = 0;
                }
            }
        }

        runner.advance_state(&mut document, "complete");
        runner.dispatch_lifecycle(&mut document, "load");

        // Elements inserted after the final script must be visible to its
        // retained event handlers.
        if let Some(runtime) = runner.runtime.as_mut() {
            let (elements, _, _) = snapshot_and_scripts(&document);
            runtime.refresh_dom_snapshot(elements);
            if let Some(body) = find_body(&document) {
                let _ = runtime.set_dom_body_node(body.index());
            }
        }
        document
    });
    (document, runner.report, runner.runtime)
}

#[derive(Debug, Clone, Copy)]
enum ScriptTiming {
    Blocking,
    Async,
    Defer,
}

struct ScriptFetch {
    kind: ScriptTiming,
    index: usize,
    url: String,
    reserved_bytes: usize,
}

struct CompletedScript {
    kind: ScriptTiming,
    index: usize,
    reserved_bytes: usize,
    result: Result<String, op_net::LoadError>,
}

struct ParserScriptRunner<'a> {
    runtime: Option<JsRuntime>,
    report: ScriptReport,
    accepted: usize,
    requests: usize,
    remaining_bytes: usize,
    started: Instant,
    network: Option<&'a NetworkContext>,
    page_base: Option<&'a str>,
    next_request: usize,
    deferred_order: Vec<usize>,
    deferred_ready: HashMap<usize, Result<String, op_net::LoadError>>,
    async_pending: usize,
    ready_state: &'static str,
}

impl ParserScriptRunner<'_> {
    fn advance_state(&mut self, document: &mut Document, state: &'static str) {
        self.ready_state = state;
        if let Some(runtime) = self.runtime.as_mut() {
            let (elements, _, _) = snapshot_and_scripts(document);
            runtime.refresh_dom_snapshot(elements);
            if let Some(body) = find_body(document)
                && runtime.set_dom_body_node(body.index()).is_err()
            {
                self.report.failed += 1;
            }
            if runtime.set_document_ready_state(state).is_err() {
                self.report.failed += 1;
            }
        }
        self.dispatch_lifecycle(document, "readystatechange");
    }

    fn dispatch_lifecycle(&mut self, document: &mut Document, event: &str) {
        if let Some(runtime) = self.runtime.as_mut()
            && runtime.dispatch_lifecycle_event(event).is_err()
        {
            self.report.failed += 1;
        }
        self.apply_mutations(document);
    }

    fn apply_mutations(&mut self, document: &mut Document) {
        let Some(runtime) = self.runtime.as_mut() else {
            return;
        };
        self.report.mutations += apply_dom_operations(document, runtime);
    }
    fn execute(&mut self, document: &mut Document, script: NodeId) -> Option<ScriptFetch> {
        let element = document.element(script)?;
        let src = element
            .attributes
            .iter()
            .find(|a| a.name == "src")
            .map(|a| a.value.as_str());
        let has_integrity = element.attributes.iter().any(|a| a.name == "integrity");
        let type_attr = element
            .attributes
            .iter()
            .find(|a| a.name == "type")
            .map(|a| a.value.trim().to_ascii_lowercase());
        let classic = type_attr
            .as_deref()
            .is_none_or(|value| matches!(value, "" | "text/javascript" | "application/javascript"));
        if !classic || self.accepted >= MAX_SCRIPTS || (src.is_some() && has_integrity) {
            self.report.skipped += 1;
            return None;
        }

        // On inline scripts async/defer attributes have no scheduling effect.
        if let Some(src) = src {
            if src.trim().is_empty() {
                self.report.skipped += 1;
                return None;
            }
            self.accepted += 1;
            let timing = if element.attributes.iter().any(|a| a.name == "async") {
                ScriptTiming::Async
            } else if element.attributes.iter().any(|a| a.name == "defer") {
                ScriptTiming::Defer
            } else {
                ScriptTiming::Blocking
            };
            let request = self.prepare_fetch(src, timing)?;
            if matches!(timing, ScriptTiming::Blocking) {
                let result = self
                    .network
                    .expect("validated network")
                    .load_script_for_page(
                        &request.url,
                        self.page_base.expect("validated page base"),
                        request.reserved_bytes,
                    );
                self.complete(
                    document,
                    CompletedScript {
                        kind: timing,
                        index: request.index,
                        reserved_bytes: request.reserved_bytes,
                        result,
                    },
                );
                None
            } else {
                match timing {
                    ScriptTiming::Defer => self.deferred_order.push(request.index),
                    ScriptTiming::Async => self.async_pending += 1,
                    ScriptTiming::Blocking => unreachable!(),
                }
                Some(request)
            }
        } else {
            let code = collect_text(document, script, MAX_SCRIPT_BYTES + 1);
            if code.is_empty() || code.len() > MAX_SCRIPT_BYTES {
                self.report.skipped += 1;
                return None;
            }
            self.accepted += 1;
            self.evaluate_code(document, &code);
            None
        }
    }

    fn prepare_fetch(&mut self, src: &str, kind: ScriptTiming) -> Option<ScriptFetch> {
        let (Some(_network), Some(base)) = (self.network, self.page_base) else {
            self.report.skipped += 1;
            return None;
        };
        if self.requests >= MAX_SCRIPT_REQUESTS
            || self.remaining_bytes == 0
            || self.started.elapsed() > SCRIPT_LOAD_DEADLINE
        {
            self.report.skipped += 1;
            return None;
        }
        let url = match resolve_script_source(base, src) {
            Ok(url) => url,
            Err(_) => {
                self.report.failed += 1;
                return None;
            }
        };
        let reserved_bytes = self.remaining_bytes.min(MAX_SCRIPT_BYTES);
        self.remaining_bytes -= reserved_bytes;
        self.requests += 1;
        let index = self.next_request;
        self.next_request += 1;
        Some(ScriptFetch {
            kind,
            index,
            url,
            reserved_bytes,
        })
    }

    fn drain_ready(
        &mut self,
        document: &mut Document,
        receiver: &std::sync::mpsc::Receiver<CompletedScript>,
    ) {
        while let Ok(completion) = receiver.try_recv() {
            self.complete(document, completion);
        }
    }

    fn complete(&mut self, document: &mut Document, completed: CompletedScript) {
        let used = completed.result.as_ref().map_or(0, String::len);
        self.remaining_bytes = self
            .remaining_bytes
            .saturating_add(completed.reserved_bytes.saturating_sub(used))
            .min(TOTAL_EXTERNAL_SCRIPT_BUDGET);
        match completed.kind {
            ScriptTiming::Defer => {
                self.deferred_ready
                    .insert(completed.index, completed.result);
            }
            ScriptTiming::Async => {
                self.async_pending = self.async_pending.saturating_sub(1);
                self.evaluate_loaded(document, completed.result);
            }
            ScriptTiming::Blocking => self.evaluate_loaded(document, completed.result),
        }
    }

    fn evaluate_loaded(
        &mut self,
        document: &mut Document,
        result: Result<String, op_net::LoadError>,
    ) {
        match result {
            Ok(code) => self.evaluate_code(document, &code),
            Err(_) => self.report.failed += 1,
        }
    }

    fn evaluate_code(&mut self, document: &mut Document, code: &str) {
        let (elements, _, _) = snapshot_and_scripts(document);
        if let Some(runtime) = self.runtime.as_mut() {
            runtime.refresh_dom_snapshot(elements);
        } else {
            let mut runtime = JsRuntime::with_instruction_budget(JS_INSTRUCTION_BUDGET);
            if runtime.install_dom_snapshot(elements).is_err() {
                self.report.failed += 1;
                return;
            }
            self.runtime = Some(runtime);
        }
        let runtime = self.runtime.as_mut().expect("runtime just installed");
        if let Some(body) = find_body(document)
            && runtime.set_dom_body_node(body.index()).is_err()
        {
            self.report.failed += 1;
            return;
        }
        if runtime.set_document_ready_state(self.ready_state).is_err() {
            self.report.failed += 1;
            return;
        }
        match runtime.eval_script(code) {
            Ok(_) => self.report.executed += 1,
            Err(_) => self.report.failed += 1,
        }
        self.apply_mutations(document);
    }
}
#[cfg(test)]
pub fn execute_inline(document: &mut Document) -> ScriptReport {
    execute_for_page(document, None, None)
}

/// Resolve external classic scripts through the page's filtered network
/// context, preserving DOM script order. This first slice executes after
/// parsing rather than blocking the HTML tokenizer.
#[cfg(test)]
pub fn execute_for_page(
    document: &mut Document,
    network: Option<&NetworkContext>,
    page_base: Option<&str>,
) -> ScriptReport {
    execute_retained(document, network, page_base).0
}

/// Return a retained VM so callbacks registered during initial scripts
/// survive until native click events reach the page worker.
#[cfg(test)]
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
    if let Some(body) = find_body(document) {
        let _ = runtime.set_dom_body_node(body.index());
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
        report.mutations += apply_dom_operations(document, &mut runtime);
    }
    (report, Some(runtime))
}

/// Dispatch the click to the hit element and its attached DOM ancestors.
pub(crate) fn dispatch_click(
    document: &mut Document,
    runtime: &mut JsRuntime,
    node: NodeId,
) -> (bool, usize) {
    if document.element(node).is_none() {
        return (false, 0);
    }
    let mut path = Vec::new();
    let mut current = Some(node);
    while let Some(id) = current {
        if path.len() >= 64 {
            break;
        }
        if document.element(id).is_some() {
            path.push(id.index());
        }
        current = document.node(id).and_then(|n| n.parent);
    }
    if !path.iter().any(|&id| runtime.has_dom_click_listener(id)) {
        return (false, 0);
    }
    let handled = runtime.dispatch_dom_click_path(&path).unwrap_or(false);
    let changed = apply_dom_operations(document, runtime);
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
