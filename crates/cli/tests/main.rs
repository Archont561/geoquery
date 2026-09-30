//! Mirrors `src/main.rs`: everything the `geoquery` binary promises a shell.
//!
//! These run the built binary rather than calling its functions, and that is the point.
//! `check` and `execute` are private to a binary crate, so a test beside them can only
//! be reached by compiling the binary in test mode — which asserts today's function
//! names. What the program actually promises is three things a script can observe: what
//! it writes to stdout, what it writes to stderr, and the code it exits with. Every
//! assertion here is one of those three.
//!
//! Registered as `[[test]] path = "crates/cli/tests/main.rs"` in the root `Cargo.toml`,
//! because the binary's manifest is the root one — see the comment at the top of that
//! file — and a test file has to sit beside the sources it tests, not beside the
//! manifest that happens to declare them.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

/// The document could not be read.
const EXIT_IO: i32 = 1;
/// The document is not a query document.
const EXIT_INVALID: i32 = 2;
/// The client is not wired to an engine yet.
const EXIT_UNIMPLEMENTED: i32 = 3;

/// The binary this test is about, built by cargo before the test runs.
///
/// `CARGO_BIN_EXE_<name>` rather than a path under `target/`: the profile, the target
/// directory and the executable suffix are cargo's to decide, and a hand-built path is
/// wrong the first time somebody runs the suite under a different profile.
fn geoquery(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_geoquery"))
        .args(args)
        .output()
        .expect("the binary cargo just built is runnable")
}

/// A document on disk at a path unique to this process and this test, so the suite can
/// run in parallel — nextest gives each test its own process — and leave nothing behind.
fn temp_query(name: &str, contents: &str) -> PathBuf {
    let file = format!("geoquery-cli-{}-{name}.json", std::process::id());
    let path = std::env::temp_dir().join(file);
    fs::write(&path, contents).expect("the temp dir is writable");
    path
}

fn stdout_of(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("the report is UTF-8")
}

fn stderr_of(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("the complaint is UTF-8")
}

fn code_of(output: &Output) -> i32 {
    output
        .status
        .code()
        .expect("the process exited, not signalled")
}

#[test]
fn check_reports_the_keys_of_a_query_object() {
    let path = temp_query(
        "keys",
        r#"{"execution": {"max_concurrency": 4}, "limit": 25}"#,
    );
    let output = geoquery(&["check", path.to_str().expect("a UTF-8 temp path")]);
    fs::remove_file(&path).ok();

    assert_eq!(code_of(&output), 0, "{}", stderr_of(&output));
    let report = stdout_of(&output);
    assert!(
        report.contains("2 keys: execution, limit"),
        "unexpected report: {report}",
    );
}

#[test]
fn check_reports_one_key_without_pluralising_it() {
    let path = temp_query("one-key", r#"{"limit": 25}"#);
    let output = geoquery(&["check", path.to_str().expect("a UTF-8 temp path")]);
    fs::remove_file(&path).ok();

    assert_eq!(code_of(&output), 0, "{}", stderr_of(&output));
    let report = stdout_of(&output);
    assert!(
        report.contains("1 key: limit"),
        "unexpected report: {report}"
    );
}

#[test]
fn check_accepts_an_empty_query_object() {
    let path = temp_query("empty", "{}");
    let output = geoquery(&["check", path.to_str().expect("a UTF-8 temp path")]);
    fs::remove_file(&path).ok();

    assert_eq!(code_of(&output), 0, "{}", stderr_of(&output));
    assert!(
        stdout_of(&output).contains("empty query object"),
        "{}",
        stdout_of(&output)
    );
}

#[test]
fn check_rejects_json_that_is_not_an_object() {
    let path = temp_query("array", r#"["not", "a", "query"]"#);
    let output = geoquery(&["check", path.to_str().expect("a UTF-8 temp path")]);
    fs::remove_file(&path).ok();

    assert_eq!(code_of(&output), EXIT_INVALID);
    let complaint = stderr_of(&output);
    assert!(
        complaint.contains("not a query object"),
        "the message should say what was found: {complaint}",
    );
}

#[test]
fn check_rejects_a_document_it_cannot_parse() {
    let path = temp_query("truncated", r#"{"limit": 25"#);
    let output = geoquery(&["check", path.to_str().expect("a UTF-8 temp path")]);
    fs::remove_file(&path).ok();

    assert_eq!(code_of(&output), EXIT_INVALID);
}

#[test]
fn check_distinguishes_a_missing_file_from_a_bad_one() {
    let output = geoquery(&["check", "/nonexistent/geoquery/query.json"]);

    assert_eq!(
        code_of(&output),
        EXIT_IO,
        "a file that is not there is an IO failure, not a parse failure",
    );
    let complaint = stderr_of(&output);
    assert!(
        !complaint.contains("cannot read /nonexistent/geoquery/query.json: cannot read"),
        "the path is named once, not once per layer that knows it: {complaint}",
    );
}

#[test]
fn execute_refuses_before_it_sends_anything() {
    let path = temp_query("execute", r#"{"limit": 25}"#);
    let output = geoquery(&[
        "query",
        "--service",
        "https://example.invalid",
        "--query",
        path.to_str().expect("a UTF-8 temp path"),
    ]);
    fs::remove_file(&path).ok();

    assert_eq!(code_of(&output), EXIT_UNIMPLEMENTED);
    let complaint = stderr_of(&output);
    assert!(
        complaint.contains("example.invalid"),
        "the refusal should name the service it declined to contact: {complaint}",
    );
    assert!(
        stdout_of(&output).is_empty(),
        "a refusal is not a result: nothing belongs on stdout",
    );
}

#[test]
fn execute_reports_a_broken_document_before_refusing() {
    let path = temp_query("execute-bad", "{");
    let output = geoquery(&[
        "query",
        "--service",
        "https://example.invalid",
        "--query",
        path.to_str().expect("a UTF-8 temp path"),
    ]);
    fs::remove_file(&path).ok();

    assert_eq!(
        code_of(&output),
        EXIT_INVALID,
        "a malformed document is the more useful complaint",
    );
}

/// The version the binary reports has to be the one the rest of the project agrees on;
/// `pixi run version-check` checks the manifests, this checks what a user is told.
#[test]
fn the_version_the_binary_reports_is_the_language_version() {
    let output = geoquery(&["--version"]);
    assert_eq!(code_of(&output), 0, "{}", stderr_of(&output));
    assert_eq!(
        stdout_of(&output).trim(),
        "geoquery 0.1.0",
        "bump pixi.toml and every manifest that inherits from it, together"
    );
}

/// The generated help is the only documentation this binary ships with, so a clap derive
/// mistake must not be able to reach a release.
#[test]
fn the_command_line_surface_is_well_formed() {
    let help = stdout_of(&geoquery(&["--help"]));
    assert!(help.contains("check"), "help is missing `check`:\n{help}");
    assert!(help.contains("query"), "help is missing `query`:\n{help}");

    let query_help = stdout_of(&geoquery(&["query", "--help"]));
    assert!(
        query_help.contains("--service <URL>"),
        "`query` help is missing --service:\n{query_help}",
    );
    assert!(
        query_help.contains("--query <PATH>"),
        "`query` help is missing --query:\n{query_help}",
    );
}

/// `--service` belongs to `query`, not to the top level: a flag on the parent would be
/// accepted by `geoquery check` too, and a service URL is exactly the argument `check`
/// must refuse to take.
#[test]
fn check_refuses_to_be_given_a_service() {
    let path = temp_query("check-service", "{}");
    let output = geoquery(&[
        "check",
        "--service",
        "https://example.invalid",
        path.to_str().expect("a UTF-8 temp path"),
    ]);
    fs::remove_file(&path).ok();

    assert_ne!(code_of(&output), 0, "`check` reaches no network, ever");
    assert!(
        stderr_of(&output).contains("--service"),
        "the complaint should name the argument: {}",
        stderr_of(&output),
    );
}

#[test]
fn the_subcommand_names_are_the_published_surface() {
    let help = stdout_of(&geoquery(&["--help"]));
    let commands: Vec<&str> = help
        .lines()
        .skip_while(|line| !line.starts_with("Commands:"))
        .skip(1)
        .take_while(|line| line.starts_with("  "))
        .filter_map(|line| line.split_whitespace().next())
        .filter(|name| *name != "help")
        .collect();
    assert_eq!(
        commands,
        ["check", "query"],
        "the published command surface:\n{help}"
    );
}
