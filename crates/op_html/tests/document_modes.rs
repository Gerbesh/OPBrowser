use op_dom::{DocumentMode, NodeKind};
use op_html::parse_document;

fn mode(source: &str) -> DocumentMode {
    parse_document(source).mode()
}

#[test]
fn modern_doctype_selects_no_quirks_and_initial_whitespace_is_ignored() {
    let document = parse_document(" \n\t<!--license--><!DOCTYPE html><html><body>x</body></html>");
    assert_eq!(document.mode(), DocumentMode::NoQuirks);

    let root_children = document.children(document.root());
    assert_eq!(root_children.len(), 3);
    assert!(matches!(
        &document.node(root_children[0]).unwrap().kind,
        NodeKind::Comment(data) if data == "license"
    ));
    assert!(document.document_type(root_children[1]).is_some());
    assert_eq!(document.element(root_children[2]).unwrap().tag_name, "html");
}

#[test]
fn missing_or_late_doctype_selects_quirks_and_late_doctype_is_not_stored() {
    assert_eq!(mode("<html><body>x</body></html>"), DocumentMode::Quirks);
    assert_eq!(mode(""), DocumentMode::Quirks);

    let document = parse_document("x<!doctype html><html></html>");
    assert_eq!(document.mode(), DocumentMode::Quirks);
    assert!(
        document
            .children(document.root())
            .iter()
            .all(|node| document.document_type(*node).is_none())
    );
}

#[test]
fn malformed_and_wrong_name_doctypes_select_quirks() {
    for source in [
        "<!doctype><html></html>",
        "<!doctype svg><html></html>",
        "<!doctype html PUBLIC nope><html></html>",
        "<!doctype html SYSTEM 'broken><html></html>",
    ] {
        assert_eq!(mode(source), DocumentMode::Quirks, "{source}");
    }
}

#[test]
fn legacy_html401_and_xhtml_doctypes_select_the_specified_modes() {
    assert_eq!(
        mode("<!DOCTYPE html PUBLIC '-//W3C//DTD HTML 4.01 Transitional//EN'><html></html>"),
        DocumentMode::Quirks
    );
    assert_eq!(
        mode(
            "<!DOCTYPE html PUBLIC '-//W3C//DTD HTML 4.01 Transitional//EN' 'legacy.dtd'><html></html>"
        ),
        DocumentMode::LimitedQuirks
    );
    assert_eq!(
        mode("<!DOCTYPE html PUBLIC '-//W3C//DTD XHTML 1.0 Transitional//EN'><html></html>"),
        DocumentMode::LimitedQuirks
    );
}

#[test]
fn legacy_matching_is_ascii_case_insensitive() {
    assert_eq!(
        mode("<!DOCTYPE html PUBLIC '-//w3c//dtd xhtml 1.0 transitional//en'><html></html>"),
        DocumentMode::LimitedQuirks
    );
    assert_eq!(
        mode(
            "<!DOCTYPE html PUBLIC '-//microsoft//dtd internet explorer 3.0 tables//en'><html></html>"
        ),
        DocumentMode::Quirks
    );
}
