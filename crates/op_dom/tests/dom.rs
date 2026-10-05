use op_dom::{Document, NodeKind};

#[test]
fn builds_and_moves_dom_nodes() {
    let mut document = Document::new();
    let root = document.root();
    let html = document.create_element("html");
    let body = document.create_element("body");
    let text = document.create_text("hello");

    document.append_child(root, html).unwrap();
    document.append_child(html, body).unwrap();
    document.append_child(html, text).unwrap();
    document.append_child(body, text).unwrap();

    assert_eq!(document.node(text).unwrap().parent, Some(body));
    assert_eq!(document.node(body).unwrap().children, vec![text]);
    assert!(!document.node(html).unwrap().children.contains(&text));

    match &document.node(text).unwrap().kind {
        NodeKind::Text(value) => assert_eq!(value, "hello"),
        other => panic!("unexpected node kind: {other:?}"),
    }
}
