use op_dom::{Document, NodeId, NodeKind};
use op_html::parse_document;

fn child_element(document: &Document, parent: NodeId, name: &str) -> NodeId {
    document
        .children(parent)
        .iter()
        .copied()
        .find(|node| {
            document
                .element(*node)
                .is_some_and(|element| element.tag_name == name)
        })
        .unwrap_or_else(|| panic!("missing <{name}> child"))
}

fn html(document: &Document) -> NodeId {
    child_element(document, document.root(), "html")
}

fn head(document: &Document) -> NodeId {
    child_element(document, html(document), "head")
}

fn body(document: &Document) -> NodeId {
    child_element(document, html(document), "body")
}

#[test]
fn comments_become_dom_nodes_without_becoming_text_or_changing_open_elements() {
    let document = parse_document("<!--before--><p>a<!--</p><img src=x>-->b<!--unfinished");
    let root_children = document.children(document.root());
    assert_eq!(root_children.len(), 2);
    assert!(matches!(
        &document.node(root_children[0]).unwrap().kind,
        NodeKind::Comment(data) if data == "before"
    ));

    let paragraph = child_element(&document, body(&document), "p");
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
    assert_eq!(document.element(html).unwrap().tag_name, "html");
    let body = child_element(&document, html, "body");
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

    let html = html(&document);
    let html_children = document.children(html);
    assert_eq!(document.element(html_children[0]).unwrap().tag_name, "head");
    assert_eq!(document.element(html_children[1]).unwrap().tag_name, "body");

    let body = body(&document);
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
    let div = child_element(&document, body(&document), "div");
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
fn mismatched_end_tag_closes_matching_ancestor_and_keeps_tail_in_body() {
    let document = parse_document("<div><p>text</div>tail");
    let body = body(&document);
    let div = document.children(body)[0];
    let p = document.children(div)[0];

    match &document.node(document.children(p)[0]).unwrap().kind {
        NodeKind::Text(text) => assert_eq!(text, "text"),
        other => panic!("expected paragraph text, got {other:?}"),
    }

    match &document.node(document.children(body)[1]).unwrap().kind {
        NodeKind::Text(text) => assert_eq!(text, "tail"),
        other => panic!("expected body trailing text, got {other:?}"),
    }
}

#[test]
fn creates_missing_html_head_and_body_and_routes_head_content() {
    let document = parse_document(
        "<!doctype html><title>A &amp; B</title><meta charset=utf-8><link rel=stylesheet href=x><p id=p>Hello",
    );

    let html = html(&document);
    let head = head(&document);
    let body = body(&document);
    assert_eq!(document.children(html).len(), 2);

    let head_children = document.children(head);
    assert_eq!(head_children.len(), 3);
    let title = head_children[0];
    assert_eq!(document.element(title).unwrap().tag_name, "title");
    assert!(matches!(
        &document.node(document.children(title)[0]).unwrap().kind,
        NodeKind::Text(text) if text == "A & B"
    ));
    assert_eq!(document.element(head_children[1]).unwrap().tag_name, "meta");
    assert_eq!(document.element(head_children[2]).unwrap().tag_name, "link");

    let paragraph = document.children(body)[0];
    assert_eq!(document.element(paragraph).unwrap().tag_name, "p");
    assert_eq!(
        document.element(paragraph).unwrap().attributes[0].value,
        "p"
    );
}

#[test]
fn comments_follow_initial_before_head_in_head_and_after_head_locations() {
    let document = parse_document(
        "<!--document--><html><!--before-head--><head><!--in-head--></head><!--after-head--><body>x",
    );

    assert!(matches!(
        &document.node(document.children(document.root())[0]).unwrap().kind,
        NodeKind::Comment(data) if data == "document"
    ));

    let html = html(&document);
    let html_children = document.children(html);
    assert!(matches!(
        &document.node(html_children[0]).unwrap().kind,
        NodeKind::Comment(data) if data == "before-head"
    ));

    let head = head(&document);
    assert!(matches!(
        &document.node(document.children(head)[0]).unwrap().kind,
        NodeKind::Comment(data) if data == "in-head"
    ));

    assert!(html_children.iter().any(|node| {
        matches!(
            &document.node(*node).unwrap().kind,
            NodeKind::Comment(data) if data == "after-head"
        )
    }));
}

#[test]
fn head_only_tokens_after_head_are_inserted_back_into_head() {
    let document = parse_document(
        "<html><head></head><meta name=x><style>p { color: red }</style><body><p>x</p>",
    );
    let head = head(&document);
    let children = document.children(head);
    assert_eq!(document.element(children[0]).unwrap().tag_name, "meta");
    assert_eq!(document.element(children[1]).unwrap().tag_name, "style");
    assert!(matches!(
        &document.node(document.children(children[1])[0]).unwrap().kind,
        NodeKind::Text(text) if text == "p { color: red }"
    ));
    assert_eq!(
        document
            .element(document.children(body(&document))[0])
            .unwrap()
            .tag_name,
        "p"
    );
}

#[test]
fn duplicate_html_and_body_tokens_merge_new_attributes_without_replacing_existing_ones() {
    let document = parse_document(
        "<html lang=en><head></head><body class=first><html dir=rtl lang=fr><body id=main class=second><p>x",
    );
    let html = document.element(html(&document)).unwrap();
    assert!(
        html.attributes
            .iter()
            .any(|attribute| attribute.name == "lang" && attribute.value == "en")
    );
    assert!(
        html.attributes
            .iter()
            .any(|attribute| attribute.name == "dir" && attribute.value == "rtl")
    );
    assert!(
        !html
            .attributes
            .iter()
            .any(|attribute| attribute.name == "lang" && attribute.value == "fr")
    );

    let body = document.element(body(&document)).unwrap();
    assert!(
        body.attributes
            .iter()
            .any(|attribute| attribute.name == "class" && attribute.value == "first")
    );
    assert!(
        body.attributes
            .iter()
            .any(|attribute| attribute.name == "id" && attribute.value == "main")
    );
}

#[test]
fn self_closing_syntax_does_not_close_normal_html_elements() {
    let document = parse_document("<div/>inside<span/>nested</span>tail</div>");
    let div = child_element(&document, body(&document), "div");
    let children = document.children(div);
    assert!(matches!(
        &document.node(children[0]).unwrap().kind,
        NodeKind::Text(text) if text == "inside"
    ));
    assert_eq!(document.element(children[1]).unwrap().tag_name, "span");
    assert!(matches!(
        &document.node(children[2]).unwrap().kind,
        NodeKind::Text(text) if text == "tail"
    ));
}
