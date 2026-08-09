use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::compute_plugin_sharing_directive::canonical_compute_plugin_ijson_and_sha256;

use super::{domain_json_and_digest, MAX_LEDGER_JSON_BYTES};

const APPLICATION_DIGEST_DOMAIN: &[u8] = b"ELON-COMPUTE-ATTEMPT-DISPATCH-APPLICATION-V1";

pub(crate) const COMPUTE_ATTEMPT_DISPATCH_APPLICATION_ACTION_V185_ACTIVATE: &str = "v185_activate";

/// Immutable facts proving which v185 activation one accepted Adapter ACK applied locally.
/// This envelope carries no bearer credential and grants no dispatch or lease authority.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeAttemptDispatchApplicationEnvelope {
    pub schema: String,
    pub application_id: String,
    pub application_digest: String,
    pub command_id: String,
    pub ack_id: String,
    pub action: String,
    pub lease_id: String,
    pub activation_request_digest: String,
    pub lease_digest: String,
    pub applied_at: String,
}

pub(crate) fn canonical_dispatch_application_json_and_digest(
    envelope: &ComputeAttemptDispatchApplicationEnvelope,
) -> Result<(String, String)> {
    #[derive(Serialize)]
    struct DigestProjection<'a> {
        schema: &'a str,
        application_id: &'a str,
        command_id: &'a str,
        ack_id: &'a str,
        action: &'a str,
        lease_id: &'a str,
        activation_request_digest: &'a str,
        lease_digest: &'a str,
        applied_at: &'a str,
    }

    let projection = DigestProjection {
        schema: &envelope.schema,
        application_id: &envelope.application_id,
        command_id: &envelope.command_id,
        ack_id: &envelope.ack_id,
        action: &envelope.action,
        lease_id: &envelope.lease_id,
        activation_request_digest: &envelope.activation_request_digest,
        lease_digest: &envelope.lease_digest,
        applied_at: &envelope.applied_at,
    };
    let (_, digest) = domain_json_and_digest(APPLICATION_DIGEST_DOMAIN, &projection)?;
    let (json, _) = canonical_compute_plugin_ijson_and_sha256(envelope, MAX_LEDGER_JSON_BYTES)?;
    Ok((json, digest))
}
