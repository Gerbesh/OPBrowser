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
