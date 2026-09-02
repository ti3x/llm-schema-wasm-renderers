//! End-to-end tests for the pug → HTML renderer.

use pug_wasm::render;

fn assert_renders(src: &str, data: &str, expected: &str) {
    let got = render(src, data).expect("render failed");
    assert_eq!(
        got, expected,
        "\nfor source:\n{src}\nwith data:\n{data}\ngot:\n{got}\nexpected:\n{expected}\n"
    );
}

// ─── Basic tags ────────────────────────────────────────────────────────

#[test]
fn empty_tag() {
    assert_renders("p", "", "<p></p>");
}

#[test]
fn tag_with_text() {
    assert_renders("p hello", "", "<p>hello</p>");
}

#[test]
fn class_shorthand() {
    assert_renders("div.foo.bar", "", "<div class=\"foo bar\"></div>");
}

#[test]
fn id_shorthand() {
    assert_renders("h1#title hi", "", "<h1 id=\"title\">hi</h1>");
}

#[test]
fn class_id_combined() {
    assert_renders("a.btn#go.primary Go", "", "<a id=\"go\" class=\"btn primary\">Go</a>");
}

#[test]
fn dot_only_becomes_div() {
    assert_renders(".wrap inner", "", "<div class=\"wrap\">inner</div>");
}

// ─── Attributes ────────────────────────────────────────────────────────

#[test]
fn attr_simple() {
    assert_renders(r#"a(href="/about")"#, "", "<a href=\"/about\"></a>");
}

#[test]
fn attr_multiple() {
    assert_renders(
        r#"a(href="/", target="_blank")"#,
        "",
        "<a href=\"/\" target=\"_blank\"></a>",
    );
}

#[test]
fn attr_boolean_true() {
    assert_renders(r#"input(disabled)"#, "", "<input disabled>");
}

#[test]
fn attr_expression_eval() {
    assert_renders(
        r#"a(href=url)"#,
        r#"{"url": "https://example.com/"}"#,
        "<a href=\"https://example.com/\"></a>",
    );
}

#[test]
fn attr_escapes_quote() {
    assert_renders(
        r#"a(title=name)"#,
        r#"{"name": "She said \"hi\""}"#,
        "<a title=\"She said &quot;hi&quot;\"></a>",
    );
}

#[test]
fn class_attr_merges_with_shorthand() {
    assert_renders(
        r#"button.btn(class="primary")"#,
        "",
        "<button class=\"btn primary\"></button>",
    );
}

// ─── Interpolation ─────────────────────────────────────────────────────

#[test]
fn interp_escaped() {
    assert_renders(
        "p Hello #{name}",
        r#"{"name": "<world>"}"#,
        "<p>Hello &lt;world&gt;</p>",
    );
}

#[test]
fn interp_raw() {
    assert_renders(
        "p Hello !{html}",
        r#"{"html": "<b>world</b>"}"#,
        "<p>Hello <b>world</b></p>",
    );
}

#[test]
fn text_escapes_angle_brackets() {
    assert_renders(
        "p <hello>",
        "",
        "<p>&lt;hello&gt;</p>",
    );
}

#[test]
fn buffered_expr_escaped() {
    assert_renders(
        "p= title",
        r#"{"title": "<x>"}"#,
        "<p>&lt;x&gt;</p>",
    );
}

#[test]
fn buffered_expr_raw() {
    assert_renders(
        "p!= html",
        r#"{"html": "<b>x</b>"}"#,
        "<p><b>x</b></p>",
    );
}

// ─── Doctype + void elements ───────────────────────────────────────────

#[test]
fn doctype_html() {
    assert_renders("doctype html", "", "<!DOCTYPE html>");
}

#[test]
fn void_tag_renders_open_only() {
    assert_renders("br", "", "<br>");
}

#[test]
fn void_tag_self_closing_slash() {
    assert_renders("img(src=\"a.png\")", "", "<img src=\"a.png\">");
}

// ─── Nesting ───────────────────────────────────────────────────────────

#[test]
fn nested_tags() {
    let src = "ul\n  li one\n  li two";
    assert_renders(src, "", "<ul><li>one</li><li>two</li></ul>");
}

#[test]
fn deeper_nesting() {
    let src = "div\n  p\n    span hi";
    assert_renders(src, "", "<div><p><span>hi</span></p></div>");
}

// ─── Conditionals ──────────────────────────────────────────────────────

#[test]
fn if_truthy() {
    let src = "if show\n  p yes\nelse\n  p no";
    assert_renders(src, r#"{"show": true}"#, "<p>yes</p>");
}

#[test]
fn if_falsy() {
    let src = "if show\n  p yes\nelse\n  p no";
    assert_renders(src, r#"{"show": false}"#, "<p>no</p>");
}

#[test]
fn unless_inverts() {
    let src = "unless show\n  p hidden";
    assert_renders(src, r#"{"show": false}"#, "<p>hidden</p>");
    assert_renders(src, r#"{"show": true}"#, "");
}

#[test]
fn else_if_chain() {
    let src = "if x == 1\n  p one\nelse if x == 2\n  p two\nelse\n  p other";
    assert_renders(src, r#"{"x": 2}"#, "<p>two</p>");
    assert_renders(src, r#"{"x": 9}"#, "<p>other</p>");
}

// ─── Iteration ─────────────────────────────────────────────────────────

#[test]
fn each_array() {
    let src = "ul\n  each item in items\n    li= item";
    assert_renders(
        src,
        r#"{"items": ["a", "b", "c"]}"#,
        "<ul><li>a</li><li>b</li><li>c</li></ul>",
    );
}

#[test]
fn each_with_index() {
    let src = "each item, i in items\n  p= i + \":\" + item";
    assert_renders(
        src,
        r#"{"items": ["a", "b"]}"#,
        "<p>0:a</p><p>1:b</p>",
    );
}

#[test]
fn each_object() {
    let src = "each v, k in obj\n  p= k + \"=\" + v";
    assert_renders(
        src,
        r#"{"obj": {"a": 1, "b": 2}}"#,
        "<p>a=1</p><p>b=2</p>",
    );
}

// ─── Code declarations ─────────────────────────────────────────────────

#[test]
fn var_declaration() {
    let src = "- var x = 42\np= x";
    assert_renders(src, "", "<p>42</p>");
}

#[test]
fn let_and_const_too() {
    assert_renders("- let x = 1\np= x", "", "<p>1</p>");
    assert_renders("- const x = 2\np= x", "", "<p>2</p>");
}

#[test]
fn arithmetic_in_expr() {
    let src = "p= (a + b) * 2";
    assert_renders(src, r#"{"a": 3, "b": 4}"#, "<p>14</p>");
}

#[test]
fn string_method_whitelist() {
    let src = "p= name.toUpperCase()";
    assert_renders(src, r#"{"name": "alice"}"#, "<p>ALICE</p>");
}

#[test]
fn array_length() {
    let src = "p= items.length";
    assert_renders(src, r#"{"items": [1,2,3]}"#, "<p>3</p>");
}

// ─── Comments ──────────────────────────────────────────────────────────

#[test]
fn silent_comment_omitted() {
    let src = "//- hidden\np shown";
    assert_renders(src, "", "<p>shown</p>");
}

#[test]
fn visible_comment_emits() {
    let src = "// a note\np text";
    let got = pug_wasm::render(src, "").unwrap();
    assert!(got.contains("<!--"));
    assert!(got.contains("a note"));
    assert!(got.ends_with("<p>text</p>"));
}

// ─── Block text (script tag) ───────────────────────────────────────────

#[test]
fn block_text() {
    let src = "p.\n  hello\n  world";
    assert_renders(src, "", "<p>hello\nworld</p>");
}

// ═══════════════════════════════════════════════════════════════════════
//  SECURITY TESTS — must all produce harmless output, never execute.
// ═══════════════════════════════════════════════════════════════════════

fn assert_rejected(src: &str, data: &str, fragment_of_err: &str) {
    let r = render(src, data);
    let err = r.expect_err(&format!(
        "expected error containing `{fragment_of_err}` but render succeeded for source:\n{src}"
    ));
    let msg = err.to_string();
    assert!(
        msg.contains(fragment_of_err),
        "expected error containing `{fragment_of_err}`, got: {msg}"
    );
}

#[test]
fn sec_blocks_arrow_function() {
    assert_rejected("p= (()=>1)()", "", "arrow function");
}

#[test]
fn sec_blocks_constructor_identifier() {
    assert_rejected(
        "p= constructor.constructor",
        "",
        "is not allowed",
    );
}

#[test]
fn sec_blocks_proto_identifier() {
    assert_rejected("p= __proto__", "", "is not allowed");
}

#[test]
fn sec_blocks_member_constructor() {
    // Identifier `obj` is fine; the `.constructor` access is blocked at eval.
    assert_rejected(
        "p= obj.constructor",
        r#"{"obj": {}}"#,
        "is not allowed",
    );
}

#[test]
fn sec_blocks_bare_function_call() {
    // `foo()` is bare function call syntax — disallowed; only `recv.method(...)` is.
    assert_rejected("p= foo()", "", "function calls are not allowed");
}

#[test]
fn sec_blocks_function_keyword() {
    assert_rejected("p= function x(){}", "", "is not allowed");
}

#[test]
fn sec_blocks_new_keyword() {
    assert_rejected("p= new Foo", "", "is not allowed");
}

#[test]
fn sec_blocks_eval_identifier() {
    assert_rejected("p= eval", "", "is not allowed");
}

#[test]
fn sec_blocks_this() {
    assert_rejected("p= this", "", "is not allowed");
}

#[test]
fn sec_blocks_assignment() {
    assert_rejected("- var x = 1\np= x = 2", "", "assignment");
}

#[test]
fn sec_blocks_index_constructor_string() {
    assert_rejected(
        "p= obj[\"constructor\"]",
        r#"{"obj": {}}"#,
        "is not allowed",
    );
}

#[test]
fn sec_method_not_in_whitelist_rejected() {
    // `.slice()` is not on the string whitelist.
    assert_rejected(
        "p= name.slice(0)",
        r#"{"name": "alice"}"#,
        "not allowed",
    );
}

#[test]
fn sec_script_tag_becomes_text_only() {
    // Pug allows the user to write a `script.` block; we faithfully emit it
    // as `<script>...</script>` in the OUTPUT. The web playground sandboxes
    // the preview iframe so it won't execute. The renderer's responsibility
    // is just: don't evaluate the contents as JS.
    let src = "script.\n  alert(document.cookie)";
    let got = render(src, "").expect("render should succeed");
    assert_eq!(got, "<script>alert(document.cookie)</script>");
}

#[test]
fn sec_iter_limit_aborts() {
    // Self-nested loops over the same large array would otherwise OOM/spin.
    // We rely on the iteration cap.
    let src = "each a in xs\n  each b in xs\n    p hi";
    // build a json array of 400 items → 400*400 = 160k > 100k cap
    let mut items = String::from("[");
    for i in 0..400 {
        if i > 0 { items.push(','); }
        items.push_str(&i.to_string());
    }
    items.push(']');
    let data = format!(r#"{{"xs": {items}}}"#);
    let err = render(src, &data).expect_err("should hit iteration cap");
    assert!(err.to_string().contains("iterations"));
}
