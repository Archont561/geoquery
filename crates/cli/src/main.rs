//! The `geoquery` command-line client.
//!
//! A presentation layer: parse arguments, ask `geoquery-core` what a document is, print
//! the answer in words and pick an exit code. Every rule about what a query is belongs
//! to the library, because a rule in the binary is a rule the service has to write again.
//!
//! Phase 0 is deliberately a shell around the two things that are finished: it reads a
//! query document and tells you whether it is one, and it refuses — loudly, with an exit
//! code — to pretend it can execute it. The engine arrives with Phase 1, and a client
//! that silently returned empty results until then would be worse than one that
//! declines.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use geoquery_core::{QueryDocument, QueryDocumentError};

/// The document could not be read.
const EXIT_IO: u8 = 1;
/// The document is not a query document.
const EXIT_INVALID: u8 = 2;
/// The client is not wired to an engine yet.
const EXIT_UNIMPLEMENTED: u8 = 3;

/// One protocol-independent query language for geospatial resources and services.
#[derive(Debug, Parser)]
#[command(
    name = "geoquery",
    // The version reported is the language's, not the binary's. What a user needs to
    // know when a service misbehaves is which protocol this build speaks, and the two
    // numbers are the same today — `pixi run version-check` is what keeps them so.
    version = geoquery_core::VERSION,
    about,
    long_about = None
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Read a query document, report what it contains, contact nothing.
    Check {
        /// Path to the JSON query document.
        query: PathBuf,
    },
    /// Send a query document to a service.
    Query {
        /// Base URL of the geoquery service to send the query to.
        #[arg(long, value_name = "URL")]
        service: String,
        /// Path to the JSON query document.
        #[arg(long, value_name = "PATH")]
        query: PathBuf,
    },
}

/// A failure the user is meant to read: a message and the code the shell should see.
///
/// A struct rather than a boxed error, because every failure here is a decision this
/// program makes about what a shell should see next — and an error chain that ended in
/// `Box<dyn Error>` would throw away exactly the information the exit code needs.
#[derive(Debug)]
struct Failure {
    message: String,
    code: u8,
}

impl Failure {
    fn new(message: impl Into<String>, code: u8) -> Self {
        Self {
            message: message.into(),
            code,
        }
    }
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(report) => {
            println!("{report}");
            ExitCode::SUCCESS
        }
        Err(failure) => {
            eprintln!("geoquery: {failure}", failure = failure.message);
            ExitCode::from(failure.code)
        }
    }
}

fn run(cli: Cli) -> Result<String, Failure> {
    match cli.command {
        Command::Check { query } => check(&query),
        Command::Query { service, query } => execute(&service, &query),
    }
}

/// Report the shape of a query document without sending it anywhere.
fn check(path: &Path) -> Result<String, Failure> {
    let document = QueryDocument::read(path).map_err(|error| describe(path, &error))?;
    if document.is_empty() {
        return Ok(format!("{} is an empty query object", path.display()));
    }
    let keys: Vec<&str> = document.keys().collect();
    Ok(format!(
        "{} is a query object with {} key{}: {}",
        path.display(),
        keys.len(),
        if keys.len() == 1 { "" } else { "s" },
        keys.join(", "),
    ))
}

/// Phrase a library failure for someone holding a terminal.
///
/// The library classifies — unreadable, invalid, not an object — because that is what a
/// program has to branch on. The words, and the exit code each one earns, are this
/// program's business: they are the difference between "the file is not there" and
/// "your file is wrong", and getting that wrong sends people looking in the wrong
/// place.
fn describe(path: &Path, error: &QueryDocumentError) -> Failure {
    let code = match error {
        QueryDocumentError::Unreadable { .. } => EXIT_IO,
        QueryDocumentError::Invalid { .. } | QueryDocumentError::NotAnObject { .. } => EXIT_INVALID,
    };
    let message = match error {
        // The library's wording for this one is already complete — path and the
        // operating system's complaint — so it is printed as it stands. Repeating the
        // path in front of it produced "cannot read x: cannot read x: ...", which reads
        // like two failures and hides the one.
        QueryDocumentError::Unreadable { .. } => error.to_string(),
        QueryDocumentError::Invalid { source } => {
            format!("{} is not a valid query document: {source}", path.display())
        }
        QueryDocumentError::NotAnObject { found } => {
            format!(
                "{} is valid JSON but not a query object (found {found})",
                path.display()
            )
        }
    };
    Failure::new(message, code)
}

/// The engine does not exist yet, and this is what saying so looks like.
fn execute(service: &str, path: &Path) -> Result<String, Failure> {
    // Named inside the message rather than interpolated from a `use`: it is the only
    // place the language version reaches a user, and an import used once for a format
    // string is an import that goes stale silently.
    const LANGUAGE: &str = geoquery_core::VERSION;
    // Parse before declining: a malformed document is worth reporting even when the
    // request cannot be sent, and it costs nothing to check first.
    check(path)?;
    Err(Failure::new(
        format!(
            "no engine yet — {service} was not contacted. \
             geoquery {LANGUAGE} reads query documents; executing them arrives with the \
             query engine.",
        ),
        EXIT_UNIMPLEMENTED,
    ))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use clap::CommandFactory;

    use super::{EXIT_INVALID, EXIT_IO, EXIT_UNIMPLEMENTED, Failure, check, execute};
    use geoquery_core::{PROTOCOL_VERSION, VERSION, user_agent};

    /// A document on disk in a path unique to this process and this test, so the suite
    /// can run in parallel and leave nothing behind.
    fn temp_query(name: &str, contents: &str) -> PathBuf {
        let file = format!("geoquery-cli-{}-{name}.json", std::process::id());
        let path = std::env::temp_dir().join(file);
        fs::write(&path, contents).expect("the temp dir is writable");
        path
    }

    fn failure_of(result: Result<String, Failure>) -> Failure {
        match result {
            Ok(report) => panic!("expected a failure, got {report}"),
            Err(failure) => failure,
        }
    }

    #[test]
    fn check_reports_the_keys_of_a_query_object() {
        let path = temp_query(
            "keys",
            r#"{"execution": {"max_concurrency": 4}, "limit": 25}"#,
        );
        let report = check(&path).expect("a query object is accepted");
        assert!(
            report.contains("2 keys: execution, limit"),
            "unexpected report: {report}",
        );
        fs::remove_file(&path).ok();
    }

    #[test]
    fn check_reports_one_key_without_pluralising_it() {
        let path = temp_query("one-key", r#"{"limit": 25}"#);
        let report = check(&path).expect("a one-key query is accepted");
        assert!(
            report.contains("1 key: limit"),
            "unexpected report: {report}"
        );
        fs::remove_file(&path).ok();
    }

    #[test]
    fn check_accepts_an_empty_query_object() {
        let path = temp_query("empty", "{}");
        let report = check(&path).expect("an empty object is a degenerate query, not an error");
        assert!(report.contains("empty query object"), "{report}");
        fs::remove_file(&path).ok();
    }

    #[test]
    fn check_rejects_json_that_is_not_an_object() {
        let path = temp_query("array", r#"["not", "a", "query"]"#);
        let failure = failure_of(check(&path));
        assert_eq!(failure.code, EXIT_INVALID);
        assert!(
            failure.message.contains("not a query object"),
            "the message should say what was found: {}",
            failure.message,
        );
        fs::remove_file(&path).ok();
    }

    #[test]
    fn check_rejects_a_document_it_cannot_parse() {
        let path = temp_query("truncated", r#"{"limit": 25"#);
        let failure = failure_of(check(&path));
        assert_eq!(failure.code, EXIT_INVALID);
        fs::remove_file(&path).ok();
    }

    #[test]
    fn check_distinguishes_a_missing_file_from_a_bad_one() {
        let failure = failure_of(check(std::path::Path::new(
            "/nonexistent/geoquery/query.json",
        )));
        assert_eq!(
            failure.code, EXIT_IO,
            "a file that is not there is an IO failure, not a parse failure",
        );
        assert!(
            !failure
                .message
                .contains("cannot read /nonexistent/geoquery/query.json: cannot read"),
            "the path is named once, not once per layer that knows it: {}",
            failure.message,
        );
    }

    #[test]
    fn execute_refuses_before_it_sends_anything() {
        let path = temp_query("execute", r#"{"limit": 25}"#);
        let failure = failure_of(execute("https://example.invalid", &path));
        assert_eq!(failure.code, EXIT_UNIMPLEMENTED);
        assert!(
            failure.message.contains("example.invalid"),
            "the refusal should name the service it declined to contact: {}",
            failure.message,
        );
        fs::remove_file(&path).ok();
    }

    #[test]
    fn execute_reports_a_broken_document_before_refusing() {
        let path = temp_query("execute-bad", "{");
        let failure = failure_of(execute("https://example.invalid", &path));
        assert_eq!(
            failure.code, EXIT_INVALID,
            "a malformed document is the more useful complaint",
        );
        fs::remove_file(&path).ok();
    }

    /// The version the binary reports has to be the one the rest of the project agrees
    /// on; `pixi run version-check` checks the manifests, this checks the wiring.
    #[test]
    fn the_version_the_binary_reports_is_the_language_version() {
        assert_eq!(
            VERSION, "0.1.0",
            "bump pixi.toml and every Cargo manifest together"
        );
        assert_eq!(PROTOCOL_VERSION, VERSION, "one number today");
        assert_eq!(
            user_agent(),
            "geoquery/0.1.0 (query protocol 0.1.0)",
            "the client identifies itself with both"
        );
    }

    /// The generated help is the only documentation this binary has yet, so a clap
    /// derive mistake must not be able to reach a release.
    #[test]
    fn the_command_line_surface_is_well_formed() {
        let mut top_level = super::Cli::command();
        let help = top_level.render_long_help().to_string();
        assert!(help.contains("check"), "help is missing `check`:\n{help}");
        assert!(help.contains("query"), "help is missing `query`:\n{help}");

        // `--service` belongs to `query`, not to the top level: a flag on the parent
        // would be accepted by `geoquery check` too, and a service URL is exactly the
        // argument `check` must refuse to take.
        let mut query = top_level
            .find_subcommand_mut("query")
            .expect("`query` is a subcommand")
            .clone();
        let query_help = query.render_long_help().to_string();
        assert!(
            query_help.contains("--service <URL>"),
            "`query` help is missing --service:\n{query_help}",
        );
        assert!(
            query_help.contains("--query <PATH>"),
            "`query` help is missing --query:\n{query_help}",
        );
    }

    #[test]
    fn the_subcommand_names_are_the_published_surface() {
        let names: Vec<String> = super::Cli::command()
            .get_subcommands()
            .map(|subcommand| subcommand.get_name().to_owned())
            .collect();
        assert_eq!(
            names,
            vec!["check".to_owned(), "query".to_owned()],
            "the published command surface",
        );
    }
}
