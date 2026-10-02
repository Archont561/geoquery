//! The dispatcher, tested through the one function a binding calls.
//!
//! Every assertion here parses the response back into [`EngineResponse`], because that is
//! what a binding does with it. Reaching for `Operation::Ping` and a private helper instead
//! would test this crate's internals, and the internals are one `match` away from the
//! behaviour that matters: what a caller in another language is told.

use geoquery_protocol::{EngineResponse, Operation, TRANSPORT_VERSION};
use serde_json::{Value, json};

/// Ask the engine to run `operation` with `payload`, and read the response back.
fn invoke(operation: Operation, payload: &Value) -> EngineResponse {
    let request = json!({
        "transportVersion": TRANSPORT_VERSION,
        "operation": operation,
        "payload": payload,
    });
    send(&request)
}

/// Send a request built by hand, for the cases a well-formed one cannot express: an
/// operation this build has no arm for, a transport version it does not speak.
fn send(request: &Value) -> EngineResponse {
    let text = serde_json::to_string(request).expect("a request serializes");
    serde_json::from_str(&geoquery_engine::invoke(&text)).expect("a response deserializes")
}

#[test]
fn ping_echoes_an_entire_payload_rather_than_one_field_of_it() {
    // The reason `ping` exists. A payload that survives the round trip whole — nested,
    // non-ASCII, and numeric — is the evidence that the boundary passes values rather than
    // patterns it happens to recognise. An addon that returned a single string would pass a
    // ping that only ever sent a string.
    let payload = json!({ "message": "héllo", "nested": { "n": [1, 2, 3] } });

    let response = invoke(Operation::Ping, &payload);

    assert!(response.ok);
    assert_eq!(response.result["engine"], "geoquery-engine");
    assert_eq!(response.result["echo"], payload);
}

#[test]
fn a_missing_payload_is_echoed_as_null_rather_than_failing() {
    // The counterpart to `default` on the request: an argument-less caller omits the field,
    // and the round trip still has to produce a response rather than an error.
    let response = send(&json!({
        "transportVersion": TRANSPORT_VERSION,
        "operation": "ping",
    }));

    assert!(response.ok);
    assert_eq!(response.result["echo"], Value::Null);
}

#[test]
fn protocol_version_reports_the_engine_and_the_query_protocol() {
    // Three numbers a caller can check itself: this build's version, the query protocol it
    // speaks, and the user-agent that combines them. The first is asserted for equality and
    // the third for containment, because the header's format is core's to change and this
    // test should not fail the day it does.
    let response = invoke(Operation::ProtocolVersion, &json!({}));

    assert!(response.ok);
    assert_eq!(response.result["version"], geoquery_core::VERSION);
    assert_eq!(
        response.result["protocolVersion"],
        geoquery_core::PROTOCOL_VERSION
    );
    assert!(
        response.result["userAgent"]
            .as_str()
            .expect("the user-agent is a string")
            .contains(geoquery_core::VERSION)
    );
}

#[test]
fn parse_document_sorts_the_keys_it_reports() {
    // Sorted rather than in document order, because a caller comparing two results, or
    // snapshotting them, should not see a difference that means nothing.
    let response = invoke(
        Operation::ParseDocument,
        &json!({ "document": r#"{"temporal": {}, "bbox": [], "spatial": []}"# }),
    );

    assert!(response.ok);
    assert_eq!(
        response.result["keys"],
        json!(["bbox", "spatial", "temporal"])
    );
}

#[test]
fn parse_document_reports_a_non_object_with_core_wording() {
    // The failure text comes from core and is not restated here. That is the point of the
    // assertion being a substring match: if a future core change words this differently, the
    // engine needs no edit, and this test fails loudly enough to make someone read why.
    let response = invoke(Operation::ParseDocument, &json!({ "document": "[]" }));

    assert!(!response.ok);
    assert!(
        response.result["error"]
            .as_str()
            .expect("an error is a string")
            .contains("not a query document"),
        "the engine reported {:?}",
        response.result["error"]
    );
}

#[test]
fn a_missing_document_argument_is_named_rather_than_swallowed() {
    // `parseDocument` with no `document` is a caller's mistake, and it is reported as one.
    // The alternative — treating it as an empty document — would return `keys: []`, which a
    // caller could not tell apart from a genuinely empty query.
    let response = invoke(Operation::ParseDocument, &json!({}));

    assert!(!response.ok);
    assert!(
        response.result["error"]
            .as_str()
            .expect("an error is a string")
            .contains("`document`")
    );
}

#[test]
fn an_unknown_operation_is_answered_in_rather_than_crashing() {
    // The dispatcher's own path, distinct from a request that fails to parse: the operation
    // name is not in `Operation`, so the envelope never decodes. A binding must get a
    // response it can show a user, not an exception across an FFI boundary.
    let response = send(&json!({
        "transportVersion": TRANSPORT_VERSION,
        "operation": "explain",
        "payload": {},
    }));

    assert!(!response.ok);
    assert!(
        response.result["error"]
            .as_str()
            .is_some_and(|e| e.contains("invalid request"))
    );
    // The parse failure is carried as text, because its type is serde's and not this
    // project's; a binding should not have to match on a Rust error variant.
    assert!(response.result["detail"].is_string());
}

#[test]
fn a_wrong_transport_version_is_refused_with_both_numbers() {
    // Both versions travel back so a caller can tell "you are too old for this engine" from
    // "you are too new for it" without a second round trip, and so the mismatch is visible
    // in a log rather than inferred from a refusal.
    let response = send(&json!({
        "transportVersion": 999,
        "operation": "ping",
        "payload": {},
    }));

    assert!(!response.ok);
    assert_eq!(response.result["supported"], TRANSPORT_VERSION);
    assert_eq!(response.result["received"], 999);
}

#[test]
fn an_answer_is_always_in_this_engines_dialect() {
    // Even to a question written in a version it does not speak. A binding parses the
    // response with the field names it knows; returning the caller's own version in the
    // response would tell it to parse a shape that was never used.
    let response = send(&json!({
        "transportVersion": 999,
        "operation": "ping",
        "payload": {},
    }));

    assert_eq!(response.transport_version, TRANSPORT_VERSION);
}

#[test]
fn malformed_json_is_answered_rather_than_reaching_the_caller_as_a_panic() {
    // Every path out of `invoke` is a response string. A binding calls it across an FFI
    // boundary where a Rust panic is not an error a caller can handle, so the guarantee is
    // worth pinning down rather than leaving to inspection.
    let response = geoquery_engine::invoke("{not json");

    let parsed: EngineResponse =
        serde_json::from_str(&response).expect("malformed input still answers");
    assert!(!parsed.ok);
}
