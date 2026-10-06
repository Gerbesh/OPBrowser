use op_dom::{Document, DocumentTypeData, NodeKind};

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

#[test]
fn insert_before_moves_nodes_and_preserves_sibling_order() {
    let mut document = Document::new();
    let root = document.root();
    let body = document.create_element("body");
    let table = document.create_element("table");
    let tail = document.create_text("tail");
    let fostered = document.create_text("fostered");

    document.append_child(root, body).unwrap();
    document.append_child(body, table).unwrap();
    document.append_child(body, tail).unwrap();
    document.insert_before(body, fostered, Some(table)).unwrap();

    assert_eq!(document.children(body), &[fostered, table, tail]);
    assert_eq!(document.node(fostered).unwrap().parent, Some(body));

    document.insert_before(body, tail, Some(table)).unwrap();
    assert_eq!(document.children(body), &[fostered, tail, table]);
}

#[test]
fn stores_comment_and_document_type_nodes_without_special_child_behavior() {
    let mut document = Document::new();
    let root = document.root();
    let comment = document.create_comment("license");
    let doctype = document.create_document_type(DocumentTypeData {
        name: Some("html".into()),
        public_identifier: Some("pub".into()),
        system_identifier: Some("sys".into()),
        force_quirks: true,
    });

    document.append_child(root, comment).unwrap();
    document.append_child(root, doctype).unwrap();

    assert!(matches!(
        &document.node(comment).unwrap().kind,
        NodeKind::Comment(data) if data == "license"
    ));
    assert_eq!(
        document.document_type(doctype),
        Some(&DocumentTypeData {
            name: Some("html".into()),
            public_identifier: Some("pub".into()),
            system_identifier: Some("sys".into()),
            force_quirks: true,
        })
    );
}
