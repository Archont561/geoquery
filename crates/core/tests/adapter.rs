//! Mirrors `src/adapter.rs`: the boundary between the engine and a protocol.
//!
//! This file is the proof that the contract is usable from outside. An integration test
//! is a separate crate that links `geoquery-core` the way any dependent does, so the
//! `FakeAdapter` below can only reach the public surface — if it compiles here, an
//! adapter crate can be written against this trait alone, which is what AC#4 asks for.
//!
//! The fake is deliberately a whole small adapter rather than a stub per test: a trait is
//! only proven usable by something that uses all of it at once.

use std::collections::BTreeMap;

use async_trait::async_trait;
use geoquery_core::{
    AdapterError, CapabilityReport, Confidence, Detection, Endpoint, QueryFeature, QueryResult,
    ServiceAdapter, Support,
};
use geoquery_types::{
    CapabilitySet, GeoQuery, GeoResult, Provenance, ResultType, ServiceDescriptor, ServiceType,
    SpatialOperation, SpatialPredicate,
};
use pretty_assertions::assert_eq;
use serde_json::json;

/// An adapter for a catalogue that lives in a `BTreeMap`.
///
/// It speaks no protocol, which is the point: everything it does is what the contract
/// requires of a real one, and nothing it does depends on a network.
#[derive(Debug)]
struct FakeAdapter {
    /// Owned rather than a literal, which is the case the trait's `-> &str` exists for:
    /// a user registering the same adapter against two catalogues names them apart at
    /// runtime, so the name cannot be `&'static str`.
    name: String,
    /// What this adapter claims to serve, keyed by endpoint URL.
    catalogues: BTreeMap<String, Vec<&'static str>>,
}

impl FakeAdapter {
    fn new() -> Self {
        Self {
            name: "fake".to_owned(),
            catalogues: BTreeMap::from([(
                "https://fake.test/stac".to_owned(),
                vec!["scene-a", "scene-b"],
            )]),
        }
    }

    fn declared() -> CapabilitySet {
        CapabilitySet {
            spatial: vec![SpatialOperation::Bbox],
            temporal: Some(true),
            pagination: Some(true),
            ..CapabilitySet::default()
        }
    }
}

#[async_trait]
impl ServiceAdapter for FakeAdapter {
    fn name(&self) -> &str {
        &self.name
    }

    fn detect(&self, endpoint: &Endpoint) -> Detection {
        if self.catalogues.contains_key(&endpoint.url) {
            return Detection::Match {
                service_type: ServiceType::Stac,
                confidence: Confidence::CERTAIN,
            };
        }
        if endpoint.url.contains("stac") {
            return Detection::Uncertain {
                reason: "the path says STAC but this adapter has never seen this host".to_owned(),
            };
        }
        Detection::NoMatch
    }

    async fn describe(&self, endpoint: &Endpoint) -> Result<ServiceDescriptor, AdapterError> {
        if !self.catalogues.contains_key(&endpoint.url) {
            return Err(AdapterError::Unreachable {
                service: endpoint.url.clone(),
                detail: "no such catalogue".to_owned(),
            });
        }
        let mut descriptor = ServiceDescriptor::new(ServiceType::Stac, endpoint.url.clone());
        descriptor.id = Some("fake".to_owned());
        descriptor.capabilities = Some(Self::declared());
        Ok(descriptor)
    }

    async fn query(
        &self,
        service: &ServiceDescriptor,
        query: &GeoQuery,
    ) -> Result<QueryResult, AdapterError> {
        let report = CapabilityReport::for_query(query, service);
        if report.refused().next().is_some() {
            return Err(AdapterError::UnsupportedQuery { report });
        }
        let provenance = Provenance::new(
            self.name(),
            service.url.clone(),
            ServiceType::Stac,
            serde_json::Map::new(),
            "2026-10-04T09:00:00Z".parse().expect("a valid instant"),
        );
        let ids = self
            .catalogues
            .get(&service.url)
            .cloned()
            .unwrap_or_default();
        Ok(QueryResult {
            results: ids
                .into_iter()
                .map(|id| GeoResult::new(id, ResultType::Feature, provenance.clone()))
                .collect(),
            degradations: report.not_pushed().cloned().collect(),
            next_page: None,
            provenance,
        })
    }
}

fn endpoint(url: &str) -> Endpoint {
    Endpoint::new(url)
}

#[tokio::test]
async fn an_adapter_written_outside_this_crate_can_satisfy_the_whole_contract() {
    // The compile is most of the assertion. The rest checks that a caller holding only
    // the trait object — which is how the engine will hold it — can still get at
    // everything: detection, description, and a query against the description.
    let adapter: Box<dyn ServiceAdapter> = Box::new(FakeAdapter::new());

    assert_eq!(
        adapter.detect(&endpoint("https://fake.test/stac")),
        Detection::Match {
            service_type: ServiceType::Stac,
            confidence: Confidence::CERTAIN,
        }
    );

    let described = adapter
        .describe(&endpoint("https://fake.test/stac"))
        .await
        .expect("the catalogue is there");
    assert_eq!(described.r#type, ServiceType::Stac);

    let answered = adapter
        .query(&described, &GeoQuery::default())
        .await
        .expect("an empty query asks for nothing unsupported");
    assert_eq!(answered.results.len(), 2);
    assert_eq!(answered.provenance.source, "fake");
}

#[tokio::test]
async fn an_adapter_that_does_not_recognise_an_endpoint_says_so_instead_of_failing() {
    // Not recognising a URL is the normal case — most adapters will not match most
    // endpoints — so it has to be an ordinary answer and not an error. Reserving errors
    // for things that went wrong is what lets the registry treat a real failure as one.
    let adapter = FakeAdapter::new();

    assert_eq!(
        adapter.detect(&endpoint("https://example.org/wms")),
        Detection::NoMatch
    );
    assert!(matches!(
        adapter.detect(&endpoint("https://unknown.test/stac/v1")),
        Detection::Uncertain { .. }
    ));
}

#[tokio::test]
async fn the_registry_can_hold_adapters_it_knows_nothing_about() {
    // What the plugin model needs: a heterogeneous collection, asked in turn, best match
    // wins. None of this names a concrete adapter type, which is the property that lets a
    // user register one at runtime.
    let registry: Vec<Box<dyn ServiceAdapter>> = vec![Box::new(FakeAdapter::new())];
    let probe = endpoint("https://fake.test/stac");

    let best = registry
        .iter()
        .filter_map(|adapter| match adapter.detect(&probe) {
            Detection::Match {
                confidence,
                service_type,
            } => Some((confidence, service_type, adapter.name())),
            Detection::Uncertain { .. } | Detection::NoMatch => None,
        })
        .max_by_key(|(confidence, _, _)| *confidence);

    assert_eq!(
        best.map(|(_, _, name)| name),
        Some("fake"),
        "detections must be orderable for `best match wins` to mean anything"
    );
}

#[test]
fn a_confidence_is_a_number_between_zero_and_one_or_it_is_not_a_confidence() {
    // The constructor is the only way in, so `Ord` below can be total without a branch
    // that cannot happen. A NaN admitted here would make sorting incoherent rather than
    // merely wrong.
    assert!(Confidence::new(0.0).is_some());
    assert!(Confidence::new(1.0).is_some());
    assert!(Confidence::new(0.5).is_some());

    assert_eq!(Confidence::new(f64::NAN), None);
    assert_eq!(Confidence::new(f64::INFINITY), None);
    assert_eq!(Confidence::new(-0.000_1), None);
    assert_eq!(Confidence::new(1.000_1), None);
}

#[test]
fn confidences_sort_so_that_the_best_match_wins() {
    let mut scores: Vec<Confidence> = [0.4, 1.0, 0.0, 0.9]
        .into_iter()
        .map(|value| Confidence::new(value).expect("all in range"))
        .collect();
    scores.sort_unstable();

    assert_eq!(
        scores.iter().map(|score| score.get()).collect::<Vec<_>>(),
        vec![0.0, 0.4, 0.9, 1.0]
    );
    assert_eq!(scores.iter().copied().max(), Some(Confidence::CERTAIN));
}

#[test]
fn a_confidence_crosses_the_wire_as_a_bare_number_and_is_checked_on_the_way_back() {
    // Deserialization has to go through the same gate as the constructor. A derived impl
    // would let a hand-written JSON document create the NaN the constructor refuses.
    let score = Confidence::new(0.75).expect("in range");
    assert_eq!(serde_json::to_string(&score).expect("serializes"), "0.75");
    assert_eq!(
        serde_json::from_str::<Confidence>("0.75").expect("deserializes"),
        score
    );

    let bad = serde_json::from_str::<Confidence>("1.5");
    assert!(bad.is_err(), "out of range must not survive a round trip");
}

#[tokio::test]
async fn a_result_set_says_who_answered_even_when_the_answer_is_empty() {
    // Provenance on each `GeoResult` cannot cover this: a service that matched nothing
    // produces no results to hang it on, and "nothing found" is only interpretable
    // alongside who was asked and what they were sent.
    let adapter = FakeAdapter::new();
    let mut empty = ServiceDescriptor::new(ServiceType::Stac, "https://fake.test/empty");
    empty.capabilities = Some(FakeAdapter::declared());

    let answered = adapter
        .query(&empty, &GeoQuery::default())
        .await
        .expect("an unknown catalogue is empty, not broken");

    assert!(answered.results.is_empty());
    assert_eq!(answered.provenance.service, "https://fake.test/empty");
}

#[tokio::test]
async fn an_adapter_reports_what_it_could_not_push_rather_than_dropping_it() {
    // The service takes a bbox but not an exact intersects. The query still runs, and the
    // result carries the finding that says the geometry test has not been applied yet.
    let adapter = FakeAdapter::new();
    let described = adapter
        .describe(&endpoint("https://fake.test/stac"))
        .await
        .expect("the catalogue is there");

    let answered = adapter
        .query(
            &described,
            &GeoQuery {
                spatial: Some(SpatialPredicate {
                    op: SpatialOperation::Intersects,
                    bbox: None,
                    geometry: Some(json!({ "type": "Point", "coordinates": [21.0, 52.2] })),
                    distance: None,
                    unit: None,
                    crs: None,
                }),
                ..GeoQuery::default()
            },
        )
        .await
        .expect("an approximable predicate is not a refusal");

    assert_eq!(answered.degradations.len(), 1);
    assert!(matches!(
        answered.degradations[0].support,
        Support::Approximated { .. }
    ));
}

#[tokio::test]
async fn a_query_the_service_cannot_answer_fails_with_the_report_that_explains_why() {
    // The error is not a sentence about an unsupported query, it is the report. Whoever
    // catches it can tell a user which feature was the problem without parsing anything.
    let adapter = FakeAdapter::new();
    let described = adapter
        .describe(&endpoint("https://fake.test/stac"))
        .await
        .expect("the catalogue is there");

    let refused = adapter
        .query(
            &described,
            &GeoQuery {
                semantic: Some("flood risk".to_owned()),
                ..GeoQuery::default()
            },
        )
        .await
        .expect_err("no semantic capability and no full text to fall back to");

    let AdapterError::UnsupportedQuery { report } = refused else {
        panic!("expected an unsupported query, got {refused:?}");
    };
    assert_eq!(
        report
            .refused()
            .map(|finding| &finding.feature)
            .collect::<Vec<_>>(),
        vec![&QueryFeature::Semantic]
    );
}

#[test]
fn adapter_errors_read_as_sentences() {
    // These reach a user through a CLI line, an HTTP body and a Python exception, so the
    // wording lives with the variant rather than at each of those three call sites.
    let cases = [
        (
            AdapterError::Unreachable {
                service: "https://fake.test/stac".to_owned(),
                detail: "connection refused".to_owned(),
            },
            "could not reach https://fake.test/stac: connection refused",
        ),
        (
            AdapterError::Service {
                status: 503,
                body: "upstream is restarting".to_owned(),
            },
            "the service answered 503: upstream is restarting",
        ),
        (
            AdapterError::Auth {
                profile: Some("nasa".to_owned()),
            },
            "authentication failed for profile `nasa`",
        ),
        (
            AdapterError::Auth { profile: None },
            "the service requires authentication and no profile was configured",
        ),
        (
            AdapterError::Malformed {
                detail: "item 3 has no geometry".to_owned(),
            },
            "the service sent something this adapter could not read: item 3 has no geometry",
        ),
        (
            AdapterError::Timeout {
                service: "private-stac".to_owned(),
                after_ms: 30_000,
            },
            "private-stac did not answer within 30000ms",
        ),
    ];

    for (error, expected) in cases {
        assert_eq!(error.to_string(), expected);
    }
}

#[test]
fn an_unsupported_query_error_names_the_feature_that_caused_it() {
    let report = CapabilityReport::for_query(
        &GeoQuery {
            semantic: Some("flood risk".to_owned()),
            ..GeoQuery::default()
        },
        &ServiceDescriptor::new(ServiceType::Stac, "https://fake.test/stac"),
    );

    assert_eq!(
        AdapterError::UnsupportedQuery { report }.to_string(),
        "the service cannot answer this query: semantic ranking is refused: \
         the service declared nothing about it"
    );
}

#[test]
fn an_endpoint_carries_what_is_needed_to_knock_on_the_door() {
    // Auth is a profile name rather than a secret. Credentials belong to whatever
    // resolves the profile, so an `Endpoint` can be logged, cached and compared without
    // leaking one.
    let mut probe = Endpoint::new("https://fake.test/stac");
    probe.auth_profile = Some("nasa".to_owned());
    probe
        .headers
        .insert("accept".to_owned(), "application/json".to_owned());

    assert_eq!(probe.url, "https://fake.test/stac");
    assert_eq!(probe.auth_profile.as_deref(), Some("nasa"));
    assert_eq!(probe.headers["accept"], "application/json");
    assert_eq!(Endpoint::new("https://fake.test/stac").auth_profile, None);
}

/// An adapter that knows more about its protocol than any one service admits.
#[derive(Debug)]
struct WellInformedAdapter {
    name: String,
}

#[async_trait]
impl ServiceAdapter for WellInformedAdapter {
    fn name(&self) -> &str {
        &self.name
    }

    fn detect(&self, _endpoint: &Endpoint) -> Detection {
        Detection::NoMatch
    }

    async fn describe(&self, _endpoint: &Endpoint) -> Result<ServiceDescriptor, AdapterError> {
        Err(AdapterError::Malformed {
            detail: "this adapter only exists to answer questions about capability".to_owned(),
        })
    }

    async fn query(
        &self,
        _service: &ServiceDescriptor,
        _query: &GeoQuery,
    ) -> Result<QueryResult, AdapterError> {
        Err(AdapterError::Malformed {
            detail: "nor to run anything".to_owned(),
        })
    }

    fn capabilities(&self, service: &ServiceDescriptor) -> CapabilitySet {
        // Every STAC search endpoint takes a bounding box whether or not its landing page
        // says so. The adapter knows that; the descriptor does not.
        let mut known = service.capabilities.clone().unwrap_or_default();
        if !known.spatial.contains(&SpatialOperation::Bbox) {
            known.spatial.push(SpatialOperation::Bbox);
        }
        known
    }
}

#[tokio::test]
async fn by_default_an_adapter_reports_exactly_what_the_service_declared() {
    let adapter = FakeAdapter::new();
    let described = adapter
        .describe(&endpoint("https://fake.test/stac"))
        .await
        .expect("the catalogue is there");

    assert_eq!(adapter.capabilities(&described), FakeAdapter::declared());
}

#[test]
fn an_adapter_that_knows_its_protocol_can_rescue_an_under_described_service() {
    // The point of `capabilities` being a method rather than a field read. A service that
    // published nothing would otherwise be planned against as if it could do nothing, and
    // every predicate would be dragged back here for no reason.
    let bare = ServiceDescriptor::new(ServiceType::Stac, "https://quiet.test/stac");
    let query = GeoQuery {
        spatial: Some(SpatialPredicate {
            op: SpatialOperation::Bbox,
            bbox: Some([20.8, 52.1, 21.3, 52.4]),
            geometry: None,
            distance: None,
            unit: None,
            crs: None,
        }),
        ..GeoQuery::default()
    };

    // The provided `report` reads the adapter's view, not the descriptor's.
    let adapter = WellInformedAdapter {
        name: "well-informed".to_owned(),
    };
    assert_eq!(
        adapter.report(&bare, &query).findings()[0].support,
        Support::Pushed
    );
    // And the descriptor on its own still says it knows nothing, so the difference is
    // genuinely the adapter's knowledge rather than a mutation of the service.
    assert_eq!(
        CapabilityReport::for_query(&query, &bare).findings()[0].support,
        Support::Local {
            cause: geoquery_core::Cause::Undeclared
        }
    );
    assert_eq!(bare.capabilities, None);
}

#[test]
fn the_default_report_agrees_with_reading_the_descriptor_directly() {
    let adapter = FakeAdapter::new();
    let mut described = ServiceDescriptor::new(ServiceType::Stac, "https://fake.test/stac");
    described.capabilities = Some(FakeAdapter::declared());
    let query = GeoQuery {
        limit: Some(10),
        ..GeoQuery::default()
    };

    assert_eq!(
        adapter.report(&described, &query),
        CapabilityReport::for_query(&query, &described)
    );
}

#[test]
fn a_rejected_confidence_says_what_it_was_given() {
    let error = Confidence::try_from(1.5).expect_err("1.5 is not a confidence");

    assert_eq!(
        error.to_string(),
        "confidence must be between 0 and 1, got 1.5"
    );
    assert_eq!(
        Confidence::new(f64::from(Confidence::CERTAIN)),
        Some(Confidence::CERTAIN),
        "converting to f64 and back must be the identity"
    );
}
