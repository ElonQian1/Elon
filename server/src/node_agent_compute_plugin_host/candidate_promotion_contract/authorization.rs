use std::{error::Error as StdError, fmt, time::Instant};

use anyhow::Error;
use serde::Serialize;

use crate::node_agent_compute_plugin_host::{
    identity::ComputePluginReleaseRef,
    install_plan_admission_validation::is_identifier,
    local_authority::{
        ComputePluginCandidatePromotionAuthorityFacts,
        ComputePluginPostRevalidationPromotionAuthoritySession,
    },
    signed_artifact_verification::jcs_sha256_hex,
};

use super::{
    receipts::build_candidate_promotion_receipts, CandidatePromotionReceiptPair,
    RevalidatedCandidatePromotion,
};

const INSTALL_ID_BINDING_SCHEMA: &str = "elon.compute_plugin.install_id_binding.v1";
const PROMOTION_ID_BINDING_SCHEMA: &str = "elon.compute_plugin.promotion_id_binding.v1";

#[derive(Serialize)]
struct CandidatePromotionIdentifierBinding<'a> {
    schema: &'static str,
    installation_id_digest: &'a str,
    clock_epoch_digest: &'a str,
    candidate_token_digest: &'a str,
    plugin_id: &'a str,
    slot_ref: &'a str,
    release: &'a ComputePluginReleaseRef,
    owner_plan_id: &'a str,
    owner_plan_digest: &'a str,
    application_inventory_revision: i64,
    permission_grant_digest: &'a str,
    staging_receipt_digest: &'a str,
    health_receipt_digest: &'a str,
    candidate_generation: i64,
    install_generation_before: i64,
    install_generation_after: i64,
    activation_generation_before: i64,
    activation_generation_after: i64,
    authority_state_revision_before: i64,
    authority_state_revision_after: i64,
    inventory_revision_before: i64,
    inventory_revision_after: i64,
    inventory_digest_before: &'a str,
    inventory_digest_after: &'a str,
    authority_epoch_before: i64,
    authority_epoch_after: i64,
    process_owner_epoch: i64,
    trusted_time_high_water_ms_before: i64,
    authority_updated_at_ms_before: i64,
    promoted_at_ms: i64,
}

/// One authenticated and source-bound authority grant. It owns the revalidated candidate and is
/// deliberately non-cloneable; only its borrowed permit may cross the Store boundary.
#[must_use = "authorized candidate promotion must be stored or returned for cleanup"]
pub(in crate::node_agent_compute_plugin_host) struct AuthorizedCandidatePromotion<'root, 'authority>
{
    revalidated: RevalidatedCandidatePromotion<'root>,
    authority_session: ComputePluginPostRevalidationPromotionAuthoritySession<'authority>,
    facts: ComputePluginCandidatePromotionAuthorityFacts,
    install_id: String,
    promotion_id: String,
    receipts: CandidatePromotionReceiptPair,
}

pub(in crate::node_agent_compute_plugin_host) struct CandidatePromotionAuthorizationFailure<'root> {
    error: Error,
    revalidated: RevalidatedCandidatePromotion<'root>,
}

/// Borrowed, linear evidence that the exact revalidated custody was bound to current Store facts.
pub(in crate::node_agent_compute_plugin_host) struct ValidatedCandidatePromotionStorePermit<
    'permit,
    'root,
> {
    authorized: &'permit AuthorizedCandidatePromotion<'root, 'permit>,
}

pub(in crate::node_agent_compute_plugin_host) fn authorize_candidate_promotion<
    'root,
    'authority,
>(
    revalidated: RevalidatedCandidatePromotion<'root>,
    authority_session: ComputePluginPostRevalidationPromotionAuthoritySession<'authority>,
) -> std::result::Result<
    AuthorizedCandidatePromotion<'root, 'authority>,
    CandidatePromotionAuthorizationFailure<'root>,
> {
    match authorize(revalidated, authority_session) {
        Ok(authorized) => Ok(authorized),
        Err((error, revalidated)) => {
            Err(CandidatePromotionAuthorizationFailure { error, revalidated })
        }
    }
}

fn authorize<'root, 'authority>(
    revalidated: RevalidatedCandidatePromotion<'root>,
    authority_session: ComputePluginPostRevalidationPromotionAuthoritySession<'authority>,
) -> std::result::Result<
    AuthorizedCandidatePromotion<'root, 'authority>,
    (Error, RevalidatedCandidatePromotion<'root>),
> {
    let guard = revalidated.staged().archive().snapshot_cancellation_guard();
    if let Err(error) = revalidated.trusted_time().ensure_live(Instant::now()) {
        return Err((error, revalidated));
    }
    if !authority_session.was_observed_strictly_after(revalidated.revalidated_at())
        || authority_session.trusted_now_ms()
            != revalidated.trusted_time().trusted_now().timestamp_millis()
        || authority_session.installation_id_digest()
            != revalidated.trusted_time().installation_id_digest()
        || authority_session.clock_epoch_digest() != revalidated.trusted_time().clock_epoch_digest()
    {
        return Err((
            anyhow::anyhow!("COMPUTE_PLUGIN_PROMOTION_AUTHORITY_NOT_POST_REVALIDATION"),
            revalidated,
        ));
    }
    if let Err(error) = authority_session.validate_source(&guard) {
        return Err((error, revalidated));
    }
    let facts = match authority_session.read_candidate_promotion_binding(&revalidated) {
        Ok(facts) => facts,
        Err(error) => return Err((error, revalidated)),
    };
    let install_id = match derive_promotion_identifier(
        "cpi",
        INSTALL_ID_BINDING_SCHEMA,
        &authority_session,
        &facts,
    ) {
        Ok(value) => value,
        Err(error) => return Err((error, revalidated)),
    };
    let promotion_id = match derive_promotion_identifier(
        "cpp",
        PROMOTION_ID_BINDING_SCHEMA,
        &authority_session,
        &facts,
    ) {
        Ok(value) => value,
        Err(error) => return Err((error, revalidated)),
    };
    if !is_identifier(&install_id) || !is_identifier(&promotion_id) {
        return Err((
            anyhow::anyhow!("COMPUTE_PLUGIN_PROMOTION_RECEIPT_ID_INVALID"),
            revalidated,
        ));
    }
    let receipts = match build_candidate_promotion_receipts(
        &authority_session,
        &facts,
        &install_id,
        &promotion_id,
    ) {
        Ok(receipts) => receipts,
        Err(error) => return Err((error, revalidated)),
    };
    Ok(AuthorizedCandidatePromotion {
        revalidated,
        authority_session,
        facts,
        install_id,
        promotion_id,
        receipts,
    })
}

fn derive_promotion_identifier(
    prefix: &str,
    schema: &'static str,
    session: &ComputePluginPostRevalidationPromotionAuthoritySession<'_>,
    facts: &ComputePluginCandidatePromotionAuthorityFacts,
) -> anyhow::Result<String> {
    let binding = CandidatePromotionIdentifierBinding {
        schema,
        installation_id_digest: session.installation_id_digest(),
        clock_epoch_digest: session.clock_epoch_digest(),
        candidate_token_digest: facts.candidate_token_digest(),
        plugin_id: facts.plugin_id(),
        slot_ref: facts.slot_ref(),
        release: facts.release(),
        owner_plan_id: facts.owner_plan_id(),
        owner_plan_digest: facts.owner_plan_digest(),
        application_inventory_revision: facts.application_inventory_revision(),
        permission_grant_digest: facts.permission_grant_digest(),
        staging_receipt_digest: facts.staging_receipt_digest(),
        health_receipt_digest: facts.health_receipt_digest(),
        candidate_generation: facts.candidate_generation(),
        install_generation_before: facts.install_generation_before(),
        install_generation_after: facts.install_generation_after(),
        activation_generation_before: facts.activation_generation_before(),
        activation_generation_after: facts.activation_generation_after(),
        authority_state_revision_before: facts.authority_state_revision_before(),
        authority_state_revision_after: facts.authority_state_revision_after(),
        inventory_revision_before: facts.inventory_revision_before(),
        inventory_revision_after: facts.inventory_revision_after(),
        inventory_digest_before: facts.inventory_digest_before(),
        inventory_digest_after: facts.inventory_digest_after(),
        authority_epoch_before: facts.authority_epoch_before(),
        authority_epoch_after: facts.authority_epoch_after(),
        process_owner_epoch: facts.process_owner_epoch(),
        trusted_time_high_water_ms_before: facts.trusted_time_high_water_ms_before(),
        authority_updated_at_ms_before: facts.authority_updated_at_ms_before(),
        promoted_at_ms: facts.promoted_at_ms(),
    };
    Ok(format!("{prefix}_{}", jcs_sha256_hex(&binding)?))
}

impl<'permit, 'root> ValidatedCandidatePromotionStorePermit<'permit, 'root> {
    pub(super) fn new(authorized: &'permit AuthorizedCandidatePromotion<'root, 'permit>) -> Self {
        Self { authorized }
    }

    pub(in crate::node_agent_compute_plugin_host) fn revalidated(
        &self,
    ) -> &RevalidatedCandidatePromotion<'root> {
        &self.authorized.revalidated
    }

    pub(in crate::node_agent_compute_plugin_host) fn facts(
        &self,
    ) -> &ComputePluginCandidatePromotionAuthorityFacts {
        &self.authorized.facts
    }

    pub(in crate::node_agent_compute_plugin_host) fn install_id(&self) -> &str {
        &self.authorized.install_id
    }

    pub(in crate::node_agent_compute_plugin_host) fn promotion_id(&self) -> &str {
        &self.authorized.promotion_id
    }

    pub(in crate::node_agent_compute_plugin_host) fn receipts(
        &self,
    ) -> &CandidatePromotionReceiptPair {
        &self.authorized.receipts
    }
}

impl AuthorizedCandidatePromotion<'_, '_> {
    pub(super) fn authority_session(
        &self,
    ) -> &ComputePluginPostRevalidationPromotionAuthoritySession<'_> {
        &self.authority_session
    }

    pub(super) fn facts(&self) -> &ComputePluginCandidatePromotionAuthorityFacts {
        &self.facts
    }

    pub(super) fn install_id(&self) -> &str {
        &self.install_id
    }

    pub(super) fn promotion_id(&self) -> &str {
        &self.promotion_id
    }

    pub(super) fn receipts(&self) -> &CandidatePromotionReceiptPair {
        &self.receipts
    }
}

impl<'root> AuthorizedCandidatePromotion<'root, '_> {
    pub(super) fn into_parts(
        self,
    ) -> (
        RevalidatedCandidatePromotion<'root>,
        CandidatePromotionReceiptPair,
    ) {
        (self.revalidated, self.receipts)
    }
}

impl<'root> CandidatePromotionAuthorizationFailure<'root> {
    pub(in crate::node_agent_compute_plugin_host) fn into_parts(
        self,
    ) -> (Error, RevalidatedCandidatePromotion<'root>) {
        (self.error, self.revalidated)
    }
}

impl fmt::Display for CandidatePromotionAuthorizationFailure<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:#}", self.error)
    }
}

impl fmt::Debug for CandidatePromotionAuthorizationFailure<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CandidatePromotionAuthorizationFailure")
            .field("error", &self.error)
            .finish_non_exhaustive()
    }
}

impl StdError for CandidatePromotionAuthorizationFailure<'_> {}

impl fmt::Debug for AuthorizedCandidatePromotion<'_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AuthorizedCandidatePromotion")
            .field("install_id", &"<redacted>")
            .field("promotion_id", &"<redacted>")
            .field("facts", &self.facts)
            .finish_non_exhaustive()
    }
}
