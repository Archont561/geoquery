//! Mirrors `src/lib.rs`, which is a re-export list and nothing else — so this is a test
//! about the re-export list.
//!
//! `document::QueryDocument` and `geoquery_core::QueryDocument` are the same type, and
//! only the second one is a promise: a dependent that imports from the crate root must
//! not break because a module was renamed or split. The per-module tests beside this
//! file cover the behaviour; this one covers the shape of the front door.

use geoquery_core::{PROTOCOL_VERSION, QueryDocument, QueryDocumentError, VERSION, user_agent};

#[test]
fn the_crate_root_is_the_whole_public_surface() {
    let document: QueryDocument = QueryDocument::parse("{}").expect("an empty query object");
    assert!(document.is_empty());

    let error: QueryDocumentError =
        QueryDocument::parse("[]").expect_err("an array is not a document");
    assert!(
        error.to_string().contains("not a query document"),
        "the error type reached the root with its Display intact: {error}"
    );

    assert_eq!(PROTOCOL_VERSION, VERSION);
    assert!(user_agent().contains(VERSION));
}

#[test]
fn the_modules_and_the_root_name_the_same_types() {
    let from_module = geoquery_core::document::QueryDocument::parse(r#"{"limit": 1}"#)
        .expect("a one-key document");
    let from_root = QueryDocument::parse(r#"{"limit": 1}"#).expect("a one-key document");
    assert_eq!(
        from_module, from_root,
        "the re-export is an alias, not a second type"
    );
    assert_eq!(geoquery_core::version::VERSION, VERSION);
}
