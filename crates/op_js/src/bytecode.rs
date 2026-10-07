use crate::{
    AssignmentTarget, BinaryOp, Expression, JsError, JsValue, LogicalOp, Program, Statement,
    UnaryOp, VariableKind, parse_script,
};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq)]
pub struct CompiledScript {
    pub(crate) code: Vec<Instruction>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct FunctionTemplate {
    pub(crate) name: Option<String>,
    pub(crate) params: Vec<String>,
    pub(crate) code: Vec<Instruction>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Instruction {
    Push(JsValue),
    Load(String),
    Declare {
        name: String,
        kind: VariableKind,
        has_initializer: bool,
    },
    Assign(String),
    CreateObject(Vec<bool>),
    CreateArray(Vec<bool>),
    CreateFunction(Arc<FunctionTemplate>),
    GetProperty,
    SetProperty,
    Call(usize),
    Unary(UnaryOp),
    Binary(BinaryOp),
    Pop,
    EnterScope,
    ExitScope,
    UnwindScopes(usize),
    Jump(usize),
    JumpIfFalse(usize),
    JumpIfTrue(usize),
    Return,
    SetCompletion,
    Halt,
}

pub fn compile_script(source: &str) -> Result<CompiledScript, JsError> {
    let program = parse_script(source)?;
    Ok(compile_program(&program))
}

pub(crate) fn compile_program(program: &Program) -> CompiledScript {
    let mut compiler = Compiler::default();
    for statement in &program.statements {
        compiler.statement(statement);
    }
    compiler.code.push(Instruction::Halt);
    CompiledScript {
        code: compiler.code,
    }
}

#[derive(Default)]
struct Compiler {
    code: Vec<Instruction>,
    loops: Vec<LoopContext>,
    scope_depth: usize,
}

struct LoopContext {
    continue_target: usize,
    scope_depth: usize,
    break_jumps: Vec<usize>,
}

impl Compiler {
    fn statement(&mut self, statement: &Statement) {
        match statement {
            Statement::Variable { kind, declarations } => {
                for declaration in declarations {
                    let has_initializer = declaration.initializer.is_some();
                    if let Some(initializer) = &declaration.initializer {
                        self.expression(initializer);
                    } else {
                        self.code.push(Instruction::Push(JsValue::Undefined));
                    }
                    self.code.push(Instruction::Declare {
                        name: declaration.name.clone(),
                        kind: *kind,
                        has_initializer,
                    });
                }
                self.code.push(Instruction::Push(JsValue::Undefined));
                self.code.push(Instruction::SetCompletion);
            }
            Statement::Block(statements) => {
                self.code.push(Instruction::EnterScope);
                self.scope_depth += 1;
                for statement in statements {
                    self.statement(statement);
                }
                self.scope_depth -= 1;
                self.code.push(Instruction::ExitScope);
            }
            Statement::If {
                test,
                consequent,
                alternate,
            } => self.if_statement(test, consequent, alternate.as_deref()),
            Statement::While { test, body } => self.while_statement(test, body),
            Statement::FunctionDeclaration { name, params, body } => {
                let template = compile_function_template(Some(name.clone()), params, body);
                self.code.push(Instruction::CreateFunction(template));
                self.code.push(Instruction::Declare {
                    name: name.clone(),
                    kind: VariableKind::Var,
                    has_initializer: true,
                });
                self.code.push(Instruction::Push(JsValue::Undefined));
                self.code.push(Instruction::SetCompletion);
            }
            Statement::Return(value) => {
                if let Some(value) = value {
                    self.expression(value);
                } else {
                    self.code.push(Instruction::Push(JsValue::Undefined));
                }
                self.code.push(Instruction::Return);
            }
            Statement::Break => {
                let (loop_scope_depth, _) = self
                    .loops
                    .last()
                    .map(|context| (context.scope_depth, context.continue_target))
                    .expect("parser only emits break inside loops");
                let unwind = self.scope_depth.saturating_sub(loop_scope_depth);
                if unwind != 0 {
                    self.code.push(Instruction::UnwindScopes(unwind));
                }
                let jump = self.emit_jump();
                self.loops
                    .last_mut()
                    .expect("loop context must exist")
                    .break_jumps
                    .push(jump);
            }
            Statement::Continue => {
                let context = self
                    .loops
                    .last()
                    .expect("parser only emits continue inside loops");
                let unwind = self.scope_depth.saturating_sub(context.scope_depth);
                let target = context.continue_target;
                if unwind != 0 {
                    self.code.push(Instruction::UnwindScopes(unwind));
                }
                self.code.push(Instruction::Jump(target));
            }
            Statement::Expression(expression) => {
                self.expression(expression);
                self.code.push(Instruction::SetCompletion);
            }
            Statement::Empty => {}
        }
    }

    fn if_statement(
        &mut self,
        test: &Expression,
        consequent: &Statement,
        alternate: Option<&Statement>,
    ) {
        self.expression(test);
        let false_jump = self.emit_jump_if_false();
        self.code.push(Instruction::Pop);
        self.statement(consequent);
        let end_jump = self.emit_jump();

        let false_target = self.code.len();
        self.patch_jump(false_jump, false_target);
        self.code.push(Instruction::Pop);
        if let Some(alternate) = alternate {
            self.statement(alternate);
        }

        let end = self.code.len();
        self.patch_jump(end_jump, end);
    }

    fn while_statement(&mut self, test: &Expression, body: &Statement) {
        let loop_start = self.code.len();
        self.expression(test);
        let false_jump = self.emit_jump_if_false();
        self.code.push(Instruction::Pop);

        self.loops.push(LoopContext {
            continue_target: loop_start,
            scope_depth: self.scope_depth,
            break_jumps: Vec::new(),
        });
        self.statement(body);
        let loop_context = self.loops.pop().expect("loop context must exist");
        self.code.push(Instruction::Jump(loop_start));

        let false_cleanup = self.code.len();
        self.patch_jump(false_jump, false_cleanup);
        self.code.push(Instruction::Pop);
        let end = self.code.len();
        for jump in loop_context.break_jumps {
            self.patch_jump(jump, end);
        }
    }

    fn expression(&mut self, expression: &Expression) {
        match expression {
            Expression::Literal(value) => self.code.push(Instruction::Push(value.clone())),
            Expression::Identifier(name) => self.code.push(Instruction::Load(name.clone())),
            Expression::ObjectLiteral(properties) => {
                let mut prototype_setters = Vec::with_capacity(properties.len());
                for property in properties {
                    self.code
                        .push(Instruction::Push(JsValue::String(property.key.clone())));
                    self.expression(&property.value);
                    prototype_setters.push(property.prototype_setter);
                }
                self.code.push(Instruction::CreateObject(prototype_setters));
            }
            Expression::ArrayLiteral(elements) => {
                let mut present = Vec::with_capacity(elements.len());
                for element in elements {
                    if let Some(element) = element {
                        self.expression(element);
                        present.push(true);
                    } else {
                        self.code.push(Instruction::Push(JsValue::Undefined));
                        present.push(false);
                    }
                }
                self.code.push(Instruction::CreateArray(present));
            }
            Expression::Member { object, property } => {
                self.expression(object);
                self.expression(property);
                self.code.push(Instruction::GetProperty);
            }
            Expression::Call { callee, arguments } => {
                self.expression(callee);
                for argument in arguments {
                    self.expression(argument);
                }
                self.code.push(Instruction::Call(arguments.len()));
            }
            Expression::Function { name, params, body } => {
                let template = compile_function_template(name.clone(), params, body);
                self.code.push(Instruction::CreateFunction(template));
            }
            Expression::Unary { op, argument } => {
                self.expression(argument);
                self.code.push(Instruction::Unary(*op));
            }
            Expression::Binary { left, op, right } => {
                self.expression(left);
                self.expression(right);
                self.code.push(Instruction::Binary(*op));
            }
            Expression::Logical { left, op, right } => {
                self.expression(left);
                let short_circuit = match op {
                    LogicalOp::And => self.emit_jump_if_false(),
                    LogicalOp::Or => self.emit_jump_if_true(),
                };
                self.code.push(Instruction::Pop);
                self.expression(right);
                let end = self.code.len();
                self.patch_jump(short_circuit, end);
            }
            Expression::Assignment { target, value } => match target {
                AssignmentTarget::Identifier(name) => {
                    self.expression(value);
                    self.code.push(Instruction::Assign(name.clone()));
                }
                AssignmentTarget::Member { object, property } => {
                    self.expression(object);
                    self.expression(property);
                    self.expression(value);
                    self.code.push(Instruction::SetProperty);
                }
            },
        }
    }

    fn emit_jump(&mut self) -> usize {
        let index = self.code.len();
        self.code.push(Instruction::Jump(usize::MAX));
        index
    }

    fn emit_jump_if_false(&mut self) -> usize {
        let index = self.code.len();
        self.code.push(Instruction::JumpIfFalse(usize::MAX));
        index
    }

    fn emit_jump_if_true(&mut self) -> usize {
        let index = self.code.len();
        self.code.push(Instruction::JumpIfTrue(usize::MAX));
        index
    }

    fn patch_jump(&mut self, index: usize, target: usize) {
        match self.code.get_mut(index) {
            Some(
                Instruction::Jump(current)
                | Instruction::JumpIfFalse(current)
                | Instruction::JumpIfTrue(current),
            ) => *current = target,
            _ => unreachable!("compiler jump patch must reference a jump instruction"),
        }
    }
}

fn compile_function_template(
    name: Option<String>,
    params: &[String],
    body: &[Statement],
) -> Arc<FunctionTemplate> {
    let mut compiler = Compiler::default();
    for statement in body {
        compiler.statement(statement);
    }
    compiler.code.push(Instruction::Push(JsValue::Undefined));
    compiler.code.push(Instruction::Return);
    Arc::new(FunctionTemplate {
        name,
        params: params.to_vec(),
        code: compiler.code,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compiles_script_to_owned_bytecode() {
        let script = compile_script("let x = 2; x * 4").unwrap();
        assert!(matches!(script.code.last(), Some(Instruction::Halt)));
        assert!(
            script
                .code
                .iter()
                .any(|instruction| matches!(instruction, Instruction::Binary(BinaryOp::Multiply)))
        );
    }

    #[test]
    fn compiles_control_flow_to_patched_jumps() {
        let script = compile_script(
            "let x = 0; while (x < 3) { if (x === 1) { x = x + 1; continue; } x = x + 1; } x",
        )
        .unwrap();
        assert!(script.code.iter().any(
            |instruction| matches!(instruction, Instruction::JumpIfFalse(target) if *target != usize::MAX)
        ));
        assert!(script.code.iter().any(
            |instruction| matches!(instruction, Instruction::Jump(target) if *target != usize::MAX)
        ));
        assert!(
            script
                .code
                .iter()
                .any(|instruction| matches!(instruction, Instruction::EnterScope))
        );
    }

    #[test]
    fn compiles_object_array_and_member_operations() {
        let script =
            compile_script("let a = {x: 1}; let b = [a.x,, 3]; b[0] = b[2]; b.length").unwrap();
        assert!(
            script
                .code
                .iter()
                .any(|instruction| matches!(instruction, Instruction::CreateObject(flags) if flags == &[false]))
        );
        assert!(script.code.iter().any(
            |instruction| matches!(instruction, Instruction::CreateArray(present) if present == &[true, false, true])
        ));
        assert!(
            script
                .code
                .iter()
                .any(|instruction| matches!(instruction, Instruction::GetProperty))
        );
        assert!(
            script
                .code
                .iter()
                .any(|instruction| matches!(instruction, Instruction::SetProperty))
        );
    }

    #[test]
    fn compiles_functions_calls_returns_and_scope_unwind() {
        let script = compile_script(
            "function outer(x) { while (x) { { if (x === 2) break; x = x - 1; continue; } } return function inner(y) { return x + y; }; } outer(3)(4)",
        )
        .unwrap();
        assert!(
            script
                .code
                .iter()
                .any(|instruction| matches!(instruction, Instruction::CreateFunction(_)))
        );
        assert!(
            script
                .code
                .iter()
                .any(|instruction| matches!(instruction, Instruction::Call(1)))
        );
        let function = script
            .code
            .iter()
            .find_map(|instruction| match instruction {
                Instruction::CreateFunction(template) => Some(template),
                _ => None,
            });
        let function = function.expect("function declaration should compile to a template");
        assert!(
            function
                .code
                .iter()
                .any(|instruction| matches!(instruction, Instruction::UnwindScopes(_)))
        );
        assert!(
            function
                .code
                .iter()
                .any(|instruction| matches!(instruction, Instruction::Return))
        );
    }
}
