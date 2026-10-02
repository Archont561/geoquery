//! The wire shapes, tested as bytes.
//!
//! Asserted through serialised JSON rather than by constructing the structs, because the
//! thing a binding depends on is the field *names* on the wire — `transportVersion` and not
//! `transport_version` — and those are decided by the serde attributes rather than by the
//! struct definitions. A test that builds an `EngineRequest` and compares it to another
//! would pass if every attribute on this crate were deleted.

use geoquery_protocol::{EngineRequest, EngineResponse, Operation, TRANSPORT_VERSION};

#[test]
fn an_operation_is_a_camel_cased_string_on_the_wire() {
    let request = EngineRequest {
        transport_version: TRANSPORT_VERSION,
        operation: Operation::ParseDocument,
        payload: serde_json::json!({}),
    };

    let text = serde_json::to_string(&request).expect("a request serializes");

    assert_eq!(
        text,
        r#"{"transportVersion":1,"operation":"parseDocument","payload":{}}"#
    );
}

#[test]
fn a_response_is_camel_cased_on_the_wire_too() {
    let response = EngineResponse {
        transport_version: TRANSPORT_VERSION,
        ok: true,
        result: serde_json::json!({ "keys": [] }),
    };

    let text = serde_json::to_string(&response).expect("a response serializes");

    assert_eq!(
        text,
        r#"{"transportVersion":1,"ok":true,"result":{"keys":[]}}"#
    );
}

#[test]
fn an_omitted_payload_decodes_as_null_and_re_encodes_as_absent() {
    // `default` rather than `required`: a binding that sends `{transportVersion, operation}`
    // for an operation that takes no arguments is not making a mistake, and rejecting it
    // would make every argument-less call a special case in every language.
    let request: EngineRequest =
        serde_json::from_str(r#"{"transportVersion":1,"operation":"protocolVersion"}"#)
            .expect("a payload may be omitted");

    assert_eq!(request.payload, serde_json::Value::Null);
}

#[test]
fn the_operation_set_is_closed() {
    // The assertion behind the enum's shape: an operation this build has never heard of does
    // not decode. Without it, a future operation would arrive as a request this engine could
    // not serve and had no way to refuse.
    let decoded = serde_json::from_str::<EngineRequest>(
        r#"{"transportVersion":1,"operation":"explain","payload":{}}"#,
    );

    assert!(decoded.is_err(), "an unknown operation must not decode");
}
