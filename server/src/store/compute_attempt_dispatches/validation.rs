use anyhow::{bail, Result};
use chrono::{DateTime, SecondsFormat};

use crate::{
    compute_federation::attempt_gateway::{
        canonical_adapter_ack_json_and_digest, canonical_adapter_binding_json_and_digest,
        canonical_dispatch_application_json_and_digest, canonical_dispatch_command_json_and_digest,
        ComputeAttemptAdapterAckEnvelope, ComputeAttemptDispatchApplicationEnvelope,
        ValidatedComputeAttemptStartDispatch, VerifiedComputeAttemptAdapterAck,
        COMPUTE_ATTEMPT_ADAPTER_ACK_ACCEPTED, COMPUTE_ATTEMPT_ADAPTER_ACK_REJECTED,
        COMPUTE_ATTEMPT_ADAPTER_ACK_SCHEMA, COMPUTE_ATTEMPT_ADAPTER_BINDING_SCHEMA,
        COMPUTE_ATTEMPT_DISPATCH_APPLICATION_ACTION_V185_ACTIVATE,
        COMPUTE_ATTEMPT_DISPATCH_APPLICATION_SCHEMA, COMPUTE_ATTEMPT_DISPATCH_COMMAND_SCHEMA,
        COMPUTE_ATTEMPT_DISPATCH_COMMAND_TYPE_START, COMPUTE_ATTEMPT_ROUTE_PROVIDER_ENDPOINT,
        COMPUTE_ATTEMPT_ROUTE_SERVER_ADAPTER,
    },
    compute_federation::provider::PROVIDER_KIND_EXTERNAL_POOL,
    store::ComputeAttemptActivationReceipt,
};

const MAX_SAFE_INTEGER: i64 = 9_007_199_254_740_991;
const MIN_ACTIVATION_MARGIN_SECONDS: i64 = 60;

pub(super) struct PreparedStartDispatch {
    pub command_json: String,
    pub command_digest: String,
    pub adapter_json: String,
    pub adapter_digest: String,
}

pub(super) struct PreparedVerifiedAck {
    pub ack_json: String,
    pub ack_digest: String,
    pub adapter_digest: String,
}

pub(in crate::store) struct PreparedApplication {
    pub(in crate::store) application_id: String,
    pub(in crate::store) application_json: String,
    pub(in crate::store) application_digest: String,
    envelope: ComputeAttemptDispatchApplicationEnvelope,
}

impl PreparedApplication {
    pub(in crate::store) fn envelope(&self) -> &ComputeAttemptDispatchApplicationEnvelope {
        &self.envelope
    }
}

pub(super) fn prepare_start_dispatch(
    plan: &ValidatedComputeAttemptStartDispatch,
) -> Result<PreparedStartDispatch> {
    let command = plan.command();
    let start = &command.command;
    let adapter = plan.adapter();
    let activation = plan.activation();
    validate_identifier(&command.command_id, "Attempt dispatch command ID", 160)?;
    validate_identifier(&start.identity.job_id, "Attempt Job ID", 160)?;
    validate_identifier(
        &start.identity.reservation_id,
        "Attempt Reservation ID",
        160,
    )?;
    validate_identifier(&start.identity.attempt_lease_id, "Attempt Lease ID", 160)?;
    validate_identifier(&start.provider.provider_id, "Provider ID", 160)?;
    validate_identifier(&start.offer.offer_id, "Offer ID", 160)?;
    validate_identifier(&start.capacity_claim.claim_id, "Capacity Claim ID", 160)?;
    validate_identifier(&start.executor_id, "Attempt executor ID", 160)?;
    validate_optional_identifier(start.identity.shard_id.as_deref(), "Attempt shard ID", 160)?;
    validate_identifier(&start.execution_plan.plan_id, "execution plan ID", 160)?;
    validate_identifier(
        &start.execution_plan.plan_schema,
        "execution plan schema",
        160,
    )?;
    validate_digest(&start.execution_plan.plan_digest, "execution plan digest")?;
    validate_identifier(
        activation.lease_credential_ref(),
        "Lease credential ref",
        512,
    )?;
    validate_identifier(
        activation.lease_credential_hint(),
        "Lease credential hint",
        160,
    )?;
    validate_identifier(
        activation.idempotency_key(),
        "activation idempotency key",
        160,
    )?;
    validate_identifier(activation.activated_by_user_id(), "activation actor", 160)?;
    if command.schema != COMPUTE_ATTEMPT_DISPATCH_COMMAND_SCHEMA
        || start.command_type != COMPUTE_ATTEMPT_DISPATCH_COMMAND_TYPE_START
        || start.identity.attempt_no != 1
        || start.identity.fencing_generation != 1
        || start.identity.job_id != start.job.job_id
        || start.identity.reservation_id != start.reservation.reservation_id
        || start.provider.provider_id != adapter.provider_id
        || start.provider.provider_id.is_empty()
        || start.offer.provider_id != start.provider.provider_id
        || !(1..=MAX_SAFE_INTEGER).contains(&start.provider.policy_revision)
        || !(1..=MAX_SAFE_INTEGER).contains(&start.job.job_revision)
        || !(1..=MAX_SAFE_INTEGER).contains(&start.reservation.reservation_revision)
        || !(1..=MAX_SAFE_INTEGER).contains(&start.capacity_claim.claim_revision)
        || !(1..=MAX_SAFE_INTEGER).contains(&start.offer.offer_version)
    {
        bail!("Attempt Start dispatch identity or version binding is invalid");
    }
    for (label, digest) in [
        ("command digest", command.command_digest.as_str()),
        ("Provider digest", start.provider.provider_digest.as_str()),
        ("Offer digest", start.offer.offer_digest.as_str()),
        ("Job digest", start.job.job_digest.as_str()),
        (
            "Reservation digest",
            start.reservation.reservation_digest.as_str(),
        ),
        (
            "Capacity Claim digest",
            start.capacity_claim.claim_digest.as_str(),
        ),
    ] {
        validate_digest(digest, label)?;
    }
    let issued_at = parse_timestamp(&command.issued_at, "command issued_at")?;
    let not_after = parse_timestamp(&command.not_after, "command not_after")?;
    let lease_expires_at = parse_timestamp(&start.lease_expires_at, "lease expires_at")?;
    let hard_deadline_at = parse_timestamp(&start.hard_deadline_at, "hard deadline")?;
    if issued_at >= not_after
        || issued_at >= lease_expires_at
        || not_after > lease_expires_at
        || lease_expires_at >= hard_deadline_at
        || (lease_expires_at - not_after).num_seconds() < MIN_ACTIVATION_MARGIN_SECONDS
    {
        bail!("Attempt Start dispatch time window is invalid");
    }
    validate_adapter_binding(adapter)?;
    let (command_json, command_digest) = canonical_dispatch_command_json_and_digest(command)?;
    if command_digest != command.command_digest {
        bail!("Attempt Start command digest does not match its canonical payload");
    }
    let (adapter_json, adapter_digest) = canonical_adapter_binding_json_and_digest(adapter)?;
    Ok(PreparedStartDispatch {
        command_json,
        command_digest,
        adapter_json,
        adapter_digest,
    })
}

pub(super) fn prepare_verified_ack(
    verified: &VerifiedComputeAttemptAdapterAck,
) -> Result<PreparedVerifiedAck> {
    let adapter = verified.adapter();
    let ack = verified.ack();
    validate_adapter_binding(adapter)?;
    validate_identifier(&ack.ack_id, "Adapter ACK ID", 160)?;
    validate_identifier(&ack.adapter_ack_id, "remote Adapter ACK ID", 160)?;
    validate_identifier(&ack.command_id, "Attempt command ID", 160)?;
    validate_digest(&ack.command_digest, "Attempt command digest")?;
    validate_digest(&ack.adapter_binding_digest, "Adapter binding digest")?;
    validate_digest(&ack.ack_digest, "Adapter ACK digest")?;
    if ack.schema != COMPUTE_ATTEMPT_ADAPTER_ACK_SCHEMA {
        bail!("Adapter ACK schema is not supported");
    }
    match ack.outcome.as_str() {
        COMPUTE_ATTEMPT_ADAPTER_ACK_ACCEPTED => {
            validate_identifier(
                ack.remote_execution_ref.as_deref().unwrap_or_default(),
                "remote execution ref",
                512,
            )?;
            if ack.reason_code.is_some() {
                bail!("Accepted Adapter ACK cannot carry a rejection reason");
            }
        }
        COMPUTE_ATTEMPT_ADAPTER_ACK_REJECTED => {
            validate_identifier(
                ack.reason_code.as_deref().unwrap_or_default(),
                "Adapter rejection reason",
                160,
            )?;
            if ack.remote_execution_ref.is_some() {
                bail!("Rejected Adapter ACK cannot carry a remote execution ref");
            }
        }
        _ => bail!("Adapter ACK outcome is not supported"),
    }
    if parse_timestamp(&ack.observed_at, "ACK observed_at")?
        > parse_timestamp(&ack.received_at, "ACK received_at")?
    {
        bail!("Adapter ACK cannot be received before it was observed");
    }
    let (_, adapter_digest) = canonical_adapter_binding_json_and_digest(adapter)?;
    if adapter_digest != ack.adapter_binding_digest {
        bail!("Adapter ACK is not bound to the verified Adapter configuration");
    }
    let (ack_json, ack_digest) = canonical_adapter_ack_json_and_digest(ack)?;
    if ack_digest != ack.ack_digest {
        bail!("Adapter ACK digest does not match its canonical payload");
    }
    Ok(PreparedVerifiedAck {
        ack_json,
        ack_digest,
        adapter_digest,
    })
}

pub(super) fn prepare_application(
    ack: &ComputeAttemptAdapterAckEnvelope,
    activation: &ComputeAttemptActivationReceipt,
) -> Result<PreparedApplication> {
    let application_id = application_id_for_ack(ack);
    let mut envelope = ComputeAttemptDispatchApplicationEnvelope {
        schema: COMPUTE_ATTEMPT_DISPATCH_APPLICATION_SCHEMA.to_string(),
        application_id,
        application_digest: String::new(),
        command_id: ack.command_id.clone(),
        ack_id: ack.ack_id.clone(),
        action: COMPUTE_ATTEMPT_DISPATCH_APPLICATION_ACTION_V185_ACTIVATE.to_string(),
        lease_id: activation.lease.lease_id.clone(),
        activation_request_digest: activation.request_digest.clone(),
        lease_digest: activation.lease_digest.clone(),
        applied_at: activation.activated_at.clone(),
    };
    let (_, application_digest) = canonical_dispatch_application_json_and_digest(&envelope)?;
    envelope.application_digest = application_digest.clone();
    let (application_json, recomputed_digest) =
        canonical_dispatch_application_json_and_digest(&envelope)?;
    if recomputed_digest != application_digest {
        bail!("Attempt dispatch application digest is not stable");
    }
    Ok(PreparedApplication {
        application_id: envelope.application_id.clone(),
        application_json,
        application_digest,
        envelope,
    })
}

pub(super) fn application_id_for_ack(ack: &ComputeAttemptAdapterAckEnvelope) -> String {
    format!("attempt_dispatch_application_{}", ack.ack_digest)
}

fn validate_adapter_binding(
    adapter: &crate::compute_federation::attempt_gateway::ComputeAttemptAdapterBinding,
) -> Result<()> {
    if adapter.schema != COMPUTE_ATTEMPT_ADAPTER_BINDING_SCHEMA
        || !(1..=MAX_SAFE_INTEGER).contains(&adapter.config_revision)
    {
        bail!("Attempt Adapter binding schema or revision is invalid");
    }
    for (label, value, limit) in [
        ("Provider ID", adapter.provider_id.as_str(), 160),
        ("Provider kind", adapter.provider_kind.as_str(), 80),
        ("Adapter ID", adapter.adapter_id.as_str(), 160),
        ("Adapter version", adapter.adapter_version.as_str(), 80),
    ] {
        validate_identifier(value, label, limit)?;
    }
    match adapter.route_kind.as_str() {
        COMPUTE_ATTEMPT_ROUTE_PROVIDER_ENDPOINT => {
            validate_identifier(
                adapter.endpoint_id.as_deref().unwrap_or_default(),
                "endpoint ID",
                160,
            )?;
            validate_identifier(
                adapter.endpoint_transport.as_deref().unwrap_or_default(),
                "endpoint transport",
                80,
            )?;
        }
        COMPUTE_ATTEMPT_ROUTE_SERVER_ADAPTER => {
            if adapter.endpoint_id.is_some() || adapter.endpoint_transport.is_some() {
                bail!("Server Adapter route cannot carry a Provider endpoint");
            }
        }
        _ => bail!("Attempt Adapter route kind is not supported"),
    }
    if adapter.provider_kind == PROVIDER_KIND_EXTERNAL_POOL
        && adapter.route_kind != COMPUTE_ATTEMPT_ROUTE_SERVER_ADAPTER
    {
        bail!("External-pool execution must use a server Adapter route");
    }
    validate_exact_value(&adapter.config_digest, "Adapter config digest")
}

fn validate_optional_identifier(value: Option<&str>, label: &str, limit: usize) -> Result<()> {
    if let Some(value) = value {
        validate_identifier(value, label, limit)?;
    }
    Ok(())
}

fn validate_identifier(value: &str, label: &str, limit: usize) -> Result<()> {
    if value.is_empty()
        || value.trim() != value
        || value.len() > limit
        || value.chars().any(char::is_control)
    {
        bail!("{label} is invalid");
    }
    Ok(())
}

fn validate_exact_value(value: &str, label: &str) -> Result<()> {
    if value.trim().is_empty() || value.trim() != value {
        bail!("{label} is invalid");
    }
    Ok(())
}

fn validate_digest(value: &str, label: &str) -> Result<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        bail!("{label} must be a lowercase SHA-256 digest");
    }
    Ok(())
}

fn parse_timestamp(value: &str, label: &str) -> Result<DateTime<chrono::FixedOffset>> {
    let parsed = DateTime::parse_from_rfc3339(value)
        .map_err(|_| anyhow::anyhow!("{label} is not RFC3339"))?;
    if parsed.offset().local_minus_utc() != 0
        || parsed.to_rfc3339_opts(SecondsFormat::Nanos, true) != value
    {
        bail!("{label} must use canonical UTC nanoseconds");
    }
    Ok(parsed)
}
