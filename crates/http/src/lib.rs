//! The HTTP interface, and the first non-CLI way to reach the engine.
//!
//! axum, because the routing is a POST and a few resource paths, and because utoipa reads
//! the router's own types to generate the `OpenAPI` document — a spec written beside the
//! routes is a spec that is wrong the first time a route changes.
//!
//! Streaming is SSE, not WebSocket: the flow is a server sending results as they arrive
//! from sources, which is unidirectional, and a unidirectional flow over WebSocket is a
//! protocol the client has to implement twice.
//!
//! Lands in Phase 4. Scaffolding only; see the note in `../Cargo.toml`.
