//! End-to-end smoke + security tests against the public `render()` API.
//! Mirrors the pug-webassembly test style: each test is one spec + state
//! pair plus an expected output or expected error substring.

use json_render_wasm::render;

fn ok(spec: &str, state: &str) -> String {
    render(spec, state).expect("render should succeed")
}

fn err(spec: &str, state: &str) -> String {
    render(spec, state).expect_err("render should fail").to_string()
}

// ─── basics ────────────────────────────────────────────────────────────

#[test]
fn text_renders_literal() {
    let html = ok(r#"{"tag":"Text","props":{"value":"hello"}}"#, "");
    assert_eq!(html, "<span class=\"jr-text\">hello</span>");
}

#[test]
fn text_escapes_html() {
    let html = ok(
        r#"{"tag":"Text","props":{"value":"<script>x</script>"}}"#,
        "",
    );
    assert!(html.contains("&lt;script&gt;"));
    assert!(!html.contains("<script>"));
}

#[test]
fn heading_clamps_level() {
    let html = ok(
        r#"{"tag":"Heading","props":{"level":9,"value":"hi"}}"#,
        "",
    );
    assert!(html.starts_with("<h6"));
}

#[test]
fn container_renders_children() {
    let html = ok(
        r#"{"tag":"Container","children":[
            {"tag":"Heading","props":{"level":1,"value":"Title"}},
            {"tag":"Text","props":{"value":"body"}}
        ]}"#,
        "",
    );
    assert!(html.contains("<h1"));
    assert!(html.contains("body"));
}

#[test]
fn card_with_title_and_body() {
    let html = ok(
        r#"{"tag":"Card","props":{"title":"Greeting"},"children":[
            {"tag":"Text","props":{"value":"hello"}}
        ]}"#,
        "",
    );
    assert!(html.contains("jr-card-title"));
    assert!(html.contains("Greeting"));
    assert!(html.contains("hello"));
}

// ─── bindings ──────────────────────────────────────────────────────────

#[test]
fn bind_state_resolves() {
    let html = ok(
        r#"{"tag":"Text","props":{"value":"$bindState.user.name"}}"#,
        r#"{"user":{"name":"Ada"}}"#,
    );
    assert!(html.contains("Ada"));
}

#[test]
fn list_repeats_with_item_binding() {
    let html = ok(
        r#"{"tag":"List","props":{"items":"$bindState.users"},
           "children":[{"tag":"Text","props":{"value":"$item.name"}}]}"#,
        r#"{"users":[{"name":"a"},{"name":"b"},{"name":"c"}]}"#,
    );
    assert_eq!(html.matches("<li>").count(), 3);
    assert!(html.contains(">a<"));
    assert!(html.contains(">b<"));
    assert!(html.contains(">c<"));
}

#[test]
fn list_exposes_index() {
    let html = ok(
        r#"{"tag":"List","props":{"items":"$bindState.xs"},
           "children":[{"tag":"Text","props":{"value":"$index"}}]}"#,
        r#"{"xs":["x","x","x"]}"#,
    );
    assert!(html.contains(">0<"));
    assert!(html.contains(">2<"));
}

#[test]
fn table_renders_columns_and_rows() {
    let html = ok(
        r#"{"tag":"Table","props":{
            "columns":[
                {"header":"Name","field":"n"},
                {"header":"Age","field":"a"}
            ],
            "rows":"$bindState.people"
        }}"#,
        r#"{"people":[{"n":"Ada","a":36},{"n":"Bob","a":42}]}"#,
    );
    assert!(html.contains("<th>Name</th>"));
    assert!(html.contains("<th>Age</th>"));
    assert!(html.contains("<td>Ada</td>"));
    assert!(html.contains("<td>42</td>"));
}

// ─── security: unknown tags ────────────────────────────────────────────

#[test]
fn unknown_component_rejected() {
    let e = err(r#"{"tag":"script","props":{}}"#, "");
    assert!(e.contains("unknown component"));
}

#[test]
fn unknown_field_rejected() {
    let e = err(
        r#"{"tag":"Text","props":{"value":"x"},"onClick":"evil()"}"#,
        "",
    );
    assert!(e.contains("unknown field"));
}

// ─── security: binding sanitization ────────────────────────────────────

#[test]
fn binding_with_parens_rejected() {
    let e = err(
        r#"{"tag":"Text","props":{"value":"$bindState.foo()"}}"#,
        "",
    );
    assert!(e.contains("disallowed character") || e.contains("rejected"));
}

#[test]
fn binding_eval_rejected() {
    let e = err(
        r#"{"tag":"Text","props":{"value":"$bindState.eval"}}"#,
        "",
    );
    assert!(e.contains("disallowed token"));
}

#[test]
fn binding_proto_rejected() {
    let e = err(
        r#"{"tag":"Text","props":{"value":"$bindState.__proto__"}}"#,
        "",
    );
    assert!(e.contains("disallowed token") || e.contains("not allowed"));
}

#[test]
fn literal_dollar_amount_is_not_a_binding() {
    let html = ok(
        r#"{"tag":"Text","props":{"value":"$12,400"}}"#,
        "",
    );
    assert!(html.contains("$12,400"));
}

#[test]
fn literal_dollar_decimal_is_not_a_binding() {
    let html = ok(
        r#"{"tag":"Heading","props":{"level":3,"value":"$249.00"}}"#,
        "",
    );
    assert!(html.contains("$249.00"));
}

#[test]
fn binding_with_known_root_but_bad_path_rejected() {
    // `$bindState` is a recognized root, so this MUST parse as a binding —
    // and the parenthesis makes it a syntactically invalid one.
    let e = err(
        r#"{"tag":"Text","props":{"value":"$bindState.foo()"}}"#,
        "",
    );
    assert!(e.contains("disallowed") || e.contains("rejected"));
}

#[test]
fn string_starting_with_unknown_dollar_root_is_literal() {
    // Not one of the three known roots → treat as literal text.
    let html = ok(
        r#"{"tag":"Text","props":{"value":"$window.location"}}"#,
        "",
    );
    assert!(html.contains("$window.location"));
}

// ─── security: URL sanitization ────────────────────────────────────────

#[test]
fn link_javascript_url_neutralized() {
    let html = ok(
        r#"{"tag":"Link","props":{"href":"javascript:alert(1)","value":"x"}}"#,
        "",
    );
    assert!(html.contains("href=\"#\""));
    assert!(!html.to_ascii_lowercase().contains("javascript:"));
}

#[test]
fn image_data_url_neutralized() {
    let html = ok(
        r#"{"tag":"Image","props":{"src":"data:text/html,<script>","alt":""}}"#,
        "",
    );
    assert!(html.contains("src=\"#\""));
}

// ─── security: limits ──────────────────────────────────────────────────

#[test]
fn children_on_leaf_rejected() {
    let e = err(
        r#"{"tag":"Text","children":[{"tag":"Text","props":{"value":"x"}}]}"#,
        "",
    );
    assert!(e.contains("does not accept children"));
}
