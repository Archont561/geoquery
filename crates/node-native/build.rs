//! Build script for the N-API addon.
//!
//! Its only job is to emit the link and cfg directives Node needs from an addon the
//! compiler cannot infer: which N-API version is targeted, and that the Node symbols are
//! resolved by the host at load time rather than at link time.

// `napi_build::setup()` does that work. It is not optional: a cdylib built without it
// compiles cleanly and then fails to load with an unresolved-symbol error that names no
// source file and no policy, which is the worst shape a build failure can have.
fn main() {
    napi_build::setup();
}
