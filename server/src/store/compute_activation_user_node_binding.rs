use anyhow::Result;
use rusqlite::Transaction;

use super::{
    compute_activation_requests::SubmitComputeActivationEvidenceRequest,
    compute_user_node_provider_bindings::require_user_node_provider_activation_binding_on,
};

pub(super) fn require_submission_binding_on(
    transaction: &Transaction<'_>,
    input: &SubmitComputeActivationEvidenceRequest,
) -> Result<()> {
    require_user_node_provider_activation_binding_on(
        transaction,
        &input.provider_id,
        &input.node_binding_ref,
        &input.owner_user_id,
        input.expected_provider_policy_revision,
        &input.expected_provider_digest,
    )
}
