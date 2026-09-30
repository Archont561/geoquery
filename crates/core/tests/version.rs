//! Mirrors `src/version.rs`: what this build says it speaks, and how it says it.
//!
//! An integration test rather than a `#[cfg(test)] mod tests`, because every item under
//! test is part of the published surface and reaching it the way a dependent crate does
//! is what makes the test an assertion about the interface rather than about today's
//! module layout.

use geoquery_core::{PROTOCOL_VERSION, VERSION, user_agent};

#[test]
fn the_client_identifies_itself_with_both_numbers() {
    // An operator reading a server log needs to know which protocol version sent the
    // request, not which build of the client it happened to come from.
    assert_eq!(
        user_agent(),
        format!("geoquery/{VERSION} (query protocol {PROTOCOL_VERSION})")
    );
    assert!(user_agent().contains(PROTOCOL_VERSION));
}

#[test]
fn the_version_is_a_release_number_and_not_a_placeholder() {
    assert_eq!(
        VERSION.split('.').count(),
        3,
        "the language version reaches users through `geoquery --version`: {VERSION}"
    );
    assert_eq!(
        PROTOCOL_VERSION, VERSION,
        "one number today; the commit that changes that changes this test with it"
    );
}
