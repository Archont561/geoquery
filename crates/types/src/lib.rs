//! The canonical query AST and the data model around it.
//!
//! This crate is the API. Everything else in the project — the CLI, the HTTP server, the
//! MCP tools, the TypeScript and Python SDKs — is an adapter to the types here, and the
//! TypeScript definitions are generated from them rather than written twice. That is why
//! it depends on `ts-rs`: a second hand-written copy of a query shape is a second thing to
//! keep correct, and the copy is the one the SDK uses.
//!
//! It depends on nothing inside the workspace. The language is not allowed to depend on
//! the engine that executes it, because the engine is a program and the language is a
//! document format; a type that can only be expressed by asking the engine what it means
//! is a type the clients cannot construct.
//!
//! Lands in Phase 1 (`backlog/milestones/`). Scaffolding only: the member
//! exists so the lockfile and the licence policy cover the dependencies above from the
//! first commit. See the comment at the top of `../Cargo.toml` for why that is worth a
//! few hundred crates of compile time.
