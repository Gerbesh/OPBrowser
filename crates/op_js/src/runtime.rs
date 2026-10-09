use crate::{
    BinaryOp, CompiledScript, JsError, JsErrorKind, JsValue, ObjectId, UnaryOp, UpdateOp,
    VariableKind,
    bytecode::{FunctionTemplate, Instruction, TryTemplate},
    compile_script,
};
use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
    time::{Duration, Instant},
};

const DEFAULT_INSTRUCTION_BUDGET: usize = 1_000_000;
const DEFAULT_OBJECT_BUDGET: usize = 100_000;
const DEFAULT_ENVIRONMENT_BUDGET: usize = 100_000;
const DEFAULT_CALL_DEPTH_BUDGET: usize = 64;
const MAX_PENDING_TIMERS: usize = 64;
const MAX_TOTAL_TIMERS: u32 = 512;
const MAX_TIMER_DELAY_MS: u64 = 60_000;
const MIN_INTERVAL_DELAY_MS: u64 = 4;
const MAX_PENDING_MICROTASKS: usize = 256;
const MAX_TOTAL_MICROTASKS: u32 = 1024;
const MAX_PENDING_TEXT_REQUESTS: usize = 8;
const MAX_TOTAL_TEXT_REQUESTS: u32 = 32;
const MAX_HTTP_HEADERS: usize = 32;
const MAX_HTTP_HEADER_BYTES: usize = 8192;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct EnvironmentId(usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EnvironmentKind {
    Global,
    Function,
    Block,
}

#[derive(Debug, Clone)]
struct Binding {
    value: JsValue,
    mutable: bool,
    declaration_kind: VariableKind,
}

#[derive(Debug, Clone)]
struct Environment {
    parent: Option<EnvironmentId>,
    kind: EnvironmentKind,
    bindings: HashMap<String, Binding>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ObjectKind {
    Ordinary,
    Array,
    Function,
    DomElement(usize),
    DomText(usize),
    DomClassList(usize),
    DomNodeList(usize),
    DomHtmlCollection(usize),
    DomTagCollection,
    DomQueryNodeList,
    DomEvent,
    DomStyle(usize),
    Headers,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BuiltinFunction {
    Error,
    TypeError,
    ReferenceError,
    SyntaxError,
    StringCharAt,
    ArrayPush,
    ArrayPop,
    DomGetElementById,
    DomGetElementsByTagName,
    DomQuerySelector,
    DomQuerySelectorAll,
    DomMatches,
    DomClosest,
    DomHasAttribute,
    DomHasAttributes,
    DomCreateElement,
    DomCreateTextNode,
    DomAppendChild,
    DomInsertBefore,
    DomRemoveChild,
    DomReplaceChild,
    DomRemoveSelf,
    DomClassAdd,
    DomClassRemove,
    DomClassContains,
    DomClassToggle,
    DomNodeListItem,
    DomHtmlCollectionNamedItem,
    DomContains,
    DomClick,
    DomDispatchEvent,
    DomEventConstructor,
    DomCreateEvent,
    DomInitEvent,
    DomStyleSetProperty,
    DomStyleGetPropertyValue,
    DomStyleRemoveProperty,
    DomSetAttribute,
    DomGetAttribute,
    DomRemoveAttribute,
    DomAddEventListener,
    DomRemoveEventListener,
    EventStopPropagation,
    EventPreventDefault,
    LifecycleAddEventListener,
    LifecycleRemoveEventListener,
    SetTimeout,
    ClearTimeout,
    SetInterval,
    ClearInterval,
    QueueMicrotask,
    OpFetchText,
    HeadersConstructor,
    HeadersGet,
    HeadersHas,
    HeadersSet,
    HeadersAppend,
    HeadersDelete,
    JsonParse,
    JsonStringify,
    ObjectConstructor,
    ArrayConstructor,
    BooleanConstructor,
    NumberConstructor,
    StringConstructor,
    IsNaN,
    IsFinite,
    ArrayIsArray,
    ArrayOf,
    NumberIsNaN,
    NumberIsFinite,
    ObjectIs,
    ObjectPrototypeValueOf,
    ObjectPrototypeToString,
    BoxedPrimitiveValueOf,
    BoxedPrimitiveToString,
}

#[derive(Debug, Clone)]
enum FunctionImplementation {
    User {
        template: Arc<FunctionTemplate>,
        closure: EnvironmentId,
    },
    Builtin(BuiltinFunction),
}

#[derive(Debug, Clone)]
struct FunctionObject {
    implementation: FunctionImplementation,
}

#[derive(Debug, Clone)]
struct JsObject {
    properties: HashMap<String, JsValue>,
    prototype: Option<ObjectId>,
    kind: ObjectKind,
    function: Option<FunctionObject>,
}

#[derive(Debug, Clone, PartialEq)]
enum RunOutcome {
    Complete(JsValue),
    Returned(JsValue),
    Thrown(JsValue),
    Break,
    Continue,
}

enum CallOutcome {
    Value(JsValue),
    Thrown(JsValue),
}

/// A bounded, detached DOM view supplied by OPBrowser's page engine.
#[derive(Debug, Clone)]
pub struct DomElementSnapshot {
    pub node: usize,
    pub id: String,
    pub text_content: String,
}

#[derive(Debug, Clone)]
struct PendingTimer {
    id: u32,
    due: Instant,
    callback: JsValue,
    arguments: Vec<JsValue>,
    interval: Option<Duration>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TimerReport {
    pub fired: usize,
    pub failed: usize,
}

/// A bounded resource request detached from the live JS heap. The page
/// engine uses its own page identity to discard stale completions.
#[derive(Debug, Clone)]
pub struct TextRequest {
    pub id: u32,
    pub url: String,
    /// True for standard fetch, false for legacy callback status errors.
    pub include_http_errors: bool,
    pub request_headers: Vec<(String, String)>,
    pub reject_redirect: bool,
}

/// Detached text-response metadata delivered on the original page worker.
#[derive(Debug, Clone)]
pub struct TextResponse {
    pub text: String,
    pub address: String,
    pub status: u32,
    pub status_text: String,
    pub headers: Vec<(String, String)>,
    pub redirected: bool,
}

/// A host mutation applied by the browser only after VM execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DomTextMutation {
    pub node: usize,
    pub text_content: String,
}

/// DOM operations are detached from the authoritative op_dom Document and
/// replayed by the page engine in script order. Synthetic IDs are bounded,
/// and never treated as op_dom NodeIds until the host resolves them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomOperation {
    CreateElement {
        node: usize,
        tag: String,
    },
    CreateText {
        node: usize,
        text: String,
    },
    AppendChild {
        parent: usize,
        child: usize,
    },
    InsertBefore {
        parent: usize,
        child: usize,
        reference: Option<usize>,
    },
    RemoveChild {
        parent: usize,
        child: usize,
    },
    ReplaceChild {
        parent: usize,
        new_child: usize,
        old_child: usize,
    },
    SetAttribute {
        node: usize,
        name: String,
        value: String,
    },
    RemoveAttribute {
        node: usize,
        name: String,
    },
    SetId {
        node: usize,
        id: String,
    },
    SetText(DomTextMutation),
}

#[derive(Debug)]
pub struct JsRuntime {
    heap: Vec<JsObject>,
    environments: Vec<Environment>,
    global_env: EnvironmentId,
    object_prototype: ObjectId,
    array_prototype: ObjectId,
    error_prototype: ObjectId,
    type_error_prototype: ObjectId,
    reference_error_prototype: ObjectId,
    syntax_error_prototype: ObjectId,
    global_object: ObjectId,
    instruction_budget: usize,
    object_budget: usize,
    environment_budget: usize,
    call_depth_budget: usize,
    dom_ids: HashMap<String, usize>,
    dom_text: HashMap<usize, String>,
    dom_mutations: Vec<DomTextMutation>,
    dom_operations: Vec<DomOperation>,
    dom_node_objects: HashMap<usize, ObjectId>,
    dom_node_aliases: HashMap<usize, usize>,
    dom_pending_ids: HashMap<usize, String>,
    dom_pending_parents: HashMap<usize, usize>,
    dom_attributes: HashMap<usize, HashMap<String, String>>,
    dom_class_lists: HashMap<usize, ObjectId>,
    dom_node_lists: HashMap<usize, ObjectId>,
    dom_html_collections: HashMap<usize, ObjectId>,
    dom_tag_collections: HashMap<ObjectId, (usize, String)>,
    dom_query_lists: HashMap<ObjectId, Vec<usize>>,
    dom_styles: HashMap<usize, ObjectId>,
    dom_children: HashMap<usize, Vec<usize>>,
    dom_text_nodes: std::collections::HashSet<usize>,
    dom_tags: HashMap<usize, String>,
    dom_attached: std::collections::HashSet<usize>,
    next_virtual_dom_node: usize,
    dom_click_listeners: HashMap<usize, Vec<JsValue>>,
    dom_click_capture_listeners: HashMap<usize, Vec<JsValue>>,
    dom_onclick: HashMap<usize, JsValue>,
    dom_typed_listeners: HashMap<(usize, String, bool), Vec<JsValue>>,
    dom_dispatch_depth: usize,
    dom_programmatic_click_depth: usize,
    dom_document: Option<ObjectId>,
    lifecycle_listeners: HashMap<(ObjectId, String, bool), Vec<JsValue>>,
    timers: Vec<PendingTimer>,
    next_timer_id: u32,
    microtasks: VecDeque<JsValue>,
    microtasks_scheduled: u32,
    text_requests: VecDeque<TextRequest>,
    text_callbacks: HashMap<u32, JsValue>,
    header_values: HashMap<ObjectId, HashMap<String, String>>,
    boxed_values: HashMap<ObjectId, JsValue>,
    next_text_request_id: u32,
}

impl Default for JsRuntime {
    fn default() -> Self {
        let object_prototype = ObjectId(0);
        let array_prototype = ObjectId(1);
        let error_prototype = ObjectId(2);
        let type_error_prototype = ObjectId(3);
        let reference_error_prototype = ObjectId(4);
        let global_object = ObjectId(5);
        let syntax_error_prototype = ObjectId(6);

        let ordinary = |prototype, properties| JsObject {
            properties,
            prototype,
            kind: ObjectKind::Ordinary,
            function: None,
        };

        let mut error_properties = HashMap::new();
        error_properties.insert("name".into(), JsValue::String("Error".into()));
        error_properties.insert("message".into(), JsValue::String(String::new()));

        let mut type_error_properties = HashMap::new();
        type_error_properties.insert("name".into(), JsValue::String("TypeError".into()));
        type_error_properties.insert("message".into(), JsValue::String(String::new()));

        let mut reference_error_properties = HashMap::new();
        reference_error_properties.insert("name".into(), JsValue::String("ReferenceError".into()));
        reference_error_properties.insert("message".into(), JsValue::String(String::new()));

        let heap = vec![
            ordinary(None, HashMap::new()),
            ordinary(Some(object_prototype), HashMap::new()),
            ordinary(Some(object_prototype), error_properties),
            ordinary(Some(error_prototype), type_error_properties),
            ordinary(Some(error_prototype), reference_error_properties),
            ordinary(Some(object_prototype), HashMap::new()),
            ordinary(
                Some(error_prototype),
                HashMap::from([
                    ("name".into(), JsValue::String("SyntaxError".into())),
                    ("message".into(), JsValue::String(String::new())),
                ]),
            ),
        ];

        let global_env = EnvironmentId(0);
        let environments = vec![Environment {
            parent: None,
            kind: EnvironmentKind::Global,
            bindings: HashMap::new(),
        }];

        let mut runtime = Self {
            heap,
            environments,
            global_env,
            object_prototype,
            array_prototype,
            error_prototype,
            type_error_prototype,
            reference_error_prototype,
            syntax_error_prototype,
            global_object,
            instruction_budget: DEFAULT_INSTRUCTION_BUDGET,
            object_budget: DEFAULT_OBJECT_BUDGET,
            environment_budget: DEFAULT_ENVIRONMENT_BUDGET,
            call_depth_budget: DEFAULT_CALL_DEPTH_BUDGET,
            dom_ids: HashMap::new(),
            dom_text: HashMap::new(),
            dom_mutations: Vec::new(),
            dom_operations: Vec::new(),
            dom_node_objects: HashMap::new(),
            dom_node_aliases: HashMap::new(),
            dom_pending_ids: HashMap::new(),
            dom_pending_parents: HashMap::new(),
            dom_attributes: HashMap::new(),
            dom_class_lists: HashMap::new(),
            dom_node_lists: HashMap::new(),
            dom_html_collections: HashMap::new(),
            dom_tag_collections: HashMap::new(),
            dom_query_lists: HashMap::new(),
            dom_styles: HashMap::new(),
            dom_children: HashMap::new(),
            dom_text_nodes: std::collections::HashSet::new(),
            dom_tags: HashMap::new(),
            dom_attached: std::collections::HashSet::new(),
            next_virtual_dom_node: 1,
            dom_click_listeners: HashMap::new(),
            dom_click_capture_listeners: HashMap::new(),
            dom_onclick: HashMap::new(),
            dom_typed_listeners: HashMap::new(),
            dom_dispatch_depth: 0,
            dom_programmatic_click_depth: 0,
            dom_document: None,
            lifecycle_listeners: HashMap::new(),
            timers: Vec::new(),
            next_timer_id: 1,
            microtasks: VecDeque::new(),
            microtasks_scheduled: 0,
            text_requests: VecDeque::new(),
            text_callbacks: HashMap::new(),
            header_values: HashMap::new(),
            boxed_values: HashMap::new(),
            next_text_request_id: 1,
        };

        runtime.install_global_binding(
            "this",
            JsValue::Object(global_object),
            false,
            VariableKind::Const,
        );
        runtime
            .install_error_constructor("Error", BuiltinFunction::Error, error_prototype)
            .expect("built-in Error constructor must fit initial runtime budgets");
        runtime
            .install_error_constructor(
                "TypeError",
                BuiltinFunction::TypeError,
                type_error_prototype,
            )
            .expect("built-in TypeError constructor must fit initial runtime budgets");
        let event_ctor = runtime
            .allocate_lifecycle_method("Event", BuiltinFunction::DomEventConstructor)
            .expect("Event constructor fits VM initial budget");
        runtime.install_global_binding(
            "Event",
            JsValue::Object(event_ctor),
            false,
            VariableKind::Const,
        );
        runtime
            .install_error_constructor(
                "ReferenceError",
                BuiltinFunction::ReferenceError,
                reference_error_prototype,
            )
            .expect("built-in ReferenceError constructor must fit initial runtime budgets");
        runtime
            .install_error_constructor(
                "SyntaxError",
                BuiltinFunction::SyntaxError,
                syntax_error_prototype,
            )
            .expect("built-in SyntaxError constructor must fit VM budgets");
        runtime
            .install_standard_primitives()
            .expect("standard primitive builtins must fit initial VM budgets");
        // Promise reactions use the original FIFO microtask queue.
        let microtask = runtime
            .allocate_lifecycle_method("queueMicrotask", BuiltinFunction::QueueMicrotask)
            .expect("built-in microtask function must fit runtime budgets");
        runtime.install_global_binding(
            "queueMicrotask",
            JsValue::Object(microtask),
            false,
            VariableKind::Const,
        );
        runtime
            .install_json_methods()
            .expect("built-in JSON methods must fit runtime budgets");
        runtime
            .eval_script(include_str!("async_promise.js"))
            .expect("bundled Promise bootstrap must compile and run");

        runtime
    }
}

impl JsRuntime {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_instruction_budget(instruction_budget: usize) -> Self {
        Self {
            instruction_budget,
            ..Self::default()
        }
    }

    /// Install a deliberately minimal document host object. DOM identities are
    /// detached snapshots, never borrowed pointers into the live tree.
    pub fn install_dom_snapshot(
        &mut self,
        elements: impl IntoIterator<Item = DomElementSnapshot>,
    ) -> Result<(), JsError> {
        self.dom_ids.clear();
        self.dom_text.clear();
        self.dom_mutations.clear();
        self.dom_operations.clear();
        self.dom_node_objects.clear();
        self.dom_node_aliases.clear();
        self.dom_pending_ids.clear();
        self.dom_pending_parents.clear();
        self.dom_attributes.clear();
        self.dom_class_lists.clear();
        self.dom_node_lists.clear();
        self.dom_html_collections.clear();
        self.dom_tag_collections.clear();
        self.dom_query_lists.clear();
        self.dom_styles.clear();
        self.dom_children.clear();
        self.dom_text_nodes.clear();
        self.dom_tags.clear();
        self.dom_attached.clear();
        self.next_virtual_dom_node = 1;
        self.dom_click_listeners.clear();
        self.dom_click_capture_listeners.clear();
        self.dom_onclick.clear();
        self.dom_typed_listeners.clear();
        self.dom_dispatch_depth = 0;
        self.dom_programmatic_click_depth = 0;
        self.lifecycle_listeners.clear();
        self.timers.clear();
        self.next_timer_id = 1;
        self.microtasks.clear();
        self.microtasks_scheduled = 0;
        self.text_requests.clear();
        self.text_callbacks.clear();
        self.header_values.clear();
        self.next_text_request_id = 1;
        for element in elements.into_iter().take(4096) {
            self.dom_ids.entry(element.id).or_insert(element.node);
            self.dom_text.insert(element.node, element.text_content);
            self.dom_attached.insert(element.node);
        }
        let function = self.allocate_object_with_function(
            ObjectKind::Function,
            Some(self.object_prototype),
            HashMap::from([
                ("name".to_owned(), JsValue::String("getElementById".into())),
                ("length".to_owned(), JsValue::Number(1.0)),
            ]),
            Some(FunctionObject {
                implementation: FunctionImplementation::Builtin(BuiltinFunction::DomGetElementById),
            }),
        )?;
        let get_by_tag = self.allocate_lifecycle_method(
            "getElementsByTagName",
            BuiltinFunction::DomGetElementsByTagName,
        )?;
        self.object_mut(get_by_tag)?
            .properties
            .insert("length".into(), JsValue::Number(1.0));
        let query =
            self.allocate_lifecycle_method("querySelector", BuiltinFunction::DomQuerySelector)?;
        let query_all = self
            .allocate_lifecycle_method("querySelectorAll", BuiltinFunction::DomQuerySelectorAll)?;
        self.object_mut(query)?
            .properties
            .insert("length".into(), JsValue::Number(1.0));
        self.object_mut(query_all)?
            .properties
            .insert("length".into(), JsValue::Number(1.0));
        let create =
            self.allocate_lifecycle_method("createElement", BuiltinFunction::DomCreateElement)?;
        let create_event =
            self.allocate_lifecycle_method("createEvent", BuiltinFunction::DomCreateEvent)?;
        let create_text =
            self.allocate_lifecycle_method("createTextNode", BuiltinFunction::DomCreateTextNode)?;
        let add_listener = self.allocate_lifecycle_method(
            "addEventListener",
            BuiltinFunction::LifecycleAddEventListener,
        )?;
        let remove_listener = self.allocate_lifecycle_method(
            "removeEventListener",
            BuiltinFunction::LifecycleRemoveEventListener,
        )?;
        let document = self.allocate_object(
            ObjectKind::Ordinary,
            Some(self.object_prototype),
            HashMap::from([
                ("getElementById".into(), JsValue::Object(function)),
                ("getElementsByTagName".into(), JsValue::Object(get_by_tag)),
                ("querySelector".into(), JsValue::Object(query)),
                ("querySelectorAll".into(), JsValue::Object(query_all)),
                ("createElement".into(), JsValue::Object(create)),
                ("createTextNode".into(), JsValue::Object(create_text)),
                ("createEvent".into(), JsValue::Object(create_event)),
                ("body".into(), JsValue::Null),
                ("addEventListener".into(), JsValue::Object(add_listener)),
                (
                    "removeEventListener".into(),
                    JsValue::Object(remove_listener),
                ),
                ("readyState".into(), JsValue::String("loading".into())),
                ("onreadystatechange".into(), JsValue::Null),
            ]),
        )?;
        self.dom_document = Some(document);
        self.object_mut(self.global_object)?
            .properties
            .insert("addEventListener".into(), JsValue::Object(add_listener));
        self.object_mut(self.global_object)?.properties.insert(
            "removeEventListener".into(),
            JsValue::Object(remove_listener),
        );
        self.object_mut(self.global_object)?
            .properties
            .insert("onload".into(), JsValue::Null);
        self.install_global_binding(
            "window",
            JsValue::Object(self.global_object),
            false,
            VariableKind::Const,
        );
        self.install_global_binding(
            "document",
            JsValue::Object(document),
            false,
            VariableKind::Const,
        );
        for (name, builtin) in [
            ("setTimeout", BuiltinFunction::SetTimeout),
            ("clearTimeout", BuiltinFunction::ClearTimeout),
            ("setInterval", BuiltinFunction::SetInterval),
            ("clearInterval", BuiltinFunction::ClearInterval),
            ("opFetchText", BuiltinFunction::OpFetchText),
        ] {
            let function = self.allocate_lifecycle_method(name, builtin)?;
            self.object_mut(self.global_object)?
                .properties
                .insert(name.into(), JsValue::Object(function));
            self.install_global_binding(
                name,
                JsValue::Object(function),
                false,
                VariableKind::Const,
            );
        }
        let headers_ctor =
            self.allocate_lifecycle_method("Headers", BuiltinFunction::HeadersConstructor)?;
        self.install_global_binding(
            "Headers",
            JsValue::Object(headers_ctor),
            false,
            VariableKind::Const,
        );
        // Standards-shaped host facade; no networking work is done in JS.
        self.eval_script(include_str!("async_fetch.js"))?;
        Ok(())
    }
    /// Drain only requests, never JS closures, to the network dispatcher.
    pub fn take_text_requests(&mut self) -> Vec<TextRequest> {
        self.text_requests.drain(..).collect()
    }

    pub fn has_text_requests(&self) -> bool {
        !self.text_requests.is_empty()
    }

    /// Execute the callback on the page-owning worker after a validated
    /// network completion. Failed callbacks do not stop other tasks.
    pub fn complete_text_request(&mut self, id: u32, result: Result<String, String>) -> bool {
        self.complete_text_response_request(
            id,
            result.map(|text| TextResponse {
                text,
                address: String::new(),
                status: 200,
                status_text: "OK".into(),
                headers: Vec::new(),
                redirected: false,
            }),
        )
    }

    pub fn complete_text_response_request(
        &mut self,
        id: u32,
        result: Result<TextResponse, String>,
    ) -> bool {
        let Some(callback) = self.text_callbacks.remove(&id) else {
            return false;
        };
        let arguments = match result {
            Ok(response) => match self.build_response_metadata(&response) {
                Ok(metadata) => vec![
                    JsValue::String(response.text),
                    JsValue::Null,
                    JsValue::Object(metadata),
                ],
                Err(_) => vec![
                    JsValue::Null,
                    JsValue::String("response metadata budget exceeded".into()),
                    JsValue::Null,
                ],
            },
            Err(error) => vec![JsValue::Null, JsValue::String(error), JsValue::Null],
        };
        let mut steps = 0;
        let failed = !matches!(
            self.call_value(
                callback,
                JsValue::Object(self.global_object),
                arguments,
                &mut steps,
                0,
            ),
            Ok(CallOutcome::Value(_))
        );
        self.drain_microtasks();
        failed
    }

    fn build_response_metadata(&mut self, response: &TextResponse) -> Result<ObjectId, JsError> {
        let headers = self.make_headers(response.headers.iter().cloned().collect())?;
        self.allocate_object(
            ObjectKind::Ordinary,
            Some(self.object_prototype),
            HashMap::from([
                ("status".into(), JsValue::Number(f64::from(response.status))),
                (
                    "statusText".into(),
                    JsValue::String(response.status_text.clone()),
                ),
                ("url".into(), JsValue::String(response.address.clone())),
                ("redirected".into(), JsValue::Boolean(response.redirected)),
                ("headers".into(), JsValue::Object(headers)),
            ]),
        )
    }

    fn install_standard_primitives(&mut self) -> Result<(), JsError> {
        let value_of =
            self.allocate_lifecycle_method("valueOf", BuiltinFunction::ObjectPrototypeValueOf)?;
        let to_string =
            self.allocate_lifecycle_method("toString", BuiltinFunction::ObjectPrototypeToString)?;
        self.object_mut(self.object_prototype)?.properties.extend([
            ("valueOf".into(), JsValue::Object(value_of)),
            ("toString".into(), JsValue::Object(to_string)),
        ]);
        let push = self.allocate_lifecycle_method("push", BuiltinFunction::ArrayPush)?;
        let pop = self.allocate_lifecycle_method("pop", BuiltinFunction::ArrayPop)?;
        self.object_mut(push)?
            .properties
            .insert("length".into(), JsValue::Number(1.0));
        self.object_mut(pop)?
            .properties
            .insert("length".into(), JsValue::Number(0.0));
        self.object_mut(self.array_prototype)?.properties.extend([
            ("push".into(), JsValue::Object(push)),
            ("pop".into(), JsValue::Object(pop)),
        ]);
        for (name, builtin) in [
            ("Object", BuiltinFunction::ObjectConstructor),
            ("Array", BuiltinFunction::ArrayConstructor),
            ("Boolean", BuiltinFunction::BooleanConstructor),
            ("Number", BuiltinFunction::NumberConstructor),
            ("String", BuiltinFunction::StringConstructor),
            ("isNaN", BuiltinFunction::IsNaN),
            ("isFinite", BuiltinFunction::IsFinite),
        ] {
            let id = self.allocate_lifecycle_method(name, builtin)?;
            self.object_mut(id)?
                .properties
                .insert("length".into(), JsValue::Number(1.0));
            if name == "Object" {
                let prototype = self.object_prototype;
                self.object_mut(id)?
                    .properties
                    .insert("prototype".into(), JsValue::Object(prototype));
            } else if name == "Array" {
                let prototype = self.array_prototype;
                self.object_mut(id)?
                    .properties
                    .insert("prototype".into(), JsValue::Object(prototype));
            } else if matches!(name, "Boolean" | "Number" | "String") {
                let primitive_value = self
                    .allocate_lifecycle_method("valueOf", BuiltinFunction::BoxedPrimitiveValueOf)?;
                let primitive_string = self.allocate_lifecycle_method(
                    "toString",
                    BuiltinFunction::BoxedPrimitiveToString,
                )?;
                let proto = self.allocate_object(
                    ObjectKind::Ordinary,
                    Some(self.object_prototype),
                    HashMap::from([
                        ("constructor".into(), JsValue::Object(id)),
                        ("valueOf".into(), JsValue::Object(primitive_value)),
                        ("toString".into(), JsValue::Object(primitive_string)),
                    ]),
                )?;
                self.object_mut(id)?
                    .properties
                    .insert("prototype".into(), JsValue::Object(proto));
                let intrinsic = match name {
                    "String" => JsValue::String(String::new()),
                    "Number" => JsValue::Number(0.0),
                    "Boolean" => JsValue::Boolean(false),
                    _ => unreachable!("primitive constructor"),
                };
                self.boxed_values.insert(proto, intrinsic);
                if name == "String" {
                    let char_at =
                        self.allocate_lifecycle_method("charAt", BuiltinFunction::StringCharAt)?;
                    self.object_mut(char_at)?
                        .properties
                        .insert("length".into(), JsValue::Number(1.0));
                    self.object_mut(proto)?
                        .properties
                        .insert("charAt".into(), JsValue::Object(char_at));
                }
            }
            self.install_global_binding(name, JsValue::Object(id), true, VariableKind::Var);
        }
        if let Some(JsValue::Object(number_id)) = self.global("Number").cloned() {
            self.object_mut(number_id)?
                .properties
                .extend(HashMap::from([
                    ("MAX_VALUE".into(), JsValue::Number(f64::MAX)),
                    ("MIN_VALUE".into(), JsValue::Number(f64::from_bits(1))),
                    ("POSITIVE_INFINITY".into(), JsValue::Number(f64::INFINITY)),
                    (
                        "NEGATIVE_INFINITY".into(),
                        JsValue::Number(f64::NEG_INFINITY),
                    ),
                    ("NaN".into(), JsValue::Number(f64::NAN)),
                    ("EPSILON".into(), JsValue::Number(f64::EPSILON)),
                    (
                        "MAX_SAFE_INTEGER".into(),
                        JsValue::Number(9_007_199_254_740_991.0),
                    ),
                    (
                        "MIN_SAFE_INTEGER".into(),
                        JsValue::Number(-9_007_199_254_740_991.0),
                    ),
                ]));
        }
        for (constructor, method, builtin, length) in [
            ("Array", "isArray", BuiltinFunction::ArrayIsArray, 1.0),
            ("Array", "of", BuiltinFunction::ArrayOf, 0.0),
            ("Number", "isNaN", BuiltinFunction::NumberIsNaN, 1.0),
            ("Number", "isFinite", BuiltinFunction::NumberIsFinite, 1.0),
            ("Object", "is", BuiltinFunction::ObjectIs, 2.0),
        ] {
            let method_id = self.allocate_lifecycle_method(method, builtin)?;
            self.object_mut(method_id)?
                .properties
                .insert("length".into(), JsValue::Number(length));
            if let Some(JsValue::Object(constructor_id)) = self.global(constructor).cloned() {
                self.object_mut(constructor_id)?
                    .properties
                    .insert(method.into(), JsValue::Object(method_id));
            }
        }
        self.install_global_binding("NaN", JsValue::Number(f64::NAN), false, VariableKind::Const);
        self.install_global_binding(
            "Infinity",
            JsValue::Number(f64::INFINITY),
            false,
            VariableKind::Const,
        );
        Ok(())
    }

    fn box_primitive(
        &mut self,
        primitive: JsValue,
        constructor_name: &str,
    ) -> Result<JsValue, JsError> {
        let constructor = self.global(constructor_name).cloned();
        let prototype = if let Some(JsValue::Object(id)) = constructor {
            match self.get_object_property(id, "prototype")? {
                JsValue::Object(proto) => proto,
                _ => self.object_prototype,
            }
        } else {
            self.object_prototype
        };
        let boxed = self.allocate_object(ObjectKind::Ordinary, Some(prototype), HashMap::new())?;
        self.boxed_values.insert(boxed, primitive);
        Ok(JsValue::Object(boxed))
    }

    fn coerce_to_primitive(
        &mut self,
        value: JsValue,
        steps: &mut usize,
        call_depth: usize,
    ) -> Result<CallOutcome, JsError> {
        let JsValue::Object(id) = value.clone() else {
            return Ok(CallOutcome::Value(value));
        };
        // Ordinary ToPrimitive with default hint. Date's preferred hint is
        // not supported yet. Call user functions with the actual receiver;
        // throwing valueOf/toString must propagate to try/catch unchanged.
        for method in ["valueOf", "toString"] {
            let candidate = self.get_object_property(id, method)?;
            let JsValue::Object(method_id) = candidate.clone() else {
                continue;
            };
            if self.object(method_id)?.function.is_none() {
                continue;
            }
            match self.call_value(candidate, value.clone(), Vec::new(), steps, call_depth + 1)? {
                CallOutcome::Value(primitive) if !matches!(primitive, JsValue::Object(_)) => {
                    return Ok(CallOutcome::Value(primitive));
                }
                CallOutcome::Thrown(thrown) => return Ok(CallOutcome::Thrown(thrown)),
                _ => {}
            }
        }
        Err(JsError::type_error(
            "cannot convert object to primitive value",
        ))
    }

    fn typeof_value(&self, value: &JsValue) -> Result<JsValue, JsError> {
        let category = match value {
            JsValue::Undefined => "undefined",
            JsValue::Null => "object",
            JsValue::Boolean(_) => "boolean",
            JsValue::Number(_) => "number",
            JsValue::String(_) => "string",
            JsValue::Object(id) if self.object(*id)?.function.is_some() => "function",
            JsValue::Object(_) => "object",
        };
        Ok(JsValue::String(category.into()))
    }

    /// Bounded HasProperty for the initial original JS in operator.
    /// Includes original DOM live accessors not stored as own properties.
    fn has_property(&self, start: ObjectId, key: &str) -> Result<bool, JsError> {
        let mut current = Some(start);
        let mut remaining = self.heap.len().saturating_add(1);
        while let Some(id) = current {
            if remaining == 0 {
                return Err(JsError::type_error("cyclic prototype chain"));
            }
            remaining -= 1;
            let obj = self.object(id)?;
            match obj.kind {
                ObjectKind::DomElement(_)
                    if matches!(
                        key,
                        "childElementCount"
                            | "firstElementChild"
                            | "lastElementChild"
                            | "previousElementSibling"
                            | "nextElementSibling"
                            | "children"
                            | "parentNode"
                            | "childNodes"
                            | "firstChild"
                            | "lastChild"
                            | "nodeType"
                            | "nodeName"
                            | "tagName"
                            | "isConnected"
                            | "nodeValue"
                            | "ownerDocument"
                            | "className"
                            | "classList"
                            | "style"
                            | "previousSibling"
                            | "nextSibling"
                    ) =>
                {
                    return Ok(true);
                }
                ObjectKind::DomText(_)
                    if matches!(
                        key,
                        "parentNode"
                            | "childNodes"
                            | "firstChild"
                            | "lastChild"
                            | "nodeType"
                            | "nodeName"
                            | "isConnected"
                            | "ownerDocument"
                            | "previousSibling"
                            | "nextSibling"
                            | "length"
                    ) =>
                {
                    return Ok(true);
                }
                ObjectKind::DomNodeList(parent) => {
                    if key == "length" {
                        return Ok(true);
                    }
                    if let Ok(index) = key.parse::<usize>()
                        && self
                            .dom_children
                            .get(&parent)
                            .is_some_and(|c| index < c.len())
                    {
                        return Ok(true);
                    }
                }
                ObjectKind::DomHtmlCollection(parent) => {
                    if key == "length" {
                        return Ok(true);
                    }
                    if let Ok(index) = key.parse::<usize>()
                        && index < self.html_collection_nodes(parent).len()
                    {
                        return Ok(true);
                    }
                    if self.html_collection_named(parent, key).is_some() {
                        return Ok(true);
                    }
                }
                ObjectKind::DomQueryNodeList => {
                    if key == "length" {
                        return Ok(true);
                    }
                    if let Ok(index) = key.parse::<usize>()
                        && self
                            .dom_query_lists
                            .get(&id)
                            .is_some_and(|nodes| index < nodes.len())
                    {
                        return Ok(true);
                    }
                }
                ObjectKind::DomTagCollection => {
                    if key == "length" {
                        return Ok(true);
                    }
                    if let Some((root, tag)) = self.dom_tag_collections.get(&id)
                        && let Ok(index) = key.parse::<usize>()
                        && index < self.elements_by_tag(*root, tag).len()
                    {
                        return Ok(true);
                    }
                }
                ObjectKind::DomClassList(_) if key == "length" || key == "value" => {
                    return Ok(true);
                }
                ObjectKind::DomStyle(_) if key == "cssText" => return Ok(true),
                _ => {}
            }
            if obj.properties.contains_key(key) {
                return Ok(true);
            }
            current = obj.prototype;
        }
        Ok(false)
    }

    fn binary_with_coercion(
        &mut self,
        op: BinaryOp,
        left: JsValue,
        right: JsValue,
        steps: &mut usize,
        call_depth: usize,
    ) -> Result<CallOutcome, JsError> {
        if op == BinaryOp::In {
            let JsValue::Object(rhs) = right else {
                return Err(JsError::type_error("right operand of in must be an object"));
            };
            // Full ToPropertyKey on user-defined object keys remains future work.
            let key = left.to_js_string();
            return self
                .has_property(rhs, &key)
                .map(|exists| CallOutcome::Value(JsValue::Boolean(exists)));
        }
        if op == BinaryOp::InstanceOf {
            let JsValue::Object(function_id) = right else {
                return Err(JsError::type_error(
                    "instanceof right operand is not callable",
                ));
            };
            if self.object(function_id)?.function.is_none() {
                return Err(JsError::type_error(
                    "instanceof right operand is not callable",
                ));
            }
            let prototype = self.get_object_property(function_id, "prototype")?;
            let JsValue::Object(prototype) = prototype else {
                return Err(JsError::type_error(
                    "instanceof constructor has invalid prototype",
                ));
            };
            let JsValue::Object(object) = left else {
                return Ok(CallOutcome::Value(JsValue::Boolean(false)));
            };
            let parent = self.object(object)?.prototype;
            let matches = if let Some(parent) = parent {
                self.prototype_chain_contains(parent, prototype)?
            } else {
                false
            };
            return Ok(CallOutcome::Value(JsValue::Boolean(matches)));
        }
        let needs_left = matches!(left, JsValue::Object(_));
        let needs_right = matches!(right, JsValue::Object(_));
        let coerce = match op {
            BinaryOp::StrictEqual
            | BinaryOp::StrictNotEqual
            | BinaryOp::InstanceOf
            | BinaryOp::In => (false, false),
            BinaryOp::Equal | BinaryOp::NotEqual => {
                // Objects compare by identity, but object-to-primitive
                // conversion is required for loose equality vs scalars.
                (needs_left && !needs_right, needs_right && !needs_left)
            }
            _ => (needs_left, needs_right),
        };
        let left = if coerce.0 {
            match self.coerce_to_primitive(left, steps, call_depth)? {
                CallOutcome::Value(v) => v,
                CallOutcome::Thrown(v) => return Ok(CallOutcome::Thrown(v)),
            }
        } else {
            left
        };
        let right = if coerce.1 {
            match self.coerce_to_primitive(right, steps, call_depth)? {
                CallOutcome::Value(v) => v,
                CallOutcome::Thrown(v) => return Ok(CallOutcome::Thrown(v)),
            }
        } else {
            right
        };
        Ok(CallOutcome::Value(apply_binary(op, left, right)))
    }

    fn array_length(&self, id: ObjectId) -> Result<usize, JsError> {
        let obj = self.object(id)?;
        let value = obj.properties.get("length");
        match value {
            Some(JsValue::Number(n))
                if n.is_finite() && *n >= 0.0 && *n <= 4096.0 && n.fract() == 0.0 =>
            {
                Ok(*n as usize)
            }
            _ => Err(JsError::type_error("invalid array length")),
        }
    }

    fn install_json_methods(&mut self) -> Result<(), JsError> {
        let parse = self.allocate_lifecycle_method("parse", BuiltinFunction::JsonParse)?;
        let stringify =
            self.allocate_lifecycle_method("stringify", BuiltinFunction::JsonStringify)?;
        let object = self.allocate_object(
            ObjectKind::Ordinary,
            Some(self.object_prototype),
            HashMap::from([
                ("parse".into(), JsValue::Object(parse)),
                ("stringify".into(), JsValue::Object(stringify)),
            ]),
        )?;
        self.install_global_binding("JSON", JsValue::Object(object), false, VariableKind::Const);
        Ok(())
    }

    fn json_to_value(
        &mut self,
        value: crate::json::JsonValue,
        depth: usize,
    ) -> Result<JsValue, JsError> {
        if depth > 64 {
            return Err(JsError::execution_limit("JSON nesting budget exceeded"));
        }
        use crate::json::JsonValue;
        Ok(match value {
            JsonValue::Null => JsValue::Null,
            JsonValue::Boolean(flag) => JsValue::Boolean(flag),
            JsonValue::Number(number) => JsValue::Number(number),
            JsonValue::String(text) => JsValue::String(text),
            JsonValue::Array(elements) => {
                let mut props = HashMap::new();
                props.insert("length".into(), JsValue::Number(elements.len() as f64));
                for (index, element) in elements.into_iter().enumerate() {
                    props.insert(index.to_string(), self.json_to_value(element, depth + 1)?);
                }
                let object =
                    self.allocate_object(ObjectKind::Array, Some(self.array_prototype), props)?;
                JsValue::Object(object)
            }
            JsonValue::Object(fields) => {
                let mut props = HashMap::new();
                for (key, entry) in fields {
                    // Always write JSON keys as own data properties, including
                    // "__proto__", never through JS's prototype setter.
                    props.insert(key, self.json_to_value(entry, depth + 1)?);
                }
                let object =
                    self.allocate_object(ObjectKind::Ordinary, Some(self.object_prototype), props)?;
                JsValue::Object(object)
            }
        })
    }

    fn json_from_value(
        &self,
        value: &JsValue,
        seen: &mut Vec<ObjectId>,
        depth: usize,
    ) -> Result<Option<String>, String> {
        if depth > 64 {
            return Err("JSON nesting budget exceeded".into());
        }
        let result = match value {
            JsValue::Undefined => return Ok(None),
            JsValue::Null => "null".into(),
            JsValue::Boolean(flag) => flag.to_string(),
            JsValue::Number(number) if !number.is_finite() => "null".into(),
            JsValue::Number(number) if *number == 0.0 => "0".into(),
            JsValue::Number(number) => number.to_string(),
            JsValue::String(text) => crate::json::quote(text),
            JsValue::Object(id) => {
                if seen.contains(id) {
                    return Err("Converting circular structure to JSON".into());
                }
                let object = self.object(*id).map_err(|error| error.to_string())?;
                if object.function.is_some() {
                    return Ok(None);
                }
                seen.push(*id);
                let mut items = Vec::new();
                if object.kind == ObjectKind::Array {
                    let length = match object.properties.get("length") {
                        Some(JsValue::Number(n)) if n.is_finite() && *n >= 0.0 && *n <= 4096.0 => {
                            *n as usize
                        }
                        _ => return Err("invalid JSON array length".into()),
                    };
                    for i in 0..length {
                        let value = object
                            .properties
                            .get(&i.to_string())
                            .unwrap_or(&JsValue::Undefined);
                        let part = self.json_from_value(value, seen, depth + 1)?;
                        items.push(part.unwrap_or_else(|| "null".into()));
                    }
                    seen.pop();
                    format!("[{}]", items.join(","))
                } else {
                    // Deterministic key order for now. Insertion-order
                    // enumeration and toJSON hooks need dedicated VM support.
                    let mut keys: Vec<_> = object.properties.keys().collect();
                    keys.sort();
                    for key in keys {
                        let part =
                            self.json_from_value(&object.properties[key], seen, depth + 1)?;
                        if let Some(part) = part {
                            items.push(format!("{}:{}", crate::json::quote(key), part));
                        }
                    }
                    seen.pop();
                    format!("{{{}}}", items.join(","))
                }
            }
        };
        if result.len() > 64 * 1024 {
            return Err("JSON output exceeds 64 KiB".into());
        }
        Ok(Some(result))
    }

    fn header_name(value: &JsValue) -> Result<String, JsError> {
        let key = value.to_js_string().to_ascii_lowercase();
        if key.is_empty()
            || key.len() > 128
            || !key
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"!#$%&'*+-.^_|~".contains(&b))
        {
            return Err(JsError::type_error("invalid HTTP header name"));
        }
        Ok(key)
    }

    fn header_value(value: &JsValue) -> Result<String, JsError> {
        let text = value.to_js_string();
        let text = text.trim().to_owned();
        if text.len() > 4096 || text.chars().any(|c| c.is_control()) {
            return Err(JsError::type_error("invalid HTTP header value"));
        }
        Ok(text)
    }

    fn check_header_budget(values: &HashMap<String, String>) -> Result<(), JsError> {
        if values.len() > MAX_HTTP_HEADERS
            || values.iter().map(|(k, v)| k.len() + v.len()).sum::<usize>() > MAX_HTTP_HEADER_BYTES
        {
            return Err(JsError::type_error("Headers budget exceeded"));
        }
        Ok(())
    }

    fn read_headers_init(
        &self,
        init: Option<&JsValue>,
    ) -> Result<HashMap<String, String>, JsError> {
        let mut result = HashMap::new();
        let Some(value) = init else {
            return Ok(result);
        };
        if matches!(value, JsValue::Undefined | JsValue::Null) {
            return Ok(result);
        }
        let JsValue::Object(id) = value else {
            return Err(JsError::type_error(
                "Headers requires an object, array or Headers",
            ));
        };
        let object = self.object(*id)?;
        if object.kind == ObjectKind::Headers {
            return Ok(self.header_values.get(id).cloned().unwrap_or_default());
        }
        let mut pairs: Vec<(JsValue, JsValue)> = Vec::new();
        if object.kind == ObjectKind::Array {
            let count = match object.properties.get("length") {
                Some(JsValue::Number(n))
                    if *n >= 0.0 && *n <= MAX_HTTP_HEADERS as f64 && *n == n.floor() =>
                {
                    *n as usize
                }
                _ => return Err(JsError::type_error("invalid header list length")),
            };
            for index in 0..count {
                let entry = object
                    .properties
                    .get(&index.to_string())
                    .ok_or_else(|| JsError::type_error("missing header pair"))?;
                let JsValue::Object(pair_id) = entry else {
                    return Err(JsError::type_error("header entry must be a pair"));
                };
                let pair = self.object(*pair_id)?;
                if pair.kind != ObjectKind::Array
                    || !matches!(pair.properties.get("length"), Some(JsValue::Number(2.0)))
                {
                    return Err(JsError::type_error("header pair must have two entries"));
                }
                pairs.push((
                    pair.properties
                        .get("0")
                        .cloned()
                        .unwrap_or(JsValue::Undefined),
                    pair.properties
                        .get("1")
                        .cloned()
                        .unwrap_or(JsValue::Undefined),
                ));
            }
        } else if object.kind == ObjectKind::Ordinary {
            if object.properties.len() > MAX_HTTP_HEADERS {
                return Err(JsError::type_error("too many header fields"));
            }
            pairs.extend(
                object
                    .properties
                    .iter()
                    .map(|(key, value)| (JsValue::String(key.clone()), value.clone())),
            );
        } else {
            return Err(JsError::type_error("unsupported Headers initializer"));
        }
        for (key, value) in pairs {
            let key = Self::header_name(&key)?;
            let value = Self::header_value(&value)?;
            let current = result.entry(key).or_insert_with(String::new);
            if !current.is_empty() {
                current.push_str(", ");
            }
            current.push_str(&value);
        }
        Self::check_header_budget(&result)?;
        Ok(result)
    }

    fn make_headers(&mut self, values: HashMap<String, String>) -> Result<ObjectId, JsError> {
        Self::check_header_budget(&values)?;
        let mut methods = HashMap::new();
        for (name, builtin) in [
            ("get", BuiltinFunction::HeadersGet),
            ("has", BuiltinFunction::HeadersHas),
            ("set", BuiltinFunction::HeadersSet),
            ("append", BuiltinFunction::HeadersAppend),
            ("delete", BuiltinFunction::HeadersDelete),
        ] {
            let method = self.allocate_lifecycle_method(name, builtin)?;
            methods.insert(name.into(), JsValue::Object(method));
        }
        let id = self.allocate_object(ObjectKind::Headers, Some(self.object_prototype), methods)?;
        self.header_values.insert(id, values);
        Ok(id)
    }

    fn allowed_request_headers(&self, value: &JsValue) -> Result<Vec<(String, String)>, JsError> {
        let JsValue::Object(id) = value else {
            return Err(JsError::type_error("request headers must be Headers"));
        };
        if self.object(*id)?.kind != ObjectKind::Headers {
            return Err(JsError::type_error("request headers must be Headers"));
        }
        let values = self.header_values.get(id).cloned().unwrap_or_default();
        Self::check_header_budget(&values)?;
        let mut headers = Vec::new();
        for (name, value) in values {
            // Same-origin safe subset, excluding cookies, authorization and
            // hop-by-hop fields. No arbitrary header injection.
            if !(matches!(
                name.as_str(),
                "accept" | "accept-language" | "if-none-match" | "if-modified-since"
            ) || name.starts_with("x-"))
            {
                return Err(JsError::type_error("unsupported request header"));
            }
            if !value.bytes().all(|b| (32..=126).contains(&b)) {
                return Err(JsError::type_error("non-ASCII request header value"));
            }
            headers.push((name, value));
        }
        headers.sort();
        Ok(headers)
    }

    /// The host decides when to wake the page worker; no VM timer spawns threads.
    pub fn next_timer_wait(&self) -> Option<Duration> {
        if !self.microtasks.is_empty() {
            return Some(Duration::ZERO);
        }
        self.timers
            .iter()
            .map(|timer| timer.due.saturating_duration_since(Instant::now()))
            .min()
    }

    /// Run at most max_callbacks due tasks on the page-owning thread.
    /// A timer removed by clearTimeout in an earlier callback never fires.
    pub fn run_due_timers(&mut self, max_callbacks: usize) -> TimerReport {
        let mut report = TimerReport::default();
        report.failed += self.drain_microtasks();
        for _ in 0..max_callbacks.min(16) {
            let now = Instant::now();
            let Some((index, _)) = self
                .timers
                .iter()
                .enumerate()
                .filter(|(_, timer)| timer.due <= now)
                .min_by_key(|(_, timer)| (timer.due, timer.id))
            else {
                break;
            };
            let timer = self.timers.swap_remove(index);
            // Reinsert before callback so clearInterval can cancel itself.
            if let Some(period) = timer.interval {
                let mut next = timer.clone();
                next.due = Instant::now() + period;
                self.timers.push(next);
            }
            report.fired += 1;
            let mut steps = 0;
            match self.call_value(
                timer.callback,
                JsValue::Object(self.global_object),
                timer.arguments,
                &mut steps,
                0,
            ) {
                Ok(CallOutcome::Value(_)) => {}
                Ok(CallOutcome::Thrown(_)) | Err(_) => report.failed += 1,
            }
            report.failed += self.drain_microtasks();
        }
        report
    }

    /// FIFO checkpoint after a top-level job. Tasks queued by tasks also
    /// run in this checkpoint, within the bounded per-checkpoint budget.
    pub fn drain_microtasks(&mut self) -> usize {
        let mut failed = 0;
        for _ in 0..MAX_PENDING_MICROTASKS {
            let Some(callback) = self.microtasks.pop_front() else {
                break;
            };
            let mut steps = 0;
            match self.call_value(
                callback,
                JsValue::Object(self.global_object),
                Vec::new(),
                &mut steps,
                0,
            ) {
                Ok(CallOutcome::Value(_)) => {}
                Ok(CallOutcome::Thrown(_)) | Err(_) => failed += 1,
            }
        }
        failed
    }

    /// Construct one of the document/window host methods.
    fn allocate_lifecycle_method(
        &mut self,
        name: &str,
        builtin: BuiltinFunction,
    ) -> Result<ObjectId, JsError> {
        self.allocate_object_with_function(
            ObjectKind::Function,
            Some(self.object_prototype),
            HashMap::from([
                ("name".into(), JsValue::String(name.into())),
                ("length".into(), JsValue::Number(2.0)),
            ]),
            Some(FunctionObject {
                implementation: FunctionImplementation::Builtin(builtin),
            }),
        )
    }

    /// Called by the page engine at parser and resource lifecycle boundaries.
    /// The readyState property is host-owned rather than writable JS state.
    pub fn set_document_ready_state(&mut self, state: &str) -> Result<(), JsError> {
        if !matches!(state, "loading" | "interactive" | "complete") {
            return Err(JsError::type_error("invalid document readyState"));
        }
        if let Some(document) = self.dom_document {
            self.object_mut(document)?
                .properties
                .insert("readyState".into(), JsValue::String(state.into()));
        }
        Ok(())
    }

    /// Dispatch a bounded non-bubbling lifecycle event on document/window.
    pub fn dispatch_lifecycle_event(&mut self, name: &str) -> Result<(), JsError> {
        let Some(document) = self.dom_document else {
            return Ok(());
        };
        let receiver = match name {
            "readystatechange" | "DOMContentLoaded" => document,
            "load" => self.global_object,
            _ => return Err(JsError::type_error("unknown lifecycle event")),
        };
        let event = self.allocate_object(
            ObjectKind::Ordinary,
            Some(self.object_prototype),
            HashMap::from([
                ("type".into(), JsValue::String(name.into())),
                ("target".into(), JsValue::Object(document)),
                ("currentTarget".into(), JsValue::Object(receiver)),
                ("eventPhase".into(), JsValue::Number(2.0)),
                ("bubbles".into(), JsValue::Boolean(false)),
                ("cancelable".into(), JsValue::Boolean(false)),
                ("defaultPrevented".into(), JsValue::Boolean(false)),
            ]),
        )?;
        let mut handlers = Vec::new();
        for capture in [true, false] {
            if let Some(registered) =
                self.lifecycle_listeners
                    .get(&(receiver, name.into(), capture))
            {
                handlers.extend(registered.clone());
            }
        }
        let property = match name {
            "readystatechange" => Some("onreadystatechange"),
            "load" => Some("onload"),
            _ => None,
        };
        if let Some(property) = property
            && let Some(callback) = self.object(receiver)?.properties.get(property)
            && let JsValue::Object(id) = callback
            && self.object(*id)?.function.is_some()
        {
            handlers.push(callback.clone());
        }
        let mut steps = 0;
        for callback in handlers {
            self.call_event_handler(callback, receiver, event, &mut steps)?;
        }
        self.finish_event_dispatch(event)
    }
    pub fn refresh_dom_snapshot(&mut self, elements: impl IntoIterator<Item = DomElementSnapshot>) {
        self.dom_ids.clear();
        for element in elements.into_iter().take(4096) {
            self.dom_ids.entry(element.id).or_insert(element.node);
            self.dom_text.insert(element.node, element.text_content);
            self.dom_attached.insert(element.node);
        }
    }
    /// Synchronize real parsed-node attributes at parser script boundaries.
    /// Detached/created attributes are kept by the VM across host commits.
    pub fn sync_dom_attributes(&mut self, node: usize, attributes: Vec<(String, String)>) {
        self.dom_attributes
            .insert(node, attributes.into_iter().collect());
    }

    pub fn sync_dom_existing_node(
        &mut self,
        node: usize,
        parent: Option<usize>,
        attrs: Vec<(String, String)>,
    ) {
        self.dom_attached.insert(node);
        if let Some(parent) = parent {
            self.dom_pending_parents.insert(node, parent);
        }
        self.sync_dom_attributes(node, attrs);
    }

    pub fn sync_dom_tag(&mut self, node: usize, tag: String) {
        self.dom_tags.insert(node, tag);
    }

    pub fn sync_dom_children(&mut self, node: usize, children: Vec<usize>) {
        self.dom_children.insert(node, children);
    }

    pub fn sync_dom_text_node(&mut self, node: usize, parent: Option<usize>, value: String) {
        self.dom_text_nodes.insert(node);
        self.dom_text.insert(node, value);
        self.dom_attached.insert(node);
        if let Some(parent) = parent {
            self.dom_pending_parents.insert(node, parent);
        }
    }
    /// A host may associate a newly committed synthetic element with its
    /// physical NodeId. This preserves JS object identity across later reads.
    pub fn bind_dom_node(&mut self, virtual_node: usize, physical_node: usize) {
        self.dom_node_aliases.insert(virtual_node, physical_node);
        if let Some(&object) = self.dom_node_objects.get(&virtual_node) {
            self.dom_node_objects.insert(physical_node, object);
            self.heap[object.0].kind = match self.heap[object.0].kind {
                ObjectKind::DomElement(_) => ObjectKind::DomElement(physical_node),
                ObjectKind::DomText(_) => ObjectKind::DomText(physical_node),
                kind => kind,
            };
        }
        if let Some(list) = self.dom_node_lists.remove(&virtual_node) {
            self.dom_node_lists.insert(physical_node, list);
            self.heap[list.0].kind = ObjectKind::DomNodeList(physical_node);
        }
        for nodes in self.dom_query_lists.values_mut() {
            for node in nodes {
                if *node == virtual_node {
                    *node = physical_node;
                }
            }
        }
        if let Some(collection) = self.dom_html_collections.remove(&virtual_node) {
            self.dom_html_collections.insert(physical_node, collection);
            self.heap[collection.0].kind = ObjectKind::DomHtmlCollection(physical_node);
        }
        if let Some(list) = self.dom_class_lists.remove(&virtual_node) {
            self.dom_class_lists.insert(physical_node, list);
            self.heap[list.0].kind = ObjectKind::DomClassList(physical_node);
        }
        if let Some(style) = self.dom_styles.remove(&virtual_node) {
            self.dom_styles.insert(physical_node, style);
            self.heap[style.0].kind = ObjectKind::DomStyle(physical_node);
        }
        if let Some(children) = self.dom_children.remove(&virtual_node) {
            self.dom_children.insert(physical_node, children);
        }
        if let Some(parent) = self.dom_pending_parents.remove(&virtual_node) {
            self.dom_pending_parents.insert(physical_node, parent);
        }
        if let Some(attributes) = self.dom_attributes.remove(&virtual_node) {
            self.dom_attributes.insert(physical_node, attributes);
        }
        if let Some(tag) = self.dom_tags.remove(&virtual_node) {
            self.dom_tags.insert(physical_node, tag);
        }
        if let Some(text) = self.dom_text.remove(&virtual_node) {
            self.dom_text.insert(physical_node, text);
        }
        if self.dom_text_nodes.remove(&virtual_node) {
            self.dom_text_nodes.insert(physical_node);
        }
        self.dom_attached.insert(virtual_node);
        self.dom_attached.insert(physical_node);
        if let Some(mut listeners) = self.dom_click_listeners.remove(&virtual_node) {
            self.dom_click_listeners
                .entry(physical_node)
                .or_default()
                .append(&mut listeners);
        }
        if let Some(mut listeners) = self.dom_click_capture_listeners.remove(&virtual_node) {
            self.dom_click_capture_listeners
                .entry(physical_node)
                .or_default()
                .append(&mut listeners);
        }
        let typed_keys = self
            .dom_typed_listeners
            .keys()
            .filter(|(node, _, _)| *node == virtual_node)
            .cloned()
            .collect::<Vec<_>>();
        for (node, name, capture) in typed_keys {
            if let Some(mut handlers) =
                self.dom_typed_listeners
                    .remove(&(node, name.clone(), capture))
            {
                self.dom_typed_listeners
                    .entry((physical_node, name, capture))
                    .or_default()
                    .append(&mut handlers);
            }
        }
        if let Some(callback) = self.dom_onclick.remove(&virtual_node) {
            self.dom_onclick.insert(physical_node, callback);
        }
    }

    /// Expose the real tree-builder's body element as document.body.
    pub fn set_dom_body_node(&mut self, node: usize) -> Result<(), JsError> {
        let object = self.dom_element_object(node)?;
        if let Some(document) = self.dom_document {
            self.object_mut(document)?
                .properties
                .insert("body".into(), JsValue::Object(object));
        }
        self.dom_attached.insert(node);
        Ok(())
    }
    pub fn resolve_dom_node(&self, node: usize) -> usize {
        self.dom_node_aliases.get(&node).copied().unwrap_or(node)
    }
    pub fn take_dom_operations(&mut self) -> Vec<DomOperation> {
        self.dom_mutations.clear();
        std::mem::take(&mut self.dom_operations)
    }
    pub fn take_dom_mutations(&mut self) -> Vec<DomTextMutation> {
        std::mem::take(&mut self.dom_mutations)
    }

    /// Dispatch a click on one element, without an ancestor path.
    pub fn dispatch_dom_click(&mut self, node: usize) -> Result<bool, JsError> {
        self.dispatch_dom_click_path(&[node])
    }

    /// Deliver a click through capture, target, then bubbling phases.
    /// The supplied path is target-first and bounded by the page engine.
    pub fn dispatch_dom_click_path(&mut self, path: &[usize]) -> Result<bool, JsError> {
        if path.is_empty() || !path.iter().any(|&node| self.has_dom_click_listener(node)) {
            return Ok(false);
        }
        let target = path[0];
        let target_object = self.dom_element_object(target)?;
        let stop = self.allocate_object_with_function(
            ObjectKind::Function,
            Some(self.object_prototype),
            HashMap::from([
                ("name".into(), JsValue::String("stopPropagation".into())),
                ("length".into(), JsValue::Number(0.0)),
            ]),
            Some(FunctionObject {
                implementation: FunctionImplementation::Builtin(
                    BuiltinFunction::EventStopPropagation,
                ),
            }),
        )?;
        let prevent = self.allocate_object_with_function(
            ObjectKind::Function,
            Some(self.object_prototype),
            HashMap::from([
                ("name".into(), JsValue::String("preventDefault".into())),
                ("length".into(), JsValue::Number(0.0)),
            ]),
            Some(FunctionObject {
                implementation: FunctionImplementation::Builtin(
                    BuiltinFunction::EventPreventDefault,
                ),
            }),
        )?;
        let event = self.allocate_object(
            ObjectKind::Ordinary,
            Some(self.object_prototype),
            HashMap::from([
                ("type".into(), JsValue::String("click".into())),
                ("target".into(), JsValue::Object(target_object)),
                ("currentTarget".into(), JsValue::Null),
                ("bubbles".into(), JsValue::Boolean(true)),
                ("cancelable".into(), JsValue::Boolean(true)),
                ("defaultPrevented".into(), JsValue::Boolean(false)),
                ("eventPhase".into(), JsValue::Number(0.0)),
                ("stopPropagation".into(), JsValue::Object(stop)),
                ("preventDefault".into(), JsValue::Object(prevent)),
                ("__stopPropagation".into(), JsValue::Boolean(false)),
            ]),
        )?;
        let mut steps = 0;
        let mut handled = false;

        // Capture travels root -> parent of target.
        for &node in path.iter().take(64).skip(1).rev() {
            let handlers = self
                .dom_click_capture_listeners
                .get(&node)
                .cloned()
                .unwrap_or_default();
            if handlers.is_empty() {
                continue;
            }
            handled = true;
            let receiver = self.dom_element_object(node)?;
            self.prepare_event_callback(event, receiver, 1)?;
            for handler in handlers {
                self.call_event_handler(handler, receiver, event, &mut steps)?;
            }
            if self.event_propagation_stopped(event)? {
                self.finish_event_dispatch(event)?;
                return Ok(handled);
            }
        }

        // At target, capture listeners run before ordinary listeners.
        let receiver = target_object;
        let captures = self
            .dom_click_capture_listeners
            .get(&target)
            .cloned()
            .unwrap_or_default();
        let mut ordinary = self
            .dom_click_listeners
            .get(&target)
            .cloned()
            .unwrap_or_default();
        if let Some(handler) = self.dom_onclick.get(&target).cloned() {
            ordinary.push(handler);
        }
        if !captures.is_empty() || !ordinary.is_empty() {
            handled = true;
            self.prepare_event_callback(event, receiver, 2)?;
            for handler in captures.into_iter().chain(ordinary) {
                self.call_event_handler(handler, receiver, event, &mut steps)?;
            }
        }
        if self.event_propagation_stopped(event)? {
            self.finish_event_dispatch(event)?;
            return Ok(handled);
        }

        // Bubble travels parent -> root.
        for &node in path.iter().take(64).skip(1) {
            let mut handlers = self
                .dom_click_listeners
                .get(&node)
                .cloned()
                .unwrap_or_default();
            if let Some(handler) = self.dom_onclick.get(&node).cloned() {
                handlers.push(handler);
            }
            if handlers.is_empty() {
                continue;
            }
            handled = true;
            let receiver = self.dom_element_object(node)?;
            self.prepare_event_callback(event, receiver, 3)?;
            for handler in handlers {
                self.call_event_handler(handler, receiver, event, &mut steps)?;
            }
            if self.event_propagation_stopped(event)? {
                break;
            }
        }
        self.finish_event_dispatch(event)?;
        Ok(handled)
    }

    /// Bounded synchronous EventTarget.dispatchEvent for original DOM element
    /// listeners, including click listeners installed by the existing API.
    /// No default browser activation occurs.
    fn dispatch_custom_event(&mut self, event: ObjectId, path: &[usize]) -> Result<bool, JsError> {
        if self.dom_dispatch_depth >= 16 {
            return Err(JsError::execution_limit(
                "event dispatch recursion budget exceeded",
            ));
        }
        if !self
            .object(event)?
            .properties
            .get("__initialized")
            .is_some_and(JsValue::is_truthy)
        {
            return Err(JsError::type_error("event is not initialized"));
        }
        if matches!(
            self.object(event)?.properties.get("__dispatching"),
            Some(JsValue::Boolean(true))
        ) {
            return Err(JsError::type_error("event is already being dispatched"));
        }
        let event_type = self
            .object(event)?
            .properties
            .get("type")
            .cloned()
            .unwrap_or(JsValue::Undefined)
            .to_js_string();
        let bubbles = self
            .object(event)?
            .properties
            .get("bubbles")
            .is_some_and(JsValue::is_truthy);
        if let Some(target) = path.first().copied() {
            let object = self.dom_any_node_object(target)?;
            self.object_mut(event)?
                .properties
                .insert("target".into(), JsValue::Object(object));
        }
        self.object_mut(event)?
            .properties
            .insert("__dispatching".into(), JsValue::Boolean(true));
        self.object_mut(event)?
            .properties
            .insert("__stopPropagation".into(), JsValue::Boolean(false));
        self.dom_dispatch_depth += 1;
        let result = self.dispatch_custom_event_inner(event, path, &event_type, bubbles);
        self.dom_dispatch_depth -= 1;
        self.object_mut(event)?
            .properties
            .insert("__dispatching".into(), JsValue::Boolean(false));
        self.finish_event_dispatch(event)?;
        result?;
        let canceled = self
            .object(event)?
            .properties
            .get("defaultPrevented")
            .is_some_and(JsValue::is_truthy);
        Ok(!canceled)
    }

    fn dispatch_custom_event_inner(
        &mut self,
        event: ObjectId,
        path: &[usize],
        event_type: &str,
        bubbles: bool,
    ) -> Result<(), JsError> {
        let Some(&target) = path.first() else {
            return Ok(());
        };
        let mut steps = 0usize;
        for &node in path.iter().take(64).skip(1).rev() {
            let mut handlers = self
                .dom_typed_listeners
                .get(&(node, event_type.into(), true))
                .cloned()
                .unwrap_or_default();
            if event_type == "click" {
                handlers.extend(
                    self.dom_click_capture_listeners
                        .get(&node)
                        .cloned()
                        .unwrap_or_default(),
                );
            }
            if !handlers.is_empty() {
                let receiver = self.dom_element_object(node)?;
                self.prepare_event_callback(event, receiver, 1)?;
                for handler in handlers {
                    self.call_event_handler(handler, receiver, event, &mut steps)?;
                }
            }
            if self.event_propagation_stopped(event)? {
                return Ok(());
            }
        }
        let receiver = self.dom_element_object(target)?;
        let mut target_handlers = self
            .dom_typed_listeners
            .get(&(target, event_type.into(), true))
            .cloned()
            .unwrap_or_default();
        if event_type == "click" {
            target_handlers.extend(
                self.dom_click_capture_listeners
                    .get(&target)
                    .cloned()
                    .unwrap_or_default(),
            );
        }
        target_handlers.extend(
            self.dom_typed_listeners
                .get(&(target, event_type.into(), false))
                .cloned()
                .unwrap_or_default(),
        );
        if event_type == "click" {
            target_handlers.extend(
                self.dom_click_listeners
                    .get(&target)
                    .cloned()
                    .unwrap_or_default(),
            );
            if let Some(handler) = self.dom_onclick.get(&target).cloned() {
                target_handlers.push(handler);
            }
        }
        self.prepare_event_callback(event, receiver, 2)?;
        for handler in target_handlers {
            self.call_event_handler(handler, receiver, event, &mut steps)?;
        }
        if !bubbles || self.event_propagation_stopped(event)? {
            return Ok(());
        }
        for &node in path.iter().take(64).skip(1) {
            let mut handlers = self
                .dom_typed_listeners
                .get(&(node, event_type.into(), false))
                .cloned()
                .unwrap_or_default();
            if event_type == "click" {
                handlers.extend(
                    self.dom_click_listeners
                        .get(&node)
                        .cloned()
                        .unwrap_or_default(),
                );
                if let Some(handler) = self.dom_onclick.get(&node).cloned() {
                    handlers.push(handler);
                }
            }
            if !handlers.is_empty() {
                let receiver = self.dom_element_object(node)?;
                self.prepare_event_callback(event, receiver, 3)?;
                for handler in handlers {
                    self.call_event_handler(handler, receiver, event, &mut steps)?;
                }
            }
            if self.event_propagation_stopped(event)? {
                break;
            }
        }
        Ok(())
    }

    /// Make a previously detached subtree discoverable through getElementById
    /// within the current script, without waiting for host operation replay.
    fn attach_dom_subtree(&mut self, root: usize) {
        let mut by_parent: HashMap<usize, Vec<usize>> = HashMap::new();
        for (&node, &parent) in &self.dom_pending_parents {
            by_parent.entry(parent).or_default().push(node);
        }
        let mut visited = std::collections::HashSet::new();
        let mut stack = vec![root];
        while let Some(node) = stack.pop() {
            if !visited.insert(node) {
                continue;
            }
            self.dom_attached.insert(node);
            if let Some(id) = self.dom_pending_ids.get(&node).cloned().or_else(|| {
                self.dom_attributes
                    .get(&node)
                    .and_then(|attrs| attrs.get("id"))
                    .cloned()
            }) && !id.is_empty()
            {
                self.dom_ids.insert(id, node);
            }
            if visited.len() >= 20_000 {
                break;
            }
            if let Some(children) = by_parent.get(&node) {
                stack.extend(children.iter().copied());
            }
        }
    }

    fn detach_dom_subtree(&mut self, root: usize) -> Result<(), JsError> {
        for children in self.dom_children.values_mut() {
            children.retain(|&node| node != root);
        }
        let mut by_parent: HashMap<usize, Vec<usize>> = HashMap::new();
        for (&child, &parent) in &self.dom_pending_parents {
            by_parent.entry(parent).or_default().push(child);
        }
        let mut visited = std::collections::HashSet::new();
        let mut stack = vec![root];
        while let Some(node) = stack.pop() {
            if !visited.insert(node) {
                continue;
            }
            if visited.len() > 20_000 {
                return Err(JsError::execution_limit("DOM subtree limit exceeded"));
            }
            if let Some(children) = by_parent.get(&node) {
                stack.extend(children.iter().copied());
            }
        }
        self.dom_attached.retain(|node| !visited.contains(node));
        self.dom_ids.retain(|_, node| !visited.contains(node));
        Ok(())
    }

    fn dom_any_node_object(&mut self, node: usize) -> Result<ObjectId, JsError> {
        if let Some(&existing) = self.dom_node_objects.get(&node) {
            return Ok(existing);
        }
        if self.dom_text_nodes.contains(&node) {
            let text = self.dom_text.get(&node).cloned().unwrap_or_default();
            let object = self.allocate_object(
                ObjectKind::DomText(node),
                Some(self.object_prototype),
                HashMap::from([
                    ("data".into(), JsValue::String(text.clone())),
                    ("nodeValue".into(), JsValue::String(text.clone())),
                    ("textContent".into(), JsValue::String(text)),
                    ("nodeType".into(), JsValue::Number(3.0)),
                ]),
            )?;
            self.dom_node_objects.insert(node, object);
            return Ok(object);
        }
        self.dom_element_object(node)
    }

    fn dom_style_object(&mut self, node: usize) -> Result<ObjectId, JsError> {
        if let Some(&style) = self.dom_styles.get(&node) {
            return Ok(style);
        }
        let mut methods = HashMap::new();
        for (name, builtin, length) in [
            ("setProperty", BuiltinFunction::DomStyleSetProperty, 2.0),
            (
                "getPropertyValue",
                BuiltinFunction::DomStyleGetPropertyValue,
                1.0,
            ),
            (
                "removeProperty",
                BuiltinFunction::DomStyleRemoveProperty,
                1.0,
            ),
        ] {
            let function = self.allocate_lifecycle_method(name, builtin)?;
            self.object_mut(function)?
                .properties
                .insert("length".into(), JsValue::Number(length));
            methods.insert(name.into(), JsValue::Object(function));
        }
        let object = self.allocate_object(
            ObjectKind::DomStyle(node),
            Some(self.object_prototype),
            methods,
        )?;
        self.dom_styles.insert(node, object);
        Ok(object)
    }

    /// Read the bounded CSS declaration list without splitting semicolons
    /// inside quoted strings, escaped sequences or balanced parentheses.
    /// The CSS engine remains authoritative for selector/layout semantics.
    fn dom_style_declarations(&self, node: usize) -> Vec<(String, String)> {
        let css = self
            .dom_attributes
            .get(&node)
            .and_then(|attrs| attrs.get("style"))
            .map_or("", String::as_str);
        let mut declarations: Vec<(String, String)> = Vec::new();
        let mut chunks = Vec::new();
        let mut start = 0usize;
        let mut quote = None;
        let mut escaped = false;
        let mut depth = 0usize;
        for (index, ch) in css.char_indices() {
            if escaped {
                escaped = false;
                continue;
            }
            if ch == '\\' {
                escaped = true;
                continue;
            }
            if let Some(q) = quote {
                if ch == q {
                    quote = None;
                }
                continue;
            }
            match ch {
                '\'' | '"' => quote = Some(ch),
                '(' => depth = (depth + 1).min(32),
                ')' => depth = depth.saturating_sub(1),
                ';' if depth == 0 => {
                    chunks.push(&css[start..index]);
                    start = index + ch.len_utf8();
                    if chunks.len() >= 128 {
                        break;
                    }
                }
                _ => {}
            }
        }
        if chunks.len() < 128 {
            chunks.push(&css[start..]);
        }
        for chunk in chunks {
            let Some((name, value)) = chunk.split_once(':') else {
                continue;
            };
            let name = name.trim().to_ascii_lowercase();
            if name.is_empty() {
                continue;
            }
            let value = value.trim().to_owned();
            if let Some((_, old)) = declarations.iter_mut().find(|(k, _)| k == &name) {
                *old = value;
            } else {
                declarations.push((name, value));
            }
        }
        declarations
    }

    fn style_property_name(name: &str) -> String {
        let mut output = String::new();
        for c in name.chars() {
            if c.is_ascii_uppercase() {
                output.push('-');
                output.push(c.to_ascii_lowercase());
            } else {
                output.push(c);
            }
        }
        output
    }

    fn set_dom_style_property(
        &mut self,
        node: usize,
        name: &str,
        value: Option<String>,
    ) -> Result<String, JsError> {
        if name.len() > 128
            || name.is_empty()
            || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
        {
            return Err(JsError::type_error("unsupported CSS property name"));
        }
        let mut declarations = self.dom_style_declarations(node);
        let original = declarations
            .iter()
            .find(|(key, _)| key == name)
            .map_or_else(String::new, |(_, v)| v.clone());
        declarations.retain(|(key, _)| key != name);
        if let Some(value) = value.filter(|value| !value.trim().is_empty()) {
            declarations.push((name.into(), value));
        }
        let css = declarations
            .iter()
            .map(|(key, value)| format!("{key}: {value}"))
            .collect::<Vec<_>>()
            .join("; ");
        self.stage_dom_attribute(node, "style", css)?;
        Ok(original)
    }

    fn html_collection_nodes(&self, parent: usize) -> Vec<usize> {
        self.dom_children
            .get(&parent)
            .into_iter()
            .flatten()
            .copied()
            .filter(|node| self.dom_tags.contains_key(node))
            .collect()
    }

    fn html_collection_named(&self, parent: usize, name: &str) -> Option<usize> {
        if name.is_empty() {
            return None;
        }
        self.html_collection_nodes(parent).into_iter().find(|node| {
            self.dom_attributes.get(node).is_some_and(|attrs| {
                attrs.get("id").is_some_and(|id| id == name)
                    || attrs.get("name").is_some_and(|value| value == name)
            })
        })
    }

    fn elements_by_tag(&self, root: usize, tag: &str) -> Vec<usize> {
        let mut matches = Vec::new();
        let mut stack = self.dom_children.get(&root).cloned().unwrap_or_default();
        stack.reverse();
        let mut visited = std::collections::HashSet::new();
        while let Some(node) = stack.pop() {
            if !visited.insert(node) || visited.len() > 20_000 {
                break;
            }
            if self
                .dom_tags
                .get(&node)
                .is_some_and(|name| tag == "*" || name.eq_ignore_ascii_case(tag))
            {
                matches.push(node);
            }
            if let Some(children) = self.dom_children.get(&node) {
                stack.extend(children.iter().rev().copied());
            }
        }
        matches
    }

    fn dom_tag_collection_object(&mut self, root: usize, tag: String) -> Result<ObjectId, JsError> {
        let item = self.allocate_lifecycle_method("item", BuiltinFunction::DomNodeListItem)?;
        self.object_mut(item)?
            .properties
            .insert("length".into(), JsValue::Number(1.0));
        let collection = self.allocate_object(
            ObjectKind::DomTagCollection,
            Some(self.object_prototype),
            HashMap::from([("item".into(), JsValue::Object(item))]),
        )?;
        self.dom_tag_collections.insert(collection, (root, tag));
        Ok(collection)
    }

    fn dom_html_collection_object(&mut self, parent: usize) -> Result<ObjectId, JsError> {
        if let Some(&collection) = self.dom_html_collections.get(&parent) {
            return Ok(collection);
        }
        let item = self.allocate_lifecycle_method("item", BuiltinFunction::DomNodeListItem)?;
        self.object_mut(item)?
            .properties
            .insert("length".into(), JsValue::Number(1.0));
        let named = self
            .allocate_lifecycle_method("namedItem", BuiltinFunction::DomHtmlCollectionNamedItem)?;
        self.object_mut(named)?
            .properties
            .insert("length".into(), JsValue::Number(1.0));
        let object = self.allocate_object(
            ObjectKind::DomHtmlCollection(parent),
            Some(self.object_prototype),
            HashMap::from([
                ("item".into(), JsValue::Object(item)),
                ("namedItem".into(), JsValue::Object(named)),
            ]),
        )?;
        self.dom_html_collections.insert(parent, object);
        Ok(object)
    }

    /// Deliberately bounded ASCII subset of compound selectors; this is
    /// separate from the layout CSS parser and is not full Selectors Level 4.
    fn simple_selector_matches(&self, node: usize, compound: &str) -> bool {
        let Some(tag) = self.dom_tags.get(&node) else {
            return false;
        };
        let bytes = compound.as_bytes();
        let mut pos = 0usize;
        if pos < bytes.len() && bytes[pos] != b'#' && bytes[pos] != b'.' {
            while pos < bytes.len() && bytes[pos] != b'#' && bytes[pos] != b'.' {
                pos += 1;
            }
            let expected = &compound[..pos];
            if expected != "*" && !tag.eq_ignore_ascii_case(expected) {
                return false;
            }
        }
        if pos == 0 && bytes.first().is_none_or(|b| *b != b'#' && *b != b'.') {
            return false;
        }
        while pos < bytes.len() {
            let marker = bytes[pos];
            pos += 1;
            let start = pos;
            while pos < bytes.len() && bytes[pos] != b'#' && bytes[pos] != b'.' {
                pos += 1;
            }
            let value = &compound[start..pos];
            if value.is_empty() {
                return false;
            }
            let matches = match marker {
                b'#' => self
                    .dom_attributes
                    .get(&node)
                    .and_then(|attrs| attrs.get("id"))
                    .is_some_and(|id| id == value),
                b'.' => self
                    .dom_attributes
                    .get(&node)
                    .and_then(|attrs| attrs.get("class"))
                    .is_some_and(|class| {
                        class.split_ascii_whitespace().any(|token| token == value)
                    }),
                _ => false,
            };
            if !matches {
                return false;
            }
        }
        true
    }

    /// Parse tag.class#id, descendant and direct-child combinators plus comma
    /// lists. Fail closed on pseudo classes, attribute selectors and escapes.
    fn selector_groups(selector: &str) -> Result<Vec<Vec<(String, bool)>>, JsError> {
        if selector.len() > 256 || !selector.is_ascii() {
            return Err(JsError::type_error("unsupported CSS selector"));
        }
        let mut groups = Vec::new();
        for group in selector.split(',') {
            if groups.len() >= 8 {
                return Err(JsError::execution_limit("selector groups exceeded"));
            }
            let spaced = group.replace('>', " > ");
            let mut sequence: Vec<(String, bool)> = Vec::new();
            let mut direct = false;
            for word in spaced.split_ascii_whitespace() {
                if word == ">" {
                    if sequence.is_empty() || direct {
                        return Err(JsError::type_error("invalid child combinator"));
                    }
                    direct = true;
                    continue;
                }
                let bytes = word.as_bytes();
                if bytes.is_empty()
                    || !bytes.iter().all(|b| {
                        b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'#' | b'.' | b'*')
                    })
                    || word[1..].contains('*')
                    || (word.contains('*') && word != "*")
                    || bytes.last().is_some_and(|b| *b == b'#' || *b == b'.')
                    || bytes.windows(2).any(|pair| {
                        matches!(
                            pair,
                            [b'#', b'#'] | [b'#', b'.'] | [b'.', b'#'] | [b'.', b'.']
                        )
                    })
                    || sequence.len() >= 16
                {
                    return Err(JsError::type_error("unsupported compound CSS selector"));
                }
                sequence.push((word.into(), direct));
                direct = false;
            }
            if sequence.is_empty() || direct {
                return Err(JsError::type_error("empty or unfinished CSS selector"));
            }
            groups.push(sequence);
        }
        Ok(groups)
    }

    fn selector_chain_matches(
        &self,
        node: usize,
        root: usize,
        sequence: &[(String, bool)],
    ) -> bool {
        let Some((last, _)) = sequence.last() else {
            return false;
        };
        if !self.simple_selector_matches(node, last) {
            return false;
        }
        let mut current = node;
        for part in (1..sequence.len()).rev() {
            let (wanted, _) = &sequence[part - 1];
            let direct = sequence[part].1;
            let mut ancestor = self.dom_pending_parents.get(&current).copied();
            let mut found = None;
            let mut checked = 0usize;
            while let Some(candidate) = ancestor {
                checked += 1;
                if checked > 256 {
                    return false;
                }
                if self.simple_selector_matches(candidate, wanted) {
                    found = Some(candidate);
                    break;
                }
                if candidate == root || direct {
                    break;
                }
                ancestor = self.dom_pending_parents.get(&candidate).copied();
            }
            let Some(match_node) = found else {
                return false;
            };
            current = match_node;
        }
        true
    }

    fn query_descendants(&self, root: usize, selector: &str) -> Result<Vec<usize>, JsError> {
        let groups = Self::selector_groups(selector)?;
        let mut matches = Vec::new();
        let mut stack = self.dom_children.get(&root).cloned().unwrap_or_default();
        stack.reverse();
        let mut visited = std::collections::HashSet::new();
        while let Some(node) = stack.pop() {
            if !visited.insert(node) || visited.len() > 20_000 {
                break;
            }
            if groups
                .iter()
                .any(|group| self.selector_chain_matches(node, root, group))
            {
                matches.push(node);
                if matches.len() >= 4096 {
                    break;
                }
            }
            if let Some(children) = self.dom_children.get(&node) {
                stack.extend(children.iter().rev().copied());
            }
        }
        Ok(matches)
    }

    fn static_node_list(&mut self, nodes: Vec<usize>) -> Result<ObjectId, JsError> {
        let item = self.allocate_lifecycle_method("item", BuiltinFunction::DomNodeListItem)?;
        self.object_mut(item)?
            .properties
            .insert("length".into(), JsValue::Number(1.0));
        let list = self.allocate_object(
            ObjectKind::DomQueryNodeList,
            Some(self.object_prototype),
            HashMap::from([("item".into(), JsValue::Object(item))]),
        )?;
        self.dom_query_lists.insert(list, nodes);
        Ok(list)
    }

    fn dom_node_list_object(&mut self, node: usize) -> Result<ObjectId, JsError> {
        if let Some(&existing) = self.dom_node_lists.get(&node) {
            return Ok(existing);
        }
        let item = self.allocate_lifecycle_method("item", BuiltinFunction::DomNodeListItem)?;
        self.object_mut(item)?
            .properties
            .insert("length".into(), JsValue::Number(1.0));
        let object = self.allocate_object(
            ObjectKind::DomNodeList(node),
            Some(self.object_prototype),
            HashMap::from([("item".into(), JsValue::Object(item))]),
        )?;
        self.dom_node_lists.insert(node, object);
        Ok(object)
    }

    fn stage_dom_move(&mut self, parent: usize, child: usize, reference: Option<usize>) {
        for children in self.dom_children.values_mut() {
            children.retain(|&existing| existing != child);
        }
        let siblings = self.dom_children.entry(parent).or_default();
        let position = reference.and_then(|ref_id| siblings.iter().position(|&n| n == ref_id));
        if let Some(position) = position {
            siblings.insert(position, child);
        } else {
            siblings.push(child);
        }
        self.dom_pending_parents.insert(child, parent);
    }

    fn dom_class_list_object(&mut self, node: usize) -> Result<ObjectId, JsError> {
        if let Some(&object) = self.dom_class_lists.get(&node) {
            return Ok(object);
        }
        let mut methods = HashMap::new();
        for (name, builtin, length) in [
            ("add", BuiltinFunction::DomClassAdd, 1.0),
            ("remove", BuiltinFunction::DomClassRemove, 1.0),
            ("contains", BuiltinFunction::DomClassContains, 1.0),
            ("toggle", BuiltinFunction::DomClassToggle, 1.0),
        ] {
            let function = self.allocate_lifecycle_method(name, builtin)?;
            self.object_mut(function)?
                .properties
                .insert("length".into(), JsValue::Number(length));
            methods.insert(name.to_owned(), JsValue::Object(function));
        }
        let list = self.allocate_object(
            ObjectKind::DomClassList(node),
            Some(self.object_prototype),
            methods,
        )?;
        self.dom_class_lists.insert(node, list);
        Ok(list)
    }

    fn class_tokens(&self, node: usize) -> Vec<String> {
        self.dom_attributes
            .get(&node)
            .and_then(|attrs| attrs.get("class"))
            .map(|value| value.split_ascii_whitespace().map(str::to_owned).collect())
            .unwrap_or_default()
    }

    fn stage_dom_attribute(
        &mut self,
        node: usize,
        name: &str,
        value: String,
    ) -> Result<(), JsError> {
        if self.dom_operations.len() >= 256 || value.len() > 64 * 1024 {
            return Err(JsError::execution_limit(
                "DOM attribute mutation budget exceeded",
            ));
        }
        self.dom_attributes
            .entry(node)
            .or_default()
            .insert(name.to_owned(), value.clone());
        self.dom_operations.push(DomOperation::SetAttribute {
            node,
            name: name.to_owned(),
            value,
        });
        Ok(())
    }

    fn dom_element_object(&mut self, node: usize) -> Result<ObjectId, JsError> {
        if let Some(&id) = self.dom_node_objects.get(&node) {
            return Ok(id);
        }
        let id = self
            .dom_ids
            .iter()
            .find_map(|(id, &value)| (value == node).then_some(id.clone()))
            .or_else(|| self.dom_pending_ids.get(&node).cloned())
            .unwrap_or_default();
        let listener = self
            .allocate_lifecycle_method("addEventListener", BuiltinFunction::DomAddEventListener)?;
        let remove_listener = self.allocate_lifecycle_method(
            "removeEventListener",
            BuiltinFunction::DomRemoveEventListener,
        )?;
        let append =
            self.allocate_lifecycle_method("appendChild", BuiltinFunction::DomAppendChild)?;
        let insert =
            self.allocate_lifecycle_method("insertBefore", BuiltinFunction::DomInsertBefore)?;
        let remove =
            self.allocate_lifecycle_method("removeChild", BuiltinFunction::DomRemoveChild)?;
        let replace =
            self.allocate_lifecycle_method("replaceChild", BuiltinFunction::DomReplaceChild)?;
        let remove_self =
            self.allocate_lifecycle_method("remove", BuiltinFunction::DomRemoveSelf)?;
        let dispatch =
            self.allocate_lifecycle_method("dispatchEvent", BuiltinFunction::DomDispatchEvent)?;
        self.object_mut(dispatch)?
            .properties
            .insert("length".into(), JsValue::Number(1.0));
        let click = self.allocate_lifecycle_method("click", BuiltinFunction::DomClick)?;
        self.object_mut(click)?
            .properties
            .insert("length".into(), JsValue::Number(0.0));
        let contains = self.allocate_lifecycle_method("contains", BuiltinFunction::DomContains)?;
        self.object_mut(contains)?
            .properties
            .insert("length".into(), JsValue::Number(1.0));
        let set_attr =
            self.allocate_lifecycle_method("setAttribute", BuiltinFunction::DomSetAttribute)?;
        let get_attr =
            self.allocate_lifecycle_method("getAttribute", BuiltinFunction::DomGetAttribute)?;
        let has_attr =
            self.allocate_lifecycle_method("hasAttribute", BuiltinFunction::DomHasAttribute)?;
        self.object_mut(has_attr)?
            .properties
            .insert("length".into(), JsValue::Number(1.0));
        let has_attrs =
            self.allocate_lifecycle_method("hasAttributes", BuiltinFunction::DomHasAttributes)?;
        self.object_mut(has_attrs)?
            .properties
            .insert("length".into(), JsValue::Number(0.0));
        let by_tag = self.allocate_lifecycle_method(
            "getElementsByTagName",
            BuiltinFunction::DomGetElementsByTagName,
        )?;
        self.object_mut(by_tag)?
            .properties
            .insert("length".into(), JsValue::Number(1.0));
        let query =
            self.allocate_lifecycle_method("querySelector", BuiltinFunction::DomQuerySelector)?;
        let query_all = self
            .allocate_lifecycle_method("querySelectorAll", BuiltinFunction::DomQuerySelectorAll)?;
        self.object_mut(query)?
            .properties
            .insert("length".into(), JsValue::Number(1.0));
        self.object_mut(query_all)?
            .properties
            .insert("length".into(), JsValue::Number(1.0));
        let matches = self.allocate_lifecycle_method("matches", BuiltinFunction::DomMatches)?;
        let closest = self.allocate_lifecycle_method("closest", BuiltinFunction::DomClosest)?;
        self.object_mut(matches)?
            .properties
            .insert("length".into(), JsValue::Number(1.0));
        self.object_mut(closest)?
            .properties
            .insert("length".into(), JsValue::Number(1.0));
        let remove_attr =
            self.allocate_lifecycle_method("removeAttribute", BuiltinFunction::DomRemoveAttribute)?;
        let element = self.allocate_object(
            ObjectKind::DomElement(node),
            Some(self.object_prototype),
            HashMap::from([
                ("id".into(), JsValue::String(id)),
                (
                    "textContent".into(),
                    JsValue::String(self.dom_text.get(&node).cloned().unwrap_or_default()),
                ),
                ("addEventListener".into(), JsValue::Object(listener)),
                (
                    "removeEventListener".into(),
                    JsValue::Object(remove_listener),
                ),
                ("appendChild".into(), JsValue::Object(append)),
                ("insertBefore".into(), JsValue::Object(insert)),
                ("removeChild".into(), JsValue::Object(remove)),
                ("replaceChild".into(), JsValue::Object(replace)),
                ("remove".into(), JsValue::Object(remove_self)),
                ("contains".into(), JsValue::Object(contains)),
                ("click".into(), JsValue::Object(click)),
                ("dispatchEvent".into(), JsValue::Object(dispatch)),
                ("setAttribute".into(), JsValue::Object(set_attr)),
                ("getAttribute".into(), JsValue::Object(get_attr)),
                ("hasAttribute".into(), JsValue::Object(has_attr)),
                ("hasAttributes".into(), JsValue::Object(has_attrs)),
                ("getElementsByTagName".into(), JsValue::Object(by_tag)),
                ("querySelector".into(), JsValue::Object(query)),
                ("querySelectorAll".into(), JsValue::Object(query_all)),
                ("matches".into(), JsValue::Object(matches)),
                ("closest".into(), JsValue::Object(closest)),
                ("removeAttribute".into(), JsValue::Object(remove_attr)),
            ]),
        )?;
        self.dom_node_objects.insert(node, element);
        Ok(element)
    }

    fn prepare_event_callback(
        &mut self,
        event: ObjectId,
        receiver: ObjectId,
        phase: u8,
    ) -> Result<(), JsError> {
        self.object_mut(event)?
            .properties
            .insert("currentTarget".into(), JsValue::Object(receiver));
        self.object_mut(event)?
            .properties
            .insert("eventPhase".into(), JsValue::Number(f64::from(phase)));
        Ok(())
    }

    fn call_event_handler(
        &mut self,
        handler: JsValue,
        receiver: ObjectId,
        event: ObjectId,
        steps: &mut usize,
    ) -> Result<(), JsError> {
        match self.call_value(
            handler,
            JsValue::Object(receiver),
            vec![JsValue::Object(event)],
            steps,
            1,
        )? {
            CallOutcome::Value(_) => Ok(()),
            CallOutcome::Thrown(value) => {
                Err(JsError::exception(self.describe_thrown_value(&value)))
            }
        }
    }

    fn event_propagation_stopped(&self, event: ObjectId) -> Result<bool, JsError> {
        Ok(matches!(
            self.object(event)?.properties.get("__stopPropagation"),
            Some(JsValue::Boolean(true))
        ))
    }

    fn finish_event_dispatch(&mut self, event: ObjectId) -> Result<(), JsError> {
        self.object_mut(event)?
            .properties
            .insert("currentTarget".into(), JsValue::Null);
        self.object_mut(event)?
            .properties
            .insert("eventPhase".into(), JsValue::Number(0.0));
        // DOM event callbacks constitute one task. Flush queued microtasks
        // after all listeners have run, not between capture and bubble.
        let _ = self.drain_microtasks();
        Ok(())
    }

    pub fn has_dom_click_listener(&self, node: usize) -> bool {
        self.dom_click_listeners
            .get(&node)
            .is_some_and(|v| !v.is_empty())
            || self
                .dom_click_capture_listeners
                .get(&node)
                .is_some_and(|v| !v.is_empty())
            || self.dom_onclick.contains_key(&node)
    }

    pub fn eval_script(&mut self, source: &str) -> Result<JsValue, JsError> {
        let script = compile_script(source)?;
        self.execute(&script)
    }

    pub fn execute(&mut self, script: &CompiledScript) -> Result<JsValue, JsError> {
        let mut steps = 0usize;
        let outcome = self.run_code(&script.code, self.global_env, &mut steps, 0);
        let result = match outcome {
            Ok(RunOutcome::Complete(value) | RunOutcome::Returned(value)) => Ok(value),
            Ok(RunOutcome::Thrown(value)) => {
                Err(JsError::exception(self.describe_thrown_value(&value)))
            }
            Ok(RunOutcome::Break) => Err(JsError::type_error("break escaped script control flow")),
            Ok(RunOutcome::Continue) => {
                Err(JsError::type_error("continue escaped script control flow"))
            }
            Err(error) => Err(error),
        };
        // End-of-script microtask checkpoint; never run inline inside queueMicrotask().
        let _ = self.drain_microtasks();
        result
    }

    fn run_code(
        &mut self,
        code: &[Instruction],
        start_env: EnvironmentId,
        steps: &mut usize,
        call_depth: usize,
    ) -> Result<RunOutcome, JsError> {
        let mut stack = Vec::new();
        let mut completion = JsValue::Undefined;
        let mut env = start_env;
        let mut ip = 0usize;

        while let Some(instruction) = code.get(ip) {
            *steps = steps.saturating_add(1);
            if *steps > self.instruction_budget {
                return Err(JsError::execution_limit(format!(
                    "script exceeded instruction budget of {}",
                    self.instruction_budget
                )));
            }

            match instruction {
                Instruction::Push(value) => stack.push(value.clone()),
                Instruction::Load(name) => {
                    stack.push(self.load_binding(env, name)?.clone());
                }
                Instruction::TypeofBinding(name) => {
                    let value = match self.load_binding(env, name) {
                        Ok(value) => value.clone(),
                        Err(error) if error.kind == JsErrorKind::Reference => JsValue::Undefined,
                        Err(error) => return Err(error),
                    };
                    stack.push(self.typeof_value(&value)?);
                }
                Instruction::Declare {
                    name,
                    kind,
                    has_initializer,
                } => {
                    let value = stack
                        .pop()
                        .expect("compiler must push declaration initializer");
                    self.declare_binding(env, name, *kind, *has_initializer, value)?;
                }
                Instruction::Assign(name) => {
                    let value = stack
                        .last()
                        .expect("compiler must leave assignment value on stack")
                        .clone();
                    self.assign_binding(env, name, value)?;
                }
                Instruction::UpdateBinding { name, op, prefix } => {
                    let previous = self.load_binding(env, name)?.clone();
                    let updated = apply_update(*op, &previous);
                    self.assign_binding(env, name, updated.clone())?;
                    stack.push(if *prefix { updated } else { previous });
                }
                Instruction::UpdateProperty { op, prefix } => {
                    let key = stack.pop().expect("compiler must push update property key");
                    let target = stack
                        .pop()
                        .expect("compiler must push update property target");
                    let key = to_property_key(key);
                    let previous = self.get_property(&target, &key)?;
                    let updated = apply_update(*op, &previous);
                    self.set_property(&target, &key, updated.clone())?;
                    stack.push(if *prefix { updated } else { previous });
                }
                Instruction::CreateObject(prototype_setters) => {
                    let mut entries = Vec::with_capacity(prototype_setters.len());
                    for prototype_setter in prototype_setters.iter().rev().copied() {
                        let value = stack
                            .pop()
                            .expect("compiler must push object property value");
                        let key = stack.pop().expect("compiler must push object property key");
                        entries.push((to_property_key(key), value, prototype_setter));
                    }
                    entries.reverse();

                    let mut properties = HashMap::new();
                    let mut prototype = Some(self.object_prototype);
                    for (key, value, prototype_setter) in entries {
                        if prototype_setter {
                            match value {
                                JsValue::Object(id) => prototype = Some(id),
                                JsValue::Null => prototype = None,
                                _ => {}
                            }
                        } else {
                            properties.insert(key, value);
                        }
                    }

                    let id = self.allocate_object(ObjectKind::Ordinary, prototype, properties)?;
                    stack.push(JsValue::Object(id));
                }
                Instruction::CreateArray(present) => {
                    let mut values = Vec::with_capacity(present.len());
                    for _ in 0..present.len() {
                        values.push(
                            stack
                                .pop()
                                .expect("compiler must push every array element slot"),
                        );
                    }
                    values.reverse();

                    let mut properties = HashMap::new();
                    for (index, (value, present)) in
                        values.into_iter().zip(present.iter().copied()).enumerate()
                    {
                        if present {
                            properties.insert(index.to_string(), value);
                        }
                    }
                    properties.insert("length".into(), JsValue::Number(present.len() as f64));
                    let id = self.allocate_object(
                        ObjectKind::Array,
                        Some(self.array_prototype),
                        properties,
                    )?;
                    stack.push(JsValue::Object(id));
                }
                Instruction::CreateFunction(template) => {
                    let id = self.allocate_function(template.clone(), env)?;
                    stack.push(JsValue::Object(id));
                }
                Instruction::GetProperty => {
                    let key = stack
                        .pop()
                        .expect("compiler must push computed property key");
                    let target = stack.pop().expect("compiler must push property target");
                    let key = to_property_key(key);
                    stack.push(self.get_property(&target, &key)?);
                }
                Instruction::SetProperty => {
                    let value = stack
                        .pop()
                        .expect("compiler must push property assignment value");
                    let key = stack
                        .pop()
                        .expect("compiler must push property assignment key");
                    let target = stack
                        .pop()
                        .expect("compiler must push property assignment target");
                    let key = to_property_key(key);
                    self.set_property(&target, &key, value.clone())?;
                    stack.push(value);
                }
                Instruction::Call {
                    argument_count,
                    has_receiver,
                } => {
                    let mut arguments = Vec::with_capacity(*argument_count);
                    for _ in 0..*argument_count {
                        arguments.push(
                            stack
                                .pop()
                                .expect("compiler must push every function argument"),
                        );
                    }
                    arguments.reverse();
                    let callee = stack.pop().expect("compiler must push function callee");
                    let this_value = if *has_receiver {
                        stack
                            .pop()
                            .expect("compiler must preserve method receiver before callee")
                    } else {
                        JsValue::Object(self.global_object)
                    };
                    match self.call_value(callee, this_value, arguments, steps, call_depth + 1)? {
                        CallOutcome::Value(value) => stack.push(value),
                        CallOutcome::Thrown(value) => return Ok(RunOutcome::Thrown(value)),
                    }
                }
                Instruction::Construct(argument_count) => {
                    let mut arguments = Vec::with_capacity(*argument_count);
                    for _ in 0..*argument_count {
                        arguments.push(
                            stack
                                .pop()
                                .expect("compiler must push every constructor argument"),
                        );
                    }
                    arguments.reverse();
                    let callee = stack.pop().expect("compiler must push constructor callee");
                    match self.construct_value(callee, arguments, steps, call_depth + 1)? {
                        CallOutcome::Value(value) => stack.push(value),
                        CallOutcome::Thrown(value) => return Ok(RunOutcome::Thrown(value)),
                    }
                }
                Instruction::Unary(op) => {
                    let value = stack.pop().expect("compiler must push unary operand");
                    let result = if *op == UnaryOp::Typeof {
                        self.typeof_value(&value)?
                    } else {
                        apply_unary(*op, value)
                    };
                    stack.push(result);
                }
                Instruction::Binary(op) => {
                    let right = stack.pop().expect("compiler must push right operand");
                    let left = stack.pop().expect("compiler must push left operand");
                    match self.binary_with_coercion(*op, left, right, steps, call_depth)? {
                        CallOutcome::Value(value) => stack.push(value),
                        CallOutcome::Thrown(value) => return Ok(RunOutcome::Thrown(value)),
                    }
                }
                Instruction::Dup => {
                    let value = stack
                        .last()
                        .expect("compiler must leave a value to duplicate")
                        .clone();
                    stack.push(value);
                }
                Instruction::Pop => {
                    stack.pop().expect("compiler must leave a value to pop");
                }
                Instruction::EnterScope => {
                    env = self.allocate_environment(Some(env), EnvironmentKind::Block)?;
                }
                Instruction::ExitScope => {
                    env = self.parent_environment(env)?;
                }
                Instruction::UnwindScopes(count) => {
                    for _ in 0..*count {
                        env = self.parent_environment(env)?;
                    }
                }
                Instruction::Jump(target) => {
                    debug_assert!(*target <= code.len());
                    ip = *target;
                    continue;
                }
                Instruction::JumpIfFalse(target) => {
                    let value = stack
                        .last()
                        .expect("compiler must leave branch condition on stack");
                    if !value.is_truthy() {
                        debug_assert!(*target <= code.len());
                        ip = *target;
                        continue;
                    }
                }
                Instruction::JumpIfTrue(target) => {
                    let value = stack
                        .last()
                        .expect("compiler must leave branch condition on stack");
                    if value.is_truthy() {
                        debug_assert!(*target <= code.len());
                        ip = *target;
                        continue;
                    }
                }
                Instruction::Try {
                    template,
                    break_target,
                    break_unwind,
                    continue_target,
                    continue_unwind,
                } => match self.run_try(template, env, steps, call_depth)? {
                    RunOutcome::Complete(value) => completion = value,
                    RunOutcome::Break => {
                        let Some(target) = break_target else {
                            return Ok(RunOutcome::Break);
                        };
                        debug_assert!(*target != usize::MAX && *target <= code.len());
                        for _ in 0..*break_unwind {
                            env = self.parent_environment(env)?;
                        }
                        ip = *target;
                        continue;
                    }
                    RunOutcome::Continue => {
                        let Some(target) = continue_target else {
                            return Ok(RunOutcome::Continue);
                        };
                        debug_assert!(*target != usize::MAX && *target <= code.len());
                        for _ in 0..*continue_unwind {
                            env = self.parent_environment(env)?;
                        }
                        ip = *target;
                        continue;
                    }
                    outcome => return Ok(outcome),
                },
                Instruction::Throw => {
                    let value = stack.pop().expect("compiler must push thrown value");
                    return Ok(RunOutcome::Thrown(value));
                }
                Instruction::BreakSignal => return Ok(RunOutcome::Break),
                Instruction::ContinueSignal => return Ok(RunOutcome::Continue),
                Instruction::Return => {
                    let value = stack
                        .pop()
                        .expect("compiler must push function return value");
                    return Ok(RunOutcome::Returned(value));
                }
                Instruction::SetCompletion => {
                    completion = stack.pop().expect("compiler must push completion value");
                }
                Instruction::Halt => break,
            }

            ip = ip.saturating_add(1);
        }

        debug_assert!(stack.is_empty());
        Ok(RunOutcome::Complete(completion))
    }

    fn run_try(
        &mut self,
        template: &TryTemplate,
        env: EnvironmentId,
        steps: &mut usize,
        call_depth: usize,
    ) -> Result<RunOutcome, JsError> {
        let mut outcome =
            self.run_code_catching_runtime_errors(&template.try_code, env, steps, call_depth)?;

        if let RunOutcome::Thrown(value) = outcome.clone()
            && let Some(catch_code) = &template.catch_code
        {
            let catch_env = self.allocate_environment(Some(env), EnvironmentKind::Block)?;
            if let Some(param) = &template.catch_param {
                self.environments[catch_env.0].bindings.insert(
                    param.clone(),
                    Binding {
                        value,
                        mutable: true,
                        declaration_kind: VariableKind::Let,
                    },
                );
            }
            outcome =
                self.run_code_catching_runtime_errors(catch_code, catch_env, steps, call_depth)?;
        }

        if let Some(finally_code) = &template.finally_code {
            let finally_outcome =
                self.run_code_catching_runtime_errors(finally_code, env, steps, call_depth)?;
            if !matches!(finally_outcome, RunOutcome::Complete(_)) {
                outcome = finally_outcome;
            }
        }

        Ok(outcome)
    }

    fn run_code_catching_runtime_errors(
        &mut self,
        code: &[Instruction],
        env: EnvironmentId,
        steps: &mut usize,
        call_depth: usize,
    ) -> Result<RunOutcome, JsError> {
        match self.run_code(code, env, steps, call_depth) {
            Ok(outcome) => Ok(outcome),
            Err(error) => {
                if let Some(value) = self.error_object_from_runtime_error(&error)? {
                    Ok(RunOutcome::Thrown(value))
                } else {
                    Err(error)
                }
            }
        }
    }

    fn call_value(
        &mut self,
        callee: JsValue,
        this_value: JsValue,
        arguments: Vec<JsValue>,
        steps: &mut usize,
        call_depth: usize,
    ) -> Result<CallOutcome, JsError> {
        if call_depth > self.call_depth_budget {
            return Err(JsError::execution_limit(format!(
                "script exceeded call depth budget of {}",
                self.call_depth_budget
            )));
        }

        let JsValue::Object(id) = callee else {
            return Err(JsError::type_error("value is not callable"));
        };
        let function = self
            .object(id)?
            .function
            .clone()
            .ok_or_else(|| JsError::type_error("value is not callable"))?;

        match function.implementation {
            FunctionImplementation::Builtin(builtin) => {
                self.call_builtin(builtin, this_value, arguments)
            }
            FunctionImplementation::User { template, closure } => self.call_user_function(
                id, template, closure, this_value, arguments, steps, call_depth,
            ),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn call_user_function(
        &mut self,
        function_id: ObjectId,
        template: Arc<FunctionTemplate>,
        closure: EnvironmentId,
        this_value: JsValue,
        arguments: Vec<JsValue>,
        steps: &mut usize,
        call_depth: usize,
    ) -> Result<CallOutcome, JsError> {
        let function_env = self.allocate_environment(Some(closure), EnvironmentKind::Function)?;

        self.environments[function_env.0].bindings.insert(
            "this".into(),
            Binding {
                value: this_value,
                mutable: false,
                declaration_kind: VariableKind::Const,
            },
        );

        let arguments_object = self.allocate_arguments_object(&arguments)?;
        self.environments[function_env.0].bindings.insert(
            "arguments".into(),
            Binding {
                value: JsValue::Object(arguments_object),
                mutable: true,
                declaration_kind: VariableKind::Var,
            },
        );

        if let Some(name) = &template.name {
            self.environments[function_env.0].bindings.insert(
                name.clone(),
                Binding {
                    value: JsValue::Object(function_id),
                    mutable: false,
                    declaration_kind: VariableKind::Const,
                },
            );
        }

        for (index, parameter) in template.params.iter().enumerate() {
            let value = arguments.get(index).cloned().unwrap_or(JsValue::Undefined);
            self.environments[function_env.0].bindings.insert(
                parameter.clone(),
                Binding {
                    value,
                    mutable: true,
                    declaration_kind: VariableKind::Var,
                },
            );
        }

        match self.run_code(&template.code, function_env, steps, call_depth)? {
            RunOutcome::Returned(value) => Ok(CallOutcome::Value(value)),
            RunOutcome::Complete(_) => Ok(CallOutcome::Value(JsValue::Undefined)),
            RunOutcome::Thrown(value) => Ok(CallOutcome::Thrown(value)),
            RunOutcome::Break => Err(JsError::type_error("break escaped a function body")),
            RunOutcome::Continue => Err(JsError::type_error("continue escaped a function body")),
        }
    }

    fn call_builtin(
        &mut self,
        builtin: BuiltinFunction,
        this_value: JsValue,
        arguments: Vec<JsValue>,
    ) -> Result<CallOutcome, JsError> {
        match builtin {
            BuiltinFunction::ObjectPrototypeValueOf => {
                return Ok(CallOutcome::Value(this_value));
            }
            BuiltinFunction::ObjectPrototypeToString => {
                let tag = if let JsValue::Object(id) = this_value {
                    if self.object(id)?.kind == ObjectKind::Array {
                        "Array"
                    } else {
                        "Object"
                    }
                } else {
                    "Object"
                };
                return Ok(CallOutcome::Value(JsValue::String(format!(
                    "[object {tag}]"
                ))));
            }
            BuiltinFunction::BoxedPrimitiveValueOf => {
                let JsValue::Object(id) = this_value else {
                    return Err(JsError::type_error("valueOf called on non-boxed value"));
                };
                return Ok(CallOutcome::Value(
                    self.boxed_values
                        .get(&id)
                        .ok_or_else(|| JsError::type_error("incompatible valueOf receiver"))?
                        .clone(),
                ));
            }
            BuiltinFunction::BoxedPrimitiveToString => {
                let JsValue::Object(id) = this_value else {
                    return Err(JsError::type_error("toString called on non-boxed value"));
                };
                return Ok(CallOutcome::Value(JsValue::String(
                    self.boxed_values
                        .get(&id)
                        .ok_or_else(|| JsError::type_error("incompatible toString receiver"))?
                        .to_js_string(),
                )));
            }
            BuiltinFunction::StringCharAt => {
                let primitive = match &this_value {
                    JsValue::String(text) => JsValue::String(text.clone()),
                    JsValue::Object(id) => self
                        .boxed_values
                        .get(id)
                        .cloned()
                        .unwrap_or_else(|| this_value.clone()),
                    JsValue::Null | JsValue::Undefined => {
                        return Err(JsError::type_error(
                            "String.prototype.charAt receiver is null or undefined",
                        ));
                    }
                    _ => this_value.clone(),
                };
                let text = primitive.to_js_string();
                let index = arguments.first().map(JsValue::to_number).unwrap_or(0.0);
                let index = if index.is_nan() { 0.0 } else { index.trunc() };
                if index < 0.0 || index >= text.encode_utf16().count() as f64 {
                    return Ok(CallOutcome::Value(JsValue::String(String::new())));
                }
                let units: Vec<u16> = text.encode_utf16().collect();
                let unit = units[index as usize];
                // VM strings currently use valid Unicode scalar strings rather
                // than arbitrary UTF-16 code units. A lone surrogate cannot
                // be represented exactly; reject instead of corrupting UTF-8.
                if (0xd800..=0xdfff).contains(&unit) {
                    return Err(JsError::type_error(
                        "charAt lone UTF-16 surrogate is not yet supported",
                    ));
                }
                let value = String::from_utf16(&[unit])
                    .map_err(|_| JsError::type_error("invalid charAt code unit"))?;
                return Ok(CallOutcome::Value(JsValue::String(value)));
            }
            BuiltinFunction::ArrayPush => {
                let JsValue::Object(id) = this_value else {
                    return Err(JsError::type_error("Array.push receiver must be an array"));
                };
                if self.object(id)?.kind != ObjectKind::Array {
                    return Err(JsError::type_error("Array.push receiver must be an array"));
                }
                let length = self.array_length(id)?;
                let next = length
                    .checked_add(arguments.len())
                    .ok_or_else(|| JsError::execution_limit("array size overflow"))?;
                if next > 4096 {
                    return Err(JsError::execution_limit("Array.push capacity exceeded"));
                }
                for (index, value) in arguments.into_iter().enumerate() {
                    self.object_mut(id)?
                        .properties
                        .insert((length + index).to_string(), value);
                }
                self.object_mut(id)?
                    .properties
                    .insert("length".into(), JsValue::Number(next as f64));
                return Ok(CallOutcome::Value(JsValue::Number(next as f64)));
            }
            BuiltinFunction::ArrayPop => {
                let JsValue::Object(id) = this_value else {
                    return Err(JsError::type_error("Array.pop receiver must be an array"));
                };
                if self.object(id)?.kind != ObjectKind::Array {
                    return Err(JsError::type_error("Array.pop receiver must be an array"));
                }
                let length = self.array_length(id)?;
                if length == 0 {
                    return Ok(CallOutcome::Value(JsValue::Undefined));
                }
                let key = (length - 1).to_string();
                let value = self.get_object_property(id, &key)?;
                self.object_mut(id)?.properties.remove(&key);
                self.object_mut(id)?
                    .properties
                    .insert("length".into(), JsValue::Number((length - 1) as f64));
                return Ok(CallOutcome::Value(value));
            }
            BuiltinFunction::BooleanConstructor => {
                return Ok(CallOutcome::Value(JsValue::Boolean(
                    arguments.first().is_some_and(JsValue::is_truthy),
                )));
            }
            BuiltinFunction::NumberConstructor => {
                return Ok(CallOutcome::Value(JsValue::Number(
                    arguments.first().map(JsValue::to_number).unwrap_or(0.0),
                )));
            }
            BuiltinFunction::StringConstructor => {
                return Ok(CallOutcome::Value(JsValue::String(
                    arguments
                        .first()
                        .map(JsValue::to_js_string)
                        .unwrap_or_default(),
                )));
            }
            BuiltinFunction::IsNaN => {
                return Ok(CallOutcome::Value(JsValue::Boolean(
                    arguments
                        .first()
                        .unwrap_or(&JsValue::Undefined)
                        .to_number()
                        .is_nan(),
                )));
            }
            BuiltinFunction::IsFinite => {
                return Ok(CallOutcome::Value(JsValue::Boolean(
                    arguments
                        .first()
                        .unwrap_or(&JsValue::Undefined)
                        .to_number()
                        .is_finite(),
                )));
            }
            BuiltinFunction::ArrayIsArray => {
                let is_array = match arguments.first() {
                    Some(JsValue::Object(id)) => self.object(*id)?.kind == ObjectKind::Array,
                    _ => false,
                };
                return Ok(CallOutcome::Value(JsValue::Boolean(is_array)));
            }
            BuiltinFunction::ArrayOf => {
                if arguments.len() > 4096 {
                    return Err(JsError::execution_limit("Array.of length budget exceeded"));
                }
                let mut props = HashMap::new();
                for (index, value) in arguments.iter().enumerate() {
                    props.insert(index.to_string(), value.clone());
                }
                props.insert("length".into(), JsValue::Number(arguments.len() as f64));
                let id =
                    self.allocate_object(ObjectKind::Array, Some(self.array_prototype), props)?;
                return Ok(CallOutcome::Value(JsValue::Object(id)));
            }
            BuiltinFunction::NumberIsNaN => {
                return Ok(CallOutcome::Value(JsValue::Boolean(matches!(
                    arguments.first(), Some(JsValue::Number(value)) if value.is_nan()
                ))));
            }
            BuiltinFunction::NumberIsFinite => {
                return Ok(CallOutcome::Value(JsValue::Boolean(matches!(
                    arguments.first(), Some(JsValue::Number(value)) if value.is_finite()
                ))));
            }
            BuiltinFunction::ObjectIs => {
                let first = arguments.first().unwrap_or(&JsValue::Undefined);
                let second = arguments.get(1).unwrap_or(&JsValue::Undefined);
                let equal = match (first, second) {
                    (JsValue::Number(a), JsValue::Number(b)) => {
                        (a.is_nan() && b.is_nan())
                            || a.to_bits() == b.to_bits()
                            || (*a != 0.0 && *b != 0.0 && a == b)
                    }
                    _ => strict_equal(first, second),
                };
                return Ok(CallOutcome::Value(JsValue::Boolean(equal)));
            }
            BuiltinFunction::ObjectConstructor => {
                if let Some(JsValue::Object(id)) = arguments.first() {
                    return Ok(CallOutcome::Value(JsValue::Object(*id)));
                }
                if let Some(
                    value @ (JsValue::Boolean(_) | JsValue::Number(_) | JsValue::String(_)),
                ) = arguments.first()
                {
                    let class = match value {
                        JsValue::Boolean(_) => "Boolean",
                        JsValue::Number(_) => "Number",
                        _ => "String",
                    };
                    return Ok(CallOutcome::Value(
                        self.box_primitive(value.clone(), class)?,
                    ));
                }
                let id = self.allocate_object(
                    ObjectKind::Ordinary,
                    Some(self.object_prototype),
                    HashMap::new(),
                )?;
                return Ok(CallOutcome::Value(JsValue::Object(id)));
            }
            BuiltinFunction::ArrayConstructor => {
                let mut props = HashMap::new();
                let length = if arguments.len() == 1 {
                    match &arguments[0] {
                        JsValue::Number(n) => {
                            if !n.is_finite()
                                || *n < 0.0
                                || n.fract() != 0.0
                                || *n > u32::MAX as f64
                            {
                                return Err(JsError::type_error("invalid array length"));
                            }
                            *n
                        }
                        value => {
                            props.insert("0".into(), value.clone());
                            1.0
                        }
                    }
                } else {
                    for (index, value) in arguments.iter().enumerate() {
                        props.insert(index.to_string(), value.clone());
                    }
                    arguments.len() as f64
                };
                // Protect bounded memory and Promise array-like iteration.
                if length > 4096.0 {
                    return Err(JsError::execution_limit("Array length budget exceeded"));
                }
                props.insert("length".into(), JsValue::Number(length));
                let id =
                    self.allocate_object(ObjectKind::Array, Some(self.array_prototype), props)?;
                return Ok(CallOutcome::Value(JsValue::Object(id)));
            }
            _ => {}
        }
        if builtin == BuiltinFunction::JsonParse {
            if arguments
                .get(1)
                .is_some_and(|v| !matches!(v, JsValue::Undefined))
            {
                return Err(JsError::type_error(
                    "JSON.parse reviver is not supported yet",
                ));
            }
            let source = arguments
                .first()
                .map(JsValue::to_js_string)
                .unwrap_or_else(|| "undefined".into());
            let value = match crate::json::parse(&source) {
                Ok(value) => value,
                Err(message) => {
                    let error = self.allocate_error_object(
                        "SyntaxError",
                        &message,
                        self.syntax_error_prototype,
                    )?;
                    return Ok(CallOutcome::Thrown(JsValue::Object(error)));
                }
            };
            return Ok(CallOutcome::Value(self.json_to_value(value, 0)?));
        }
        if builtin == BuiltinFunction::JsonStringify {
            if arguments
                .iter()
                .skip(1)
                .any(|v| !matches!(v, JsValue::Undefined))
            {
                return Err(JsError::type_error(
                    "JSON.stringify replacer/space not supported yet",
                ));
            }
            let value = arguments.first().unwrap_or(&JsValue::Undefined);
            let output = self.json_from_value(value, &mut Vec::new(), 0);
            return match output {
                Ok(Some(text)) => Ok(CallOutcome::Value(JsValue::String(text))),
                Ok(None) => Ok(CallOutcome::Value(JsValue::Undefined)),
                Err(message) => {
                    let error = self.allocate_error_object(
                        "TypeError",
                        &message,
                        self.type_error_prototype,
                    )?;
                    Ok(CallOutcome::Thrown(JsValue::Object(error)))
                }
            };
        }
        if builtin == BuiltinFunction::OpFetchText {
            let url = arguments
                .first()
                .map(JsValue::to_js_string)
                .unwrap_or_default();
            if url.is_empty() || url.len() > 2048 || url.chars().any(char::is_control) {
                return Err(JsError::type_error("invalid opFetchText URL"));
            }
            let Some(JsValue::Object(callback)) = arguments.get(1) else {
                return Err(JsError::type_error("opFetchText callback must be callable"));
            };
            if self.object(*callback)?.function.is_none() {
                return Err(JsError::type_error("opFetchText callback must be callable"));
            }
            if self.text_callbacks.len() >= MAX_PENDING_TEXT_REQUESTS
                || self.next_text_request_id > MAX_TOTAL_TEXT_REQUESTS
            {
                return Err(JsError::execution_limit("network task budget exceeded"));
            }
            let include_http_errors = matches!(arguments.get(2), Some(JsValue::Boolean(true)));
            let request_headers = if let Some(value) = arguments.get(3) {
                self.allowed_request_headers(value)?
            } else {
                Vec::new()
            };
            let reject_redirect = matches!(arguments.get(4), Some(JsValue::Boolean(true)));
            let id = self.next_text_request_id;
            self.next_text_request_id += 1;
            self.text_callbacks.insert(id, JsValue::Object(*callback));
            self.text_requests.push_back(TextRequest {
                id,
                url,
                include_http_errors,
                request_headers,
                reject_redirect,
            });
            return Ok(CallOutcome::Value(JsValue::Number(f64::from(id))));
        }
        if builtin == BuiltinFunction::HeadersConstructor {
            let values = self.read_headers_init(arguments.first())?;
            let id = self.make_headers(values)?;
            return Ok(CallOutcome::Value(JsValue::Object(id)));
        }
        if matches!(
            builtin,
            BuiltinFunction::HeadersGet
                | BuiltinFunction::HeadersHas
                | BuiltinFunction::HeadersSet
                | BuiltinFunction::HeadersAppend
                | BuiltinFunction::HeadersDelete
        ) {
            let JsValue::Object(id) = this_value else {
                return Err(JsError::type_error("Headers receiver must be Headers"));
            };
            if self.object(id)?.kind != ObjectKind::Headers {
                return Err(JsError::type_error("Headers receiver must be Headers"));
            }
            let name = Self::header_name(arguments.first().unwrap_or(&JsValue::Undefined))?;
            if matches!(
                builtin,
                BuiltinFunction::HeadersGet | BuiltinFunction::HeadersHas
            ) {
                let value = self.header_values.get(&id).and_then(|v| v.get(&name));
                return Ok(CallOutcome::Value(
                    if builtin == BuiltinFunction::HeadersHas {
                        JsValue::Boolean(value.is_some())
                    } else {
                        value
                            .map(|v| JsValue::String(v.clone()))
                            .unwrap_or(JsValue::Null)
                    },
                ));
            }
            if builtin == BuiltinFunction::HeadersDelete {
                if let Some(v) = self.header_values.get_mut(&id) {
                    v.remove(&name);
                }
                return Ok(CallOutcome::Value(JsValue::Undefined));
            }
            let value = Self::header_value(arguments.get(1).unwrap_or(&JsValue::Undefined))?;
            let mut updated = self.header_values.get(&id).cloned().unwrap_or_default();
            if builtin == BuiltinFunction::HeadersAppend {
                let existing = updated.entry(name).or_default();
                if !existing.is_empty() {
                    existing.push_str(", ");
                }
                existing.push_str(&value);
            } else {
                updated.insert(name, value);
            }
            Self::check_header_budget(&updated)?;
            self.header_values.insert(id, updated);
            return Ok(CallOutcome::Value(JsValue::Undefined));
        }
        if matches!(
            builtin,
            BuiltinFunction::SetTimeout
                | BuiltinFunction::ClearTimeout
                | BuiltinFunction::SetInterval
                | BuiltinFunction::ClearInterval
                | BuiltinFunction::QueueMicrotask
        ) {
            if matches!(
                builtin,
                BuiltinFunction::ClearTimeout | BuiltinFunction::ClearInterval
            ) {
                let id = arguments
                    .first()
                    .map(JsValue::to_number)
                    .unwrap_or(f64::NAN);
                if id.is_finite() && id >= 1.0 && id <= f64::from(u32::MAX) {
                    self.timers.retain(|timer| f64::from(timer.id) != id);
                }
                return Ok(CallOutcome::Value(JsValue::Undefined));
            }
            let Some(JsValue::Object(callback)) = arguments.first() else {
                return Err(JsError::type_error("setTimeout requires a function"));
            };
            if self.object(*callback)?.function.is_none() {
                return Err(JsError::type_error("setTimeout callback must be callable"));
            }
            if matches!(builtin, BuiltinFunction::QueueMicrotask) {
                if self.microtasks.len() >= MAX_PENDING_MICROTASKS
                    || self.microtasks_scheduled >= MAX_TOTAL_MICROTASKS
                {
                    return Err(JsError::execution_limit("microtask budget exceeded"));
                }
                self.microtasks_scheduled += 1;
                self.microtasks.push_back(JsValue::Object(*callback));
                return Ok(CallOutcome::Value(JsValue::Undefined));
            }
            if self.timers.len() >= MAX_PENDING_TIMERS || self.next_timer_id > MAX_TOTAL_TIMERS {
                return Err(JsError::execution_limit("timer budget exceeded"));
            }
            let raw_delay = arguments.get(1).map(JsValue::to_number).unwrap_or(0.0);
            let delay_ms = if raw_delay.is_nan() || raw_delay <= 0.0 {
                0
            } else {
                raw_delay.min(MAX_TIMER_DELAY_MS as f64) as u64
            };
            let interval = matches!(builtin, BuiltinFunction::SetInterval)
                .then(|| Duration::from_millis(delay_ms.max(MIN_INTERVAL_DELAY_MS)));
            let id = self.next_timer_id;
            self.next_timer_id += 1;
            self.timers.push(PendingTimer {
                id,
                due: Instant::now() + interval.unwrap_or(Duration::from_millis(delay_ms)),
                callback: JsValue::Object(*callback),
                arguments: arguments.into_iter().skip(2).collect(),
                interval,
            });
            return Ok(CallOutcome::Value(JsValue::Number(f64::from(id))));
        }
        if matches!(
            builtin,
            BuiltinFunction::EventStopPropagation | BuiltinFunction::EventPreventDefault
        ) {
            let JsValue::Object(event) = this_value else {
                return Err(JsError::type_error(
                    "event method receiver is not an object",
                ));
            };
            match builtin {
                BuiltinFunction::EventStopPropagation => {
                    self.object_mut(event)?
                        .properties
                        .insert("__stopPropagation".into(), JsValue::Boolean(true));
                }
                BuiltinFunction::EventPreventDefault => {
                    if self
                        .object(event)?
                        .properties
                        .get("cancelable")
                        .is_some_and(JsValue::is_truthy)
                    {
                        self.object_mut(event)?
                            .properties
                            .insert("defaultPrevented".into(), JsValue::Boolean(true));
                    }
                }
                _ => unreachable!(),
            }
            return Ok(CallOutcome::Value(JsValue::Undefined));
        }
        if matches!(
            builtin,
            BuiltinFunction::LifecycleAddEventListener
                | BuiltinFunction::LifecycleRemoveEventListener
        ) {
            let JsValue::Object(receiver) = this_value else {
                return Err(JsError::type_error(
                    "event target must be document or window",
                ));
            };
            if Some(receiver) != self.dom_document && receiver != self.global_object {
                return Err(JsError::type_error("unsupported event target"));
            }
            let event = arguments
                .first()
                .map(JsValue::to_js_string)
                .unwrap_or_default();
            let allowed = if Some(receiver) == self.dom_document {
                matches!(event.as_str(), "readystatechange" | "DOMContentLoaded")
            } else {
                event == "load"
            };
            if !allowed {
                return Ok(CallOutcome::Value(JsValue::Undefined));
            }
            let remove = matches!(builtin, BuiltinFunction::LifecycleRemoveEventListener);
            let Some(JsValue::Object(callback)) = arguments.get(1) else {
                if remove {
                    return Ok(CallOutcome::Value(JsValue::Undefined));
                }
                return Err(JsError::type_error("lifecycle listener must be callable"));
            };
            if self.object(*callback)?.function.is_none() {
                if remove {
                    return Ok(CallOutcome::Value(JsValue::Undefined));
                }
                return Err(JsError::type_error("lifecycle listener must be callable"));
            }
            let capture = matches!(arguments.get(2), Some(JsValue::Boolean(true)));
            let key = (receiver, event, capture);
            if remove {
                if let Some(handlers) = self.lifecycle_listeners.get_mut(&key) {
                    handlers.retain(|existing| existing != &JsValue::Object(*callback));
                }
            } else {
                if self
                    .lifecycle_listeners
                    .values()
                    .map(Vec::len)
                    .sum::<usize>()
                    >= 256
                {
                    return Err(JsError::execution_limit(
                        "lifecycle listener budget exceeded",
                    ));
                }
                let handlers = self.lifecycle_listeners.entry(key).or_default();
                if !handlers.contains(&JsValue::Object(*callback)) {
                    handlers.push(JsValue::Object(*callback));
                }
            }
            return Ok(CallOutcome::Value(JsValue::Undefined));
        }
        if matches!(builtin, BuiltinFunction::DomGetElementById) {
            let Some(id) = arguments.first().map(JsValue::to_js_string) else {
                return Ok(CallOutcome::Value(JsValue::Null));
            };
            let Some(node) = self.dom_ids.get(&id).copied() else {
                return Ok(CallOutcome::Value(JsValue::Null));
            };
            let element = self.dom_element_object(node)?;
            return Ok(CallOutcome::Value(JsValue::Object(element)));
        }
        if builtin == BuiltinFunction::DomCreateElement {
            if !matches!(this_value,JsValue::Object(object) if Some(object)==self.dom_document) {
                return Err(JsError::type_error(
                    "createElement requires document receiver",
                ));
            }
            let Some(value) = arguments.first() else {
                return Err(JsError::type_error("createElement requires tag name"));
            };
            let tag = value.to_js_string().to_ascii_lowercase();
            if tag.is_empty()
                || tag.len() > 64
                || !tag.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
                || !tag.as_bytes()[0].is_ascii_alphabetic()
            {
                return Err(JsError::type_error("invalid DOM tag name"));
            }
            if self.next_virtual_dom_node > 256 || self.dom_operations.len() >= 256 {
                return Err(JsError::execution_limit("dynamic DOM node budget exceeded"));
            }
            let virtual_node = usize::MAX - self.next_virtual_dom_node;
            self.next_virtual_dom_node += 1;
            self.dom_tags.insert(virtual_node, tag.clone());
            self.dom_operations.push(DomOperation::CreateElement {
                node: virtual_node,
                tag,
            });
            let object = self.dom_element_object(virtual_node)?;
            return Ok(CallOutcome::Value(JsValue::Object(object)));
        }
        if builtin == BuiltinFunction::DomCreateTextNode {
            if !matches!(this_value,JsValue::Object(object) if Some(object)==self.dom_document) {
                return Err(JsError::type_error(
                    "createTextNode requires document receiver",
                ));
            }
            let text = arguments
                .first()
                .unwrap_or(&JsValue::Undefined)
                .to_js_string();
            if text.len() > 64 * 1024
                || self.next_virtual_dom_node > 256
                || self.dom_operations.len() >= 256
            {
                return Err(JsError::execution_limit("createTextNode budget exceeded"));
            }
            let node = usize::MAX - self.next_virtual_dom_node;
            self.next_virtual_dom_node += 1;
            self.dom_text.insert(node, text.clone());
            self.dom_text_nodes.insert(node);
            self.dom_operations.push(DomOperation::CreateText {
                node,
                text: text.clone(),
            });
            let object = self.allocate_object(
                ObjectKind::DomText(node),
                Some(self.object_prototype),
                HashMap::from([
                    ("textContent".into(), JsValue::String(text.clone())),
                    ("nodeValue".into(), JsValue::String(text.clone())),
                    ("data".into(), JsValue::String(text)),
                    ("nodeType".into(), JsValue::Number(3.0)),
                ]),
            )?;
            self.dom_node_objects.insert(node, object);
            return Ok(CallOutcome::Value(JsValue::Object(object)));
        }
        if matches!(
            builtin,
            BuiltinFunction::DomStyleSetProperty
                | BuiltinFunction::DomStyleGetPropertyValue
                | BuiltinFunction::DomStyleRemoveProperty
        ) {
            let JsValue::Object(receiver) = this_value else {
                return Err(JsError::type_error(
                    "style method requires CSSStyleDeclaration",
                ));
            };
            let ObjectKind::DomStyle(node) = self.object(receiver)?.kind else {
                return Err(JsError::type_error(
                    "style method requires CSSStyleDeclaration",
                ));
            };
            let name = arguments
                .first()
                .unwrap_or(&JsValue::Undefined)
                .to_js_string();
            let name = Self::style_property_name(name.trim());
            if builtin == BuiltinFunction::DomStyleGetPropertyValue {
                let value = self
                    .dom_style_declarations(node)
                    .into_iter()
                    .find(|(key, _)| key == &name)
                    .map_or_else(String::new, |(_, value)| value);
                return Ok(CallOutcome::Value(JsValue::String(value)));
            }
            if builtin == BuiltinFunction::DomStyleRemoveProperty {
                let previous = self.set_dom_style_property(node, &name, None)?;
                return Ok(CallOutcome::Value(JsValue::String(previous)));
            }
            let value = arguments
                .get(1)
                .unwrap_or(&JsValue::Undefined)
                .to_js_string();
            if value.len() > 64 * 1024 {
                return Err(JsError::execution_limit(
                    "CSS property value budget exceeded",
                ));
            }
            self.set_dom_style_property(node, &name, Some(value))?;
            return Ok(CallOutcome::Value(JsValue::Undefined));
        }
        if builtin == BuiltinFunction::DomCreateEvent {
            if Some(this_value.clone()) != self.dom_document.map(JsValue::Object) {
                return Err(JsError::type_error("createEvent requires Document"));
            }
            let name = arguments
                .first()
                .unwrap_or(&JsValue::Undefined)
                .to_js_string();
            if !matches!(name.as_str(), "Event" | "Events" | "HTMLEvents") {
                return Err(JsError::type_error("unsupported legacy event interface"));
            }
            let stop = self.allocate_lifecycle_method(
                "stopPropagation",
                BuiltinFunction::EventStopPropagation,
            )?;
            let prevent = self.allocate_lifecycle_method(
                "preventDefault",
                BuiltinFunction::EventPreventDefault,
            )?;
            let init =
                self.allocate_lifecycle_method("initEvent", BuiltinFunction::DomInitEvent)?;
            let event = self.allocate_object(
                ObjectKind::DomEvent,
                Some(self.object_prototype),
                HashMap::from([
                    ("type".into(), JsValue::String(String::new())),
                    ("target".into(), JsValue::Null),
                    ("currentTarget".into(), JsValue::Null),
                    ("bubbles".into(), JsValue::Boolean(false)),
                    ("cancelable".into(), JsValue::Boolean(false)),
                    ("defaultPrevented".into(), JsValue::Boolean(false)),
                    ("isTrusted".into(), JsValue::Boolean(false)),
                    ("eventPhase".into(), JsValue::Number(0.0)),
                    ("stopPropagation".into(), JsValue::Object(stop)),
                    ("preventDefault".into(), JsValue::Object(prevent)),
                    ("initEvent".into(), JsValue::Object(init)),
                    ("__initialized".into(), JsValue::Boolean(false)),
                    ("__dispatching".into(), JsValue::Boolean(false)),
                    ("__stopPropagation".into(), JsValue::Boolean(false)),
                ]),
            )?;
            return Ok(CallOutcome::Value(JsValue::Object(event)));
        }
        if builtin == BuiltinFunction::DomInitEvent {
            let JsValue::Object(receiver) = this_value else {
                return Err(JsError::type_error("initEvent receiver is not Event"));
            };
            if self.object(receiver)?.kind != ObjectKind::DomEvent {
                return Err(JsError::type_error("initEvent requires Event"));
            }
            if self
                .object(receiver)?
                .properties
                .get("__dispatching")
                .is_some_and(JsValue::is_truthy)
            {
                return Ok(CallOutcome::Value(JsValue::Undefined));
            }
            let kind = arguments
                .first()
                .unwrap_or(&JsValue::Undefined)
                .to_js_string();
            if kind.len() > 128 {
                return Err(JsError::execution_limit("event type budget exceeded"));
            }
            let bubbles = arguments.get(1).is_some_and(JsValue::is_truthy);
            let cancelable = arguments.get(2).is_some_and(JsValue::is_truthy);
            let properties = &mut self.object_mut(receiver)?.properties;
            properties.insert("type".into(), JsValue::String(kind));
            properties.insert("bubbles".into(), JsValue::Boolean(bubbles));
            properties.insert("cancelable".into(), JsValue::Boolean(cancelable));
            properties.insert("defaultPrevented".into(), JsValue::Boolean(false));
            properties.insert("__initialized".into(), JsValue::Boolean(true));
            return Ok(CallOutcome::Value(JsValue::Undefined));
        }
        if builtin == BuiltinFunction::DomEventConstructor {
            let event_type = arguments
                .first()
                .unwrap_or(&JsValue::Undefined)
                .to_js_string();
            if event_type.len() > 128 {
                return Err(JsError::execution_limit(
                    "Event.type length budget exceeded",
                ));
            }
            let options = arguments.get(1).cloned().unwrap_or(JsValue::Undefined);
            let bubbles = if let JsValue::Object(id) = options {
                self.get_object_property(id, "bubbles")?.is_truthy()
            } else {
                false
            };
            let cancelable = if let JsValue::Object(id) = options {
                self.get_object_property(id, "cancelable")?.is_truthy()
            } else {
                false
            };
            let stop = self.allocate_lifecycle_method(
                "stopPropagation",
                BuiltinFunction::EventStopPropagation,
            )?;
            let prevent = self.allocate_lifecycle_method(
                "preventDefault",
                BuiltinFunction::EventPreventDefault,
            )?;
            let init =
                self.allocate_lifecycle_method("initEvent", BuiltinFunction::DomInitEvent)?;
            let event = self.allocate_object(
                ObjectKind::DomEvent,
                Some(self.object_prototype),
                HashMap::from([
                    ("initEvent".into(), JsValue::Object(init)),
                    ("__initialized".into(), JsValue::Boolean(true)),
                    ("type".into(), JsValue::String(event_type)),
                    ("target".into(), JsValue::Null),
                    ("currentTarget".into(), JsValue::Null),
                    ("bubbles".into(), JsValue::Boolean(bubbles)),
                    ("cancelable".into(), JsValue::Boolean(cancelable)),
                    ("defaultPrevented".into(), JsValue::Boolean(false)),
                    ("isTrusted".into(), JsValue::Boolean(false)),
                    ("eventPhase".into(), JsValue::Number(0.0)),
                    ("stopPropagation".into(), JsValue::Object(stop)),
                    ("preventDefault".into(), JsValue::Object(prevent)),
                    ("__dispatching".into(), JsValue::Boolean(false)),
                    ("__stopPropagation".into(), JsValue::Boolean(false)),
                ]),
            )?;
            return Ok(CallOutcome::Value(JsValue::Object(event)));
        }
        if builtin == BuiltinFunction::DomDispatchEvent {
            let JsValue::Object(receiver) = this_value else {
                return Err(JsError::type_error("dispatchEvent needs Element receiver"));
            };
            let ObjectKind::DomElement(node) = self.object(receiver)?.kind else {
                return Err(JsError::type_error("dispatchEvent needs Element receiver"));
            };
            let Some(JsValue::Object(event)) = arguments.first() else {
                return Err(JsError::type_error("dispatchEvent requires an Event"));
            };
            if self.object(*event)?.kind != ObjectKind::DomEvent {
                return Err(JsError::type_error("dispatchEvent requires an Event"));
            }
            let mut path = vec![node];
            let mut cursor = self.dom_pending_parents.get(&node).copied();
            while let Some(parent) = cursor {
                if path.len() >= 64 {
                    return Err(JsError::execution_limit("event path budget exceeded"));
                }
                path.push(parent);
                cursor = self.dom_pending_parents.get(&parent).copied();
            }
            let uncanceled = self.dispatch_custom_event(*event, &path)?;
            return Ok(CallOutcome::Value(JsValue::Boolean(uncanceled)));
        }
        if builtin == BuiltinFunction::DomClick {
            let JsValue::Object(receiver) = this_value else {
                return Err(JsError::type_error("click requires an element"));
            };
            let ObjectKind::DomElement(node) = self.object(receiver)?.kind else {
                return Err(JsError::type_error("click requires an element"));
            };
            if self.dom_programmatic_click_depth >= 8 {
                return Err(JsError::execution_limit(
                    "nested programmatic click budget exceeded",
                ));
            }
            let mut path = Vec::new();
            let mut cursor = Some(node);
            while let Some(current) = cursor {
                if path.len() >= 256 {
                    return Err(JsError::execution_limit("DOM click path too deep"));
                }
                path.push(current);
                cursor = self.dom_pending_parents.get(&current).copied();
            }
            self.dom_programmatic_click_depth += 1;
            let dispatched = self.dispatch_dom_click_path(&path);
            self.dom_programmatic_click_depth -= 1;
            dispatched?;
            return Ok(CallOutcome::Value(JsValue::Undefined));
        }
        if builtin == BuiltinFunction::DomContains {
            let JsValue::Object(receiver) = this_value else {
                return Err(JsError::type_error("contains requires DOM node receiver"));
            };
            let root = match self.object(receiver)?.kind {
                ObjectKind::DomElement(node) | ObjectKind::DomText(node) => node,
                _ => return Err(JsError::type_error("contains requires DOM node receiver")),
            };
            let Some(JsValue::Object(argument)) = arguments.first() else {
                return Ok(CallOutcome::Value(JsValue::Boolean(false)));
            };
            let current = match self.object(*argument)?.kind {
                ObjectKind::DomElement(node) | ObjectKind::DomText(node) => Some(node),
                _ => None,
            };
            let mut cursor = current;
            for _ in 0..=20_000 {
                let Some(node) = cursor else {
                    return Ok(CallOutcome::Value(JsValue::Boolean(false)));
                };
                if node == root {
                    return Ok(CallOutcome::Value(JsValue::Boolean(true)));
                }
                cursor = self.dom_pending_parents.get(&node).copied();
            }
            return Err(JsError::execution_limit("DOM ancestor depth exceeded"));
        }
        if builtin == BuiltinFunction::DomHtmlCollectionNamedItem {
            let JsValue::Object(receiver) = this_value else {
                return Err(JsError::type_error("namedItem requires HTMLCollection"));
            };
            let ObjectKind::DomHtmlCollection(parent) = self.object(receiver)?.kind else {
                return Err(JsError::type_error("namedItem requires HTMLCollection"));
            };
            let name = arguments
                .first()
                .unwrap_or(&JsValue::Undefined)
                .to_js_string();
            return match self.html_collection_named(parent, &name) {
                Some(node) => self
                    .dom_any_node_object(node)
                    .map(|obj| CallOutcome::Value(JsValue::Object(obj))),
                None => Ok(CallOutcome::Value(JsValue::Null)),
            };
        }
        if builtin == BuiltinFunction::DomNodeListItem {
            let JsValue::Object(receiver) = this_value else {
                return Err(JsError::type_error("item requires NodeList"));
            };
            let kind = self.object(receiver)?.kind;
            if kind == ObjectKind::DomQueryNodeList {
                let index = arguments.first().unwrap_or(&JsValue::Undefined).to_number();
                let found = if index.is_finite() && index >= 0.0 && index.fract() == 0.0 {
                    self.dom_query_lists
                        .get(&receiver)
                        .and_then(|nodes| nodes.get(index as usize))
                        .copied()
                } else {
                    None
                };
                return match found {
                    Some(node) => self
                        .dom_any_node_object(node)
                        .map(|id| CallOutcome::Value(JsValue::Object(id))),
                    None => Ok(CallOutcome::Value(JsValue::Null)),
                };
            }
            let parent = match kind {
                ObjectKind::DomNodeList(parent) | ObjectKind::DomHtmlCollection(parent) => parent,
                ObjectKind::DomTagCollection => {
                    let (root, tag) = self
                        .dom_tag_collections
                        .get(&receiver)
                        .ok_or_else(|| JsError::type_error("invalid tag collection"))?;
                    let number = arguments.first().unwrap_or(&JsValue::Undefined).to_number();
                    let child = if number.is_finite() && number >= 0.0 && number.fract() == 0.0 {
                        self.elements_by_tag(*root, tag)
                            .get(number as usize)
                            .copied()
                    } else {
                        None
                    };
                    return match child {
                        Some(child) => self
                            .dom_any_node_object(child)
                            .map(|id| CallOutcome::Value(JsValue::Object(id))),
                        None => Ok(CallOutcome::Value(JsValue::Null)),
                    };
                }
                _ => {
                    return Err(JsError::type_error(
                        "item requires NodeList or HTMLCollection",
                    ));
                }
            };
            let number = arguments.first().unwrap_or(&JsValue::Undefined).to_number();
            let child = if number.is_finite() && number >= 0.0 && number.fract() == 0.0 {
                if matches!(kind, ObjectKind::DomHtmlCollection(_)) {
                    self.html_collection_nodes(parent)
                        .get(number as usize)
                        .copied()
                } else {
                    self.dom_children
                        .get(&parent)
                        .and_then(|children| children.get(number as usize))
                        .copied()
                }
            } else {
                None
            };
            return match child {
                Some(child) => self
                    .dom_any_node_object(child)
                    .map(|id| CallOutcome::Value(JsValue::Object(id))),
                None => Ok(CallOutcome::Value(JsValue::Null)),
            };
        }
        if matches!(
            builtin,
            BuiltinFunction::DomClassAdd
                | BuiltinFunction::DomClassRemove
                | BuiltinFunction::DomClassContains
                | BuiltinFunction::DomClassToggle
        ) {
            let JsValue::Object(receiver) = this_value else {
                return Err(JsError::type_error(
                    "classList method requires a DOMTokenList",
                ));
            };
            let ObjectKind::DomClassList(node) = self.object(receiver)?.kind else {
                return Err(JsError::type_error(
                    "classList method requires a DOMTokenList",
                ));
            };
            let mut incoming = Vec::new();
            let operands = if matches!(
                builtin,
                BuiltinFunction::DomClassAdd | BuiltinFunction::DomClassRemove
            ) {
                arguments.as_slice()
            } else {
                &arguments[..arguments.len().min(1)]
            };
            for arg in operands {
                let token = arg.to_js_string();
                if token.is_empty() || token.len() > 512 || token.chars().any(char::is_whitespace) {
                    return Err(JsError::type_error("classList token invalid"));
                }
                incoming.push(token);
            }
            if builtin == BuiltinFunction::DomClassAdd || builtin == BuiltinFunction::DomClassRemove
            {
                let mut tokens = self.class_tokens(node);
                let original = tokens.clone();
                for token in incoming {
                    let contains = tokens.contains(&token);
                    if builtin == BuiltinFunction::DomClassAdd && !contains {
                        tokens.push(token);
                    } else if builtin == BuiltinFunction::DomClassRemove && contains {
                        tokens.retain(|current| current != &token);
                    }
                }
                if tokens != original {
                    self.stage_dom_attribute(node, "class", tokens.join(" "))?;
                }
                return Ok(CallOutcome::Value(JsValue::Undefined));
            }
            let Some(token) = incoming.into_iter().next() else {
                return Err(JsError::type_error("classList token argument is required"));
            };
            let mut tokens = self.class_tokens(node);
            let exists = tokens.contains(&token);
            if builtin == BuiltinFunction::DomClassContains {
                return Ok(CallOutcome::Value(JsValue::Boolean(exists)));
            }
            let desired = arguments.get(1).map(JsValue::is_truthy).unwrap_or(!exists);
            if desired != exists {
                if desired {
                    tokens.push(token);
                } else {
                    tokens.retain(|current| current != &token);
                }
                self.stage_dom_attribute(node, "class", tokens.join(" "))?;
            }
            return Ok(CallOutcome::Value(JsValue::Boolean(desired)));
        }
        if builtin == BuiltinFunction::DomRemoveSelf {
            let JsValue::Object(receiver) = this_value else {
                return Err(JsError::type_error("remove needs an element receiver"));
            };
            let ObjectKind::DomElement(node) = self.object(receiver)?.kind else {
                return Err(JsError::type_error("remove needs an element receiver"));
            };
            if let Some(&parent) = self.dom_pending_parents.get(&node) {
                if self.dom_operations.len() >= 256 {
                    return Err(JsError::execution_limit("DOM mutation budget exceeded"));
                }
                self.dom_pending_parents.remove(&node);
                self.dom_operations.push(DomOperation::RemoveChild {
                    parent,
                    child: node,
                });
                self.detach_dom_subtree(node)?;
            }
            return Ok(CallOutcome::Value(JsValue::Undefined));
        }
        if builtin == BuiltinFunction::DomReplaceChild {
            let JsValue::Object(receiver) = this_value else {
                return Err(JsError::type_error("replaceChild needs element receiver"));
            };
            let ObjectKind::DomElement(parent) = self.object(receiver)?.kind else {
                return Err(JsError::type_error("replaceChild needs element receiver"));
            };
            let node_arg = |at: usize| -> Result<(usize, ObjectId), JsError> {
                let Some(JsValue::Object(id)) = arguments.get(at) else {
                    return Err(JsError::type_error("replaceChild expects two nodes"));
                };
                let node = match self.object(*id)?.kind {
                    ObjectKind::DomElement(n) | ObjectKind::DomText(n) => n,
                    _ => return Err(JsError::type_error("replaceChild expects DOM nodes")),
                };
                Ok((node, *id))
            };
            let (new_child, _) = node_arg(0)?;
            let (old_child, old_object) = node_arg(1)?;
            if self.dom_pending_parents.get(&old_child).copied() != Some(parent) {
                return Err(JsError::type_error("replaceChild old node is not a child"));
            }
            let mut cursor = Some(parent);
            for _ in 0..=20_000 {
                let Some(current) = cursor else {
                    break;
                };
                if current == new_child && new_child != old_child {
                    return Err(JsError::type_error("cyclic DOM replacement"));
                }
                cursor = self.dom_pending_parents.get(&current).copied();
            }
            if cursor.is_some() || self.dom_operations.len() >= 256 {
                return Err(JsError::execution_limit("DOM replace budget exceeded"));
            }
            if new_child != old_child {
                self.dom_operations.push(DomOperation::ReplaceChild {
                    parent,
                    new_child,
                    old_child,
                });
                self.stage_dom_move(parent, new_child, Some(old_child));
                self.attach_dom_subtree(new_child);
                self.dom_pending_parents.remove(&old_child);
                self.detach_dom_subtree(old_child)?;
            }
            return Ok(CallOutcome::Value(JsValue::Object(old_object)));
        }
        if matches!(
            builtin,
            BuiltinFunction::DomMatches | BuiltinFunction::DomClosest
        ) {
            let JsValue::Object(receiver) = this_value else {
                return Err(JsError::type_error("matches/closest requires an Element"));
            };
            let ObjectKind::DomElement(node) = self.object(receiver)?.kind else {
                return Err(JsError::type_error("matches/closest requires an Element"));
            };
            let selector = arguments
                .first()
                .unwrap_or(&JsValue::Undefined)
                .to_js_string();
            let groups = Self::selector_groups(&selector)?;
            let mut cursor = Some(node);
            let mut visited = std::collections::HashSet::new();
            while let Some(current) = cursor {
                if !visited.insert(current) || visited.len() > 256 {
                    return Err(JsError::execution_limit("closest ancestor budget exceeded"));
                }
                if groups
                    .iter()
                    .any(|group| self.selector_chain_matches(current, 0, group))
                {
                    if builtin == BuiltinFunction::DomMatches {
                        return Ok(CallOutcome::Value(JsValue::Boolean(true)));
                    }
                    return self
                        .dom_any_node_object(current)
                        .map(|id| CallOutcome::Value(JsValue::Object(id)));
                }
                if builtin == BuiltinFunction::DomMatches {
                    break;
                }
                cursor = self.dom_pending_parents.get(&current).copied();
            }
            return Ok(CallOutcome::Value(
                if builtin == BuiltinFunction::DomMatches {
                    JsValue::Boolean(false)
                } else {
                    JsValue::Null
                },
            ));
        }
        if matches!(
            builtin,
            BuiltinFunction::DomQuerySelector | BuiltinFunction::DomQuerySelectorAll
        ) {
            let root = match this_value {
                JsValue::Object(id) if Some(id) == self.dom_document => 0,
                JsValue::Object(id) => match self.object(id)?.kind {
                    ObjectKind::DomElement(node) => node,
                    _ => return Err(JsError::type_error("selector requires Document or Element")),
                },
                _ => return Err(JsError::type_error("selector requires DOM receiver")),
            };
            let selector = arguments
                .first()
                .unwrap_or(&JsValue::Undefined)
                .to_js_string();
            let nodes = self.query_descendants(root, &selector)?;
            if builtin == BuiltinFunction::DomQuerySelector {
                return match nodes.first().copied() {
                    Some(node) => self
                        .dom_any_node_object(node)
                        .map(|id| CallOutcome::Value(JsValue::Object(id))),
                    None => Ok(CallOutcome::Value(JsValue::Null)),
                };
            }
            let list = self.static_node_list(nodes)?;
            return Ok(CallOutcome::Value(JsValue::Object(list)));
        }
        if builtin == BuiltinFunction::DomGetElementsByTagName {
            let root = match this_value {
                JsValue::Object(id) if Some(id) == self.dom_document => 0,
                JsValue::Object(id) => match self.object(id)?.kind {
                    ObjectKind::DomElement(root) => root,
                    _ => {
                        return Err(JsError::type_error(
                            "getElementsByTagName needs Document or Element",
                        ));
                    }
                },
                _ => {
                    return Err(JsError::type_error(
                        "getElementsByTagName needs DOM receiver",
                    ));
                }
            };
            let tag = arguments
                .first()
                .unwrap_or(&JsValue::Undefined)
                .to_js_string()
                .to_ascii_lowercase();
            if tag.len() > 128 {
                return Err(JsError::execution_limit("tag name too long"));
            }
            let list = self.dom_tag_collection_object(root, tag)?;
            return Ok(CallOutcome::Value(JsValue::Object(list)));
        }
        if builtin == BuiltinFunction::DomHasAttributes {
            let JsValue::Object(id) = this_value else {
                return Err(JsError::type_error("hasAttributes requires an Element"));
            };
            let ObjectKind::DomElement(node) = self.object(id)?.kind else {
                return Err(JsError::type_error("hasAttributes requires an Element"));
            };
            return Ok(CallOutcome::Value(JsValue::Boolean(
                self.dom_attributes
                    .get(&node)
                    .is_some_and(|attrs| !attrs.is_empty()),
            )));
        }
        if matches!(
            builtin,
            BuiltinFunction::DomSetAttribute
                | BuiltinFunction::DomGetAttribute
                | BuiltinFunction::DomHasAttribute
                | BuiltinFunction::DomRemoveAttribute
        ) {
            let JsValue::Object(receiver) = this_value else {
                return Err(JsError::type_error(
                    "attribute method requires element receiver",
                ));
            };
            let ObjectKind::DomElement(node) = self.object(receiver)?.kind else {
                return Err(JsError::type_error(
                    "attribute method requires element receiver",
                ));
            };
            let Some(name_value) = arguments.first() else {
                return Err(JsError::type_error("attribute name is missing"));
            };
            let name = name_value.to_js_string().to_ascii_lowercase();
            if name.is_empty()
                || name.len() > 128
                || !name
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b':')
            {
                return Err(JsError::type_error("invalid attribute name"));
            }
            if builtin == BuiltinFunction::DomHasAttribute {
                return Ok(CallOutcome::Value(JsValue::Boolean(
                    self.dom_attributes
                        .get(&node)
                        .is_some_and(|attrs| attrs.contains_key(&name)),
                )));
            }
            if builtin == BuiltinFunction::DomGetAttribute {
                return Ok(CallOutcome::Value(
                    self.dom_attributes
                        .get(&node)
                        .and_then(|attrs| attrs.get(&name))
                        .map(|text| JsValue::String(text.clone()))
                        .unwrap_or(JsValue::Null),
                ));
            }
            if self.dom_operations.len() >= 256 {
                return Err(JsError::execution_limit(
                    "DOM attribute mutation budget exceeded",
                ));
            }
            if builtin == BuiltinFunction::DomRemoveAttribute {
                self.dom_attributes.entry(node).or_default().remove(&name);
                if name == "id" {
                    self.dom_ids.retain(|_, value| *value != node);
                    self.dom_pending_ids.remove(&node);
                    self.object_mut(receiver)?
                        .properties
                        .insert("id".into(), JsValue::String(String::new()));
                }
                self.dom_operations
                    .push(DomOperation::RemoveAttribute { node, name });
                return Ok(CallOutcome::Value(JsValue::Undefined));
            }
            let value = arguments
                .get(1)
                .unwrap_or(&JsValue::Undefined)
                .to_js_string();
            if value.len() > 64 * 1024 {
                return Err(JsError::execution_limit(
                    "DOM attribute value length budget exceeded",
                ));
            }
            if name == "id" {
                self.dom_ids.retain(|_, old| *old != node);
                self.dom_pending_ids.insert(node, value.clone());
                if self.dom_attached.contains(&node) && !value.is_empty() {
                    self.dom_ids.insert(value.clone(), node);
                }
                self.object_mut(receiver)?
                    .properties
                    .insert("id".into(), JsValue::String(value.clone()));
            }
            self.dom_attributes
                .entry(node)
                .or_default()
                .insert(name.clone(), value.clone());
            self.dom_operations
                .push(DomOperation::SetAttribute { node, name, value });
            return Ok(CallOutcome::Value(JsValue::Undefined));
        }
        if matches!(
            builtin,
            BuiltinFunction::DomInsertBefore | BuiltinFunction::DomRemoveChild
        ) {
            let JsValue::Object(receiver) = this_value else {
                return Err(JsError::type_error("DOM parent receiver required"));
            };
            let ObjectKind::DomElement(parent) = self.object(receiver)?.kind else {
                return Err(JsError::type_error("DOM parent receiver required"));
            };
            let Some(JsValue::Object(child_object)) = arguments.first() else {
                return Err(JsError::type_error("DOM child argument required"));
            };
            let child = match self.object(*child_object)?.kind {
                ObjectKind::DomElement(node) | ObjectKind::DomText(node) => node,
                _ => return Err(JsError::type_error("DOM child argument must be a node")),
            };
            if builtin == BuiltinFunction::DomRemoveChild {
                if self.dom_pending_parents.get(&child).copied() != Some(parent) {
                    return Err(JsError::type_error(
                        "removeChild target is not a direct child",
                    ));
                }
                if self.dom_operations.len() >= 256 {
                    return Err(JsError::execution_limit("DOM operation budget exceeded"));
                }
                self.dom_pending_parents.remove(&child);
                self.dom_operations
                    .push(DomOperation::RemoveChild { parent, child });
                self.detach_dom_subtree(child)?;
                return Ok(CallOutcome::Value(JsValue::Object(*child_object)));
            }
            let reference = match arguments.get(1) {
                Some(JsValue::Object(ref_object)) => {
                    let reference = match self.object(*ref_object)?.kind {
                        ObjectKind::DomElement(node) | ObjectKind::DomText(node) => node,
                        _ => return Err(JsError::type_error("invalid insertBefore reference")),
                    };
                    Some(reference)
                }
                Some(JsValue::Null) => None,
                _ => {
                    return Err(JsError::type_error(
                        "insertBefore requires reference or null",
                    ));
                }
            };
            if reference.is_some_and(|reference| {
                self.dom_pending_parents.get(&reference).copied() != Some(parent)
            }) {
                return Err(JsError::type_error("reference is not a direct child"));
            }
            if child == parent {
                return Err(JsError::type_error("cannot insert element into itself"));
            }
            let mut cursor = Some(parent);
            for _ in 0..=20_000 {
                let Some(current) = cursor else {
                    break;
                };
                if current == child {
                    return Err(JsError::type_error("cyclic DOM insertion"));
                }
                cursor = self.dom_pending_parents.get(&current).copied();
            }
            if cursor.is_some() {
                return Err(JsError::execution_limit("DOM ancestor depth exceeded"));
            }
            if self.dom_operations.len() >= 256 {
                return Err(JsError::execution_limit("DOM operation budget exceeded"));
            }
            self.stage_dom_move(parent, child, reference);
            self.dom_operations.push(DomOperation::InsertBefore {
                parent,
                child,
                reference,
            });
            if self.dom_attached.contains(&parent) {
                self.attach_dom_subtree(child);
            }
            return Ok(CallOutcome::Value(JsValue::Object(*child_object)));
        }
        if builtin == BuiltinFunction::DomAppendChild {
            let JsValue::Object(parent_id) = this_value else {
                return Err(JsError::type_error("appendChild requires element receiver"));
            };
            let ObjectKind::DomElement(parent) = self.object(parent_id)?.kind else {
                return Err(JsError::type_error("appendChild requires element receiver"));
            };
            let Some(JsValue::Object(child_id)) = arguments.first() else {
                return Err(JsError::type_error("appendChild requires a DOM element"));
            };
            let child = match self.object(*child_id)?.kind {
                ObjectKind::DomElement(node) | ObjectKind::DomText(node) => node,
                _ => return Err(JsError::type_error("appendChild requires a DOM node")),
            };
            if parent == child {
                return Err(JsError::type_error("cannot append element to itself"));
            }
            // Detect cycles among nodes whose relationships were created
            // during this script, before committing them to the host tree.
            let mut cursor = Some(parent);
            for _ in 0..=256 {
                let Some(current) = cursor else { break };
                if current == child {
                    return Err(JsError::type_error("cyclic DOM attachment"));
                }
                cursor = self.dom_pending_parents.get(&current).copied();
            }
            if cursor.is_some() {
                return Err(JsError::execution_limit("DOM attachment depth exceeded"));
            }
            if self.dom_operations.len() >= 256 {
                return Err(JsError::execution_limit(
                    "dynamic DOM mutation budget exceeded",
                ));
            }
            self.stage_dom_move(parent, child, None);
            self.dom_operations
                .push(DomOperation::AppendChild { parent, child });
            if self.dom_attached.contains(&parent) {
                self.attach_dom_subtree(child);
            }
            return Ok(CallOutcome::Value(JsValue::Object(*child_id)));
        }
        if matches!(
            builtin,
            BuiltinFunction::DomAddEventListener | BuiltinFunction::DomRemoveEventListener
        ) {
            let JsValue::Object(receiver) = this_value else {
                return Err(JsError::type_error(
                    "addEventListener receiver is not an element",
                ));
            };
            let ObjectKind::DomElement(node) = self.object(receiver)?.kind else {
                return Err(JsError::type_error(
                    "addEventListener receiver is not an element",
                ));
            };
            let event_type = arguments
                .first()
                .unwrap_or(&JsValue::Undefined)
                .to_js_string();
            if event_type.len() > 128 {
                return Err(JsError::execution_limit("listener type budget exceeded"));
            }

            let Some(callback) = arguments.get(1).cloned() else {
                if matches!(builtin, BuiltinFunction::DomRemoveEventListener) {
                    return Ok(CallOutcome::Value(JsValue::Undefined));
                }
                return Err(JsError::type_error("event listener callback is missing"));
            };
            let JsValue::Object(function) = callback else {
                if matches!(builtin, BuiltinFunction::DomRemoveEventListener) {
                    return Ok(CallOutcome::Value(JsValue::Undefined));
                }
                return Err(JsError::type_error("event listener must be callable"));
            };
            if self.object(function)?.function.is_none() {
                if matches!(builtin, BuiltinFunction::DomRemoveEventListener) {
                    return Ok(CallOutcome::Value(JsValue::Undefined));
                }
                return Err(JsError::type_error("event listener must be callable"));
            }
            let capture = matches!(arguments.get(2), Some(JsValue::Boolean(true)));
            if event_type != "click" {
                let key = (node, event_type, capture);
                if matches!(builtin, BuiltinFunction::DomRemoveEventListener) {
                    if let Some(listeners) = self.dom_typed_listeners.get_mut(&key) {
                        listeners.retain(|value| value != &JsValue::Object(function));
                    }
                    return Ok(CallOutcome::Value(JsValue::Undefined));
                }
                if self
                    .dom_typed_listeners
                    .values()
                    .map(Vec::len)
                    .sum::<usize>()
                    >= 256
                {
                    return Err(JsError::execution_limit("typed listener budget exceeded"));
                }
                let listeners = self.dom_typed_listeners.entry(key).or_default();
                if !listeners.contains(&JsValue::Object(function)) {
                    listeners.push(JsValue::Object(function));
                }
                return Ok(CallOutcome::Value(JsValue::Undefined));
            }
            if matches!(builtin, BuiltinFunction::DomRemoveEventListener) {
                let listeners = if capture {
                    self.dom_click_capture_listeners.get_mut(&node)
                } else {
                    self.dom_click_listeners.get_mut(&node)
                };
                if let Some(listeners) = listeners {
                    listeners.retain(|listener| listener != &JsValue::Object(function));
                }
                return Ok(CallOutcome::Value(JsValue::Undefined));
            }
            let total = self
                .dom_click_listeners
                .values()
                .map(Vec::len)
                .sum::<usize>()
                + self
                    .dom_click_capture_listeners
                    .values()
                    .map(Vec::len)
                    .sum::<usize>();
            if total >= 256 {
                return Err(JsError::execution_limit(
                    "DOM event listener budget exceeded",
                ));
            }
            let listeners = if capture {
                self.dom_click_capture_listeners.entry(node).or_default()
            } else {
                self.dom_click_listeners.entry(node).or_default()
            };
            if !listeners.contains(&JsValue::Object(function)) {
                listeners.push(JsValue::Object(function));
            }
            return Ok(CallOutcome::Value(JsValue::Undefined));
        }
        let message = arguments
            .first()
            .filter(|value| !matches!(value, JsValue::Undefined))
            .map(JsValue::to_js_string)
            .unwrap_or_default();
        let (name, prototype) = match builtin {
            BuiltinFunction::Error => ("Error", self.error_prototype),
            BuiltinFunction::TypeError => ("TypeError", self.type_error_prototype),
            BuiltinFunction::ReferenceError => ("ReferenceError", self.reference_error_prototype),
            BuiltinFunction::SyntaxError => ("SyntaxError", self.syntax_error_prototype),
            BuiltinFunction::DomGetElementById
            | BuiltinFunction::DomGetElementsByTagName
            | BuiltinFunction::DomQuerySelector
            | BuiltinFunction::DomQuerySelectorAll
            | BuiltinFunction::DomMatches
            | BuiltinFunction::DomClosest
            | BuiltinFunction::DomHasAttribute
            | BuiltinFunction::DomHasAttributes
            | BuiltinFunction::DomCreateElement
            | BuiltinFunction::DomCreateTextNode
            | BuiltinFunction::DomAppendChild
            | BuiltinFunction::DomInsertBefore
            | BuiltinFunction::DomRemoveChild
            | BuiltinFunction::DomReplaceChild
            | BuiltinFunction::DomRemoveSelf
            | BuiltinFunction::DomClassAdd
            | BuiltinFunction::DomClassRemove
            | BuiltinFunction::DomClassContains
            | BuiltinFunction::DomClassToggle
            | BuiltinFunction::DomNodeListItem
            | BuiltinFunction::DomHtmlCollectionNamedItem
            | BuiltinFunction::DomContains
            | BuiltinFunction::DomClick
            | BuiltinFunction::DomDispatchEvent
            | BuiltinFunction::DomEventConstructor
            | BuiltinFunction::DomCreateEvent
            | BuiltinFunction::DomInitEvent
            | BuiltinFunction::DomStyleSetProperty
            | BuiltinFunction::DomStyleGetPropertyValue
            | BuiltinFunction::DomStyleRemoveProperty
            | BuiltinFunction::DomSetAttribute
            | BuiltinFunction::DomGetAttribute
            | BuiltinFunction::DomRemoveAttribute
            | BuiltinFunction::DomAddEventListener
            | BuiltinFunction::DomRemoveEventListener
            | BuiltinFunction::EventStopPropagation
            | BuiltinFunction::EventPreventDefault
            | BuiltinFunction::LifecycleAddEventListener
            | BuiltinFunction::LifecycleRemoveEventListener
            | BuiltinFunction::SetTimeout
            | BuiltinFunction::ClearTimeout
            | BuiltinFunction::SetInterval
            | BuiltinFunction::ClearInterval
            | BuiltinFunction::QueueMicrotask
            | BuiltinFunction::OpFetchText
            | BuiltinFunction::HeadersConstructor
            | BuiltinFunction::HeadersGet
            | BuiltinFunction::HeadersHas
            | BuiltinFunction::HeadersSet
            | BuiltinFunction::HeadersAppend
            | BuiltinFunction::HeadersDelete
            | BuiltinFunction::JsonParse
            | BuiltinFunction::JsonStringify
            | BuiltinFunction::StringCharAt
            | BuiltinFunction::ArrayPush
            | BuiltinFunction::ArrayPop
            | BuiltinFunction::ObjectConstructor
            | BuiltinFunction::ArrayConstructor
            | BuiltinFunction::BooleanConstructor
            | BuiltinFunction::NumberConstructor
            | BuiltinFunction::StringConstructor
            | BuiltinFunction::IsNaN
            | BuiltinFunction::IsFinite
            | BuiltinFunction::ArrayIsArray
            | BuiltinFunction::ArrayOf
            | BuiltinFunction::NumberIsNaN
            | BuiltinFunction::NumberIsFinite
            | BuiltinFunction::ObjectIs
            | BuiltinFunction::ObjectPrototypeValueOf
            | BuiltinFunction::ObjectPrototypeToString
            | BuiltinFunction::BoxedPrimitiveValueOf
            | BuiltinFunction::BoxedPrimitiveToString => {
                unreachable!("handled above")
            }
        };
        let error = self.allocate_error_object(name, &message, prototype)?;
        Ok(CallOutcome::Value(JsValue::Object(error)))
    }

    fn construct_value(
        &mut self,
        callee: JsValue,
        arguments: Vec<JsValue>,
        steps: &mut usize,
        call_depth: usize,
    ) -> Result<CallOutcome, JsError> {
        let JsValue::Object(function_id) = callee.clone() else {
            return Err(JsError::type_error("value is not a constructor"));
        };
        let function = self
            .object(function_id)?
            .function
            .clone()
            .ok_or_else(|| JsError::type_error("value is not a constructor"))?;

        if let FunctionImplementation::Builtin(builtin) = function.implementation {
            if matches!(
                builtin,
                BuiltinFunction::ArrayIsArray
                    | BuiltinFunction::ArrayOf
                    | BuiltinFunction::StringCharAt
                    | BuiltinFunction::ArrayPush
                    | BuiltinFunction::ArrayPop
                    | BuiltinFunction::NumberIsNaN
                    | BuiltinFunction::NumberIsFinite
                    | BuiltinFunction::ObjectIs
            ) {
                return Err(JsError::type_error("built-in method is not a constructor"));
            }
            if matches!(
                builtin,
                BuiltinFunction::BooleanConstructor
                    | BuiltinFunction::NumberConstructor
                    | BuiltinFunction::StringConstructor
            ) {
                let class = match builtin {
                    BuiltinFunction::BooleanConstructor => "Boolean",
                    BuiltinFunction::NumberConstructor => "Number",
                    _ => "String",
                };
                let primitive = match self.call_value(
                    callee,
                    JsValue::Undefined,
                    arguments,
                    steps,
                    call_depth,
                )? {
                    CallOutcome::Value(value) => value,
                    CallOutcome::Thrown(value) => return Ok(CallOutcome::Thrown(value)),
                };
                return Ok(CallOutcome::Value(self.box_primitive(primitive, class)?));
            }
            return self.call_value(callee, JsValue::Undefined, arguments, steps, call_depth);
        }

        let prototype = match self.get_object_property(function_id, "prototype")? {
            JsValue::Object(id) => Some(id),
            _ => Some(self.object_prototype),
        };
        let receiver = self.allocate_object(ObjectKind::Ordinary, prototype, HashMap::new())?;
        match self.call_value(
            callee,
            JsValue::Object(receiver),
            arguments,
            steps,
            call_depth,
        )? {
            CallOutcome::Value(JsValue::Object(id)) => Ok(CallOutcome::Value(JsValue::Object(id))),
            CallOutcome::Value(_) => Ok(CallOutcome::Value(JsValue::Object(receiver))),
            CallOutcome::Thrown(value) => Ok(CallOutcome::Thrown(value)),
        }
    }

    pub fn global(&self, name: &str) -> Option<&JsValue> {
        Some(
            &self.environments[self.global_env.0]
                .bindings
                .get(name)?
                .value,
        )
    }

    fn load_binding(&self, start: EnvironmentId, name: &str) -> Result<&JsValue, JsError> {
        let environment = self
            .find_binding_environment(start, name)
            .ok_or_else(|| JsError::reference(format!("{name} is not defined")))?;
        Ok(&self.environments[environment.0].bindings[name].value)
    }

    fn assign_binding(
        &mut self,
        start: EnvironmentId,
        name: &str,
        value: JsValue,
    ) -> Result<(), JsError> {
        // Classic scripts currently execute in sloppy mode. An assignment
        // to an unresolvable reference creates a mutable global binding.
        // Strict/module scripts must reject this when strict mode lands.
        let environment = if let Some(env) = self.find_binding_environment(start, name) {
            env
        } else {
            self.install_global_binding(name, value, true, VariableKind::Var);
            return Ok(());
        };
        let binding = self.environments[environment.0]
            .bindings
            .get_mut(name)
            .expect("resolved environment must contain binding");
        if !binding.mutable {
            return Err(JsError::type_error(format!(
                "assignment to constant variable {name}"
            )));
        }
        binding.value = value;
        Ok(())
    }

    fn declare_binding(
        &mut self,
        current: EnvironmentId,
        name: &str,
        kind: VariableKind,
        has_initializer: bool,
        value: JsValue,
    ) -> Result<(), JsError> {
        let target = if kind == VariableKind::Var {
            self.nearest_var_environment(current)?
        } else {
            current
        };

        if let Some(existing) = self.environments[target.0].bindings.get_mut(name) {
            if kind == VariableKind::Var && existing.declaration_kind == VariableKind::Var {
                if has_initializer {
                    existing.value = value;
                }
                return Ok(());
            }
            return Err(JsError::syntax(
                0,
                format!("identifier {name} has already been declared"),
            ));
        }

        self.environments[target.0].bindings.insert(
            name.to_owned(),
            Binding {
                value,
                mutable: kind != VariableKind::Const,
                declaration_kind: kind,
            },
        );
        Ok(())
    }

    fn find_binding_environment(&self, start: EnvironmentId, name: &str) -> Option<EnvironmentId> {
        let mut current = Some(start);
        let mut remaining = self.environments.len().saturating_add(1);
        while let Some(id) = current {
            if remaining == 0 {
                return None;
            }
            remaining -= 1;
            let environment = self.environments.get(id.0)?;
            if environment.bindings.contains_key(name) {
                return Some(id);
            }
            current = environment.parent;
        }
        None
    }

    fn nearest_var_environment(&self, start: EnvironmentId) -> Result<EnvironmentId, JsError> {
        let mut current = Some(start);
        let mut remaining = self.environments.len().saturating_add(1);
        while let Some(id) = current {
            if remaining == 0 {
                break;
            }
            remaining -= 1;
            let environment = self
                .environments
                .get(id.0)
                .ok_or_else(|| JsError::type_error("invalid lexical environment"))?;
            if matches!(
                environment.kind,
                EnvironmentKind::Global | EnvironmentKind::Function
            ) {
                return Ok(id);
            }
            current = environment.parent;
        }
        Err(JsError::type_error("missing function/global environment"))
    }

    fn allocate_environment(
        &mut self,
        parent: Option<EnvironmentId>,
        kind: EnvironmentKind,
    ) -> Result<EnvironmentId, JsError> {
        if self.environments.len() >= self.environment_budget {
            return Err(JsError::execution_limit(format!(
                "runtime exceeded lexical environment budget of {}",
                self.environment_budget
            )));
        }
        if let Some(parent) = parent
            && self.environments.get(parent.0).is_none()
        {
            return Err(JsError::type_error("invalid parent lexical environment"));
        }
        let id = EnvironmentId(self.environments.len());
        self.environments.push(Environment {
            parent,
            kind,
            bindings: HashMap::new(),
        });
        Ok(id)
    }

    fn parent_environment(&self, id: EnvironmentId) -> Result<EnvironmentId, JsError> {
        self.environments
            .get(id.0)
            .and_then(|environment| environment.parent)
            .ok_or_else(|| JsError::type_error("cannot exit root lexical environment"))
    }

    pub fn get_property(&mut self, target: &JsValue, key: &str) -> Result<JsValue, JsError> {
        match target {
            JsValue::Object(id) => {
                let kind = self.object(*id)?.kind;
                match kind {
                    ObjectKind::DomEvent if key == "returnValue" => {
                        let prevented = self
                            .object(*id)?
                            .properties
                            .get("defaultPrevented")
                            .is_some_and(JsValue::is_truthy);
                        return Ok(JsValue::Boolean(!prevented));
                    }
                    ObjectKind::DomElement(node) | ObjectKind::DomText(node) => {
                        if key == "nodeName" {
                            let name = if matches!(kind, ObjectKind::DomText(_)) {
                                "#text".to_owned()
                            } else {
                                self.dom_tags
                                    .get(&node)
                                    .cloned()
                                    .unwrap_or_default()
                                    .to_ascii_uppercase()
                            };
                            return Ok(JsValue::String(name));
                        }
                        if key == "tagName" && matches!(kind, ObjectKind::DomElement(_)) {
                            let tag = self
                                .dom_tags
                                .get(&node)
                                .cloned()
                                .unwrap_or_default()
                                .to_ascii_uppercase();
                            return Ok(JsValue::String(tag));
                        }
                        if key == "isConnected" {
                            let mut cursor = Some(node);
                            for _ in 0..=20_000 {
                                let Some(current) = cursor else {
                                    return Ok(JsValue::Boolean(false));
                                };
                                if current == 0 {
                                    return Ok(JsValue::Boolean(true));
                                }
                                cursor = self.dom_pending_parents.get(&current).copied();
                            }
                            return Err(JsError::execution_limit("DOM ancestor depth exceeded"));
                        }
                        if key == "previousSibling" || key == "nextSibling" {
                            let parent = self.dom_pending_parents.get(&node).copied();
                            let siblings = parent.and_then(|parent| self.dom_children.get(&parent));
                            let idx = siblings
                                .and_then(|siblings| siblings.iter().position(|&n| n == node));
                            let other = match (siblings, idx) {
                                (Some(siblings), Some(idx))
                                    if key == "previousSibling" && idx > 0 =>
                                {
                                    siblings.get(idx - 1).copied()
                                }
                                (Some(siblings), Some(idx)) if key == "nextSibling" => {
                                    siblings.get(idx + 1).copied()
                                }
                                _ => None,
                            };
                            return match other {
                                Some(other) => self.dom_any_node_object(other).map(JsValue::Object),
                                None => Ok(JsValue::Null),
                            };
                        }
                        if key == "nodeValue" && matches!(kind, ObjectKind::DomElement(_)) {
                            return Ok(JsValue::Null);
                        }
                        if key == "ownerDocument" {
                            return Ok(self
                                .dom_document
                                .map(JsValue::Object)
                                .unwrap_or(JsValue::Null));
                        }
                        if key == "length" && matches!(kind, ObjectKind::DomText(_)) {
                            return Ok(JsValue::Number(
                                self.dom_text
                                    .get(&node)
                                    .map_or(0, |value| value.encode_utf16().count())
                                    as f64,
                            ));
                        }
                        if key == "parentNode" {
                            return match self.dom_pending_parents.get(&node).copied() {
                                Some(parent) if parent == 0 && self.dom_document.is_some() => {
                                    Ok(JsValue::Object(self.dom_document.expect("present")))
                                }
                                Some(parent) => {
                                    self.dom_any_node_object(parent).map(JsValue::Object)
                                }
                                None => Ok(JsValue::Null),
                            };
                        }
                        if key == "childNodes" {
                            return self.dom_node_list_object(node).map(JsValue::Object);
                        }
                        if matches!(kind, ObjectKind::DomElement(_)) {
                            if key == "children" {
                                return self.dom_html_collection_object(node).map(JsValue::Object);
                            }
                            if key == "childElementCount" {
                                let children = self.dom_children.get(&node);
                                let count = children.map_or(0, |children| {
                                    children
                                        .iter()
                                        .filter(|id| self.dom_tags.contains_key(id))
                                        .count()
                                });
                                return Ok(JsValue::Number(count as f64));
                            }
                            if key == "firstElementChild" || key == "lastElementChild" {
                                let children = self.dom_children.get(&node);
                                let child = if key == "firstElementChild" {
                                    children.and_then(|children| {
                                        children
                                            .iter()
                                            .copied()
                                            .find(|id| self.dom_tags.contains_key(id))
                                    })
                                } else {
                                    children.and_then(|children| {
                                        children
                                            .iter()
                                            .rev()
                                            .copied()
                                            .find(|id| self.dom_tags.contains_key(id))
                                    })
                                };
                                return match child {
                                    Some(child) => {
                                        self.dom_any_node_object(child).map(JsValue::Object)
                                    }
                                    None => Ok(JsValue::Null),
                                };
                            }
                            if key == "previousElementSibling" || key == "nextElementSibling" {
                                let parent = self.dom_pending_parents.get(&node).copied();
                                let siblings =
                                    parent.and_then(|parent| self.dom_children.get(&parent));
                                let other = siblings.and_then(|siblings| {
                                    let index = siblings.iter().position(|&other| other == node)?;
                                    if key == "previousElementSibling" {
                                        siblings[..index]
                                            .iter()
                                            .rev()
                                            .copied()
                                            .find(|other| self.dom_tags.contains_key(other))
                                    } else {
                                        siblings[index + 1..]
                                            .iter()
                                            .copied()
                                            .find(|other| self.dom_tags.contains_key(other))
                                    }
                                });
                                return match other {
                                    Some(other) => {
                                        self.dom_any_node_object(other).map(JsValue::Object)
                                    }
                                    None => Ok(JsValue::Null),
                                };
                            }
                        }
                        if key == "firstChild" || key == "lastChild" {
                            let children = self.dom_children.get(&node);
                            let child = if key == "firstChild" {
                                children.and_then(|c| c.first()).copied()
                            } else {
                                children.and_then(|c| c.last()).copied()
                            };
                            return match child {
                                Some(child) => self.dom_any_node_object(child).map(JsValue::Object),
                                None => Ok(JsValue::Null),
                            };
                        }
                        if key == "nodeType" {
                            return Ok(JsValue::Number(
                                if matches!(kind, ObjectKind::DomText(_)) {
                                    3.0
                                } else {
                                    1.0
                                },
                            ));
                        }
                        if key == "className" && matches!(kind, ObjectKind::DomElement(_)) {
                            return Ok(JsValue::String(
                                self.dom_attributes
                                    .get(&node)
                                    .and_then(|attrs| attrs.get("class"))
                                    .cloned()
                                    .unwrap_or_default(),
                            ));
                        }
                        if key == "classList" && matches!(kind, ObjectKind::DomElement(_)) {
                            return self.dom_class_list_object(node).map(JsValue::Object);
                        }
                        if key == "style" && matches!(kind, ObjectKind::DomElement(_)) {
                            return self.dom_style_object(node).map(JsValue::Object);
                        }
                    }
                    ObjectKind::DomClassList(node) => {
                        let tokens = self.class_tokens(node);
                        if key == "length" {
                            return Ok(JsValue::Number(tokens.len() as f64));
                        }
                        if key == "value" {
                            return Ok(JsValue::String(
                                self.dom_attributes
                                    .get(&node)
                                    .and_then(|attrs| attrs.get("class"))
                                    .cloned()
                                    .unwrap_or_default(),
                            ));
                        }
                        if let Ok(index) = key.parse::<usize>() {
                            return Ok(tokens
                                .get(index)
                                .cloned()
                                .map(JsValue::String)
                                .unwrap_or(JsValue::Undefined));
                        }
                    }
                    ObjectKind::DomStyle(node) => {
                        if key == "cssText" {
                            return Ok(JsValue::String(
                                self.dom_attributes
                                    .get(&node)
                                    .and_then(|attrs| attrs.get("style"))
                                    .cloned()
                                    .unwrap_or_default(),
                            ));
                        }
                        if !matches!(key, "setProperty" | "getPropertyValue" | "removeProperty") {
                            let name = Self::style_property_name(key);
                            let value = self
                                .dom_style_declarations(node)
                                .into_iter()
                                .find(|(name_key, _)| name_key == &name)
                                .map_or_else(String::new, |(_, value)| value);
                            return Ok(JsValue::String(value));
                        }
                    }
                    ObjectKind::DomHtmlCollection(node) => {
                        if key == "length" {
                            return Ok(JsValue::Number(
                                self.html_collection_nodes(node).len() as f64
                            ));
                        }
                        if let Ok(index) = key.parse::<usize>() {
                            return match self.html_collection_nodes(node).get(index).copied() {
                                Some(child) => self.dom_any_node_object(child).map(JsValue::Object),
                                None => Ok(JsValue::Undefined),
                            };
                        }
                        if !matches!(key, "item" | "namedItem")
                            && let Some(child) = self.html_collection_named(node, key)
                        {
                            return self.dom_any_node_object(child).map(JsValue::Object);
                        }
                    }
                    ObjectKind::DomQueryNodeList => {
                        let nodes = self.dom_query_lists.get(id);
                        if key == "length" {
                            return Ok(JsValue::Number(nodes.map_or(0, Vec::len) as f64));
                        }
                        if let Ok(index) = key.parse::<usize>() {
                            return match nodes.and_then(|nodes| nodes.get(index)).copied() {
                                Some(node) => self.dom_any_node_object(node).map(JsValue::Object),
                                None => Ok(JsValue::Undefined),
                            };
                        }
                    }
                    ObjectKind::DomTagCollection => {
                        let (root, tag) = self
                            .dom_tag_collections
                            .get(id)
                            .ok_or_else(|| JsError::type_error("missing tag collection"))?;
                        let matching = self.elements_by_tag(*root, tag);
                        if key == "length" {
                            return Ok(JsValue::Number(matching.len() as f64));
                        }
                        if let Ok(index) = key.parse::<usize>() {
                            return match matching.get(index).copied() {
                                Some(node) => self.dom_any_node_object(node).map(JsValue::Object),
                                None => Ok(JsValue::Undefined),
                            };
                        }
                    }
                    ObjectKind::DomNodeList(node) => {
                        let children = self.dom_children.get(&node);
                        if key == "length" {
                            return Ok(JsValue::Number(children.map_or(0, Vec::len) as f64));
                        }
                        if let Ok(index) = key.parse::<usize>() {
                            let child = children.and_then(|c| c.get(index)).copied();
                            return match child {
                                Some(child) => self.dom_any_node_object(child).map(JsValue::Object),
                                None => Ok(JsValue::Undefined),
                            };
                        }
                    }
                    _ => {}
                }
                if key == "length"
                    && let Some(JsValue::String(value)) = self.boxed_values.get(id)
                {
                    return Ok(JsValue::Number(value.encode_utf16().count() as f64));
                }
                if matches!(key, "textContent" | "nodeValue" | "data")
                    && let ObjectKind::DomText(node) = self.object(*id)?.kind
                {
                    return Ok(JsValue::String(
                        self.dom_text.get(&node).cloned().unwrap_or_default(),
                    ));
                }
                if key == "textContent"
                    && let ObjectKind::DomElement(node) = self.object(*id)?.kind
                {
                    return Ok(JsValue::String(
                        self.dom_text.get(&node).cloned().unwrap_or_default(),
                    ));
                }
                if key == "__proto__" {
                    let object = self.object(*id)?;
                    if let Some(value) = object.properties.get(key) {
                        return Ok(value.clone());
                    }
                    return Ok(object
                        .prototype
                        .map(JsValue::Object)
                        .unwrap_or(JsValue::Null));
                }
                self.get_object_property(*id, key)
            }
            JsValue::String(value) if key == "length" => {
                Ok(JsValue::Number(value.encode_utf16().count() as f64))
            }
            JsValue::String(_) => {
                let Some(JsValue::Object(string_ctor)) = self.global("String") else {
                    return Ok(JsValue::Undefined);
                };
                let JsValue::Object(prototype) =
                    self.get_object_property(*string_ctor, "prototype")?
                else {
                    return Ok(JsValue::Undefined);
                };
                self.get_object_property(prototype, key)
            }
            JsValue::Null | JsValue::Undefined => Err(JsError::type_error(
                "cannot read properties of null or undefined",
            )),
            _ => Ok(JsValue::Undefined),
        }
    }

    fn set_property(&mut self, target: &JsValue, key: &str, value: JsValue) -> Result<(), JsError> {
        let JsValue::Object(id) = target else {
            return match target {
                JsValue::Null | JsValue::Undefined => Err(JsError::type_error(
                    "cannot set properties of null or undefined",
                )),
                _ => Err(JsError::type_error(
                    "property assignment on primitive values is not implemented",
                )),
            };
        };

        if key == "returnValue" && self.object(*id)?.kind == ObjectKind::DomEvent {
            if !value.is_truthy()
                && self
                    .object(*id)?
                    .properties
                    .get("cancelable")
                    .is_some_and(JsValue::is_truthy)
            {
                self.object_mut(*id)?
                    .properties
                    .insert("defaultPrevented".into(), JsValue::Boolean(true));
            }
            return Ok(());
        }
        if key == "readyState" && self.dom_document == Some(*id) {
            return Ok(());
        }
        if let ObjectKind::DomStyle(node) = self.object(*id)?.kind {
            if key == "cssText" {
                self.stage_dom_attribute(node, "style", value.to_js_string())?;
                return Ok(());
            }
            let name = Self::style_property_name(key);
            self.set_dom_style_property(node, &name, Some(value.to_js_string()))?;
            return Ok(());
        }
        if key == "className"
            && let ObjectKind::DomElement(node) = self.object(*id)?.kind
        {
            self.stage_dom_attribute(node, "class", value.to_js_string())?;
            return Ok(());
        }

        if key == "id"
            && let ObjectKind::DomElement(node) = self.object(*id)?.kind
        {
            let id_string = value.to_js_string();
            if id_string.len() > 512 || self.dom_operations.len() >= 256 {
                return Err(JsError::execution_limit(
                    "DOM id or mutation budget exceeded",
                ));
            }
            self.dom_ids.retain(|_, old| *old != node);
            self.dom_pending_ids.insert(node, id_string.clone());
            self.dom_attributes
                .entry(node)
                .or_default()
                .insert("id".into(), id_string.clone());
            if self.dom_attached.contains(&node) && !id_string.is_empty() {
                self.dom_ids.insert(id_string.clone(), node);
            }
            self.dom_operations.push(DomOperation::SetId {
                node,
                id: id_string,
            });
        }
        if key == "onclick"
            && let ObjectKind::DomElement(node) = self.object(*id)?.kind
        {
            match &value {
                JsValue::Null | JsValue::Undefined => {
                    self.dom_onclick.remove(&node);
                }
                JsValue::Object(function) if self.object(*function)?.function.is_some() => {
                    self.dom_onclick.insert(node, value.clone());
                }
                _ => return Err(JsError::type_error("onclick must be callable")),
            }
        }
        if (key == "textContent" || key == "nodeValue" || key == "data")
            && let ObjectKind::DomText(node) = self.object(*id)?.kind
        {
            if self.dom_operations.len() >= 256 {
                return Err(JsError::execution_limit(
                    "DOM text mutation budget exceeded",
                ));
            }
            let text = value.to_js_string();
            if text.len() > 64 * 1024 {
                return Err(JsError::execution_limit("DOM text length budget exceeded"));
            }
            self.dom_text.insert(node, text.clone());
            let mutation = DomTextMutation {
                node,
                text_content: text.clone(),
            };
            self.dom_mutations.push(mutation.clone());
            self.dom_operations.push(DomOperation::SetText(mutation));
            self.object_mut(*id)?
                .properties
                .insert("textContent".into(), JsValue::String(text.clone()));
            self.object_mut(*id)?
                .properties
                .insert("nodeValue".into(), JsValue::String(text.clone()));
            self.object_mut(*id)?
                .properties
                .insert("data".into(), JsValue::String(text));
            return Ok(());
        }
        if key == "textContent"
            && let ObjectKind::DomElement(node) = self.object(*id)?.kind
        {
            if self.dom_mutations.len() >= 256 {
                return Err(JsError::execution_limit("DOM mutation budget exceeded"));
            }
            let text = value.to_js_string();
            if text.len() > 64 * 1024 {
                return Err(JsError::execution_limit("DOM text length budget exceeded"));
            }
            self.dom_text.insert(node, text.clone());
            let mutation = DomTextMutation {
                node,
                text_content: text,
            };
            self.dom_mutations.push(mutation.clone());
            self.dom_operations.push(DomOperation::SetText(mutation));
        }
        if key == "__proto__" {
            if self.object(*id)?.properties.contains_key(key) {
                self.object_mut(*id)?
                    .properties
                    .insert(key.to_owned(), value);
                return Ok(());
            }
            let new_prototype = match value {
                JsValue::Object(prototype) => Some(prototype),
                JsValue::Null => None,
                _ => return Ok(()),
            };
            if let Some(prototype) = new_prototype
                && self.prototype_chain_contains(prototype, *id)?
            {
                return Err(JsError::type_error("cyclic object prototype value"));
            }
            self.object_mut(*id)?.prototype = new_prototype;
            return Ok(());
        }

        let is_array = self.object(*id)?.kind == ObjectKind::Array;
        let array_index = is_array.then(|| array_index(key)).flatten();
        self.object_mut(*id)?
            .properties
            .insert(key.to_owned(), value);

        if let Some(index) = array_index {
            let length = self
                .object(*id)?
                .properties
                .get("length")
                .and_then(|value| match value {
                    JsValue::Number(length) => Some(*length),
                    _ => None,
                })
                .unwrap_or(0.0);
            let required = index as f64 + 1.0;
            if required > length {
                self.object_mut(*id)?
                    .properties
                    .insert("length".into(), JsValue::Number(required));
            }
        }

        Ok(())
    }

    fn get_object_property(&self, start: ObjectId, key: &str) -> Result<JsValue, JsError> {
        let mut current = Some(start);
        let mut remaining = self.heap.len().saturating_add(1);
        while let Some(id) = current {
            if remaining == 0 {
                return Err(JsError::type_error("cyclic prototype chain"));
            }
            remaining -= 1;
            let object = self.object(id)?;
            if let Some(value) = object.properties.get(key) {
                return Ok(value.clone());
            }
            current = object.prototype;
        }
        Ok(JsValue::Undefined)
    }

    fn prototype_chain_contains(&self, start: ObjectId, needle: ObjectId) -> Result<bool, JsError> {
        let mut current = Some(start);
        let mut remaining = self.heap.len().saturating_add(1);
        while let Some(id) = current {
            if id == needle {
                return Ok(true);
            }
            if remaining == 0 {
                return Ok(true);
            }
            remaining -= 1;
            current = self.object(id)?.prototype;
        }
        Ok(false)
    }

    fn allocate_function(
        &mut self,
        template: Arc<FunctionTemplate>,
        closure: EnvironmentId,
    ) -> Result<ObjectId, JsError> {
        if self.environments.get(closure.0).is_none() {
            return Err(JsError::type_error("invalid function closure environment"));
        }
        let mut properties = HashMap::new();
        properties.insert(
            "length".into(),
            JsValue::Number(template.params.len() as f64),
        );
        properties.insert(
            "name".into(),
            JsValue::String(template.name.clone().unwrap_or_default()),
        );
        let function_id = self.allocate_object_with_function(
            ObjectKind::Function,
            Some(self.object_prototype),
            properties,
            Some(FunctionObject {
                implementation: FunctionImplementation::User { template, closure },
            }),
        )?;

        let mut prototype_properties = HashMap::new();
        prototype_properties.insert("constructor".into(), JsValue::Object(function_id));
        let prototype = self.allocate_object(
            ObjectKind::Ordinary,
            Some(self.object_prototype),
            prototype_properties,
        )?;
        self.object_mut(function_id)?
            .properties
            .insert("prototype".into(), JsValue::Object(prototype));
        Ok(function_id)
    }

    fn allocate_arguments_object(&mut self, arguments: &[JsValue]) -> Result<ObjectId, JsError> {
        let mut properties = HashMap::new();
        for (index, value) in arguments.iter().enumerate() {
            properties.insert(index.to_string(), value.clone());
        }
        properties.insert("length".into(), JsValue::Number(arguments.len() as f64));
        self.allocate_object(
            ObjectKind::Ordinary,
            Some(self.object_prototype),
            properties,
        )
    }

    fn install_global_binding(
        &mut self,
        name: &str,
        value: JsValue,
        mutable: bool,
        declaration_kind: VariableKind,
    ) {
        self.environments[self.global_env.0].bindings.insert(
            name.to_owned(),
            Binding {
                value: value.clone(),
                mutable,
                declaration_kind,
            },
        );
        self.heap[self.global_object.0]
            .properties
            .insert(name.to_owned(), value);
    }

    fn install_error_constructor(
        &mut self,
        name: &str,
        builtin: BuiltinFunction,
        prototype: ObjectId,
    ) -> Result<ObjectId, JsError> {
        let mut properties = HashMap::new();
        properties.insert("name".into(), JsValue::String(name.into()));
        properties.insert("length".into(), JsValue::Number(1.0));
        properties.insert("prototype".into(), JsValue::Object(prototype));
        let function = self.allocate_object_with_function(
            ObjectKind::Function,
            Some(self.object_prototype),
            properties,
            Some(FunctionObject {
                implementation: FunctionImplementation::Builtin(builtin),
            }),
        )?;
        self.object_mut(prototype)?
            .properties
            .insert("constructor".into(), JsValue::Object(function));
        self.install_global_binding(name, JsValue::Object(function), true, VariableKind::Var);
        Ok(function)
    }

    fn allocate_error_object(
        &mut self,
        name: &str,
        message: &str,
        prototype: ObjectId,
    ) -> Result<ObjectId, JsError> {
        let mut properties = HashMap::new();
        properties.insert("name".into(), JsValue::String(name.into()));
        properties.insert("message".into(), JsValue::String(message.into()));
        self.allocate_object(ObjectKind::Ordinary, Some(prototype), properties)
    }

    fn error_object_from_runtime_error(
        &mut self,
        error: &JsError,
    ) -> Result<Option<JsValue>, JsError> {
        let (name, prototype) = match error.kind {
            JsErrorKind::Type => ("TypeError", self.type_error_prototype),
            JsErrorKind::Reference => ("ReferenceError", self.reference_error_prototype),
            JsErrorKind::Exception => ("Error", self.error_prototype),
            JsErrorKind::Syntax | JsErrorKind::ExecutionLimit => return Ok(None),
        };
        let id = self.allocate_error_object(name, &error.message, prototype)?;
        Ok(Some(JsValue::Object(id)))
    }

    fn describe_thrown_value(&self, value: &JsValue) -> String {
        let JsValue::Object(id) = value else {
            return value.to_js_string();
        };
        let Ok(object) = self.object(*id) else {
            return value.to_js_string();
        };
        let name = object.properties.get("name").and_then(|value| match value {
            JsValue::String(value) => Some(value.as_str()),
            _ => None,
        });
        let message = object
            .properties
            .get("message")
            .and_then(|value| match value {
                JsValue::String(value) => Some(value.as_str()),
                _ => None,
            });
        match (name, message) {
            (Some(name), Some("")) => name.to_owned(),
            (Some(name), Some(message)) => format!("{name}: {message}"),
            _ => value.to_js_string(),
        }
    }

    fn allocate_object(
        &mut self,
        kind: ObjectKind,
        prototype: Option<ObjectId>,
        properties: HashMap<String, JsValue>,
    ) -> Result<ObjectId, JsError> {
        self.allocate_object_with_function(kind, prototype, properties, None)
    }

    fn allocate_object_with_function(
        &mut self,
        kind: ObjectKind,
        prototype: Option<ObjectId>,
        properties: HashMap<String, JsValue>,
        function: Option<FunctionObject>,
    ) -> Result<ObjectId, JsError> {
        if self.heap.len() >= self.object_budget {
            return Err(JsError::execution_limit(format!(
                "runtime exceeded object budget of {}",
                self.object_budget
            )));
        }
        if let Some(prototype) = prototype {
            self.object(prototype)?;
        }
        let id = ObjectId(self.heap.len());
        self.heap.push(JsObject {
            properties,
            prototype,
            kind,
            function,
        });
        Ok(id)
    }

    fn object(&self, id: ObjectId) -> Result<&JsObject, JsError> {
        self.heap
            .get(id.0)
            .ok_or_else(|| JsError::type_error("invalid object reference"))
    }

    fn object_mut(&mut self, id: ObjectId) -> Result<&mut JsObject, JsError> {
        self.heap
            .get_mut(id.0)
            .ok_or_else(|| JsError::type_error("invalid object reference"))
    }
}

fn to_property_key(value: JsValue) -> String {
    value.to_js_string()
}

fn array_index(key: &str) -> Option<u32> {
    if key.is_empty() || (key.len() > 1 && key.starts_with('0')) {
        return None;
    }
    let value = key.parse::<u32>().ok()?;
    if value == u32::MAX || value.to_string() != key {
        return None;
    }
    Some(value)
}

fn apply_update(op: UpdateOp, value: &JsValue) -> JsValue {
    let number = value.to_number();
    JsValue::Number(match op {
        UpdateOp::Increment => number + 1.0,
        UpdateOp::Decrement => number - 1.0,
    })
}

fn apply_unary(op: UnaryOp, value: JsValue) -> JsValue {
    match op {
        UnaryOp::Plus => JsValue::Number(value.to_number()),
        UnaryOp::Minus => JsValue::Number(-value.to_number()),
        UnaryOp::Not => JsValue::Boolean(!value.is_truthy()),
        UnaryOp::Void => JsValue::Undefined,
        UnaryOp::Typeof => unreachable!("typeof needs runtime heap or missing name inspection"),
    }
}

fn apply_binary(op: BinaryOp, left: JsValue, right: JsValue) -> JsValue {
    match op {
        BinaryOp::Add => {
            if matches!(left, JsValue::String(_)) || matches!(right, JsValue::String(_)) {
                JsValue::String(left.to_js_string() + &right.to_js_string())
            } else {
                JsValue::Number(left.to_number() + right.to_number())
            }
        }
        BinaryOp::Subtract => JsValue::Number(left.to_number() - right.to_number()),
        BinaryOp::Multiply => JsValue::Number(left.to_number() * right.to_number()),
        BinaryOp::Divide => JsValue::Number(left.to_number() / right.to_number()),
        BinaryOp::Remainder => JsValue::Number(left.to_number() % right.to_number()),
        BinaryOp::StrictEqual => JsValue::Boolean(strict_equal(&left, &right)),
        BinaryOp::StrictNotEqual => JsValue::Boolean(!strict_equal(&left, &right)),
        BinaryOp::Equal => JsValue::Boolean(abstract_equal(&left, &right)),
        BinaryOp::NotEqual => JsValue::Boolean(!abstract_equal(&left, &right)),
        BinaryOp::Less => JsValue::Boolean(compare(&left, &right, |a, b| a < b)),
        BinaryOp::LessEqual => JsValue::Boolean(compare(&left, &right, |a, b| a <= b)),
        BinaryOp::Greater => JsValue::Boolean(compare(&left, &right, |a, b| a > b)),
        BinaryOp::GreaterEqual => JsValue::Boolean(compare(&left, &right, |a, b| a >= b)),
        BinaryOp::InstanceOf => unreachable!("instanceof requires VM heap access"),
        BinaryOp::In => unreachable!("in requires VM has_property semantics"),
    }
}

fn strict_equal(left: &JsValue, right: &JsValue) -> bool {
    match (left, right) {
        (JsValue::Undefined, JsValue::Undefined) | (JsValue::Null, JsValue::Null) => true,
        (JsValue::Boolean(a), JsValue::Boolean(b)) => a == b,
        (JsValue::Number(a), JsValue::Number(b)) => !a.is_nan() && !b.is_nan() && a == b,
        (JsValue::String(a), JsValue::String(b)) => a == b,
        (JsValue::Object(a), JsValue::Object(b)) => a == b,
        _ => false,
    }
}

fn abstract_equal(left: &JsValue, right: &JsValue) -> bool {
    if strict_equal(left, right) {
        return true;
    }
    match (left, right) {
        (JsValue::Null, JsValue::Undefined) | (JsValue::Undefined, JsValue::Null) => true,
        (JsValue::Number(a), JsValue::String(_)) => {
            let b = right.to_number();
            !a.is_nan() && !b.is_nan() && *a == b
        }
        (JsValue::String(_), JsValue::Number(b)) => {
            let a = left.to_number();
            !a.is_nan() && !b.is_nan() && a == *b
        }
        (JsValue::Boolean(_), _) => abstract_equal(&JsValue::Number(left.to_number()), right),
        (_, JsValue::Boolean(_)) => abstract_equal(left, &JsValue::Number(right.to_number())),
        _ => false,
    }
}

fn compare(left: &JsValue, right: &JsValue, op: impl FnOnce(f64, f64) -> bool) -> bool {
    if let (JsValue::String(left), JsValue::String(right)) = (left, right) {
        return match left.cmp(right) {
            std::cmp::Ordering::Less => op(0.0, 1.0),
            std::cmp::Ordering::Equal => op(0.0, 0.0),
            std::cmp::Ordering::Greater => op(1.0, 0.0),
        };
    }
    op(left.to_number(), right.to_number())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evaluates_precedence_and_persistent_globals() {
        let mut runtime = JsRuntime::new();
        assert_eq!(
            runtime.eval_script("let x = 1 + 2 * 3; x").unwrap(),
            JsValue::Number(7.0)
        );
        assert_eq!(
            runtime.eval_script("x = x - 2; x / 5").unwrap(),
            JsValue::Number(1.0)
        );
        assert_eq!(runtime.global("x"), Some(&JsValue::Number(5.0)));
    }

    #[test]
    fn m419_string_char_at_handles_positions_and_boxed_receivers() {
        let mut vm = JsRuntime::new();
        let source = r#"
            var plain="Hello";
            var boxed=new String("Hi");
            plain.charAt(0)==="H" && plain.charAt(4)==="o" &&
            plain.charAt(5)==="" && plain.charAt(-1)==="" &&
            plain.charAt(1.99)==="e" && plain.charAt(NaN)==="H" &&
            boxed.charAt(1)==="i" && boxed.length===2 &&
            String.prototype.charAt.length===1 &&
            String.prototype.charAt(0)==="" &&
            typeof "hello".charAt==="function" &&
            (new Number(42)).toString()==="42"
        "#;
        assert_eq!(vm.eval_script(source).unwrap(), JsValue::Boolean(true));
        assert!(
            vm.eval_script(
                "var error=false; try{String.prototype.charAt(0)}catch(e){error=true}; error"
            )
            .is_ok()
        ); // prototype itself is a normal object, converted to text
    }

    #[test]
    fn m419_array_push_pop_preserves_length_holes_and_receiver_checks() {
        let mut vm = JsRuntime::new();
        let src = r#"
            var a=[1,2];
            var x=a.push(3,4);
            var y=a.pop();
            var z=a.pop();
            var n=a.pop();
            var m=a.pop();
            var missing=a.pop();
            var gap=new Array(2);
            var blank=gap.pop();
            var end=gap.push("X");
            x===4 && y===4 && z===3 && n===2 && m===1 &&
            missing===undefined && a.length===0 &&
            blank===undefined && gap.length===2 &&
            end===2 && gap[1]==="X" &&
            Array.prototype.push.length===1 &&
            Array.prototype.pop.length===0
        "#;
        assert_eq!(vm.eval_script(src).unwrap(), JsValue::Boolean(true));
        assert_eq!(
            vm.eval_script("var a=[]; a.pop()").unwrap(),
            JsValue::Undefined
        );
        assert!(vm.eval_script("Array.prototype.push(3)").is_err());
    }

    #[test]
    fn m419_syntax_error_is_constructor_and_json_parse_throws_it() {
        let mut vm = JsRuntime::new();
        assert_eq!(vm.eval_script(
            "var a=new SyntaxError('bad');\
             var caught=false;\
             try{JSON.parse('{bad')}catch(e){caught=e instanceof SyntaxError && e.name==='SyntaxError'}\
             caught && a instanceof SyntaxError && a instanceof Error && a.message==='bad'"
        ).unwrap(),JsValue::Boolean(true));
    }

    #[test]
    fn array_and_number_static_methods_preserve_types_and_edge_values() {
        let mut vm = JsRuntime::new();
        assert_eq!(
            vm.eval_script(
                "Array.isArray([]) && !Array.isArray({length:3}) && !Array.isArray() && \
             Array.of().length===0 && Array.of(5).length===1 && \
             Array.of(5)[0]===5 && Array.of(1,2,3)[2]===3 && \
             Number.isNaN(NaN) && !Number.isNaN('NaN') && \
             !Number.isNaN(undefined) && !Number.isNaN(0) && \
             Number.isFinite(1) && Number.isFinite(0) && \
             !Number.isFinite('1') && !Number.isFinite(Infinity) && \
             !Number.isFinite(NaN)"
            )
            .unwrap(),
            JsValue::Boolean(true)
        );
    }

    #[test]
    fn object_is_implements_same_value_negative_zero_nan_identity() {
        let mut vm = JsRuntime::new();
        assert_eq!(
            vm.eval_script(
                "var object={}; \
             Object.is(NaN,NaN) && !Object.is(0,-0) && Object.is(-0,-0) && \
             Object.is(0,0) && Object.is(object,object) && \
             !Object.is({}, {}) && !Object.is(1,'1') && \
             Object.is(undefined,undefined) && Object.is(null,null)"
            )
            .unwrap(),
            JsValue::Boolean(true)
        );
        assert_eq!(
            vm.eval_script("Object.is.length === 2 && Array.isArray.length === 1")
                .unwrap(),
            JsValue::Boolean(true)
        );
    }

    #[test]
    fn typeof_handles_unbound_names_functions_and_property_errors() {
        let mut vm = JsRuntime::new();
        assert_eq!(
            vm.eval_script(
                "typeof unknownName === 'undefined' && typeof undefined === 'undefined' && \
             typeof null === 'object' && typeof true === 'boolean' && \
             typeof 42 === 'number' && typeof 'hello' === 'string' && \
             typeof MathMissing === 'undefined' && \
             typeof (function(){}) === 'function' && \
             typeof JSON.parse === 'function' && \
             typeof ({x:1}) === 'object' && \
             typeof [1] === 'object'"
            )
            .unwrap(),
            JsValue::Boolean(true)
        );
        assert!(vm.eval_script("typeof (null).missing").is_err());
        assert_eq!(
            vm.eval_script("var counter=0; typeof (counter=9); counter")
                .unwrap(),
            JsValue::Number(9.0)
        );
    }

    #[test]
    fn conditional_operator_is_lazy_and_right_associative() {
        let mut vm = JsRuntime::new();
        assert_eq!(
            vm.eval_script(
                "var count=0; \
             var a=true ? 5 : (count=1); \
             var b=false ? (count=2) : 3; \
             var c=false ? 1 : true ? 7 : 8; \
             var d=true ? false ? 1 : 6 : 4; \
             count===0 && a===5 && b===3 && c===7 && d===6 && \
             (false || true ? 'yes':'no')==='yes'"
            )
            .unwrap(),
            JsValue::Boolean(true)
        );
        assert!(vm.eval_script("true ? 1;").is_err());
        assert_eq!(
            vm.eval_script("true ? 11 : missingName").unwrap(),
            JsValue::Number(11.0)
        );
    }

    #[test]
    fn boxed_primitives_have_identity_and_prototype_value_of() {
        let mut vm = JsRuntime::new();
        let script = r#"
            var a = new Number(3);
            var b = new Number(3);
            var c = new String("hey");
            var d = new Boolean(false);
            a !== b && a !== 3 && a == 3 &&
            a instanceof Number && !(a instanceof String) &&
            b instanceof Number && c instanceof String &&
            d instanceof Boolean && d.valueOf() === false &&
            d.toString() === "false" &&
            c.valueOf() === "hey" && c.toString() === "hey" &&
            a.valueOf() === 3 && a.toString() === "3" &&
            new Number(4) == 4 && (Object("foo") instanceof String) &&
            (Object(1) instanceof Number) &&
            Object(a) === a && Object(false) == false;
        "#;
        assert_eq!(vm.eval_script(script).unwrap(), JsValue::Boolean(true));
    }

    #[test]
    fn primitive_conversion_order_and_user_exception_semantics() {
        let mut vm = JsRuntime::new();
        let script = r#"
            var calls = "";
            var v = {
                valueOf: function(){ calls = calls + "v"; return {}; },
                toString: function(){ calls = calls + "s"; return "8"; }
            };
            var x = v + 1;
            var y = v == 8;
            var z = v >= "7";
            var thrown = "";
            try {
                ({ valueOf: function(){ throw "boom"; },
                   toString: function(){ return 1; }}) + 2;
            } catch (e) { thrown = e; }
            var bothThrow = false;
            try {
                ({ valueOf: function(){ return {}; },
                   toString: function(){ return {}; }}) + 2;
            } catch (e) { bothThrow = e instanceof TypeError; }
            x === "81" && y && z && calls === "vsvsvs" &&
                thrown === "boom" && bothThrow;
        "#;
        assert_eq!(vm.eval_script(script).unwrap(), JsValue::Boolean(true));
    }

    #[test]
    fn instanceof_void_and_sloppy_global_assignment() {
        let mut vm = JsRuntime::new();
        let source = r#"
            function Example(){ this.ready = true; }
            var entry = new Example();
            var side = 0;
            function createGlobal(){ missingGlobal = 13; }
            createGlobal();
            var empty = void (side = side + 2);
            var caught = false;
            try { entry instanceof 5; }
            catch(e) { caught = e instanceof TypeError; }
            var invalidProto = false;
            function Broken(){}
            Broken.prototype = 1;
            try { entry instanceof Broken; }
            catch(e) { invalidProto = e instanceof TypeError; }
            entry instanceof Example && !(entry instanceof Number) &&
            !("hey" instanceof String) && empty === undefined &&
            side === 2 && missingGlobal === 13 && caught && invalidProto;
        "#;
        assert_eq!(vm.eval_script(source).unwrap(), JsValue::Boolean(true));
    }

    #[test]
    fn invalid_boxed_primitive_receiver_throws() {
        let mut vm = JsRuntime::new();
        let source = r#"
            var method = Number.prototype.valueOf;
            var invalid = false;
            try { method.call(); } catch(e){invalid=e instanceof TypeError;}
            var invalid2=false;
            try { ({}).valueOf() === undefined; } catch(e) { invalid2=true; }
            // call/apply are still missing; the direct borrowed method
            // receives the non-Number global this.
            var invalid3=false;
            try { method(); } catch(e) {invalid3=e instanceof TypeError;}
            invalid3 && !invalid2;
        "#;
        // The test avoids requiring Function.prototype.call, which is
        // outside our implemented subset.
        assert_eq!(vm.eval_script(source).unwrap(), JsValue::Boolean(true));
    }

    #[test]
    fn standard_number_boolean_string_object_and_array_initial_slice() {
        let mut vm = JsRuntime::new();
        let result = vm
            .eval_script(
                "Boolean(0) === false && Boolean('x') === true && \
             Number() === 0 && Number('0xff') === 255 && \
             Number('0b101') === 5 && Number('0o11') === 9 && \
             String() === '' && String(42) === '42' && \
             isNaN('bad') && !isNaN('12') && \
             isFinite('0xf') && !isFinite(Infinity) && \
             Number.POSITIVE_INFINITY === Infinity && \
             Number.NEGATIVE_INFINITY === -Infinity && \
             Number.MAX_VALUE > 1 && Number.MIN_VALUE > 0 && \
             Array(3).length === 3 && Array('a')[0] === 'a' && \
             Object().x === undefined && Object({x:5}).x === 5",
            )
            .unwrap();
        assert_eq!(result, JsValue::Boolean(true));
        let error = vm.eval_script("Array(-1);").unwrap_err();
        assert_eq!(error.kind, JsErrorKind::Type);
        let error = vm.eval_script("Array(10000);").unwrap_err();
        assert_eq!(error.kind, JsErrorKind::ExecutionLimit);
    }

    #[test]
    fn supports_string_concat_truthiness_and_comparison() {
        let mut runtime = JsRuntime::new();
        assert_eq!(
            runtime.eval_script("'op' + 'browser'").unwrap(),
            JsValue::String("opbrowser".into())
        );
        assert_eq!(runtime.eval_script("!0").unwrap(), JsValue::Boolean(true));
        assert_eq!(
            runtime.eval_script("2 * 4 >= 8").unwrap(),
            JsValue::Boolean(true)
        );
        assert_eq!(
            runtime.eval_script("'2' == 2").unwrap(),
            JsValue::Boolean(true)
        );
        assert_eq!(
            runtime.eval_script("'2' === 2").unwrap(),
            JsValue::Boolean(false)
        );
    }

    #[test]
    fn dom_click_handlers_survive_scripts_and_mutate_text() {
        let mut vm = JsRuntime::new();
        vm.install_dom_snapshot([
            DomElementSnapshot {
                node: 1,
                id: "control".into(),
                text_content: "Click".into(),
            },
            DomElementSnapshot {
                node: 2,
                id: "result".into(),
                text_content: "Before".into(),
            },
        ])
        .unwrap();
        vm.eval_script("var button = document.getElementById('control'); var count=0;          button.addEventListener('click', function(event) {            count=count+1;            document.getElementById('result').textContent='count: '+count;          });").unwrap();
        assert!(vm.has_dom_click_listener(1));
        assert!(!vm.has_dom_click_listener(2));
        assert!(vm.dispatch_dom_click(1).unwrap());
        assert_eq!(vm.take_dom_mutations()[0].text_content, "count: 1");
        assert!(vm.dispatch_dom_click(1).unwrap());
        assert_eq!(vm.take_dom_mutations()[0].text_content, "count: 2");
        assert!(!vm.dispatch_dom_click(2).unwrap());
    }

    #[test]
    fn onclick_property_is_replaced_and_removed() {
        let mut vm = JsRuntime::new();
        vm.install_dom_snapshot([
            DomElementSnapshot {
                node: 3,
                id: "press".into(),
                text_content: "Press".into(),
            },
            DomElementSnapshot {
                node: 4,
                id: "status".into(),
                text_content: "Before".into(),
            },
        ])
        .unwrap();
        vm.eval_script("var item=document.getElementById('press');             item.onclick=function(){document.getElementById('status').textContent='one'};             item.onclick=function(){document.getElementById('status').textContent='two'};").unwrap();
        assert!(vm.dispatch_dom_click(3).unwrap());
        let updates = vm.take_dom_mutations();
        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].text_content, "two");
        vm.eval_script("item.onclick=null;").unwrap();
        assert!(!vm.has_dom_click_listener(3));
    }

    #[test]
    fn click_capture_target_and_bubble_run_in_dom_order() {
        let mut vm = JsRuntime::new();
        vm.install_dom_snapshot([
            DomElementSnapshot {
                node: 1,
                id: "root".into(),
                text_content: String::new(),
            },
            DomElementSnapshot {
                node: 2,
                id: "parent".into(),
                text_content: String::new(),
            },
            DomElementSnapshot {
                node: 3,
                id: "target".into(),
                text_content: String::new(),
            },
        ])
        .unwrap();
        vm.eval_script(
            "var log='';             var root=document.getElementById('root');             var parent=document.getElementById('parent');             var target=document.getElementById('target');             root.addEventListener('click',function(e){log=log+'RC'+e.eventPhase+';';},true);             parent.addEventListener('click',function(e){log=log+'PC'+e.eventPhase+';';},true);             target.addEventListener('click',function(e){log=log+'T'+e.eventPhase+';';});             parent.addEventListener('click',function(e){log=log+'PB'+e.eventPhase+';';});             root.addEventListener('click',function(e){log=log+'RB'+e.eventPhase+';';});"
        ).unwrap();
        assert!(vm.dispatch_dom_click_path(&[3, 2, 1]).unwrap());
        assert_eq!(
            vm.global("log"),
            Some(&JsValue::String("RC1;PC1;T2;PB3;RB3;".into()))
        );
    }

    #[test]
    fn stop_propagation_in_capture_blocks_target_and_prevent_default_sets_flag() {
        let mut vm = JsRuntime::new();
        vm.install_dom_snapshot([
            DomElementSnapshot {
                node: 1,
                id: "root".into(),
                text_content: String::new(),
            },
            DomElementSnapshot {
                node: 2,
                id: "target".into(),
                text_content: String::new(),
            },
        ])
        .unwrap();
        vm.eval_script(
            "var log=''; var prevented=false;             var root=document.getElementById('root');             var target=document.getElementById('target');             root.addEventListener('click',function(e){               e.preventDefault(); prevented=e.defaultPrevented;               e.stopPropagation(); log=log+'capture';             },true);             target.addEventListener('click',function(){log=log+'target';});"
        ).unwrap();
        assert!(vm.dispatch_dom_click_path(&[2, 1]).unwrap());
        assert_eq!(vm.global("log"), Some(&JsValue::String("capture".into())));
        assert_eq!(vm.global("prevented"), Some(&JsValue::Boolean(true)));
    }

    #[test]
    fn event_bubbles_with_original_target_and_removed_listener_stays_removed() {
        let mut vm = JsRuntime::new();
        vm.install_dom_snapshot([
            DomElementSnapshot {
                node: 10,
                id: "parent".into(),
                text_content: String::new(),
            },
            DomElementSnapshot {
                node: 11,
                id: "child".into(),
                text_content: String::new(),
            },
        ])
        .unwrap();
        vm.eval_script(
            "var parent=document.getElementById('parent');             var child=document.getElementById('child');             var seen='';             function obsolete(){seen='wrong';}             parent.addEventListener('click',obsolete);             parent.removeEventListener('click',obsolete);             child.addEventListener('click',function(e){               seen=e.target.id+':'+e.currentTarget.id+':'+e.eventPhase;             });             parent.addEventListener('click',function(e){               seen=seen+'|'+e.target.id+':'+e.currentTarget.id+':'+e.eventPhase;             });"
        ).unwrap();
        assert!(vm.dispatch_dom_click_path(&[11, 10]).unwrap());
        assert_eq!(
            vm.global("seen"),
            Some(&JsValue::String("child:child:2|child:parent:3".into()))
        );
        vm.eval_script("parent.removeEventListener('click',obsolete);")
            .unwrap();
        assert!(vm.has_dom_click_listener(10));
    }

    #[test]
    fn removing_the_last_listener_disables_click_dispatch() {
        let mut vm = JsRuntime::new();
        vm.install_dom_snapshot([DomElementSnapshot {
            node: 3,
            id: "press".into(),
            text_content: String::new(),
        }])
        .unwrap();
        vm.eval_script(
            "var el=document.getElementById('press');             function handler(){}             el.addEventListener('click',handler);             el.addEventListener('click',handler);             el.removeEventListener('click',handler);"
        ).unwrap();
        assert!(!vm.has_dom_click_listener(3));
        assert!(!vm.dispatch_dom_click(3).unwrap());
    }

    #[test]
    fn browser_dom_host_returns_objects_and_records_changes() {
        let mut runtime = JsRuntime::with_instruction_budget(10_000);
        runtime
            .install_dom_snapshot([DomElementSnapshot {
                node: 12,
                id: "headline".into(),
                text_content: "Old".into(),
            }])
            .unwrap();
        assert_eq!(
            runtime
                .eval_script("document.getElementById('headline').textContent")
                .unwrap(),
            JsValue::String("Old".into())
        );
        assert_eq!(
            runtime
                .eval_script("document.getElementById('absent')")
                .unwrap(),
            JsValue::Null
        );
        runtime
            .eval_script(
                "var heading=document.getElementById('headline');\
             heading.textContent='Updated';",
            )
            .unwrap();
        assert_eq!(
            runtime
                .eval_script("document.getElementById('headline').textContent")
                .unwrap(),
            JsValue::String("Updated".into())
        );
        assert_eq!(
            runtime.take_dom_mutations(),
            vec![DomTextMutation {
                node: 12,
                text_content: "Updated".into(),
            }]
        );
        assert!(runtime.take_dom_mutations().is_empty());
    }

    #[test]
    fn browser_dom_host_rejects_oversized_text() {
        let mut runtime = JsRuntime::new();
        runtime
            .install_dom_snapshot([DomElementSnapshot {
                node: 1,
                id: "entry".into(),
                text_content: String::new(),
            }])
            .unwrap();
        let source = format!(
            "document.getElementById('entry').textContent='{}';",
            "x".repeat(65537)
        );
        let err = runtime.eval_script(&source).unwrap_err();
        assert_eq!(err.kind, crate::JsErrorKind::ExecutionLimit);
        assert!(runtime.take_dom_mutations().is_empty());
    }

    #[test]
    fn const_assignment_and_unknown_names_fail() {
        let mut runtime = JsRuntime::new();
        runtime.eval_script("const answer = 42").unwrap();
        let error = runtime.eval_script("answer = 7").unwrap_err();
        assert_eq!(error.kind, crate::JsErrorKind::Type);

        let error = runtime.eval_script("missing + 1").unwrap_err();
        assert_eq!(error.kind, crate::JsErrorKind::Reference);
    }

    #[test]
    fn executes_control_flow_and_multiple_declarations() {
        let mut runtime = JsRuntime::new();
        assert_eq!(
            runtime
                .eval_script(
                    "let x = 0, total = 0; \
                     while (x < 10) { \
                       x = x + 1; \
                       if (x === 2) { continue; } \
                       if (x === 5) { break; } \
                       total = total + x; \
                     } \
                     total",
                )
                .unwrap(),
            JsValue::Number(8.0)
        );
        assert_eq!(runtime.global("x"), Some(&JsValue::Number(5.0)));
        assert_eq!(runtime.global("total"), Some(&JsValue::Number(8.0)));
    }

    #[test]
    fn executes_for_do_while_switch_fallthrough_break_and_continue() {
        let mut runtime = JsRuntime::new();
        assert_eq!(
            runtime
                .eval_script(
                    "let total = 0;                      for (let i = 0; i < 6; i++) {                        if (i === 2) continue;                        if (i === 5) break;                        total = total + i;                      }                      let d = 0; do { d = d + 1; } while (d < 3);                      total + d",
                )
                .unwrap(),
            JsValue::Number(11.0)
        );

        assert_eq!(
            runtime
                .eval_script(
                    "let out = 0; switch (2) {                        case 1: out = 1; break;                        case 2: out = 2;                        default: out = out + 3;                        case 4: out = out + 4;                      } out",
                )
                .unwrap(),
            JsValue::Number(9.0)
        );

        assert_eq!(
            runtime
                .eval_script(
                    "let loopSwitch = 0; for (let n = 0; n < 3; n++) {                        switch (n) { case 1: continue; default: loopSwitch = loopSwitch + n; }                      } loopSwitch",
                )
                .unwrap(),
            JsValue::Number(2.0)
        );
    }

    #[test]
    fn for_let_is_lexical_while_for_var_escapes_to_function_scope() {
        let mut runtime = JsRuntime::new();
        assert_eq!(
            runtime
                .eval_script("for (var j = 0; j < 2; j++) {} j")
                .unwrap(),
            JsValue::Number(2.0)
        );
        runtime
            .eval_script("for (let k = 0; k < 1; k++) {}")
            .unwrap();
        let error = runtime.eval_script("k").unwrap_err();
        assert_eq!(error.kind, crate::JsErrorKind::Reference);
    }

    #[test]
    fn prefix_and_postfix_updates_preserve_expression_values() {
        let mut runtime = JsRuntime::new();
        assert_eq!(
            runtime
                .eval_script(
                    "let i = 1; let a = i++; let b = ++i;                      let object = {x: 5}; let c = object.x--; let d = --object.x;                      (a === 1) && (b === 3) && (c === 5) && (d === 3) &&                      (i === 3) && (object.x === 3)",
                )
                .unwrap(),
            JsValue::Boolean(true)
        );
        let error = runtime
            .eval_script("const fixed = 1; fixed++;")
            .unwrap_err();
        assert_eq!(error.kind, crate::JsErrorKind::Type);
    }

    #[test]
    fn function_declarations_are_available_before_their_source_position() {
        let mut runtime = JsRuntime::new();
        assert_eq!(
            runtime
                .eval_script("let result = add(2, 3); function add(a, b) { return a + b; } result",)
                .unwrap(),
            JsValue::Number(5.0)
        );
        assert_eq!(
            runtime
                .eval_script(
                    "function outer() { return inner(); function inner() { return 7; } } outer()",
                )
                .unwrap(),
            JsValue::Number(7.0)
        );
    }

    #[test]
    fn explicit_throw_crosses_calls_and_catch_finally_obey_abrupt_completion() {
        let mut runtime = JsRuntime::new();
        assert_eq!(
            runtime
                .eval_script(
                    "function boom() { throw 7; }                      let x = 0;                      try { boom(); } catch (e) { x = e + 1; } finally { x = x + 2; }                      x",
                )
                .unwrap(),
            JsValue::Number(10.0)
        );

        assert_eq!(
            runtime
                .eval_script(
                    "function finalReturn() { try { return 1; } finally { return 2; } } finalReturn()",
                )
                .unwrap(),
            JsValue::Number(2.0)
        );
        assert_eq!(
            runtime
                .eval_script(
                    "function preserveReturn() { try { return 3; } finally { let ignored = 1; } } preserveReturn()",
                )
                .unwrap(),
            JsValue::Number(3.0)
        );

        let error = runtime
            .eval_script("try { throw 'boom'; } finally { 1; }")
            .unwrap_err();
        assert_eq!(error.kind, crate::JsErrorKind::Exception);
        assert_eq!(error.message, "boom");
    }

    #[test]
    fn finally_runs_before_break_and_continue_cross_try_boundaries() {
        let mut runtime = JsRuntime::new();
        assert_eq!(
            runtime
                .eval_script(
                    "let i = 0, seen = 0;                      while (i < 4) {                        i = i + 1;                        try {                          if (i === 2) continue;                          if (i === 4) break;                          seen = seen + i;                        } finally { seen = seen + 10; }                      }                      seen",
                )
                .unwrap(),
            JsValue::Number(44.0)
        );
    }

    #[test]
    fn block_let_const_shadow_while_var_uses_function_scope() {
        let mut runtime = JsRuntime::new();
        assert_eq!(
            runtime
                .eval_script("let x = 1; { let x = 2; const y = 3; } x",)
                .unwrap(),
            JsValue::Number(1.0)
        );
        assert_eq!(
            runtime
                .eval_script(
                    "function scoped() { { var inside = 7; let hidden = 9; } return inside; } scoped()",
                )
                .unwrap(),
            JsValue::Number(7.0)
        );
        let error = runtime.eval_script("hidden").unwrap_err();
        assert_eq!(error.kind, crate::JsErrorKind::Reference);
    }

    #[test]
    fn break_and_continue_unwind_nested_block_scopes() {
        let mut runtime = JsRuntime::new();
        assert_eq!(
            runtime
                .eval_script(
                    "let i = 0, total = 0; while (i < 6) { { i = i + 1; let local = i; if (i === 2) continue; if (i === 5) break; total = total + local; } } total",
                )
                .unwrap(),
            JsValue::Number(8.0)
        );
    }

    #[test]
    fn functions_accept_arguments_return_values_and_recurse() {
        let mut runtime = JsRuntime::new();
        assert_eq!(
            runtime
                .eval_script("function add(a, b) { return a + b; } add(4, 5)",)
                .unwrap(),
            JsValue::Number(9.0)
        );
        assert_eq!(
            runtime
                .eval_script(
                    "function fact(n) { if (n <= 1) return 1; return n * fact(n - 1); } fact(6)",
                )
                .unwrap(),
            JsValue::Number(720.0)
        );
        assert_eq!(
            runtime.eval_script("add.length").unwrap(),
            JsValue::Number(2.0)
        );
        assert_eq!(
            runtime.eval_script("add.name").unwrap(),
            JsValue::String("add".into())
        );
    }

    #[test]
    fn method_calls_bind_this_and_bare_calls_use_the_global_object() {
        let mut runtime = JsRuntime::new();
        assert_eq!(
            runtime
                .eval_script(
                    "function addToValue(extra) { return this.value + extra; }                      let object = {value: 5, addToValue};                      object.addToValue(3)",
                )
                .unwrap(),
            JsValue::Number(8.0)
        );
        assert_eq!(
            runtime
                .eval_script(
                    "this.marker = 11; function readGlobal() { return this.marker; } readGlobal()",
                )
                .unwrap(),
            JsValue::Number(11.0)
        );
    }

    #[test]
    fn arguments_is_array_like_and_available_inside_user_functions() {
        let mut runtime = JsRuntime::new();
        assert_eq!(
            runtime
                .eval_script(
                    "function inspect(first) { return arguments.length * 100 + arguments[0] * 10 + arguments[2]; } inspect(2, 4, 6)",
                )
                .unwrap(),
            JsValue::Number(326.0)
        );
    }

    #[test]
    fn constructors_use_function_prototypes_and_constructor_return_rules() {
        let mut runtime = JsRuntime::new();
        assert_eq!(
            runtime
                .eval_script(
                    "function Point(x, y) { this.x = x; this.y = y; }                      Point.prototype.sum = function() { return this.x + this.y; };                      let point = new Point(4, 5);                      point.sum()",
                )
                .unwrap(),
            JsValue::Number(9.0)
        );
        assert_eq!(
            runtime
                .eval_script("function Replace() { this.x = 1; return {x: 7}; } (new Replace()).x",)
                .unwrap(),
            JsValue::Number(7.0)
        );
        assert_eq!(
            runtime
                .eval_script("Point.prototype.constructor === Point")
                .unwrap(),
            JsValue::Boolean(true)
        );
    }

    #[test]
    fn runtime_type_and_reference_errors_become_catchable_error_objects() {
        let mut runtime = JsRuntime::new();
        assert_eq!(
            runtime
                .eval_script(
                    "let typeName = ''; try { null.x; } catch (e) { typeName = e.name; } typeName",
                )
                .unwrap(),
            JsValue::String("TypeError".into())
        );
        assert_eq!(
            runtime
                .eval_script(
                    "let refName = ''; try { missingName; } catch (e) { refName = e.name; } refName",
                )
                .unwrap(),
            JsValue::String("ReferenceError".into())
        );
        assert_eq!(
            runtime
                .eval_script(
                    "let error = new TypeError('bad input'); error.name + ': ' + error.message",
                )
                .unwrap(),
            JsValue::String("TypeError: bad input".into())
        );
        let error = runtime
            .eval_script("throw new ReferenceError('missing thing')")
            .unwrap_err();
        assert_eq!(error.kind, crate::JsErrorKind::Exception);
        assert_eq!(error.message, "ReferenceError: missing thing");
    }

    #[test]
    fn closures_capture_and_mutate_lexical_environments_after_return() {
        let mut runtime = JsRuntime::new();
        assert_eq!(
            runtime
                .eval_script(
                    "function makeCounter(start) { let value = start; return function(step) { value = value + step; return value; }; } let counter = makeCounter(10); counter(2); counter(3)",
                )
                .unwrap(),
            JsValue::Number(15.0)
        );
        assert_eq!(
            runtime.eval_script("counter(5)").unwrap(),
            JsValue::Number(20.0)
        );
    }

    #[test]
    fn closures_keep_exited_block_environments_alive() {
        let mut runtime = JsRuntime::new();
        assert_eq!(
            runtime
                .eval_script(
                    "let read; { let secret = 42; read = function() { return secret; }; } read()",
                )
                .unwrap(),
            JsValue::Number(42.0)
        );
    }

    #[test]
    fn named_function_expressions_can_recurse_without_leaking_the_name() {
        let mut runtime = JsRuntime::new();
        assert_eq!(
            runtime
                .eval_script(
                    "let factorial = function inner(n) { if (n <= 1) return 1; return n * inner(n - 1); }; factorial(5)",
                )
                .unwrap(),
            JsValue::Number(120.0)
        );
        let error = runtime.eval_script("inner").unwrap_err();
        assert_eq!(error.kind, crate::JsErrorKind::Reference);
    }

    #[test]
    fn calling_non_functions_and_excessive_recursion_fail_cleanly() {
        let mut runtime = JsRuntime::new();
        let error = runtime.eval_script("let x = 1; x()").unwrap_err();
        assert_eq!(error.kind, crate::JsErrorKind::Type);

        let error = runtime
            .eval_script("function recurse() { return recurse(); } recurse()")
            .unwrap_err();
        assert_eq!(error.kind, crate::JsErrorKind::ExecutionLimit);
    }

    #[test]
    fn logical_operators_short_circuit_and_preserve_operand_values() {
        let mut runtime = JsRuntime::new();
        assert_eq!(
            runtime.eval_script("false && missing").unwrap(),
            JsValue::Boolean(false)
        );
        assert_eq!(
            runtime.eval_script("true || missing").unwrap(),
            JsValue::Boolean(true)
        );
        assert_eq!(
            runtime
                .eval_script("let x = 0; false && (x = 1); true || (x = 2); x")
                .unwrap(),
            JsValue::Number(0.0)
        );
        assert_eq!(
            runtime.eval_script("0 || 'fallback'").unwrap(),
            JsValue::String("fallback".into())
        );
    }

    #[test]
    fn stops_runaway_control_flow_at_instruction_budget() {
        let mut runtime = JsRuntime::with_instruction_budget(100);
        let error = runtime.eval_script("while (true) {}").unwrap_err();
        assert_eq!(error.kind, crate::JsErrorKind::ExecutionLimit);
    }

    #[test]
    fn objects_preserve_identity_properties_and_assignment_values() {
        let mut runtime = JsRuntime::new();
        assert_eq!(
            runtime
                .eval_script(
                    "let x = 2; let a = {x, y: 3, 'name': 'op'}; let b = a; \
                     a.y = 7; a['z'] = a.y + 1; \
                     (a === b) && (a.x + a.y + a.z === 17)",
                )
                .unwrap(),
            JsValue::Boolean(true)
        );
        assert_eq!(
            runtime.eval_script("a.name + 'browser'").unwrap(),
            JsValue::String("opbrowser".into())
        );
        assert_eq!(
            runtime.eval_script("a.y = 9").unwrap(),
            JsValue::Number(9.0)
        );
    }

    #[test]
    fn prototype_chain_reads_inherited_properties_and_rejects_cycles() {
        let mut runtime = JsRuntime::new();
        assert_eq!(
            runtime
                .eval_script(
                    "let base = {answer: 42}; \
                     let child = {__proto__: base, own: 1}; \
                     child.answer + child.own",
                )
                .unwrap(),
            JsValue::Number(43.0)
        );
        assert_eq!(
            runtime.eval_script("child.__proto__ === base").unwrap(),
            JsValue::Boolean(true)
        );
        assert_eq!(
            runtime
                .eval_script("child.answer = 7; base.answer + child.answer")
                .unwrap(),
            JsValue::Number(49.0)
        );
        let error = runtime.eval_script("base.__proto__ = child").unwrap_err();
        assert_eq!(error.kind, crate::JsErrorKind::Type);
    }

    #[test]
    fn proto_shorthand_creates_an_ordinary_shadowing_property() {
        let mut runtime = JsRuntime::new();
        assert_eq!(
            runtime
                .eval_script("let __proto__ = 7; let object = {__proto__}; object.__proto__")
                .unwrap(),
            JsValue::Number(7.0)
        );
        assert_eq!(
            runtime
                .eval_script("object.__proto__ = 9; object.__proto__")
                .unwrap(),
            JsValue::Number(9.0)
        );
    }

    #[test]
    fn arrays_use_index_properties_holes_and_dynamic_length_growth() {
        let mut runtime = JsRuntime::new();
        assert_eq!(
            runtime
                .eval_script("let values = [1,,3]; values.length")
                .unwrap(),
            JsValue::Number(3.0)
        );
        assert_eq!(
            runtime.eval_script("values[1] === undefined").unwrap(),
            JsValue::Boolean(true)
        );
        assert_eq!(
            runtime.eval_script("values[5] = 9; values.length").unwrap(),
            JsValue::Number(6.0)
        );
        assert_eq!(
            runtime.eval_script("values[5]").unwrap(),
            JsValue::Number(9.0)
        );
    }

    #[test]
    fn nullish_property_access_is_a_type_error_and_string_length_is_utf16() {
        let mut runtime = JsRuntime::new();
        assert_eq!(
            runtime.eval_script("'A😀'.length").unwrap(),
            JsValue::Number(3.0)
        );
        let error = runtime.eval_script("null.x").unwrap_err();
        assert_eq!(error.kind, crate::JsErrorKind::Type);
    }
    #[test]
    fn microtasks_wait_until_click_dispatch_finishes() {
        let mut vm = JsRuntime::new();
        vm.install_dom_snapshot([DomElementSnapshot {
            node: 1,
            id: "button".into(),
            text_content: "Click".into(),
        }])
        .unwrap();
        vm.eval_script(
            "var order='';\
             document.getElementById('button').addEventListener('click',function(){\
             order=order+'A';\
             queueMicrotask(function(){order=order+'M';});\
             order=order+'B';});",
        )
        .unwrap();
        assert!(vm.dispatch_dom_click(1).unwrap());
        assert_eq!(vm.global("order"), Some(&JsValue::String("ABM".into())));
    }

    #[test]
    fn recursive_microtasks_are_bounded_and_worker_can_go_idle() {
        let mut vm = JsRuntime::new();
        vm.install_dom_snapshot([]).unwrap();
        vm.eval_script(
            "var count=0; function spin(){count=count+1;queueMicrotask(spin);}\
             queueMicrotask(spin);",
        )
        .unwrap();
        for _ in 0..5 {
            vm.run_due_timers(16);
        }
        assert!(vm.next_timer_wait().is_none());
        assert_eq!(vm.global("count"), Some(&JsValue::Number(1024.0)));
    }

    #[test]
    fn promise_executor_is_sync_but_reactions_are_fifo_microtasks() {
        let mut vm = JsRuntime::new();
        vm.eval_script(
            "var log=''; var p=new Promise(function(resolve){log=log+'E';resolve(3);});\
             p.then(function(x){log=log+'A'+x;return x+4;})\
              .then(function(x){log=log+'B'+x;});\
             queueMicrotask(function(){log=log+'M';});log=log+'S';",
        )
        .unwrap();
        assert_eq!(vm.global("log"), Some(&JsValue::String("ESA3MB7".into())));
    }

    #[test]
    fn pending_promise_settles_later_and_adopts_returned_promise() {
        let mut vm = JsRuntime::new();
        vm.eval_script(
            "var later; var log='';\
             var pending=new Promise(function(resolve){later=resolve;});\
             pending.then(function(value){return Promise.resolve(value+1);})\
                    .then(function(value){log=log+value;});",
        )
        .unwrap();
        assert_eq!(vm.global("log"), Some(&JsValue::String(String::new())));
        vm.eval_script("later(6);later(99);").unwrap();
        assert_eq!(vm.global("log"), Some(&JsValue::String("7".into())));
    }

    #[test]
    fn rejected_promise_catches_throw_and_finally_preserves_resolution() {
        let mut vm = JsRuntime::new();
        vm.eval_script(
            "var result='';\
             Promise.resolve(2).then(function(){throw 'bad';})\
                .catch(function(error){result=result+error;return 8;})\
                .finally(function(){result=result+'F';})\
                .then(function(value){result=result+value;});",
        )
        .unwrap();
        assert_eq!(vm.global("result"), Some(&JsValue::String("badF8".into())));
    }

    #[test]
    fn fetch_returns_promise_and_response_text_is_single_use() {
        let mut vm = JsRuntime::new();
        vm.install_dom_snapshot([]).unwrap();
        vm.eval_script(
            "var output='';\
             fetch('message.txt').then(function(response){\
                 output=output+response.ok+':'+response.status+':';\
                 var text=response.text();\
                 response.text().catch(function(error){output=output+'used;';});\
                 return text;\
             }).then(function(text){output=output+text;});",
        )
        .unwrap();
        let requests = vm.take_text_requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].url, "message.txt");
        assert!(!vm.complete_text_request(requests[0].id, Ok("body".into())));
        assert_eq!(
            vm.global("output"),
            Some(&JsValue::String("true:200:used;body".into()))
        );
    }

    #[test]
    fn fetch_response_headers_are_case_insensitive_and_http_error_is_fulfilled() {
        let mut vm = JsRuntime::new();
        vm.install_dom_snapshot([]).unwrap();
        vm.eval_script(
            "var result='';fetch('missing.txt').then(function(r){\
             result=r.status+'|'+r.ok+'|'+r.statusText+'|'+r.redirected+'|'+\
             r.headers.get('X-REQUEST-ID')+'|'+r.headers.has('x-request-id')+'|'+\
             r.headers.get('missing')+'|'+r.headers.get('set-cookie')+'|'+r.url;\
             return r.text();}).then(function(body){result=result+'|'+body;});",
        )
        .unwrap();
        let requests = vm.take_text_requests();
        assert_eq!(requests.len(), 1);
        assert!(requests[0].include_http_errors);
        assert!(!vm.complete_text_response_request(
            requests[0].id,
            Ok(TextResponse {
                text: "not found".into(),
                address: "https://site.test/missing.txt".into(),
                status: 404,
                status_text: "Not Found".into(),
                redirected: true,
                headers: vec![("x-request-id".into(), "abc".into())],
            }),
        ));
        assert_eq!(
            vm.global("result"),
            Some(&JsValue::String(
                "404|false|Not Found|true|abc|true|null|null|https://site.test/missing.txt|not found".into()
            ))
        );
    }

    #[test]
    fn constructed_headers_are_bounded_case_insensitive_and_copied() {
        let mut vm = JsRuntime::new();
        vm.install_dom_snapshot([]).unwrap();
        vm.eval_script(
            "var a = new Headers({'X-One':'first'});\
             a.append('x-one','second');\
             var b = new Headers(a);\
             a.set('x-one','changed');\
             var result=b.get('X-ONE')+'|'+a.get('x-one')+'|'+b.has('x-one');\
             a.delete('X-One');result=result+'|'+a.has('x-one');\
             var pair = new Headers([['X-Pair','array']]);\
             result=result+'|'+pair.get('x-pair');",
        )
        .unwrap();
        assert_eq!(
            vm.global("result"),
            Some(&JsValue::String(
                "first, second|changed|true|false|array".into()
            ))
        );
        assert!(
            vm.eval_script("new Headers({'bad header':'unsafe'});")
                .is_err()
        );
        assert!(
            vm.eval_script("new Headers({'X-OK':'hello\\r\\nInjected: yes'});")
                .is_err()
        );
    }

    #[test]
    fn request_init_is_filtered_before_host_dispatch_and_input_is_cloned() {
        let mut vm = JsRuntime::new();
        vm.install_dom_snapshot([]).unwrap();
        vm.eval_script(
            "var headers=new Headers({'X-Client':'one'});\
             var req=new Request('hello.txt',{method:'get',headers:headers,redirect:'error'});\
             headers.set('x-client','two');\
             var result=req.method+'|'+req.headers.get('x-client')+'|'+req.redirect;\
             fetch(req);\
             var errors='';\
             fetch('no.txt',{method:'POST'}).catch(function(e){errors=errors+'method;';});\
             fetch('no.txt',{headers:{Authorization:'bad'}})\
               .catch(function(e){errors=errors+'auth;';});\
             fetch('no.txt',{credentials:'include'})\
               .catch(function(e){errors=errors+'creds;';});\
             fetch('no.txt',{redirect:'manual'})\
               .catch(function(e){errors=errors+'redirect;';});",
        )
        .unwrap();
        let requests = vm.take_text_requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].url, "hello.txt");
        assert_eq!(
            requests[0].request_headers,
            vec![("x-client".into(), "one".into())]
        );
        assert!(requests[0].reject_redirect);
        assert_eq!(
            vm.global("result"),
            Some(&JsValue::String("GET|one|error".into()))
        );
        assert_eq!(
            vm.global("errors"),
            Some(&JsValue::String("method;auth;creds;redirect;".into()))
        );
    }

    #[test]
    fn json_native_parse_stringify_nested_arrays_and_safe_proto() {
        let mut vm = JsRuntime::new();
        vm.eval_script(
            r#"var obj=JSON.parse('{"person":{"name":"Привет","age":29},"array":[true,null,-1.5],"__proto__":{"safe":7}}');
            var result=obj.person.name+'|'+obj.array[0]+'|'+obj.array[2]+'|'+obj.__proto__.safe;
            var again=JSON.parse(JSON.stringify(obj));
            result=result+'|'+again.person.age+'|'+again.array.length;
            var hole=[1,,3];result=result+'|'+JSON.stringify(hole);
            var undef=JSON.stringify(undefined);result=result+'|'+(undef===undefined);
            var nonfinite=JSON.stringify([0/0,1/0]);result=result+'|'+nonfinite;"#,
        ).unwrap();
        assert_eq!(
            vm.global("result"),
            Some(&JsValue::String(
                "Привет|true|-1.5|7|29|3|[1,null,3]|true|[null,null]".into()
            ))
        );
    }

    #[test]
    fn json_errors_are_catchable_and_stringify_rejects_cycles() {
        let mut vm = JsRuntime::new();
        vm.eval_script(
            "var errors='';\
             try{JSON.parse('{bad:1}');}catch(e){errors=errors+e.name+';';}\
             try{JSON.parse('1e');}catch(e){errors=errors+e.name+';';}\
             var cycle={};cycle.self=cycle;\
             try{JSON.stringify(cycle);}catch(e){errors=errors+e.name+';';}\
             var obj={a:1,skip:undefined};\
             errors=errors+JSON.stringify(obj);",
        )
        .unwrap();
        assert_eq!(
            vm.global("errors"),
            Some(&JsValue::String(
                "SyntaxError;SyntaxError;TypeError;{\"a\":1}".into()
            ))
        );
    }

    #[test]
    fn promise_combinators_fulfill_reject_in_input_order() {
        let mut vm = JsRuntime::new();
        vm.eval_script(
            r#"var output='';
            Promise.all([Promise.resolve(2),3,Promise.resolve(5)])
              .then(function(a){output=output+'all:'+a[0]+a[1]+a[2]+';';});
            Promise.race([Promise.resolve('first'),Promise.resolve('second')])
              .then(function(x){output=output+'race:'+x+';';});
            Promise.allSettled([Promise.resolve('good'),Promise.reject('bad')])
              .then(function(a){output=output+'settled:'+a[0].status+'/'+a[1].reason+';';});
            Promise.any([Promise.reject('err'),Promise.resolve('ok')])
              .then(function(v){output=output+'any:'+v+';';});
            Promise.any([]).catch(function(e){output=output+'empty:'+e.name+';';});
            Promise.all([Promise.resolve(1),Promise.reject('FAIL')])
              .catch(function(e){output=output+'rejected:'+e+';';});"#,
        )
        .unwrap();
        let output = vm.global("output").unwrap().to_js_string();
        for expected in [
            "all:235;",
            "race:first;",
            "settled:fulfilled/bad;",
            "any:ok;",
            "empty:AggregateError;",
            "rejected:FAIL;",
        ] {
            assert!(
                output.contains(expected),
                "missing {expected:?} in {output:?}"
            );
        }
    }

    #[test]
    fn promise_all_waits_for_pending_inputs_and_race_empty_stays_pending() {
        let mut vm = JsRuntime::new();
        vm.eval_script(
            "var done='';var later;\
             var p=new Promise(function(resolve){later=resolve;});\
             Promise.all([p,Promise.resolve(2)]).then(function(items){\
                done=done+'all:'+items[0]+items[1]+';';\
             });\
             Promise.race([]).then(function(){done=done+'unexpected;';});\
             Promise.all([]).then(function(items){done=done+'empty:'+items.length+';';});\
             Promise.race([p,Promise.resolve('fast')]).then(function(x){\
                done=done+'race:'+x+';';\
             });",
        )
        .unwrap();
        assert_eq!(
            vm.global("done"),
            Some(&JsValue::String("empty:0;race:fast;".into()))
        );
        vm.eval_script("later(9);").unwrap();
        assert_eq!(
            vm.global("done"),
            Some(&JsValue::String("empty:0;race:fast;all:92;".into()))
        );
    }

    #[test]
    fn json_parser_rejects_excessive_depth_and_large_input() {
        let mut vm = JsRuntime::new();
        let deep = "[".repeat(70) + "0" + &"]".repeat(70);
        let input = format!("JSON.parse({});", crate::json::quote(&deep));
        let error = vm.eval_script(&input).unwrap_err().to_string();
        assert!(
            error.contains("SyntaxError") || error.contains("nesting"),
            "{error}"
        );
        assert!(crate::json::parse(&" ".repeat(65_537)).is_err());
        assert_eq!(
            crate::json::parse("1e999"),
            Ok(crate::json::JsonValue::Number(f64::INFINITY))
        );
    }

    #[test]
    fn json_response_consumption_and_invalid_json_reject_as_promise() {
        let mut vm = JsRuntime::new();
        vm.install_dom_snapshot([]).unwrap();
        vm.eval_script(
            r#"var result='';
            fetch('valid.json').then(function(r){
              var promise=r.json();
              r.text().catch(function(e){result=result+'used;';});
              return promise;
            }).then(function(data){result=result+data.answer+';';});
            fetch('bad.json').then(function(r){return r.json();})
              .catch(function(e){result=result+'bad:'+e.name+';';});"#,
        )
        .unwrap();
        let jobs = vm.take_text_requests();
        assert_eq!(jobs.len(), 2);
        assert!(!vm.complete_text_request(jobs[0].id, Ok("{\"answer\":42}".into())));
        assert!(!vm.complete_text_request(jobs[1].id, Ok("{bad}".into())));
        assert_eq!(
            vm.global("result"),
            Some(&JsValue::String("used;42;bad:SyntaxError;".into()))
        );
    }

    #[test]
    fn fetch_failure_rejects_and_unsupported_method_never_sends_request() {
        let mut vm = JsRuntime::new();
        vm.install_dom_snapshot([]).unwrap();
        vm.eval_script(
            "var errors='';\
             fetch('message.txt').catch(function(error){errors=errors+error.message;});\
             fetch('message.txt',{method:'POST'}).catch(function(error){\
                 errors=errors+'|'+error.message;\
             });",
        )
        .unwrap();
        let requests = vm.take_text_requests();
        assert_eq!(requests.len(), 1);
        assert!(!vm.complete_text_request(requests[0].id, Err("blocked".into())));
        assert_eq!(
            vm.global("errors"),
            Some(&JsValue::String("|Only GET is supportedblocked".into()))
        );
    }
}
