//! Is this a STAC API, where does it keep `/search`, and what does its `conformsTo` list
//! actually buy a caller.
//!
//! STAC API landing pages are a `Catalog` (or occasionally a bare `Collection`) with a
//! `conformsTo` array and a set of `links`. Nothing about that shape is specific to one
//! provider, which is the point: this module reads the standard rather than a particular
//! server's habits, so a service this adapter has never been pointed at before still
//! works the first time.
//!
//! Detection by `conformsTo` rather than by URL shape or status code is deliberate.
//! `https://example.org/stac`, `https://example.org/api/stac/v1` and
//! `https://example.org/v1` are all real STAC API base URLs in the wild — see
//! `.knowledge/research/open-service-targets.md` — and none of them contains a reliable
//! marker, while every conforming STAC API names itself in the one field the spec
//! requires.

use geoquery_types::{AxisOrder, CapabilitySet, CrsDescriptor, JsonObject, SpatialOperation};
use serde_json::Value;

/// Conformance URIs that mean "the STAC API core is implemented here", across the
/// `v1.0.0` and still-deployed `v1.0.0-rc.*` URI families.
fn declares_core(conforms_to: &[String]) -> bool {
    conforms_to
        .iter()
        .any(|class| class.contains("stacspec.org") && class.contains("/core"))
}

/// Whether the service advertises the item-search filter extension or a CQL2
/// conformance class, under any of the URI spellings different STAC API versions have
/// shipped it under.
fn declares_filter(conforms_to: &[String]) -> bool {
    conforms_to
        .iter()
        .any(|class| class.contains("item-search#filter") || class.contains("cql2"))
}

/// Whether the service advertises the item-search sort extension.
fn declares_sort(conforms_to: &[String]) -> bool {
    conforms_to
        .iter()
        .any(|class| class.contains("item-search#sort"))
}

/// What this adapter learned from a landing page, before anything has been translated
/// into a [`geoquery_types::ServiceDescriptor`].
///
/// A separate type from `ServiceDescriptor` rather than building one directly, so that
/// [`parse_landing`] stays a pure function a test can call without constructing an
/// `Endpoint` or an HTTP client — see `tests/landing.rs`.
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedLanding {
    /// Absolute URL of the item-search endpoint, taken from the `search` link when the
    /// landing page names one and guessed as `{base}/search` when it does not.
    pub search_url: String,
    /// Absolute URL of the collections endpoint, taken from the `data` link when the
    /// landing page names one and guessed as `{base}/collections` when it does not.
    pub collections_url: String,
    /// What the service declares it can do.
    ///
    /// This is the service's own claim, not a statement about what this adapter version
    /// implements — a server offering CQL2 filtering still gets `attribute: Some(true)`
    /// here even though [`crate::request::translate`] never sends a filter. The two are
    /// kept apart so `geoquery source describe` can show a user what a future adapter
    /// version could reach, without this one pretending to reach it already.
    pub capabilities: CapabilitySet,
    /// Everything else worth keeping from the landing page: the raw `conformsTo` list,
    /// the resolved `search` and `collections` URLs, and the declared `type`.
    pub metadata: JsonObject,
}

/// Read a landing page response, or say why it is not a STAC API's.
///
/// # Errors
///
/// A message naming what was found instead, for [`crate::adapter::StacAdapter::describe`]
/// to wrap in [`geoquery_core::AdapterError::Malformed`].
pub fn parse_landing(base_url: &str, body: &Value) -> Result<ParsedLanding, String> {
    let object = body
        .as_object()
        .ok_or_else(|| format!("{base_url} did not answer with a JSON object"))?;

    let kind = object.get("type").and_then(Value::as_str).unwrap_or("");
    let conforms_to: Vec<String> = object
        .get("conformsTo")
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(|value| value.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default();

    if !matches!(kind, "Catalog" | "Collection") || !declares_core(&conforms_to) {
        return Err(format!(
            "{base_url} does not look like a STAC API: type is `{kind}` and conformsTo \
             does not list the STAC API core conformance class"
        ));
    }

    let links = object.get("links").and_then(Value::as_array);
    let search_url = find_link(links, "search")
        .unwrap_or_else(|| format!("{}/search", base_url.trim_end_matches('/')));
    let collections_url = find_link(links, "data")
        .unwrap_or_else(|| format!("{}/collections", base_url.trim_end_matches('/')));

    let capabilities = CapabilitySet {
        // STAC API's core conformance class guarantees both `bbox` and `intersects` on
        // `/search`; neither is a separate conformance class to check for.
        spatial: vec![SpatialOperation::Bbox, SpatialOperation::Intersects],
        bbox: Some(true),
        geometry_filter: Some(true),
        temporal: Some(true),
        pagination: Some(true),
        attribute: Some(declares_filter(&conforms_to)),
        sorting: Some(declares_sort(&conforms_to)),
        crs: vec![CrsDescriptor {
            code: "OGC:CRS84".to_owned(),
            axis_order: AxisOrder::LonLat,
        }],
        formats: vec!["application/geo+json".to_owned()],
        ..CapabilitySet::default()
    };

    let mut metadata = JsonObject::new();
    metadata.insert("landingType".to_owned(), Value::String(kind.to_owned()));
    metadata.insert("searchUrl".to_owned(), Value::String(search_url.clone()));
    metadata.insert(
        "collectionsUrl".to_owned(),
        Value::String(collections_url.clone()),
    );
    metadata.insert(
        "conformsTo".to_owned(),
        Value::Array(conforms_to.into_iter().map(Value::String).collect()),
    );

    Ok(ParsedLanding {
        search_url,
        collections_url,
        capabilities,
        metadata,
    })
}

/// The `href` of the first link with the given `rel`, from a landing page's `links` array.
///
/// First rather than best: a STAC API that lists `search` twice — once per HTTP method,
/// which Earth Search and Planetary Computer both do — points both links at the same
/// URL, so there is nothing to choose between them.
fn find_link(links: Option<&Vec<Value>>, rel: &str) -> Option<String> {
    links?.iter().find_map(|link| {
        let object = link.as_object()?;
        if object.get("rel").and_then(Value::as_str) != Some(rel) {
            return None;
        }
        object
            .get("href")
            .and_then(Value::as_str)
            .map(str::to_owned)
    })
}

/// The `id` of every collection in a `/collections` response.
///
/// Returns an empty list for a response that is not a collections document rather than
/// an error: a service that answers `/collections` with something this adapter cannot
/// read still has a working `/search`, and [`crate::adapter::StacAdapter::describe`]
/// treats an empty collection list as "none discovered", not as a reason to fail
/// registration.
#[must_use]
pub fn collection_ids(body: &Value) -> Vec<String> {
    body.get("collections")
        .and_then(Value::as_array)
        .map(|collections| {
            collections
                .iter()
                .filter_map(|collection| collection.get("id").and_then(Value::as_str))
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}
