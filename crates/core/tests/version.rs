//! Mirrors `src/version.rs`: what this build says it speaks, and how it says it.
//!
//! An integration test rather than a `#[cfg(test)] mod tests`, because every item under
//! test is part of the published surface and reaching it the way a dependent crate does
//! is what makes the test an assertion about the interface rather than about today's
//! module layout.

use geoquery_core::{PROTOCOL_VERSION, VERSION, user_agent};

#[test]
fn the_client_identifies_itself_with_both_numbers() {
    // Written out rather than rebuilt from the same `format!` the function uses. A test
    // that recomputes the answer the way the code computes it cannot disagree with the
    // code about whether the format is *right* — only notice that it changed, and the
    // repair for that is always to paste the new string in, which is no assertion at all.
    //
    // `crates/cli/tests/main.rs` pins `geoquery 0.1.0` for the same reason and bumps with
    // it; a literal that has to be updated deliberately is the point, not the cost.
    assert_eq!(
        user_agent(),
        "geoquery/0.1.0 (query protocol 0.1.0)",
        "bump pixi.toml and every manifest that inherits from it, together"
    );
}

#[test]
fn the_user_agent_names_the_protocol_and_not_only_the_build() {
    // Why the header carries two numbers. An operator reading a server log needs to know
    // which protocol version sent the request; the build number cannot answer that once
    // the two are allowed to diverge, and the label is what makes the second one legible.
    let agent = user_agent();

    assert!(agent.contains("query protocol"), "{agent}");
    assert!(agent.contains(PROTOCOL_VERSION), "{agent}");
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
