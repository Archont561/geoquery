//! STAC `Item` / `ItemCollection` → `GeoResult`, with provenance attached and the raw
//! item preserved.
//!
//! Normalization here is deliberately permissive about what it does not understand and
//! strict about the one thing it must have: every `GeoResult` carries an `id`, so a
//! feature this module cannot even name is a feature it refuses rather than returns
//! half-built. Everything else — geometry, a bounding box, a temporal extent, assets,
//! links — is optional on both sides, and an absent or malformed optional field is simply
//! left out of the normalized result rather than failing the whole response.

use std::fmt;

use chrono::{DateTime, Utc};
use geoquery_types::{
    Asset, BoundingBox, GeoResult, JsonObject, Link, Provenance, ResourceRef, ResultType,
    ServiceRef, ServiceType, TemporalExtent,
};
use serde_json::Value;

/// Why a STAC response could not be turned into `GeoResult`s at all.
///
/// Covers the response as a whole; a single malformed feature inside an otherwise good
/// `FeatureCollection` is not represented here; see [`normalize_response`]'s docs for why
/// that case is counted rather than rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NormalizationError {
    /// The response body is not a JSON object at all.
    NotAnObject,
    /// The response is a JSON object, but not a `Feature` or `FeatureCollection`.
    UnexpectedType {
        /// The `type` member found instead, or `"(missing)"` if there was none.
        found: String,
    },
}

impl fmt::Display for NormalizationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotAnObject => f.write_str("the search response is not a JSON object"),
            Self::UnexpectedType { found } => {
                write!(
                    f,
                    "expected a STAC Feature or FeatureCollection, found `{found}`"
                )
            }
        }
    }
}

impl std::error::Error for NormalizationError {}

/// The outcome of normalizing one STAC response: the results that parsed, and a count of
/// the features that did not.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct NormalizedResponse {
    /// Every feature that had at least an `id`, normalized.
    pub results: Vec<GeoResult>,
    /// How many entries in `features` had no usable `id` and were dropped.
    ///
    /// Counted rather than silently discarded: a response that drops items is a response
    /// worth noticing, even though dropping an unidentifiable item is the only safe thing
    /// to do with one — `GeoResult::id` is not optional, by the design in
    /// `crates/types/src/result.rs`.
    pub skipped: usize,
    /// The opaque pagination token from the response's own `next` link, if it offered
    /// one.
    pub next_page: Option<String>,
}

/// Normalize a STAC search response into `GeoResult`s.
///
/// Accepts a bare `Feature` as well as a `FeatureCollection`: `GET /search` against a
/// single matching item is rare but legal, and a caller should not have to branch on
/// which shape came back.
///
/// # Errors
///
/// [`NormalizationError`] if the response is not JSON, or is JSON that is neither shape.
pub fn normalize_response(
    body: &Value,
    source_id: &str,
    service_url: &str,
    request: &JsonObject,
    timestamp: DateTime<Utc>,
    duration_ms: Option<u64>,
) -> Result<NormalizedResponse, NormalizationError> {
    let object = body.as_object().ok_or(NormalizationError::NotAnObject)?;
    let features: Vec<&Value> = match object.get("type").and_then(Value::as_str) {
        Some("FeatureCollection") => object
            .get("features")
            .and_then(Value::as_array)
            .map(|features| features.iter().collect())
            .unwrap_or_default(),
        Some("Feature") => vec![body],
        other => {
            return Err(NormalizationError::UnexpectedType {
                found: other.unwrap_or("(missing)").to_owned(),
            });
        }
    };

    let mut results = Vec::with_capacity(features.len());
    let mut skipped = 0;
    for feature in features {
        match normalize_item(
            feature,
            source_id,
            service_url,
            request,
            timestamp,
            duration_ms,
        ) {
            Some(result) => results.push(result),
            None => skipped += 1,
        }
    }

    Ok(NormalizedResponse {
        results,
        skipped,
        next_page: next_link(object),
    })
}

/// The `href` of the response's `next` link, STAC API's pagination mechanism.
///
/// Surfaced opaquely rather than followed: this spike returns one page per query, and a
/// caller that wants more sends `next_page` back as a request of its own choosing rather
/// than this adapter deciding how many pages "enough" is.
fn next_link(object: &serde_json::Map<String, Value>) -> Option<String> {
    object.get("links")?.as_array()?.iter().find_map(|link| {
        let link = link.as_object()?;
        if link.get("rel").and_then(Value::as_str) != Some("next") {
            return None;
        }
        link.get("href").and_then(Value::as_str).map(str::to_owned)
    })
}

/// Normalize one STAC `Feature`, or `None` if it has no usable `id`.
fn normalize_item(
    item: &Value,
    source_id: &str,
    service_url: &str,
    request: &JsonObject,
    timestamp: DateTime<Utc>,
    duration_ms: Option<u64>,
) -> Option<GeoResult> {
    let id = item.get("id").and_then(Value::as_str)?.to_owned();
    let collection = item
        .get("collection")
        .and_then(Value::as_str)
        .map(str::to_owned);
    let properties = item
        .get("properties")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    let mut provenance = Provenance::new(
        source_id,
        service_url,
        ServiceType::Stac,
        request.clone(),
        timestamp,
    );
    provenance.collection.clone_from(&collection);
    provenance.duration_ms = duration_ms;

    let mut result = GeoResult::new(id, ResultType::Feature, provenance);
    result.resource = collection.map(|id| ResourceRef { id, title: None });
    result.service = Some(ServiceRef {
        id: Some(source_id.to_owned()),
        url: service_url.to_owned(),
        r#type: ServiceType::Stac,
    });
    result.title = properties
        .get("title")
        .and_then(Value::as_str)
        .map(str::to_owned);
    result.geometry = item
        .get("geometry")
        .cloned()
        .filter(|value| !value.is_null());
    result.bbox = item.get("bbox").and_then(parse_bbox);
    result.temporal = temporal_extent(&properties);
    result.assets = item
        .get("assets")
        .and_then(Value::as_object)
        .map(normalize_assets)
        .unwrap_or_default();
    result.links = item
        .get("links")
        .and_then(Value::as_array)
        .map(|links| normalize_links(links.as_slice()))
        .unwrap_or_default();
    result.properties = properties;
    // The source payload, verbatim: normalization above is necessarily incomplete — a
    // STAC item carries extensions this adapter has never heard of — and `raw` is the
    // escape hatch the design corpus asks for rather than a debugging convenience.
    result.raw = Some(item.clone());

    Some(result)
}

fn parse_bbox(value: &Value) -> Option<BoundingBox> {
    let ordinates = value.as_array()?;
    let [west, south, east, north] = <[Value; 4]>::try_from(ordinates.clone()).ok()?;
    Some([
        west.as_f64()?,
        south.as_f64()?,
        east.as_f64()?,
        north.as_f64()?,
    ])
}

/// A result's temporal extent, preferring an explicit `start_datetime`/`end_datetime`
/// pair — which a STAC item carries instead of `datetime` when it covers an interval
/// rather than an instant — and falling back to `datetime` as a degenerate interval whose
/// bounds are equal.
fn temporal_extent(properties: &JsonObject) -> Option<TemporalExtent> {
    let start = parse_instant_field(properties, "start_datetime");
    let end = parse_instant_field(properties, "end_datetime");
    if start.is_some() || end.is_some() {
        return Some(TemporalExtent { start, end });
    }
    let instant = parse_instant_field(properties, "datetime")?;
    Some(TemporalExtent {
        start: Some(instant),
        end: Some(instant),
    })
}

fn parse_instant_field(properties: &JsonObject, field: &str) -> Option<DateTime<Utc>> {
    let text = properties.get(field)?.as_str()?;
    DateTime::parse_from_rfc3339(text)
        .ok()
        .map(|instant| instant.with_timezone(&Utc))
}

/// A STAC item's `assets` object, keyed by asset name, into the `Vec<Asset>` `GeoResult`
/// expects.
///
/// `properties` is a `serde_json::Map`, a `BTreeMap` in this workspace (the
/// `preserve_order` feature is not enabled — see `Cargo.toml`), so iterating it already
/// visits assets in a fixed, alphabetical-by-key order; nothing here needs to sort to make
/// that true.
fn normalize_assets(raw: &serde_json::Map<String, Value>) -> Vec<Asset> {
    raw.iter()
        .filter_map(|(key, value)| {
            let object = value.as_object()?;
            let href = object.get("href").and_then(Value::as_str)?.to_owned();
            let roles = object
                .get("roles")
                .and_then(Value::as_array)
                .map(|roles| {
                    roles
                        .iter()
                        .filter_map(|role| role.as_str().map(str::to_owned))
                        .collect()
                })
                .unwrap_or_default();
            let mut metadata = object.clone();
            metadata.remove("href");
            metadata.remove("type");
            metadata.remove("title");
            metadata.remove("roles");
            // The STAC asset *key* ("blue", "thumbnail", ...) is part of what identifies
            // the asset and `Asset` has no field for it, so it travels in `metadata`
            // rather than being dropped on the floor.
            metadata.insert("key".to_owned(), Value::String(key.clone()));
            Some(Asset {
                href,
                media_type: object
                    .get("type")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                title: object
                    .get("title")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                roles,
                metadata,
            })
        })
        .collect()
}

fn normalize_links(raw: &[Value]) -> Vec<Link> {
    raw.iter()
        .filter_map(|link| {
            let object = link.as_object()?;
            Some(Link {
                href: object.get("href").and_then(Value::as_str)?.to_owned(),
                rel: object.get("rel").and_then(Value::as_str)?.to_owned(),
                media_type: object
                    .get("type")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                title: object
                    .get("title")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
            })
        })
        .collect()
}
