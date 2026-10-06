use crate::{
    AttributeMatcher, AttributeSelector, Combinator, CompoundSelector, CssError, Declaration,
    NthExpression, ParseResult, PseudoClass, PseudoElement, Selector, SimpleSelector, Specificity,
    StyleRule, Stylesheet, Token, TokenKind, tokenize,
};

pub fn parse_stylesheet(input: &str) -> ParseResult<Stylesheet> {
    let tokenized = tokenize(input);
    let mut parser = Parser {
        tokens: &tokenized.tokens,
        errors: tokenized.errors,
    };
    let rules = parser.parse_rule_list();
    ParseResult {
        value: Stylesheet { rules },
        errors: parser.errors,
    }
}

pub fn parse_declaration_list(input: &str) -> ParseResult<Vec<Declaration>> {
    let tokenized = tokenize(input);
    let mut parser = Parser {
        tokens: &tokenized.tokens,
        errors: tokenized.errors,
    };
    let declarations = parser.parse_declarations(&tokenized.tokens);
    ParseResult {
        value: declarations,
        errors: parser.errors,
    }
}

struct Parser<'a> {
    tokens: &'a [Token],
    errors: Vec<CssError>,
}

impl Parser<'_> {
    fn parse_rule_list(&mut self) -> Vec<StyleRule> {
        let mut rules = Vec::new();
        let mut index = 0;
        while index < self.tokens.len() {
            while index < self.tokens.len()
                && matches!(
                    self.tokens[index].kind,
                    TokenKind::Whitespace | TokenKind::Semicolon
                )
            {
                index += 1;
            }
            if index >= self.tokens.len() {
                break;
            }

            if matches!(self.tokens[index].kind, TokenKind::AtKeyword(_)) {
                let start = self.tokens[index].start;
                index = skip_at_rule(self.tokens, index);
                self.errors.push(CssError {
                    offset: start,
                    message: "at-rules are not supported yet and were ignored".into(),
                });
                continue;
            }

            let prelude_start = index;
            let Some(open) = find_rule_block_start(self.tokens, index) else {
                self.errors.push(CssError {
                    offset: self.tokens[index].start,
                    message: "qualified rule is missing an opening block".into(),
                });
                break;
            };
            let Some(close) = find_matching_close_curly(self.tokens, open) else {
                self.errors.push(CssError {
                    offset: self.tokens[open].start,
                    message: "unterminated CSS rule block".into(),
                });
                break;
            };

            let prelude = trim_whitespace(&self.tokens[prelude_start..open]);
            let body = &self.tokens[open + 1..close];
            match parse_selector_list(prelude) {
                Ok(selectors) if !selectors.is_empty() => {
                    let declarations = self.parse_declarations(body);
                    rules.push(StyleRule {
                        selectors,
                        declarations,
                    });
                }
                Ok(_) => self.errors.push(CssError {
                    offset: self.tokens[open].start,
                    message: "empty selector list".into(),
                }),
                Err(error) => self.errors.push(error),
            }

            index = close + 1;
        }
        rules
    }

    fn parse_declarations(&mut self, tokens: &[Token]) -> Vec<Declaration> {
        let mut declarations = Vec::new();
        let mut start = 0;
        while start < tokens.len() {
            while start < tokens.len()
                && matches!(
                    tokens[start].kind,
                    TokenKind::Whitespace | TokenKind::Semicolon
                )
            {
                start += 1;
            }
            if start >= tokens.len() {
                break;
            }

            let end = find_declaration_end(tokens, start);
            let segment = trim_whitespace(&tokens[start..end]);
            if !segment.is_empty() {
                match parse_declaration(segment) {
                    Ok(declaration) => declarations.push(declaration),
                    Err(error) => self.errors.push(error),
                }
            }
            start = if end < tokens.len() { end + 1 } else { end };
        }
        declarations
    }
}

fn parse_declaration(tokens: &[Token]) -> Result<Declaration, CssError> {
    let Some(first) = tokens.first() else {
        unreachable!("empty declarations are filtered by caller");
    };
    let TokenKind::Ident(name) = &first.kind else {
        return Err(CssError {
            offset: first.start,
            message: "declaration must start with a property name".into(),
        });
    };
    if name == "--" {
        return Err(CssError {
            offset: first.start,
            message: "bare '--' is not a custom property name".into(),
        });
    }

    let mut colon = 1;
    while colon < tokens.len() && matches!(tokens[colon].kind, TokenKind::Whitespace) {
        colon += 1;
    }
    if colon >= tokens.len() || !matches!(tokens[colon].kind, TokenKind::Colon) {
        return Err(CssError {
            offset: first.start,
            message: format!("property {name} is missing ':'"),
        });
    }

    let value_tokens = trim_whitespace(&tokens[colon + 1..]);
    let (value_tokens, important) = strip_important(value_tokens);
    if value_tokens.is_empty() && !name.starts_with("--") {
        return Err(CssError {
            offset: tokens[colon].end,
            message: format!("property {name} has an empty value"),
        });
    }
    let normalized_name = if name.starts_with("--") {
        name.clone()
    } else {
        name.to_ascii_lowercase()
    };

    let value: Vec<_> = value_tokens
        .iter()
        .map(|token| token.kind.clone())
        .collect();
    if !crate::custom::valid_var_syntax(&value) {
        return Err(CssError {
            offset: tokens[colon].end,
            message: format!("property {name} has invalid var() syntax"),
        });
    }
    Ok(Declaration {
        name: normalized_name,
        value,
        important,
    })
}

fn strip_important(tokens: &[Token]) -> (&[Token], bool) {
    let mut end = tokens.len();
    while end > 0 && matches!(tokens[end - 1].kind, TokenKind::Whitespace) {
        end -= 1;
    }
    if end == 0 {
        return (&tokens[..end], false);
    }
    let TokenKind::Ident(keyword) = &tokens[end - 1].kind else {
        return (&tokens[..end], false);
    };
    if !keyword.eq_ignore_ascii_case("important") {
        return (&tokens[..end], false);
    }
    let mut bang = end - 1;
    while bang > 0 && matches!(tokens[bang - 1].kind, TokenKind::Whitespace) {
        bang -= 1;
    }
    if bang == 0 || tokens[bang - 1].kind != TokenKind::Delim('!') {
        return (&tokens[..end], false);
    }
    let mut value_end = bang - 1;
    while value_end > 0 && matches!(tokens[value_end - 1].kind, TokenKind::Whitespace) {
        value_end -= 1;
    }
    (&tokens[..value_end], true)
}

fn parse_selector_list(tokens: &[Token]) -> Result<Vec<Selector>, CssError> {
    if tokens.is_empty() {
        return Ok(Vec::new());
    }
    let mut selectors = Vec::new();
    let mut start = 0;
    let mut parens = 0_u32;
    let mut squares = 0_u32;
    for index in 0..=tokens.len() {
        let at_separator = if index == tokens.len() {
            true
        } else {
            match tokens[index].kind {
                TokenKind::Function(_) | TokenKind::OpenParen => {
                    parens += 1;
                    false
                }
                TokenKind::CloseParen => {
                    parens = parens.saturating_sub(1);
                    false
                }
                TokenKind::OpenSquare => {
                    squares += 1;
                    false
                }
                TokenKind::CloseSquare => {
                    squares = squares.saturating_sub(1);
                    false
                }
                TokenKind::Comma if parens == 0 && squares == 0 => true,
                _ => false,
            }
        };
        if at_separator {
            let group = trim_whitespace(&tokens[start..index]);
            if group.is_empty() {
                let offset = tokens
                    .get(index)
                    .or_else(|| tokens.last())
                    .map_or(0, |token| token.start);
                return Err(CssError {
                    offset,
                    message: "empty selector in selector list".into(),
                });
            }
            selectors.push(parse_selector(group)?);
            start = index + 1;
        }
    }
    Ok(selectors)
}

fn parse_selector(tokens: &[Token]) -> Result<Selector, CssError> {
    let mut compounds = Vec::new();
    let mut combinators = Vec::new();
    let mut pseudo_element = None;
    let mut specificity = Specificity::default();
    let mut index = 0;

    loop {
        let (compound, compound_pseudo, next) = parse_compound(tokens, index, &mut specificity)?;
        compounds.push(compound);
        index = next;

        if let Some(pseudo) = compound_pseudo {
            pseudo_element = Some(pseudo);
            while index < tokens.len() && matches!(tokens[index].kind, TokenKind::Whitespace) {
                index += 1;
            }
            if index != tokens.len() {
                return Err(CssError {
                    offset: tokens[index].start,
                    message: "pseudo-element must terminate the selector".into(),
                });
            }
            break;
        }

        let whitespace_start = index;
        while index < tokens.len() && matches!(tokens[index].kind, TokenKind::Whitespace) {
            index += 1;
        }
        let had_whitespace = index > whitespace_start;
        if index >= tokens.len() {
            break;
        }

        let explicit = match tokens[index].kind {
            TokenKind::Delim('>') => Some((Combinator::Child, '>')),
            TokenKind::Delim('+') => Some((Combinator::AdjacentSibling, '+')),
            TokenKind::Delim('~') => Some((Combinator::GeneralSibling, '~')),
            _ => None,
        };
        if let Some((combinator, symbol)) = explicit {
            combinators.push(combinator);
            index += 1;
            while index < tokens.len() && matches!(tokens[index].kind, TokenKind::Whitespace) {
                index += 1;
            }
            if index >= tokens.len() {
                return Err(CssError {
                    offset: tokens.last().map_or(0, |token| token.end),
                    message: format!("selector cannot end with '{symbol}'"),
                });
            }
        } else if had_whitespace {
            combinators.push(Combinator::Descendant);
        } else {
            return Err(CssError {
                offset: tokens[index].start,
                message: "unsupported or malformed selector syntax".into(),
            });
        }
    }

    debug_assert_eq!(compounds.len(), combinators.len() + 1);
    Ok(Selector {
        compounds,
        combinators,
        pseudo_element,
        specificity,
    })
}

fn parse_compound(
    tokens: &[Token],
    start: usize,
    specificity: &mut Specificity,
) -> Result<(CompoundSelector, Option<PseudoElement>, usize), CssError> {
    let mut index = start;
    let mut simple = Vec::new();
    let mut pseudo_element = None;

    if let Some(token) = tokens.get(index) {
        match &token.kind {
            TokenKind::Ident(name) => {
                let selector = SimpleSelector::Type(name.to_ascii_lowercase());
                specificity.add_simple(&selector);
                simple.push(selector);
                index += 1;
            }
            TokenKind::Delim('*') => {
                simple.push(SimpleSelector::Universal);
                index += 1;
            }
            _ => {}
        }
    }

    while let Some(token) = tokens.get(index) {
        match &token.kind {
            TokenKind::Hash { value, id: true } => {
                let selector = SimpleSelector::Id(value.clone());
                specificity.add_simple(&selector);
                simple.push(selector);
                index += 1;
            }
            TokenKind::Delim('.') => {
                let Some(next) = tokens.get(index + 1) else {
                    return Err(CssError {
                        offset: token.start,
                        message: "class selector is missing a class name".into(),
                    });
                };
                let TokenKind::Ident(name) = &next.kind else {
                    return Err(CssError {
                        offset: next.start,
                        message: "class selector requires an identifier".into(),
                    });
                };
                let selector = SimpleSelector::Class(name.clone());
                specificity.add_simple(&selector);
                simple.push(selector);
                index += 2;
            }
            TokenKind::OpenSquare => {
                let (selector, next) = parse_attribute_selector(tokens, index)?;
                specificity.add_simple(&selector);
                simple.push(selector);
                index = next;
            }
            TokenKind::Colon
                if matches!(
                    tokens.get(index + 1).map(|token| &token.kind),
                    Some(TokenKind::Colon)
                ) =>
            {
                let (pseudo, next) = parse_pseudo_element(tokens, index)?;
                specificity.types = specificity.types.saturating_add(1);
                pseudo_element = Some(pseudo);
                index = next;
                break;
            }
            TokenKind::Colon => {
                let (selector, next) = parse_pseudo_class(tokens, index)?;
                specificity.add_simple(&selector);
                simple.push(selector);
                index = next;
            }
            _ => break,
        }
    }

    if simple.is_empty() {
        if pseudo_element.is_some() {
            simple.push(SimpleSelector::Universal);
        } else {
            let token = tokens.get(start).expect("selector group is non-empty");
            return Err(CssError {
                offset: token.start,
                message: "unsupported or malformed selector syntax".into(),
            });
        }
    }
    Ok((CompoundSelector { simple }, pseudo_element, index))
}

fn parse_pseudo_element(
    tokens: &[Token],
    start: usize,
) -> Result<(PseudoElement, usize), CssError> {
    let Some(name_token) = tokens.get(start + 2) else {
        return Err(selector_error(
            tokens,
            start,
            "pseudo-element is missing a name",
        ));
    };
    let TokenKind::Ident(name) = &name_token.kind else {
        return Err(CssError {
            offset: name_token.start,
            message: "pseudo-element requires an identifier".into(),
        });
    };
    let pseudo = match name.to_ascii_lowercase().as_str() {
        "before" => PseudoElement::Before,
        "after" => PseudoElement::After,
        _ => {
            return Err(CssError {
                offset: name_token.start,
                message: format!("unsupported pseudo-element ::{name}"),
            });
        }
    };
    Ok((pseudo, start + 3))
}

fn parse_attribute_selector(
    tokens: &[Token],
    start: usize,
) -> Result<(SimpleSelector, usize), CssError> {
    let mut index = start + 1;
    skip_selector_whitespace(tokens, &mut index);
    let Some(name_token) = tokens.get(index) else {
        return Err(selector_error(
            tokens,
            start,
            "unterminated attribute selector",
        ));
    };
    let TokenKind::Ident(name) = &name_token.kind else {
        return Err(CssError {
            offset: name_token.start,
            message: "attribute selector requires an attribute name".into(),
        });
    };
    let name = name.to_ascii_lowercase();
    index += 1;
    skip_selector_whitespace(tokens, &mut index);

    if matches!(
        tokens.get(index).map(|token| &token.kind),
        Some(TokenKind::CloseSquare)
    ) {
        return Ok((
            SimpleSelector::Attribute(AttributeSelector {
                name,
                matcher: AttributeMatcher::Exists,
                value: None,
                case_insensitive: false,
            }),
            index + 1,
        ));
    }

    let (matcher, operator_len) = match tokens.get(index).map(|token| &token.kind) {
        Some(TokenKind::Delim('=')) => (AttributeMatcher::Exact, 1),
        Some(TokenKind::Delim('~'))
            if matches!(
                tokens.get(index + 1).map(|token| &token.kind),
                Some(TokenKind::Delim('='))
            ) =>
        {
            (AttributeMatcher::Includes, 2)
        }
        Some(TokenKind::Delim('|'))
            if matches!(
                tokens.get(index + 1).map(|token| &token.kind),
                Some(TokenKind::Delim('='))
            ) =>
        {
            (AttributeMatcher::DashMatch, 2)
        }
        Some(TokenKind::Delim('^'))
            if matches!(
                tokens.get(index + 1).map(|token| &token.kind),
                Some(TokenKind::Delim('='))
            ) =>
        {
            (AttributeMatcher::Prefix, 2)
        }
        Some(TokenKind::Delim('$'))
            if matches!(
                tokens.get(index + 1).map(|token| &token.kind),
                Some(TokenKind::Delim('='))
            ) =>
        {
            (AttributeMatcher::Suffix, 2)
        }
        Some(TokenKind::Delim('*'))
            if matches!(
                tokens.get(index + 1).map(|token| &token.kind),
                Some(TokenKind::Delim('='))
            ) =>
        {
            (AttributeMatcher::Substring, 2)
        }
        Some(token) => {
            return Err(CssError {
                offset: tokens[index].start,
                message: format!("unsupported attribute selector operator: {token:?}"),
            });
        }
        None => {
            return Err(selector_error(
                tokens,
                start,
                "unterminated attribute selector",
            ));
        }
    };
    index += operator_len;
    skip_selector_whitespace(tokens, &mut index);

    let Some(value_token) = tokens.get(index) else {
        return Err(selector_error(
            tokens,
            start,
            "attribute selector is missing a value",
        ));
    };
    let value = match &value_token.kind {
        TokenKind::Ident(value) | TokenKind::String(value) => value.clone(),
        _ => {
            return Err(CssError {
                offset: value_token.start,
                message: "attribute selector value must be an identifier or string".into(),
            });
        }
    };
    index += 1;
    skip_selector_whitespace(tokens, &mut index);

    let mut case_insensitive = false;
    if let Some(Token {
        kind: TokenKind::Ident(flag),
        ..
    }) = tokens.get(index)
    {
        if flag.eq_ignore_ascii_case("i") {
            case_insensitive = true;
        } else if !flag.eq_ignore_ascii_case("s") {
            return Err(CssError {
                offset: tokens[index].start,
                message: "attribute selector flag must be i or s".into(),
            });
        }
        index += 1;
        skip_selector_whitespace(tokens, &mut index);
    }

    if !matches!(
        tokens.get(index).map(|token| &token.kind),
        Some(TokenKind::CloseSquare)
    ) {
        return Err(selector_error(
            tokens,
            start,
            "unterminated attribute selector",
        ));
    }

    Ok((
        SimpleSelector::Attribute(AttributeSelector {
            name,
            matcher,
            value: Some(value),
            case_insensitive,
        }),
        index + 1,
    ))
}

fn parse_pseudo_class(tokens: &[Token], start: usize) -> Result<(SimpleSelector, usize), CssError> {
    let Some(token) = tokens.get(start + 1) else {
        return Err(selector_error(
            tokens,
            start,
            "pseudo-class is missing a name",
        ));
    };
    match &token.kind {
        TokenKind::Ident(name) => {
            let pseudo = match name.to_ascii_lowercase().as_str() {
                "root" => PseudoClass::Root,
                "first-child" => PseudoClass::FirstChild,
                "last-child" => PseudoClass::LastChild,
                "only-child" => PseudoClass::OnlyChild,
                "empty" => PseudoClass::Empty,
                "link" => PseudoClass::Link,
                _ => {
                    return Err(CssError {
                        offset: token.start,
                        message: format!("unsupported pseudo-class :{name}"),
                    });
                }
            };
            Ok((SimpleSelector::PseudoClass(pseudo), start + 2))
        }
        TokenKind::Function(name) => parse_functional_pseudo(tokens, start + 1, name),
        _ => Err(CssError {
            offset: token.start,
            message: "pseudo-class requires an identifier or supported function".into(),
        }),
    }
}

fn parse_functional_pseudo(
    tokens: &[Token],
    function_index: usize,
    name: &str,
) -> Result<(SimpleSelector, usize), CssError> {
    let Some(close) = find_matching_close_paren(tokens, function_index) else {
        return Err(selector_error(
            tokens,
            function_index,
            "unterminated functional pseudo-class",
        ));
    };
    let arguments = trim_whitespace(&tokens[function_index + 1..close]);
    if arguments.is_empty() {
        return Err(CssError {
            offset: tokens[function_index].start,
            message: format!("functional pseudo-class :{name}() requires an argument"),
        });
    }

    let selector = match name.to_ascii_lowercase().as_str() {
        "is" => SimpleSelector::Is(parse_function_selector_list(
            arguments,
            tokens[function_index].start,
        )?),
        "where" => SimpleSelector::Where(parse_function_selector_list(
            arguments,
            tokens[function_index].start,
        )?),
        "not" => SimpleSelector::Not(parse_function_selector_list(
            arguments,
            tokens[function_index].start,
        )?),
        "nth-child" => {
            SimpleSelector::NthChild(parse_nth_expression(arguments).ok_or_else(|| CssError {
                offset: tokens[function_index].start,
                message: "invalid :nth-child() expression".into(),
            })?)
        }
        _ => {
            return Err(CssError {
                offset: tokens[function_index].start,
                message: format!("unsupported functional pseudo-class :{name}()"),
            });
        }
    };
    Ok((selector, close + 1))
}

fn parse_function_selector_list(
    tokens: &[Token],
    offset: usize,
) -> Result<Vec<Selector>, CssError> {
    let selectors = parse_selector_list(tokens)?;
    if selectors
        .iter()
        .any(|selector| selector.pseudo_element.is_some())
    {
        return Err(CssError {
            offset,
            message: "pseudo-elements are not supported inside functional pseudo-class arguments"
                .into(),
        });
    }
    Ok(selectors)
}

fn find_matching_close_paren(tokens: &[Token], function_index: usize) -> Option<usize> {
    let mut depth = 1_u32;
    for (index, token) in tokens.iter().enumerate().skip(function_index + 1) {
        match token.kind {
            TokenKind::Function(_) | TokenKind::OpenParen => depth += 1,
            TokenKind::CloseParen => {
                depth -= 1;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

fn parse_nth_expression(tokens: &[Token]) -> Option<NthExpression> {
    let mut value = String::new();
    for token in tokens
        .iter()
        .filter(|token| !matches!(token.kind, TokenKind::Whitespace))
    {
        match &token.kind {
            TokenKind::Ident(part) | TokenKind::Number(part) => value.push_str(part),
            TokenKind::Dimension { number, unit } => {
                value.push_str(number);
                value.push_str(unit);
            }
            TokenKind::Delim('+' | '-') => {
                if let TokenKind::Delim(ch) = token.kind {
                    value.push(ch);
                }
            }
            _ => return None,
        }
    }
    let value = value.to_ascii_lowercase();
    match value.as_str() {
        "odd" => return Some(NthExpression { a: 2, b: 1 }),
        "even" => return Some(NthExpression { a: 2, b: 0 }),
        _ => {}
    }
    if let Some(n) = value.find('n') {
        if value[n + 1..].contains('n') {
            return None;
        }
        let a = match &value[..n] {
            "" | "+" => 1,
            "-" => -1,
            value => value.parse::<i32>().ok()?,
        };
        let b = if value[n + 1..].is_empty() {
            0
        } else {
            value[n + 1..].parse::<i32>().ok()?
        };
        Some(NthExpression { a, b })
    } else {
        Some(NthExpression {
            a: 0,
            b: value.parse::<i32>().ok()?,
        })
    }
}

fn skip_selector_whitespace(tokens: &[Token], index: &mut usize) {
    while *index < tokens.len() && matches!(tokens[*index].kind, TokenKind::Whitespace) {
        *index += 1;
    }
}

fn selector_error(tokens: &[Token], start: usize, message: &str) -> CssError {
    CssError {
        offset: tokens.get(start).map_or(0, |token| token.start),
        message: message.into(),
    }
}

fn find_rule_block_start(tokens: &[Token], start: usize) -> Option<usize> {
    let mut parens = 0_u32;
    let mut squares = 0_u32;
    for (index, token) in tokens.iter().enumerate().skip(start) {
        match token.kind {
            TokenKind::OpenParen | TokenKind::Function(_) => parens += 1,
            TokenKind::CloseParen => parens = parens.saturating_sub(1),
            TokenKind::OpenSquare => squares += 1,
            TokenKind::CloseSquare => squares = squares.saturating_sub(1),
            TokenKind::OpenCurly if parens == 0 && squares == 0 => return Some(index),
            TokenKind::CloseCurly if parens == 0 && squares == 0 => return None,
            _ => {}
        }
    }
    None
}

fn find_matching_close_curly(tokens: &[Token], open: usize) -> Option<usize> {
    let mut depth = 1_u32;
    for (index, token) in tokens.iter().enumerate().skip(open + 1) {
        match token.kind {
            TokenKind::OpenCurly => depth += 1,
            TokenKind::CloseCurly => {
                depth -= 1;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

fn find_declaration_end(tokens: &[Token], start: usize) -> usize {
    let mut parens = 0_u32;
    let mut squares = 0_u32;
    let mut curlies = 0_u32;
    for (index, token) in tokens.iter().enumerate().skip(start) {
        match token.kind {
            TokenKind::OpenParen | TokenKind::Function(_) => parens += 1,
            TokenKind::CloseParen => parens = parens.saturating_sub(1),
            TokenKind::OpenSquare => squares += 1,
            TokenKind::CloseSquare => squares = squares.saturating_sub(1),
            TokenKind::OpenCurly => curlies += 1,
            TokenKind::CloseCurly => curlies = curlies.saturating_sub(1),
            TokenKind::Semicolon if parens == 0 && squares == 0 && curlies == 0 => return index,
            _ => {}
        }
    }
    tokens.len()
}

fn skip_at_rule(tokens: &[Token], start: usize) -> usize {
    let mut index = start + 1;
    let mut block_depth = 0_u32;
    while index < tokens.len() {
        match tokens[index].kind {
            TokenKind::Semicolon if block_depth == 0 => return index + 1,
            TokenKind::OpenCurly => block_depth += 1,
            TokenKind::CloseCurly => {
                if block_depth == 0 {
                    return index;
                }
                block_depth -= 1;
                if block_depth == 0 {
                    return index + 1;
                }
            }
            _ => {}
        }
        index += 1;
    }
    index
}

fn trim_whitespace(mut tokens: &[Token]) -> &[Token] {
    while tokens
        .first()
        .is_some_and(|token| matches!(token.kind, TokenKind::Whitespace))
    {
        tokens = &tokens[1..];
    }
    while tokens
        .last()
        .is_some_and(|token| matches!(token.kind, TokenKind::Whitespace))
    {
        tokens = &tokens[..tokens.len() - 1];
    }
    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_selector_groups_combinators_specificity_and_declarations() {
        let parsed = parse_stylesheet(
            "article.card > p.note, #hero { Color: red; margin: 10px 20px !important; }",
        );
        assert!(parsed.errors.is_empty(), "{:?}", parsed.errors);
        assert_eq!(parsed.value.rules.len(), 1);
        let rule = &parsed.value.rules[0];
        assert_eq!(rule.selectors.len(), 2);
        assert_eq!(rule.selectors[0].combinators, vec![Combinator::Child]);
        assert_eq!(
            rule.selectors[0].specificity,
            Specificity {
                ids: 0,
                classes: 2,
                types: 2,
            }
        );
        assert_eq!(
            rule.selectors[1].specificity,
            Specificity {
                ids: 1,
                classes: 0,
                types: 0,
            }
        );
        assert_eq!(rule.declarations[0].name, "color");
        assert_eq!(rule.declarations[1].name, "margin");
        assert!(rule.declarations[1].important);
    }

    #[test]
    fn parses_attribute_sibling_and_structural_pseudo_selectors() {
        let parsed = parse_stylesheet(
            "main[data-mode='dark' i] > a[href^='https'][rel~=external]:link + span:first-child ~ em:last-child { color:red }",
        );
        assert!(parsed.errors.is_empty(), "{:?}", parsed.errors);
        let selector = &parsed.value.rules[0].selectors[0];
        assert_eq!(
            selector.combinators,
            vec![
                Combinator::Child,
                Combinator::AdjacentSibling,
                Combinator::GeneralSibling,
            ]
        );
        assert_eq!(
            selector.specificity,
            Specificity {
                ids: 0,
                classes: 6,
                types: 4,
            }
        );
        let SimpleSelector::Attribute(attribute) = &selector.compounds[0].simple[1] else {
            panic!("expected attribute selector");
        };
        assert_eq!(attribute.matcher, AttributeMatcher::Exact);
        assert_eq!(attribute.value.as_deref(), Some("dark"));
        assert!(attribute.case_insensitive);
        assert!(matches!(
            selector.compounds[1].simple.last(),
            Some(SimpleSelector::PseudoClass(PseudoClass::Link))
        ));
    }

    #[test]
    fn parses_functional_pseudo_classes_and_specificity() {
        let parsed = parse_stylesheet(
            "section:is(.card, #hero) > p:not(.skip):where(.note, #ignored):nth-child(2n+1) { color:red }",
        );
        assert!(parsed.errors.is_empty(), "{:?}", parsed.errors);
        let selector = &parsed.value.rules[0].selectors[0];
        assert_eq!(
            selector.specificity,
            Specificity {
                ids: 1,
                classes: 2,
                types: 2,
            }
        );
        assert!(matches!(
            selector.compounds[0].simple.last(),
            Some(SimpleSelector::Is(selectors)) if selectors.len() == 2
        ));
        assert!(matches!(
            selector.compounds[1].simple[1],
            SimpleSelector::Not(_)
        ));
        assert!(matches!(
            selector.compounds[1].simple[2],
            SimpleSelector::Where(_)
        ));
        assert_eq!(
            selector.compounds[1].simple[3],
            SimpleSelector::NthChild(NthExpression { a: 2, b: 1 })
        );
    }

    #[test]
    fn parses_before_after_as_terminal_pseudo_elements_with_type_specificity() {
        let parsed = parse_stylesheet(".note::before, ::after { content:\"!\"; color:red }");
        assert!(parsed.errors.is_empty(), "{:?}", parsed.errors);
        let selectors = &parsed.value.rules[0].selectors;
        assert_eq!(selectors[0].pseudo_element, Some(PseudoElement::Before));
        assert_eq!(selectors[1].pseudo_element, Some(PseudoElement::After));
        assert_eq!(
            selectors[0].specificity,
            Specificity {
                ids: 0,
                classes: 1,
                types: 1,
            }
        );
        assert_eq!(
            selectors[1].specificity,
            Specificity {
                ids: 0,
                classes: 0,
                types: 1,
            }
        );

        let invalid = parse_stylesheet("div::before span { color:red }");
        assert!(invalid.value.rules.is_empty());
        assert_eq!(invalid.errors.len(), 1);
        assert!(invalid.errors[0].message.contains("terminate"));

        let nested = parse_stylesheet("div:is(::before,.note) { color:red }");
        assert!(nested.value.rules.is_empty());
        assert_eq!(nested.errors.len(), 1);
        assert!(nested.errors[0].message.contains("inside functional"));
    }

    #[test]
    fn parses_all_attribute_match_operators_and_case_flags() {
        let parsed = parse_stylesheet(
            "[a][b=x][c~=y][d|=en][e^=pre][f$='end'][g*=mid i][h=value s] { color:red }",
        );
        assert!(parsed.errors.is_empty(), "{:?}", parsed.errors);
        let simple = &parsed.value.rules[0].selectors[0].compounds[0].simple;
        let matchers: Vec<_> = simple
            .iter()
            .filter_map(|selector| match selector {
                SimpleSelector::Attribute(attribute) => Some(attribute.matcher),
                _ => None,
            })
            .collect();
        assert_eq!(
            matchers,
            vec![
                AttributeMatcher::Exists,
                AttributeMatcher::Exact,
                AttributeMatcher::Includes,
                AttributeMatcher::DashMatch,
                AttributeMatcher::Prefix,
                AttributeMatcher::Suffix,
                AttributeMatcher::Substring,
                AttributeMatcher::Exact,
            ]
        );
    }

    #[test]
    fn parses_inline_declaration_lists_and_preserves_custom_property_case() {
        let parsed = parse_declaration_list("--Brand: #fff; FONT-SIZE: 16px; broken; color: blue");
        assert_eq!(parsed.value.len(), 3);
        assert_eq!(parsed.value[0].name, "--Brand");
        assert_eq!(parsed.value[1].name, "font-size");
        assert_eq!(parsed.value[2].name, "color");
        assert_eq!(parsed.errors.len(), 1);
        assert!(parsed.errors[0].message.contains("missing ':'"));
    }

    #[test]
    fn accepts_empty_custom_properties_and_rejects_bare_name_and_empty_normal_values() {
        let parsed = parse_declaration_list(
            "--empty:; --priority: !important; --:red; color:; padding: !important; --valid:blue",
        );
        assert_eq!(parsed.value.len(), 3);
        assert_eq!(parsed.errors.len(), 3);
        assert!(parsed.value[0].value.is_empty());
        assert!(parsed.value[1].value.is_empty());
        assert!(parsed.value[1].important);
        assert_eq!(parsed.value[2].name, "--valid");
    }

    #[test]
    fn rejects_malformed_var_syntax_even_inside_unused_fallback_branches() {
        let parsed = parse_declaration_list(
            "color:red; color:var(foo); color:var(--known,var(--)); --x:var(--a --b); padding:var(--missing,); color:var(--good,blue)",
        );
        assert_eq!(parsed.errors.len(), 3);
        assert_eq!(parsed.value.len(), 3);
        assert_eq!(parsed.value[0].value, vec![TokenKind::Ident("red".into())]);
        assert_eq!(parsed.value[1].name, "padding");
        assert_eq!(parsed.value[2].name, "color");
    }

    #[test]
    fn malformed_rule_does_not_poison_following_rule() {
        let parsed = parse_stylesheet(
            ".ok { color: red; bad } :hover { color: black } p.note { width: 20px }",
        );
        assert_eq!(parsed.value.rules.len(), 2);
        assert_eq!(parsed.value.rules[0].selectors[0].specificity.classes, 1);
        assert_eq!(parsed.value.rules[1].selectors[0].specificity.types, 1);
        assert_eq!(parsed.value.rules[1].selectors[0].specificity.classes, 1);
        assert_eq!(parsed.errors.len(), 2);
    }

    #[test]
    fn ignores_at_rules_as_an_explicit_initial_limitation() {
        let parsed = parse_stylesheet("@media screen { p { color:red } } h1 { color: blue }");
        assert_eq!(parsed.value.rules.len(), 1);
        assert_eq!(parsed.errors.len(), 1);
        assert!(parsed.errors[0].message.contains("at-rules"));
    }

    #[test]
    fn rejects_non_identifier_hash_as_id_selector() {
        let parsed = parse_stylesheet("#123 { color: red } #hero { color: blue }");
        assert_eq!(parsed.value.rules.len(), 1);
        assert_eq!(parsed.errors.len(), 1);
        assert_eq!(
            parsed.value.rules[0].selectors[0].specificity,
            Specificity {
                ids: 1,
                classes: 0,
                types: 0,
            }
        );
    }

    #[test]
    fn descendant_selector_is_distinct_from_compound_selector() {
        let parsed = parse_stylesheet("main .card.feature span { color: red }");
        assert!(parsed.errors.is_empty());
        let selector = &parsed.value.rules[0].selectors[0];
        assert_eq!(
            selector.combinators,
            vec![Combinator::Descendant, Combinator::Descendant]
        );
        assert_eq!(selector.compounds.len(), 3);
        assert_eq!(
            selector.specificity,
            Specificity {
                ids: 0,
                classes: 2,
                types: 2,
            }
        );
    }
}
