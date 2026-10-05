//! Mirrors `src/main.rs`: everything the `geoquery` binary promises a shell.
//!
//! These run the built binary rather than calling its functions, and that is the point.
//! Every function in `src/main.rs` is private to a binary crate, so a test beside them
//! can only be reached by compiling the binary in test mode. What the program actually
//! promises is three things a script can observe: what it writes to stdout, what it
//! writes to stderr, and the code it exits with. Every assertion here is one of those
//! three.
//!
//! The source-registration and query tests run against `wiremock`, never against a live
//! STAC service — `crates/adapter-stac/tests/` already proves the translation and
//! normalization logic without a network, and what is left to prove here is that the
//! binary wires a registry, an adapter and an execution policy together the way it
//! claims to.
//!
//! Registered as `[[test]] path = "crates/cli/tests/main.rs"` in the root `Cargo.toml`,
//! because the binary's manifest is the root one — see the comment at the top of that
//! file — and a test file has to sit beside the sources it tests, not beside the
//! manifest that happens to declare them.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

use serde_json::{Value, json};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// The document could not be read.
const EXIT_IO: i32 = 1;
/// The document is not a valid query document, or not a valid `GeoQuery`.
const EXIT_INVALID: i32 = 2;
/// A registry or source problem.
const EXIT_SOURCE: i32 = 3;
/// The federated run failed outright, or `strict` would not accept a partial one.
const EXIT_QUERY_FAILED: i32 = 4;

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

/// A fresh, never-before-used registry path for one test, so the suite can run in
/// parallel — nextest gives each test its own process, but every test in this file
/// shares that one process — and no test can see another's sources.
fn temp_registry(name: &str) -> PathBuf {
    let file = format!("geoquery-{}-{name}-registry.json", std::process::id());
    std::env::temp_dir().join(file)
}

/// A document on disk at a path unique to this process and this test, so the suite can
/// run in parallel and leave nothing behind.
fn temp_file(name: &str, contents: &str) -> PathBuf {
    let file = format!("geoquery-{}-{name}.json", std::process::id());
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

// ── check ─────────────────────────────────────────────────────────────────────────

#[test]
fn check_reports_the_keys_of_a_query_object() {
    let path = temp_file(
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
    let path = temp_file("one-key", r#"{"limit": 25}"#);
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
    let path = temp_file("empty", "{}");
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
    let path = temp_file("array", r#"["not", "a", "query"]"#);
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
    let path = temp_file("truncated", r#"{"limit": 25"#);
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
fn check_refuses_an_argument_that_belongs_to_query() {
    let path = temp_file("check-source", "{}");
    let output = geoquery(&[
        "check",
        "--source",
        "earth-search",
        path.to_str().expect("a UTF-8 temp path"),
    ]);
    fs::remove_file(&path).ok();

    assert_ne!(code_of(&output), 0, "`check` reaches no source, ever");
    assert!(
        stderr_of(&output).contains("--source"),
        "the complaint should name the argument: {}",
        stderr_of(&output),
    );
}

// ── surface ───────────────────────────────────────────────────────────────────────

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

#[test]
fn the_command_line_surface_is_well_formed() {
    let help = stdout_of(&geoquery(&["--help"]));
    assert!(help.contains("check"), "help is missing `check`:\n{help}");
    assert!(help.contains("source"), "help is missing `source`:\n{help}");
    assert!(help.contains("query"), "help is missing `query`:\n{help}");

    let query_help = stdout_of(&geoquery(&["query", "--help"]));
    assert!(query_help.contains("--query <PATH>"));
    assert!(query_help.contains("--source <ID>"));
    assert!(query_help.contains("--policy"));

    let source_help = stdout_of(&geoquery(&["source", "--help"]));
    assert!(source_help.contains("add"));
    assert!(source_help.contains("list"));
    assert!(source_help.contains("describe"));
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
        ["check", "source", "query"],
        "the published command surface:\n{help}"
    );
}

#[test]
fn an_unknown_policy_is_refused_by_the_argument_parser() {
    let path = temp_file("policy", r#"{"limit": 1}"#);
    let output = geoquery(&[
        "--registry",
        temp_registry("unknown-policy-arg").to_str().unwrap(),
        "query",
        "--query",
        path.to_str().unwrap(),
        "--policy",
        "aggressive",
    ]);
    fs::remove_file(&path).ok();
    assert_ne!(code_of(&output), 0);
    assert!(stderr_of(&output).contains("aggressive"));
}

// ── source add / list / describe ────────────────────────────────────────────────────

fn earth_search_landing(base: &str) -> Value {
    json!({
        "type": "Catalog",
        "conformsTo": [
            "https://api.stacspec.org/v1.0.0/core",
            "https://api.stacspec.org/v1.0.0/item-search"
        ],
        "links": [
            { "rel": "data", "href": format!("{base}/collections") },
            { "rel": "search", "href": format!("{base}/search"), "method": "POST" }
        ]
    })
}

fn collections_body() -> Value {
    json!({ "collections": [{ "id": "sentinel-2-l2a" }, { "id": "landsat-c2-l2" }] })
}

/// Mount a landing page and a collections document on `server`, the way a conforming
/// STAC API answers both.
async fn mount_stac_landing(server: &MockServer) {
    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(earth_search_landing(&server.uri())))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/collections"))
        .respond_with(ResponseTemplate::new(200).set_body_json(collections_body()))
        .mount(server)
        .await;
}

#[tokio::test]
async fn source_add_registers_a_discovered_service() {
    let server = MockServer::start().await;
    mount_stac_landing(&server).await;
    let registry = temp_registry("add-ok");

    let output = geoquery(&[
        "--registry",
        registry.to_str().unwrap(),
        "source",
        "add",
        "earth-search",
        "--type",
        "stac",
        "--url",
        &server.uri(),
    ]);
    fs::remove_file(&registry).ok();

    assert_eq!(code_of(&output), 0, "{}", stderr_of(&output));
    let report = stdout_of(&output);
    assert!(report.contains("earth-search"), "{report}");
    assert!(
        report.contains('2'),
        "two collections were discovered: {report}"
    );
}

#[tokio::test]
async fn source_add_rejects_an_unsupported_type() {
    let registry = temp_registry("add-unsupported-type");
    let output = geoquery(&[
        "--registry",
        registry.to_str().unwrap(),
        "source",
        "add",
        "a-wfs-source",
        "--type",
        "wfs",
        "--url",
        "https://example.test/wfs",
    ]);
    fs::remove_file(&registry).ok();

    assert_eq!(code_of(&output), EXIT_SOURCE);
    assert!(stderr_of(&output).contains("wfs"));
    assert!(
        !registry.exists(),
        "a rejected registration must not write anything"
    );
}

#[tokio::test]
async fn source_add_reports_a_discovery_failure_without_writing_the_registry() {
    let registry = temp_registry("add-discovery-failure");
    let output = geoquery(&[
        "--registry",
        registry.to_str().unwrap(),
        "source",
        "add",
        "nothing-here",
        "--type",
        "stac",
        "--url",
        "http://127.0.0.1:0/",
    ]);

    assert_eq!(code_of(&output), EXIT_SOURCE);
    assert!(
        !registry.exists(),
        "a failed discovery must not write a partial registry"
    );
}

#[test]
fn source_list_reports_an_empty_registry_plainly() {
    let registry = temp_registry("list-empty");
    let output = geoquery(&["--registry", registry.to_str().unwrap(), "source", "list"]);
    assert_eq!(code_of(&output), 0, "{}", stderr_of(&output));
    assert!(stdout_of(&output).contains("no sources registered"));
}

#[tokio::test]
async fn source_list_and_describe_see_what_add_registered() {
    let server = MockServer::start().await;
    mount_stac_landing(&server).await;
    let registry = temp_registry("list-and-describe");

    let add = geoquery(&[
        "--registry",
        registry.to_str().unwrap(),
        "source",
        "add",
        "earth-search",
        "--type",
        "stac",
        "--url",
        &server.uri(),
    ]);
    assert_eq!(code_of(&add), 0, "{}", stderr_of(&add));

    let list = geoquery(&["--registry", registry.to_str().unwrap(), "source", "list"]);
    assert_eq!(code_of(&list), 0);
    let listed = stdout_of(&list);
    assert!(listed.contains("earth-search"));
    assert!(listed.contains(&server.uri()));

    let describe = geoquery(&[
        "--registry",
        registry.to_str().unwrap(),
        "source",
        "describe",
        "earth-search",
    ]);
    fs::remove_file(&registry).ok();
    assert_eq!(code_of(&describe), 0);
    let descriptor: Value =
        serde_json::from_str(&stdout_of(&describe)).expect("describe prints JSON");
    assert_eq!(descriptor["descriptor"]["id"], "earth-search");
    assert_eq!(descriptor["descriptor"]["type"], "stac");
    assert_eq!(
        descriptor["descriptor"]["collections"],
        json!(["sentinel-2-l2a", "landsat-c2-l2"])
    );
}

#[test]
fn source_describe_names_the_source_it_could_not_find() {
    let registry = temp_registry("describe-missing");
    let output = geoquery(&[
        "--registry",
        registry.to_str().unwrap(),
        "source",
        "describe",
        "nobody-registered-this",
    ]);
    assert_eq!(code_of(&output), EXIT_SOURCE);
    assert!(stderr_of(&output).contains("nobody-registered-this"));
}

// ── query ─────────────────────────────────────────────────────────────────────────

async fn register_mock_source(registry: &std::path::Path, id: &str, server: &MockServer) {
    mount_stac_landing(server).await;
    let add = geoquery(&[
        "--registry",
        registry.to_str().unwrap(),
        "source",
        "add",
        id,
        "--type",
        "stac",
        "--url",
        &server.uri(),
    ]);
    assert_eq!(
        code_of(&add),
        0,
        "fixture setup failed: {}",
        stderr_of(&add)
    );
}

/// Point an already-registered source's search endpoint at a port nothing listens on.
///
/// Port 1 rather than a chosen-and-dropped `MockServer`: low ports are reserved for
/// privileged processes on every platform this suite runs on, so nothing in a sandbox or
/// CI binds it, and the failure this test wants — a connection refused, deterministically
/// — does not depend on timing or on no other test having claimed the same ephemeral port.
fn break_search_endpoint(registry: &std::path::Path, id: &str) {
    let text = fs::read_to_string(registry).expect("the registry was just written");
    let mut document: Value = serde_json::from_str(&text).expect("the registry is JSON");
    document["sources"][id]["descriptor"]["metadata"]["searchUrl"] =
        json!("http://127.0.0.1:1/search");
    fs::write(registry, serde_json::to_string_pretty(&document).unwrap())
        .expect("the registry is writable");
}

fn mount_search(features: &Value) -> wiremock::Mock {
    Mock::given(method("POST"))
        .and(path("/search"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({ "type": "FeatureCollection", "features": features })),
        )
}

#[tokio::test]
async fn query_runs_against_a_registered_source_and_prints_normalized_results() {
    let server = MockServer::start().await;
    let registry = temp_registry("query-ok");
    register_mock_source(&registry, "earth-search", &server).await;
    mount_search(&json!([{
        "type": "Feature",
        "id": "item-1",
        "collection": "sentinel-2-l2a",
        "geometry": null,
        "properties": { "datetime": "2024-06-15T00:00:00Z" },
        "assets": {},
        "links": []
    }]))
    .mount(&server)
    .await;

    let query = temp_file(
        "ok",
        r#"{"spatial": {"op": "bbox", "bbox": [20.85, 52.10, 21.25, 52.35]}, "limit": 5}"#,
    );
    let output = geoquery(&[
        "--registry",
        registry.to_str().unwrap(),
        "query",
        "--query",
        query.to_str().unwrap(),
    ]);
    fs::remove_file(&query).ok();
    fs::remove_file(&registry).ok();

    assert_eq!(code_of(&output), 0, "{}", stderr_of(&output));
    let body: Value = serde_json::from_str(&stdout_of(&output)).expect("query prints JSON");
    assert_eq!(body["status"], "ok");
    assert_eq!(body["results"].as_array().unwrap().len(), 1);
    assert_eq!(body["results"][0]["id"], "item-1");
    assert_eq!(body["results"][0]["provenance"]["source"], "earth-search");
    assert_eq!(body["sources"][0]["source"], "earth-search");
    assert_eq!(body["sources"][0]["status"], "ok");
    assert_eq!(
        body["sources"][0]["pushed"]["bbox"],
        json!([20.85, 52.10, 21.25, 52.35])
    );
}

#[tokio::test]
async fn query_reports_a_degraded_source_without_failing_the_run() {
    let server = MockServer::start().await;
    let registry = temp_registry("query-degraded");
    register_mock_source(&registry, "earth-search", &server).await;
    mount_search(&json!([])).mount(&server).await;

    // `bbox` narrows the request, so this source is worth asking even under `balanced`;
    // `semantic` has nothing this adapter can push for it, which is the degradation this
    // test is actually about.
    let query = temp_file(
        "degraded",
        r#"{"spatial": {"op": "bbox", "bbox": [20.85, 52.10, 21.25, 52.35]}, "semantic": "flood risk"}"#,
    );
    let output = geoquery(&[
        "--registry",
        registry.to_str().unwrap(),
        "query",
        "--query",
        query.to_str().unwrap(),
    ]);
    fs::remove_file(&query).ok();
    fs::remove_file(&registry).ok();

    assert_eq!(
        code_of(&output),
        0,
        "a degraded source is still a success: {}",
        stderr_of(&output)
    );
    let body: Value = serde_json::from_str(&stdout_of(&output)).unwrap();
    assert_eq!(body["status"], "ok");
    assert_eq!(body["sources"][0]["status"], "degraded");
    assert_eq!(body["sources"][0]["degradations"][0]["feature"], "semantic");
}

#[tokio::test]
async fn balanced_policy_tolerates_one_failing_source_among_several() {
    let healthy = MockServer::start().await;
    let registry = temp_registry("query-partial");
    register_mock_source(&registry, "healthy", &healthy).await;
    mount_search(&json!([])).mount(&healthy).await;

    // A second registered source that described cleanly but no longer answers by the
    // time the query runs. Registered against a real mock server first — discovery has
    // to succeed for `source add` to accept it — then its search endpoint is rewritten
    // to a port nothing listens on. Dropping the `MockServer` instead is not reliable
    // here: `cargo test` runs this file's tests in parallel, and the OS is free to hand
    // the freed ephemeral port straight to another test's server before this one's
    // request goes out.
    let flaky_landing = MockServer::start().await;
    register_mock_source(&registry, "flaky", &flaky_landing).await;
    break_search_endpoint(&registry, "flaky");

    let query = temp_file("partial", r#"{"limit": 1}"#);
    let output = geoquery(&[
        "--registry",
        registry.to_str().unwrap(),
        "query",
        "--query",
        query.to_str().unwrap(),
    ]);

    assert_eq!(
        code_of(&output),
        0,
        "balanced is the default and must not fail the whole run: {}",
        stderr_of(&output)
    );
    let body: Value = serde_json::from_str(&stdout_of(&output)).unwrap();
    assert_eq!(body["status"], "partial");
    assert_eq!(body["policy"], "balanced");
    let statuses: Vec<&str> = body["sources"]
        .as_array()
        .unwrap()
        .iter()
        .map(|source| source["status"].as_str().unwrap())
        .collect();
    assert!(statuses.contains(&"ok"), "{statuses:?}");
    assert!(
        statuses.iter().any(|status| *status != "ok"),
        "the flaky source must be visible, not hidden: {statuses:?}"
    );

    // Re-run the identical setup under `strict`: the same partial answer is not good
    // enough, and the shell must be able to tell.
    let strict_output = geoquery(&[
        "--registry",
        registry.to_str().unwrap(),
        "query",
        "--query",
        query.to_str().unwrap(),
        "--policy",
        "strict",
    ]);
    fs::remove_file(&query).ok();
    fs::remove_file(&registry).ok();

    assert_eq!(
        code_of(&strict_output),
        EXIT_QUERY_FAILED,
        "strict must not accept a partial run: {}",
        stdout_of(&strict_output)
    );
    let strict_body: Value = serde_json::from_str(&stdout_of(&strict_output))
        .expect("strict still prints the full JSON explanation, just with a failing exit code");
    assert_eq!(strict_body["status"], "partial");
}

#[test]
fn query_fails_cleanly_when_no_source_is_registered() {
    let registry = temp_registry("query-no-sources");
    let query = temp_file("no-sources", "{}");
    let output = geoquery(&[
        "--registry",
        registry.to_str().unwrap(),
        "query",
        "--query",
        query.to_str().unwrap(),
    ]);
    fs::remove_file(&query).ok();

    assert_eq!(code_of(&output), EXIT_SOURCE);
    assert!(stderr_of(&output).contains("no sources registered"));
}

#[tokio::test]
async fn query_names_a_requested_source_that_is_not_registered() {
    let server = MockServer::start().await;
    let registry = temp_registry("query-unknown-source");
    register_mock_source(&registry, "earth-search", &server).await;

    let query = temp_file("unknown-source", "{}");
    let output = geoquery(&[
        "--registry",
        registry.to_str().unwrap(),
        "query",
        "--query",
        query.to_str().unwrap(),
        "--source",
        "not-registered",
    ]);
    fs::remove_file(&query).ok();
    fs::remove_file(&registry).ok();

    assert_eq!(code_of(&output), EXIT_SOURCE);
    assert!(stderr_of(&output).contains("not-registered"));
}

#[test]
fn query_rejects_a_document_that_does_not_even_parse_as_a_geo_query() {
    let registry = temp_registry("query-invalid-document");
    let query = temp_file(
        "invalid-doc",
        r#"{"execution": {"mode": "not-a-real-mode"}}"#,
    );
    let output = geoquery(&[
        "--registry",
        registry.to_str().unwrap(),
        "query",
        "--query",
        query.to_str().unwrap(),
    ]);
    fs::remove_file(&query).ok();

    assert_eq!(code_of(&output), EXIT_INVALID);
}

#[test]
fn query_rejects_a_geo_query_that_parses_but_fails_validation() {
    let registry = temp_registry("query-fails-validate");
    // A `dwithin` predicate with a distance but no unit: shape is fine, meaning is not —
    // see `crates/types/tests/query.rs`'s `a_distance_without_a_unit_is_refused`.
    let query = temp_file(
        "fails-validate",
        r#"{"spatial": {"op": "dwithin", "geometry": {"type": "Point", "coordinates": [21.01, 52.23]}, "distance": 50000}}"#,
    );
    let output = geoquery(&[
        "--registry",
        registry.to_str().unwrap(),
        "query",
        "--query",
        query.to_str().unwrap(),
    ]);
    fs::remove_file(&query).ok();

    assert_eq!(code_of(&output), EXIT_INVALID, "{}", stderr_of(&output));
    assert!(stderr_of(&output).contains("not a valid query"));
}

// ── conformance-driven pushdown through the CLI ──────────────────────────────────

/// Register a source whose landing page advertises `classes` on top of the STAC core.
///
/// The conformance list is the whole point of these cases: it is what decides whether a
/// filter reaches the service, and it travels from the landing page into the registry and
/// back out at query time, which is a path no unit test covers.
async fn register_source_declaring(
    registry: &std::path::Path,
    id: &str,
    server: &MockServer,
    classes: &[&str],
) {
    let mut conforms_to = vec![
        "https://api.stacspec.org/v1.0.0/core".to_owned(),
        "https://api.stacspec.org/v1.0.0/item-search".to_owned(),
    ];
    conforms_to.extend(classes.iter().map(|class| (*class).to_owned()));
    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "type": "Catalog",
            "conformsTo": conforms_to,
            "links": [
                { "rel": "data", "href": format!("{}/collections", server.uri()) },
                { "rel": "search", "href": format!("{}/search", server.uri()), "method": "POST" }
            ]
        })))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/collections"))
        .respond_with(ResponseTemplate::new(200).set_body_json(collections_body()))
        .mount(server)
        .await;

    let add = geoquery(&[
        "--registry",
        registry.to_str().unwrap(),
        "source",
        "add",
        id,
        "--type",
        "stac",
        "--url",
        &server.uri(),
    ]);
    assert_eq!(
        code_of(&add),
        0,
        "fixture setup failed: {}",
        stderr_of(&add)
    );
}

const CLOUD_FILTER: &str =
    r#"{"filters": {"field": "eo:cloud_cover", "op": "<", "value": 10}, "limit": 5}"#;

#[tokio::test]
async fn a_filter_reaches_a_source_that_declared_cql2_json() {
    let server = MockServer::start().await;
    let registry = temp_registry("query-cql2");
    register_source_declaring(
        &registry,
        "planetary-computer",
        &server,
        &[
            "https://api.stacspec.org/v1.0.0-rc.2/item-search#filter",
            "http://www.opengis.net/spec/cql2/1.0/conf/cql2-json",
            "http://www.opengis.net/spec/cql2/1.0/conf/basic-cql2",
        ],
    )
    .await;
    mount_search(&json!([{
        "type": "Feature",
        "id": "item-1",
        "collection": "sentinel-2-l2a",
        "geometry": null,
        "properties": { "datetime": "2024-06-15T00:00:00Z", "eo:cloud_cover": 4 },
        "assets": {},
        "links": []
    }]))
    .mount(&server)
    .await;

    let query = temp_file("cql2", CLOUD_FILTER);
    let output = geoquery(&[
        "--registry",
        registry.to_str().unwrap(),
        "query",
        "--query",
        query.to_str().unwrap(),
    ]);
    fs::remove_file(&query).ok();
    fs::remove_file(&registry).ok();

    assert_eq!(code_of(&output), 0, "{}", stderr_of(&output));
    let body: Value = serde_json::from_str(&stdout_of(&output)).expect("query prints JSON");
    assert_eq!(body["status"], "ok");
    assert_eq!(body["sources"][0]["status"], "ok");
    assert_eq!(
        body["sources"][0]["pushed"]["filter"],
        json!({ "op": "<", "args": [{ "property": "eo:cloud_cover" }, 10] }),
        "the filter that was sent is visible as the request that was sent"
    );
    assert_eq!(body["sources"][0]["pushed"]["filter-lang"], "cql2-json");
    assert_eq!(body["results"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn a_filter_only_query_skips_a_source_that_cannot_push_it() {
    // The `balanced` rule: a source that can narrow nothing the query asked for is not
    // worth an unfiltered fetch. The same document against the service above is answered;
    // the difference is one conformance class, which is the point.
    let server = MockServer::start().await;
    let registry = temp_registry("query-no-cql2");
    register_source_declaring(&registry, "earth-search", &server, &[]).await;

    let query = temp_file("no-cql2", CLOUD_FILTER);
    let output = geoquery(&[
        "--registry",
        registry.to_str().unwrap(),
        "query",
        "--query",
        query.to_str().unwrap(),
    ]);
    fs::remove_file(&query).ok();
    fs::remove_file(&registry).ok();

    let body: Value = serde_json::from_str(&stdout_of(&output)).expect("query prints JSON");
    assert_eq!(body["sources"][0]["status"], "skipped");
    assert!(
        body["sources"][0]["reason"]
            .as_str()
            .unwrap()
            .contains("balanced policy"),
        "the skip says which policy decided it: {}",
        body["sources"][0]["reason"]
    );
}

#[tokio::test]
async fn exploratory_asks_a_source_that_balanced_would_have_skipped() {
    let server = MockServer::start().await;
    let registry = temp_registry("query-exploratory-filter");
    register_source_declaring(&registry, "earth-search", &server, &[]).await;
    mount_search(&json!([])).mount(&server).await;

    let query = temp_file("exploratory-filter", CLOUD_FILTER);
    let output = geoquery(&[
        "--registry",
        registry.to_str().unwrap(),
        "query",
        "--query",
        query.to_str().unwrap(),
        "--policy",
        "exploratory",
    ]);
    fs::remove_file(&query).ok();
    fs::remove_file(&registry).ok();

    assert_eq!(code_of(&output), 0, "{}", stderr_of(&output));
    let body: Value = serde_json::from_str(&stdout_of(&output)).expect("query prints JSON");
    assert_eq!(body["sources"][0]["status"], "degraded");
    assert_eq!(
        body["sources"][0]["degradations"][0]["feature"], "attribute-filter",
        "the refusal is reported rather than the query quietly running unfiltered"
    );
    assert!(
        !body["sources"][0]["pushed"]
            .as_object()
            .unwrap()
            .contains_key("filter"),
        "nothing the service did not declare was sent: {}",
        body["sources"][0]["pushed"]
    );
}
