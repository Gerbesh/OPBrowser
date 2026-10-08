use op_dom::{Document, NodeId, NodeKind};
use op_html::parse_document;

#[test]
fn script_hook_sees_only_inserted_nodes_and_flushes_script_text() {
    let mut observed = 0;
    let document = op_html::parse_document_with_script_hook(
        "<!doctype html><p id='first'>Earlier</p><script>var x=1;</script><p id='later'>Later</p>",
        |document, script| {
            observed += 1;
            assert_eq!(document.element(script).unwrap().tag_name, "script");
            assert!(document.children(script).iter().any(|&node| {
                matches!(&document.node(node).unwrap().kind, NodeKind::Text(text) if text == "var x=1;")
            }));
            let ids: Vec<_> = (0..document.len())
                .filter_map(|index| document.node_id(index))
                .filter_map(|node| document.element(node))
                .flat_map(|element| element.attributes.iter())
                .filter(|attribute| attribute.name == "id")
                .map(|attribute| attribute.value.as_str())
                .collect();
            assert!(ids.contains(&"first"));
            assert!(!ids.contains(&"later"));
        },
    );
    assert_eq!(observed, 1);
    assert!(document.node_id(document.len() - 1).is_some());
    assert!(document.len() > 5);
}
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

#[test]
fn block_starts_close_open_paragraphs_and_stray_p_end_tags_create_empty_paragraphs() {
    let document = parse_document("<p>one<div>two</div>three<p>four<p>five");
    let body_node = body(&document);
    let children = document.children(body_node);

    assert_eq!(document.element(children[0]).unwrap().tag_name, "p");
    assert!(matches!(
        &document.node(document.children(children[0])[0]).unwrap().kind,
        NodeKind::Text(text) if text == "one"
    ));
    assert_eq!(document.element(children[1]).unwrap().tag_name, "div");
    assert!(matches!(
        &document.node(children[2]).unwrap().kind,
        NodeKind::Text(text) if text == "three"
    ));
    assert_eq!(document.element(children[3]).unwrap().tag_name, "p");
    assert_eq!(document.element(children[4]).unwrap().tag_name, "p");

    let document = parse_document("<div>a</p>b</div>");
    let div = child_element(&document, body(&document), "div");
    let children = document.children(div);
    assert!(matches!(
        &document.node(children[0]).unwrap().kind,
        NodeKind::Text(text) if text == "a"
    ));
    assert_eq!(document.element(children[1]).unwrap().tag_name, "p");
    assert!(document.children(children[1]).is_empty());
    assert!(matches!(
        &document.node(children[2]).unwrap().kind,
        NodeKind::Text(text) if text == "b"
    ));
}

#[test]
fn list_and_description_items_generate_implied_end_tags() {
    let document = parse_document("<ul><li>one<li>two<li><span>three</ul><dl><dt>A<dd>B<dt>C</dl>");
    let body = body(&document);
    let ul = document.children(body)[0];
    let list_items = document.children(ul);
    assert_eq!(list_items.len(), 3);
    assert!(
        list_items
            .iter()
            .all(|node| document.element(*node).unwrap().tag_name == "li")
    );
    assert!(matches!(
        &document.node(document.children(list_items[0])[0]).unwrap().kind,
        NodeKind::Text(text) if text == "one"
    ));
    assert!(matches!(
        &document.node(document.children(list_items[1])[0]).unwrap().kind,
        NodeKind::Text(text) if text == "two"
    ));
    assert_eq!(
        document
            .element(document.children(list_items[2])[0])
            .unwrap()
            .tag_name,
        "span"
    );

    let dl = document.children(body)[1];
    let description_items = document.children(dl);
    assert_eq!(description_items.len(), 3);
    assert_eq!(
        document.element(description_items[0]).unwrap().tag_name,
        "dt"
    );
    assert_eq!(
        document.element(description_items[1]).unwrap().tag_name,
        "dd"
    );
    assert_eq!(
        document.element(description_items[2]).unwrap().tag_name,
        "dt"
    );
}

#[test]
fn heading_start_and_end_rules_close_the_heading_in_scope() {
    let document = parse_document("<h1>one<h2>two</h3>tail");
    let body = body(&document);
    let children = document.children(body);

    assert_eq!(document.element(children[0]).unwrap().tag_name, "h1");
    assert_eq!(document.element(children[1]).unwrap().tag_name, "h2");
    assert!(matches!(
        &document.node(children[2]).unwrap().kind,
        NodeKind::Text(text) if text == "tail"
    ));
}

#[test]
fn nested_button_start_closes_the_button_already_in_scope() {
    let document = parse_document("<button>one<button>two</button>tail");
    let body = body(&document);
    let children = document.children(body);

    assert_eq!(document.element(children[0]).unwrap().tag_name, "button");
    assert_eq!(document.element(children[1]).unwrap().tag_name, "button");
    assert!(matches!(
        &document.node(children[2]).unwrap().kind,
        NodeKind::Text(text) if text == "tail"
    ));
}

#[test]
fn generic_end_tags_stop_at_special_elements_instead_of_crossing_scope_boundaries() {
    let document = parse_document("<div><span>a</unknown>b</span></div><span>c</span>");
    let body = body(&document);
    let div = document.children(body)[0];
    let span = document.children(div)[0];
    let span_children = document.children(span);

    assert!(matches!(
        &document.node(span_children[0]).unwrap().kind,
        NodeKind::Text(text) if text == "a"
    ));
    assert!(matches!(
        &document.node(span_children[1]).unwrap().kind,
        NodeKind::Text(text) if text == "b"
    ));
    assert_eq!(
        document
            .element(document.children(body)[1])
            .unwrap()
            .tag_name,
        "span"
    );
}

#[test]
fn head_tokens_seen_inside_body_return_to_head_without_breaking_body_stack() {
    let document = parse_document(
        "<body><div>before<style>.x{color:red}</style>after</div><meta name=x></body>",
    );
    let head = head(&document);
    let head_children = document.children(head);
    assert_eq!(
        document.element(head_children[0]).unwrap().tag_name,
        "style"
    );
    assert_eq!(document.element(head_children[1]).unwrap().tag_name, "meta");
    assert!(matches!(
        &document.node(document.children(head_children[0])[0]).unwrap().kind,
        NodeKind::Text(text) if text == ".x{color:red}"
    ));

    let div = child_element(&document, body(&document), "div");
    let div_children = document.children(div);
    assert!(matches!(
        &document.node(div_children[0]).unwrap().kind,
        NodeKind::Text(text) if text == "before"
    ));
    assert!(matches!(
        &document.node(div_children[1]).unwrap().kind,
        NodeKind::Text(text) if text == "after"
    ));
}

#[test]
fn legacy_image_alias_and_br_end_tag_follow_in_body_recovery() {
    let document = parse_document("<image src=x>one</br>two");
    let body = body(&document);
    let children = document.children(body);

    assert_eq!(document.element(children[0]).unwrap().tag_name, "img");
    assert!(matches!(
        &document.node(children[1]).unwrap().kind,
        NodeKind::Text(text) if text == "one"
    ));
    assert_eq!(document.element(children[2]).unwrap().tag_name, "br");
    assert!(matches!(
        &document.node(children[3]).unwrap().kind,
        NodeKind::Text(text) if text == "two"
    ));
}

#[test]
fn body_and_html_end_tags_use_after_body_modes_and_preserve_recovery_stack() {
    let document = parse_document(
        "<html><body><p>inside</p></body><!--after-body--><div>recovered</div></html><!--after-html-->",
    );

    let root = document.root();
    let root_children = document.children(root);
    assert_eq!(root_children.len(), 2);
    assert!(matches!(
        &document.node(root_children[1]).unwrap().kind,
        NodeKind::Comment(data) if data == "after-html"
    ));

    let html = html(&document);
    let html_children = document.children(html);
    assert!(matches!(
        &document.node(*html_children.last().unwrap()).unwrap().kind,
        NodeKind::Comment(data) if data == "after-body"
    ));

    let body = body(&document);
    let body_children = document.children(body);
    assert_eq!(document.element(body_children[0]).unwrap().tag_name, "p");
    assert_eq!(document.element(body_children[1]).unwrap().tag_name, "div");
    assert!(matches!(
        &document.node(document.children(body_children[1])[0]).unwrap().kind,
        NodeKind::Text(text) if text == "recovered"
    ));
}

#[test]
fn after_body_and_after_after_body_delegate_their_in_body_tokens() {
    let document = parse_document(
        "<html data-a=1><body>x</body> \n<html data-b=2></html> \t<!--document-tail-->",
    );

    let html_node = html(&document);
    let html_element = document.element(html_node).unwrap();
    assert!(
        html_element
            .attributes
            .iter()
            .any(|attribute| attribute.name == "data-a" && attribute.value == "1")
    );
    assert!(
        html_element
            .attributes
            .iter()
            .any(|attribute| attribute.name == "data-b" && attribute.value == "2")
    );

    let body_node = body(&document);
    let body_children = document.children(body_node);
    assert_eq!(body_children.len(), 3);
    assert!(matches!(
        &document.node(body_children[0]).unwrap().kind,
        NodeKind::Text(text) if text == "x"
    ));
    assert!(matches!(
        &document.node(body_children[1]).unwrap().kind,
        NodeKind::Text(text) if text == " \n"
    ));
    assert!(matches!(
        &document.node(body_children[2]).unwrap().kind,
        NodeKind::Text(text) if text == " \t"
    ));

    let root_children = document.children(document.root());
    assert!(matches!(
        &document.node(*root_children.last().unwrap()).unwrap().kind,
        NodeKind::Comment(data) if data == "document-tail"
    ));
}

#[test]
fn adoption_agency_reconstructs_misnested_inline_formatting() {
    let document = parse_document("<p>1<b>2<i>3</b>4</i>5");
    let paragraph = child_element(&document, body(&document), "p");
    let children = document.children(paragraph);

    assert_eq!(children.len(), 4);
    assert!(matches!(
        &document.node(children[0]).unwrap().kind,
        NodeKind::Text(text) if text == "1"
    ));

    let bold = children[1];
    assert_eq!(document.element(bold).unwrap().tag_name, "b");
    let bold_children = document.children(bold);
    assert_eq!(bold_children.len(), 2);
    assert!(matches!(
        &document.node(bold_children[0]).unwrap().kind,
        NodeKind::Text(text) if text == "2"
    ));
    let first_italic = bold_children[1];
    assert_eq!(document.element(first_italic).unwrap().tag_name, "i");
    assert!(matches!(
        &document.node(document.children(first_italic)[0]).unwrap().kind,
        NodeKind::Text(text) if text == "3"
    ));

    let reconstructed_italic = children[2];
    assert_eq!(
        document.element(reconstructed_italic).unwrap().tag_name,
        "i"
    );
    assert!(matches!(
        &document.node(document.children(reconstructed_italic)[0]).unwrap().kind,
        NodeKind::Text(text) if text == "4"
    ));
    assert!(matches!(
        &document.node(children[3]).unwrap().kind,
        NodeKind::Text(text) if text == "5"
    ));
}

#[test]
fn adoption_agency_reparents_a_furthest_block() {
    let document = parse_document("<b>1<p>2</b>3</p>");
    let body = body(&document);
    let children = document.children(body);

    assert_eq!(children.len(), 2);
    let original_bold = children[0];
    assert_eq!(document.element(original_bold).unwrap().tag_name, "b");
    assert!(matches!(
        &document.node(document.children(original_bold)[0]).unwrap().kind,
        NodeKind::Text(text) if text == "1"
    ));

    let paragraph = children[1];
    assert_eq!(document.element(paragraph).unwrap().tag_name, "p");
    let paragraph_children = document.children(paragraph);
    assert_eq!(paragraph_children.len(), 2);
    let adopted_bold = paragraph_children[0];
    assert_eq!(document.element(adopted_bold).unwrap().tag_name, "b");
    assert!(matches!(
        &document.node(document.children(adopted_bold)[0]).unwrap().kind,
        NodeKind::Text(text) if text == "2"
    ));
    assert!(matches!(
        &document.node(paragraph_children[1]).unwrap().kind,
        NodeKind::Text(text) if text == "3"
    ));
}

#[test]
fn repeated_anchor_start_tags_close_the_previous_active_anchor() {
    let document = parse_document("<a href=a>one<a href=b>two</a>three");
    let body = body(&document);
    let children = document.children(body);

    assert_eq!(children.len(), 3);
    let first = children[0];
    let second = children[1];
    assert_eq!(document.element(first).unwrap().tag_name, "a");
    assert_eq!(document.element(second).unwrap().tag_name, "a");
    assert_eq!(document.element(first).unwrap().attributes[0].value, "a");
    assert_eq!(document.element(second).unwrap().attributes[0].value, "b");
    assert!(matches!(
        &document.node(document.children(first)[0]).unwrap().kind,
        NodeKind::Text(text) if text == "one"
    ));
    assert!(matches!(
        &document.node(document.children(second)[0]).unwrap().kind,
        NodeKind::Text(text) if text == "two"
    ));
    assert!(matches!(
        &document.node(children[2]).unwrap().kind,
        NodeKind::Text(text) if text == "three"
    ));
}

#[test]
fn formatting_markers_prevent_inner_formatting_from_leaking_past_object() {
    let document = parse_document("<b>1<object><i>2</object>3</b>");
    let body = body(&document);
    let bold = document.children(body)[0];
    assert_eq!(document.element(bold).unwrap().tag_name, "b");

    let children = document.children(bold);
    assert_eq!(children.len(), 3);
    assert!(matches!(
        &document.node(children[0]).unwrap().kind,
        NodeKind::Text(text) if text == "1"
    ));

    let object = children[1];
    assert_eq!(document.element(object).unwrap().tag_name, "object");
    let italic = document.children(object)[0];
    assert_eq!(document.element(italic).unwrap().tag_name, "i");
    assert!(matches!(
        &document.node(document.children(italic)[0]).unwrap().kind,
        NodeKind::Text(text) if text == "2"
    ));

    assert!(matches!(
        &document.node(children[2]).unwrap().kind,
        NodeKind::Text(text) if text == "3"
    ));
}

#[test]
fn table_modes_create_implicit_tbody_and_close_cells_rows_and_sections() {
    let document = parse_document("<table><tr><td>A<td>B</table>tail");
    let body = body(&document);
    let body_children = document.children(body);

    assert_eq!(body_children.len(), 2);
    let table = body_children[0];
    assert_eq!(document.element(table).unwrap().tag_name, "table");

    let tbody = document.children(table)[0];
    assert_eq!(document.element(tbody).unwrap().tag_name, "tbody");
    let row = document.children(tbody)[0];
    assert_eq!(document.element(row).unwrap().tag_name, "tr");

    let cells = document.children(row);
    assert_eq!(cells.len(), 2);
    assert_eq!(document.element(cells[0]).unwrap().tag_name, "td");
    assert_eq!(document.element(cells[1]).unwrap().tag_name, "td");
    assert!(matches!(
        &document.node(document.children(cells[0])[0]).unwrap().kind,
        NodeKind::Text(text) if text == "A"
    ));
    assert!(matches!(
        &document.node(document.children(cells[1])[0]).unwrap().kind,
        NodeKind::Text(text) if text == "B"
    ));
    assert!(matches!(
        &document.node(body_children[1]).unwrap().kind,
        NodeKind::Text(text) if text == "tail"
    ));
}

#[test]
fn non_whitespace_table_text_is_foster_parented_before_the_table() {
    let document =
        parse_document("<p>before</p><table>alpha<tr><td>cell</td></tr>omega</table><p>after</p>");
    let body = body(&document);
    let children = document.children(body);

    assert_eq!(children.len(), 5);
    assert_eq!(document.element(children[0]).unwrap().tag_name, "p");
    assert!(matches!(
        &document.node(children[1]).unwrap().kind,
        NodeKind::Text(text) if text == "alpha"
    ));
    assert!(matches!(
        &document.node(children[2]).unwrap().kind,
        NodeKind::Text(text) if text == "omega"
    ));
    assert_eq!(document.element(children[3]).unwrap().tag_name, "table");
    assert_eq!(document.element(children[4]).unwrap().tag_name, "p");

    let row = document.children(document.children(children[3])[0])[0];
    let cell = document.children(row)[0];
    assert!(matches!(
        &document.node(document.children(cell)[0]).unwrap().kind,
        NodeKind::Text(text) if text == "cell"
    ));
}

#[test]
fn whitespace_table_text_stays_inside_the_table() {
    let document = parse_document("<table> \n<tr><td>x</td></tr></table>");
    let table = child_element(&document, body(&document), "table");
    let children = document.children(table);

    assert_eq!(children.len(), 2);
    assert!(matches!(
        &document.node(children[0]).unwrap().kind,
        NodeKind::Text(text) if text == " \n"
    ));
    assert_eq!(document.element(children[1]).unwrap().tag_name, "tbody");
}

#[test]
fn table_caption_colgroup_and_header_cells_follow_table_modes() {
    let document =
        parse_document("<table><caption>Cap</caption><col span=2><tr><th>H<td>D</table>");
    let table = child_element(&document, body(&document), "table");
    let children = document.children(table);

    assert_eq!(children.len(), 3);
    let caption = children[0];
    assert_eq!(document.element(caption).unwrap().tag_name, "caption");
    assert!(matches!(
        &document.node(document.children(caption)[0]).unwrap().kind,
        NodeKind::Text(text) if text == "Cap"
    ));

    let colgroup = children[1];
    assert_eq!(document.element(colgroup).unwrap().tag_name, "colgroup");
    let col = document.children(colgroup)[0];
    assert_eq!(document.element(col).unwrap().tag_name, "col");
    assert_eq!(document.element(col).unwrap().attributes[0].value, "2");

    let tbody = children[2];
    assert_eq!(document.element(tbody).unwrap().tag_name, "tbody");
    let row = document.children(tbody)[0];
    let cells = document.children(row);
    assert_eq!(document.element(cells[0]).unwrap().tag_name, "th");
    assert_eq!(document.element(cells[1]).unwrap().tag_name, "td");
}

#[test]
fn elements_misnested_directly_in_table_are_foster_parented_before_it() {
    let document = parse_document(
        "<div>before<table><span>outside</span><tr><td>cell</td></tr></table>after</div>",
    );
    let div = child_element(&document, body(&document), "div");
    let children = document.children(div);

    assert_eq!(children.len(), 4);
    assert!(matches!(
        &document.node(children[0]).unwrap().kind,
        NodeKind::Text(text) if text == "before"
    ));
    let span = children[1];
    assert_eq!(document.element(span).unwrap().tag_name, "span");
    assert!(matches!(
        &document.node(document.children(span)[0]).unwrap().kind,
        NodeKind::Text(text) if text == "outside"
    ));
    assert_eq!(document.element(children[2]).unwrap().tag_name, "table");
    assert!(matches!(
        &document.node(children[3]).unwrap().kind,
        NodeKind::Text(text) if text == "after"
    ));
}

#[test]
fn head_text_tokens_inside_table_return_to_table_mode() {
    let document = parse_document("<table><style>td{color:red}</style><tr><td>x</td></tr></table>");

    let head = head(&document);
    let style = child_element(&document, head, "style");
    assert!(matches!(
        &document.node(document.children(style)[0]).unwrap().kind,
        NodeKind::Text(text) if text == "td{color:red}"
    ));

    let table = child_element(&document, body(&document), "table");
    let tbody = child_element(&document, table, "tbody");
    let row = child_element(&document, tbody, "tr");
    let cell = child_element(&document, row, "td");
    assert!(matches!(
        &document.node(document.children(cell)[0]).unwrap().kind,
        NodeKind::Text(text) if text == "x"
    ));
}

#[test]
fn closing_a_table_cell_clears_its_formatting_marker_before_fostered_row_text() {
    let document = parse_document("<table><tr><td><b>x</td>y</tr></table>");
    let body = body(&document);
    let children = document.children(body);

    assert_eq!(children.len(), 2);
    assert!(matches!(
        &document.node(children[0]).unwrap().kind,
        NodeKind::Text(text) if text == "y"
    ));

    let table = children[1];
    assert_eq!(document.element(table).unwrap().tag_name, "table");
    let tbody = document.children(table)[0];
    let row = document.children(tbody)[0];
    let cell = document.children(row)[0];
    let bold = document.children(cell)[0];
    assert_eq!(document.element(bold).unwrap().tag_name, "b");
    assert!(matches!(
        &document.node(document.children(bold)[0]).unwrap().kind,
        NodeKind::Text(text) if text == "x"
    ));
}
