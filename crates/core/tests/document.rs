//! Mirrors `src/document.rs`: reading a query document, and saying what is wrong with
//! one.
//!
//! Every case here goes through the crate's public surface, which is the surface the
//! CLI and the service both use. A document that parses is a `QueryDocument`; there is
//! no way to build one out of anything else, and these tests are about what the two
//! constructors accept and how they classify what they refuse.

use serde_json::json;

use geoquery_core::document::json_type_name;
use geoquery_core::{QueryDocument, QueryDocumentError};

#[test]
fn keys_come_back_sorted() {
    let document = QueryDocument::parse(r#"{"limit": 25, "execution": {}, "scope": []}"#)
        .expect("a three-key object is a document");
    assert_eq!(
        document.keys().collect::<Vec<_>>(),
        ["execution", "limit", "scope"]
    );
}

#[test]
fn an_empty_object_is_a_document_with_no_keys() {
    let document = QueryDocument::parse("{}").expect("an empty object is degenerate, not invalid");
    assert!(document.is_empty());
    assert_eq!(document.keys().count(), 0);
}

#[test]
fn a_value_is_not_a_document() {
    // Parsing to `Value` first, then checking the shape, is what makes these
    // NotAnObject rather than serde's "invalid type: sequence, expected a map": the
    // first says what the user wrote, the second says what we wished for.
    for (text, expected) in [
        ("[]", "an array"),
        ("7", "a number"),
        (r#""a string""#, "a string"),
        ("true", "a boolean"),
        ("null", "null"),
    ] {
        let error = QueryDocument::parse(text).expect_err("a bare value is not a document");
        match error {
            QueryDocumentError::NotAnObject { found } => assert_eq!(found, expected),
            other => panic!("{text} produced {other:?}"),
        }
    }
}

#[test]
fn a_missing_file_is_not_a_malformed_one() {
    let error = QueryDocument::read(std::path::Path::new("/nonexistent/geoquery/query.json"))
        .expect_err("there is no file there");
    assert!(
        matches!(error, QueryDocumentError::Unreadable { .. }),
        "got {error:?}",
    );
}

#[test]
fn a_truncated_document_reports_the_parser() {
    let error = QueryDocument::parse(r#"{"limit": 25"#).expect_err("truncated JSON");
    assert!(
        matches!(error, QueryDocumentError::Invalid { .. }),
        "{error:?}"
    );
    assert!(
        error.to_string().contains("line"),
        "the parser says where: {error}"
    );
}

#[test]
fn values_are_readable_by_key() {
    let document = QueryDocument::parse(r#"{"limit": 25, "execution": {"max_concurrency": 4}}"#)
        .expect("a document");
    assert_eq!(document.get("limit"), Some(&json!(25)));
    assert_eq!(
        document.get("execution"),
        Some(&json!({"max_concurrency": 4})),
        "nested values are values, not opaque text",
    );
    assert_eq!(
        document.get("scope"),
        None,
        "a key the document does not have reads back as absent, not as null",
    );
}

#[test]
fn reading_and_parsing_agree() {
    let text = r#"{"filters": {"op": "all", "filter": []}}"#;
    let document = QueryDocument::parse(text).expect("a document");
    // Nextest runs every test in its own process and several at once, so a fixed path
    // under `tests/` would be a file two tests could be writing at the same time. The
    // process id plus the test's own name is the convention the whole workspace uses.
    let path = std::env::temp_dir().join(format!(
        "geoquery-core-{}-reading-and-parsing-agree.json",
        std::process::id()
    ));
    std::fs::write(&path, text).expect("the temp dir is writable");
    let read = QueryDocument::read(&path).expect("the same document from disk");
    std::fs::remove_file(&path).ok();
    assert_eq!(document, read);
}

#[test]
fn a_failure_hands_over_the_error_underneath_it() {
    // `source` is the chain every error reporter walks to print "caused by". The two
    // failures that wrap something else have to hand it over, or the operating system's
    // reason and the parser's position stop at this crate's boundary; the one this crate
    // classified itself has to not invent a cause it does not have.
    use std::error::Error as _;

    let unreadable = QueryDocument::read(std::path::Path::new("/nonexistent/geoquery/query.json"))
        .expect_err("there is no file there");
    assert!(
        unreadable.source().is_some(),
        "the operating system said why: {unreadable}"
    );

    let invalid = QueryDocument::parse(r#"{"limit": 25"#).expect_err("truncated JSON");
    assert!(
        invalid.source().is_some(),
        "the parser said where: {invalid}"
    );

    let not_an_object = QueryDocument::parse("[]").expect_err("an array is not a document");
    assert!(
        not_an_object.source().is_none(),
        "nothing underlies a shape this crate classified itself: {not_an_object}"
    );
}

#[test]
fn an_object_is_named_like_every_other_json_type() {
    // The other five names are reached through `NotAnObject`; this one cannot be, because
    // an object is the case that succeeds. It is still the answer the function owes a
    // caller that asks about a value nothing rejected.
    assert_eq!(json_type_name(&json!({})), "an object");
}
