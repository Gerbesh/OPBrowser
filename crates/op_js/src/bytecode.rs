use crate::{
    BinaryOp, Expression, JsError, JsValue, Program, Statement, UnaryOp, VariableKind, parse_script,
};

#[derive(Debug, Clone, PartialEq)]
pub struct CompiledScript {
    pub(crate) code: Vec<Instruction>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Instruction {
    Push(JsValue),
    Load(String),
    Declare { name: String, mutable: bool },
    Assign(String),
    Unary(UnaryOp),
    Binary(BinaryOp),
    SetCompletion,
    Halt,
}

pub fn compile_script(source: &str) -> Result<CompiledScript, JsError> {
    let program = parse_script(source)?;
    Ok(compile_program(&program))
}

pub(crate) fn compile_program(program: &Program) -> CompiledScript {
    let mut code = Vec::new();
    for statement in &program.statements {
        compile_statement(statement, &mut code);
    }
    code.push(Instruction::Halt);
    CompiledScript { code }
}

fn compile_statement(statement: &Statement, code: &mut Vec<Instruction>) {
    match statement {
        Statement::Variable {
            kind,
            name,
            initializer,
        } => {
            if let Some(initializer) = initializer {
                compile_expression(initializer, code);
            } else {
                code.push(Instruction::Push(JsValue::Undefined));
            }
            code.push(Instruction::Declare {
                name: name.clone(),
                mutable: !matches!(kind, VariableKind::Const),
            });
            code.push(Instruction::Push(JsValue::Undefined));
            code.push(Instruction::SetCompletion);
        }
        Statement::Expression(expression) => {
            compile_expression(expression, code);
            code.push(Instruction::SetCompletion);
        }
        Statement::Empty => {}
    }
}

fn compile_expression(expression: &Expression, code: &mut Vec<Instruction>) {
    match expression {
        Expression::Literal(value) => code.push(Instruction::Push(value.clone())),
        Expression::Identifier(name) => code.push(Instruction::Load(name.clone())),
        Expression::Unary { op, argument } => {
            compile_expression(argument, code);
            code.push(Instruction::Unary(*op));
        }
        Expression::Binary { left, op, right } => {
            compile_expression(left, code);
            compile_expression(right, code);
            code.push(Instruction::Binary(*op));
        }
        Expression::Assignment { name, value } => {
            compile_expression(value, code);
            code.push(Instruction::Assign(name.clone()));
        }
    }
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
}
