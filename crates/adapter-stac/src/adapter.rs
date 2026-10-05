//! `StacAdapter`: the live HTTP glue around [`crate::landing`], [`crate::request`] and
//! [`crate::response`].
//!
//! Everything that decides *what* to send or *how* to read an answer lives in the three
//! sibling modules, all of which are plain functions over JSON values — this module is
//! only the part that actually owns an HTTP client and knows how a `reqwest::Error`
//! becomes an [`AdapterError`].

use std::time::{Duration, Instant};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use geoquery_core::{AdapterError, Confidence, Detection, Endpoint, QueryResult, ServiceAdapter};
use geoquery_types::{GeoQuery, GeoResult, JsonObject, Provenance, ServiceDescriptor, ServiceType};
use reqwest::Client;
use serde_json::Value;

use crate::landing::{self, ParsedLanding, StacConformance};
use crate::request::translate;
use crate::response::{NextLink, normalize_response, project_fields};

/// Per-request timeout. A narrow spike needs one sane number rather than a configuration
/// surface; `Endpoint` already carries the hooks (`auth_profile`, `headers`) a later
/// version would extend to cover it per source.
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(20);

/// How many pages one query may fetch.
///
/// A bound rather than a tuning knob. STAC's `next` link is the service's own statement
/// that more exists, and a client that trusts it unconditionally has written a loop whose
/// length a remote service chooses — including the degenerate case of a server whose
/// `next` link points back at the same page. Ten pages is far past what a `limit` the
/// size of a human request needs and far short of a catalogue scan; when the cap stops a
/// run early, the unused `next` link is still returned in
/// [`QueryResult::next_page`], so the result says that it is partial instead of
/// pretending to be the whole answer.
const MAX_PAGES: usize = 10;

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

    /// Search, then follow the service's own `next` links until the caller's window is
    /// filled.
    ///
    /// `wanted` is how many items have to arrive before the window can be cut —
    /// `offset + limit`, or `None` for a query that named no limit. `None` fetches
    /// exactly one page: a query without a limit is a request for what the service
    /// returns by default, not an instruction to walk a catalogue, and the `next` link
    /// comes back untouched for a caller who wants the next page to ask for it.
    async fn fetch_pages(
        &self,
        context: &PageContext<'_>,
        search_url: &str,
        body: &Value,
        wanted: Option<usize>,
    ) -> Result<FetchedPages, AdapterError> {
        let PageContext {
            label,
            service_url,
            request,
            timestamp,
        } = *context;
        let mut fetched = FetchedPages::default();
        let mut page = PageRequest::post(search_url, body.clone());

        for _ in 0..MAX_PAGES {
            let payload = match &page {
                PageRequest::Post { url, body } => self.post_json(label, url, body).await?,
                PageRequest::Get { url } => self.get_json(url).await?,
            };
            // Each page is normalized against the request that *started* the run, not
            // against the page-token request that fetched it: provenance records the
            // question a user asked, and `{"token": "next:…"}` is not that question.
            let normalized =
                normalize_response(&payload, label, service_url, request, timestamp, None)
                    .map_err(|error| AdapterError::Malformed {
                        detail: error.to_string(),
                    })?;

            fetched.results.extend(normalized.results);
            fetched.skipped += normalized.skipped;
            fetched.next = normalized.next;
            fetched.pages += 1;

            let enough = wanted.is_none_or(|wanted| fetched.results.len() >= wanted);
            let Some(next) = fetched.next.as_ref().filter(|_| !enough) else {
                break;
            };
            let Some(following) = PageRequest::following(next, body) else {
                // A `next` link this adapter cannot turn into a request — no `href`, or a
                // method it does not speak — ends the run with the link still reported,
                // rather than failing a query that has already returned real results.
                tracing::debug!(source = %label, method = %next.method, "unfollowable next link");
                break;
            };
            page = following;
        }

        if fetched.pages >= MAX_PAGES && fetched.next.is_some() {
            tracing::warn!(
                source = %label,
                pages = fetched.pages,
                "stopped at the page cap with more pages advertised; the result is one \
                 page sequence, not the whole answer",
            );
        }

        Ok(fetched)
    }
}

/// What every page of one query shares: who is being asked, and the question that was
/// put.
///
/// Grouped rather than passed one by one, because these four travel together through the
/// page loop and into every page's normalization — and because a function that takes
/// seven positional arguments invites the call site that gets two of them the wrong way
/// round.
#[derive(Debug, Clone, Copy)]
struct PageContext<'a> {
    /// The registered source name, or its URL when it has no name yet.
    label: &'a str,
    /// The service's base URL, as recorded on every result's provenance.
    service_url: &'a str,
    /// The item-search request that started the run, as recorded on every result's
    /// provenance.
    request: &'a JsonObject,
    /// When the run started.
    timestamp: DateTime<Utc>,
}

/// Everything one `fetch_pages` run collected.
#[derive(Debug, Default)]
struct FetchedPages {
    /// Normalized items, in the order the pages returned them.
    results: Vec<GeoResult>,
    /// How many items were dropped for having no usable `id`, across every page.
    skipped: usize,
    /// The last page's `next` link, if it had one. Present when the run stopped because
    /// it had enough or hit [`MAX_PAGES`]; absent when the service said there is no more.
    next: Option<NextLink>,
    /// How many pages were actually requested.
    pages: usize,
}

/// One HTTP request for one page of results.
#[derive(Debug)]
enum PageRequest {
    /// The item-search request this adapter builds itself, and any `next` link that asks
    /// to be followed with a body.
    Post {
        /// Where to send it.
        url: String,
        /// What to send.
        body: Value,
    },
    /// A `next` link whose href carries the whole request.
    Get {
        /// Where to send it.
        url: String,
    },
}

impl PageRequest {
    fn post(url: &str, body: Value) -> Self {
        Self::Post {
            url: url.to_owned(),
            body,
        }
    }

    /// The request a `next` link asks for, or `None` if this adapter cannot make it.
    ///
    /// The STAC API pagination rules, followed literally: a `POST` link's `body` either
    /// replaces the original request or — when `merge` is true, which is what every
    /// pgstac-backed service sends — is laid over it, so that the page token travels with
    /// the filters that produced the first page rather than instead of them.
    fn following(next: &NextLink, original: &Value) -> Option<Self> {
        if next.href.is_empty() {
            return None;
        }
        match next.method.as_str() {
            "GET" => Some(Self::Get {
                url: next.href.clone(),
            }),
            "POST" => {
                let body = match (&next.body, next.merge) {
                    (Some(members), true) => {
                        let mut merged = original.as_object().cloned().unwrap_or_default();
                        for (name, value) in members {
                            merged.insert(name.clone(), value.clone());
                        }
                        Value::Object(merged)
                    }
                    (Some(members), false) => Value::Object(members.clone()),
                    // A POST link with no body of its own is the original request again,
                    // which would fetch the same page forever. `MAX_PAGES` would stop it;
                    // refusing to start is cheaper and says why in the log.
                    (None, _) => return None,
                };
                Some(Self::Post {
                    url: next.href.clone(),
                    body,
                })
            }
            _ => None,
        }
    }
}

/// How many items have to arrive before the caller's window can be cut out of them.
///
/// `offset + limit`, because STAC counts from the beginning of the result set; `None`
/// when the query named no limit, which means one page and no following.
fn window(query: &GeoQuery) -> Option<usize> {
    let limit = query.limit? as usize;
    Some(limit + query.offset.unwrap_or(0) as usize)
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
            // Not stored separately: `metadata["conformsTo"]` is the service's own words,
            // and `StacConformance::for_service` re-reads them at query time, so a second
            // copy here would be a second thing to keep in step.
            conformance: _,
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

        let (request, findings) = translate(query, StacConformance::for_service(service));
        let request_object = request.to_json_object();
        let body = Value::Object(request_object.clone());

        let started = Instant::now();
        let timestamp = Utc::now();
        let context = PageContext {
            label: &label,
            service_url: &service.url,
            request: &request_object,
            timestamp,
        };
        let fetched = self
            .fetch_pages(&context, &search_url, &body, window(query))
            .await?;
        let duration_ms = u64::try_from(started.elapsed().as_millis()).ok();

        if fetched.skipped > 0 {
            tracing::warn!(
                source = %label,
                skipped = fetched.skipped,
                "dropped STAC items with no usable id",
            );
        }

        // The window the caller asked for, cut out of the pages that arrived. STAC has no
        // skip count, so `offset` is served here and nowhere else — and
        // `request::translate` reported it as `Support::Local` precisely because this is
        // the line that does it.
        let mut results = fetched.results;
        let offset = query.offset.unwrap_or(0) as usize;
        if offset > 0 {
            results.drain(..offset.min(results.len()));
        }
        if let Some(limit) = query.limit.map(|limit| limit as usize) {
            results.truncate(limit);
        }
        project_fields(&mut results, &query.fields);
        // One duration for the whole fetch rather than one per page: the question a reader
        // of provenance is asking is how long this source took to answer, and a per-page
        // figure for a result that took three pages to assemble answers a question nobody
        // put.
        for result in &mut results {
            result.provenance.duration_ms = duration_ms;
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
            results,
            degradations,
            next_page: fetched.next.map(|next| next.href),
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
