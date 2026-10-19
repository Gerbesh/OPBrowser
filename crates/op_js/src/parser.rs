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
    DoWhile {
        body: Box<Statement>,
        test: Expression,
    },
    For {
        initializer: Option<ForInitializer>,
        test: Option<Expression>,
        update: Option<Expression>,
        body: Box<Statement>,
    },
    ForOf {
        kind: Option<VariableKind>,
        name: String,
        iterable: Expression,
        body: Box<Statement>,
    },
    ForIn {
        kind: Option<VariableKind>,
        name: String,
        object: Expression,
        body: Box<Statement>,
    },
    Switch {
        discriminant: Expression,
        cases: Vec<SwitchCase>,
    },
    Try {
        block: Vec<Statement>,
        handler: Option<CatchClause>,
        finalizer: Option<Vec<Statement>>,
    },
    Throw(Expression),
    FunctionDeclaration {
        name: String,
        params: Vec<String>,
        body: Vec<Statement>,
        is_async: bool,
    },
    Return(Option<Expression>),
    Labeled {
        label: String,
        body: Box<Statement>,
    },
    Break(Option<String>),
    Continue(Option<String>),
    Expression(Expression),
    Empty,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VariableDeclarator {
    pub name: String,
    pub initializer: Option<Expression>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ObjectProperty {
    pub key: String,
    pub value: Expression,
    pub prototype_setter: bool,
    pub kind: ObjectPropertyKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectPropertyKind {
    Data,
    Getter,
    Setter,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ForInitializer {
    Variable {
        kind: VariableKind,
        declarations: Vec<VariableDeclarator>,
    },
    Expression(Expression),
}

#[derive(Debug, Clone, PartialEq)]
pub struct SwitchCase {
    pub test: Option<Expression>,
    pub consequent: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CatchClause {
    pub param: Option<String>,
    pub body: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AssignmentTarget {
    Identifier(String),
    Member {
        object: Box<Expression>,
        property: Box<Expression>,
    },
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
    RegExp {
        pattern: String,
        flags: String,
    },
    ForOfValues(Box<Expression>),
    ForInKeys(Box<Expression>),
    /// Untagged template: cooked quasis and ordered substitution expressions.
    Template {
        quasis: Vec<String>,
        expressions: Vec<Expression>,
    },
    Arrow {
        params: Vec<String>,
        body: Vec<Statement>,
        is_async: bool,
    },
    Identifier(String),
    Await(Box<Expression>),
    This,
    ObjectLiteral(Vec<ObjectProperty>),
    ArrayLiteral(Vec<Option<Expression>>),
    Member {
        object: Box<Expression>,
        property: Box<Expression>,
    },
    Call {
        callee: Box<Expression>,
        arguments: Vec<Expression>,
    },
    /// Compiler-only invocation with an already evaluated receiver and callee.
    CallWithReceiver {
        receiver: Box<Expression>,
        callee: Box<Expression>,
        arguments: Vec<Expression>,
    },
    New {
        callee: Box<Expression>,
        arguments: Vec<Expression>,
    },
    Function {
        name: Option<String>,
        params: Vec<String>,
        body: Vec<Statement>,
        is_async: bool,
    },
    Update {
        target: AssignmentTarget,
        op: UpdateOp,
        prefix: bool,
    },
    Unary {
        op: UnaryOp,
        argument: Box<Expression>,
    },
    Binary {
        left: Box<Expression>,
        op: BinaryOp,
        right: Box<Expression>,
    },
    Conditional {
        test: Box<Expression>,
        consequent: Box<Expression>,
        alternate: Box<Expression>,
    },
    Logical {
        left: Box<Expression>,
        op: LogicalOp,
        right: Box<Expression>,
    },
    Assignment {
        target: AssignmentTarget,
        value: Box<Expression>,
    },
    Sequence(Vec<Expression>),
    CompoundAssignment {
        target: AssignmentTarget,
        op: BinaryOp,
        value: Box<Expression>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateOp {
    Increment,
    Decrement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Plus,
    Minus,
    Not,
    BitwiseNot,
    Void,
    Delete,
    Typeof,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogicalOp {
    And,
    Or,
    Nullish,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
    ShiftLeft,
    ShiftRight,
    UnsignedShiftRight,
    BitwiseAnd,
    BitwiseXor,
    BitwiseOr,
    Equal,
    NotEqual,
    StrictEqual,
    StrictNotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    InstanceOf,
    In,
}

pub fn parse_script(source: &str) -> Result<Program, JsError> {
    Parser::new(tokenize(source)?).parse_program()
}
fn unsupported_await_location(statement: &Statement) -> &'static str {
    match statement {
        Statement::Try { .. } => "try/catch/finally",
        Statement::While { .. }
        | Statement::DoWhile { .. }
        | Statement::For { .. }
        | Statement::ForOf { .. }
        | Statement::ForIn { .. } => "loop",
        Statement::If { .. } => "if branch or test",
        Statement::Block(_) => "nested block",
        Statement::Switch { .. } => "switch",
        Statement::Labeled { .. } => "labeled statement",
        Statement::Variable { .. } => "variable initializer",
        Statement::Return(_) => "return expression",
        Statement::Throw(_) => "throw expression",
        Statement::Expression(_) => "expression",
        _ => "statement",
    }
}

struct Parser {
    tokens: Vec<Token>,
    index: usize,
    loop_depth: usize,
    breakable_depth: usize,
    function_depth: usize,
    in_async_function: bool,
    await_count: usize,
    await_offsets: Vec<usize>,
    labels: Vec<(String, bool)>,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            index: 0,
            loop_depth: 0,
            breakable_depth: 0,
            function_depth: 0,
            in_async_function: false,
            await_count: 0,
            await_offsets: Vec::new(),
            labels: Vec::new(),
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
        if matches!(self.current().kind, TokenKind::Identifier(_))
            && self
                .tokens
                .get(self.index + 1)
                .is_some_and(|token| matches!(token.kind, TokenKind::Colon))
        {
            return self.labeled_statement();
        }
        if self.take(&TokenKind::If) {
            return self.if_statement();
        }
        if self.take(&TokenKind::While) {
            return self.while_statement();
        }
        if self.take(&TokenKind::Do) {
            return self.do_while_statement();
        }
        if self.take(&TokenKind::For) {
            return self.for_statement();
        }
        if self.take(&TokenKind::Switch) {
            return self.switch_statement();
        }
        if self.take(&TokenKind::Try) {
            return self.try_statement();
        }
        if self.take(&TokenKind::Throw) {
            let value = self.sequence()?;
            self.optional_semicolon();
            return Ok(Statement::Throw(value));
        }
        if self.take(&TokenKind::Function) {
            return self.function_declaration(false);
        }
        if matches!(&self.current().kind, TokenKind::Identifier(name) if name == "async")
            && self
                .tokens
                .get(self.index + 1)
                .is_some_and(|next| matches!(next.kind, TokenKind::Function))
        {
            self.advance(); // contextual async
            self.advance(); // function
            return self.function_declaration(true);
        }
        if self.take(&TokenKind::Return) {
            if self.function_depth == 0 {
                return Err(JsError::syntax(
                    self.previous().start,
                    "return is only valid inside a function",
                ));
            }
            let value = if self.check(&TokenKind::Semicolon) || self.check(&TokenKind::RightBrace) {
                None
            } else {
                Some(self.sequence()?)
            };
            self.optional_semicolon();
            return Ok(Statement::Return(value));
        }
        if self.take(&TokenKind::Break) {
            let offset = self.previous().start;
            let label = self.optional_jump_label();
            if let Some(name) = &label {
                self.validate_jump_label(name, false, offset)?;
            } else if self.breakable_depth == 0 {
                return Err(JsError::syntax(
                    offset,
                    "break is only valid inside a loop, switch or labeled statement",
                ));
            }
            self.optional_semicolon();
            return Ok(Statement::Break(label));
        }
        if self.take(&TokenKind::Continue) {
            let offset = self.previous().start;
            let label = self.optional_jump_label();
            if let Some(name) = &label {
                self.validate_jump_label(name, true, offset)?;
            } else if self.loop_depth == 0 {
                return Err(JsError::syntax(
                    offset,
                    "continue is only valid inside a loop",
                ));
            }
            self.optional_semicolon();
            return Ok(Statement::Continue(label));
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

        let expression = self.sequence()?;
        self.optional_semicolon();
        Ok(Statement::Expression(expression))
    }

    fn labeled_statement(&mut self) -> Result<Statement, JsError> {
        let token = self.advance().clone();
        let TokenKind::Identifier(label) = token.kind else {
            unreachable!("label lookahead requires identifier")
        };
        self.advance(); // colon, validated by lookahead
        if self.labels.iter().any(|(active, _)| active == &label) {
            return Err(JsError::syntax(token.start, "duplicate statement label"));
        }
        if self.labels.len() >= 64 {
            return Err(JsError::execution_limit("statement label nesting exceeded"));
        }
        let mut lookahead = self.index;
        while let (Some(first), Some(second)) =
            (self.tokens.get(lookahead), self.tokens.get(lookahead + 1))
        {
            if matches!(first.kind, TokenKind::Identifier(_))
                && matches!(second.kind, TokenKind::Colon)
            {
                lookahead += 2;
            } else {
                break;
            }
        }
        let iteration = self.tokens.get(lookahead).is_some_and(|token| {
            matches!(
                token.kind,
                TokenKind::For | TokenKind::While | TokenKind::Do
            )
        });
        self.labels.push((label.clone(), iteration));
        let body = self.statement();
        self.labels.pop();
        Ok(Statement::Labeled {
            label,
            body: Box::new(body?),
        })
    }

    fn optional_jump_label(&mut self) -> Option<String> {
        let TokenKind::Identifier(name) = &self.current().kind else {
            return None;
        };
        let name = name.clone();
        self.advance();
        Some(name)
    }

    fn validate_jump_label(
        &self,
        name: &str,
        is_continue: bool,
        offset: usize,
    ) -> Result<(), JsError> {
        let Some((_, iteration)) = self.labels.iter().rev().find(|(label, _)| label == name) else {
            return Err(JsError::syntax(offset, "undefined statement label"));
        };
        if is_continue && !iteration {
            return Err(JsError::syntax(
                offset,
                "continue label must name an iteration statement",
            ));
        }
        Ok(())
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
        self.breakable_depth += 1;
        let body = self.statement();
        self.breakable_depth -= 1;
        self.loop_depth -= 1;
        Ok(Statement::While {
            test,
            body: Box::new(body?),
        })
    }

    fn do_while_statement(&mut self) -> Result<Statement, JsError> {
        self.loop_depth += 1;
        self.breakable_depth += 1;
        let body = self.statement();
        self.breakable_depth -= 1;
        self.loop_depth -= 1;
        let body = body?;

        if !self.take(&TokenKind::While) {
            return Err(JsError::syntax(
                self.current().start,
                "expected 'while' after do-while body",
            ));
        }
        let test = self.parenthesized_expression("while")?;
        self.optional_semicolon();
        Ok(Statement::DoWhile {
            body: Box::new(body),
            test,
        })
    }

    fn for_statement(&mut self) -> Result<Statement, JsError> {
        if !self.take(&TokenKind::LeftParen) {
            return Err(JsError::syntax(
                self.current().start,
                "expected '(' after for",
            ));
        }

        // The contextual "of" token is not a reserved word. Detect the
        // single-binding for-of grammar before the classic for initializer.
        let (kind, binding_index) = match self.current().kind {
            TokenKind::Let => (Some(VariableKind::Let), self.index + 1),
            TokenKind::Const => (Some(VariableKind::Const), self.index + 1),
            TokenKind::Var => (Some(VariableKind::Var), self.index + 1),
            _ => (None, self.index),
        };
        let binding_name = self.tokens.get(binding_index).and_then(|token| {
            if let TokenKind::Identifier(name) = &token.kind {
                Some(name.clone())
            } else {
                None
            }
        });
        let for_in = self
            .tokens
            .get(binding_index + 1)
            .and_then(|token| match &token.kind {
                TokenKind::Identifier(word) if word == "of" => Some(false),
                TokenKind::In => Some(true),
                _ => None,
            });
        if let (Some(name), Some(for_in)) = (binding_name, for_in) {
            self.index = binding_index + 2;
            // In a for-in head, the RHS is an Expression (including comma).
            // In for-of, it is only an AssignmentExpression. YouTube uses
            // "for (x in style[key] = value, other)" in shipped JS.
            let iterable = if for_in {
                self.sequence()?
            } else {
                self.assignment()?
            };
            if !self.take(&TokenKind::RightParen) {
                return Err(JsError::syntax(
                    self.current().start,
                    "expected ')' after for-of iterable",
                ));
            }
            self.loop_depth += 1;
            self.breakable_depth += 1;
            let body = self.statement();
            self.breakable_depth -= 1;
            self.loop_depth -= 1;
            return Ok(if for_in {
                Statement::ForIn {
                    kind,
                    name,
                    object: iterable,
                    body: Box::new(body?),
                }
            } else {
                Statement::ForOf {
                    kind,
                    name,
                    iterable,
                    body: Box::new(body?),
                }
            });
        }

        let initializer = if self.take(&TokenKind::Semicolon) {
            None
        } else {
            let variable_kind = if self.take(&TokenKind::Let) {
                Some(VariableKind::Let)
            } else if self.take(&TokenKind::Const) {
                Some(VariableKind::Const)
            } else if self.take(&TokenKind::Var) {
                Some(VariableKind::Var)
            } else {
                None
            };

            let initializer = if let Some(kind) = variable_kind {
                Some(ForInitializer::Variable {
                    kind,
                    declarations: self.variable_declarations(kind)?,
                })
            } else {
                Some(ForInitializer::Expression(self.sequence()?))
            };

            if !self.take(&TokenKind::Semicolon) {
                let message = match &self.current().kind {
                    TokenKind::In => "for-in enumeration is not supported",
                    TokenKind::Identifier(word) if word == "of" => {
                        "for-of requires a single identifier binding"
                    }
                    _ => "expected ';' after for initializer",
                };
                return Err(JsError::syntax(self.current().start, message));
            }
            initializer
        };

        let test = if self.take(&TokenKind::Semicolon) {
            None
        } else {
            let test = self.sequence()?;
            if !self.take(&TokenKind::Semicolon) {
                return Err(JsError::syntax(
                    self.current().start,
                    "expected ';' after for condition",
                ));
            }
            Some(test)
        };

        let update = if self.take(&TokenKind::RightParen) {
            None
        } else {
            let update = self.sequence()?;
            if !self.take(&TokenKind::RightParen) {
                return Err(JsError::syntax(
                    self.current().start,
                    "expected ')' after for update",
                ));
            }
            Some(update)
        };

        self.loop_depth += 1;
        self.breakable_depth += 1;
        let body = self.statement();
        self.breakable_depth -= 1;
        self.loop_depth -= 1;

        Ok(Statement::For {
            initializer,
            test,
            update,
            body: Box::new(body?),
        })
    }

    fn switch_statement(&mut self) -> Result<Statement, JsError> {
        let discriminant = self.parenthesized_expression("switch")?;
        if !self.take(&TokenKind::LeftBrace) {
            return Err(JsError::syntax(
                self.current().start,
                "expected '{' after switch condition",
            ));
        }

        self.breakable_depth += 1;
        let result = (|| {
            let mut cases = Vec::new();
            let mut saw_default = false;

            while !self.check(&TokenKind::RightBrace) {
                if self.check(&TokenKind::Eof) {
                    return Err(JsError::syntax(
                        self.current().start,
                        "expected '}' after switch body",
                    ));
                }

                let test = if self.take(&TokenKind::Case) {
                    let test = self.assignment()?;
                    if !self.take(&TokenKind::Colon) {
                        return Err(JsError::syntax(
                            self.current().start,
                            "expected ':' after switch case",
                        ));
                    }
                    Some(test)
                } else if self.take(&TokenKind::Default) {
                    if saw_default {
                        return Err(JsError::syntax(
                            self.previous().start,
                            "switch may only contain one default clause",
                        ));
                    }
                    saw_default = true;
                    if !self.take(&TokenKind::Colon) {
                        return Err(JsError::syntax(
                            self.current().start,
                            "expected ':' after switch default",
                        ));
                    }
                    None
                } else {
                    return Err(JsError::syntax(
                        self.current().start,
                        "expected case/default in switch body",
                    ));
                };

                let mut consequent = Vec::new();
                while !self.check(&TokenKind::Case)
                    && !self.check(&TokenKind::Default)
                    && !self.check(&TokenKind::RightBrace)
                {
                    consequent.push(self.statement()?);
                }
                cases.push(SwitchCase { test, consequent });
            }
            self.advance();
            Ok(Statement::Switch {
                discriminant,
                cases,
            })
        })();
        self.breakable_depth -= 1;
        result
    }

    fn try_statement(&mut self) -> Result<Statement, JsError> {
        let block = self.required_block("try")?;

        let handler = if self.take(&TokenKind::Catch) {
            let param = if self.take(&TokenKind::LeftParen) {
                let token = self.advance().clone();
                let TokenKind::Identifier(name) = token.kind else {
                    return Err(JsError::syntax(token.start, "expected catch binding"));
                };
                if !self.take(&TokenKind::RightParen) {
                    return Err(JsError::syntax(
                        self.current().start,
                        "expected ')' after catch binding",
                    ));
                }
                Some(name)
            } else {
                None
            };
            Some(CatchClause {
                param,
                body: self.required_block("catch")?,
            })
        } else {
            None
        };

        let finalizer = if self.take(&TokenKind::Finally) {
            Some(self.required_block("finally")?)
        } else {
            None
        };

        if handler.is_none() && finalizer.is_none() {
            return Err(JsError::syntax(
                self.current().start,
                "try requires catch or finally",
            ));
        }

        Ok(Statement::Try {
            block,
            handler,
            finalizer,
        })
    }

    fn required_block(&mut self, owner: &str) -> Result<Vec<Statement>, JsError> {
        if !self.take(&TokenKind::LeftBrace) {
            return Err(JsError::syntax(
                self.current().start,
                format!("expected '{{' after {owner}"),
            ));
        }
        let Statement::Block(statements) = self.block_statement()? else {
            unreachable!("block_statement always returns a block")
        };
        Ok(statements)
    }

    fn function_declaration(&mut self, is_async: bool) -> Result<Statement, JsError> {
        let token = self.advance().clone();
        let TokenKind::Identifier(name) = token.kind else {
            return Err(JsError::syntax(
                token.start,
                "expected function name after 'function'",
            ));
        };
        let (params, body) = self.function_tail(is_async)?;
        Ok(Statement::FunctionDeclaration {
            name,
            params,
            body,
            is_async,
        })
    }

    fn function_tail(&mut self, is_async: bool) -> Result<(Vec<String>, Vec<Statement>), JsError> {
        if !self.take(&TokenKind::LeftParen) {
            return Err(JsError::syntax(
                self.current().start,
                "expected '(' before function parameters",
            ));
        }

        let mut params = Vec::new();
        if !self.take(&TokenKind::RightParen) {
            loop {
                let token = self.advance().clone();
                let TokenKind::Identifier(name) = token.kind else {
                    return Err(JsError::syntax(token.start, "expected parameter name"));
                };
                if is_async && name == "await" {
                    return Err(JsError::syntax(
                        token.start,
                        "await cannot be an async parameter",
                    ));
                }
                params.push(name);
                if self.take(&TokenKind::RightParen) {
                    break;
                }
                if !self.take(&TokenKind::Comma) {
                    return Err(JsError::syntax(
                        self.current().start,
                        "expected ',' or ')' after parameter",
                    ));
                }
            }
        }

        if !self.take(&TokenKind::LeftBrace) {
            return Err(JsError::syntax(
                self.current().start,
                "expected '{' before function body",
            ));
        }

        let saved_loop_depth = self.loop_depth;
        let saved_breakable_depth = self.breakable_depth;
        let saved_labels = std::mem::take(&mut self.labels);
        let saved_async = self.in_async_function;
        let saved_await_count = self.await_count;
        let saved_await_offsets = std::mem::take(&mut self.await_offsets);
        self.await_count = 0;
        self.in_async_function = is_async;
        self.loop_depth = 0;
        self.breakable_depth = 0;
        self.function_depth += 1;
        let mut body = Vec::new();
        let result = (|| {
            while !self.check(&TokenKind::RightBrace) {
                if self.check(&TokenKind::Eof) {
                    return Err(JsError::syntax(
                        self.current().start,
                        "expected '}' after function body",
                    ));
                }
                let before_await = self.await_count;
                let statement = self.statement()?;
                if self.await_count != before_await
                    && !crate::await_lowering::supports_awaits(
                        &statement,
                        self.await_count - before_await,
                    )
                {
                    return Err(JsError::syntax(
                        self.await_offsets
                            .get(before_await)
                            .copied()
                            .unwrap_or(self.current().start),
                        format!(
                            "unsupported await placement in {}",
                            unsupported_await_location(&statement)
                        ),
                    ));
                }
                body.push(statement);
            }
            self.advance();
            Ok(())
        })();
        self.function_depth -= 1;
        self.loop_depth = saved_loop_depth;
        self.breakable_depth = saved_breakable_depth;
        self.labels = saved_labels;
        self.in_async_function = saved_async;
        self.await_count = saved_await_count;
        self.await_offsets = saved_await_offsets;
        result?;
        let body = if is_async {
            crate::await_lowering::lower_async_await(body)
        } else {
            body
        };
        Ok((params, body))
    }

    fn parenthesized_expression(&mut self, owner: &str) -> Result<Expression, JsError> {
        if !self.take(&TokenKind::LeftParen) {
            return Err(JsError::syntax(
                self.current().start,
                format!("expected '(' after {owner}"),
            ));
        }
        let expression = self.sequence()?;
        if !self.take(&TokenKind::RightParen) {
            return Err(JsError::syntax(
                self.current().start,
                format!("expected ')' after {owner} condition"),
            ));
        }
        Ok(expression)
    }

    fn variable_statement(&mut self, kind: VariableKind) -> Result<Statement, JsError> {
        let declarations = self.variable_declarations(kind)?;
        self.optional_semicolon();
        Ok(Statement::Variable { kind, declarations })
    }

    fn variable_declarations(
        &mut self,
        kind: VariableKind,
    ) -> Result<Vec<VariableDeclarator>, JsError> {
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
        Ok(declarations)
    }

    /// Only plain identifiers are accepted as arrow parameters for now.
    /// Lookahead leaves grouped expressions unchanged unless => follows.
    fn arrow_parameters(&self) -> Option<(Vec<String>, usize)> {
        self.arrow_parameters_at(self.index)
    }

    fn arrow_parameters_at(&self, start: usize) -> Option<(Vec<String>, usize)> {
        if let TokenKind::Identifier(name) = &self.tokens.get(start)?.kind
            && matches!(self.tokens.get(start + 1)?.kind, TokenKind::Arrow)
        {
            return Some((vec![name.clone()], start + 2));
        }
        if !matches!(self.tokens.get(start)?.kind, TokenKind::LeftParen) {
            return None;
        }
        let mut cursor = start + 1;
        let mut params = Vec::new();
        loop {
            if matches!(self.tokens.get(cursor)?.kind, TokenKind::RightParen) {
                cursor += 1;
                break;
            }
            let TokenKind::Identifier(name) = &self.tokens.get(cursor)?.kind else {
                return None;
            };
            params.push(name.clone());
            if params.len() > 128 {
                return None;
            }
            cursor += 1;
            if matches!(self.tokens.get(cursor)?.kind, TokenKind::Comma) {
                cursor += 1;
            } else if matches!(self.tokens.get(cursor)?.kind, TokenKind::RightParen) {
                cursor += 1;
                break;
            } else {
                return None;
            }
        }
        matches!(self.tokens.get(cursor)?.kind, TokenKind::Arrow).then_some((params, cursor + 1))
    }

    fn arrow_function(
        &mut self,
        params: Vec<String>,
        body_start: usize,
        is_async: bool,
    ) -> Result<Expression, JsError> {
        self.index = body_start;
        let saved_async = self.in_async_function;
        let saved_count = self.await_count;
        let saved_await_offsets = std::mem::take(&mut self.await_offsets);
        self.in_async_function = is_async;
        self.await_count = 0;
        let parsed = if self.take(&TokenKind::LeftBrace) {
            let saved_loop = self.loop_depth;
            let saved_breakable = self.breakable_depth;
            let saved_labels = std::mem::take(&mut self.labels);
            self.loop_depth = 0;
            self.breakable_depth = 0;
            self.function_depth += 1;
            let result = (|| {
                let mut body = Vec::new();
                while !self.check(&TokenKind::RightBrace) {
                    if self.check(&TokenKind::Eof) {
                        return Err(JsError::syntax(
                            self.current().start,
                            "expected '}' after arrow body",
                        ));
                    }
                    let before = self.await_count;
                    let statement = self.statement()?;
                    if self.await_count != before
                        && !crate::await_lowering::supports_awaits(
                            &statement,
                            self.await_count - before,
                        )
                    {
                        return Err(JsError::syntax(
                            self.await_offsets
                                .get(before)
                                .copied()
                                .unwrap_or(self.current().start),
                            format!(
                                "unsupported await placement in {}",
                                unsupported_await_location(&statement)
                            ),
                        ));
                    }
                    body.push(statement);
                }
                self.advance();
                Ok(body)
            })();
            self.function_depth -= 1;
            self.loop_depth = saved_loop;
            self.breakable_depth = saved_breakable;
            self.labels = saved_labels;
            result
        } else {
            let before = self.await_count;
            self.assignment().and_then(|value| {
                if self.await_count != before
                    && !crate::await_lowering::supports_awaits(
                        &Statement::Return(Some(value.clone())),
                        self.await_count - before,
                    )
                {
                    return Err(JsError::syntax(
                        self.await_offsets
                            .get(before)
                            .copied()
                            .unwrap_or(self.current().start),
                        "await is only supported in a direct initializer, statement or return",
                    ));
                }
                Ok(vec![Statement::Return(Some(value))])
            })
        };
        self.in_async_function = saved_async;
        self.await_count = saved_count;
        self.await_offsets = saved_await_offsets;
        let body = parsed?;
        Ok(Expression::Arrow {
            params,
            body: if is_async {
                crate::await_lowering::lower_async_await(body)
            } else {
                body
            },
            is_async,
        })
    }

    fn sequence(&mut self) -> Result<Expression, JsError> {
        let first = self.assignment()?;
        if !self.take(&TokenKind::Comma) {
            return Ok(first);
        }
        let mut expressions = vec![first, self.assignment()?];
        while self.take(&TokenKind::Comma) {
            expressions.push(self.assignment()?);
        }
        Ok(Expression::Sequence(expressions))
    }

    fn assignment(&mut self) -> Result<Expression, JsError> {
        if matches!(&self.current().kind, TokenKind::Identifier(name) if name == "async")
            && let Some((params, body_start)) = self.arrow_parameters_at(self.index + 1)
        {
            if params.iter().any(|param| param == "await") {
                return Err(JsError::syntax(
                    self.current().start,
                    "await cannot be an async parameter",
                ));
            }
            return self.arrow_function(params, body_start, true);
        }
        if let Some((params, body_start)) = self.arrow_parameters() {
            return self.arrow_function(params, body_start, false);
        }
        let left = self.conditional()?;
        let compound = if self.take(&TokenKind::Equal) {
            None
        } else if self.take(&TokenKind::PlusEqual) {
            Some(BinaryOp::Add)
        } else if self.take(&TokenKind::MinusEqual) {
            Some(BinaryOp::Subtract)
        } else if self.take(&TokenKind::StarEqual) {
            Some(BinaryOp::Multiply)
        } else if self.take(&TokenKind::SlashEqual) {
            Some(BinaryOp::Divide)
        } else if self.take(&TokenKind::PercentEqual) {
            Some(BinaryOp::Remainder)
        } else if self.take(&TokenKind::ShiftLeftEqual) {
            Some(BinaryOp::ShiftLeft)
        } else if self.take(&TokenKind::ShiftRightEqual) {
            Some(BinaryOp::ShiftRight)
        } else if self.take(&TokenKind::UnsignedShiftRightEqual) {
            Some(BinaryOp::UnsignedShiftRight)
        } else {
            return Ok(left);
        };

        let target = assignment_target(left, self.previous().start)?;
        let value = self.assignment()?;
        if let Some(op) = compound {
            Ok(Expression::CompoundAssignment {
                target,
                op,
                value: Box::new(value),
            })
        } else {
            Ok(Expression::Assignment {
                target,
                value: Box::new(value),
            })
        }
    }

    fn conditional(&mut self) -> Result<Expression, JsError> {
        let test = self.nullish()?;
        if !self.take(&TokenKind::Question) {
            return Ok(test);
        }
        // The two arms are assignment expressions. Recursive descent
        // preserves nested conditional right associativity and laziness.
        let consequent = self.assignment()?;
        if !self.take(&TokenKind::Colon) {
            return Err(JsError::syntax(
                self.current().start,
                "expected ':' in conditional expression",
            ));
        }
        let alternate = self.assignment()?;
        Ok(Expression::Conditional {
            test: Box::new(test),
            consequent: Box::new(consequent),
            alternate: Box::new(alternate),
        })
    }

    fn nullish(&mut self) -> Result<Expression, JsError> {
        let start = self.index;
        let mut expression = self.logical_or()?;
        let left_has_logic = self.has_unparenthesized_logic(start, self.index);
        while self.take(&TokenKind::QuestionQuestion) {
            if left_has_logic {
                return Err(JsError::syntax(
                    self.previous().start,
                    "parentheses required when mixing ?? with && or ||",
                ));
            }
            let right_start = self.index;
            let right = self.logical_or()?;
            if self.has_unparenthesized_logic(right_start, self.index) {
                return Err(JsError::syntax(
                    self.current().start,
                    "parentheses required when mixing ?? with && or ||",
                ));
            }
            expression = Expression::Logical {
                left: Box::new(expression),
                op: LogicalOp::Nullish,
                right: Box::new(right),
            };
        }
        Ok(expression)
    }

    fn has_unparenthesized_logic(&self, start: usize, end: usize) -> bool {
        let (mut parens, mut brackets, mut braces, mut templates) =
            (0usize, 0usize, 0usize, 0usize);
        for token in &self.tokens[start..end] {
            match token.kind {
                TokenKind::TemplateHead(_) => templates += 1,
                TokenKind::TemplateTail(_) => templates = templates.saturating_sub(1),
                TokenKind::LeftParen => parens += 1,
                TokenKind::RightParen => parens = parens.saturating_sub(1),
                TokenKind::LeftBracket => brackets += 1,
                TokenKind::RightBracket => brackets = brackets.saturating_sub(1),
                TokenKind::LeftBrace => braces += 1,
                TokenKind::RightBrace => braces = braces.saturating_sub(1),
                TokenKind::AmpAmp | TokenKind::PipePipe
                    if parens == 0 && brackets == 0 && braces == 0 && templates == 0 =>
                {
                    return true;
                }
                _ => {}
            }
        }
        false
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
        let mut expression = self.bitwise_or()?;
        while self.take(&TokenKind::AmpAmp) {
            let right = self.bitwise_or()?;
            expression = Expression::Logical {
                left: Box::new(expression),
                op: LogicalOp::And,
                right: Box::new(right),
            };
        }
        Ok(expression)
    }

    fn bitwise_or(&mut self) -> Result<Expression, JsError> {
        let mut expression = self.bitwise_xor()?;
        while self.take(&TokenKind::Pipe) {
            let right = self.bitwise_xor()?;
            expression = Expression::Binary {
                left: Box::new(expression),
                op: BinaryOp::BitwiseOr,
                right: Box::new(right),
            };
        }
        Ok(expression)
    }

    fn bitwise_xor(&mut self) -> Result<Expression, JsError> {
        let mut expression = self.bitwise_and()?;
        while self.take(&TokenKind::Caret) {
            let right = self.bitwise_and()?;
            expression = Expression::Binary {
                left: Box::new(expression),
                op: BinaryOp::BitwiseXor,
                right: Box::new(right),
            };
        }
        Ok(expression)
    }

    fn bitwise_and(&mut self) -> Result<Expression, JsError> {
        let mut expression = self.equality()?;
        while self.take(&TokenKind::Amp) {
            let right = self.equality()?;
            expression = Expression::Binary {
                left: Box::new(expression),
                op: BinaryOp::BitwiseAnd,
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
        let mut expression = self.shift()?;
        loop {
            let op = if self.take(&TokenKind::Less) {
                Some(BinaryOp::Less)
            } else if self.take(&TokenKind::LessEqual) {
                Some(BinaryOp::LessEqual)
            } else if self.take(&TokenKind::Greater) {
                Some(BinaryOp::Greater)
            } else if self.take(&TokenKind::GreaterEqual) {
                Some(BinaryOp::GreaterEqual)
            } else if self.take(&TokenKind::InstanceOf) {
                Some(BinaryOp::InstanceOf)
            } else if self.take(&TokenKind::In) {
                Some(BinaryOp::In)
            } else {
                None
            };
            let Some(op) = op else {
                return Ok(expression);
            };
            let right = self.shift()?;
            expression = Expression::Binary {
                left: Box::new(expression),
                op,
                right: Box::new(right),
            };
        }
    }

    fn shift(&mut self) -> Result<Expression, JsError> {
        let mut expression = self.additive()?;
        loop {
            let op = if self.take(&TokenKind::ShiftLeft) {
                BinaryOp::ShiftLeft
            } else if self.take(&TokenKind::ShiftRight) {
                BinaryOp::ShiftRight
            } else if self.take(&TokenKind::UnsignedShiftRight) {
                BinaryOp::UnsignedShiftRight
            } else {
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
        if self.in_async_function
            && matches!(&self.current().kind, TokenKind::Identifier(name) if name == "await")
        {
            let offset = self.advance().start;
            self.await_count += 1;
            self.await_offsets.push(offset);
            if self.await_count > 32 {
                return Err(JsError::syntax(offset, "await chain budget exceeded"));
            }
            return Ok(Expression::Await(Box::new(self.unary()?)));
        }
        let update = if self.take(&TokenKind::PlusPlus) {
            Some(UpdateOp::Increment)
        } else if self.take(&TokenKind::MinusMinus) {
            Some(UpdateOp::Decrement)
        } else {
            None
        };
        if let Some(op) = update {
            let offset = self.previous().start;
            let argument = self.member()?;
            return Ok(Expression::Update {
                target: assignment_target(argument, offset)?,
                op,
                prefix: true,
            });
        }

        let op = if self.take(&TokenKind::Plus) {
            Some(UnaryOp::Plus)
        } else if self.take(&TokenKind::Minus) {
            Some(UnaryOp::Minus)
        } else if self.take(&TokenKind::Bang) {
            Some(UnaryOp::Not)
        } else if self.take(&TokenKind::Tilde) {
            Some(UnaryOp::BitwiseNot)
        } else if self.take(&TokenKind::Void) {
            Some(UnaryOp::Void)
        } else if self.take(&TokenKind::Delete) {
            Some(UnaryOp::Delete)
        } else if self.take(&TokenKind::Typeof) {
            Some(UnaryOp::Typeof)
        } else {
            None
        };
        if let Some(op) = op {
            return Ok(Expression::Unary {
                op,
                argument: Box::new(self.unary()?),
            });
        }
        self.member()
    }

    fn member(&mut self) -> Result<Expression, JsError> {
        let mut expression = self.primary()?;
        loop {
            if self.take(&TokenKind::LeftParen) {
                let arguments = self.arguments_after_left_paren()?;
                expression = Expression::Call {
                    callee: Box::new(expression),
                    arguments,
                };
                continue;
            }
            if self.take(&TokenKind::Dot) {
                let token = self.advance().clone();
                let Some(name) = identifier_name(&token.kind) else {
                    return Err(JsError::syntax(
                        token.start,
                        "expected property name after '.'",
                    ));
                };
                expression = Expression::Member {
                    object: Box::new(expression),
                    property: Box::new(Expression::Literal(JsValue::String(name))),
                };
                continue;
            }

            if self.take(&TokenKind::LeftBracket) {
                let property = self.assignment()?;
                if !self.take(&TokenKind::RightBracket) {
                    return Err(JsError::syntax(
                        self.current().start,
                        "expected ']' after computed property",
                    ));
                }
                expression = Expression::Member {
                    object: Box::new(expression),
                    property: Box::new(property),
                };
                continue;
            }

            if self.check(&TokenKind::TemplateNoSub(String::new()))
                || self.check(&TokenKind::TemplateHead(String::new()))
            {
                return Err(JsError::syntax(
                    self.current().start,
                    "tagged template literals are not supported",
                ));
            }

            let update = if self.take(&TokenKind::PlusPlus) {
                Some(UpdateOp::Increment)
            } else if self.take(&TokenKind::MinusMinus) {
                Some(UpdateOp::Decrement)
            } else {
                None
            };
            if let Some(op) = update {
                let offset = self.previous().start;
                return Ok(Expression::Update {
                    target: assignment_target(expression, offset)?,
                    op,
                    prefix: false,
                });
            }

            return Ok(expression);
        }
    }

    fn primary(&mut self) -> Result<Expression, JsError> {
        let token = self.advance().clone();
        match token.kind {
            TokenKind::Number(value) => Ok(Expression::Literal(JsValue::Number(value))),
            TokenKind::RegExp(pattern, flags) => Ok(Expression::RegExp { pattern, flags }),
            TokenKind::String(value) | TokenKind::TemplateNoSub(value) => {
                Ok(Expression::Literal(JsValue::String(value)))
            }
            TokenKind::TemplateHead(first) => {
                let mut quasis = vec![first];
                let mut expressions = Vec::new();
                loop {
                    if expressions.len() >= 128 {
                        return Err(JsError::syntax(
                            token.start,
                            "template interpolation budget exceeded",
                        ));
                    }
                    expressions.push(self.assignment()?);
                    if !self.take(&TokenKind::TemplateExprEnd) {
                        return Err(JsError::syntax(
                            self.current().start,
                            "expected '}' after template substitution",
                        ));
                    }
                    match self.advance().kind.clone() {
                        TokenKind::TemplateMiddle(value) => quasis.push(value),
                        TokenKind::TemplateTail(value) => {
                            quasis.push(value);
                            return Ok(Expression::Template {
                                quasis,
                                expressions,
                            });
                        }
                        _ => {
                            return Err(JsError::syntax(
                                self.previous().start,
                                "expected template continuation",
                            ));
                        }
                    }
                }
            }
            TokenKind::True => Ok(Expression::Literal(JsValue::Boolean(true))),
            TokenKind::False => Ok(Expression::Literal(JsValue::Boolean(false))),
            TokenKind::Null => Ok(Expression::Literal(JsValue::Null)),
            TokenKind::Undefined => Ok(Expression::Literal(JsValue::Undefined)),
            TokenKind::Identifier(name) if name == "async" && self.take(&TokenKind::Function) => {
                let name = if let TokenKind::Identifier(name) = &self.current().kind {
                    let name = name.clone();
                    self.advance();
                    Some(name)
                } else {
                    None
                };
                let (params, body) = self.function_tail(true)?;
                Ok(Expression::Function {
                    name,
                    params,
                    body,
                    is_async: true,
                })
            }
            TokenKind::Identifier(name) => Ok(Expression::Identifier(name)),
            TokenKind::This => Ok(Expression::This),
            TokenKind::New => self.new_expression(token.start),
            TokenKind::Function => {
                let name = if let TokenKind::Identifier(name) = &self.current().kind {
                    let name = name.clone();
                    self.advance();
                    Some(name)
                } else {
                    None
                };
                let (params, body) = self.function_tail(false)?;
                Ok(Expression::Function {
                    name,
                    params,
                    body,
                    is_async: false,
                })
            }
            TokenKind::LeftBrace => self.object_literal(),
            TokenKind::LeftBracket => self.array_literal(),
            TokenKind::LeftParen => {
                let expression = self.sequence()?;
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

    fn new_expression(&mut self, offset: usize) -> Result<Expression, JsError> {
        if self.check(&TokenKind::Eof) {
            return Err(JsError::syntax(offset, "expected constructor after 'new'"));
        }

        let mut callee = self.primary()?;
        loop {
            if self.take(&TokenKind::Dot) {
                let token = self.advance().clone();
                let Some(name) = identifier_name(&token.kind) else {
                    return Err(JsError::syntax(
                        token.start,
                        "expected property name after '.'",
                    ));
                };
                callee = Expression::Member {
                    object: Box::new(callee),
                    property: Box::new(Expression::Literal(JsValue::String(name))),
                };
                continue;
            }

            if self.take(&TokenKind::LeftBracket) {
                let property = self.assignment()?;
                if !self.take(&TokenKind::RightBracket) {
                    return Err(JsError::syntax(
                        self.current().start,
                        "expected ']' after computed constructor property",
                    ));
                }
                callee = Expression::Member {
                    object: Box::new(callee),
                    property: Box::new(property),
                };
                continue;
            }
            break;
        }

        let arguments = if self.take(&TokenKind::LeftParen) {
            self.arguments_after_left_paren()?
        } else {
            Vec::new()
        };

        Ok(Expression::New {
            callee: Box::new(callee),
            arguments,
        })
    }

    fn arguments_after_left_paren(&mut self) -> Result<Vec<Expression>, JsError> {
        let mut arguments = Vec::new();
        if self.take(&TokenKind::RightParen) {
            return Ok(arguments);
        }

        loop {
            arguments.push(self.assignment()?);
            if self.take(&TokenKind::RightParen) {
                return Ok(arguments);
            }
            if !self.take(&TokenKind::Comma) {
                return Err(JsError::syntax(
                    self.current().start,
                    "expected ',' or ')' after argument",
                ));
            }
        }
    }

    fn object_literal(&mut self) -> Result<Expression, JsError> {
        let mut properties = Vec::new();
        let mut saw_prototype_setter = false;
        if self.take(&TokenKind::RightBrace) {
            return Ok(Expression::ObjectLiteral(properties));
        }

        loop {
            let token = self.advance().clone();
            let (key, shorthand) = match token.kind {
                TokenKind::Identifier(name) => (name.clone(), Some(name)),
                TokenKind::String(name) => (name, None),
                TokenKind::Number(value) => (JsValue::Number(value).to_js_string(), None),
                kind => {
                    let Some(name) = identifier_name(&kind) else {
                        return Err(JsError::syntax(
                            token.start,
                            "expected object property name",
                        ));
                    };
                    (name, None)
                }
            };

            // Accessor names are contextual: get(), {get: value} and
            // {get} are ordinary properties.
            let accessor = matches!(key.as_str(), "get" | "set")
                && !self.check(&TokenKind::LeftParen)
                && !self.check(&TokenKind::Colon)
                && self
                    .tokens
                    .get(self.index + 1)
                    .is_some_and(|next| same_variant(&next.kind, &TokenKind::LeftParen));
            let (key, kind, value, prototype_setter) = if accessor {
                let name_token = self.advance().clone();
                let property_name = match name_token.kind {
                    TokenKind::Identifier(name) | TokenKind::String(name) => name,
                    TokenKind::Number(number) => JsValue::Number(number).to_js_string(),
                    other => identifier_name(&other).ok_or_else(|| {
                        JsError::syntax(name_token.start, "expected accessor property name")
                    })?,
                };
                let (params, body) = self.function_tail(false)?;
                let kind = if key == "get" {
                    ObjectPropertyKind::Getter
                } else {
                    ObjectPropertyKind::Setter
                };
                let expected = if kind == ObjectPropertyKind::Getter {
                    0
                } else {
                    1
                };
                if params.len() != expected {
                    return Err(JsError::syntax(
                        name_token.start,
                        "invalid accessor parameter count",
                    ));
                }
                (
                    property_name,
                    kind,
                    Expression::Function {
                        name: None,
                        params,
                        body,
                        is_async: false,
                    },
                    false,
                )
            } else if self.check(&TokenKind::LeftParen) {
                let (params, body) = self.function_tail(false)?;
                (
                    key.clone(),
                    ObjectPropertyKind::Data,
                    Expression::Function {
                        name: Some(key),
                        params,
                        body,
                        is_async: false,
                    },
                    false,
                )
            } else {
                let has_colon = self.take(&TokenKind::Colon);
                let value = if has_colon {
                    self.assignment()?
                } else if let Some(name) = shorthand {
                    Expression::Identifier(name)
                } else {
                    return Err(JsError::syntax(
                        self.current().start,
                        "expected ':' after object property name",
                    ));
                };
                let prototype_setter = has_colon && key == "__proto__";
                (key, ObjectPropertyKind::Data, value, prototype_setter)
            };
            if prototype_setter {
                if saw_prototype_setter {
                    return Err(JsError::syntax(
                        token.start,
                        "duplicate __proto__ fields are not allowed in object literals",
                    ));
                }
                saw_prototype_setter = true;
            }
            properties.push(ObjectProperty {
                key,
                value,
                prototype_setter,
                kind,
            });

            if self.take(&TokenKind::RightBrace) {
                break;
            }
            if !self.take(&TokenKind::Comma) {
                return Err(JsError::syntax(
                    self.current().start,
                    "expected ',' or '}' after object property",
                ));
            }
            if self.take(&TokenKind::RightBrace) {
                break;
            }
        }

        Ok(Expression::ObjectLiteral(properties))
    }

    fn array_literal(&mut self) -> Result<Expression, JsError> {
        let mut elements = Vec::new();
        loop {
            if self.take(&TokenKind::RightBracket) {
                break;
            }
            if self.take(&TokenKind::Comma) {
                elements.push(None);
                continue;
            }

            elements.push(Some(self.assignment()?));
            if self.take(&TokenKind::RightBracket) {
                break;
            }
            if !self.take(&TokenKind::Comma) {
                return Err(JsError::syntax(
                    self.current().start,
                    "expected ',' or ']' after array element",
                ));
            }
        }
        Ok(Expression::ArrayLiteral(elements))
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

fn assignment_target(expression: Expression, offset: usize) -> Result<AssignmentTarget, JsError> {
    match expression {
        Expression::Identifier(name) => Ok(AssignmentTarget::Identifier(name)),
        Expression::Member { object, property } => {
            Ok(AssignmentTarget::Member { object, property })
        }
        _ => Err(JsError::syntax(offset, "invalid assignment target")),
    }
}

fn same_variant(left: &TokenKind, right: &TokenKind) -> bool {
    std::mem::discriminant(left) == std::mem::discriminant(right)
}

fn identifier_name(kind: &TokenKind) -> Option<String> {
    let name = match kind {
        TokenKind::Identifier(name) => return Some(name.clone()),
        TokenKind::Let => "let",
        TokenKind::Const => "const",
        TokenKind::Var => "var",
        TokenKind::If => "if",
        TokenKind::Else => "else",
        TokenKind::While => "while",
        TokenKind::For => "for",
        TokenKind::Do => "do",
        TokenKind::Switch => "switch",
        TokenKind::Case => "case",
        TokenKind::Default => "default",
        TokenKind::Break => "break",
        TokenKind::Continue => "continue",
        TokenKind::Function => "function",
        TokenKind::Return => "return",
        TokenKind::This => "this",
        TokenKind::New => "new",
        TokenKind::Delete => "delete",
        TokenKind::Throw => "throw",
        TokenKind::Try => "try",
        TokenKind::Catch => "catch",
        TokenKind::Finally => "finally",
        TokenKind::True => "true",
        TokenKind::False => "false",
        TokenKind::Null => "null",
        TokenKind::Undefined => "undefined",
        _ => return None,
    };
    Some(name.to_owned())
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
    fn parses_objects_arrays_members_and_member_assignment() {
        let program = parse_script(
            "let key = 'x'; let object = {x: 1, key, 3: 'three'}; let array = [object.x,, object[key]]; object[key] = array[0]; object.x",
        )
        .unwrap();
        assert!(matches!(program.statements[1], Statement::Variable { .. }));
        let Statement::Variable { declarations, .. } = &program.statements[2] else {
            panic!("expected array declaration");
        };
        assert!(matches!(
            declarations[0].initializer,
            Some(Expression::ArrayLiteral(_))
        ));
        assert!(matches!(
            program.statements[3],
            Statement::Expression(Expression::Assignment {
                target: AssignmentTarget::Member { .. },
                ..
            })
        ));
        assert!(matches!(
            program.statements[4],
            Statement::Expression(Expression::Member { .. })
        ));
    }

    #[test]
    fn async_function_parser_validates_bounded_await_subset() {
        let program = parse_script(
            "async function work(x) { return x; } let callback=async function(x){return x+1;};",
        )
        .unwrap();
        assert!(matches!(
            program.statements[0],
            Statement::FunctionDeclaration { is_async: true, .. }
        ));
        let Statement::Variable { declarations, .. } = &program.statements[1] else {
            panic!("expected async function expression");
        };
        assert!(matches!(
            declarations[0].initializer,
            Some(Expression::Function { is_async: true, .. })
        ));
        assert!(parse_script("async function f() { return await Promise.resolve(3); }").is_ok());
        assert!(
            parse_script("async function f() { const x=await Promise.resolve(3); return x+1; }")
                .is_ok()
        );
        assert!(
            parse_script("async function f() { if (true) { await Promise.resolve(3); } }").is_err()
        );
        assert!(parse_script("async function f() { const x=await Promise.resolve(3)+1; }").is_ok());
        assert!(parse_script("async function f() { const a=await 1, b=2; }").is_ok());
        assert!(
            parse_script("async function f() { try { const a=await 1, b=2; } finally {} }")
                .is_err()
        );
        assert!(parse_script("async function f() { return await await 1; }").is_ok());
        assert!(
            parse_script(
                "async function f() { async function nested(){ return await 2; } return nested(); }"
            )
            .is_ok()
        );
        assert!(parse_script("async function f(await) {return 1;}").is_err());
        assert!(
            parse_script("async function f() { function inner() { return 5; } return inner(); }")
                .is_ok()
        );
        assert!(parse_script("var async=3; async+2;").is_ok());
    }

    #[test]
    fn unsupported_await_diagnostics_point_at_the_actual_operator() {
        let source = "async function f(){try{let x=1;await Promise.resolve(2);}finally{}}";
        let error = parse_script(source).unwrap_err();
        assert_eq!(error.offset, source.find("await").unwrap());
        assert!(error.message.contains("try/catch/finally"));
        let nested = "async function g(){const a=0,b=await 1;return b;}";
        assert!(parse_script(nested).is_ok());
    }

    #[test]
    fn async_arrow_syntax_is_contextual_and_rejects_unsupported_await_positions() {
        assert!(
            parse_script("var f=async x=>x*2; var g=async (x,y)=>{const z=await x;return z+y;};")
                .is_ok()
        );
        assert!(parse_script("var g=async()=>await Promise.resolve(9);").is_ok());
        assert!(parse_script("var result=(async e=>e+e)('abc');").is_ok());
        assert!(parse_script("var async=7; var f=async=>async+1;").is_ok());
        assert!(parse_script("var g=async(await)=>1;").is_err());
        assert!(parse_script("var g=async()=>{if(true){await 1;}};").is_err());
        assert!(parse_script("var g=async()=>await Promise.resolve(1)+2;").is_ok());
    }

    #[test]
    fn parses_function_declarations_expressions_calls_and_returns() {
        let program = parse_script(
            "function add(a, b) { return a + b; } let fn1 = function inner(x) { return x; }; add(1, fn1(2));",
        )
        .unwrap();
        assert!(matches!(
            program.statements[0],
            Statement::FunctionDeclaration { .. }
        ));
        let Statement::Variable { declarations, .. } = &program.statements[1] else {
            panic!("expected function expression declaration");
        };
        assert!(matches!(
            declarations[0].initializer,
            Some(Expression::Function { .. })
        ));
        assert!(matches!(
            program.statements[2],
            Statement::Expression(Expression::Call { .. })
        ));
        assert!(parse_script("return 1;").is_err());
    }

    #[test]
    fn parses_this_and_new_constructor_expressions() {
        let program =
            parse_script("function Point(x) { this.x = x; } let point = new Point(7); point.x;")
                .unwrap();
        let Statement::FunctionDeclaration { body, .. } = &program.statements[0] else {
            panic!("expected constructor declaration");
        };
        assert!(matches!(
            body[0],
            Statement::Expression(Expression::Assignment {
                target: AssignmentTarget::Member { .. },
                ..
            })
        ));
        let Statement::Variable { declarations, .. } = &program.statements[1] else {
            panic!("expected constructed value declaration");
        };
        assert!(matches!(
            declarations[0].initializer,
            Some(Expression::New { .. })
        ));
        assert!(parse_script("new").is_err());
    }

    #[test]
    fn parses_statement_labels_and_validates_nested_break_and_continue() {
        let script = parse_script("outer: inner: for(let i=0;i<2;i++){continue outer;}").unwrap();
        let Statement::Labeled { label, body } = &script.statements[0] else {
            panic!("outer label missing");
        };
        assert_eq!(label, "outer");
        let Statement::Labeled { label, body: inner } = body.as_ref() else {
            panic!("inner label missing");
        };
        assert_eq!(label, "inner");
        assert!(matches!(inner.as_ref(), Statement::For { .. }));
        let block = parse_script("label: {break label;}").unwrap();
        assert!(matches!(block.statements[0], Statement::Labeled { .. }));
    }

    #[test]
    fn parse_web_shift_and_compound_assignments() {
        assert!(parse_script("this.playbackRate *= -1;").is_ok());
        assert!(parse_script("var i=Math.random()*1E9>>>0; d[f]=(d[f]||0)|1<<e%32;").is_ok());
        assert!(parse_script("x<<=2;y>>=2;z>>>=0;v/=2;v%=2;").is_ok());
        assert!(parse_script("1<<;").is_err());
        assert!(parse_script("var x=3; x>>>;").is_err());
    }

    #[test]
    fn parses_for_in_sequence_but_does_not_loosen_for_of_grammar() {
        let program = parse_script("for(x in a[b]=c, keys){x;}").unwrap();
        let Statement::ForIn { object, .. } = &program.statements[0] else {
            panic!("expected for-in statement")
        };
        assert!(matches!(object, Expression::Sequence(items) if items.len()==2));
        assert!(parse_script("for(const x in ({a:1},{b:2})){}").is_ok());
        assert!(parse_script("for(const x of [1,2],[3,4]){}").is_err());
    }

    #[test]
    fn parses_for_do_switch_try_catch_finally_and_throw() {
        let program = parse_script(
            "for (let i = 0; i < 3; i++) { if (i === 1) continue; }              do { break; } while (true);              switch (2) { case 1: break; case 2: throw 7; default: break; }              try { throw 1; } catch (e) { e; } finally { 2; }              try { 1; } catch { 2; }",
        )
        .unwrap();
        assert!(matches!(program.statements[0], Statement::For { .. }));
        assert!(matches!(program.statements[1], Statement::DoWhile { .. }));
        assert!(matches!(program.statements[2], Statement::Switch { .. }));
        assert!(matches!(program.statements[3], Statement::Try { .. }));
        assert!(matches!(program.statements[4], Statement::Try { .. }));
        assert!(parse_script("switch (1) { default: 1; default: 2; }").is_err());
        assert!(parse_script("try { 1; }").is_err());
    }

    #[test]
    fn duplicate_proto_setters_are_rejected_but_shorthand_is_ordinary() {
        assert!(parse_script("let o = {__proto__: null, '__proto__': null};").is_err());
        assert!(parse_script("let __proto__ = 1; let o = {__proto__};").is_ok());
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
