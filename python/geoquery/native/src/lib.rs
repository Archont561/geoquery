//! Native Python bindings for the geoquery language.
//!
//! This module is deliberately a thin FFI boundary. Query parsing and protocol semantics
//! stay in `geoquery-core`, so Python cannot develop a second interpretation of a query.
//! Execution will be added behind the same boundary when the engine is ready.

use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;

/// The native module shared by the Python package and the Rust query language.
#[pymodule]
fn _native(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(protocol_version, module)?)?;
    module.add_function(wrap_pyfunction!(check_query, module)?)?;
    Ok(())
}

/// Return the query protocol version implemented by this extension.
#[pyfunction]
fn protocol_version() -> &'static str {
    geoquery_core::VERSION
}

/// Validate a JSON query document in Rust and return its sorted top-level keys.
///
/// Keeping this first operation small makes the boundary testable before the execution
/// engine exists. It also proves that Python is using the same parser as the CLI and the
/// future HTTP service, rather than a Python-only request model.
#[pyfunction]
fn check_query(query: &str) -> PyResult<Vec<String>> {
    let document = geoquery_core::QueryDocument::parse(query).map_err(|error| {
        let message = error.to_string();
        match error {
            geoquery_core::QueryDocumentError::Invalid { .. }
            | geoquery_core::QueryDocumentError::NotAnObject { .. } => {
                PyErr::new::<PyValueError, _>(message)
            }
            geoquery_core::QueryDocumentError::Unreadable { .. } => {
                PyErr::new::<PyRuntimeError, _>(message)
            }
        }
    })?;

    Ok(document.keys().map(str::to_owned).collect())
}
