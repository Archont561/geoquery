//! The transport every geoquery binding speaks.
//!
//! A binding does not expose the engine's functions. It exposes one call that takes a
//! request and returns a response, both of them JSON, and this crate is what those two
//! shapes are. The point is that the *wire* is the interface, so adding an operation is a
//! change to one enum rather than a change to every FFI signature in every language, and a
//! binding that was compiled against an older version still parses a request it does not
//! understand an `operation` for.
//!
//! Numbers on this wire are 64-bit: a signed integer, an unsigned integer, or a double.
//! JSON itself bounds neither the magnitude nor the precision of a number, so that is a
//! promise of this transport rather than of the format, and it is the strongest promise
//! the transport can make — JavaScript's `JSON.parse` rounds an integer past 2^53 before
//! a request reaches the engine, so an exactness the TypeScript binding cannot honour
//! would not be one contract but three. [`EngineRequest::payload`] says what happens at
//! the edge, and `the_number_domain` in `tests/lib.rs` pins it.
//!
//! Two versions exist in this project and they are not the same thing, so they are not
//! given the same name. [`TRANSPORT_VERSION`] is this envelope's shape, below. The query
//! protocol's version is `geoquery_core::PROTOCOL_VERSION`, a semver string meaning "which
//! queries this engine understands". Calling the number below `PROTOCOL_VERSION` is how a
//! caller ends up sending `1` where `0.1.0` belongs.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The shape version of this envelope.
///
/// Bumped when a request or response field changes meaning — a new required field, a
/// changed default, a removed operation. Bumping it is the escape hatch that lets a new
/// engine keep serving old bindings: an engine that receives a version it does not know
/// answers `ok: false` rather than guessing, so a client written against a future
/// transport fails loudly against an old engine instead of silently reading the wrong
/// field.
///
/// Starts at 1 because this is the first envelope.
pub const TRANSPORT_VERSION: u32 = 1;

/// One request to the engine.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineRequest {
    /// The envelope shape the caller speaks. Checked against [`TRANSPORT_VERSION`].
    pub transport_version: u32,
    /// What to do.
    pub operation: Operation,
    /// The operation's arguments. Defaults to null so an argument-less operation is a
    /// request with one omitted field rather than a special-cased one.
    ///
    /// Any JSON value, within the 64-bit number domain described above. An integer too
    /// large for `i64` or `u64` is carried as the nearest double rather than refused:
    /// `serde_json` parses it that way, and the alternative — the `arbitrary_precision`
    /// feature — was measured and declined, because it makes every number a
    /// heap-allocated string including every coordinate of every geometry, to buy
    /// exactness for magnitudes no geospatial payload contains.
    #[serde(default)]
    pub payload: Value,
}

/// Everything the engine can be asked to do.
///
/// A closed enum on purpose. An unknown operation is a request that cannot be served, and
/// it should arrive as one — a string with a typo in it is a mistake a caller can read and
/// fix, whereas an open `Custom(String)` would turn every future engine operation into a
/// silent no-op for every binding compiled before it existed.
///
/// The set grows with the engine. Everything here is what `geoquery-core` can actually do
/// today; execution arrives as `Query` when it does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Operation {
    /// Echo the payload back with the engine's name. Proves the boundary is live.
    Ping,
    /// This build's version, the query protocol version it speaks, and its user-agent.
    ProtocolVersion,
    /// Parse a query document and report its sorted top-level keys.
    ParseDocument,
}

/// One response from the engine.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineResponse {
    /// Always this engine's [`TRANSPORT_VERSION`], even for a request that carried a
    /// different one — the answer is in this engine's dialect, whatever the question was
    /// written in.
    pub transport_version: u32,
    /// Whether the operation succeeded. On `false`, `result` holds the error.
    pub ok: bool,
    /// The operation's answer, or the error.
    pub result: Value,
}
