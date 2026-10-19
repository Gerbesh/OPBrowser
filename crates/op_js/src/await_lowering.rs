//! Bounded CPS lowering for async expressions. This is not a resumable VM
//! frame: conditionally executed branches and try/finally require a separate
//! interpreter continuation design and remain syntax errors at the call site.
use crate::{AssignmentTarget, Expression, JsValue, Statement, VariableDeclarator, VariableKind};

struct Split {
    before: Vec<Statement>,
    awaited: Expression,
    after: Expression,
}

struct StatementSplit {
    before: Vec<Statement>,
    awaited: Expression,
    after: Statement,
}

fn fresh(serial: &mut usize, label: &str) -> String {
    let id = *serial;
    *serial += 1;
    format!("\0op-{label}-{id}")
}

fn identifier(name: String) -> Expression {
    Expression::Identifier(name)
}

fn declaration(name: String, value: Expression) -> Statement {
    Statement::Variable {
        kind: VariableKind::Const,
        declarations: vec![VariableDeclarator {
            name,
            initializer: Some(value),
        }],
    }
}

fn capture(before: &mut Vec<Statement>, value: Expression, serial: &mut usize) -> Expression {
    let name = fresh(serial, "capture");
    before.push(declaration(name.clone(), value));
    identifier(name)
}

/// Extract the leftmost await and freeze every operand already evaluated
/// before it. In particular, a method call must read its property/getter
/// before awaiting an argument, then forward the saved receiver to the original VM Call opcode.
fn split(expression: Expression, serial: &mut usize, resumed: &str) -> Option<Split> {
    match expression {
        Expression::Await(argument) => {
            if let Some(inner) = split(*argument.clone(), serial, resumed) {
                return Some(Split {
                    before: inner.before,
                    awaited: inner.awaited,
                    after: Expression::Await(Box::new(inner.after)),
                });
            }
            Some(Split {
                before: Vec::new(),
                awaited: *argument,
                after: identifier(resumed.to_owned()),
            })
        }
        Expression::Unary { op, argument } => {
            let part = split(*argument, serial, resumed)?;
            Some(Split {
                before: part.before,
                awaited: part.awaited,
                after: Expression::Unary {
                    op,
                    argument: Box::new(part.after),
                },
            })
        }
        Expression::Binary { left, op, right } => {
            if let Some(part) = split(*left.clone(), serial, resumed) {
                return Some(Split {
                    before: part.before,
                    awaited: part.awaited,
                    after: Expression::Binary {
                        left: Box::new(part.after),
                        op,
                        right,
                    },
                });
            }
            let mut part = split(*right, serial, resumed)?;
            let mut before = Vec::new();
            let left = capture(&mut before, *left, serial);
            before.append(&mut part.before);
            Some(Split {
                before,
                awaited: part.awaited,
                after: Expression::Binary {
                    left: Box::new(left),
                    op,
                    right: Box::new(part.after),
                },
            })
        }
        Expression::Member { object, property } => {
            if let Some(part) = split(*object.clone(), serial, resumed) {
                return Some(Split {
                    before: part.before,
                    awaited: part.awaited,
                    after: Expression::Member {
                        object: Box::new(part.after),
                        property,
                    },
                });
            }
            let mut part = split(*property, serial, resumed)?;
            let mut before = Vec::new();
            let object = capture(&mut before, *object, serial);
            before.append(&mut part.before);
            Some(Split {
                before,
                awaited: part.awaited,
                after: Expression::Member {
                    object: Box::new(object),
                    property: Box::new(part.after),
                },
            })
        }
        Expression::Call { callee, arguments } => {
            if let Some(part) = split(*callee.clone(), serial, resumed) {
                return Some(Split {
                    before: part.before,
                    awaited: part.awaited,
                    after: Expression::Call {
                        callee: Box::new(part.after),
                        arguments,
                    },
                });
            }
            let position = arguments.iter().position(|arg| {
                let mut probe_serial = 0;
                split(arg.clone(), &mut probe_serial, resumed).is_some()
            })?;
            let mut before = Vec::new();
            let mut remaining = arguments.into_iter();
            let mut prior = Vec::new();
            let (after_callee, receiver) = match *callee {
                Expression::Member { object, property } => {
                    let receiver = capture(&mut before, *object, serial);
                    let key = capture(&mut before, *property, serial);
                    let function = capture(
                        &mut before,
                        Expression::Member {
                            object: Box::new(receiver.clone()),
                            property: Box::new(key),
                        },
                        serial,
                    );
                    (function, Some(receiver))
                }
                expression => (capture(&mut before, expression, serial), None),
            };
            for _ in 0..position {
                prior.push(capture(
                    &mut before,
                    remaining.next().expect("previous argument"),
                    serial,
                ));
            }
            let mut part = split(remaining.next().expect("await argument"), serial, resumed)?;
            before.append(&mut part.before);
            prior.push(part.after);
            prior.extend(remaining);
            Some(Split {
                before,
                awaited: part.awaited,
                after: if let Some(receiver) = receiver {
                    Expression::CallWithReceiver {
                        receiver: Box::new(receiver),
                        callee: Box::new(after_callee),
                        arguments: prior,
                    }
                } else {
                    Expression::Call {
                        callee: Box::new(after_callee),
                        arguments: prior,
                    }
                },
            })
        }
        Expression::CallWithReceiver {
            receiver,
            callee,
            arguments,
        } => {
            if let Some(part) = split(*receiver.clone(), serial, resumed) {
                return Some(Split {
                    before: part.before,
                    awaited: part.awaited,
                    after: Expression::CallWithReceiver {
                        receiver: Box::new(part.after),
                        callee,
                        arguments,
                    },
                });
            }
            if let Some(part) = split(*callee.clone(), serial, resumed) {
                let mut before = Vec::new();
                let saved_receiver = capture(&mut before, *receiver, serial);
                before.extend(part.before);
                return Some(Split {
                    before,
                    awaited: part.awaited,
                    after: Expression::CallWithReceiver {
                        receiver: Box::new(saved_receiver),
                        callee: Box::new(part.after),
                        arguments,
                    },
                });
            }
            let position = arguments.iter().position(|arg| {
                let mut probe_serial = 0;
                split(arg.clone(), &mut probe_serial, resumed).is_some()
            })?;
            let mut before = Vec::new();
            let receiver = capture(&mut before, *receiver, serial);
            let callee = capture(&mut before, *callee, serial);
            let mut remaining = arguments.into_iter();
            let mut prior = Vec::new();
            for _ in 0..position {
                prior.push(capture(
                    &mut before,
                    remaining.next().expect("prior explicit receiver argument"),
                    serial,
                ));
            }
            let mut part = split(remaining.next().expect("await argument"), serial, resumed)?;
            before.append(&mut part.before);
            prior.push(part.after);
            prior.extend(remaining);
            Some(Split {
                before,
                awaited: part.awaited,
                after: Expression::CallWithReceiver {
                    receiver: Box::new(receiver),
                    callee: Box::new(callee),
                    arguments: prior,
                },
            })
        }
        Expression::New { callee, arguments } => {
            if let Some(part) = split(*callee.clone(), serial, resumed) {
                return Some(Split {
                    before: part.before,
                    awaited: part.awaited,
                    after: Expression::New {
                        callee: Box::new(part.after),
                        arguments,
                    },
                });
            }
            let position = arguments.iter().position(|arg| {
                let mut probe_serial = 0;
                split(arg.clone(), &mut probe_serial, resumed).is_some()
            })?;
            let mut before = Vec::new();
            let constructor = capture(&mut before, *callee, serial);
            let mut remaining = arguments.into_iter();
            let mut prior = Vec::new();
            for _ in 0..position {
                prior.push(capture(
                    &mut before,
                    remaining.next().expect("previous constructor argument"),
                    serial,
                ));
            }
            let mut part = split(
                remaining.next().expect("constructor await argument"),
                serial,
                resumed,
            )?;
            before.append(&mut part.before);
            prior.push(part.after);
            prior.extend(remaining);
            Some(Split {
                before,
                awaited: part.awaited,
                after: Expression::New {
                    callee: Box::new(constructor),
                    arguments: prior,
                },
            })
        }
        Expression::ArrayLiteral(elements) => {
            let position = elements.iter().position(|element| {
                let mut probe_serial = 0;
                element
                    .as_ref()
                    .is_some_and(|e| split(e.clone(), &mut probe_serial, resumed).is_some())
            })?;
            let mut before = Vec::new();
            let mut remaining = elements.into_iter();
            let mut prior = Vec::new();
            for _ in 0..position {
                prior.push(
                    remaining
                        .next()
                        .expect("prior array slot")
                        .map(|value| capture(&mut before, value, serial)),
                );
            }
            let mut part = split(
                remaining.next().flatten().expect("await element"),
                serial,
                resumed,
            )?;
            before.append(&mut part.before);
            prior.push(Some(part.after));
            prior.extend(remaining);
            Some(Split {
                before,
                awaited: part.awaited,
                after: Expression::ArrayLiteral(prior),
            })
        }
        Expression::Sequence(expressions) => {
            let position = expressions.iter().position(|expr| {
                let mut probe_serial = 0;
                split(expr.clone(), &mut probe_serial, resumed).is_some()
            })?;
            let mut remaining = expressions.into_iter();
            let mut before = Vec::new();
            for _ in 0..position {
                before.push(Statement::Expression(
                    remaining.next().expect("previous sequence item"),
                ));
            }
            let mut part = split(
                remaining.next().expect("await sequence item"),
                serial,
                resumed,
            )?;
            before.append(&mut part.before);
            let mut tail = vec![part.after];
            tail.extend(remaining);
            Some(Split {
                before,
                awaited: part.awaited,
                after: if tail.len() == 1 {
                    tail.remove(0)
                } else {
                    Expression::Sequence(tail)
                },
            })
        }
        Expression::Assignment {
            target: AssignmentTarget::Identifier(name),
            value,
        } => {
            let part = split(*value, serial, resumed)?;
            Some(Split {
                before: part.before,
                awaited: part.awaited,
                after: Expression::Assignment {
                    target: AssignmentTarget::Identifier(name),
                    value: Box::new(part.after),
                },
            })
        }
        Expression::Conditional {
            test,
            consequent,
            alternate,
        } => {
            let part = split(*test, serial, resumed)?;
            Some(Split {
                before: part.before,
                awaited: part.awaited,
                after: Expression::Conditional {
                    test: Box::new(part.after),
                    consequent,
                    alternate,
                },
            })
        }
        Expression::Logical { left, op, right } => {
            // Right operand may be skipped, and cannot be extracted eagerly.
            let part = split(*left, serial, resumed)?;
            Some(Split {
                before: part.before,
                awaited: part.awaited,
                after: Expression::Logical {
                    left: Box::new(part.after),
                    op,
                    right,
                },
            })
        }
        _ => None,
    }
}

fn split_statement(
    statement: Statement,
    serial: &mut usize,
    resumed: &str,
) -> Option<StatementSplit> {
    let reconstruct = |part: Split, build: fn(Expression) -> Statement| StatementSplit {
        before: part.before,
        awaited: part.awaited,
        after: build(part.after),
    };
    match statement {
        Statement::Expression(expression) => {
            let part = split(expression, serial, resumed)?;
            Some(reconstruct(part, Statement::Expression))
        }
        Statement::Return(Some(expression)) => {
            let part = split(expression, serial, resumed)?;
            Some(reconstruct(part, |expr| Statement::Return(Some(expr))))
        }
        Statement::Variable {
            kind,
            mut declarations,
        } if declarations.len() == 1 => {
            let mut declaration = declarations.pop().expect("one declarator");
            let part = split(declaration.initializer.take()?, serial, resumed)?;
            declaration.initializer = Some(part.after);
            Some(StatementSplit {
                before: part.before,
                awaited: part.awaited,
                after: Statement::Variable {
                    kind,
                    declarations: vec![declaration],
                },
            })
        }
        _ => None,
    }
}

/// Validate count of await nodes actually extractable in this statement.
/// A leftover await in a short-circuit RHS, loop, try or unsupported target
/// must not accidentally compile into the unreachable bytecode Await variant.
pub(crate) fn supports_awaits(statement: &Statement, expected: usize) -> bool {
    let mut serial = 0;
    let mut candidate = statement.clone();
    for _ in 0..expected {
        let resumed = fresh(&mut serial, "result");
        let Some(part) = split_statement(candidate, &mut serial, &resumed) else {
            return false;
        };
        candidate = part.after;
    }
    let resumed = fresh(&mut serial, "result");
    split_statement(candidate, &mut serial, &resumed).is_none()
}

pub(crate) fn lower_async_await(statements: Vec<Statement>) -> Vec<Statement> {
    let mut serial = 0;
    lower_sequence(statements, &mut serial)
}

fn lower_sequence(statements: Vec<Statement>, serial: &mut usize) -> Vec<Statement> {
    let mut prefix = Vec::new();
    let mut remaining = statements.into_iter();
    while let Some(statement) = remaining.next() {
        let resumed = fresh(serial, "result");
        if let Some(part) = split_statement(statement.clone(), serial, &resumed) {
            prefix.extend(part.before);
            let mut tail = vec![part.after];
            tail.extend(remaining);
            prefix.push(Statement::Return(Some(await_chain(
                part.awaited,
                Expression::Arrow {
                    params: vec![resumed],
                    body: lower_sequence(tail, serial),
                    is_async: false,
                },
            ))));
            return prefix;
        }
        prefix.push(statement);
    }
    prefix
}

fn await_chain(value: Expression, callback: Expression) -> Expression {
    let resolved = Expression::Call {
        callee: Box::new(Expression::Member {
            object: Box::new(Expression::Identifier("Promise".into())),
            property: Box::new(Expression::Literal(JsValue::String("resolve".into()))),
        }),
        arguments: vec![value],
    };
    Expression::Call {
        callee: Box::new(Expression::Member {
            object: Box::new(resolved),
            property: Box::new(Expression::Literal(JsValue::String("then".into()))),
        }),
        arguments: vec![callback],
    }
}
