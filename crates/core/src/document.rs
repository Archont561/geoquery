//! A query document: the JSON object a query arrives as.
//!
//! Reading and describing a document is the whole of this module, and that is not a
//! shortcut. The document format is fixed by the protocol, so the code that reads it
//! stays correct while the engine behind it is being written — and a client that can
//! reject a malformed document without a server is worth having on day one.

use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

/// A query document.
///
/// Constructed only by the two functions that can tell a document from anything else, so
/// holding one means the JSON parsed *and* it was an object: a `String` is not a query
/// document with a bad field in it, and there is no way to build one here.
#[derive(Debug, Clone, PartialEq)]
pub struct QueryDocument {
    object: Map<String, Value>,
}

impl QueryDocument {
    /// Read and parse a document from disk.
    ///
    /// # Errors
    ///
    /// [`QueryDocumentError::Unreadable`] if the file cannot be opened,
    /// [`QueryDocumentError::Invalid`] if it is not JSON,
    /// [`QueryDocumentError::NotAnObject`] if it is JSON but not an object.
    pub fn read(path: &Path) -> Result<Self, QueryDocumentError> {
        let file = File::open(path).map_err(|source| QueryDocumentError::Unreadable {
            path: path.to_path_buf(),
            source,
        })?;
        // `from_reader` rather than reading to a String first: a query document can be
        // large, and there is no reason to hold both the bytes and the parsed tree.
        let value: Value = serde_json::from_reader(file)
            .map_err(|source| QueryDocumentError::Invalid { source })?;
        Self::from_value(value)
    }

    /// Parse a document from text.
    ///
    /// # Errors
    ///
    /// As [`Self::read`], minus the unreadable case.
    pub fn parse(text: &str) -> Result<Self, QueryDocumentError> {
        let value: Value =
            serde_json::from_str(text).map_err(|source| QueryDocumentError::Invalid { source })?;
        Self::from_value(value)
    }

    /// Take the object out of a parsed value, or say what the value was instead.
    fn from_value(value: Value) -> Result<Self, QueryDocumentError> {
        match value {
            Value::Object(object) => Ok(Self { object }),
            other => Err(not_an_object(&other)),
        }
    }

    /// The document's top-level keys, in ascending order.
    ///
    /// Sorted rather than in document order, because the only consumer today is a human
    /// reading a line of output and a stable order makes that line diffable. The protocol
    /// does not give these keys an order and a future validator must not assume one.
    pub fn keys(&self) -> impl Iterator<Item = &str> + '_ {
        let mut keys: Vec<&str> = self.object.keys().map(String::as_str).collect();
        keys.sort_unstable();
        keys.into_iter()
    }

    /// Whether the document has no keys at all.
    ///
    /// Not the same as invalid: `{}` is a degenerate query, and refusing it here would
    /// mean this crate had opinions about what an empty query means before the engine
    /// exists to have them.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.object.is_empty()
    }

    /// The value of a top-level key.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.object.get(key)
    }
}

/// Everything that can be wrong with a document, classified.
///
/// An enum rather than a message, because the caller is a program that has to decide
/// what to *do*: an unreadable file is exit code 1, a malformed document is exit code 2,
/// and neither is a bug in the query. The wording of each failure belongs to whoever
/// prints it, which is why there is no message here.
#[derive(Debug)]
pub enum QueryDocumentError {
    /// The file could not be opened.
    Unreadable {
        /// The path that was asked for.
        path: PathBuf,
        /// What the operating system said.
        source: io::Error,
    },
    /// The bytes are not JSON.
    Invalid {
        /// The parser's complaint, verbatim: it knows where the document stopped
        /// making sense and a paraphrase would lose that.
        source: serde_json::Error,
    },
    /// The JSON is valid, and is not an object.
    NotAnObject {
        /// What it is instead.
        found: &'static str,
    },
}

impl std::fmt::Display for QueryDocumentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unreadable { path, source } => {
                write!(f, "cannot read {}: {source}", path.display())
            }
            Self::Invalid { source } => write!(f, "invalid JSON: {source}"),
            Self::NotAnObject { found } => write!(f, "not a query document: found {found}"),
        }
    }
}

impl std::error::Error for QueryDocumentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Unreadable { source, .. } => Some(source),
            Self::Invalid { source } => Some(source),
            Self::NotAnObject { .. } => None,
        }
    }
}

/// The JSON type of a value, for a message that says what was found instead of what was
/// expected.
#[must_use]
pub fn json_type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "a boolean",
        Value::Number(_) => "a number",
        Value::String(_) => "a string",
        Value::Array(_) => "an array",
        Value::Object(_) => "an object",
    }
}

/// Classify a value that is not an object, for [`QueryDocumentError::NotAnObject`].
pub(crate) fn not_an_object(value: &Value) -> QueryDocumentError {
    QueryDocumentError::NotAnObject {
        found: json_type_name(value),
    }
}

#[cfg(test)]
mod tests {
    use super::{QueryDocument, QueryDocumentError};
    use serde_json::json;

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
        let document =
            QueryDocument::parse("{}").expect("an empty object is degenerate, not invalid");
        assert!(document.is_empty());
        assert_eq!(document.keys().count(), 0);
    }

    #[test]
    fn a_value_is_not_a_document() {
        // Parsing to `Value` first, then checking the shape, is what makes these
        // NotAnObject rather than serde's "invalid type: sequence, expected a map":
        // the first says what the user wrote, the second says what we wished for.
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
        let document =
            QueryDocument::parse(r#"{"limit": 25, "execution": {"max_concurrency": 4}}"#)
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
        let path = std::env::temp_dir().join(format!("geoquery-core-{}.json", std::process::id()));
        std::fs::write(&path, text).expect("the temp dir is writable");
        let read = QueryDocument::read(&path).expect("the same document from disk");
        std::fs::remove_file(&path).ok();
        assert_eq!(document, read);
    }
}
