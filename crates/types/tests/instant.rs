//! Integration tests for `geoquery_types::instant`.
//!
//! `instant` is a private `serde` helper, so it has no public surface of its own. These
//! reach it through the three public fields annotated with it, which is the only way a
//! caller ever reaches it — and the point of the module is that those three fields agree
//! about what a timestamp is, so exercising them together is exercising the thing itself.

use chrono::{DateTime, Utc};
use geoquery_types::{Provenance, SourceMetadata, TemporalExtent};
use pretty_assertions::assert_eq;
use serde_json::json;

fn instant(text: &str) -> DateTime<Utc> {
    text.parse().expect("a test timestamp is valid")
}

#[test]
fn every_instant_on_the_wire_accepts_a_bare_date() {
    // A date is what a hand-written `resource.yaml` or a fixture carries, and the query
    // AST already reads one. A descriptor that refused the same string would mean the
    // dialect depended on which struct the field happened to sit in.
    let extent: TemporalExtent = serde_json::from_value(json!({ "start": "2026-01-01" }))
        .expect("an extent takes a bare date");
    assert_eq!(extent.start, Some(instant("2026-01-01T00:00:00Z")));

    let source: SourceMetadata = serde_json::from_value(json!({ "fetchedAt": "2026-01-01" }))
        .expect("a fetch time takes a bare date");
    assert_eq!(source.fetched_at, Some(instant("2026-01-01T00:00:00Z")));

    let provenance: Provenance = serde_json::from_value(json!({
        "source": "fixture",
        "service": "https://example.test/stac",
        "protocol": "stac",
        "query": {},
        "timestamp": "2026-01-01"
    }))
    .expect("a provenance timestamp takes a bare date");
    assert_eq!(provenance.timestamp, instant("2026-01-01T00:00:00Z"));
}

#[test]
fn sub_second_precision_survives_a_round_trip() {
    // Normalising to one output format must not round the value on the way through: a
    // fetch time recorded to the millisecond is a different instant from the second it
    // falls in, and two results ordered by it would silently tie.
    let source: SourceMetadata =
        serde_json::from_value(json!({ "fetchedAt": "2026-01-05T10:00:00.123Z" }))
            .expect("a millisecond instant parses");

    assert_eq!(
        serde_json::to_value(&source).expect("it serializes"),
        json!({ "fetchedAt": "2026-01-05T10:00:00.123Z" })
    );
}

#[test]
fn an_instant_that_is_neither_form_is_refused_the_same_way_everywhere() {
    // One dialect means one complaint. A caller that mistyped a date should not have to
    // learn which struct it was in to find out what the field would have accepted.
    for document in [
        json!({ "start": "the first of January" }),
        json!({ "end": "01/01/2026" }),
    ] {
        let error =
            serde_json::from_value::<TemporalExtent>(document.clone()).expect_err("not an instant");
        assert!(
            error.to_string().contains("RFC 3339"),
            "{document} produced {error}"
        );
    }

    // And a bound the document must carry, not only the ones it may leave out. Serde
    // types `with` against the field, so an `Option` field and a bare one are read by
    // different functions — "the same way everywhere" is the claim, so both are asserted.
    let error = serde_json::from_value::<Provenance>(json!({
        "source": "fixture",
        "service": "https://example.test/stac",
        "protocol": "stac",
        "query": {},
        "timestamp": "the first of January"
    }))
    .expect_err("not an instant");
    assert!(error.to_string().contains("RFC 3339"), "{error}");
}
