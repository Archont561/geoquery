//! `StacAdapter`: the live HTTP glue around [`crate::landing`], [`crate::request`] and
//! [`crate::response`].
//!
//! Everything that decides *what* to send or *how* to read an answer lives in the three
//! sibling modules, all of which are plain functions over JSON values — this module is
//! only the part that actually owns an HTTP client and knows how a `reqwest::Error`
//! becomes an [`AdapterError`].

use std::time::{Duration, Instant};

use async_trait::async_trait;
use chrono::Utc;
use geoquery_core::{AdapterError, Confidence, Detection, Endpoint, QueryResult, ServiceAdapter};
use geoquery_types::{GeoQuery, Provenance, ServiceDescriptor, ServiceType};
use reqwest::Client;
use serde_json::Value;

use crate::landing::{self, ParsedLanding};
use crate::request::translate;
use crate::response::normalize_response;

/// Per-request timeout. A narrow spike needs one sane number rather than a configuration
/// surface; `Endpoint` already carries the hooks (`auth_profile`, `headers`) a later
/// version would extend to cover it per source.
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(20);

/// Hostnames this adapter recognises on sight, from
/// `.knowledge/research/open-service-targets.md`'s STAC API table.
///
/// [`detect`](StacAdapter::detect) is a hint, not a verdict — [`describe`] is what
/// actually confirms a service — so this list exists to make `geoquery source add`
/// slightly more confident about a URL it already knows, not to gate what can be added.
const KNOWN_STAC_HOSTS: [&str; 4] = [
    "earth-search.aws.element84.com",
    "planetarycomputer.microsoft.com",
    "landsatlook.usgs.gov",
    "stac.dataspace.copernicus.eu",
];

/// A geoquery source adapter for STAC API.
#[derive(Debug)]
pub struct StacAdapter {
    client: Client,
}

impl Default for StacAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl StacAdapter {
    /// Build an adapter with the one HTTP client it reuses for every request.
    ///
    /// # Panics
    ///
    /// Never in practice: the client configuration here (timeout and user agent) is
    /// static and always valid. `reqwest::ClientBuilder::build` can fail on a platform
    /// unable to initialise TLS at all, which is a startup-time environment problem, not
    /// a value this constructor could be given wrong.
    #[must_use]
    pub fn new() -> Self {
        let client = Client::builder()
            .user_agent(geoquery_core::user_agent())
            .timeout(DEFAULT_TIMEOUT)
            .build()
            .expect("a minimal reqwest client always builds");
        Self { client }
    }

    /// `GET` a URL and parse the body as JSON, translating transport and status failures
    /// into the matching [`AdapterError`]. The URL doubles as the error label: `describe`
    /// runs before a source has a registered name, so the URL is the only thing worth
    /// naming it by.
    async fn get_json(&self, url: &str) -> Result<Value, AdapterError> {
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|error| transport_error(url, &error))?;
        read_json(response).await
    }

    /// `POST` a JSON body to a URL and parse the response as JSON, labelling any failure
    /// with the registered source name rather than its URL.
    async fn post_json(&self, label: &str, url: &str, body: &Value) -> Result<Value, AdapterError> {
        let response = self
            .client
            .post(url)
            .json(body)
            .send()
            .await
            .map_err(|error| transport_error(label, &error))?;
        read_json(response).await
    }
}

#[async_trait]
impl ServiceAdapter for StacAdapter {
    fn name(&self) -> &'static str {
        "stac"
    }

    fn detect(&self, endpoint: &Endpoint) -> Detection {
        let lower = endpoint.url.to_ascii_lowercase();
        if KNOWN_STAC_HOSTS.iter().any(|host| lower.contains(host)) {
            // Confident, not certain: a URL on a known host is still unconfirmed until
            // `describe` asks it. `Confidence::CERTAIN` is reserved for the case the
            // trait docs describe — an endpoint that announced itself.
            return Detection::Match {
                service_type: ServiceType::Stac,
                confidence: Confidence::new(0.8).expect("0.8 is within 0.0..=1.0"),
            };
        }
        if lower.contains("stac") {
            return Detection::Uncertain {
                reason: "the URL mentions STAC but this adapter has not confirmed it by \
                         asking the service"
                    .to_owned(),
            };
        }
        Detection::NoMatch
    }

    async fn describe(&self, endpoint: &Endpoint) -> Result<ServiceDescriptor, AdapterError> {
        let landing_body = self.get_json(&endpoint.url).await?;
        let ParsedLanding {
            search_url,
            collections_url,
            capabilities,
            metadata,
        } = landing::parse_landing(&endpoint.url, &landing_body)
            .map_err(|detail| AdapterError::Malformed { detail })?;

        let collections_body = self.get_json(&collections_url).await?;
        let collections = landing::collection_ids(&collections_body);

        let mut descriptor = ServiceDescriptor::new(ServiceType::Stac, endpoint.url.clone());
        descriptor.capabilities = Some(capabilities);
        descriptor.collections = collections;
        descriptor.metadata = metadata;
        let _ = search_url; // kept on the descriptor via `metadata["searchUrl"]`.
        Ok(descriptor)
    }

    async fn query(
        &self,
        service: &ServiceDescriptor,
        query: &GeoQuery,
    ) -> Result<QueryResult, AdapterError> {
        let label = service.id.clone().unwrap_or_else(|| service.url.clone());
        let search_url = service
            .metadata
            .get("searchUrl")
            .and_then(Value::as_str)
            .map_or_else(
                || format!("{}/search", service.url.trim_end_matches('/')),
                str::to_owned,
            );

        let (request, findings) = translate(query);
        let request_object = request.to_json_object();
        let body = Value::Object(request_object.clone());

        let started = Instant::now();
        let payload = self.post_json(&label, &search_url, &body).await?;
        let duration_ms = u64::try_from(started.elapsed().as_millis()).ok();
        let timestamp = Utc::now();

        let normalized = normalize_response(
            &payload,
            &label,
            &service.url,
            &request_object,
            timestamp,
            duration_ms,
        )
        .map_err(|error| AdapterError::Malformed {
            detail: error.to_string(),
        })?;

        if normalized.skipped > 0 {
            tracing::warn!(
                source = %label,
                skipped = normalized.skipped,
                "dropped STAC items with no usable id",
            );
        }

        let mut provenance = Provenance::new(
            label,
            service.url.clone(),
            ServiceType::Stac,
            request_object,
            timestamp,
        );
        provenance.duration_ms = duration_ms;

        // `degradations` is documented as "every part of the query that did not reach the
        // service as written" — the pushed half of `findings` already shows up in
        // `provenance.query` as the request that was actually sent, so it is dropped here
        // rather than duplicated.
        let degradations = findings
            .into_iter()
            .filter(|finding| finding.support != geoquery_core::Support::Pushed)
            .collect();

        Ok(QueryResult {
            provenance,
            results: normalized.results,
            degradations,
            next_page: normalized.next_page,
        })
    }

    fn capabilities(&self, service: &ServiceDescriptor) -> geoquery_types::CapabilitySet {
        service.capabilities.clone().unwrap_or_default()
    }
}

/// Read a response as JSON, or turn a non-2xx status into [`AdapterError::Service`].
async fn read_json(response: reqwest::Response) -> Result<Value, AdapterError> {
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(AdapterError::Service {
            status: status.as_u16(),
            body,
        });
    }
    response
        .json()
        .await
        .map_err(|error| AdapterError::Malformed {
            detail: error.to_string(),
        })
}

/// Classify a `reqwest::Error` the way [`AdapterError`] distinguishes failures: a timeout
/// is worth retrying, anything else reaching this function means the service could not be
/// reached at all.
fn transport_error(label: &str, error: &reqwest::Error) -> AdapterError {
    if error.is_timeout() {
        AdapterError::Timeout {
            service: label.to_owned(),
            after_ms: u64::try_from(DEFAULT_TIMEOUT.as_millis()).unwrap_or(u64::MAX),
        }
    } else {
        AdapterError::Unreachable {
            service: label.to_owned(),
            detail: error.to_string(),
        }
    }
}
