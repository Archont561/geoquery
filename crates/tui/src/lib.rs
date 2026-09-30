//! The interactive terminal interface: a result browser, a source dashboard, and an
//! ASCII map — all of which are ways of looking at a `GeoResult` that a terminal can
//! draw without rendering a map tile.
//!
//! It is the only interface with no protocol, so it is the one that can be built and
//! thrown away cheaply. That is also why the roadmap defers it: a TUI is a presentation of
//! results the engine produced, and until there are results from more than one source it
//! has nothing to compare them with.
//!
//! Lands in Phase 4, after the engine. Scaffolding only; see the note in
//! `../Cargo.toml`.
