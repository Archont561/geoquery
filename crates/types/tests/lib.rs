//! Mirrors `src/lib.rs` — which is the canonical query AST every other surface adapts to and, in Phase 0, documentation with no code
//! behind it yet.
//!
//! The file exists so the mirror is complete: one test file per source file, so the
//! first test of the canonical query AST every other surface adapts to has an obvious home rather than a decision attached to it. The
//! one thing it can assert today is that the crate links from outside its own build,
//! which is not what `cargo build` proves for a library that nothing in the workspace
//! depends on yet — a crate can compile and still fail to be usable as a dependency.

#[test]
fn the_crate_links_from_outside_its_own_build() {
    use geoquery_types as _;
}
