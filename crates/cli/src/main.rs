//! The `geoquery` command-line client.
//!
//! A presentation layer: parse arguments, ask `geoquery-core` and the registered adapters
//! what a document means and what a service answers, print the result as JSON, and pick
//! an exit code. Every rule about what a query *means* belongs to `geoquery-types` and
//! `geoquery-core`; every rule about what a *source* is belongs to an adapter crate. This
//! binary only wires the three together and phrases what they say for a terminal.
//!
//! Phase 1 is one live adapter — STAC — reached through a small local source registry.
//! `geoquery source add|list|describe` manages that registry; `geoquery query` runs a
//! `GeoQuery` document against the sources it names, federated if it names more than one,
//! and prints normalized results with provenance and per-source status. `geoquery check`
//! is unchanged from Phase 0: it reads a query document and says what it contains,
//! without contacting anything.

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use chrono::{DateTime, Utc};
use clap::{Parser, Subcommand};
use geoquery_adapter_stac::StacAdapter;
use geoquery_core::{
    CapabilityFinding, Endpoint, ExecutionStatus, QueryDocument, QueryDocumentError, QueryFeature,
    ServiceAdapter, SourceOutcome, SourceStatus, Support,
};
use geoquery_types::{ExecutionPolicy, GeoQuery, ServiceDescriptor, ServiceType};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// The document could not be read.
const EXIT_IO: u8 = 1;
/// The document is not a query document, or is one `GeoQuery` refuses.
const EXIT_INVALID: u8 = 2;
/// A registry or source problem: an unknown id, an unsupported type, a missing file.
const EXIT_SOURCE: u8 = 3;
/// The federated run itself failed: nobody answered, or `strict` would not accept what
/// did.
const EXIT_QUERY_FAILED: u8 = 4;

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
    /// Where the source registry lives.
    ///
    /// Global rather than per-subcommand: every command that touches sources should
    /// accept the same override, and a flag declared once cannot drift between them.
    /// Defaults to `$GEOQUERY_HOME/sources.json` when `GEOQUERY_HOME` is set, otherwise
    /// `~/.geoquery/sources.json`.
    #[arg(long, global = true, value_name = "PATH")]
    registry: Option<PathBuf>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Read a query document, report what it contains, contact nothing.
    Check {
        /// Path to the JSON query document.
        query: PathBuf,
    },
    /// Manage the local registry of queryable sources.
    Source {
        #[command(subcommand)]
        action: SourceCommand,
    },
    /// Run a query document against one or more registered sources.
    Query {
        /// Path to the JSON `GeoQuery` document.
        #[arg(long, value_name = "PATH")]
        query: PathBuf,
        /// A registered source to query. Repeatable; every registered source is used
        /// when none is named.
        #[arg(long = "source", value_name = "ID")]
        sources: Vec<String>,
        /// How tolerant the run is of a source that cannot fully honour the query.
        /// Overrides the document's own `execution.policy` when given. Defaults to
        /// `balanced`.
        #[arg(long, value_parser = parse_policy)]
        policy: Option<ExecutionPolicy>,
    },
}

#[derive(Debug, Subcommand)]
enum SourceCommand {
    /// Discover a service and register it under a name.
    Add {
        /// Name to register the source under.
        id: String,
        /// Source protocol. Only `stac` is implemented in this build.
        #[arg(long = "type", value_name = "TYPE")]
        kind: String,
        /// The service's base URL.
        #[arg(long, value_name = "URL")]
        url: String,
    },
    /// List every registered source.
    List,
    /// Print everything discovered about one registered source.
    Describe {
        /// The registered source's name.
        id: String,
    },
}

/// Parse `--policy`, naming the value that was rejected in the error clap prints.
fn parse_policy(text: &str) -> Result<ExecutionPolicy, String> {
    match text {
        "strict" => Ok(ExecutionPolicy::Strict),
        "balanced" => Ok(ExecutionPolicy::Balanced),
        "exploratory" => Ok(ExecutionPolicy::Exploratory),
        other => Err(format!(
            "`{other}` is not a policy; expected `strict`, `balanced` or `exploratory`"
        )),
    }
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

/// What a successful run prints, and the exit code it earns.
///
/// Separate from an outright error because `query` can succeed at printing a complete,
/// honest answer — normalized results, every source's status — and still owe the shell a
/// nonzero code, when that answer is a partial one `strict` was asked not to accept. A
/// bare `String` cannot carry that; collapsing the two into "ok means zero" would mean
/// choosing between hiding the JSON or hiding the failure, and the design corpus rules
/// out both.
struct Report {
    message: String,
    code: u8,
}

impl Report {
    fn ok(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            code: 0,
        }
    }

    fn with_code(message: impl Into<String>, code: u8) -> Self {
        Self {
            message: message.into(),
            code,
        }
    }
}

fn main() -> ExitCode {
    let runtime = tokio::runtime::Runtime::new().expect("the Tokio runtime starts");
    match runtime.block_on(run(Cli::parse())) {
        Ok(report) => {
            println!("{}", report.message);
            ExitCode::from(report.code)
        }
        Err(failure) => {
            eprintln!("geoquery: {failure}", failure = failure.message);
            ExitCode::from(failure.code)
        }
    }
}

async fn run(cli: Cli) -> Result<Report, Failure> {
    let registry_path = cli.registry.unwrap_or_else(default_registry_path);
    match cli.command {
        Command::Check { query } => check(&query).map(Report::ok),
        Command::Source { action } => source_command(&registry_path, action).await,
        Command::Query {
            query,
            sources,
            policy,
        } => run_query(&registry_path, &query, &sources, policy).await,
    }
}

/// Where the registry lives when `--registry` was not given.
fn default_registry_path() -> PathBuf {
    if let Ok(home) = std::env::var("GEOQUERY_HOME") {
        return PathBuf::from(home).join("sources.json");
    }
    let base = std::env::var_os("HOME").map_or_else(|| PathBuf::from("."), PathBuf::from);
    base.join(".geoquery").join("sources.json")
}

/// Report the shape of a query document without sending it anywhere.
fn check(path: &Path) -> Result<String, Failure> {
    let document =
        QueryDocument::read(path).map_err(|error| describe_document_error(path, &error))?;
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
fn describe_document_error(path: &Path, error: &QueryDocumentError) -> Failure {
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

// ── the source registry ──────────────────────────────────────────────────────────

/// The registry file: every source this installation has registered, by name.
///
/// Plain JSON rather than the YAML the longer-term design in
/// `.knowledge/infrastructure/storage.md` describes — this spike keeps the format a
/// `serde_json::Value` away from every other structure in the binary, and a format
/// migration is cheap exactly because nothing outside this file reads it directly.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct Registry {
    #[serde(default)]
    sources: BTreeMap<String, SourceEntry>,
}

/// One registered source: when it was added, and everything discovery found.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SourceEntry {
    #[serde(rename = "addedAt")]
    added_at: DateTime<Utc>,
    descriptor: ServiceDescriptor,
}

impl Registry {
    /// Load the registry, or an empty one if the file has never been written.
    fn load(path: &Path) -> Result<Self, Failure> {
        match std::fs::read_to_string(path) {
            Ok(text) => serde_json::from_str(&text).map_err(|error| {
                Failure::new(
                    format!("{} is not a valid registry file: {error}", path.display()),
                    EXIT_INVALID,
                )
            }),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Self::default()),
            Err(error) => Err(Failure::new(
                format!("cannot read {}: {error}", path.display()),
                EXIT_IO,
            )),
        }
    }

    /// Persist the registry, creating its parent directory if this is the first source.
    fn save(&self, path: &Path) -> Result<(), Failure> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| {
                Failure::new(
                    format!("cannot create {}: {error}", parent.display()),
                    EXIT_IO,
                )
            })?;
        }
        let text = serde_json::to_string_pretty(self).expect("a registry always serializes");
        std::fs::write(path, text).map_err(|error| {
            Failure::new(format!("cannot write {}: {error}", path.display()), EXIT_IO)
        })
    }
}

async fn source_command(registry_path: &Path, action: SourceCommand) -> Result<Report, Failure> {
    match action {
        SourceCommand::Add { id, kind, url } => source_add(registry_path, &id, &kind, &url)
            .await
            .map(Report::ok),
        SourceCommand::List => source_list(registry_path).map(Report::ok),
        SourceCommand::Describe { id } => source_describe(registry_path, &id).map(Report::ok),
    }
}

async fn source_add(
    registry_path: &Path,
    id: &str,
    kind: &str,
    url: &str,
) -> Result<String, Failure> {
    if kind != "stac" {
        return Err(Failure::new(
            format!("unsupported source type `{kind}`: only `stac` is implemented in this build"),
            EXIT_SOURCE,
        ));
    }

    let adapter = StacAdapter::new();
    let endpoint = Endpoint::new(url.to_owned());
    let mut descriptor = adapter
        .describe(&endpoint)
        .await
        .map_err(|error| Failure::new(format!("could not register {url}: {error}"), EXIT_SOURCE))?;
    descriptor.id = Some(id.to_owned());

    let mut registry = Registry::load(registry_path)?;
    registry.sources.insert(
        id.to_owned(),
        SourceEntry {
            added_at: Utc::now(),
            descriptor: descriptor.clone(),
        },
    );
    registry.save(registry_path)?;

    let collections = descriptor.collections.len();
    Ok(format!(
        "registered `{id}` ({url}) as stac: {collections} collection{} found",
        if collections == 1 { "" } else { "s" }
    ))
}

fn source_list(registry_path: &Path) -> Result<String, Failure> {
    let registry = Registry::load(registry_path)?;
    if registry.sources.is_empty() {
        return Ok("no sources registered; see `geoquery source add --help`".to_owned());
    }
    let mut lines = vec![format!(
        "{:<20} {:<6} {:<11} URL",
        "NAME", "TYPE", "COLLECTIONS"
    )];
    for (id, entry) in &registry.sources {
        lines.push(format!(
            "{:<20} {:<6} {:<11} {}",
            id,
            entry.descriptor.r#type,
            entry.descriptor.collections.len(),
            entry.descriptor.url,
        ));
    }
    Ok(lines.join("\n"))
}

fn source_describe(registry_path: &Path, id: &str) -> Result<String, Failure> {
    let registry = Registry::load(registry_path)?;
    let entry = registry.sources.get(id).ok_or_else(|| {
        Failure::new(
            format!("no source named `{id}` is registered; run `geoquery source list`"),
            EXIT_SOURCE,
        )
    })?;
    Ok(serde_json::to_string_pretty(entry).expect("a registered source always serializes"))
}

// ── running a query ──────────────────────────────────────────────────────────────

async fn run_query(
    registry_path: &Path,
    query_path: &Path,
    requested_sources: &[String],
    policy_override: Option<ExecutionPolicy>,
) -> Result<Report, Failure> {
    let text = std::fs::read_to_string(query_path).map_err(|error| {
        Failure::new(
            format!("cannot read {}: {error}", query_path.display()),
            EXIT_IO,
        )
    })?;
    let query: GeoQuery = serde_json::from_str(&text).map_err(|error| {
        Failure::new(
            format!(
                "{} is not a valid GeoQuery document: {error}",
                query_path.display()
            ),
            EXIT_INVALID,
        )
    })?;
    if let Err(problems) = query.validate() {
        let detail = problems
            .iter()
            .map(std::string::ToString::to_string)
            .collect::<Vec<_>>()
            .join("; ");
        return Err(Failure::new(
            format!("{} is not a valid query: {detail}", query_path.display()),
            EXIT_INVALID,
        ));
    }

    let registry = Registry::load(registry_path)?;
    let target_ids = resolve_targets(&registry, requested_sources)?;
    let policy = policy_override.unwrap_or_else(|| {
        query.execution.as_ref().map_or(
            ExecutionPolicy::Balanced,
            geoquery_types::ExecutionOptions::policy,
        )
    });

    let mut outcome = geoquery_core::ExecutionOutcome::default();
    let mut reports = Vec::new();

    for id in target_ids {
        // Already validated by `resolve_targets`.
        let descriptor = registry.sources[&id].descriptor.clone();
        run_one_source(&id, &descriptor, &query, policy, &mut outcome, &mut reports).await;
    }

    // `ExecutionOutcome::status` is the one place "did this run go well" is computed, so
    // this binary reads it rather than re-deriving the same answer from `outcome.sources`
    // a second, independently-maintained way.
    let status = outcome.status();
    let strict_violation = policy == ExecutionPolicy::Strict
        && outcome
            .sources
            .iter()
            .any(|s| !matches!(s.outcome, SourceOutcome::Ok));

    let body = json!({
        "status": status.as_str(),
        "policy": policy_word(policy),
        "results": outcome.results,
        "sources": reports,
    });
    let message = serde_json::to_string_pretty(&body).expect("the response is serializable");

    let code = if status == ExecutionStatus::Failed || strict_violation {
        EXIT_QUERY_FAILED
    } else {
        0
    };
    Ok(Report::with_code(message, code))
}

/// Query every registered source named, or every registered source when none was named.
///
/// # Errors
///
/// [`Failure`] naming a requested source that is not registered, so the message quotes
/// the one id that was wrong rather than listing the whole registry.
fn resolve_targets(registry: &Registry, requested: &[String]) -> Result<Vec<String>, Failure> {
    if requested.is_empty() {
        if registry.sources.is_empty() {
            return Err(Failure::new(
                "no sources registered; run `geoquery source add` first",
                EXIT_SOURCE,
            ));
        }
        return Ok(registry.sources.keys().cloned().collect());
    }
    for id in requested {
        if !registry.sources.contains_key(id) {
            return Err(Failure::new(
                format!("no source named `{id}` is registered; run `geoquery source list`"),
                EXIT_SOURCE,
            ));
        }
    }
    Ok(requested.to_vec())
}

/// Run `query` against one source, recording its outcome into `results`, `sources` and
/// `reports` — the canonical status, and the richer JSON this binary prints,
/// respectively.
async fn run_one_source(
    id: &str,
    descriptor: &ServiceDescriptor,
    query: &GeoQuery,
    policy: ExecutionPolicy,
    outcome: &mut geoquery_core::ExecutionOutcome,
    reports: &mut Vec<Value>,
) {
    if descriptor.r#type != ServiceType::Stac {
        let reason = format!(
            "no adapter implemented for `{}` sources in this build",
            descriptor.r#type
        );
        outcome.sources.push(SourceStatus {
            source: id.to_owned(),
            outcome: SourceOutcome::Skipped {
                reason: reason.clone(),
            },
        });
        reports.push(json!({ "source": id, "status": "skipped", "reason": reason }));
        return;
    }

    let (_, findings) = geoquery_adapter_stac::translate(
        query,
        geoquery_adapter_stac::StacConformance::for_service(descriptor),
    );
    if let Some(reason) = skip_reason(policy, query, &findings) {
        outcome.sources.push(SourceStatus {
            source: id.to_owned(),
            outcome: SourceOutcome::Skipped {
                reason: reason.clone(),
            },
        });
        reports.push(json!({ "source": id, "status": "skipped", "reason": reason }));
        return;
    }

    let adapter = StacAdapter::new();
    match adapter.query(descriptor, query).await {
        Ok(result) => {
            let count = result.results.len();
            let pushed = Value::Object(result.provenance.query.clone());
            let duration_ms = result.provenance.duration_ms;
            let degraded = !result.degradations.is_empty();
            reports.push(json!({
                "source": id,
                "status": if degraded { "degraded" } else { "ok" },
                "pushed": pushed,
                "degradations": result.degradations,
                "count": count,
                "durationMs": duration_ms,
            }));
            outcome.sources.push(SourceStatus {
                source: id.to_owned(),
                outcome: if degraded {
                    SourceOutcome::Degraded {
                        findings: result.degradations,
                    }
                } else {
                    SourceOutcome::Ok
                },
            });
            outcome.results.extend(result.results);
        }
        Err(error) => {
            reports.push(json!({
                "source": id,
                "status": error.kind(),
                "error": error.to_string(),
            }));
            outcome.sources.push(SourceStatus {
                source: id.to_owned(),
                outcome: SourceOutcome::Failed { error },
            });
        }
    }
}

/// Whether a source should be asked at all, given the execution policy and what
/// [`geoquery_adapter_stac::translate`] found it could push.
///
/// `strict` refuses any source the query cannot be carried out on exactly as written;
/// `balanced` (the default) still asks a source that can push *something*, but skips one
/// that would otherwise be asked to return its whole catalog unfiltered; `exploratory`
/// always asks, on the chance an under-described capability still works.
fn skip_reason(
    policy: ExecutionPolicy,
    query: &GeoQuery,
    findings: &[CapabilityFinding],
) -> Option<String> {
    let unsupported: Vec<String> = findings
        .iter()
        .filter(|finding| finding.support != Support::Pushed)
        .map(|finding| finding.feature.to_string())
        .collect();

    match policy {
        ExecutionPolicy::Strict => (!unsupported.is_empty()).then(|| {
            format!(
                "strict policy: this source cannot fully honour the query ({})",
                unsupported.join(", ")
            )
        }),
        ExecutionPolicy::Balanced => {
            let asked_for_narrowing = query.spatial.is_some()
                || query.temporal.is_some()
                || query.filters.is_some()
                || query.semantic.is_some();
            let pushed_narrowing = findings.iter().any(|finding| {
                finding.support == Support::Pushed
                    && matches!(
                        finding.feature,
                        // Attribute filtering belongs here for the same reason the other
                        // two do: it is a predicate that narrows what comes back. It was
                        // absent while nothing could push one — a list of features that
                        // no adapter could ever satisfy would have made this rule unfalsi-
                        // fiable — and a source that accepts a CQL2 filter is now exactly
                        // the source `balanced` exists to keep, not to skip.
                        QueryFeature::Spatial(_)
                            | QueryFeature::Temporal(_)
                            | QueryFeature::AttributeFilter
                    )
            });
            (asked_for_narrowing && !pushed_narrowing).then(|| {
                "balanced policy: none of the requested predicates can be pushed to this \
                 source, skipped to avoid an unfiltered fetch"
                    .to_owned()
            })
        }
        ExecutionPolicy::Exploratory => None,
    }
}

fn policy_word(policy: ExecutionPolicy) -> &'static str {
    match policy {
        ExecutionPolicy::Strict => "strict",
        ExecutionPolicy::Balanced => "balanced",
        ExecutionPolicy::Exploratory => "exploratory",
    }
}
