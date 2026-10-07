use crate::{
    BinaryOp, CompiledScript, JsError, JsValue, ObjectId, UnaryOp, bytecode::Instruction,
    compile_script,
};
use std::collections::HashMap;

const DEFAULT_INSTRUCTION_BUDGET: usize = 1_000_000;
const DEFAULT_OBJECT_BUDGET: usize = 100_000;

#[derive(Debug, Clone)]
struct Binding {
    value: JsValue,
    mutable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ObjectKind {
    Ordinary,
    Array,
}

#[derive(Debug, Clone)]
struct JsObject {
    properties: HashMap<String, JsValue>,
    prototype: Option<ObjectId>,
    kind: ObjectKind,
}

#[derive(Debug)]
pub struct JsRuntime {
    globals: HashMap<String, Binding>,
    heap: Vec<JsObject>,
    object_prototype: ObjectId,
    array_prototype: ObjectId,
    instruction_budget: usize,
    object_budget: usize,
}

impl Default for JsRuntime {
    fn default() -> Self {
        let object_prototype = ObjectId(0);
        let array_prototype = ObjectId(1);
        let heap = vec![
            JsObject {
                properties: HashMap::new(),
                prototype: None,
                kind: ObjectKind::Ordinary,
            },
            JsObject {
                properties: HashMap::new(),
                prototype: Some(object_prototype),
                kind: ObjectKind::Ordinary,
            },
        ];
        Self {
            globals: HashMap::new(),
            heap,
            object_prototype,
            array_prototype,
            instruction_budget: DEFAULT_INSTRUCTION_BUDGET,
            object_budget: DEFAULT_OBJECT_BUDGET,
        }
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

    pub fn eval_script(&mut self, source: &str) -> Result<JsValue, JsError> {
        let script = compile_script(source)?;
        self.execute(&script)
    }

    pub fn execute(&mut self, script: &CompiledScript) -> Result<JsValue, JsError> {
        let mut stack = Vec::new();
        let mut completion = JsValue::Undefined;
        let mut ip = 0usize;
        let mut steps = 0usize;

        while let Some(instruction) = script.code.get(ip) {
            steps = steps.saturating_add(1);
            if steps > self.instruction_budget {
                return Err(JsError::execution_limit(format!(
                    "script exceeded instruction budget of {}",
                    self.instruction_budget
                )));
            }

            match instruction {
                Instruction::Push(value) => stack.push(value.clone()),
                Instruction::Load(name) => {
                    let value = self
                        .globals
                        .get(name)
                        .ok_or_else(|| JsError::reference(format!("{name} is not defined")))?
                        .value
                        .clone();
                    stack.push(value);
                }
                Instruction::Declare { name, mutable } => {
                    if self.globals.contains_key(name) {
                        return Err(JsError::syntax(
                            0,
                            format!("identifier {name} has already been declared"),
                        ));
                    }
                    let value = stack
                        .pop()
                        .expect("compiler must push declaration initializer");
                    self.globals.insert(
                        name.clone(),
                        Binding {
                            value,
                            mutable: *mutable,
                        },
                    );
                }
                Instruction::Assign(name) => {
                    let value = stack
                        .last()
                        .expect("compiler must leave assignment value on stack")
                        .clone();
                    let binding = self
                        .globals
                        .get_mut(name)
                        .ok_or_else(|| JsError::reference(format!("{name} is not defined")))?;
                    if !binding.mutable {
                        return Err(JsError::type_error(format!(
                            "assignment to constant variable {name}"
                        )));
                    }
                    binding.value = value;
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
                Instruction::Unary(op) => {
                    let value = stack.pop().expect("compiler must push unary operand");
                    stack.push(apply_unary(*op, value));
                }
                Instruction::Binary(op) => {
                    let right = stack.pop().expect("compiler must push right operand");
                    let left = stack.pop().expect("compiler must push left operand");
                    stack.push(apply_binary(*op, left, right));
                }
                Instruction::Pop => {
                    stack.pop().expect("compiler must leave a value to pop");
                }
                Instruction::Jump(target) => {
                    debug_assert!(*target <= script.code.len());
                    ip = *target;
                    continue;
                }
                Instruction::JumpIfFalse(target) => {
                    let value = stack
                        .last()
                        .expect("compiler must leave branch condition on stack");
                    if !value.is_truthy() {
                        debug_assert!(*target <= script.code.len());
                        ip = *target;
                        continue;
                    }
                }
                Instruction::JumpIfTrue(target) => {
                    let value = stack
                        .last()
                        .expect("compiler must leave branch condition on stack");
                    if value.is_truthy() {
                        debug_assert!(*target <= script.code.len());
                        ip = *target;
                        continue;
                    }
                }
                Instruction::SetCompletion => {
                    completion = stack.pop().expect("compiler must push completion value");
                }
                Instruction::Halt => break,
            }

            ip = ip.saturating_add(1);
        }

        debug_assert!(stack.is_empty());
        Ok(completion)
    }

    pub fn global(&self, name: &str) -> Option<&JsValue> {
        Some(&self.globals.get(name)?.value)
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

    fn allocate_object(
        &mut self,
        kind: ObjectKind,
        prototype: Option<ObjectId>,
        properties: HashMap<String, JsValue>,
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
