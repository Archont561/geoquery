//! Mirrors `src/lib.rs`, which in Phase 0 is documentation with no code behind it yet.
//!
//! The file exists so the mirror is complete: one test file per source file, so that the first
//! test of the code generator has an obvious home rather than a decision attached to it.
//!
//! The one thing it can assert today is that the crate links from outside its own build, which
//! is not what `cargo build` proves for a library that nothing in the workspace depends on yet
//! — a crate can compile and still fail to be usable as a dependency.

#[test]
fn the_crate_links_from_outside_its_own_build() {
    use geoquery_xtask as _;
}
