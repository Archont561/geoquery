//! What version this build speaks, and how it identifies itself to a service.

/// The crate's version, and the project's: checked against the root `pixi.toml` by
/// `pixi run version-check`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The version of the query protocol this build speaks.
///
/// A separate constant rather than a bare alias, because the two are allowed to diverge
/// and the moment they do is the moment a service needs to know which one its client
/// means. Today they are the same number, and
/// `the_version_the_binary_reports_is_the_language_version` in the CLI asserts that, so
/// the first commit that changes one without the other is a failing test rather than a
/// client nobody can identify.
pub const PROTOCOL_VERSION: &str = VERSION;

/// The `User-Agent` a service should see from this client.
///
/// A protocol implementation that does not identify itself is indistinguishable from
/// any other HTTP client on the network, and the first question an operator asks about
/// a slow or failing query is who sent it.
#[must_use]
pub fn user_agent() -> String {
    format!("geoquery/{VERSION} (query protocol {PROTOCOL_VERSION})")
}
