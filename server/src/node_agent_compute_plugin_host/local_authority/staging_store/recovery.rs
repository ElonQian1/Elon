use anyhow::{bail, Result};
use rusqlite::Transaction;

use super::{
    recovery_session::ComputePluginCandidateStagingRecoveryAuthoritySession,
    types::HashedComputePluginCandidateStagingReceipt,
};
use crate::node_agent_compute_plugin_host::candidate_staging_contract::ComputePluginCandidateStagingRecoveryKey;

use super::super::plan_application::read_authority_plan_application_state;

mod row;
mod validation;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host) enum ComputePluginCandidateStagingRecoveryOutcome {
    NotCreated,
    Staged(HashedComputePluginCandidateStagingReceipt),
}

impl ComputePluginCandidateStagingRecoveryOutcome {
    pub(in crate::node_agent_compute_plugin_host) fn is_not_created(&self) -> bool {
        matches!(self, Self::NotCreated)
    }

    pub(in crate::node_agent_compute_plugin_host) fn staged_receipt(
        &self,
    ) -> Option<&HashedComputePluginCandidateStagingReceipt> {
        match self {
            Self::NotCreated => None,
            Self::Staged(receipt) => Some(receipt),
        }
    }
}

impl ComputePluginCandidateStagingRecoveryAuthoritySession<'_> {
    pub(in crate::node_agent_compute_plugin_host) fn read_candidate_staging_outcome(
        &self,
        key: &ComputePluginCandidateStagingRecoveryKey,
    ) -> Result<ComputePluginCandidateStagingRecoveryOutcome> {
        validation::validate_recovery_provenance(self, key)?;
        self.authority
            .with_deferred(|transaction| read_outcome(transaction, self, key))
    }
}

fn read_outcome(
    transaction: &Transaction<'_>,
    session: &ComputePluginCandidateStagingRecoveryAuthoritySession<'_>,
    key: &ComputePluginCandidateStagingRecoveryKey,
) -> Result<ComputePluginCandidateStagingRecoveryOutcome> {
    let exact = row::read_exact_candidate_staging(transaction, key)?;
    let identity_matches = row::count_candidate_staging_identity_matches(transaction, key)?;
    let authority = read_authority_plan_application_state(transaction, &session.trusted_now)?;
    match exact {
        Some(stored) => {
            if identity_matches != 1 {
                bail!("COMPUTE_PLUGIN_STAGING_RECOVERY_IDENTITY_COLLISION");
            }
            let receipt = validation::validate_staged_row(session, key, &authority, &stored)?;
            Ok(ComputePluginCandidateStagingRecoveryOutcome::Staged(
                receipt,
            ))
        }
        None => {
            if identity_matches != 0 {
                bail!("COMPUTE_PLUGIN_STAGING_RECOVERY_IDENTITY_COLLISION");
            }
            validation::validate_not_created(transaction, session, key, &authority)?;
            Ok(ComputePluginCandidateStagingRecoveryOutcome::NotCreated)
        }
    }
}
