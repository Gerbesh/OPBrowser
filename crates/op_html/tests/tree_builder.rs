use op_dom::NodeKind;
use op_html::parse_document;

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
