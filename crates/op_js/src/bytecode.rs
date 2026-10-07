use crate::{
    AssignmentTarget, BinaryOp, CatchClause, Expression, ForInitializer, JsError, JsValue,
    LogicalOp, Program, Statement, SwitchCase, UnaryOp, UpdateOp, VariableDeclarator, VariableKind,
    parse_script,
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
pub(crate) struct TryTemplate {
    pub(crate) try_code: Vec<Instruction>,
    pub(crate) catch_param: Option<String>,
    pub(crate) catch_code: Option<Vec<Instruction>>,
    pub(crate) finally_code: Option<Vec<Instruction>>,
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
    UpdateBinding {
        name: String,
        op: UpdateOp,
        prefix: bool,
    },
    UpdateProperty {
        op: UpdateOp,
        prefix: bool,
    },
    CreateObject(Vec<bool>),
    CreateArray(Vec<bool>),
    CreateFunction(Arc<FunctionTemplate>),
    GetProperty,
    SetProperty,
    Call {
        argument_count: usize,
        has_receiver: bool,
    },
    Construct(usize),
    Unary(UnaryOp),
    Binary(BinaryOp),
    Dup,
    Pop,
    EnterScope,
    ExitScope,
    UnwindScopes(usize),
    Jump(usize),
    JumpIfFalse(usize),
    JumpIfTrue(usize),
    Try {
        template: Arc<TryTemplate>,
        break_target: Option<usize>,
        break_unwind: usize,
        continue_target: Option<usize>,
        continue_unwind: usize,
    },
    Throw,
    BreakSignal,
    ContinueSignal,
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
    compiler.statement_list(&program.statements);
    compiler.code.push(Instruction::Halt);
    CompiledScript {
        code: compiler.code,
    }
}

#[derive(Default)]
struct Compiler {
    code: Vec<Instruction>,
    controls: Vec<ControlContext>,
    scope_depth: usize,
}

struct ControlContext {
    scope_depth: usize,
    is_loop: bool,
    break_jumps: Vec<usize>,
    continue_jumps: Vec<usize>,
    break_try_patches: Vec<usize>,
    continue_try_patches: Vec<usize>,
}

impl Compiler {
    fn statement(&mut self, statement: &Statement) {
        match statement {
            Statement::Variable { kind, declarations } => {
                self.declarations(*kind, declarations);
                self.code.push(Instruction::Push(JsValue::Undefined));
                self.code.push(Instruction::SetCompletion);
            }
            Statement::Block(statements) => {
                self.block(statements);
            }
            Statement::If {
                test,
                consequent,
                alternate,
            } => self.if_statement(test, consequent, alternate.as_deref()),
            Statement::While { test, body } => self.while_statement(test, body),
            Statement::DoWhile { body, test } => self.do_while_statement(body, test),
            Statement::For {
                initializer,
                test,
                update,
                body,
            } => self.for_statement(initializer.as_ref(), test.as_ref(), update.as_ref(), body),
            Statement::Switch {
                discriminant,
                cases,
            } => self.switch_statement(discriminant, cases),
            Statement::Try {
                block,
                handler,
                finalizer,
            } => self.try_statement(block, handler.as_ref(), finalizer.as_deref()),
            Statement::Throw(value) => {
                self.expression(value);
                self.code.push(Instruction::Throw);
            }
            Statement::FunctionDeclaration { name, params, body } => {
                self.function_declaration(name, params, body);
            }
            Statement::Return(value) => {
                if let Some(value) = value {
                    self.expression(value);
                } else {
                    self.code.push(Instruction::Push(JsValue::Undefined));
                }
                self.code.push(Instruction::Return);
            }
            Statement::Break => self.break_statement(),
            Statement::Continue => self.continue_statement(),
            Statement::Expression(expression) => {
                self.expression(expression);
                self.code.push(Instruction::SetCompletion);
            }
            Statement::Empty => {}
        }
    }

    fn statement_list(&mut self, statements: &[Statement]) {
        for statement in statements {
            if let Statement::FunctionDeclaration { name, params, body } = statement {
                self.function_declaration(name, params, body);
            }
        }
        for statement in statements {
            if !matches!(statement, Statement::FunctionDeclaration { .. }) {
                self.statement(statement);
            }
        }
    }

    fn function_declaration(&mut self, name: &str, params: &[String], body: &[Statement]) {
        let template = compile_function_template(Some(name.to_owned()), params, body);
        self.code.push(Instruction::CreateFunction(template));
        self.code.push(Instruction::Declare {
            name: name.to_owned(),
            kind: VariableKind::Var,
            has_initializer: true,
        });
        self.code.push(Instruction::Push(JsValue::Undefined));
        self.code.push(Instruction::SetCompletion);
    }

    fn block(&mut self, statements: &[Statement]) {
        self.code.push(Instruction::EnterScope);
        self.scope_depth += 1;
        self.statement_list(statements);
        self.scope_depth -= 1;
        self.code.push(Instruction::ExitScope);
    }

    fn declarations(&mut self, kind: VariableKind, declarations: &[VariableDeclarator]) {
        for declaration in declarations {
            let has_initializer = declaration.initializer.is_some();
            if let Some(initializer) = &declaration.initializer {
                self.expression(initializer);
            } else {
                self.code.push(Instruction::Push(JsValue::Undefined));
            }
            self.code.push(Instruction::Declare {
                name: declaration.name.clone(),
                kind,
                has_initializer,
            });
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

    fn push_control(&mut self, is_loop: bool) {
        self.controls.push(ControlContext {
            scope_depth: self.scope_depth,
            is_loop,
            break_jumps: Vec::new(),
            continue_jumps: Vec::new(),
            break_try_patches: Vec::new(),
            continue_try_patches: Vec::new(),
        });
    }

    fn pop_control(&mut self) -> ControlContext {
        self.controls.pop().expect("control context must exist")
    }

    fn while_statement(&mut self, test: &Expression, body: &Statement) {
        let loop_start = self.code.len();
        self.expression(test);
        let false_jump = self.emit_jump_if_false();
        self.code.push(Instruction::Pop);

        self.push_control(true);
        self.statement(body);
        let context = self.pop_control();
        for jump in context.continue_jumps {
            self.patch_jump(jump, loop_start);
        }
        for instruction in context.continue_try_patches {
            self.patch_try_continue(instruction, loop_start);
        }
        self.code.push(Instruction::Jump(loop_start));

        let false_cleanup = self.code.len();
        self.patch_jump(false_jump, false_cleanup);
        self.code.push(Instruction::Pop);
        let end = self.code.len();
        for jump in context.break_jumps {
            self.patch_jump(jump, end);
        }
        for instruction in context.break_try_patches {
            self.patch_try_break(instruction, end);
        }
    }

    fn do_while_statement(&mut self, body: &Statement, test: &Expression) {
        let loop_start = self.code.len();
        self.push_control(true);
        self.statement(body);
        let context = self.pop_control();

        let continue_target = self.code.len();
        for jump in context.continue_jumps {
            self.patch_jump(jump, continue_target);
        }
        for instruction in context.continue_try_patches {
            self.patch_try_continue(instruction, continue_target);
        }

        self.expression(test);
        let exit_jump = self.emit_jump_if_false();
        self.code.push(Instruction::Pop);
        self.code.push(Instruction::Jump(loop_start));
        let exit_cleanup = self.code.len();
        self.patch_jump(exit_jump, exit_cleanup);
        self.code.push(Instruction::Pop);

        let end = self.code.len();
        for jump in context.break_jumps {
            self.patch_jump(jump, end);
        }
        for instruction in context.break_try_patches {
            self.patch_try_break(instruction, end);
        }
    }

    fn for_statement(
        &mut self,
        initializer: Option<&ForInitializer>,
        test: Option<&Expression>,
        update: Option<&Expression>,
        body: &Statement,
    ) {
        self.code.push(Instruction::EnterScope);
        self.scope_depth += 1;

        if let Some(initializer) = initializer {
            match initializer {
                ForInitializer::Variable { kind, declarations } => {
                    self.declarations(*kind, declarations);
                }
                ForInitializer::Expression(expression) => {
                    self.expression(expression);
                    self.code.push(Instruction::Pop);
                }
            }
        }

        let loop_start = self.code.len();
        let false_jump = test.map(|test| {
            self.expression(test);
            let jump = self.emit_jump_if_false();
            self.code.push(Instruction::Pop);
            jump
        });

        self.push_control(true);
        self.statement(body);
        let context = self.pop_control();

        let continue_target = self.code.len();
        for jump in context.continue_jumps {
            self.patch_jump(jump, continue_target);
        }
        for instruction in context.continue_try_patches {
            self.patch_try_continue(instruction, continue_target);
        }
        if let Some(update) = update {
            self.expression(update);
            self.code.push(Instruction::Pop);
        }
        self.code.push(Instruction::Jump(loop_start));

        if let Some(false_jump) = false_jump {
            let false_cleanup = self.code.len();
            self.patch_jump(false_jump, false_cleanup);
            self.code.push(Instruction::Pop);
        }

        let exit_scope = self.code.len();
        for jump in context.break_jumps {
            self.patch_jump(jump, exit_scope);
        }
        for instruction in context.break_try_patches {
            self.patch_try_break(instruction, exit_scope);
        }
        self.scope_depth -= 1;
        self.code.push(Instruction::ExitScope);
    }

    fn switch_statement(&mut self, discriminant: &Expression, cases: &[SwitchCase]) {
        self.expression(discriminant);
        self.code.push(Instruction::EnterScope);
        self.scope_depth += 1;

        let mut match_jumps = Vec::new();
        for (case_index, case) in cases.iter().enumerate() {
            if let Some(test) = &case.test {
                self.code.push(Instruction::Dup);
                self.expression(test);
                self.code.push(Instruction::Binary(BinaryOp::StrictEqual));
                let jump = self.emit_jump_if_true();
                match_jumps.push((case_index, jump));
                self.code.push(Instruction::Pop);
            }
        }

        self.code.push(Instruction::Pop);
        let no_match_jump = self.emit_jump();

        let mut body_jumps = Vec::new();
        for (case_index, match_jump) in match_jumps {
            let stub = self.code.len();
            self.patch_jump(match_jump, stub);
            self.code.push(Instruction::Pop);
            self.code.push(Instruction::Pop);
            let body_jump = self.emit_jump();
            body_jumps.push((case_index, body_jump));
        }

        self.push_control(false);
        let mut body_targets = Vec::with_capacity(cases.len());
        for case in cases {
            body_targets.push(self.code.len());
            for statement in &case.consequent {
                self.statement(statement);
            }
        }
        let context = self.pop_control();

        for (case_index, jump) in body_jumps {
            self.patch_jump(jump, body_targets[case_index]);
        }

        let default_target = cases
            .iter()
            .position(|case| case.test.is_none())
            .map(|index| body_targets[index]);
        let exit_scope = self.code.len();
        self.patch_jump(no_match_jump, default_target.unwrap_or(exit_scope));
        for jump in context.break_jumps {
            self.patch_jump(jump, exit_scope);
        }
        for instruction in context.break_try_patches {
            self.patch_try_break(instruction, exit_scope);
        }

        self.scope_depth -= 1;
        self.code.push(Instruction::ExitScope);
    }

    fn try_statement(
        &mut self,
        block: &[Statement],
        handler: Option<&CatchClause>,
        finalizer: Option<&[Statement]>,
    ) {
        let try_code = compile_nested_block(block);
        let (catch_param, catch_code) = if let Some(handler) = handler {
            (
                handler.param.clone(),
                Some(compile_nested_block(&handler.body)),
            )
        } else {
            (None, None)
        };
        let finally_code = finalizer.map(compile_nested_block);

        let break_context = self.controls.len().checked_sub(1);
        let continue_context = self.controls.iter().rposition(|context| context.is_loop);
        let break_unwind = break_context
            .map(|index| {
                self.scope_depth
                    .saturating_sub(self.controls[index].scope_depth)
            })
            .unwrap_or(0);
        let continue_unwind = continue_context
            .map(|index| {
                self.scope_depth
                    .saturating_sub(self.controls[index].scope_depth)
            })
            .unwrap_or(0);

        let instruction = self.code.len();
        self.code.push(Instruction::Try {
            template: Arc::new(TryTemplate {
                try_code,
                catch_param,
                catch_code,
                finally_code,
            }),
            break_target: break_context.map(|_| usize::MAX),
            break_unwind,
            continue_target: continue_context.map(|_| usize::MAX),
            continue_unwind,
        });

        if let Some(index) = break_context {
            self.controls[index].break_try_patches.push(instruction);
        }
        if let Some(index) = continue_context {
            self.controls[index].continue_try_patches.push(instruction);
        }
    }

    fn break_statement(&mut self) {
        let Some(context_index) = self.controls.len().checked_sub(1) else {
            self.code.push(Instruction::BreakSignal);
            return;
        };
        let scope_depth = self.controls[context_index].scope_depth;
        let unwind = self.scope_depth.saturating_sub(scope_depth);
        if unwind != 0 {
            self.code.push(Instruction::UnwindScopes(unwind));
        }
        let jump = self.emit_jump();
        self.controls[context_index].break_jumps.push(jump);
    }

    fn continue_statement(&mut self) {
        let Some(context_index) = self.controls.iter().rposition(|context| context.is_loop) else {
            self.code.push(Instruction::ContinueSignal);
            return;
        };
        let scope_depth = self.controls[context_index].scope_depth;
        let unwind = self.scope_depth.saturating_sub(scope_depth);
        if unwind != 0 {
            self.code.push(Instruction::UnwindScopes(unwind));
        }
        let jump = self.emit_jump();
        self.controls[context_index].continue_jumps.push(jump);
    }

    fn expression(&mut self, expression: &Expression) {
        match expression {
            Expression::Literal(value) => self.code.push(Instruction::Push(value.clone())),
            Expression::Identifier(name) => self.code.push(Instruction::Load(name.clone())),
            Expression::This => self.code.push(Instruction::Load("this".into())),
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
                let has_receiver = if let Expression::Member { object, property } = callee.as_ref()
                {
                    self.expression(object);
                    self.code.push(Instruction::Dup);
                    self.expression(property);
                    self.code.push(Instruction::GetProperty);
                    true
                } else {
                    self.expression(callee);
                    false
                };
                for argument in arguments {
                    self.expression(argument);
                }
                self.code.push(Instruction::Call {
                    argument_count: arguments.len(),
                    has_receiver,
                });
            }
            Expression::New { callee, arguments } => {
                self.expression(callee);
                for argument in arguments {
                    self.expression(argument);
                }
                self.code.push(Instruction::Construct(arguments.len()));
            }
            Expression::Function { name, params, body } => {
                let template = compile_function_template(name.clone(), params, body);
                self.code.push(Instruction::CreateFunction(template));
            }
            Expression::Update { target, op, prefix } => match target {
                AssignmentTarget::Identifier(name) => {
                    self.code.push(Instruction::UpdateBinding {
                        name: name.clone(),
                        op: *op,
                        prefix: *prefix,
                    });
                }
                AssignmentTarget::Member { object, property } => {
                    self.expression(object);
                    self.expression(property);
                    self.code.push(Instruction::UpdateProperty {
                        op: *op,
                        prefix: *prefix,
                    });
                }
            },
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

    fn patch_try_break(&mut self, index: usize, target: usize) {
        match self.code.get_mut(index) {
            Some(Instruction::Try {
                break_target: Some(current),
                ..
            }) => *current = target,
            _ => unreachable!("try break patch must reference a try instruction"),
        }
    }

    fn patch_try_continue(&mut self, index: usize, target: usize) {
        match self.code.get_mut(index) {
            Some(Instruction::Try {
                continue_target: Some(current),
                ..
            }) => *current = target,
            _ => unreachable!("try continue patch must reference a try instruction"),
        }
    }
}

fn compile_nested_block(statements: &[Statement]) -> Vec<Instruction> {
    let mut compiler = Compiler::default();
    compiler.block(statements);
    compiler.code
}

fn compile_function_template(
    name: Option<String>,
    params: &[String],
    body: &[Statement],
) -> Arc<FunctionTemplate> {
    let mut compiler = Compiler::default();
    compiler.statement_list(body);
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
    fn compiles_for_do_switch_and_exception_regions() {
        let script = compile_script(
            "let x = 0; for (let i = 0; i < 3; i++) { x = x + i; } do { x = x - 1; } while (x > 2); switch (x) { case 2: x = 7; break; default: x = 9; } try { throw x; } catch (e) { x = e + 1; } finally { x = x + 1; } x",
        )
        .unwrap();
        assert!(
            script
                .code
                .iter()
                .any(|instruction| matches!(instruction, Instruction::Dup))
        );
        assert!(
            script
                .code
                .iter()
                .any(|instruction| matches!(instruction, Instruction::Try { .. }))
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
        assert!(script.code.iter().any(|instruction| matches!(
            instruction,
            Instruction::Call {
                argument_count: 1,
                ..
            }
        )));
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
