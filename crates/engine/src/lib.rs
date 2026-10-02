//! The dispatcher behind [`invoke`].
//!
//! Everything a binding can ask the engine to do is one arm of one match, and each arm
//! calls into `geoquery-core` and hands back JSON. No query rule lives here: this crate
//! translates a wire request into a core call, and if a rule ever needs stating twice —
//! once here and once in core — it is stated in core and this arm reads it.

use geoquery_protocol::{EngineRequest, EngineResponse, Operation, TRANSPORT_VERSION};
use serde_json::{Value, json};

/// Run one request and return one response, both as JSON text.
///
/// A string in and a string out, rather than typed values, because that is the entire
/// contract with the bindings: they pass the text across an FFI boundary they do not share
/// types with. Returning `String` rather than `EngineResponse` is what forces the response
/// to be *serialisable* — a future field that cannot cross the wire fails in this
/// function's `expect` at the boundary, rather than in whichever binding happened to run
/// first.
///
/// # Panics
///
/// If an [`EngineResponse`] cannot be serialised, which cannot happen for the three value
/// shapes this crate constructs. The alternative is a `Result` every binding would have to
/// unwrap anyway, for a case that would itself be a bug in this file.
#[must_use]
pub fn invoke(request_json: &str) -> String {
    let response = match serde_json::from_str::<EngineRequest>(request_json) {
        Ok(request) if request.transport_version == TRANSPORT_VERSION => run(request),
        Ok(request) => failure(json!({
            "error": "unsupported transport version",
            "supported": TRANSPORT_VERSION,
            "received": request.transport_version,
        })),
        Err(error) => failure(json!({
            "error": "invalid request",
            "detail": error.to_string(),
        })),
    };
    serde_json::to_string(&response).expect("engine responses are serializable")
}

/// Dispatch one request whose transport version has already been accepted.
fn run(request: EngineRequest) -> EngineResponse {
    match request.operation {
        // The payload is echoed rather than interpreted, which is the whole test: a string
        // that survives the round trip is proof the boundary did not truncate, re-encode or
        // reinterpret it.
        Operation::Ping => success(json!({ "engine": "geoquery-engine", "echo": request.payload })),
        Operation::ProtocolVersion => success(json!({
            "version": geoquery_core::VERSION,
            "protocolVersion": geoquery_core::PROTOCOL_VERSION,
            "userAgent": geoquery_core::user_agent(),
        })),
        Operation::ParseDocument => parse_document(&request.payload),
    }
}

/// Parse the `document` field of a payload, or explain why it could not be read.
fn parse_document(payload: &Value) -> EngineResponse {
    // `as_str` rather than a serde struct: the payload is arbitrary JSON by design, and a
    // missing or non-string field is a caller's mistake to report, not a parse failure to
    // disguise as one.
    let Some(document) = payload.get("document").and_then(Value::as_str) else {
        return failure(json!({
            "error": "parseDocument needs a `document` string in the payload",
        }));
    };

    match geoquery_core::QueryDocument::parse(document) {
        Ok(parsed) => success(json!({ "keys": parsed.keys().collect::<Vec<_>>() })),
        // `to_string` and not a structured error: the error's classification is a core
        // decision, and duplicating its variants here would be the second interpretation
        // of a query document this project is built to avoid. A binding that needs to
        // branch on the kind of failure gets `ok: false` plus core's own wording.
        Err(error) => failure(json!({ "error": error.to_string() })),
    }
}

/// A response whose operation succeeded.
fn success(result: Value) -> EngineResponse {
    EngineResponse { transport_version: TRANSPORT_VERSION, ok: true, result }
}

/// A response whose operation did not.
///
/// Not a Rust error: the request arrived intact and was understood, so the engine is
/// answering rather than refusing, and a binding that cannot parse a response is a bug
/// worth distinguishing from one that rejected the caller's query.
fn failure(result: Value) -> EngineResponse {
    EngineResponse { transport_version: TRANSPORT_VERSION, ok: false, result }
}