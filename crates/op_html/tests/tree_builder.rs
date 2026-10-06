use op_dom::NodeKind;
use op_html::parse_document;

#[test]
fn comments_become_dom_nodes_without_becoming_text_or_changing_open_elements() {
    let document = parse_document("<!--before--><p>a<!--</p><img src=x>-->b<!--unfinished");
    let children = document.children(document.root());
    assert_eq!(children.len(), 2);
    assert!(matches!(
        &document.node(children[0]).unwrap().kind,
        NodeKind::Comment(data) if data == "before"
    ));
    let paragraph = children[1];
    assert_eq!(document.element(paragraph).unwrap().tag_name, "p");
    let children = document.children(paragraph);
    assert_eq!(children.len(), 4);
    assert!(
        matches!(&document.node(children[0]).unwrap().kind, NodeKind::Text(text) if text == "a")
    );
    assert!(
        matches!(&document.node(children[1]).unwrap().kind, NodeKind::Comment(data) if data == "</p><img src=x>")
    );
    assert!(
        matches!(&document.node(children[2]).unwrap().kind, NodeKind::Text(text) if text == "b")
    );
    assert!(
        matches!(&document.node(children[3]).unwrap().kind, NodeKind::Comment(data) if data == "unfinished")
    );
}

#[test]
fn keeps_the_initial_doctype_and_ignores_late_doctypes() {
    let document = parse_document(
        "<!--before--><!doctype HTML PUBLIC 'pub' 'sys'><html><body>x<!doctype bogus><!--inside--></body></html><!doctype late><!--after-->",
    );
    let root = document.root();
    let children = document.children(root);
    assert_eq!(children.len(), 4);
    assert!(matches!(
        &document.node(children[0]).unwrap().kind,
        NodeKind::Comment(data) if data == "before"
    ));
    let doctype = document.document_type(children[1]).unwrap();
    assert_eq!(doctype.name.as_deref(), Some("html"));
    assert_eq!(doctype.public_identifier.as_deref(), Some("pub"));
    assert_eq!(doctype.system_identifier.as_deref(), Some("sys"));
    assert!(!doctype.force_quirks);

    let html = children[2];
    let body = document.children(html)[0];
    let body_children = document.children(body);
    assert_eq!(body_children.len(), 2);
    assert!(matches!(
        &document.node(body_children[0]).unwrap().kind,
        NodeKind::Text(text) if text == "x"
    ));
    assert!(matches!(
        &document.node(body_children[1]).unwrap().kind,
        NodeKind::Comment(data) if data == "inside"
    ));
    assert!(matches!(
        &document.node(children[3]).unwrap().kind,
        NodeKind::Comment(data) if data == "after"
    ));
}

#[test]
fn builds_nested_dom_and_preserves_attributes() {
    let document = parse_document(
        "<html><body><h1 id='title'>Hello <em>OP</em></h1><p data-x=42>World</p></body></html>",
    );

    let root_children = document.children(document.root());
    assert_eq!(root_children.len(), 1);

    let html = root_children[0];
    assert_eq!(document.element(html).unwrap().tag_name, "html");

    let body = document.children(html)[0];
    assert_eq!(document.element(body).unwrap().tag_name, "body");

    let h1 = document.children(body)[0];
    let h1_element = document.element(h1).unwrap();
    assert_eq!(h1_element.tag_name, "h1");
    assert_eq!(h1_element.attributes[0].name, "id");
    assert_eq!(h1_element.attributes[0].value, "title");

    let h1_children = document.children(h1);
    assert_eq!(h1_children.len(), 2);

    match &document.node(h1_children[0]).unwrap().kind {
        NodeKind::Text(text) => assert_eq!(text, "Hello "),
        other => panic!("expected text node, got {other:?}"),
    }

    let em = h1_children[1];
    assert_eq!(document.element(em).unwrap().tag_name, "em");

    let p = document.children(body)[1];
    assert_eq!(document.element(p).unwrap().attributes[0].value, "42");
}

#[test]
fn void_elements_do_not_capture_following_content() {
    let document = parse_document("<div>before<br>after<img src=x>end</div>");
    let div = document.children(document.root())[0];
    let children = document.children(div);

    assert_eq!(children.len(), 5);
    assert_eq!(document.element(children[1]).unwrap().tag_name, "br");
    assert_eq!(document.element(children[3]).unwrap().tag_name, "img");

    match &document.node(children[4]).unwrap().kind {
        NodeKind::Text(text) => assert_eq!(text, "end"),
        other => panic!("expected trailing text, got {other:?}"),
    }
}

#[test]
fn mismatched_end_tag_closes_matching_ancestor() {
    let document = parse_document("<div><p>text</div>tail");
    let div = document.children(document.root())[0];
    let p = document.children(div)[0];

    match &document.node(document.children(p)[0]).unwrap().kind {
        NodeKind::Text(text) => assert_eq!(text, "text"),
        other => panic!("expected paragraph text, got {other:?}"),
    }

    match &document
        .node(document.children(document.root())[1])
        .unwrap()
        .kind
    {
        NodeKind::Text(text) => assert_eq!(text, "tail"),
        other => panic!("expected root-level trailing text, got {other:?}"),
    }
}
