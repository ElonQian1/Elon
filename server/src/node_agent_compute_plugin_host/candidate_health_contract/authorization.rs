use std::{error::Error as StdError, fmt};

use anyhow::Error;
use uuid::Uuid;

use super::ValidatedCandidateHealthPublication;
use crate::node_agent_compute_plugin_host::{
    install_plan_admission_validation::is_identifier,
    local_authority::{
        ComputePluginCandidateHealthAuthorityFacts, ComputePluginCandidateHealthAuthoritySession,
        ComputePluginFetchProcessFence, ComputePluginLocalAuthority,
    },
};

#[must_use = "authorized candidate health must be stored or returned for cleanup"]
pub(in crate::node_agent_compute_plugin_host) struct AuthorizedCandidateHealthStore<
    'root,
    'authority,
> {
    pub(super) publication: ValidatedCandidateHealthPublication<'root>,
    pub(super) authority_session: ComputePluginCandidateHealthAuthoritySession<'authority>,
    pub(super) facts: ComputePluginCandidateHealthAuthorityFacts,
    pub(super) health_id: String,
}

pub(in crate::node_agent_compute_plugin_host) struct CandidateHealthAuthorizationFailure<'root> {
    error: Error,
    publication: ValidatedCandidateHealthPublication<'root>,
}

pub(in crate::node_agent_compute_plugin_host) struct ValidatedCandidateHealthStorePermit<
    'permit,
    'root,
> {
    authorized: &'permit AuthorizedCandidateHealthStore<'root, 'permit>,
}

pub(in crate::node_agent_compute_plugin_host) fn authorize_candidate_health_store<
    'root,
    'authority,
>(
    publication: ValidatedCandidateHealthPublication<'root>,
    authority: &'authority ComputePluginLocalAuthority,
    process_fence: &'authority ComputePluginFetchProcessFence,
) -> std::result::Result<
    AuthorizedCandidateHealthStore<'root, 'authority>,
    CandidateHealthAuthorizationFailure<'root>,
> {
    match authorize(publication, authority, process_fence) {
        Ok(authorized) => Ok(authorized),
        Err((error, publication)) => {
            Err(CandidateHealthAuthorizationFailure { error, publication })
        }
    }
}

fn authorize<'root, 'authority>(
    publication: ValidatedCandidateHealthPublication<'root>,
    authority: &'authority ComputePluginLocalAuthority,
    process_fence: &'authority ComputePluginFetchProcessFence,
) -> std::result::Result<
    AuthorizedCandidateHealthStore<'root, 'authority>,
    (Error, ValidatedCandidateHealthPublication<'root>),
> {
    let authority_session = match authority
        .bind_candidate_health_authority_session(process_fence, publication.trusted_time())
    {
        Ok(session) => session,
        Err(error) => return Err((error, publication)),
    };
    let facts = match authority_session.read_candidate_health_binding(&publication) {
        Ok(facts) => facts,
        Err(error) => return Err((error, publication)),
    };
    let health_id = format!("chr_{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    if !is_identifier(&health_id) {
        return Err((
            anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_HEALTH_RECEIPT_ID_INVALID"),
            publication,
        ));
    }
    Ok(AuthorizedCandidateHealthStore {
        publication,
        authority_session,
        facts,
        health_id,
    })
}

impl<'permit, 'root> ValidatedCandidateHealthStorePermit<'permit, 'root> {
    pub(super) fn new(authorized: &'permit AuthorizedCandidateHealthStore<'root, 'permit>) -> Self {
        Self { authorized }
    }

    pub(in crate::node_agent_compute_plugin_host) fn publication(
        &self,
    ) -> &ValidatedCandidateHealthPublication<'root> {
        &self.authorized.publication
    }

    pub(in crate::node_agent_compute_plugin_host) fn facts(
        &self,
    ) -> &ComputePluginCandidateHealthAuthorityFacts {
        &self.authorized.facts
    }

    pub(in crate::node_agent_compute_plugin_host) fn health_id(&self) -> &str {
        &self.authorized.health_id
    }
}

impl<'root> CandidateHealthAuthorizationFailure<'root> {
    pub(in crate::node_agent_compute_plugin_host) fn into_parts(
        self,
    ) -> (Error, ValidatedCandidateHealthPublication<'root>) {
        (self.error, self.publication)
    }
}

impl fmt::Display for CandidateHealthAuthorizationFailure<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:#}", self.error)
    }
}

impl fmt::Debug for CandidateHealthAuthorizationFailure<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CandidateHealthAuthorizationFailure")
            .field("error", &self.error)
            .finish_non_exhaustive()
    }
}

impl StdError for CandidateHealthAuthorizationFailure<'_> {}

impl fmt::Debug for AuthorizedCandidateHealthStore<'_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AuthorizedCandidateHealthStore")
            .field("health_id", &"<redacted>")
            .field("facts", &self.facts)
            .finish_non_exhaustive()
    }
}
