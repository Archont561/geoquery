//! The boundary between the engine and a protocol.
//!
//! An adapter is the only thing in geoquery that knows what STAC, OGC API Features or a
//! bare `GeoJSON` file on a web server actually look like. Everything above it works in
//! [`GeoQuery`] and [`GeoResult`], and this module is where that translation is promised.
//!
//! The dependency arrow runs one way. An adapter crate depends on `geoquery-core` for
//! this contract and for nothing else; `geoquery-core` depends on no adapter at all. That
//! is what makes an out-of-tree adapter possible — the engine holds
//! `Box<dyn ServiceAdapter>` and never learns which protocol it got — and it is why the
//! trait is written with [`async_trait`] rather than native `async fn`, which is stable
//! but not yet dyn-compatible.
//!
//! The four methods split along one line: what can be answered from a URL alone
//! ([`detect`](ServiceAdapter::detect)), and what needs the service to answer
//! ([`describe`](ServiceAdapter::describe), [`query`](ServiceAdapter::query)). Only the
//! second half is async, so a registry can sort a hundred adapters against an endpoint
//! without awaiting anything.

use std::collections::BTreeMap;
use std::fmt;

use async_trait::async_trait;
use geoquery_types::{
    CapabilitySet, GeoQuery, GeoResult, Provenance, ServiceDescriptor, ServiceType,
};
use serde::{Deserialize, Serialize};

use crate::capability::{CapabilityFinding, CapabilityReport};

/// Where to go, and how to be let in.
///
/// The URL is a `String` rather than a parsed `Url` to match
/// [`ServiceDescriptor::url`](geoquery_types::ServiceDescriptor::url): a descriptor read
/// from a config file holds whatever the user typed, and an adapter that wants structure
/// can parse it with the rules of its own protocol.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Endpoint {
    /// The address to probe.
    pub url: String,
    /// The name of a credential profile, never a credential.
    ///
    /// Resolving the name to a secret belongs to whatever owns the credential store, so
    /// an `Endpoint` can be logged, cached and compared without leaking anything.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth_profile: Option<String>,
    /// Extra headers the user configured for this endpoint.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub headers: BTreeMap<String, String>,
}

impl Endpoint {
    /// An endpoint that is just a URL.
    #[must_use]
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            auth_profile: None,
            headers: BTreeMap::new(),
        }
    }
}

/// How sure an adapter is that an endpoint is its kind of service: `0.0` to `1.0`.
///
/// A newtype rather than a bare `f64` for one reason that matters: the registry picks the
/// best match, so confidences must be orderable, and `f64` is only partially ordered.
/// Refusing NaN and out-of-range values at construction is what makes the [`Ord`] below
/// total without a branch that cannot be reached.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "f64", into = "f64")]
pub struct Confidence(f64);

impl Confidence {
    /// No doubt at all: the endpoint announced itself.
    pub const CERTAIN: Self = Self(1.0);

    /// A confidence, or `None` if the number is not one.
    ///
    /// Rejects NaN and infinities as well as values outside `0.0..=1.0`, because a
    /// comparison against any of them is what would make "best match wins" incoherent.
    #[must_use]
    pub fn new(value: f64) -> Option<Self> {
        (value.is_finite() && (0.0..=1.0).contains(&value)).then_some(Self(value))
    }

    /// The number back out.
    #[must_use]
    pub fn get(self) -> f64 {
        self.0
    }
}

impl TryFrom<f64> for Confidence {
    type Error = ConfidenceOutOfRange;

    /// The gate deserialization goes through, so a hand-written JSON document cannot
    /// create the NaN [`Confidence::new`] refuses.
    fn try_from(value: f64) -> Result<Self, Self::Error> {
        Self::new(value).ok_or(ConfidenceOutOfRange { value })
    }
}

impl From<Confidence> for f64 {
    fn from(confidence: Confidence) -> Self {
        confidence.0
    }
}

/// A number that was offered as a [`Confidence`] and is not one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConfidenceOutOfRange {
    /// What was offered.
    pub value: f64,
}

impl fmt::Display for ConfidenceOutOfRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "confidence must be between 0 and 1, got {}", self.value)
    }
}

impl std::error::Error for ConfidenceOutOfRange {}

// Sound because the constructor and `TryFrom` are the only ways to build one and both
// reject NaN. `total_cmp` then agrees with `partial_cmp` everywhere a value can exist.
impl Eq for Confidence {}

impl Ord for Confidence {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.total_cmp(&other.0)
    }
}

// Deferring to `cmp` rather than deriving, so the partial and total orders cannot
// disagree — the derive would compare the inner `f64` directly and return `None` for a
// NaN that the constructor has already made impossible.
impl PartialOrd for Confidence {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// What an adapter makes of an endpoint it has been shown.
///
/// Three answers rather than two, because "I do not know" is a different fact from "no".
/// An endpoint every adapter rejects is a configuration error worth reporting; an
/// endpoint several adapters are unsure about is a case for asking it directly.
// Closed, unlike `AdapterError` below. Yes, no and maybe is the whole answer space, and
// every arm demands a different decision from the registry — so a caller that handled a
// future variant with a wildcard would be making the wrong one silently. Errors grow and
// are mostly displayed; this does not and is not.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Detection {
    /// This adapter recognises the endpoint.
    Match {
        /// What the endpoint turned out to be.
        service_type: ServiceType,
        /// How sure the adapter is.
        confidence: Confidence,
    },
    /// Something fits, but not enough to claim it from a URL alone.
    Uncertain {
        /// What fitted and what did not, for a log or a prompt.
        reason: String,
    },
    /// Not this adapter's kind of service.
    NoMatch,
}

/// One service's answer to one query, with everything needed to interpret it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueryResult {
    /// Who was asked, what they were sent, and when.
    ///
    /// Kept at the set level even though every [`GeoResult`] carries its own, because an
    /// empty result set has nothing to hang provenance on and "nothing found" is only
    /// interpretable next to the question that found nothing.
    pub provenance: Provenance,
    /// What came back, normalized.
    pub results: Vec<GeoResult>,
    /// Every part of the query that did not reach the service as written.
    ///
    /// Typed rather than a list of strings: the caller has to *act* on some of these —
    /// an [`Approximated`](crate::capability::Support::Approximated) spatial predicate
    /// still has to be applied to `results` — and acting on a sentence means parsing it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub degradations: Vec<CapabilityFinding>,
    /// An opaque token for the next page, if the service offered one.
    ///
    /// Opaque on purpose. Every protocol pages differently and the only portable thing to
    /// do with this is hand it back to the adapter that produced it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_page: Option<String>,
}

/// Everything that can go wrong between asking a service and understanding its answer.
///
/// Plain data, with no `source()` chain into a transport library's error type. That is
/// deliberate and it is the opposite of the choice
/// [`QueryDocumentError`](crate::document::QueryDocumentError) makes: a document error is
/// read once by whoever called the function, while an adapter error ends up in a
/// per-source status that is serialized to JSON and handed to Python, TypeScript and an
/// HTTP client. An adapter renders its underlying error into `detail` instead.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum AdapterError {
    /// The service could not be contacted at all.
    Unreachable {
        /// Which service.
        service: String,
        /// What the transport said.
        detail: String,
    },
    /// The service was contacted and refused.
    Service {
        /// The protocol status code, where there is one.
        status: u16,
        /// Whatever it sent back, for a human to read.
        body: String,
    },
    /// The service cannot answer this query, and the report says which part of it.
    ///
    /// Carries the whole [`CapabilityReport`] rather than a message so that a caller can
    /// decide what to do — drop the feature, try another source, or ask the user —
    /// without parsing prose.
    UnsupportedQuery {
        /// What was asked for, and what became of each part of it.
        report: CapabilityReport,
    },
    /// The service wants credentials that were not supplied or were not accepted.
    Auth {
        /// The profile that was tried, if one was.
        profile: Option<String>,
    },
    /// The service answered, and the answer was not something this adapter can read.
    Malformed {
        /// Where it stopped making sense.
        detail: String,
    },
    /// The service did not answer in time.
    ///
    /// Separate from [`Unreachable`](AdapterError::Unreachable) because it is the one
    /// failure that is usually about load rather than about configuration, and so the one
    /// worth retrying.
    Timeout {
        /// Which service.
        service: String,
        /// How long it was given.
        after_ms: u64,
    },
}

impl fmt::Display for AdapterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unreachable { service, detail } => {
                write!(f, "could not reach {service}: {detail}")
            }
            Self::Service { status, body } => write!(f, "the service answered {status}: {body}"),
            Self::UnsupportedQuery { report } => {
                let findings: Vec<String> = report
                    .not_pushed()
                    .map(std::string::ToString::to_string)
                    .collect();
                write!(
                    f,
                    "the service cannot answer this query: {}",
                    findings.join("; ")
                )
            }
            Self::Auth {
                profile: Some(profile),
            } => write!(f, "authentication failed for profile `{profile}`"),
            Self::Auth { profile: None } => {
                f.write_str("the service requires authentication and no profile was configured")
            }
            Self::Malformed { detail } => write!(
                f,
                "the service sent something this adapter could not read: {detail}"
            ),
            Self::Timeout { service, after_ms } => {
                write!(f, "{service} did not answer within {after_ms}ms")
            }
        }
    }
}

impl AdapterError {
    /// One stable word for this failure, for the compact per-source map in a response.
    ///
    /// Separate from [`Display`](fmt::Display), which writes a sentence for a person.
    /// This is for a machine: it is the value in
    /// `{"sources": {"private-stac": "timeout"}}`, so it has to stay put even when the
    /// wording of the sentence changes.
    #[must_use]
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Unreachable { .. } => "unreachable",
            Self::Service { .. } => "service-error",
            Self::UnsupportedQuery { .. } => "unsupported-query",
            Self::Auth { .. } => "auth",
            Self::Malformed { .. } => "malformed",
            Self::Timeout { .. } => "timeout",
        }
    }
}

impl std::error::Error for AdapterError {}

/// What a protocol implementation has to provide.
///
/// Implement this in a crate of your own and the engine can use it: nothing here names a
/// concrete adapter, a transport, or a protocol. The `Send + Sync` bound is what lets the
/// engine fan out across sources concurrently, and `Debug` is what lets the structures
/// holding one stay printable.
#[async_trait]
pub trait ServiceAdapter: fmt::Debug + Send + Sync {
    /// A short, stable name, used in provenance and in error messages.
    fn name(&self) -> &str;

    /// Guess, from the endpoint alone, whether this is a service we can speak to.
    ///
    /// Synchronous and cheap by contract: a registry calls this on every adapter it has
    /// before it calls anything that touches a network. An adapter that needs to ask the
    /// service should answer [`Detection::Uncertain`] and leave the asking to
    /// [`describe`](ServiceAdapter::describe).
    fn detect(&self, endpoint: &Endpoint) -> Detection;

    /// Ask the service what it is and what it can do.
    async fn describe(&self, endpoint: &Endpoint) -> Result<ServiceDescriptor, AdapterError>;

    /// Run one query against one service.
    ///
    /// Takes the [`ServiceDescriptor`] rather than the [`Endpoint`] because a query
    /// cannot be planned without knowing what the service supports, and re-describing per
    /// query would make every search pay for discovery.
    async fn query(
        &self,
        service: &ServiceDescriptor,
        query: &GeoQuery,
    ) -> Result<QueryResult, AdapterError>;

    /// What this service can do, as the adapter understands it.
    ///
    /// Defaults to what the service declared. Worth overriding when the protocol implies
    /// more than the service says: every STAC search endpoint accepts a bounding box
    /// whether or not its landing page mentions one, and an adapter knowing that is how
    /// an under-described service stops being treated as an incapable one.
    fn capabilities(&self, service: &ServiceDescriptor) -> CapabilitySet {
        service.capabilities.clone().unwrap_or_default()
    }

    /// Whether this adapter can honour a query against a service, and what it would cost.
    ///
    /// Provided, not required: the answer follows from
    /// [`capabilities`](ServiceAdapter::capabilities), so an adapter gets it for free and
    /// overrides it only if it knows something the capability model cannot express.
    fn report(&self, service: &ServiceDescriptor, query: &GeoQuery) -> CapabilityReport {
        let mut described = service.clone();
        described.capabilities = Some(self.capabilities(service));
        CapabilityReport::for_query(query, &described)
    }
}
