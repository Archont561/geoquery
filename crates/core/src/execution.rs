//! What one run across several sources amounts to.
//!
//! [`adapter`](crate::adapter) describes asking one service. This module describes the
//! aggregate: a merged set of results, and one [`SourceStatus`] for every source that was
//! part of the run — including the ones that failed and the ones that were never asked.
//!
//! The corpus rule is blunt: never hide source failures from applications, and let every
//! response carry per-source status. Federated search makes partial failure the normal
//! case rather than the exception, so an API that can only say "here are your results"
//! is an API that lies by omission every time one of five catalogues is down.
//!
//! What this module does *not* own is how a query gets planned or how sources get
//! chosen. That is the planner's, and it is deliberately absent here so the contract can
//! be settled before the implementation that will use it exists.

use std::fmt;

use geoquery_types::GeoResult;
use serde::{Deserialize, Serialize};

use crate::adapter::AdapterError;
use crate::capability::CapabilityFinding;

/// What became of one source in one run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum SourceOutcome {
    /// It answered, and everything asked of it was asked of it.
    Ok,
    /// It answered, but some of the query was carried out here instead.
    ///
    /// Still a success. The results are correct; they simply cost more to obtain than
    /// they would have from a service that could take the whole query.
    Degraded {
        /// What did not reach the service as written.
        findings: Vec<CapabilityFinding>,
    },
    /// It was asked and something went wrong.
    Failed {
        /// What went wrong.
        error: AdapterError,
    },
    /// It was never asked.
    ///
    /// Not an error — a decision, usually because the source could not take the query at
    /// all. Recorded rather than omitted so that the set of sources in a response is the
    /// set the planner considered, not the subset that happened to work out.
    Skipped {
        /// Why the planner left it out, in words a user can act on.
        reason: String,
    },
}

impl SourceOutcome {
    /// Whether this source produced usable results.
    ///
    /// [`Degraded`](SourceOutcome::Degraded) counts and [`Skipped`](SourceOutcome::Skipped)
    /// does not: the first answered the question, and the second was never asked it.
    #[must_use]
    pub fn succeeded(&self) -> bool {
        matches!(self, Self::Ok | Self::Degraded { .. })
    }
}

/// One source, and what became of it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SourceStatus {
    /// The source's stable name, matching
    /// [`Provenance::source`](geoquery_types::Provenance::source) on the results it
    /// produced. That correspondence is what lets a caller tie a result back to a status.
    pub source: String,
    /// What became of it.
    pub outcome: SourceOutcome,
}

impl SourceStatus {
    /// One word for this source, for the compact per-source map in a response.
    ///
    /// A failure summarizes as *what* went wrong rather than merely that something did,
    /// because the word is what tells a caller what to do next: a timeout invites a
    /// retry, an auth failure invites a credential, and a malformed response invites a
    /// bug report. `"failed"` would invite nothing.
    #[must_use]
    pub fn summary(&self) -> &'static str {
        match &self.outcome {
            SourceOutcome::Ok => "ok",
            SourceOutcome::Degraded { .. } => "degraded",
            SourceOutcome::Failed { error } => error.kind(),
            SourceOutcome::Skipped { .. } => "skipped",
        }
    }
}

/// How a whole run went.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExecutionStatus {
    /// Every source that was asked, answered.
    Ok,
    /// Some answered and some did not.
    Partial,
    /// None answered.
    Failed,
}

impl ExecutionStatus {
    /// The word that goes in a response body.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Partial => "partial",
            Self::Failed => "failed",
        }
    }
}

impl fmt::Display for ExecutionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Everything one run produced: the merged results, and what each source did.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ExecutionOutcome {
    /// Every result, normalized and merged, each carrying the provenance that says where
    /// it came from.
    #[serde(default)]
    pub results: Vec<GeoResult>,
    /// Every source the planner considered, answered or not.
    #[serde(default)]
    pub sources: Vec<SourceStatus>,
}

impl ExecutionOutcome {
    /// How the run went, worked out from the sources.
    ///
    /// Derived rather than stored, and that is the design: a status field alongside the
    /// list it summarizes is a second copy of the same fact, free to disagree with it
    /// after any edit. Here there is nowhere for the two to drift apart.
    ///
    /// A run with no sources is [`Failed`](ExecutionStatus::Failed), not
    /// [`Ok`](ExecutionStatus::Ok). An empty result list means the same thing in both
    /// cases and the status is the only thing that can distinguish a search that found
    /// nothing from a search that asked nobody.
    #[must_use]
    pub fn status(&self) -> ExecutionStatus {
        let answered = self
            .sources
            .iter()
            .filter(|status| status.outcome.succeeded())
            .count();
        if answered == 0 {
            ExecutionStatus::Failed
        } else if answered == self.sources.len() {
            ExecutionStatus::Ok
        } else {
            ExecutionStatus::Partial
        }
    }

    /// Whether any source answered with part of the query carried out locally.
    ///
    /// Separate from [`status`](ExecutionOutcome::status) on purpose. Degradation is not
    /// a lesser success — the results are correct — so it does not belong in the top
    /// line, and it must still be impossible to miss.
    #[must_use]
    pub fn is_degraded(&self) -> bool {
        self.sources
            .iter()
            .any(|status| matches!(status.outcome, SourceOutcome::Degraded { .. }))
    }

    /// Every degradation from every source, flattened.
    pub fn degradations(&self) -> impl Iterator<Item = &CapabilityFinding> {
        self.sources
            .iter()
            .filter_map(|status| match &status.outcome {
                SourceOutcome::Degraded { findings } => Some(findings.iter()),
                SourceOutcome::Ok
                | SourceOutcome::Failed { .. }
                | SourceOutcome::Skipped { .. } => None,
            })
            .flatten()
    }

    /// The sources something went wrong with.
    ///
    /// Excludes skipped sources: nothing went wrong with those.
    pub fn failures(&self) -> impl Iterator<Item = &SourceStatus> {
        self.sources
            .iter()
            .filter(|status| matches!(status.outcome, SourceOutcome::Failed { .. }))
    }

    /// How many results one source contributed.
    ///
    /// Counted from the provenance on the results rather than recorded on the status, for
    /// the same reason [`status`](ExecutionOutcome::status) is computed: a stored count is
    /// a second copy of a fact already in the data.
    #[must_use]
    pub fn contributed(&self, source: &str) -> usize {
        self.results
            .iter()
            .filter(|result| result.provenance.source == source)
            .count()
    }
}
