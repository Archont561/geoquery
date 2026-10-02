//! The N-API adapter, tested as the transport it forwards to.
//!
//! What is worth asserting about a crate this thin is that it forwards *verbatim*. The
//! adapter's only decision is to hand the string across untouched, and the two ways it could
//! stop doing that — altering the text, or interpreting it — both show up here as a response
//! that differs from the engine's.
//!
//! This is a Rust test rather than a JavaScript one on purpose: it covers the same line
//! `src/lib.rs` contains without needing Node to load a cdylib. What it cannot cover is
//! whether Node's `require` can load the artifact at all, which is what
//! `packages/client/test/native.test.ts` is for. Neither test replaces the other.

use serde_json::{Value, json};

/// Send a request through the adapter and read the response back.
fn invoke(request: Value) -> Value {
    let text = serde_json::to_string(&request).expect("a request serializes");
    serde_json::from_str(&geoquery_node_native::invoke(text)).expect("a response deserializes")
}

#[test]
fn a_request_reaches_the_engine_and_its_answer_comes_back() {
    let response = invoke(json!({ "transportVersion": 1, "operation": "ping", "payload": {} }));

    assert_eq!(response["ok"], json!(true));
    assert_eq!(response["result"]["echo"], json!({}));
}

#[test]
fn the_adapter_does_not_reinterpret_a_failed_response() {
    // The adapter has no error path of its own, and that is the property: a failure is a
    // response like any other, carrying the engine's wording. A binding that threw here would
    // have to restate the error to throw it, and every such restatement is a second place
    // the wording can drift.
    let response = invoke(json!({
        "transportVersion": 1,
        "operation": "parseDocument",
        "payload": { "document": "[]" },
    }));

    assert_eq!(response["ok"], json!(false));
    assert!(
        response["result"]["error"]
            .as_str()
            .expect("an error is a string")
            .contains("not a query document")
    );
}

#[test]
fn the_payload_is_not_rewritten_on_the_way_through() {
    // Non-ASCII text is the cheapest way to catch an adapter that round-tripped through a
    // different encoding, since it would arrive as escapes in one direction and literals in
    // the other — a difference invisible to an ASCII-only test.
    let response = invoke(json!({
        "transportVersion": 1,
        "operation": "ping",
        "payload": { "message": "naïve" },
    }));

    assert_eq!(response["result"]["echo"]["message"], json!("naïve"));
}

#[test]
fn the_returned_value_is_a_string_rather_than_a_parsed_object() {
    // The addon's signature is `String -> String`, and this asserts the other half of that:
    // there is nothing for a caller to catch except a string it has to decode itself. The
    // engine is what guarantees serialisability, and returning `EngineResponse` here would
    // have meant each binding decided how to cross the boundary on its own.
    let returned = geoquery_node_native::invoke(r#"{"transportVersion":1,"operation":"ping"}"#.to_string());

    assert!(returned.starts_with('{'), "the response is JSON text");
}