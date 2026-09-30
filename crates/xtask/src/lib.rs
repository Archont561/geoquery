//! The one tool that has to be a program rather than a task.
//!
//! `pixi run codegen` would have to load the workspace's own types into memory to derive
//! the TypeScript definitions and the JSON schemas from them, and a task runs a command —
//! it cannot be the thing that defines what the command does. So this is a crate, and it
//! is the only one: everything else in the repository is run by `pixi run <task>`, and a
//! second task runner is a second thing to keep in step.
//!
//! It lives at `crates/xtask/` rather than the repository root that
//! `.knowledge/infrastructure/xtask.md` specifies, for the same reason there is no
//! `Cargo.toml` at the root: a workspace member has to be inside the workspace, and the
//! workspace root is `crates/`.
//!
//! Lands in Phase 1, with the types it reads. Scaffolding only; see the note in
//! `../Cargo.toml`.
