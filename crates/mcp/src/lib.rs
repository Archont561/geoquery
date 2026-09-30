//! The MCP interface, and the reason a language independent of any one protocol is
//! worth having: an agent that can only speak STAC can only federate STAC, and the
//! federation is the part that was hard.
//!
//! Three tools, mapped to the three things an agent asks for. `geo_query` executes a
//! query. `geo_resource` describes something that already exists. `geo_resolve` turns a
//! name or a bbox into the resources that could answer it — which is the tool that has no
//! equivalent in any of the source protocols, and the one that justifies MCP being an
//! adapter rather than a gateway.
//!
//! Lands in Phase 4. Scaffolding only; see the note in `../Cargo.toml`.
