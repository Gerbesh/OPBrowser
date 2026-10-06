use op_html::{Doctype, Token, Tokenizer};

fn doctype(source: &str) -> Doctype {
    let tokens = Tokenizer::new(source).tokenize();
    assert_eq!(tokens.len(), 2, "{source}");
    assert_eq!(tokens[1], Token::Eof);
    match &tokens[0] {
        Token::Doctype(value) => value.clone(),
        token => panic!("expected doctype for {source}, got {token:?}"),
    }
}

#[test]
fn doctype_names_and_identifiers_preserve_missing_versus_empty_and_case() {
    assert_eq!(
        doctype("<!dOcTyPe HTML>"),
        Doctype {
            name: Some("html".into()),
            ..Doctype::default()
        }
    );
    assert_eq!(doctype("<!DOCTYPEhtml>"), doctype("<!doctype html>"));
    assert_eq!(
        doctype("<!DOCTYPE H\0TML PUBLIC'Pub&amp;\0'\"Sys\" >"),
        Doctype {
            name: Some("h\u{fffd}tml".into()),
            public_identifier: Some("Pub&amp;\u{fffd}".into()),
            system_identifier: Some("Sys".into()),
            force_quirks: false,
        }
    );
    assert_eq!(
        doctype("<!doctype html SYSTEM \"\">"),
        Doctype {
            name: Some("html".into()),
            system_identifier: Some(String::new()),
            ..Doctype::default()
        }
    );
    assert_eq!(
        doctype("<!doctype html public ''>"),
        Doctype {
            name: Some("html".into()),
            public_identifier: Some(String::new()),
            ..Doctype::default()
        }
    );
    assert_eq!(
        doctype("<!doctype h\u{b}Tml>").name.as_deref(),
        Some("h\u{b}tml")
    );
}

#[test]
fn malformed_doctype_recovery_sets_force_quirks_at_the_right_states() {
    for source in [
        "<!doctype>",
        "<!doctype",
        "<!doctype html",
        "<!doctype html >",
        "<!doctype html PUBLIC>",
        "<!doctype html SYSTEM>",
        "<!doctype html PUBLIC nope>",
        "<!doctype html public 'p' nope>",
        "<!doctype html system 's>",
        "<!doctype html public 'p>",
        "<!doctype html junk>",
        "<!doctype html system 's'",
        "<!doctype html public 'p'",
    ] {
        if source == "<!doctype html >" {
            assert!(!doctype(source).force_quirks);
        } else {
            assert!(doctype(source).force_quirks, "{source}");
        }
    }
    for source in [
        "<!doctype html system 's' junk>",
        "<!doctype html system 's' junk",
    ] {
        assert!(!doctype(source).force_quirks, "{source}");
    }
    assert_eq!(doctype("<!doctype>").name, None);
    assert_eq!(doctype("<!doctype html PUBLIC>").public_identifier, None);
    assert_eq!(
        doctype("<!doctype html system 's>")
            .system_identifier
            .as_deref(),
        Some("s")
    );
}

#[test]
fn bogus_declarations_are_comments_and_closing_resumes_normal_html() {
    for (source, data) in [
        ("<!unknown &amp;\0>", "unknown &amp;\u{fffd}"),
        ("<![CDATA[text]]>", "[CDATA[text]]"),
        ("<!DOCTYP html>", "DOCTYP html"),
        ("<!>", ""),
        ("<!unfinished", "unfinished"),
    ] {
        assert_eq!(
            Tokenizer::new(source).tokenize(),
            vec![Token::Comment(data.into()), Token::Eof]
        );
    }
    let tokens = Tokenizer::new("<!doctype html public 'ab>TAIL<p>x</p>").tokenize();
    assert!(
        matches!(&tokens[0], Token::Doctype(value) if value.force_quirks && value.public_identifier.as_deref() == Some("ab"))
    );
    assert_eq!(
        &tokens[1..5],
        &[
            Token::Character('T'),
            Token::Character('A'),
            Token::Character('I'),
            Token::Character('L')
        ]
    );
    assert!(matches!(&tokens[5], Token::StartTag { name, .. } if name == "p"));
}

#[test]
fn declarations_in_raw_text_rcdata_and_attributes_remain_literal() {
    for tag in ["script", "style", "title", "textarea"] {
        let tokens = Tokenizer::new(&format!("<{tag}><!doctype html><!bogus></{tag}>")).tokenize();
        let text: String = tokens
            .iter()
            .filter_map(|token| match token {
                Token::Character(ch) => Some(*ch),
                _ => None,
            })
            .collect();
        assert_eq!(text, "<!doctype html><!bogus>");
        assert!(
            !tokens
                .iter()
                .any(|token| matches!(token, Token::Comment(_) | Token::Doctype(_)))
        );
    }
    assert!(
        matches!(&Tokenizer::new("<p title='<!doctype html>'>").tokenize()[0], Token::StartTag { attributes, .. } if attributes[0].value == "<!doctype html>")
    );
}
