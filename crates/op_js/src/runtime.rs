use crate::{
    BinaryOp, CompiledScript, JsError, JsValue, UnaryOp, bytecode::Instruction, compile_script,
};
use std::collections::HashMap;

const DEFAULT_INSTRUCTION_BUDGET: usize = 1_000_000;

#[derive(Debug, Clone)]
struct Binding {
    value: JsValue,
    mutable: bool,
}

#[derive(Debug)]
pub struct JsRuntime {
    globals: HashMap<String, Binding>,
    instruction_budget: usize,
}

impl Default for JsRuntime {
    fn default() -> Self {
        Self {
            globals: HashMap::new(),
            instruction_budget: DEFAULT_INSTRUCTION_BUDGET,
        }
    }
}

impl JsRuntime {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_instruction_budget(instruction_budget: usize) -> Self {
        Self {
            globals: HashMap::new(),
            instruction_budget,
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
}
