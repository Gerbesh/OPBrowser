use crate::{JsError, JsValue, Token, TokenKind, tokenize};

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub statements: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Variable {
        kind: VariableKind,
        name: String,
        initializer: Option<Expression>,
    },
    Expression(Expression),
    Empty,
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
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, index: 0 }
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

    fn variable_statement(&mut self, kind: VariableKind) -> Result<Statement, JsError> {
        let token = self.advance().clone();
        let TokenKind::Identifier(name) = token.kind else {
            return Err(JsError::syntax(
                token.start,
                "expected identifier after variable declaration",
            ));
        };

        if self.check(&TokenKind::Comma) {
            return Err(JsError::syntax(
                self.current().start,
                "multiple declarators are not implemented yet",
            ));
        }

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

        self.optional_semicolon();
        Ok(Statement::Variable {
            kind,
            name,
            initializer,
        })
    }

    fn assignment(&mut self) -> Result<Expression, JsError> {
        let left = self.equality()?;
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
        let Statement::Variable {
            initializer: Some(Expression::Binary { op, right, .. }),
            ..
        } = &program.statements[0]
        else {
            panic!("unexpected AST");
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
    fn const_requires_initializer() {
        let error = parse_script("const answer;").unwrap_err();
        assert!(error.message.contains("initializer"));
    }
}
