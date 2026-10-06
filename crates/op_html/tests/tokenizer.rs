use op_html::{Attribute, Token, Tokenizer};

#[test]
fn comments_preserve_data_without_parsing_markup_or_references() {
    assert_eq!(
        Tokenizer::new("a<!-- <p>&amp;\0</p> -->b").tokenize(),
        vec![
            Token::Character('a'),
            Token::Comment(" <p>&amp;\u{fffd}</p> ".into()),
            Token::Character('b'),
            Token::Eof
        ]
    );
}

#[test]
fn comment_closing_and_eof_recovery_follow_pending_punctuation_states() {
    for (source, expected) in [
        ("<!-->", ""),
        ("<!--->", ""),
        ("<!---->", ""),
        ("<!--x--!>", "x"),
        ("<!--x---!>", "x-"),
        ("<!--", ""),
        ("<!---", ""),
        ("<!--x-", "x"),
        ("<!--x--", "x"),
        ("<!--x--!", "x"),
        ("<!--x--!y-->", "x--!y"),
        ("<!--x--!->", "x--!->"),
        ("<!--x<!--y-->", "x<!--y"),
        ("<!--<!-->", "<!"),
        ("<!--<<<!!-->", "<<<!!"),
        ("<!--x<-y-->", "x<-y"),
    ] {
        assert_eq!(
            Tokenizer::new(source).tokenize(),
            vec![Token::Comment(expected.into()), Token::Eof],
            "{source}"
        );
    }
}

#[test]
fn comment_markers_in_text_modes_and_attributes_remain_literal() {
    for tag in ["script", "style", "title", "textarea"] {
        let tokens =
            Tokenizer::new(&format!("<{tag}><!--&amp;--></{tag}><!--hidden-->")).tokenize();
        let text: String = tokens
            .iter()
            .filter_map(|token| match token {
                Token::Character(ch) => Some(*ch),
                _ => None,
            })
            .collect();
        assert_eq!(
            text,
            if matches!(tag, "title" | "textarea") {
                "<!--&-->"
            } else {
                "<!--&amp;-->"
            }
        );
        assert_eq!(
            tokens
                .iter()
                .filter(|token| matches!(token, Token::Comment(_)))
                .count(),
            1
        );
    }
    let tokens = Tokenizer::new("<p title='<!--literal-->'>x</p>").tokenize();
    assert!(
        matches!(&tokens[0], Token::StartTag { attributes, .. } if attributes[0].value == "<!--literal-->")
    );
}

#[test]
fn tokenizes_basic_tags_text_and_attributes() {
    let tokens = Tokenizer::new("<DIV ID=\"main\" disabled data-n=42 />Hi</DIV>").tokenize();

    assert_eq!(
        tokens,
        vec![
            Token::StartTag {
                name: "div".into(),
                attributes: vec![
                    Attribute {
                        name: "id".into(),
                        value: "main".into(),
                    },
                    Attribute {
                        name: "disabled".into(),
                        value: String::new(),
                    },
                    Attribute {
                        name: "data-n".into(),
                        value: "42".into(),
                    },
                ],
                self_closing: true,
            },
            Token::Character('H'),
            Token::Character('i'),
            Token::EndTag { name: "div".into() },
            Token::Eof,
        ]
    );
}

#[test]
fn keeps_only_the_first_duplicate_attribute() {
    let tokens = Tokenizer::new("<div id=first ID=second>").tokenize();

    match &tokens[0] {
        Token::StartTag { attributes, .. } => {
            assert_eq!(
                attributes,
                &vec![Attribute {
                    name: "id".into(),
                    value: "first".into(),
                }]
            );
        }
        other => panic!("unexpected first token: {other:?}"),
    }
}

#[test]
fn treats_invalid_tag_open_as_text() {
    let tokens = Tokenizer::new("a <3 b").tokenize();

    assert_eq!(
        tokens,
        vec![
            Token::Character('a'),
            Token::Character(' '),
            Token::Character('<'),
            Token::Character('3'),
            Token::Character(' '),
            Token::Character('b'),
            Token::Eof,
        ]
    );
}

#[test]
fn decodes_text_and_attribute_references_without_reparsing_markup() {
    let tokens = Tokenizer::new(
        "<a href='/?a=1&amp;b=2' title='&quot;x&quot;'>&lt;b&gt;&#x41F;&#1088;&copy;&amp;lt;</a>",
    )
    .tokenize();
    let Token::StartTag { attributes, .. } = &tokens[0] else {
        panic!("expected anchor");
    };
    assert_eq!(attributes[0].value, "/?a=1&b=2");
    assert_eq!(attributes[1].value, "\"x\"");
    let text: String = tokens
        .iter()
        .filter_map(|token| {
            if let Token::Character(ch) = token {
                Some(*ch)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(text, "<b>Пр©&lt;");
    assert_eq!(
        tokens
            .iter()
            .filter(|token| matches!(token, Token::StartTag { .. }))
            .count(),
        1
    );
}

#[test]
fn preserves_raw_text_and_decodes_rcdata_only_in_text() {
    let tokens = Tokenizer::new(
        "<script>let x = '<b>&amp;</b>';</script><textarea>&amp;<b>text</b></textarea><p>after</p>",
    )
    .tokenize();
    let text: String = tokens
        .iter()
        .filter_map(|token| {
            if let Token::Character(ch) = token {
                Some(*ch)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(text, "let x = '<b>&amp;</b>';&<b>text</b>after");
    assert!(
        !tokens
            .iter()
            .any(|token| matches!(token, Token::StartTag { name, .. } if name == "b"))
    );
    assert!(
        tokens
            .iter()
            .any(|token| matches!(token, Token::StartTag { name, .. } if name == "p"))
    );
}
