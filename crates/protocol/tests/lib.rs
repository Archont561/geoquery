//! The wire shapes, tested as bytes.
//!
//! Asserted through serialised JSON rather than by constructing the structs, because the
//! thing a binding depends on is the field *names* on the wire — `transportVersion` and not
//! `transport_version` — and those are decided by the serde attributes rather than by the
//! struct definitions. A test that builds an `EngineRequest` and compares it to another
//! would pass if every attribute on this crate were deleted.
//!
//! Two tools, two jobs. `proptest` generates the values, for the properties that must hold
//! for *any* JSON and cannot be checked one example at a time. `rstest`'s `#[fixture]`
//! builds the requests, for the examples: a fixture is the Rust name for a value a suite
//! sets up per test, and it is what keeps four call sites from spelling out the same
//! envelope by hand and drifting apart.

use geoquery_protocol::{EngineRequest, EngineResponse, Operation, TRANSPORT_VERSION};
use proptest::prelude::*;
// `fixture` rather than `rstest`: in rstest 0.24 a fixture is declared with `#[fixture]`
// directly, and importing the test macro alongside it is what older versions wanted.
use rstest::fixture;

/// Any JSON value, assembled from proptest's own strategies.
///
/// Written out rather than reached for because `serde_json::Value` does not implement
/// `Arbitrary`: `serde_json` had an `arbitrary_impl` feature and dropped it, so the recursive
/// structure has to be composed here. `prop_recursive` is what makes it terminate — depth is
/// bounded, and the leaves are the strategies that cannot recurse.
fn arb_json() -> impl Strategy<Value = serde_json::Value> {
    let leaf = prop_oneof![
        Just(serde_json::Value::Null),
        any::<bool>().prop_map(serde_json::Value::from),
        any::<String>().prop_map(serde_json::Value::from),
        any::<i64>().prop_map(serde_json::Value::from),
        // Floats are generated, not excluded. `serde_json`'s parser is not correctly rounded
        // — see the canary in the TypeScript property tests — so this is the property most
        // likely to find that again, and the range is bounded because `Value::from(f64)`
        // panics on a value `Number` cannot hold.
        (-1.0e9f64..1.0e9f64).prop_map(|number| {
            serde_json::Number::from_f64(number)
                .map_or(serde_json::Value::Null, serde_json::Value::from)
        }),
    ];
    leaf.prop_recursive(3, 8, 2, |inner| {
        prop_oneof![
            proptest::collection::vec(inner.clone(), 0..4).prop_map(serde_json::Value::from),
            proptest::collection::btree_map(any::<String>(), inner, 0..4)
                .prop_map(|entries| serde_json::Value::Object(entries.into_iter().collect())),
        ]
    })
}

/// A well-formed request, assembled rather than written out.
///
/// The point of a fixture here is that the envelope is spelled once. Four tests below
/// assert on the response, and each of them needs a valid request; writing the literal four
/// times is four chances to fix a typo in one of them and spend an afternoon on why one
/// test behaves differently from the other three.
#[fixture]
fn request() -> EngineRequest {
    EngineRequest {
        transport_version: TRANSPORT_VERSION,
        operation: Operation::Ping,
        payload: serde_json::json!({}),
    }
}

#[test]
fn an_operation_is_a_camel_cased_string_on_the_wire() {
    let text = serde_json::to_string(&request()).expect("a request serializes");

    assert_eq!(
        text,
        r#"{"transportVersion":1,"operation":"ping","payload":{}}"#
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
fn an_omitted_payload_decodes_as_null() {
    // `default` rather than `required`: a binding that sends `{transportVersion, operation}`
    // for an operation that takes no arguments is not making a mistake, and rejecting it
    // would make every argument-less call a special case in every language.
    let decoded: EngineRequest =
        serde_json::from_str(r#"{"transportVersion":1,"operation":"protocolVersion"}"#)
            .expect("a payload may be omitted");

    assert_eq!(decoded.payload, serde_json::Value::Null);
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

proptest! {
    /// Every request survives a trip through the wire and back unchanged.
    ///
    /// The property behind the whole crate: the envelope is the interface, so a request that
    /// cannot be read back is a binding that cannot send anything. `proptest` earns its place
    /// on the payload in particular — the example tests only ever send `{}`, and a payload
    /// that decodes to one `Value` but re-encodes to different text would break every binding
    /// at once while every example still passed.
    #[test]
    fn a_request_round_trips(
        operation in proptest::sample::select(vec![Operation::Ping, Operation::ProtocolVersion, Operation::ParseDocument]),
        payload in arb_json(),
    ) {
        let original = EngineRequest { transport_version: TRANSPORT_VERSION, operation, payload };

        let text = serde_json::to_string(&original).expect("a request serializes");
        let decoded: EngineRequest = serde_json::from_str(&text).expect("a request decodes");

        prop_assert_eq!(decoded, original);
    }

    /// A response round-trips too, and `ok` survives as a boolean rather than becoming text.
    #[test]
    fn a_response_round_trips(
        ok in any::<bool>(),
        result in arb_json(),
    ) {
        let original = EngineResponse { transport_version: TRANSPORT_VERSION, ok, result };

        let text = serde_json::to_string(&original).expect("a response serializes");
        let decoded: EngineResponse = serde_json::from_str(&text).expect("a response decodes");

        prop_assert_eq!(decoded, original);
    }

    /// Whatever payload goes in comes back out — including the shapes serde_json cannot
    /// represent exactly.
    ///
    /// This is the Rust half of the canary in the TypeScript property tests. `arbitrary_json`
    /// generates real JSON, so the `f64` values it produces are doubles this process already
    /// holds and serialises without going through a parser; the loss that property tests
    /// found happens when a *decimal literal* is parsed, which is a different code path. Both
    /// halves are asserted because the two disagree, and a fix that closed only one of them
    /// would leave the wire half-broken.
    #[test]
    fn a_payload_comes_back_equal(
        payload in arb_json(),
    ) {
        let original = EngineRequest {
            transport_version: TRANSPORT_VERSION,
            operation: Operation::Ping,
            payload,
        };

        let text = serde_json::to_string(&original).expect("a request serializes");
        let decoded: EngineRequest = serde_json::from_str(&text).expect("a request decodes");

        prop_assert_eq!(decoded.payload, original.payload);
    }

    /// The envelope's field names are the interface, so they are asserted for every operation
    /// rather than for the one an example happened to use.
    #[test]
    fn the_envelope_fields_are_spelled_the_same_way_for_every_operation(
        operation in proptest::sample::select(vec![Operation::Ping, Operation::ProtocolVersion, Operation::ParseDocument]),
    ) {
        let text = serde_json::to_string(&EngineRequest {
            transport_version: TRANSPORT_VERSION,
            operation,
            payload: serde_json::Value::Null,
        })
        .expect("a request serializes");

        prop_assert!(text.contains(r#""transportVersion":1"#), "{text}");
        prop_assert!(text.contains(r#""operation":"#), "{text}");
    }
}
