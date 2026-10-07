use crate::{JsError, JsValue, Token, TokenKind, tokenize};

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub statements: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Variable {
        kind: VariableKind,
        declarations: Vec<VariableDeclarator>,
    },
    Block(Vec<Statement>),
    If {
        test: Expression,
        consequent: Box<Statement>,
        alternate: Option<Box<Statement>>,
    },
    While {
        test: Expression,
        body: Box<Statement>,
    },
    Break,
    Continue,
    Expression(Expression),
    Empty,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VariableDeclarator {
    pub name: String,
    pub initializer: Option<Expression>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VariableKind {
    Let,
    Const,
    Var,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    Literal(JsValue),
    Identifier(String),
    Unary {
        op: UnaryOp,
        argument: Box<Expression>,
    },
    Binary {
        left: Box<Expression>,
        op: BinaryOp,
        right: Box<Expression>,
    },
    Logical {
        left: Box<Expression>,
        op: LogicalOp,
        right: Box<Expression>,
    },
    Assignment {
        name: String,
        value: Box<Expression>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Plus,
    Minus,
    Not,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogicalOp {
    And,
    Or,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
    Equal,
    NotEqual,
    StrictEqual,
    StrictNotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
}

pub fn parse_script(source: &str) -> Result<Program, JsError> {
    Parser::new(tokenize(source)?).parse_program()
}

struct Parser {
    tokens: Vec<Token>,
    index: usize,
    loop_depth: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            index: 0,
            loop_depth: 0,
        }
    }

    fn parse_program(mut self) -> Result<Program, JsError> {
        let mut statements = Vec::new();
        while !self.check(&TokenKind::Eof) {
            statements.push(self.statement()?);
        }
        Ok(Program { statements })
    }

    fn statement(&mut self) -> Result<Statement, JsError> {
        if self.take(&TokenKind::Semicolon) {
            return Ok(Statement::Empty);
        }
        if self.take(&TokenKind::LeftBrace) {
            return self.block_statement();
        }
        if self.take(&TokenKind::If) {
            return self.if_statement();
        }
        if self.take(&TokenKind::While) {
            return self.while_statement();
        }
        if self.take(&TokenKind::Break) {
            if self.loop_depth == 0 {
                return Err(JsError::syntax(
                    self.previous().start,
                    "break is only valid inside a loop",
                ));
            }
            self.optional_semicolon();
            return Ok(Statement::Break);
        }
        if self.take(&TokenKind::Continue) {
            if self.loop_depth == 0 {
                return Err(JsError::syntax(
                    self.previous().start,
                    "continue is only valid inside a loop",
                ));
            }
            self.optional_semicolon();
            return Ok(Statement::Continue);
        }

        let variable_kind = if self.take(&TokenKind::Let) {
            Some(VariableKind::Let)
        } else if self.take(&TokenKind::Const) {
            Some(VariableKind::Const)
        } else if self.take(&TokenKind::Var) {
            Some(VariableKind::Var)
        } else {
            None
        };

        if let Some(kind) = variable_kind {
            return self.variable_statement(kind);
        }

        let expression = self.assignment()?;
        self.optional_semicolon();
        Ok(Statement::Expression(expression))
    }

    fn block_statement(&mut self) -> Result<Statement, JsError> {
        let mut statements = Vec::new();
        while !self.check(&TokenKind::RightBrace) {
            if self.check(&TokenKind::Eof) {
                return Err(JsError::syntax(
                    self.current().start,
                    "expected '}' after block",
                ));
            }
            statements.push(self.statement()?);
        }
        self.advance();
        Ok(Statement::Block(statements))
    }

    fn if_statement(&mut self) -> Result<Statement, JsError> {
        let test = self.parenthesized_expression("if")?;
        let consequent = Box::new(self.statement()?);
        let alternate = if self.take(&TokenKind::Else) {
            Some(Box::new(self.statement()?))
        } else {
            None
        };
        Ok(Statement::If {
            test,
            consequent,
            alternate,
        })
    }

    fn while_statement(&mut self) -> Result<Statement, JsError> {
        let test = self.parenthesized_expression("while")?;
        self.loop_depth += 1;
        let body = self.statement();
        self.loop_depth -= 1;
        Ok(Statement::While {
            test,
            body: Box::new(body?),
        })
    }

    fn parenthesized_expression(&mut self, owner: &str) -> Result<Expression, JsError> {
        if !self.take(&TokenKind::LeftParen) {
            return Err(JsError::syntax(
                self.current().start,
                format!("expected '(' after {owner}"),
            ));
        }
        let expression = self.assignment()?;
        if !self.take(&TokenKind::RightParen) {
            return Err(JsError::syntax(
                self.current().start,
                format!("expected ')' after {owner} condition"),
            ));
        }
        Ok(expression)
    }

    fn variable_statement(&mut self, kind: VariableKind) -> Result<Statement, JsError> {
        let mut declarations = Vec::new();
        loop {
            let token = self.advance().clone();
            let TokenKind::Identifier(name) = token.kind else {
                return Err(JsError::syntax(
                    token.start,
                    "expected identifier after variable declaration",
                ));
            };

            let initializer = if self.take(&TokenKind::Equal) {
                Some(self.assignment()?)
            } else {
                None
            };
            if kind == VariableKind::Const && initializer.is_none() {
                return Err(JsError::syntax(
                    token.start,
                    "const declaration requires an initializer",
                ));
            }
            declarations.push(VariableDeclarator { name, initializer });

            if !self.take(&TokenKind::Comma) {
                break;
            }
        }

        self.optional_semicolon();
        Ok(Statement::Variable { kind, declarations })
    }

    fn assignment(&mut self) -> Result<Expression, JsError> {
        let left = self.logical_or()?;
        if !self.take(&TokenKind::Equal) {
            return Ok(left);
        }

        let Expression::Identifier(name) = left else {
            return Err(JsError::syntax(
                self.previous().start,
                "invalid assignment target",
            ));
        };
        let value = self.assignment()?;
        Ok(Expression::Assignment {
            name,
            value: Box::new(value),
        })
    }

    fn logical_or(&mut self) -> Result<Expression, JsError> {
        let mut expression = self.logical_and()?;
        while self.take(&TokenKind::PipePipe) {
            let right = self.logical_and()?;
            expression = Expression::Logical {
                left: Box::new(expression),
                op: LogicalOp::Or,
                right: Box::new(right),
            };
        }
        Ok(expression)
    }

    fn logical_and(&mut self) -> Result<Expression, JsError> {
        let mut expression = self.equality()?;
        while self.take(&TokenKind::AmpAmp) {
            let right = self.equality()?;
            expression = Expression::Logical {
                left: Box::new(expression),
                op: LogicalOp::And,
                right: Box::new(right),
            };
        }
        Ok(expression)
    }

    fn equality(&mut self) -> Result<Expression, JsError> {
        let mut expression = self.comparison()?;
        loop {
            let op = if self.take(&TokenKind::EqualEqual) {
                Some(BinaryOp::Equal)
            } else if self.take(&TokenKind::BangEqual) {
                Some(BinaryOp::NotEqual)
            } else if self.take(&TokenKind::EqualEqualEqual) {
                Some(BinaryOp::StrictEqual)
            } else if self.take(&TokenKind::BangEqualEqual) {
                Some(BinaryOp::StrictNotEqual)
            } else {
                None
            };
            let Some(op) = op else {
                return Ok(expression);
            };
            let right = self.comparison()?;
            expression = Expression::Binary {
                left: Box::new(expression),
                op,
                right: Box::new(right),
            };
        }
    }

    fn comparison(&mut self) -> Result<Expression, JsError> {
        let mut expression = self.additive()?;
        loop {
            let op = if self.take(&TokenKind::Less) {
                Some(BinaryOp::Less)
            } else if self.take(&TokenKind::LessEqual) {
                Some(BinaryOp::LessEqual)
            } else if self.take(&TokenKind::Greater) {
                Some(BinaryOp::Greater)
            } else if self.take(&TokenKind::GreaterEqual) {
                Some(BinaryOp::GreaterEqual)
            } else {
                None
            };
            let Some(op) = op else {
                return Ok(expression);
            };
            let right = self.additive()?;
            expression = Expression::Binary {
                left: Box::new(expression),
                op,
                right: Box::new(right),
            };
        }
    }

    fn additive(&mut self) -> Result<Expression, JsError> {
        let mut expression = self.multiplicative()?;
        loop {
            let op = if self.take(&TokenKind::Plus) {
                Some(BinaryOp::Add)
            } else if self.take(&TokenKind::Minus) {
                Some(BinaryOp::Subtract)
            } else {
                None
            };
            let Some(op) = op else {
                return Ok(expression);
            };
            let right = self.multiplicative()?;
            expression = Expression::Binary {
                left: Box::new(expression),
                op,
                right: Box::new(right),
            };
        }
    }

    fn multiplicative(&mut self) -> Result<Expression, JsError> {
        let mut expression = self.unary()?;
        loop {
            let op = if self.take(&TokenKind::Star) {
                Some(BinaryOp::Multiply)
            } else if self.take(&TokenKind::Slash) {
                Some(BinaryOp::Divide)
            } else if self.take(&TokenKind::Percent) {
                Some(BinaryOp::Remainder)
            } else {
                None
            };
            let Some(op) = op else {
                return Ok(expression);
            };
            let right = self.unary()?;
            expression = Expression::Binary {
                left: Box::new(expression),
                op,
                right: Box::new(right),
            };
        }
    }

    fn unary(&mut self) -> Result<Expression, JsError> {
        let op = if self.take(&TokenKind::Plus) {
            Some(UnaryOp::Plus)
        } else if self.take(&TokenKind::Minus) {
            Some(UnaryOp::Minus)
        } else if self.take(&TokenKind::Bang) {
            Some(UnaryOp::Not)
        } else {
            None
        };
        if let Some(op) = op {
            return Ok(Expression::Unary {
                op,
                argument: Box::new(self.unary()?),
            });
        }
        self.primary()
    }

    fn primary(&mut self) -> Result<Expression, JsError> {
        let token = self.advance().clone();
        match token.kind {
            TokenKind::Number(value) => Ok(Expression::Literal(JsValue::Number(value))),
            TokenKind::String(value) => Ok(Expression::Literal(JsValue::String(value))),
            TokenKind::True => Ok(Expression::Literal(JsValue::Boolean(true))),
            TokenKind::False => Ok(Expression::Literal(JsValue::Boolean(false))),
            TokenKind::Null => Ok(Expression::Literal(JsValue::Null)),
            TokenKind::Undefined => Ok(Expression::Literal(JsValue::Undefined)),
            TokenKind::Identifier(name) => Ok(Expression::Identifier(name)),
            TokenKind::LeftParen => {
                let expression = self.assignment()?;
                if !self.take(&TokenKind::RightParen) {
                    return Err(JsError::syntax(
                        self.current().start,
                        "expected ')' after expression",
                    ));
                }
                Ok(expression)
            }
            TokenKind::Eof => Err(JsError::syntax(token.start, "unexpected end of script")),
            _ => Err(JsError::syntax(token.start, "expected expression")),
        }
    }

    fn optional_semicolon(&mut self) {
        self.take(&TokenKind::Semicolon);
    }

    fn check(&self, kind: &TokenKind) -> bool {
        same_variant(&self.current().kind, kind)
    }

    fn take(&mut self, kind: &TokenKind) -> bool {
        if self.check(kind) {
            self.index += 1;
            true
        } else {
            false
        }
    }

    fn advance(&mut self) -> &Token {
        let current = self.index;
        if !self.check(&TokenKind::Eof) {
            self.index += 1;
        }
        &self.tokens[current]
    }

    fn current(&self) -> &Token {
        &self.tokens[self.index]
    }

    fn previous(&self) -> &Token {
        &self.tokens[self.index.saturating_sub(1)]
    }
}

fn same_variant(left: &TokenKind, right: &TokenKind) -> bool {
    std::mem::discriminant(left) == std::mem::discriminant(right)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_precedence_and_assignment() {
        let program = parse_script("let x = 1 + 2 * 3; x = x - 4; x").unwrap();
        assert_eq!(program.statements.len(), 3);
        let Statement::Variable { declarations, .. } = &program.statements[0] else {
            panic!("unexpected AST");
        };
        let Some(Expression::Binary { op, right, .. }) = &declarations[0].initializer else {
            panic!("unexpected initializer");
        };
        assert_eq!(*op, BinaryOp::Add);
        assert!(matches!(
            **right,
            Expression::Binary {
                op: BinaryOp::Multiply,
                ..
            }
        ));
        assert!(matches!(
            program.statements[1],
            Statement::Expression(Expression::Assignment { .. })
        ));
    }

    #[test]
    fn parses_control_flow_logical_ops_and_multiple_declarators() {
        let program = parse_script(
            "let x = 0, y = 1; while (x < 4 && y) { if (x === 2 || false) { break; } x = x + 1; continue; }",
        )
        .unwrap();
        let Statement::Variable { declarations, .. } = &program.statements[0] else {
            panic!("expected variable statement");
        };
        assert_eq!(declarations.len(), 2);
        assert!(matches!(program.statements[1], Statement::While { .. }));
    }

    #[test]
    fn break_and_continue_require_a_loop() {
        assert!(parse_script("break;").is_err());
        assert!(parse_script("continue;").is_err());
        assert!(parse_script("while (true) { break; }").is_ok());
    }

    #[test]
    fn const_requires_initializer() {
        let error = parse_script("const answer;").unwrap_err();
        assert!(error.message.contains("initializer"));
        let error = parse_script("const a = 1, b;").unwrap_err();
        assert!(error.message.contains("initializer"));
    }
}
