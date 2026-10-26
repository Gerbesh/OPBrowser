//! First page-scripting vertical slice: bounded, inline classic scripts and
//! detached DOM text mutations through OPBrowser's own JavaScript VM.
//! This is intentionally not a complete HTML script-processing model.
use op_dom::{Attribute, Document, NodeId, NodeKind};
use op_js::{DomElementSnapshot, DomOperation, JsError, JsRuntime};
use op_net::{NetworkContext, resolve_script_source};
use std::collections::HashMap;
use std::time::{Duration, Instant};

const MAX_NODES: usize = 20_000;
const MAX_ELEMENTS: usize = 4096;
const MAX_SCRIPT_BYTES: usize = 128 * 1024;
const MAX_SCRIPTS: usize = 16;
const MAX_SCRIPT_DIAGNOSTICS: usize = 24;
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

/// Refresh existing-node parentage/attributes before each script runs.
/// A bounded traversal ensures removeChild and insertBefore validate
/// real parser-built nodes as well as new nodes created in the VM.
fn sync_dom_tree(document: &Document, runtime: &mut JsRuntime) {
    runtime.set_dom_document_root(document.root().index());
    for child in document.children(document.root()) {
        if document
            .element(*child)
            .is_some_and(|element| element.tag_name == "html")
        {
            let _ = runtime.set_dom_document_element_node(child.index());
            break;
        }
    }
    let mut stack = vec![(document.root(), None)];
    let mut count = 0usize;
    while let Some((node, parent)) = stack.pop() {
        count += 1;
        if count > MAX_NODES {
            break;
        }
        runtime.sync_dom_children(
            node.index(),
            document
                .children(node)
                .iter()
                .map(|child| child.index())
                .collect(),
        );
        if let Some(op_dom::NodeKind::Text(text)) = document.node(node).map(|node| &node.kind) {
            runtime.sync_dom_text_node(
                node.index(),
                parent.map(|id: NodeId| id.index()),
                text.clone(),
            );
        }
        if let Some(element) = document.element(node) {
            runtime.sync_dom_tag(node.index(), element.tag_name.clone());
            runtime.sync_dom_namespace(node.index(), element.namespace_uri.clone());
            if element.tag_name == "head" {
                let _ = runtime.set_dom_head_node(node.index());
            }
        }
        if let Some(element) = document.element(node) {
            runtime.sync_dom_existing_node(
                node.index(),
                parent.map(|id: NodeId| id.index()),
                element
                    .attributes
                    .iter()
                    .map(|a| (a.name.clone(), a.value.clone()))
                    .collect(),
            );
        }
        stack.extend(
            document
                .children(node)
                .iter()
                .rev()
                .map(|id| (*id, Some(node))),
        );
    }
}

/// Commit ordered JS mutations to the authoritative DOM tree. Synthetic
/// handles are resolved through the page-owned VM, never exposed as raw
/// op_dom NodeIds. This routine is shared by parser scripts, clicks and timers.
pub(crate) fn apply_dom_operations(document: &mut Document, runtime: &mut JsRuntime) -> usize {
    let mut changed = 0;
    for op in runtime.take_dom_operations() {
        match op {
            DomOperation::CreateText { node, text } => {
                if document.allocation_headroom(MAX_NODES) > 0 {
                    let real = document.create_text(text);
                    runtime.bind_dom_node(node, real.index());
                }
            }
            DomOperation::CreateElement { node, tag } => {
                if document.allocation_headroom(MAX_NODES) == 0 {
                    continue;
                }
                let actual = document.create_element(tag);
                runtime.bind_dom_node(node, actual.index());
                // Detached creation by itself does not change visible layout.
            }
            DomOperation::CreateElementNS {
                node,
                qualified_name,
                namespace_uri,
            } => {
                if document.allocation_headroom(MAX_NODES) == 0 {
                    continue;
                }
                let actual = document.create_element_with_namespace(
                    qualified_name,
                    namespace_uri,
                    Vec::new(),
                );
                runtime.bind_dom_node(node, actual.index());
            }
            DomOperation::InsertBefore {
                parent,
                child,
                reference,
            } => {
                let parent = document.node_id(runtime.resolve_dom_node(parent));
                let child = document.node_id(runtime.resolve_dom_node(child));
                let reference =
                    reference.and_then(|node| document.node_id(runtime.resolve_dom_node(node)));
                if let (Some(parent), Some(child)) = (parent, child)
                    && document.element(parent).is_some()
                    && document.insert_before(parent, child, reference).is_ok()
                {
                    changed += 1;
                }
            }
            DomOperation::ReplaceChild {
                parent,
                new_child,
                old_child,
            } => {
                let parent = document.node_id(runtime.resolve_dom_node(parent));
                let new_child = document.node_id(runtime.resolve_dom_node(new_child));
                let old_child = document.node_id(runtime.resolve_dom_node(old_child));
                if let (Some(parent), Some(new_child), Some(old_child)) =
                    (parent, new_child, old_child)
                    && document.element(parent).is_some()
                    && document.replace_child(parent, new_child, old_child).is_ok()
                {
                    changed += 1;
                }
            }
            DomOperation::RemoveChild { parent, child } => {
                let parent = document.node_id(runtime.resolve_dom_node(parent));
                let child = document.node_id(runtime.resolve_dom_node(child));
                if let (Some(parent), Some(child)) = (parent, child)
                    && document.remove_child(parent, child).is_ok()
                {
                    changed += 1;
                }
            }
            DomOperation::SetAttribute { node, name, value } => {
                if let Some(node) = document.node_id(runtime.resolve_dom_node(node))
                    && let Some(element) = document.element_mut(node)
                {
                    if let Some(existing) = element.attributes.iter_mut().find(|a| a.name == name) {
                        existing.value = value;
                    } else {
                        element.attributes.push(Attribute { name, value });
                    }
                    changed += 1;
                }
            }
            DomOperation::RemoveAttribute { node, name } => {
                if let Some(node) = document.node_id(runtime.resolve_dom_node(node))
                    && let Some(element) = document.element_mut(node)
                {
                    let old = element.attributes.len();
                    element.attributes.retain(|a| a.name != name);
                    changed += usize::from(element.attributes.len() != old);
                }
            }
            DomOperation::AppendChild { parent, child } => {
                let parent = document.node_id(runtime.resolve_dom_node(parent));
                let child = document.node_id(runtime.resolve_dom_node(child));
                if let (Some(parent), Some(child)) = (parent, child)
                    && document.element(parent).is_some()
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
                    && (mutation.text_content.is_empty()
                        || document.element(node).is_none()
                        || document.allocation_headroom(MAX_NODES) > 0)
                {
                    let old_children = document.children(node).to_vec();
                    if document.set_text_content(node, &mutation.text_content)
                        || document.set_text_node_content(node, &mutation.text_content)
                    {
                        for child in old_children {
                            let _ = runtime.sync_dom_detached_node(child.index());
                        }
                        changed += 1;
                    }
                }
            }
        }
    }
    if changed > 0 {
        sync_dom_tree(document, runtime);
    }
    changed
}

/// A successful browser-owned checkpoint, excluding later page resource cleanup.
#[derive(Debug, Clone, Copy)]
pub(super) struct GcCheckpointPhases {
    pub started: Instant,
    pub js: Duration,
    pub native: Duration,
    pub metadata: Duration,
}

/// Native collection is prohibited while the parser owns open-element and
/// deferred-script handles. Other browser task boundaries can sweep after JS
/// roots validate and ordered host operations have been committed.
pub(super) fn collect_at_idle(
    document: &mut Document,
    runtime: &mut JsRuntime,
    force: bool,
) -> Option<GcCheckpointPhases> {
    let native_pressure = document.allocation_headroom(MAX_NODES) <= MAX_NODES / 8;
    let started = Instant::now();
    if !matches!(
        runtime.collect_if_needed_at_idle_with_host_pressure(force || native_pressure),
        Ok(Some(_))
    ) {
        return None;
    }
    let js = started.elapsed();
    let native_started = Instant::now();
    let roots = runtime.native_dom_roots();
    let freed = document.collect_garbage(&roots).ok()?;
    let native = native_started.elapsed();
    let metadata_started = Instant::now();
    let handles = freed.iter().map(|node| node.index()).collect::<Vec<_>>();
    runtime.forget_collected_native_nodes(&handles);
    Some(GcCheckpointPhases {
        started,
        js,
        native,
        metadata: metadata_started.elapsed(),
    })
}

/// Execute blocking scripts at tree-builder pauses. External async/defer
/// scripts fetch on bounded scoped workers; completions are handled at
/// subsequent parser script boundaries or after DOM construction.
/// Tokenization is eager; there is no interactive browser event loop yet.
pub(crate) fn parse_and_execute(
    html: &str,
    network: Option<&NetworkContext>,
    page_base: Option<&str>,
) -> (Document, ScriptReport, Option<JsRuntime>, Vec<String>) {
    let mut runner = ParserScriptRunner {
        runtime: None,
        report: ScriptReport::default(),
        diagnostics: Vec::new(),
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

    let mut document = std::thread::scope(|scope| {
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
    if let Some(runtime) = runner.runtime.as_mut() {
        collect_at_idle(&mut document, runtime, true);
    }
    (document, runner.report, runner.runtime, runner.diagnostics)
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

/// Explicit diagnostic opt-in only: never retain full real-world script sources.
/// Offsets are byte-indexed; the preview is clamped to valid UTF-8 boundaries.
fn bounded_js_error_context(source: &str, offset: usize) -> String {
    let mut offset = offset.min(source.len());
    while !source.is_char_boundary(offset) {
        offset -= 1;
    }
    let start = source[..offset]
        .char_indices()
        .rev()
        .nth(40)
        .map_or(0, |(index, _)| index);
    let end = source[offset..]
        .char_indices()
        .nth(48)
        .map_or(source.len(), |(index, _)| offset + index);
    let fragment = source[start..end]
        .chars()
        .map(|character| {
            if character.is_ascii_graphic() || character == ' ' {
                character
            } else {
                ' '
            }
        })
        .collect::<String>();
    format!(" [byte {offset}, near {fragment:?}]")
}

/// Candidate token locations only; without bytecode spans they are not exact call sites.
/// Opt-in, bounded and sanitized; no script body or literal values are retained.
fn bounded_reference_name_candidates(source: &str, name: &str) -> String {
    if name.len() > 48
        || !name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
    {
        return String::new();
    }
    let Ok(tokens) = op_js::tokenize(source) else {
        return String::new();
    };
    let mut total = 0_usize;
    let mut fragments = Vec::new();
    for token in tokens {
        if matches!(token.kind, op_js::TokenKind::Identifier(ref value) if value == name) {
            total += 1;
            if fragments.len() < 3 {
                fragments.push(bounded_js_error_context(source, token.start));
            }
        }
    }
    if total == 0 {
        return String::new();
    }
    format!(
        " [identifier {name:?}, {total} candidate tokens, not exact call site:{}]",
        fragments.join("")
    )
}

struct ParserScriptRunner<'a> {
    runtime: Option<JsRuntime>,
    report: ScriptReport,
    diagnostics: Vec<String>,
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
    fn record_error(&mut self, stage: &str, error: &JsError, source: &str) {
        if self.diagnostics.len() < MAX_SCRIPT_DIAGNOSTICS {
            let message = error
                .message
                .chars()
                .filter(|c| !c.is_control())
                .take(160)
                .collect::<String>();
            let context = if error.kind == op_js::JsErrorKind::Syntax
                && std::env::var_os("OPBROWSER_JS_ERROR_CONTEXT").as_deref()
                    == Some(std::ffi::OsStr::new("1"))
            {
                bounded_js_error_context(source, error.offset)
            } else if error.kind == op_js::JsErrorKind::Reference
                && std::env::var_os("OPBROWSER_JS_ERROR_CONTEXT").as_deref()
                    == Some(std::ffi::OsStr::new("1"))
            {
                error
                    .message
                    .strip_suffix(" is not defined")
                    .map_or_else(String::new, |name| {
                        bounded_reference_name_candidates(source, name)
                    })
            } else {
                String::new()
            };
            self.diagnostics
                .push(format!("{stage}:{:?}: {message}{context}", error.kind));
        }
    }

    fn record_failure(&mut self, stage: &str) {
        if self.diagnostics.len() < MAX_SCRIPT_DIAGNOSTICS {
            self.diagnostics.push(stage.to_owned());
        }
    }

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
        // Browser-owned idle checkpoint: callback results were discarded and
        // DOM mutations committed before reclamation.
        let _ = runtime.collect_if_needed_at_idle();
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
            self.record_failure(if !classic {
                "skip:unsupported_script_type"
            } else if self.accepted >= MAX_SCRIPTS {
                "skip:script_count_limit"
            } else {
                "skip:script_integrity_unsupported"
            });
            return None;
        }

        // On inline scripts async/defer attributes have no scheduling effect.
        if let Some(src) = src {
            if src.trim().is_empty() {
                self.report.skipped += 1;
                self.record_failure("skip:empty_script_src");
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
                self.record_failure("skip:empty_or_oversize_inline_script");
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
            self.record_failure("skip:no_external_script_context");
            return None;
        };
        if self.requests >= MAX_SCRIPT_REQUESTS
            || self.remaining_bytes == 0
            || self.started.elapsed() > SCRIPT_LOAD_DEADLINE
        {
            self.report.skipped += 1;
            self.record_failure("skip:external_script_budget_or_deadline");
            return None;
        }
        let url = match resolve_script_source(base, src) {
            Ok(url) => url,
            Err(_) => {
                self.report.failed += 1;
                self.record_failure("external_script_url_resolution_failed");
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
            Err(_) => {
                self.report.failed += 1;
                self.record_failure("external_script_load_failed");
            }
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
        if runtime.set_page_location(self.page_base).is_err() {
            self.report.failed += 1;
            return;
        }
        sync_dom_tree(document, runtime);
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
            Err(error) => {
                self.report.failed += 1;
                self.record_error("script", &error, code);
            }
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
    if runtime.set_page_location(page_base).is_err() {
        report.failed = scripts.len();
        return (report, None);
    }
    sync_dom_tree(document, &mut runtime);
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
        let _ = runtime.collect_if_needed_at_idle();
    }
    collect_at_idle(document, &mut runtime, true);
    (report, Some(runtime))
}

/// Dispatch native focus transitions on the existing generation-safe DOM.
pub(crate) fn dispatch_focus_change(
    document: &mut Document,
    runtime: &mut JsRuntime,
    previous: Option<NodeId>,
    next: Option<NodeId>,
) -> (usize, Option<GcCheckpointPhases>) {
    let path_for = |target: NodeId| -> Vec<usize> {
        let mut path = Vec::new();
        let mut current = Some(target);
        while let Some(node) = current {
            if path.len() >= 64 {
                break;
            }
            if document.element(node).is_some() {
                path.push(node.index());
            }
            current = document.node(node).and_then(|item| item.parent);
        }
        path
    };
    // The active element must be queryable from every focus/blur handler.
    // No focused element resolves to document.body, as in HTML documents.
    let _ = runtime.sync_native_active_element(next.map(|node| node.index()));
    if let Some(previous) = previous {
        let path = path_for(previous);
        if !path.is_empty() {
            let _ = runtime.dispatch_dom_focus_path(&path, "blur");
            let _ = runtime.dispatch_dom_focus_path(&path, "focusout");
        }
    }
    if let Some(next) = next {
        let path = path_for(next);
        if !path.is_empty() {
            let _ = runtime.dispatch_dom_focus_path(&path, "focus");
            let _ = runtime.dispatch_dom_focus_path(&path, "focusin");
        }
    }
    let changed = apply_dom_operations(document, runtime);
    let checkpoint = collect_at_idle(document, runtime, false);
    (changed, checkpoint)
}

/// Dispatch native keydown/keyup before the browser performs Tab/Enter
/// default actions. Script DOM operations commit even if the key is canceled.
pub(crate) fn dispatch_keyboard_event(
    document: &mut Document,
    runtime: &mut JsRuntime,
    target: NodeId,
    input: op_js::NativeKeyboardInput<'_>,
) -> (bool, usize, Option<GcCheckpointPhases>) {
    let mut path = Vec::new();
    let mut cursor = Some(target);
    while let Some(node) = cursor {
        if path.len() >= 64 {
            break;
        }
        if document.element(node).is_some() {
            path.push(node.index());
        }
        cursor = document.node(node).and_then(|item| item.parent);
    }
    let canceled = if path.is_empty() {
        false
    } else {
        runtime
            .dispatch_dom_keyboard_path(&path, input)
            .map_or(true, |result| result.default_prevented)
    };
    let changed = apply_dom_operations(document, runtime);
    let checkpoint = collect_at_idle(document, runtime, false);
    (canceled, changed, checkpoint)
}

/// The original DOM event system handles cancelable beforeinput and
/// bubbling input; host edits never eval synthetic JavaScript.
pub(crate) fn dispatch_text_input_event(
    document: &mut Document,
    runtime: &mut JsRuntime,
    target: NodeId,
    before: bool,
) -> (bool, usize, Option<GcCheckpointPhases>) {
    let mut path = Vec::new();
    let mut cursor = Some(target);
    while let Some(node) = cursor {
        if path.len() >= 64 {
            break;
        }
        if document.element(node).is_some() {
            path.push(node.index());
        }
        cursor = document.node(node).and_then(|node| node.parent);
    }
    let canceled = if path.is_empty() {
        false
    } else {
        runtime
            .dispatch_dom_text_input_path(&path, before)
            .map_or(true, |outcome| outcome.default_prevented)
    };
    let mutations = apply_dom_operations(document, runtime);
    let checkpoint = collect_at_idle(document, runtime, false);
    (canceled, mutations, checkpoint)
}

/// Dispatch the click to the hit element and its attached DOM ancestors.
pub(crate) fn dispatch_click(
    document: &mut Document,
    runtime: &mut JsRuntime,
    node: NodeId,
) -> (bool, bool, usize, Option<GcCheckpointPhases>) {
    if document.element(node).is_none() {
        return (false, false, 0, None);
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
    if !runtime.has_dom_click_path_listener(&path) {
        return (false, false, 0, None);
    }
    // Fail closed on VM dispatch errors: do not navigate after a failed
    // cancelable browser event, but still commit partial DOM mutations.
    let outcome = runtime
        .dispatch_dom_click_path_with_result(&path)
        .unwrap_or(op_js::DomClickDispatch {
            handled: false,
            default_prevented: true,
        });
    let changed = apply_dom_operations(document, runtime);
    let checkpoint = collect_at_idle(document, runtime, false);
    (
        outcome.handled,
        outcome.default_prevented,
        changed,
        checkpoint,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use op_html::parse_document;

    #[test]
    fn live_page_parser_passes_real_origin_to_script_runtime() {
        let html = "<div id='host'>Before</div><script>document.getElementById('host').textContent = location.hostname;</script>";
        let (document, report, runtime, diagnostics) =
            parse_and_execute(html, None, Some("https://pikabu.ru/feed?x=1"));
        assert_eq!(report.executed, 1, "{diagnostics:?}");
        assert_eq!(report.failed, 0, "{diagnostics:?}");
        let mut vm = runtime.expect("parser retains script VM");
        assert_eq!(
            vm.eval_script("document.URL").unwrap(),
            op_js::JsValue::String("https://pikabu.ru/feed?x=1".into())
        );
        assert!(collect_text(&document, document.root(), 4096).contains("pikabu.ru"));
    }

    #[test]
    fn g2b_page_parser_auto_collects_between_scripts_without_breaking_globals() {
        let html = "<div id='content'>Original</div>
          <script>
            for(var i=0;i<700;i=i+1){var first={index:i};}
            first=null;
            var keeper={answer:42};
          </script>
          <script>
            for(var j=0;j<700;j=j+1){var second={index:j};}
            second=null;
            document.getElementById('content').textContent='Updated';
          </script>";
        let (document, report, runtime, diagnostics) = parse_and_execute(html, None, None);
        assert!(diagnostics.is_empty());
        assert_eq!(report.executed, 2);
        assert_eq!(report.failed, 0);
        assert!(!document.is_empty());
        let mut vm = runtime.expect("scripted page retains a VM");
        assert!(
            vm.automatic_gc_runs() > 0,
            "browser scripts must hit real GC checkpoints"
        );
        assert!(
            // The XHR and window messaging facades add a fixed set of
            // intrinsic objects. The script must still recycle 1,400 temporaries.
            vm.heap_usage().object_slots < 1100,
            "second script must recycle temporary object slots; got {}",
            vm.heap_usage().object_slots
        );
        vm.eval_script("var retained=keeper.answer;").unwrap();
        assert_eq!(vm.global("retained"), Some(&op_js::JsValue::Number(42.0)));
    }

    #[test]
    fn regular_expression_and_compound_update_mutate_real_html_dom() {
        let source = r#"<p id='out'>Waiting</p><script>
            var query = 'https://www.youtube.com/a1';
            query = query.replace(/https:\/\/www\.youtube\.com\//g, '');
            var count = 0;
            for (const value of query.split(/\d+/)) {count += value.length;}
            document.getElementById('out').textContent = query + ':' + count;
        </script>"#;
        let (document, report, _, diagnostics) = parse_and_execute(source, None, None);
        assert_eq!(report.executed, 1, "{diagnostics:?}");
        assert_eq!(report.failed, 0, "{diagnostics:?}");
        assert!(report.mutations > 0);
        assert!(collect_text(&document, document.root(), 4096).contains("a1:1"));
    }

    #[test]
    fn real_youtube_for_in_configuration_copy_updates_dom_text() {
        let html = "<p id='out'>Waiting</p><script>
            var source = {quality:'high',autoplay:'off'};
            var copied = {}; var k;
            for (k in source) copied[k] = source[k];
            document.getElementById('out').textContent = copied.quality + '/' + copied.autoplay;
        </script>";
        let (document, report, _, diagnostics) = parse_and_execute(html, None, None);
        assert_eq!(report.executed, 1, "{diagnostics:?}");
        assert_eq!(report.failed, 0, "{diagnostics:?}");
        assert_eq!(report.mutations, 1);
        assert!(collect_text(&document, document.root(), 4096).contains("high/off"));
    }

    #[test]
    fn opt_in_syntax_excerpt_is_bounded_and_unicode_safe() {
        let source = "💡".repeat(100) + "\\nfunction broken( {";
        let rendered = bounded_js_error_context(&source, source.len() - 1);
        assert!(rendered.contains("byte "));
        assert!(!rendered.contains('💡'));
        assert!(rendered.len() < 260);
        assert!(!rendered.contains(char::from(10)));
        let at_middle_of_utf8 = bounded_js_error_context("a🐈z", 3);
        assert!(at_middle_of_utf8.contains("byte 1"));
    }

    #[test]
    fn opt_in_reference_candidates_are_bounded_and_exclude_string_literals() {
        let source = "var text='s s s'; window.s=1; s.type='lazy'; s.setAttribute('id','x');";
        let rendered = bounded_reference_name_candidates(source, "s");
        assert!(rendered.contains("3 candidate tokens"), "{rendered}");
        assert!(rendered.contains("not exact call site"));
        assert!(rendered.len() < 500);
        assert!(bounded_reference_name_candidates(source, "s;bad").is_empty());
    }

    #[test]
    fn for_of_iterates_real_dom_query_snapshot_and_updates_document() {
        let source = "<div id='result'>Before</div><span>One</span><span>Two</span>
            <script>
                var count = 0;
                for (const node of document.querySelectorAll('span')) { count = count + 1; }
                document.getElementById('result').textContent = count;
            </script>";
        let (document, report, _, diagnostics) = parse_and_execute(source, None, None);
        assert_eq!(report.executed, 1, "{diagnostics:?}");
        assert_eq!(report.failed, 0, "{diagnostics:?}");
        assert_eq!(report.mutations, 1);
        assert!(collect_text(&document, document.root(), 4096).contains('2'));
    }

    #[test]
    fn nullish_script_mutates_real_parser_dom_without_eager_fallback() {
        let source = "<h1 id='status'>Before</h1><script>
            var called = 0;
            function fallback() { called = called + 1; return 'fallback'; }
            document.getElementById('status').textContent = (0 ?? fallback()) + ':' + (null ?? 'ready');
        </script>";
        let (document, report, runtime, diagnostics) = parse_and_execute(source, None, None);
        assert_eq!(report.executed, 1, "{diagnostics:?}");
        assert_eq!(report.failed, 0, "{diagnostics:?}");
        assert_eq!(report.mutations, 1);
        assert!(collect_text(&document, document.root(), 4096).contains("0:ready"));
        let mut vm = runtime.expect("script keeps its VM");
        assert_eq!(
            vm.eval_script("called").unwrap(),
            op_js::JsValue::Number(0.0)
        );
    }

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
