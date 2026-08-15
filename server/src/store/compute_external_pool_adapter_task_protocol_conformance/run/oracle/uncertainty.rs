use anyhow::{bail, Result};

use super::super::support::{commit_uncertainty_marker_digest, ExchangeMaterial};

pub(super) enum CommitUncertaintyState {
    Clear,
    Pending(String),
    Resolved(String),
    NotApplicable,
}

pub(super) enum PostReceiptCommitUncertainty {
    None,
    MarkUnknown(String),
    MarkResolved(String),
}

pub(super) struct CommitUncertaintyObservation {
    pub(super) before: &'static str,
    pub(super) after: &'static str,
    pub(super) marker_digest: Option<String>,
    pub(super) post_receipt: PostReceiptCommitUncertainty,
}

impl CommitUncertaintyState {
    pub(super) fn for_scenario(scenario_id: &str) -> Result<Self> {
        match scenario_id {
            "synthetic_command_a" => Ok(Self::Clear),
            "synthetic_command_b" => Ok(Self::NotApplicable),
            _ => bail!("task conformance uncertainty scenario rejected"),
        }
    }

    pub(super) fn plan(
        &self,
        material: &ExchangeMaterial,
        request_digest: &str,
        remote_reference_digest: &str,
        remote_sequence: u64,
    ) -> Result<CommitUncertaintyObservation> {
        let observation = match (self, material.ordinal) {
            (Self::Clear, 1 | 2) => CommitUncertaintyObservation {
                before: "clear",
                after: "clear",
                marker_digest: None,
                post_receipt: PostReceiptCommitUncertainty::None,
            },
            (Self::Clear, 3) => {
                let marker = commit_uncertainty_marker_digest(
                    material,
                    request_digest,
                    remote_reference_digest,
                    remote_sequence,
                );
                CommitUncertaintyObservation {
                    before: "clear",
                    after: "unknown_after_remote_acceptance",
                    marker_digest: Some(marker.clone()),
                    post_receipt: PostReceiptCommitUncertainty::MarkUnknown(marker),
                }
            }
            (Self::Pending(marker), 4) => CommitUncertaintyObservation {
                before: "unknown_after_remote_acceptance",
                after: "resolved_by_reconcile",
                marker_digest: Some(marker.clone()),
                post_receipt: PostReceiptCommitUncertainty::MarkResolved(marker.clone()),
            },
            (Self::Resolved(marker), 5) if !marker.is_empty() => CommitUncertaintyObservation {
                before: "resolved_by_reconcile",
                after: "resolved_by_reconcile",
                marker_digest: None,
                post_receipt: PostReceiptCommitUncertainty::None,
            },
            (Self::NotApplicable, 6..=8) => CommitUncertaintyObservation {
                before: "not_applicable",
                after: "not_applicable",
                marker_digest: None,
                post_receipt: PostReceiptCommitUncertainty::None,
            },
            _ => bail!("task conformance commit uncertainty state gate rejected"),
        };
        Ok(observation)
    }

    pub(super) fn apply_after_receipt(
        &mut self,
        transition: PostReceiptCommitUncertainty,
    ) -> Result<()> {
        match transition {
            PostReceiptCommitUncertainty::None => Ok(()),
            PostReceiptCommitUncertainty::MarkUnknown(marker) => match self {
                Self::Clear => {
                    *self = Self::Pending(marker);
                    Ok(())
                }
                _ => bail!("task conformance uncertainty was not clear after commit replay"),
            },
            PostReceiptCommitUncertainty::MarkResolved(marker) => match self {
                Self::Pending(pending) if pending == &marker => {
                    *self = Self::Resolved(marker);
                    Ok(())
                }
                _ => bail!("task conformance reconcile did not consume pending uncertainty"),
            },
        }
    }
}
