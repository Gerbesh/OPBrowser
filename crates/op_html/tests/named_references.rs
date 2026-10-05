use op_html::{Token, Tokenizer};

fn text(tokens: &[Token]) -> String {
    tokens
        .iter()
        .filter_map(|token| match token {
            Token::Character(ch) => Some(*ch),
            _ => None,
        })
        .collect()
}

fn source_entries() -> Vec<(&'static str, String)> {
    include_str!("../data/entities.tsv")
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| {
            let (name, codepoints) = line.split_once('\t').unwrap();
            let expected = codepoints
                .split_ascii_whitespace()
                .map(|cp| char::from_u32(u32::from_str_radix(cp, 16).unwrap()).unwrap())
                .collect();
            (name, expected)
        })
        .collect()
}

#[test]
fn decodes_every_standard_spelling_in_text_attributes_and_rcdata() {
    let entries = source_entries();
    assert_eq!(entries.len(), 2231);
    for (name, expected) in entries {
        // The independent source spellings include ';' and legacy aliases; the
        // runtime uses a separately generated packed table, not this fixture.
        assert_eq!(
            text(&Tokenizer::new(name).tokenize()),
            expected,
            "{name} at EOF"
        );
        assert_eq!(
            text(&Tokenizer::new(&format!("{name}!")).tokenize()),
            format!("{expected}!"),
            "{name} before punctuation"
        );
        for quote in ["'", "\"", ""] {
            let tokens = Tokenizer::new(&format!("<p title={quote}{name}!{quote}>")).tokenize();
            let Token::StartTag { attributes, .. } = &tokens[0] else {
                panic!("expected start tag for {name}");
            };
            assert_eq!(
                attributes[0].value,
                format!("{expected}!"),
                "{name} in {quote:?} attribute"
            );
        }
        for tag in ["title", "textarea"] {
            let tokens = Tokenizer::new(&format!("<{tag}>{name}!</{tag}>after")).tokenize();
            assert_eq!(
                text(&tokens),
                format!("{expected}!after"),
                "{name} in {tag}"
            );
        }
    }
}

#[test]
fn follows_legacy_attribute_ambiguity_and_longest_match_for_all_names() {
    let entries = source_entries();
    for (name, _) in &entries {
        // Removing the semicolon and adding a non-matching suffix exercises
        // legacy fallback inside a longer canonical prefix (e.g. notin -> not).
        for input in [
            format!("{}x;", name.trim_end_matches(';')),
            format!("{name}="),
        ] {
            let longest = entries
                .iter()
                .filter(|(candidate, _)| input.starts_with(candidate))
                .max_by_key(|(candidate, _)| candidate.len());
            let expected_text = longest.map_or_else(
                || input.clone(),
                |(matched, value)| format!("{value}{}", &input[matched.len()..]),
            );
            assert_eq!(
                text(&Tokenizer::new(&input).tokenize()),
                expected_text,
                "{input}"
            );
            let expected_attribute = longest.map_or_else(
                || input.clone(),
                |(matched, value)| {
                    let ambiguous = !matched.ends_with(';')
                        && input[matched.len()..]
                            .chars()
                            .next()
                            .is_some_and(|ch| ch.is_ascii_alphanumeric() || ch == '=');
                    if ambiguous {
                        input.clone()
                    } else {
                        format!("{value}{}", &input[matched.len()..])
                    }
                },
            );
            let tokens = Tokenizer::new(&format!("<a href='{input}'>")).tokenize();
            let Token::StartTag { attributes, .. } = &tokens[0] else {
                panic!("expected anchor for {input}");
            };
            assert_eq!(attributes[0].value, expected_attribute, "{input} in href");
        }
    }
}

#[test]
fn emits_multiscalar_references_without_reparsing_output_or_raw_text() {
    let tokens = Tokenizer::new(
        "<p title='&NotEqualTilde;&fjlig;'>&NotEqualTilde;&fjlig;&ThickSpace;&lt;em&gt;&amp;Aacute;</p><script>&NotEqualTilde;</script><style>&rarr;</style>",
    )
    .tokenize();
    assert_eq!(
        text(&tokens),
        "\u{2242}\u{338}fj\u{205f}\u{200a}<em>&Aacute;&NotEqualTilde;&rarr;"
    );
    let Token::StartTag { attributes, .. } = &tokens[0] else {
        panic!("expected paragraph");
    };
    assert_eq!(attributes[0].value, "\u{2242}\u{338}fj");
    assert_eq!(
        tokens
            .iter()
            .filter(|t| matches!(t, Token::StartTag { .. }))
            .count(),
        3
    );
}
