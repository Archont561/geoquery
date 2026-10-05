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

use geoquery_types::{
    AxisOrder, CapabilitySet, CrsDescriptor, JsonObject, ServiceDescriptor, SpatialOperation,
};
use serde_json::Value;

/// Conformance URIs that mean "the STAC API core is implemented here", across the
/// `v1.0.0` and still-deployed `v1.0.0-rc.*` URI families.
fn declares_core(conforms_to: &[String]) -> bool {
    conforms_to
        .iter()
        .any(|class| class.contains("stacspec.org") && class.contains("/core"))
}

/// What a service's `conformsTo` list says about the extensions this adapter can use.
///
/// One struct rather than a scatter of predicates, because the same answer is needed in
/// two places that must not disagree: [`parse_landing`] fills a
/// [`CapabilitySet`] from it at registration time, and [`crate::request::translate`] reads
/// it again at query time to decide what may be sent. The second reader works from the
/// `conformsTo` array [`parse_landing`] kept on the descriptor's `metadata`, so the
/// decision is made from the service's own words both times rather than from a boolean
/// somebody re-derived.
///
/// Each field is a *declaration*, not a capability this adapter has verified. An
/// undeclared extension is treated as absent — see [`geoquery_core::Cause::Undeclared`]
/// for why that is the safe direction: a service handed a parameter it does not implement
/// may ignore it rather than reject it, which turns a narrow question into a wide answer
/// with nothing in the response to show that it widened.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "these are six independent declarations a service either made or did not, \
              read off one list; clippy's suggested state machine would model them as \
              stages of one thing, which they are not — a service can declare any subset"
)]
pub struct StacConformance {
    /// The Filter extension is bound to item search (`…/item-search#filter`, or OGC API
    /// Features Part 3's `…/conf/filter`), under any version's spelling of either.
    pub filter: bool,
    /// CQL2 JSON is accepted. Required separately from [`Self::filter`]: the extension
    /// lets a service implement only `cql2-text`, and a `POST /search` carrying a JSON
    /// filter to such a service is a 400 at best.
    pub cql2_json: bool,
    /// The Basic CQL2 conformance class: comparison operators and boolean connectives.
    pub basic_cql2: bool,
    /// The Advanced Comparison Operators class, which is where CQL2 keeps `LIKE`, `IN`
    /// and `BETWEEN`. A service may declare [`Self::basic_cql2`] without this one, and
    /// Planetary Computer is exactly that service.
    pub advanced_comparison: bool,
    /// The Sort extension is bound to item search (`…/item-search#sort`).
    pub sort: bool,
    /// The Fields extension is bound to item search (`…/item-search#fields`).
    pub fields: bool,
}

impl StacConformance {
    /// Read a `conformsTo` list.
    ///
    /// Matching is by substring rather than by equality, for the same reason
    /// `declares_core` above is: the same conformance class ships under `v1.0.0`,
    /// `v1.0.0-rc.2` and `v1.1.0` URIs, all of which are live on real services today —
    /// Planetary Computer's filter binding is still the `rc.2` spelling — and a parser
    /// that listed exact URIs would quietly decide a conforming service is incapable
    /// every time a new version is published.
    #[must_use]
    pub fn from_classes(conforms_to: &[String]) -> Self {
        let any = |needle: &str| conforms_to.iter().any(|class| class.contains(needle));
        Self {
            filter: any("item-search#filter") || any("ogcapi-features-3/1.0/conf/filter"),
            cql2_json: any("cql2-json"),
            basic_cql2: any("basic-cql2"),
            advanced_comparison: any("advanced-comparison-operators"),
            sort: any("item-search#sort"),
            fields: any("item-search#fields"),
        }
    }

    /// Read the `conformsTo` list [`parse_landing`] recorded on a registered service.
    ///
    /// A descriptor with no recorded conformance — one written by an older build, or by
    /// something that is not this adapter — declares nothing, which is the same answer as
    /// a service that advertised no extensions. Both end with this adapter sending only
    /// what STAC API core guarantees.
    #[must_use]
    pub fn for_service(service: &ServiceDescriptor) -> Self {
        let classes: Vec<String> = service
            .metadata
            .get("conformsTo")
            .and_then(Value::as_array)
            .map(|values| {
                values
                    .iter()
                    .filter_map(|value| value.as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default();
        Self::from_classes(&classes)
    }

    /// Whether a CQL2 JSON filter using only basic operators may be sent.
    ///
    /// All three parts are needed and each says something different: that filtering is
    /// bound to the endpoint being called, that the JSON encoding is understood, and that
    /// the operators the encoding will carry are implemented.
    #[must_use]
    pub const fn pushes_basic_filter(self) -> bool {
        self.filter && self.cql2_json && self.basic_cql2
    }

    /// Whether the service also declared the class that `LIKE` and `IN` belong to.
    #[must_use]
    pub const fn pushes_advanced_filter(self) -> bool {
        self.pushes_basic_filter() && self.advanced_comparison
    }

    /// Whether the landing page named none of the extension classes this adapter reads.
    ///
    /// Not the same as "the service supports nothing": it is "the service said nothing
    /// this adapter recognises", which is the distinction
    /// [`geoquery_core::Cause::Undeclared`] draws against
    /// [`geoquery_core::Cause::Declined`].
    #[must_use]
    pub const fn declares_nothing(self) -> bool {
        !self.filter
            && !self.cql2_json
            && !self.basic_cql2
            && !self.advanced_comparison
            && !self.sort
            && !self.fields
    }

    /// Whether the service claims attribute filtering at all, in any encoding.
    ///
    /// Broader than [`Self::pushes_basic_filter`] on purpose: this is what
    /// [`CapabilitySet::attribute`] records, and that field is documented as the
    /// service's own claim rather than a statement about what this adapter version can
    /// reach. A service offering `cql2-text` only still filters; this adapter just cannot
    /// ask it to.
    #[must_use]
    pub const fn declares_filter(self) -> bool {
        self.filter || self.cql2_json
    }
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
    /// implements — a server offering `cql2-text` and nothing else still gets
    /// `attribute: Some(true)` here, even though [`crate::request::translate`] will not
    /// send it a filter, because this adapter speaks only CQL2 JSON. The two are kept
    /// apart so `geoquery source describe` can show a user what a future adapter version
    /// could reach, without this one pretending to reach it already; [`Self::conformance`]
    /// is the narrower answer translation actually acts on.
    pub capabilities: CapabilitySet,
    /// The extension classes the landing page declared, as this adapter reads them.
    ///
    /// Derived from the same `conformsTo` list that is kept verbatim in [`Self::metadata`],
    /// and recomputed from that list at query time by
    /// [`StacConformance::for_service`] — this field is the registration-time view of the
    /// same answer, so a test can assert on it without a round trip through a descriptor.
    pub conformance: StacConformance,
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

    let conformance = StacConformance::from_classes(&conforms_to);
    let capabilities = CapabilitySet {
        // STAC API's core conformance class guarantees both `bbox` and `intersects` on
        // `/search`; neither is a separate conformance class to check for.
        spatial: vec![SpatialOperation::Bbox, SpatialOperation::Intersects],
        bbox: Some(true),
        geometry_filter: Some(true),
        temporal: Some(true),
        pagination: Some(true),
        attribute: Some(conformance.declares_filter()),
        sorting: Some(conformance.sort),
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
        conformance,
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
