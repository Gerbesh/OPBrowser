use crate::{
    BinaryOp, CompiledScript, JsError, JsErrorKind, JsValue, ObjectId, UnaryOp, UpdateOp,
    VariableKind,
    bytecode::{FunctionTemplate, Instruction, TryTemplate},
    compile_script,
};
use std::{collections::HashMap, sync::Arc};

const DEFAULT_INSTRUCTION_BUDGET: usize = 1_000_000;
const DEFAULT_OBJECT_BUDGET: usize = 100_000;
const DEFAULT_ENVIRONMENT_BUDGET: usize = 100_000;
const DEFAULT_CALL_DEPTH_BUDGET: usize = 64;

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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BuiltinFunction {
    Error,
    TypeError,
    ReferenceError,
    DomGetElementById,
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

/// A host mutation applied by the browser only after VM execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DomTextMutation {
    pub node: usize,
    pub text_content: String,
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
    global_object: ObjectId,
    instruction_budget: usize,
    object_budget: usize,
    environment_budget: usize,
    call_depth_budget: usize,
    dom_ids: HashMap<String, usize>,
    dom_text: HashMap<usize, String>,
    dom_mutations: Vec<DomTextMutation>,
}

impl Default for JsRuntime {
    fn default() -> Self {
        let object_prototype = ObjectId(0);
        let array_prototype = ObjectId(1);
        let error_prototype = ObjectId(2);
        let type_error_prototype = ObjectId(3);
        let reference_error_prototype = ObjectId(4);
        let global_object = ObjectId(5);

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
            global_object,
            instruction_budget: DEFAULT_INSTRUCTION_BUDGET,
            object_budget: DEFAULT_OBJECT_BUDGET,
            environment_budget: DEFAULT_ENVIRONMENT_BUDGET,
            call_depth_budget: DEFAULT_CALL_DEPTH_BUDGET,
            dom_ids: HashMap::new(),
            dom_text: HashMap::new(),
            dom_mutations: Vec::new(),
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
        runtime
            .install_error_constructor(
                "ReferenceError",
                BuiltinFunction::ReferenceError,
                reference_error_prototype,
            )
            .expect("built-in ReferenceError constructor must fit initial runtime budgets");

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
        for element in elements.into_iter().take(4096) {
            self.dom_ids.entry(element.id).or_insert(element.node);
            self.dom_text.insert(element.node, element.text_content);
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
        let document = self.allocate_object(
            ObjectKind::Ordinary,
            Some(self.object_prototype),
            HashMap::from([("getElementById".to_owned(), JsValue::Object(function))]),
        )?;
        self.install_global_binding(
            "document",
            JsValue::Object(document),
            false,
            VariableKind::Const,
        );
        Ok(())
    }

    pub fn take_dom_mutations(&mut self) -> Vec<DomTextMutation> {
        std::mem::take(&mut self.dom_mutations)
    }

    pub fn eval_script(&mut self, source: &str) -> Result<JsValue, JsError> {
        let script = compile_script(source)?;
        self.execute(&script)
    }

    pub fn execute(&mut self, script: &CompiledScript) -> Result<JsValue, JsError> {
        let mut steps = 0usize;
        match self.run_code(&script.code, self.global_env, &mut steps, 0)? {
            RunOutcome::Complete(value) | RunOutcome::Returned(value) => Ok(value),
            RunOutcome::Thrown(value) => {
                Err(JsError::exception(self.describe_thrown_value(&value)))
            }
            RunOutcome::Break => Err(JsError::type_error("break escaped script control flow")),
            RunOutcome::Continue => {
                Err(JsError::type_error("continue escaped script control flow"))
            }
        }
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
                    stack.push(apply_unary(*op, value));
                }
                Instruction::Binary(op) => {
                    let right = stack.pop().expect("compiler must push right operand");
                    let left = stack.pop().expect("compiler must push left operand");
                    stack.push(apply_binary(*op, left, right));
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
            FunctionImplementation::Builtin(builtin) => self.call_builtin(builtin, arguments),
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
        arguments: Vec<JsValue>,
    ) -> Result<CallOutcome, JsError> {
        if matches!(builtin, BuiltinFunction::DomGetElementById) {
            let Some(id) = arguments.first().map(JsValue::to_js_string) else {
                return Ok(CallOutcome::Value(JsValue::Null));
            };
            let Some(node) = self.dom_ids.get(&id).copied() else {
                return Ok(CallOutcome::Value(JsValue::Null));
            };
            let text = self.dom_text.get(&node).cloned().unwrap_or_default();
            let element = self.allocate_object(
                ObjectKind::DomElement(node),
                Some(self.object_prototype),
                HashMap::from([
                    ("textContent".to_owned(), JsValue::String(text)),
                    ("id".to_owned(), JsValue::String(id)),
                ]),
            )?;
            return Ok(CallOutcome::Value(JsValue::Object(element)));
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
            BuiltinFunction::DomGetElementById => unreachable!("handled above"),
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

        if matches!(function.implementation, FunctionImplementation::Builtin(_)) {
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
        let environment = self
            .find_binding_environment(start, name)
            .ok_or_else(|| JsError::reference(format!("{name} is not defined")))?;
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

    pub fn get_property(&self, target: &JsValue, key: &str) -> Result<JsValue, JsError> {
        match target {
            JsValue::Object(id) => {
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
            self.dom_mutations.push(DomTextMutation {
                node,
                text_content: text,
            });
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
}
