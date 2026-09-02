//! End-to-end tests: round-trip JSON → TOON → JSON, plus delimiter,
//! strict, and error-mapping coverage.

use serde_json::{json, Value};
use toon_wasm::options::{ConvOptions, Delimiter};
use toon_wasm::{json_to_toon, toon_to_json};

fn opts() -> ConvOptions {
    ConvOptions::default()
}

fn round_trip(value: Value) {
    let json_in = serde_json::to_string(&value).unwrap();
    let toon = json_to_toon(&json_in, &opts()).expect("encode");
    let json_out = toon_to_json(&toon, &opts()).expect("decode");
    let parsed: Value = serde_json::from_str(&json_out).unwrap();
    assert_eq!(value, parsed, "round trip lost data\nTOON was:\n{toon}");
}

// ─── round-trip shapes ─────────────────────────────────────────────────

#[test]
fn round_trip_object() {
    round_trip(json!({ "name": "Ada", "age": 36 }));
}

#[test]
fn round_trip_nested() {
    round_trip(json!({
        "user": { "id": 1, "name": "Ada", "active": true },
        "tags": ["admin", "ops"]
    }));
}

#[test]
fn round_trip_tabular_array() {
    round_trip(json!({
        "users": [
            { "id": 1, "name": "Ada",   "role": "admin" },
            { "id": 2, "name": "Bob",   "role": "user"  },
            { "id": 3, "name": "Grace", "role": "user"  }
        ]
    }));
}

#[test]
fn round_trip_mixed_array() {
    round_trip(json!({
        "items": [1, "two", { "k": "v" }, [10, 20]]
    }));
}

#[test]
fn round_trip_nulls_and_floats() {
    round_trip(json!({
        "a": null,
        "b": 1.5,
        "c": -7,
        "d": ""
    }));
}

// ─── tabular detection ─────────────────────────────────────────────────

#[test]
fn tabular_array_uses_header_row() {
    let toon = json_to_toon(
        r#"{"users":[{"id":1,"name":"a"},{"id":2,"name":"b"}]}"#,
        &opts(),
    )
    .unwrap();
    // Uniform array of objects with same keys should produce the
    // `users[2]{id,name}:` header form.
    assert!(toon.contains("users[2]{id,name}"), "got:\n{toon}");
    assert!(toon.contains("1,a"));
    assert!(toon.contains("2,b"));
}

// ─── delimiter switching ───────────────────────────────────────────────

#[test]
fn delimiter_pipe() {
    let mut o = opts();
    o.delimiter = Delimiter::Pipe;
    let toon = json_to_toon(r#"{"xs":[1,2,3]}"#, &o).unwrap();
    assert!(toon.contains('|'), "expected pipe delim in:\n{toon}");
}

#[test]
fn delimiter_round_trips_through_tab() {
    let mut o = opts();
    o.delimiter = Delimiter::Tab;
    let toon = json_to_toon(r#"{"xs":[1,2,3]}"#, &o).unwrap();
    let back = toon_to_json(&toon, &o).unwrap();
    let parsed: Value = serde_json::from_str(&back).unwrap();
    assert_eq!(parsed, json!({ "xs": [1, 2, 3] }));
}

// ─── strict mode ───────────────────────────────────────────────────────

#[test]
fn strict_rejects_wrong_length() {
    // Length marker says 3, only 2 items provided.
    let mut o = opts();
    o.strict = true;
    let bad = "items[3]: a,b\n";
    let e = toon_to_json(bad, &o).expect_err("strict should reject");
    let msg = e.to_string().to_lowercase();
    assert!(
        msg.contains("length") || msg.contains("mismatch") || msg.contains("expected"),
        "expected length error, got: {msg}"
    );
}

// ─── error mapping ─────────────────────────────────────────────────────

#[test]
fn malformed_json_input_returns_json_variant() {
    let e = json_to_toon("{ this is not json", &opts()).unwrap_err();
    assert!(matches!(e, toon_wasm::ConvError::Json(_)));
    assert!(e.to_string().contains("invalid JSON"));
}

#[test]
fn malformed_toon_input_returns_toon_variant() {
    let e = toon_to_json("[3]: a,b,c,d,e\n", &opts()).unwrap_err();
    // Anything that the decoder rejects ends up as Toon variant.
    assert!(matches!(e, toon_wasm::ConvError::Toon(_)));
}

#[test]
fn defaults_render_simple_object() {
    let toon = json_to_toon(r#"{"x":1}"#, &ConvOptions::default()).unwrap();
    assert!(toon.contains("x: 1"), "got:\n{toon}");
}
