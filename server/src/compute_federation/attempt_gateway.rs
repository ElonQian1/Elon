use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::compute_plugin_sharing_directive::canonical_compute_plugin_ijson_and_sha256;

use super::{
    capacity::ComputeCapacityClaimBinding,
    execution::{ComputeJobVersionBinding, ComputeOfferBinding},
    start_outbox::{
        ValidatedComputeStartOutboxOperation, VerifiedComputeStartOutboxRemoteObservation,
    },
};

pub(crate) const COMPUTE_ATTEMPT_DISPATCH_COMMAND_SCHEMA: &str =
    "compute_federation.attempt_dispatch_command.v1";
pub(crate) const COMPUTE_ATTEMPT_ADAPTER_BINDING_SCHEMA: &str =
    "compute_federation.attempt_adapter_binding.v1";
pub(crate) const COMPUTE_ATTEMPT_ADAPTER_ACK_SCHEMA: &str =
    "compute_federation.attempt_adapter_ack.v1";
pub(crate) const COMPUTE_ATTEMPT_DISPATCH_APPLICATION_SCHEMA: &str =
    "compute_federation.attempt_dispatch_application.v1";
pub(crate) const COMPUTE_ATTEMPT_DISPATCH_COMMAND_TYPE_START: &str = "start";
pub(crate) const COMPUTE_ATTEMPT_ADAPTER_ACK_ACCEPTED: &str = "accepted";
pub(crate) const COMPUTE_ATTEMPT_ADAPTER_ACK_REJECTED: &str = "rejected";
pub(crate) const COMPUTE_ATTEMPT_ROUTE_PROVIDER_ENDPOINT: &str = "provider_endpoint";
pub(crate) const COMPUTE_ATTEMPT_ROUTE_SERVER_ADAPTER: &str = "server_adapter";

const MAX_LEDGER_JSON_BYTES: usize = 512 * 1024;
const COMMAND_DIGEST_DOMAIN: &[u8] = b"ELON-COMPUTE-ATTEMPT-DISPATCH-COMMAND-V1";
const ADAPTER_BINDING_DIGEST_DOMAIN: &[u8] = b"ELON-COMPUTE-ATTEMPT-ADAPTER-BINDING-V1";
const ADAPTER_ACK_DIGEST_DOMAIN: &[u8] = b"ELON-COMPUTE-ATTEMPT-ADAPTER-ACK-V1";

/// Provider-neutral command identity. The referenced execution plan is immutable, but this
/// contract deliberately does not pretend that the current Job/Offer can already be projected
/// into a runnable node Host command or an external-pool request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeAttemptDispatchCommandEnvelope {
    pub schema: String,
    pub command_id: String,
    pub command_digest: String,
    pub issued_at: String,
    pub not_after: String,
    pub command: ComputeAttemptStartDispatchCommand,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeAttemptStartDispatchCommand {
    pub command_type: String,
    pub identity: ComputeAttemptDispatchIdentity,
    pub provider: ComputeAttemptProviderVersionBinding,
    pub offer: ComputeOfferBinding,
    pub job: ComputeJobVersionBinding,
    pub reservation: ComputeAttemptReservationVersionBinding,
    pub capacity_claim: ComputeCapacityClaimBinding,
    pub executor_id: String,
    pub execution_plan: ComputeAttemptExecutionPlanBinding,
    pub lease_expires_at: String,
    pub hard_deadline_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeAttemptDispatchIdentity {
    pub job_id: String,
    pub reservation_id: String,
    pub attempt_lease_id: String,
    pub attempt_no: i64,
    pub shard_id: Option<String>,
    pub fencing_generation: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeAttemptProviderVersionBinding {
    pub provider_id: String,
    pub policy_revision: i64,
    pub provider_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeAttemptReservationVersionBinding {
    pub reservation_id: String,
    pub reservation_revision: i64,
    pub reservation_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeAttemptExecutionPlanBinding {
    pub plan_id: String,
    pub plan_schema: String,
    pub plan_digest: String,
}

/// Exact server-side adapter configuration selected for one immutable command. An endpoint route
/// binds the current Provider endpoint plus a future registry-owned driver; a server-adapter route
/// binds the Provider's current adapter. Credentials never enter this binding or command JSON.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeAttemptAdapterBinding {
    pub schema: String,
    pub provider_id: String,
    pub provider_kind: String,
    pub route_kind: String,
    pub endpoint_id: Option<String>,
    pub endpoint_transport: Option<String>,
    pub adapter_id: String,
    pub adapter_version: String,
    pub config_revision: i64,
    pub config_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeAttemptAdapterAckEnvelope {
    pub schema: String,
    pub ack_id: String,
    pub adapter_ack_id: String,
    pub command_id: String,
    pub command_digest: String,
    pub adapter_binding_digest: String,
    pub outcome: String,
    pub remote_execution_ref: Option<String>,
    pub reason_code: Option<String>,
    pub observed_at: String,
    pub received_at: String,
    pub ack_digest: String,
}

/// Server-only activation material. `lease_credential_ref` is never serialized into the Provider
/// command. A future execution-plan producer and concrete Adapter must jointly validate this
/// object before they can construct `ValidatedComputeAttemptStartDispatch`.
pub(crate) struct ComputeAttemptStartActivationPlan {
    lease_credential_ref: String,
    lease_credential_hint: String,
    idempotency_key: String,
    activated_by_user_id: String,
}

/// Sealed evidence that an execution plan, Provider binding and Adapter route were validated.
/// There is intentionally no constructor in the current batch; the default registry is empty.
pub(crate) struct ValidatedComputeAttemptStartDispatch {
    command: ComputeAttemptDispatchCommandEnvelope,
    adapter: ComputeAttemptAdapterBinding,
    activation: ComputeAttemptStartActivationPlan,
    prepare_outbox: ValidatedComputeStartOutboxOperation,
}

impl ValidatedComputeAttemptStartDispatch {
    pub(crate) fn command(&self) -> &ComputeAttemptDispatchCommandEnvelope {
        &self.command
    }

    pub(crate) fn adapter(&self) -> &ComputeAttemptAdapterBinding {
        &self.adapter
    }

    pub(crate) fn activation(&self) -> &ComputeAttemptStartActivationPlan {
        &self.activation
    }

    pub(crate) fn prepare_outbox(&self) -> &ValidatedComputeStartOutboxOperation {
        &self.prepare_outbox
    }
}

impl ComputeAttemptStartActivationPlan {
    pub(crate) fn lease_credential_ref(&self) -> &str {
        &self.lease_credential_ref
    }

    pub(crate) fn lease_credential_hint(&self) -> &str {
        &self.lease_credential_hint
    }

    pub(crate) fn idempotency_key(&self) -> &str {
        &self.idempotency_key
    }

    pub(crate) fn activated_by_user_id(&self) -> &str {
        &self.activated_by_user_id
    }
}

/// Adapter-authenticated response. An `accepted` response is only provisional remote acceptance;
/// it becomes platform-applied solely when the ACK, v185 activation and application receipt commit
/// atomically. Construction remains inside this module's future concrete Adapter children.
pub(crate) struct VerifiedComputeAttemptAdapterAck {
    adapter: ComputeAttemptAdapterBinding,
    ack: ComputeAttemptAdapterAckEnvelope,
    prepare_observation: VerifiedComputeStartOutboxRemoteObservation,
}

impl VerifiedComputeAttemptAdapterAck {
    pub(crate) fn adapter(&self) -> &ComputeAttemptAdapterBinding {
        &self.adapter
    }

    pub(crate) fn ack(&self) -> &ComputeAttemptAdapterAckEnvelope {
        &self.ack
    }

    pub(crate) fn prepare_observation(&self) -> &VerifiedComputeStartOutboxRemoteObservation {
        &self.prepare_observation
    }
}

pub(crate) fn canonical_dispatch_command_json_and_digest(
    envelope: &ComputeAttemptDispatchCommandEnvelope,
) -> Result<(String, String)> {
    #[derive(Serialize)]
    struct DigestProjection<'a> {
        schema: &'a str,
        command_id: &'a str,
        issued_at: &'a str,
        not_after: &'a str,
        command: &'a ComputeAttemptStartDispatchCommand,
    }
    let projection = DigestProjection {
        schema: &envelope.schema,
        command_id: &envelope.command_id,
        issued_at: &envelope.issued_at,
        not_after: &envelope.not_after,
        command: &envelope.command,
    };
    let (_, digest) = domain_json_and_digest(COMMAND_DIGEST_DOMAIN, &projection)?;
    let (json, _) = canonical_compute_plugin_ijson_and_sha256(envelope, MAX_LEDGER_JSON_BYTES)?;
    Ok((json, digest))
}

pub(crate) fn canonical_adapter_binding_json_and_digest(
    binding: &ComputeAttemptAdapterBinding,
) -> Result<(String, String)> {
    domain_json_and_digest(ADAPTER_BINDING_DIGEST_DOMAIN, binding)
}

pub(crate) fn canonical_adapter_ack_json_and_digest(
    ack: &ComputeAttemptAdapterAckEnvelope,
) -> Result<(String, String)> {
    #[derive(Serialize)]
    struct DigestProjection<'a> {
        schema: &'a str,
        ack_id: &'a str,
        adapter_ack_id: &'a str,
        command_id: &'a str,
        command_digest: &'a str,
        adapter_binding_digest: &'a str,
        outcome: &'a str,
        remote_execution_ref: &'a Option<String>,
        reason_code: &'a Option<String>,
        observed_at: &'a str,
        received_at: &'a str,
    }
    let projection = DigestProjection {
        schema: &ack.schema,
        ack_id: &ack.ack_id,
        adapter_ack_id: &ack.adapter_ack_id,
        command_id: &ack.command_id,
        command_digest: &ack.command_digest,
        adapter_binding_digest: &ack.adapter_binding_digest,
        outcome: &ack.outcome,
        remote_execution_ref: &ack.remote_execution_ref,
        reason_code: &ack.reason_code,
        observed_at: &ack.observed_at,
        received_at: &ack.received_at,
    };
    let (_, digest) = domain_json_and_digest(ADAPTER_ACK_DIGEST_DOMAIN, &projection)?;
    let (json, _) = canonical_compute_plugin_ijson_and_sha256(ack, MAX_LEDGER_JSON_BYTES)?;
    Ok((json, digest))
}

fn domain_json_and_digest<T: Serialize>(domain: &[u8], value: &T) -> Result<(String, String)> {
    let (json, _) = canonical_compute_plugin_ijson_and_sha256(value, MAX_LEDGER_JSON_BYTES)?;
    let mut digest = Sha256::new();
    digest.update(domain);
    digest.update([0]);
    digest.update(json.as_bytes());
    Ok((json, hex::encode(digest.finalize())))
}
