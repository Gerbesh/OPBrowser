//! Bounded CPS lowering for async expressions. This is not a resumable VM
//! frame: conditionally executed branches and try/finally require a separate
//! interpreter continuation design and remain syntax errors at the call site.
use crate::{
    AssignmentTarget, BinaryOp, Expression, JsValue, LogicalOp, ObjectPropertyKind, Statement,
    VariableDeclarator, VariableKind,
};

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
        Expression::Template {
            quasis,
            expressions,
        } => {
            // Template substitutions are stringified immediately, before
            // evaluating later substitutions. Capture cooked strings rather
            // than raw objects, whose toString would otherwise run after await.
            let position = expressions.iter().position(|expression| {
                let mut probe = 0;
                split(expression.clone(), &mut probe, resumed).is_some()
            })?;
            let mut before = Vec::new();
            let mut remaining = expressions.into_iter();
            let mut prior = Vec::new();
            for _ in 0..position {
                let expression = remaining.next().expect("previous template substitution");
                prior.push(capture(
                    &mut before,
                    Expression::Template {
                        quasis: vec![String::new(), String::new()],
                        expressions: vec![expression],
                    },
                    serial,
                ));
            }
            let mut part = split(
                remaining.next().expect("await template substitution"),
                serial,
                resumed,
            )?;
            before.append(&mut part.before);
            prior.push(part.after);
            prior.extend(remaining);
            Some(Split {
                before,
                awaited: part.awaited,
                after: Expression::Template {
                    quasis,
                    expressions: prior,
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
        Expression::ObjectLiteral(properties) => {
            // Object keys are static. Capture values preceding suspension
            // in evaluation order; accessors and __proto__ remain unsupported.
            if properties.iter().any(|property| {
                property.kind != ObjectPropertyKind::Data || property.prototype_setter
            }) {
                return None;
            }
            let position = properties.iter().position(|property| {
                let mut probe_serial = 0;
                split(property.value.clone(), &mut probe_serial, resumed).is_some()
            })?;
            let mut before = Vec::new();
            let mut remaining = properties.into_iter();
            let mut prior = Vec::new();
            for _ in 0..position {
                let mut property = remaining.next().expect("prior object property");
                property.value = capture(&mut before, property.value, serial);
                prior.push(property);
            }
            let mut property = remaining.next().expect("await object property");
            let mut part = split(property.value, serial, resumed)?;
            before.append(&mut part.before);
            property.value = part.after;
            prior.push(property);
            prior.extend(remaining);
            Some(Split {
                before,
                awaited: part.awaited,
                after: Expression::ObjectLiteral(prior),
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
            if let Some(part) = split(*test.clone(), serial, resumed) {
                return Some(Split {
                    before: part.before,
                    awaited: part.awaited,
                    after: Expression::Conditional {
                        test: Box::new(part.after),
                        consequent,
                        alternate,
                    },
                });
            }
            // Both arms suspend exactly once. Select before awaiting, never
            // evaluating a skipped arm or adding a skip-path microtask.
            let (Expression::Await(yes), Expression::Await(no)) = (*consequent, *alternate) else {
                return None;
            };
            let mut probe_serial = 0;
            if split(*yes.clone(), &mut probe_serial, resumed).is_some()
                || split(*no.clone(), &mut probe_serial, resumed).is_some()
            {
                return None;
            }
            Some(Split {
                before: Vec::new(),
                awaited: Expression::Conditional {
                    test,
                    consequent: yes,
                    alternate: no,
                },
                after: identifier(resumed.to_owned()),
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
        Statement::Throw(expression) => {
            let part = split(expression, serial, resumed)?;
            Some(reconstruct(part, Statement::Throw))
        }
        Statement::If {
            test,
            consequent,
            alternate,
        } => {
            let part = split(test, serial, resumed)?;
            Some(StatementSplit {
                before: part.before,
                awaited: part.awaited,
                after: Statement::If {
                    test: part.after,
                    consequent,
                    alternate,
                },
            })
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

/// Count awaits belonging to this function, excluding nested closures
/// that have their own async lowering. Parser's count is the independent gate.
fn expression_await_count(expression: &Expression) -> usize {
    match expression {
        Expression::Await(argument) => 1 + expression_await_count(argument),
        Expression::ForOfValues(value)
        | Expression::ForInKeys(value)
        | Expression::Unary {
            argument: value, ..
        } => expression_await_count(value),
        Expression::Template { expressions, .. } => {
            expressions.iter().map(expression_await_count).sum()
        }
        Expression::ObjectLiteral(properties) => properties
            .iter()
            .map(|property| expression_await_count(&property.value))
            .sum(),
        Expression::ArrayLiteral(elements) => {
            elements.iter().flatten().map(expression_await_count).sum()
        }
        Expression::Member { object, property }
        | Expression::Binary {
            left: object,
            right: property,
            ..
        }
        | Expression::Logical {
            left: object,
            right: property,
            ..
        } => expression_await_count(object) + expression_await_count(property),
        Expression::Call { callee, arguments } | Expression::New { callee, arguments } => {
            expression_await_count(callee)
                + arguments.iter().map(expression_await_count).sum::<usize>()
        }
        Expression::CallWithReceiver {
            receiver,
            callee,
            arguments,
        } => {
            expression_await_count(receiver)
                + expression_await_count(callee)
                + arguments.iter().map(expression_await_count).sum::<usize>()
        }
        Expression::Update { target, .. } => target_await_count(target),
        Expression::Conditional {
            test,
            consequent,
            alternate,
        } => {
            expression_await_count(test)
                + expression_await_count(consequent)
                + expression_await_count(alternate)
        }
        Expression::Assignment { target, value }
        | Expression::CompoundAssignment { target, value, .. } => {
            target_await_count(target) + expression_await_count(value)
        }
        Expression::Sequence(expressions) => expressions.iter().map(expression_await_count).sum(),
        Expression::Literal(_)
        | Expression::RegExp { .. }
        | Expression::Arrow { .. }
        | Expression::Function { .. }
        | Expression::Identifier(_)
        | Expression::This => 0,
    }
}

fn target_await_count(target: &AssignmentTarget) -> usize {
    match target {
        AssignmentTarget::Identifier(_) => 0,
        AssignmentTarget::Member { object, property } => {
            expression_await_count(object) + expression_await_count(property)
        }
    }
}

fn statement_await_count(statement: &Statement) -> usize {
    match statement {
        Statement::Expression(expression) | Statement::Throw(expression) => {
            expression_await_count(expression)
        }
        Statement::Return(expression) => expression.as_ref().map_or(0, expression_await_count),
        Statement::Variable { declarations, .. } => declarations
            .iter()
            .filter_map(|declaration| declaration.initializer.as_ref())
            .map(expression_await_count)
            .sum(),
        Statement::Block(statements) => statements.iter().map(statement_await_count).sum(),
        Statement::If {
            test,
            consequent,
            alternate,
        } => {
            expression_await_count(test)
                + statement_await_count(consequent)
                + alternate.as_deref().map_or(0, statement_await_count)
        }
        // Non-branch control flow cannot suspend until real VM frames exist.
        // The parser still counts these awaits, so the mismatch rejects them.
        _ => 0,
    }
}

fn terminates_branch(statement: &Statement) -> bool {
    match statement {
        Statement::Return(_) | Statement::Throw(_) => true,
        Statement::Block(statements) => statements.last().is_some_and(terminates_branch),
        Statement::If {
            consequent,
            alternate: Some(alternate),
            ..
        } => terminates_branch(consequent) && terminates_branch(alternate),
        _ => false,
    }
}

fn supports_branch_awaits(statement: &Statement, expected: usize) -> bool {
    match statement {
        Statement::Block(statements) => statements.iter().all(|inner| {
            let count = statement_await_count(inner);
            count == 0 || supports_awaits(inner, count)
        }),
        _ => supports_awaits(statement, expected),
    }
}

fn lower_terminating_branch(statement: Statement, serial: &mut usize) -> Statement {
    match statement {
        Statement::Block(statements) => Statement::Block(lower_sequence(statements, serial)),
        statement => Statement::Block(lower_sequence(vec![statement], serial)),
    }
}

/// Lower a returning short-circuit or one-sided conditional without
/// scheduling an await job on the branch that did not run. Rewriting the
/// return itself keeps the skipped-path Promise timing intact.
fn lower_return_branch(statement: &Statement, serial: &mut usize) -> Option<Vec<Statement>> {
    let Statement::Return(Some(value)) = statement else {
        return None;
    };
    let mut prefix = Vec::new();
    let (test, yes, no) = match value.clone() {
        Expression::Conditional {
            test,
            consequent,
            alternate,
        } => {
            // Both awaited arms already use the expression-level lowerer.
            let left_await = matches!(consequent.as_ref(), Expression::Await(_));
            let right_await = matches!(alternate.as_ref(), Expression::Await(_));
            if left_await == right_await {
                return None;
            }
            (*test, *consequent, *alternate)
        }
        Expression::Logical { left, op, right }
            if matches!(right.as_ref(), Expression::Await(_)) =>
        {
            let saved = capture(&mut prefix, *left, serial);
            match op {
                LogicalOp::And => (saved.clone(), *right, saved),
                LogicalOp::Or => (saved.clone(), saved, *right),
                LogicalOp::Nullish => (
                    Expression::Binary {
                        left: Box::new(saved.clone()),
                        op: BinaryOp::Equal,
                        right: Box::new(Expression::Literal(JsValue::Null)),
                    },
                    *right,
                    saved,
                ),
            }
        }
        _ => return None,
    };
    let consequent = lower_sequence(vec![Statement::Return(Some(yes))], serial);
    let alternate = lower_sequence(vec![Statement::Return(Some(no))], serial);
    prefix.push(Statement::If {
        test,
        consequent: Box::new(Statement::Block(consequent)),
        alternate: Some(Box::new(Statement::Block(alternate))),
    });
    Some(prefix)
}

/// Validate count of await nodes actually extractable in this statement.
/// A leftover await in a short-circuit RHS, loop, try or unsupported target
/// must not accidentally compile into the unreachable bytecode Await variant.
pub(crate) fn supports_awaits(statement: &Statement, expected: usize) -> bool {
    if let Statement::Variable { kind, declarations } = statement
        && declarations.len() > 1
    {
        let mut total = 0;
        for declaration in declarations {
            let count = declaration
                .initializer
                .as_ref()
                .map_or(0, expression_await_count);
            total += count;
            if count > 0
                && !supports_awaits(
                    &Statement::Variable {
                        kind: *kind,
                        declarations: vec![declaration.clone()],
                    },
                    count,
                )
            {
                return false;
            }
        }
        return total == expected;
    }
    // A branch which awaits may be lowered independently only when it
    // unconditionally returns/throws. Other paths still fall through to the
    // original caller's remaining statements without a synthetic scope.
    if let Statement::If {
        test,
        consequent,
        alternate,
    } = statement
    {
        let yes_count = statement_await_count(consequent);
        let no_count = alternate.as_deref().map_or(0, statement_await_count);
        if yes_count + no_count > 0 {
            return expression_await_count(test) == 0
                && expected == yes_count + no_count
                && (yes_count == 0
                    || (terminates_branch(consequent)
                        && supports_branch_awaits(consequent, yes_count)))
                && (no_count == 0
                    || alternate.as_deref().is_some_and(|branch| {
                        terminates_branch(branch) && supports_branch_awaits(branch, no_count)
                    }));
        }
    }
    let mut serial = 0;
    // A single direct await in one returning branch is supported without
    // adding a Promise job to the branch that short-circuits.
    if expected == 1 && lower_return_branch(statement, &mut serial).is_some() {
        return true;
    }
    serial = 0;
    let mut candidate = statement.clone();
    let mut extracted = 0;
    while extracted < expected {
        let resumed = fresh(&mut serial, "result");
        let Some(part) = split_statement(candidate, &mut serial, &resumed) else {
            return false;
        };
        // Two direct awaits in a conditional result in one suspension,
        // because only one of the arms will be evaluated.
        let absorbed = usize::from(matches!(part.awaited, Expression::Conditional { .. }));
        candidate = part.after;
        extracted += 1 + absorbed;
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
        if let Statement::Variable { kind, declarations } = statement.clone()
            && declarations.len() > 1
            && declarations.iter().any(|declaration| {
                declaration
                    .initializer
                    .as_ref()
                    .is_some_and(|value| expression_await_count(value) > 0)
            })
        {
            // Splitting a declaration list retains its left-to-right order
            // and lets the existing single-declaration continuation resume
            // later initializers instead of eagerly evaluating them.
            let mut split = declarations
                .into_iter()
                .map(|declaration| Statement::Variable {
                    kind,
                    declarations: vec![declaration],
                })
                .collect::<Vec<_>>();
            split.extend(remaining);
            prefix.extend(lower_sequence(split, serial));
            return prefix;
        }
        if let Statement::If {
            test,
            consequent,
            alternate,
        } = statement.clone()
        {
            let yes_count = statement_await_count(&consequent);
            let no_count = alternate.as_deref().map_or(0, statement_await_count);
            if expression_await_count(&test) == 0 && yes_count + no_count > 0 {
                prefix.push(Statement::If {
                    test,
                    consequent: Box::new(lower_terminating_branch(*consequent, serial)),
                    alternate: alternate
                        .map(|branch| Box::new(lower_terminating_branch(*branch, serial))),
                });
                continue;
            }
        }
        if let Some(branch) = lower_return_branch(&statement, serial) {
            prefix.extend(branch);
            return prefix;
        }
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
