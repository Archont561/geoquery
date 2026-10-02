//! N-API adapter for the TypeScript client.
//!
//! One function, taking and returning JSON. That is the whole surface on purpose: every
//! operation the engine has is an `Operation` in `geoquery-protocol`, so adding one is a
//! change to that enum and to the engine — not a new export in this file, a new signature in
//! Node's type definitions, and a new release of the addon to go with it. A binding that
//! mirrored the engine's function list would have to be rebuilt for every operation.

use napi_derive::napi;

/// Run one transport request and return one transport response, both as JSON text.
///
/// `String` in and `String` out rather than a decoded object: the addon does not get a say
/// in what crosses the boundary, because a binding that did would be a second place where
/// the wire format is written down.
#[napi]
#[must_use]
// `&str` is the signature this function wants — the engine takes a slice and nothing here
// owns the text — but napi-rs rejects it outright: a JavaScript string is primitive and
// cannot be lent to Rust. `String` is the only signature that compiles, so the lint is
// silenced here rather than in `[workspace.lints]`, where turning it off would also silence
// it for every crate that has a real ownership mistake. `allow` rather than `expect`
// because rustc counts the lint as suppressed rather than triggered when an expectation is
// in place, and `unfulfilled_lint_expectations` is a warning here too.
#[allow(clippy::needless_pass_by_value)]
pub fn invoke(request: String) -> String {
    geoquery_engine::invoke(&request)
}
