//! The wire shapes, tested as bytes.
//!
//! Asserted through serialised JSON rather than by constructing the structs, because the
//! thing a binding depends on is the field *names* on the wire — `transportVersion` and not
//! `transport_version` — and those are decided by the serde attributes rather than by the
//! struct definitions. A test that builds an `EngineRequest` and compares it to another
//! would pass if every attribute on this crate were deleted.
//!
//! `proptest` generates the values, for the properties that must hold for *any* JSON and
//! cannot be checked one example at a time. The examples use plain helpers. `request()`
//! was an `rstest` `#[fixture]` and has stopped being one: a fixture is only ever passed
//! to a test annotated `#[rstest]`, no test here is, so the attribute and the dependency
//! behind it were doing nothing that a function does not already do.

use geoquery_protocol::{EngineRequest, EngineResponse, Operation, TRANSPORT_VERSION};
use proptest::prelude::*;

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
        // Both halves of the integer domain, not just the signed one: the wire carries
        // `u64` too, and everything above `i64::MAX` is reachable only through it. Python's
        // generator covers the same span, which is what makes the two suites describe one
        // protocol rather than two.
        any::<i64>().prop_map(serde_json::Value::from),
        any::<u64>().prop_map(serde_json::Value::from),
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

/// A well-formed request, built rather than written out as JSON.
///
/// Building it from the struct is what makes the assertion below mean something: the test
/// compares the serialised text to a literal, so the field names it checks come from the
/// serde attributes under test rather than from a string the test also wrote.
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

/// The edges of the transport's number domain, pinned as values rather than described.
///
/// `geoquery-protocol` carries JSON numbers in the 64-bit domain: a signed integer, an
/// unsigned integer, or a double. JSON itself places no limit on an integer's magnitude,
/// so the domain is a property of this transport and not of the format, and these are the
/// tests that make it a stated one.
mod the_number_domain {
    use super::{EngineRequest, Operation, TRANSPORT_VERSION};

    fn round_trip(payload: serde_json::Value) -> serde_json::Value {
        let text = serde_json::to_string(&EngineRequest {
            transport_version: TRANSPORT_VERSION,
            operation: Operation::Ping,
            payload,
        })
        .expect("a request serializes");
        serde_json::from_str::<EngineRequest>(&text)
            .expect("a request decodes")
            .payload
    }

    #[test]
    fn the_extremes_of_the_domain_survive_exactly() {
        // The values a caller is most likely to reach with a real identifier: a database
        // bigint, a snowflake id, a 64-bit hash. All three are inside the domain and all
        // three have to come back bit-for-bit, not merely close.
        for extreme in [
            serde_json::json!(i64::MIN),
            serde_json::json!(i64::MAX),
            serde_json::json!(u64::MAX),
            serde_json::json!(0),
            serde_json::json!(-1),
        ] {
            assert_eq!(round_trip(extreme.clone()), extreme);
        }
    }

    #[test]
    fn an_integer_one_step_outside_the_domain_is_carried_as_a_double() {
        // This test documents a limitation rather than a guarantee, and it exists so the
        // limitation has a name and a place. `serde_json` parses an integer too large for
        // `i64` or `u64` as an `f64`, so it comes back as the nearest double.
        //
        // Widening the domain is a one-line change — `arbitrary_precision` on the
        // `serde_json` dependency — and it was measured and declined: it makes every
        // `Number` a heap-allocated string, including every coordinate in every geometry,
        // for roughly a seventh of the time of a large payload. It would also buy an
        // exactness the TypeScript binding cannot honour, because `JSON.parse` in
        // JavaScript rounds an integer past 2^53 before the engine is ever reached. A
        // transport that is exact in two bindings and lossy in the third is a worse
        // contract than one that is bounded in all three.
        let beyond = "-9223372036854775809";
        let payload: serde_json::Value =
            serde_json::from_str(beyond).expect("JSON permits any magnitude");

        assert!(
            payload.as_i64().is_none() && payload.as_u64().is_none(),
            "the point of the example is that it does not fit"
        );
        assert_eq!(
            round_trip(payload).to_string(),
            "-9.223372036854776e+18",
            "outside the domain the value is a double, and the round trip is stable"
        );
    }

    #[test]
    fn a_double_outside_the_integer_domain_is_still_exact() {
        // Leaving the *integer* domain is not leaving the number domain. Coordinates are
        // doubles and `float_roundtrip` is enabled for them, so magnitudes far beyond any
        // integer still survive unchanged.
        for coordinate in [
            serde_json::json!(1.0e300),
            serde_json::json!(4.188_295_992_926_135e-215),
            serde_json::json!(432_288_997.760_232_03),
        ] {
            assert_eq!(round_trip(coordinate.clone()), coordinate);
        }
    }
}
