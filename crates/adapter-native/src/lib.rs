//! Sources that are not services: a directory of `GeoJSON`, a `GeoParquet` file, a
//! `resource.yaml` manifest.
//!
//! The one adapter with no network and no C dependency, which is what makes it the one a
//! test can rely on being able to run.
//!
//! `PROJ` and `GEOS` are absent, and that is a decision rather than an oversight. Both
//! are the right answer for CRS transforms and robust topology, both are C FFI, and both
//! are marked WASM-incompatible in `.knowledge/research/rust-crates.md` — so adopting them
//! now would put a C toolchain in front of every gate run and every sandbox restore to
//! serve a Phase 6 question. They arrive with the crate that answers it, and
//! `[package.build.config] compilers = ["c"]` in `../pixi.toml` is already what the
//! conda build needs when it does.
//!
//! Lands in Phase 2. Scaffolding only; see the note in `../Cargo.toml`.
