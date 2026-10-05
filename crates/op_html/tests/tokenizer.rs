use op_html::{Attribute, Token, Tokenizer};

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
