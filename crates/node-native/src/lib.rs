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
pub fn invoke(request: String) -> String {
    geoquery_engine::invoke(&request)
}