use anyhow::{bail, Result};
use serde::Deserialize;

use crate::{
    compute_federation_activation_model::ComputeActivationEvidenceRequest,
    store::{Store, SupersedeComputeActivationEvidenceRequest},
};

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SupersedeComputeActivationEvidenceRequestBody {
    pub expected_request_digest: String,
    pub reason: String,
    pub confirm_supersede: bool,
}

pub(crate) fn supersede_for_review(
    store: &Store,
    actor_user_id: &str,
    request_id: &str,
    body: SupersedeComputeActivationEvidenceRequestBody,
) -> Result<ComputeActivationEvidenceRequest> {
    if !body.confirm_supersede {
        bail!("废止已批准激活证据申请前必须显式确认");
    }
    store.supersede_compute_activation_evidence_request(SupersedeComputeActivationEvidenceRequest {
        request_id: request_id.to_string(),
        expected_request_digest: body.expected_request_digest,
        actor_user_id: actor_user_id.to_string(),
        reason: body.reason,
    })
}
