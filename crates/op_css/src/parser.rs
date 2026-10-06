use crate::{
    AttributeMatcher, AttributeSelector, Combinator, CompoundSelector, CssError, Declaration,
    NthExpression, NthSelector, ParseResult, PseudoClass, PseudoElement, Selector, SimpleSelector,
    Specificity, StyleRule, Stylesheet, Token, TokenKind, tokenize,
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

            if let TokenKind::AtKeyword(name) = &self.tokens[index].kind {
                if name.eq_ignore_ascii_case("supports") {
                    let start = self.tokens[index].start;
                    let Some(open) = find_rule_block_start(self.tokens, index + 1) else {
                        self.errors.push(CssError {
                            offset: start,
                            message: "@supports is missing an opening block".into(),
                        });
                        break;
                    };
                    let Some(close) = find_matching_close_curly(self.tokens, open) else {
                        self.errors.push(CssError {
                            offset: start,
                            message: "unterminated @supports block".into(),
                        });
                        break;
                    };
                    if supports_declaration_condition(&self.tokens[index + 1..open]) {
                        let mut nested = Parser {
                            tokens: &self.tokens[open + 1..close],
                            errors: Vec::new(),
                        };
                        rules.extend(nested.parse_rule_list());
                        self.errors.extend(nested.errors);
                    }
                    index = close + 1;
                    continue;
                }

                let start = self.tokens[index].start;
                index = skip_at_rule(self.tokens, index);
                self.errors.push(CssError {
                    offset: start,
                    message: "at-rule is not supported yet and was ignored".into(),
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

fn supports_declaration_condition(tokens: &[Token]) -> bool {
    let tokens = trim_whitespace(tokens);
    let [
        Token {
            kind: TokenKind::OpenParen,
            ..
        },
        middle @ ..,
        Token {
            kind: TokenKind::CloseParen,
            ..
        },
    ] = tokens
    else {
        return false;
    };
    let middle = trim_whitespace(middle);
    let Some(colon) = middle
        .iter()
        .position(|token| matches!(token.kind, TokenKind::Colon))
    else {
        return false;
    };
    let property = trim_whitespace(&middle[..colon]);
    let value = trim_whitespace(&middle[colon + 1..]);
    let [
        Token {
            kind: TokenKind::Ident(property),
            ..
        },
    ] = property
    else {
        return false;
    };
    if value.is_empty() {
        return false;
    }
    let value: Vec<TokenKind> = value.iter().map(|token| token.kind.clone()).collect();
    crate::computed::supports_declaration_value(property, &value)
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
    if value
        .iter()
        .any(|token| matches!(token, TokenKind::BadUrl | TokenKind::BadString))
    {
        return Err(CssError {
            offset: tokens[colon].end,
            message: format!("property {name} contains an invalid URL or string token"),
        });
    }
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
    parse_selector_list_mode(tokens, false)
}

fn parse_selector_list_mode(tokens: &[Token], forgiving: bool) -> Result<Vec<Selector>, CssError> {
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
                    if parens > 64 {
                        return Err(selector_error(
                            tokens,
                            index,
                            "selector function nesting exceeds 64 levels",
                        ));
                    }
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
                if forgiving {
                    start = index + 1;
                    continue;
                }
                let offset = tokens
                    .get(index)
                    .or_else(|| tokens.last())
                    .map_or(0, |token| token.start);
                return Err(CssError {
                    offset,
                    message: "empty selector in selector list".into(),
                });
            }
            match parse_selector(group) {
                Ok(selector) if !forgiving || selector.pseudo_element.is_none() => {
                    selectors.push(selector)
                }
                Ok(_) => {}
                Err(_) if forgiving => {}
                Err(error) => return Err(error),
            }
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
                "first-of-type" => PseudoClass::FirstOfType,
                "last-of-type" => PseudoClass::LastOfType,
                "only-of-type" => PseudoClass::OnlyOfType,
                "empty" => PseudoClass::Empty,
                "link" => PseudoClass::Link,
                "visited" => PseudoClass::Visited,
                "required" => PseudoClass::Required,
                "optional" => PseudoClass::Optional,
                "open" => PseudoClass::Open,
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
    if arguments.is_empty() && !matches!(name.to_ascii_lowercase().as_str(), "is" | "where") {
        return Err(CssError {
            offset: tokens[function_index].start,
            message: format!("functional pseudo-class :{name}() requires an argument"),
        });
    }

    let selector = match name.to_ascii_lowercase().as_str() {
        "is" => SimpleSelector::Is(parse_function_selector_list(
            arguments,
            tokens[function_index].start,
            true,
        )?),
        "where" => SimpleSelector::Where(parse_function_selector_list(
            arguments,
            tokens[function_index].start,
            true,
        )?),
        "not" => SimpleSelector::Not(parse_function_selector_list(
            arguments,
            tokens[function_index].start,
            false,
        )?),
        "lang" => SimpleSelector::Lang(parse_lang_ranges(arguments, tokens[function_index].start)?),
        "dir" => SimpleSelector::Dir(parse_dir_value(arguments, tokens[function_index].start)?),
        "nth-child" | "nth-last-child" => SimpleSelector::NthChild(parse_nth_selector(
            arguments,
            name.eq_ignore_ascii_case("nth-last-child"),
            tokens[function_index].start,
        )?),
        "nth-of-type" | "nth-last-of-type" => SimpleSelector::NthChild(NthSelector {
            expression: parse_nth_expression(arguments).ok_or_else(|| CssError {
                offset: tokens[function_index].start,
                message: "invalid nth-of-type/nth-last-of-type expression".into(),
            })?,
            of: Vec::new(),
            from_end: name.eq_ignore_ascii_case("nth-last-of-type"),
            same_type: true,
        }),
        _ => {
            return Err(CssError {
                offset: tokens[function_index].start,
                message: format!("unsupported functional pseudo-class :{name}()"),
            });
        }
    };
    Ok((selector, close + 1))
}

fn parse_lang_ranges(tokens: &[Token], offset: usize) -> Result<Vec<String>, CssError> {
    let mut ranges = Vec::new();
    let mut start = 0usize;
    for end in 0..=tokens.len() {
        if end != tokens.len() && !matches!(tokens[end].kind, TokenKind::Comma) {
            continue;
        }
        let segment = trim_whitespace(&tokens[start..end]);
        let [token] = segment else {
            return Err(CssError {
                offset,
                message: ":lang() requires a comma-separated list of identifiers or strings".into(),
            });
        };
        let range = match &token.kind {
            TokenKind::Ident(value) | TokenKind::String(value) => value.clone(),
            _ => {
                return Err(CssError {
                    offset: token.start,
                    message: ":lang() ranges must be identifiers or strings".into(),
                });
            }
        };
        ranges.push(range);
        start = end.saturating_add(1);
    }
    if ranges.is_empty() {
        return Err(CssError {
            offset,
            message: ":lang() requires at least one language range".into(),
        });
    }
    Ok(ranges)
}

fn parse_dir_value(tokens: &[Token], offset: usize) -> Result<String, CssError> {
    let [
        Token {
            kind: TokenKind::Ident(value),
            ..
        },
    ] = trim_whitespace(tokens)
    else {
        return Err(CssError {
            offset,
            message: ":dir() requires one identifier".into(),
        });
    };
    Ok(value.clone())
}

fn parse_function_selector_list(
    tokens: &[Token],
    offset: usize,
    forgiving: bool,
) -> Result<Vec<Selector>, CssError> {
    let selectors = parse_selector_list_mode(tokens, forgiving)?;
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

fn parse_nth_selector(
    tokens: &[Token],
    from_end: bool,
    offset: usize,
) -> Result<NthSelector, CssError> {
    let of_index = tokens.iter().position(|token| {
        matches!(&token.kind,
        TokenKind::Ident(name) if name.eq_ignore_ascii_case("of"))
    });
    let expression_tokens = of_index.map_or(tokens, |index| trim_whitespace(&tokens[..index]));
    let expression = parse_nth_expression(expression_tokens).ok_or_else(|| CssError {
        offset,
        message: "invalid nth-child/nth-last-child expression".into(),
    })?;
    let of = if let Some(index) = of_index {
        if index == 0 || !matches!(tokens[index - 1].kind, TokenKind::Whitespace) {
            return Err(CssError {
                offset,
                message: "nth selector requires whitespace before 'of'".into(),
            });
        }
        let selectors =
            parse_function_selector_list(trim_whitespace(&tokens[index + 1..]), offset, false)?;
        if selectors.is_empty() {
            return Err(CssError {
                offset,
                message: "nth selector 'of' list cannot be empty".into(),
            });
        }
        selectors
    } else {
        Vec::new()
    };
    Ok(NthSelector {
        expression,
        of,
        from_end,
        same_type: false,
    })
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
    let tokens = trim_whitespace(tokens);
    let mut parts: Vec<&Token> = tokens
        .iter()
        .filter(|token| !matches!(token.kind, TokenKind::Whitespace))
        .collect();
    if parts.len() == 1 {
        match &parts[0].kind {
            TokenKind::Ident(name) if name.eq_ignore_ascii_case("odd") => {
                return Some(NthExpression { a: 2, b: 1 });
            }
            TokenKind::Ident(name) if name.eq_ignore_ascii_case("even") => {
                return Some(NthExpression { a: 2, b: 0 });
            }
            TokenKind::Number(number) => {
                return Some(NthExpression {
                    a: 0,
                    b: number.parse().ok()?,
                });
            }
            _ => {}
        }
    }
    if matches!(
        parts.first().map(|token| &token.kind),
        Some(TokenKind::Delim('+'))
    ) {
        // The optional '+' before an n-ident cannot be separated by whitespace.
        if !matches!(tokens.get(1).map(|token| &token.kind), Some(TokenKind::Ident(name)) if name.starts_with(['n', 'N']))
        {
            return None;
        }
        parts.remove(0);
    }
    let (a, unit) = match &parts.first()?.kind {
        TokenKind::Dimension { number, unit } => {
            (number.parse::<i32>().ok()?, unit.to_ascii_lowercase())
        }
        TokenKind::Ident(name) => {
            let name = name.to_ascii_lowercase();
            if let Some(unit) = name.strip_prefix('-') {
                (-1, unit.to_owned())
            } else {
                (1, name)
            }
        }
        _ => return None,
    };
    let suffix = unit.strip_prefix('n')?;
    let rest = &parts[1..];
    let b = if suffix.is_empty() {
        match rest {
            [] => 0,
            [
                Token {
                    kind: TokenKind::Number(number),
                    ..
                },
            ] if number.starts_with(['+', '-']) => number.parse::<i32>().ok()?,
            [
                Token {
                    kind: TokenKind::Delim(sign @ ('+' | '-')),
                    ..
                },
                Token {
                    kind: TokenKind::Number(number),
                    ..
                },
            ] if !number.starts_with(['+', '-']) => {
                let value = number.parse::<i64>().ok()?;
                i32::try_from(if *sign == '-' { -value } else { value }).ok()?
            }
            _ => return None,
        }
    } else if suffix == "-" {
        let [
            Token {
                kind: TokenKind::Number(number),
                ..
            },
        ] = rest
        else {
            return None;
        };
        if number.starts_with(['+', '-']) {
            return None;
        }
        i32::try_from(-number.parse::<i64>().ok()?).ok()?
    } else {
        let digits = suffix.strip_prefix('-')?;
        if !rest.is_empty()
            || digits.is_empty()
            || !digits.bytes().all(|byte| byte.is_ascii_digit())
        {
            return None;
        }
        i32::try_from(-digits.parse::<i64>().ok()?).ok()?
    };
    Some(NthExpression { a, b })
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
    fn url_values_are_atomic_and_bad_tokens_invalidate_declarations() {
        let parsed = parse_declaration_list(
            "--icon:url(data:image/png;base64,a+b/c==); --bad:url(a b); color:var(--known, url(a b)); content:var(--icon); color:green",
        );
        assert_eq!(parsed.value.len(), 3);
        assert_eq!(
            parsed.value[0].value,
            [TokenKind::Url("data:image/png;base64,a+b/c==".into())]
        );
        assert_eq!(parsed.value[1].name, "content");
        assert_eq!(parsed.value[2].value, [TokenKind::Ident("green".into())]);
        assert!(
            parsed
                .errors
                .iter()
                .any(|error| error.message.contains("invalid URL"))
        );
        let parsed = parse_declaration_list("--bad:'oops\n'; color:green");
        assert!(
            parsed
                .value
                .iter()
                .all(|declaration| declaration.name != "--bad")
        );
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
            SimpleSelector::NthChild(NthSelector {
                expression: NthExpression { a: 2, b: 1 },
                of: Vec::new(),
                from_end: false,
                same_type: false
            })
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
        assert_eq!(nested.value.rules.len(), 1);
        assert!(nested.errors.is_empty());
        assert!(
            matches!(&nested.value.rules[0].selectors[0].compounds[0].simple[1],
            SimpleSelector::Is(selectors) if selectors.len() == 1)
        );
        let strict = parse_stylesheet("div:not(::before,.note) { color:red }");
        assert!(strict.value.rules.is_empty());
        assert_eq!(strict.errors.len(), 1);
    }

    #[test]
    fn forgiving_is_where_discard_invalid_branches_and_keep_valid_specificity() {
        let parsed = parse_stylesheet(
            "div:is(:unsupported, [bad=], ::before, .valid, #hero, ) { color:red }
             div:where(:unsupported, #hero) { color:blue }
             :is(:unsupported), :where(), :is() { color:green }",
        );
        assert!(parsed.errors.is_empty(), "{:?}", parsed.errors);
        assert_eq!(parsed.value.rules.len(), 3);
        let selector = &parsed.value.rules[0].selectors[0];
        assert_eq!(
            selector.specificity,
            Specificity {
                ids: 1,
                classes: 0,
                types: 1
            }
        );
        assert!(
            matches!(&selector.compounds[0].simple[1], SimpleSelector::Is(selectors) if selectors.len() == 2)
        );
        assert_eq!(
            parsed.value.rules[1].selectors[0].specificity,
            Specificity {
                ids: 0,
                classes: 0,
                types: 1
            }
        );
        for selector in &parsed.value.rules[2].selectors {
            assert_eq!(selector.specificity, Specificity::default());
        }
        let strict = parse_stylesheet(
            ".valid,:unsupported { color:red } div:not(.valid,:unsupported) { color:red }",
        );
        assert_eq!(strict.errors.len(), 2);
        assert!(strict.value.rules.is_empty());
    }

    #[test]
    fn filtered_nth_lists_are_strict_and_add_max_argument_specificity() {
        let parsed = parse_stylesheet(
            "span:nth-child(2n+1 of .pick, #hero), :nth-last-child(-n+2 of div > .pick) { color:red }",
        );
        assert!(parsed.errors.is_empty(), "{:?}", parsed.errors);
        let selectors = &parsed.value.rules[0].selectors;
        assert_eq!(
            selectors[0].specificity,
            Specificity {
                ids: 1,
                classes: 1,
                types: 1
            }
        );
        assert_eq!(
            selectors[1].specificity,
            Specificity {
                ids: 0,
                classes: 2,
                types: 1
            }
        );
        assert!(
            matches!(&selectors[1].compounds[0].simple[0], SimpleSelector::NthChild(nth)
            if nth.from_end && nth.expression == NthExpression { a:-1, b:2 } && nth.of.len() == 1)
        );

        for selector in [
            ":nth-child(3 of.target)",
            ":nth-child(3 of[target])",
            ":nth-last-child(3 of.target)",
        ] {
            let parsed = parse_stylesheet(&format!("{selector} {{ color:red }}"));
            assert!(parsed.errors.is_empty(), "{selector}: {:?}", parsed.errors);
            assert_eq!(parsed.value.rules.len(), 1, "{selector}");
        }

        for selector in [
            ":nth-child(1 of)",
            ":nth-child(1 of )",
            ":nth-child(1 of .pick,:unsupported)",
            ":nth-child(1 of ::before)",
            ":nth-last-child()",
        ] {
            let invalid = parse_stylesheet(&format!("{selector} {{ color:red }}"));
            assert!(invalid.value.rules.is_empty(), "{selector}");
            assert_eq!(invalid.errors.len(), 1, "{selector}");
        }
        let nested = format!(
            "{}span{} {{ color:red }}",
            ":is(".repeat(65),
            ")".repeat(65)
        );
        let over_budget = parse_stylesheet(&nested);
        assert!(over_budget.value.rules.is_empty());
        assert!(over_budget.errors[0].message.contains("64"));
    }

    #[test]
    fn anb_token_grammar_preserves_sign_and_whitespace_constraints() {
        for (source, a, b) in [
            ("odd", 2, 1),
            ("EVEN", 2, 0),
            ("+5", 0, 5),
            ("-3", 0, -3),
            ("2n + 1", 2, 1),
            ("2n-1", 2, -1),
            ("2n- 1", 2, -1),
            ("n", 1, 0),
            ("+n", 1, 0),
            ("-n + 3", -1, 3),
            ("+n-2", 1, -2),
            ("-n- 2", -1, -2),
            ("n +1", 1, 1),
            ("n - 1", 1, -1),
            ("0n+5", 0, 5),
            ("n-2147483648", 1, i32::MIN),
        ] {
            assert_eq!(
                parse_nth_expression(&tokenize(source).tokens),
                Some(NthExpression { a, b }),
                "{source}"
            );
        }
        for source in [
            "",
            "1 2",
            "2 n",
            "+ n",
            "n 2",
            "n + -1",
            "n+-1",
            "n- +1",
            "n- -1",
            "1.0n",
            "1e2n",
            "n + 1.0",
            "n2",
            "--n",
            "+-n",
            "n-2147483649",
            "2147483648n",
            "+ 1",
            "odd even",
        ] {
            assert_eq!(
                parse_nth_expression(&tokenize(source).tokens),
                None,
                "{source}"
            );
        }
    }

    #[test]
    fn typed_structural_pseudos_share_nth_grammar_without_filter_specificity() {
        let parsed = parse_stylesheet(
            "span:first-of-type:last-of-type:only-of-type, span:nth-of-type(2n+1), span:nth-last-of-type(-n+2) { color:red }",
        );
        assert!(parsed.errors.is_empty(), "{:?}", parsed.errors);
        let selectors = &parsed.value.rules[0].selectors;
        assert_eq!(
            selectors[0].specificity,
            Specificity {
                ids: 0,
                classes: 3,
                types: 1
            }
        );
        assert_eq!(
            selectors[1].specificity,
            Specificity {
                ids: 0,
                classes: 1,
                types: 1
            }
        );
        assert!(
            matches!(&selectors[1].compounds[0].simple[1], SimpleSelector::NthChild(nth)
            if nth.same_type && !nth.from_end && nth.of.is_empty())
        );
        assert!(
            matches!(&selectors[2].compounds[0].simple[1], SimpleSelector::NthChild(nth)
            if nth.same_type && nth.from_end)
        );
        for invalid in [
            ":nth-of-type(1 of .pick)",
            ":nth-last-of-type()",
            ":nth-of-type(2 n)",
        ] {
            let parsed = parse_stylesheet(&format!("{invalid} {{ color:red }}"));
            assert!(parsed.value.rules.is_empty(), "{invalid}");
            assert_eq!(parsed.errors.len(), 1);
        }
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
    fn unsupported_at_rules_are_ignored_without_poisoning_following_rules() {
        let parsed = parse_stylesheet("@media screen { p { color:red } } h1 { color: blue }");
        assert_eq!(parsed.value.rules.len(), 1);
        assert_eq!(parsed.errors.len(), 1);
        assert!(parsed.errors[0].message.contains("at-rule"));
    }

    #[test]
    fn supports_declaration_conditions_include_only_supported_nested_rules() {
        let parsed = parse_stylesheet(
            "@supports (color: ActiveBorder) { .yes { color: ActiveBorder } }              @supports (color: madeup-color) { .no { color: red } }              h1 { color: blue }",
        );
        assert_eq!(parsed.errors.len(), 0);
        assert_eq!(parsed.value.rules.len(), 2);
        assert_eq!(parsed.value.rules[0].selectors[0].specificity.classes, 1);
        assert_eq!(parsed.value.rules[1].selectors[0].specificity.types, 1);
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
