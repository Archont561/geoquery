//! Mirrors `src/execution.rs`: what one run across several sources amounts to.
//!
//! The rule under test is the corpus's: never hide a source failure from an application,
//! and let every response carry per-source status. Most of these tests are ways of
//! getting that wrong — a failure rounded up to success, a success rounded down, or a
//! top-line status that drifts out of step with the sources it is supposed to summarize.

use geoquery_core::{
    AdapterError, CapabilityFinding, Cause, ExecutionOutcome, ExecutionStatus, QueryFeature,
    SourceOutcome, SourceStatus, Support,
};
use geoquery_types::{GeoResult, Provenance, ResultType, ServiceType};
use pretty_assertions::assert_eq;
use serde_json::json;

fn result_from(source: &str, id: &str) -> GeoResult {
    GeoResult::new(
        id,
        ResultType::Feature,
        Provenance::new(
            source,
            format!("https://{source}.test"),
            ServiceType::Stac,
            serde_json::Map::new(),
            "2026-10-04T09:00:00Z".parse().expect("a valid instant"),
        ),
    )
}

fn ok(source: &str) -> SourceStatus {
    SourceStatus {
        source: source.to_owned(),
        outcome: SourceOutcome::Ok,
    }
}

fn timed_out(source: &str) -> SourceStatus {
    SourceStatus {
        source: source.to_owned(),
        outcome: SourceOutcome::Failed {
            error: AdapterError::Timeout {
                service: source.to_owned(),
                after_ms: 30_000,
            },
        },
    }
}

fn degraded(source: &str) -> SourceStatus {
    SourceStatus {
        source: source.to_owned(),
        outcome: SourceOutcome::Degraded {
            findings: vec![CapabilityFinding {
                feature: QueryFeature::Sort,
                support: Support::Local {
                    cause: Cause::Undeclared,
                },
            }],
        },
    }
}

#[test]
fn an_execution_that_asked_nobody_did_not_succeed() {
    // The tempting answer is "ok, zero results". It is wrong, and dangerously so: a
    // registry that matched no source and a registry whose every source returned nothing
    // produce the same empty list, and only the status can tell them apart.
    let outcome = ExecutionOutcome::default();

    assert_eq!(outcome.status(), ExecutionStatus::Failed);
    assert!(outcome.results.is_empty());
}

#[test]
fn one_source_that_answered_is_a_whole_success() {
    let outcome = ExecutionOutcome {
        results: vec![result_from("nasa", "scene-a")],
        sources: vec![ok("nasa")],
    };

    assert_eq!(outcome.status(), ExecutionStatus::Ok);
    assert!(!outcome.is_degraded());
    assert_eq!(outcome.failures().count(), 0);
}

#[test]
fn a_source_that_failed_beside_one_that_worked_makes_the_run_partial() {
    let outcome = ExecutionOutcome {
        results: vec![result_from("nasa", "scene-a")],
        sources: vec![ok("nasa"), timed_out("private-stac")],
    };

    assert_eq!(outcome.status(), ExecutionStatus::Partial);
    assert_eq!(
        outcome
            .failures()
            .map(|status| status.source.as_str())
            .collect::<Vec<_>>(),
        vec!["private-stac"]
    );
}

#[test]
fn results_arriving_does_not_excuse_a_source_that_did_not() {
    // The failure mode this guards: a run that returned plenty of data looks fine, and
    // the application never learns that half its catalogue was unreachable.
    let outcome = ExecutionOutcome {
        results: vec![
            result_from("nasa", "scene-a"),
            result_from("nasa", "scene-b"),
            result_from("nasa", "scene-c"),
        ],
        sources: vec![ok("nasa"), timed_out("private-stac")],
    };

    assert_eq!(outcome.status(), ExecutionStatus::Partial);
}

#[test]
fn every_source_failing_is_a_failure_and_not_a_partial_success() {
    let outcome = ExecutionOutcome {
        results: Vec::new(),
        sources: vec![timed_out("nasa"), timed_out("private-stac")],
    };

    assert_eq!(outcome.status(), ExecutionStatus::Failed);
    assert_eq!(outcome.failures().count(), 2);
}

#[test]
fn a_source_that_was_never_asked_is_not_a_source_that_succeeded() {
    // Skipping is a decision the planner made, usually because the source could not take
    // the query at all. It is not an error, and it is not an answer either.
    let outcome = ExecutionOutcome {
        results: Vec::new(),
        sources: vec![SourceStatus {
            source: "sentinel".to_owned(),
            outcome: SourceOutcome::Skipped {
                reason: "no semantic capability and no text index to fall back to".to_owned(),
            },
        }],
    };

    assert_eq!(outcome.status(), ExecutionStatus::Failed);
    assert_eq!(
        outcome.failures().count(),
        0,
        "skipped is not failed: nothing went wrong, it just did not happen"
    );
}

#[test]
fn degradation_does_not_lower_the_status_but_stays_visible() {
    // A source that answered, with the sort applied here instead of there, succeeded — so
    // degradation never shows up in the top line. It still has to be reachable, or the
    // corpus rule about always reporting it would be broken by the aggregate rather than
    // by the adapter. Mixed in with sources of every other kind, because collecting the
    // degradations has to skip those rather than trip over them.
    let outcome = ExecutionOutcome {
        results: vec![result_from("nasa", "scene-a")],
        sources: vec![
            ok("sentinel"),
            degraded("nasa"),
            timed_out("private-stac"),
            SourceStatus {
                source: "skipped-one".to_owned(),
                outcome: SourceOutcome::Skipped {
                    reason: "out of scope".to_owned(),
                },
            },
        ],
    };

    // Partial, because one source was never asked — and the degradation is still the
    // only thing listed, so collecting them does not pick up noise from the others.
    assert_eq!(outcome.status(), ExecutionStatus::Partial);
    assert!(outcome.is_degraded());
    assert_eq!(
        outcome
            .degradations()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        vec!["sorting is applied locally: the service declared nothing about it"]
    );
}

#[test]
fn the_top_line_cannot_drift_out_of_step_with_the_sources() {
    // `status` is computed, not stored, so there is no second place for the truth to
    // live. Changing the sources changes the answer with no call to keep them in sync —
    // which is a whole class of bug the type makes unrepresentable.
    let mut outcome = ExecutionOutcome {
        results: vec![result_from("nasa", "scene-a")],
        sources: vec![ok("nasa")],
    };
    assert_eq!(outcome.status(), ExecutionStatus::Ok);

    outcome.sources.push(timed_out("private-stac"));
    assert_eq!(outcome.status(), ExecutionStatus::Partial);

    outcome.sources.retain(|status| status.source != "nasa");
    assert_eq!(outcome.status(), ExecutionStatus::Failed);
}

#[test]
fn what_a_source_contributed_is_counted_rather_than_claimed() {
    // Every result carries provenance, so a per-source count stored alongside would be a
    // second copy of a fact already in the data, free to disagree with it.
    let outcome = ExecutionOutcome {
        results: vec![
            result_from("nasa", "scene-a"),
            result_from("private-stac", "scene-b"),
            result_from("nasa", "scene-c"),
        ],
        sources: vec![ok("nasa"), ok("private-stac")],
    };

    assert_eq!(outcome.contributed("nasa"), 2);
    assert_eq!(outcome.contributed("private-stac"), 1);
    assert_eq!(outcome.contributed("never-asked"), 0);
}

#[test]
fn the_partial_failure_shape_from_the_corpus_falls_out_of_this() {
    // `.knowledge/query/planner.md` specifies the response as
    // `{"status": "partial", "sources": {"nasa": "ok", "private-stac": "timeout"}}`.
    // Core does not own that JSON — the protocol layer does — but it has to carry enough
    // for the protocol layer to produce it without guessing, and this is the proof.
    let outcome = ExecutionOutcome {
        results: vec![result_from("nasa", "scene-a")],
        sources: vec![ok("nasa"), timed_out("private-stac")],
    };

    let rendered = json!({
        "status": outcome.status().as_str(),
        "sources": outcome
            .sources
            .iter()
            .map(|status| (status.source.clone(), status.summary().into()))
            .collect::<serde_json::Map<String, serde_json::Value>>(),
    });

    assert_eq!(
        rendered,
        json!({
            "status": "partial",
            "sources": { "nasa": "ok", "private-stac": "timeout" }
        })
    );
}

#[test]
fn a_failed_source_summarizes_as_what_went_wrong_not_merely_that_something_did() {
    // "failed" is never the useful word. A timeout invites a retry, a 401 invites a
    // credential, and a malformed response invites a bug report.
    let cases = [
        (
            AdapterError::Timeout {
                service: "a".to_owned(),
                after_ms: 1,
            },
            "timeout",
        ),
        (
            AdapterError::Auth {
                profile: Some("nasa".to_owned()),
            },
            "auth",
        ),
        (
            AdapterError::Unreachable {
                service: "a".to_owned(),
                detail: "dns".to_owned(),
            },
            "unreachable",
        ),
        (
            AdapterError::Service {
                status: 500,
                body: String::new(),
            },
            "service-error",
        ),
        (
            AdapterError::Malformed {
                detail: "x".to_owned(),
            },
            "malformed",
        ),
        (
            AdapterError::UnsupportedQuery {
                report: geoquery_core::CapabilityReport::default(),
            },
            "unsupported-query",
        ),
    ];

    for (error, expected) in cases {
        let status = SourceStatus {
            source: "a".to_owned(),
            outcome: SourceOutcome::Failed { error },
        };
        assert_eq!(status.summary(), expected);
    }
}

#[test]
fn a_degraded_or_skipped_source_summarizes_too() {
    assert_eq!(ok("nasa").summary(), "ok");
    assert_eq!(degraded("nasa").summary(), "degraded");
    assert_eq!(
        SourceStatus {
            source: "nasa".to_owned(),
            outcome: SourceOutcome::Skipped {
                reason: "out of scope".to_owned()
            },
        }
        .summary(),
        "skipped"
    );
}

#[test]
fn an_outcome_survives_a_round_trip_through_json() {
    // It crosses into Python, TypeScript and an HTTP body, so every arm of every enum in
    // it has to be expressible as data — which is why `AdapterError` carries no cause
    // chain into a transport library's error type.
    let outcome = ExecutionOutcome {
        results: vec![result_from("nasa", "scene-a")],
        sources: vec![ok("nasa"), degraded("sentinel"), timed_out("private-stac")],
    };

    let json = serde_json::to_string(&outcome).expect("serializes");
    let back: ExecutionOutcome = serde_json::from_str(&json).expect("deserializes");

    assert_eq!(back, outcome);
    assert_eq!(back.status(), ExecutionStatus::Partial);
}

#[test]
fn every_status_has_a_word() {
    assert_eq!(ExecutionStatus::Ok.as_str(), "ok");
    assert_eq!(ExecutionStatus::Partial.as_str(), "partial");
    assert_eq!(ExecutionStatus::Failed.as_str(), "failed");
    assert_eq!(ExecutionStatus::Partial.to_string(), "partial");
}
