//! The `PyO3` adapter, tested as the transport it forwards to.
//!
//! What is worth asserting about a crate this thin is that it forwards *verbatim*. The
//! adapter's only decision is to hand the string across untouched, and the two ways it
//! could stop doing that — altering the text, or interpreting it — both show up here as a
//! response that differs from the engine's.

use serde_json::{Value, json};

/// Send a request through the adapter and read the response back.
fn invoke(request: &Value) -> Value {
    let text = serde_json::to_string(&request).expect("a request serializes");
    serde_json::from_str(&geoquery_python_native::invoke(&text)).expect("a response deserializes")
}

#[test]
fn a_request_reaches_the_engine_and_its_answer_comes_back() {
    let response = invoke(&json!({ "transportVersion": 1, "operation": "ping", "payload": {} }));

    assert_eq!(response["ok"], json!(true));
    assert_eq!(response["result"]["echo"], json!({}));
}

#[test]
fn the_adapter_does_not_reinterpret_a_failed_response() {
    // The adapter has no error path of its own, and that is the property: a failure is a
    // response like any other, carrying the engine's wording. A binding that raised here
    // would have to restate the error to raise it, and every such restatement is a second
    // place the wording can drift.
    let response = invoke(&json!({
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
    let response = invoke(&json!({
        "transportVersion": 1,
        "operation": "ping",
        "payload": { "message": "naïve" },
    }));

    assert_eq!(response["result"]["echo"]["message"], json!("naïve"));
}
