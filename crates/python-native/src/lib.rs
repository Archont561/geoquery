//! `PyO3` adapter for the Python SDK.
//!
//! One function, taking and returning JSON, and for the same reason as the Node adapter:
//! every operation the engine has is an `Operation` in `geoquery-protocol`. Exposing them
//! one by one would mean a new `#[pyfunction]` and a new release of the wheel for every
//! operation the engine grows, and a Python surface that had to be kept in step with the
//! Rust one by hand.

use pyo3::prelude::*;

/// Run one transport request and return one transport response, both as JSON text.
///
/// Public so `tests/lib.rs` can drive the adapter without an interpreter attached; the
/// crate is `publish = false`, so this is not an API anyone else is committing to.
#[pyfunction]
#[must_use]
pub fn invoke(request: &str) -> String {
    geoquery_engine::invoke(request)
}

/// The extension module the `geoquery` package imports.
#[pymodule]
fn _native(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(invoke, module)?)?;
    Ok(())
}
