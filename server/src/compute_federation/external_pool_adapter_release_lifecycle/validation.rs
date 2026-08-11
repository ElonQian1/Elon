use anyhow::{bail, Result};
use chrono::{DateTime, SecondsFormat};

use super::{
    canonical::{
        canonical_external_pool_adapter_release_admission_terminal_json_and_digest,
        canonical_external_pool_adapter_release_admission_terminal_request_digest,
    },
    types::{
        ComputeExternalPoolAdapterReleaseAdmissionBinding,
        ComputeExternalPoolAdapterReleaseAdmissionTerminal,
        ComputeExternalPoolAdapterReleaseAdmissionTerminalReceipt,
        ComputeExternalPoolAdapterReleaseSuccessorAdmissionBinding,
        COMPUTE_EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_TERMINAL_CANONICALIZATION,
        COMPUTE_EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_TERMINAL_DIGEST_ALGORITHM,
        COMPUTE_EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_TERMINAL_RECEIPT_SCHEMA,
        EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_ACTOR_PLATFORM_ADMIN,
        EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_ADAPTER_EFFECT,
        EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_ARTIFACT_INTAKE_EFFECT,
        EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_CURRENTNESS_EFFECT,
        EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_EXISTING_ARTIFACT_SOURCE_EFFECT,
        EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_REVOCATION_CONFIRMATION,
        EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_ROUTE_EFFECT,
        EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_STATUS_REVOKED,
        EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_STATUS_STAGED,
        EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_STATUS_SUPERSEDED,
        EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_STATUS_WITHDRAWN,
        EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_SUPERSESSION_CONFIRMATION,
        EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_WITHDRAWAL_CONFIRMATION,
    },
};

pub(crate) fn validate_external_pool_adapter_release_admission_terminal_receipt(
    receipt: &ComputeExternalPoolAdapterReleaseAdmissionTerminalReceipt,
) -> Result<()> {
    if receipt.schema != COMPUTE_EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_TERMINAL_RECEIPT_SCHEMA
        || receipt.canonicalization
            != COMPUTE_EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_TERMINAL_CANONICALIZATION
        || receipt.digest_algorithm
            != COMPUTE_EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_TERMINAL_DIGEST_ALGORITHM
    {
        bail!("external-pool Adapter admission terminal metadata is not supported");
    }
    validate_identifier(
        &receipt.terminal_receipt_id,
        "admission terminal receipt ID",
        160,
    )?;
    validate_digest(
        &receipt.terminal_receipt_digest,
        "admission terminal receipt digest",
    )?;
    validate_digest(&receipt.request_digest, "admission terminal request digest")?;
    validate_terminal(&receipt.terminal)?;

    let request_digest = canonical_external_pool_adapter_release_admission_terminal_request_digest(
        &receipt.terminal,
    )?;
    if request_digest != receipt.request_digest {
        bail!("external-pool Adapter admission terminal request digest is not canonical");
    }
    let (_, receipt_digest) =
        canonical_external_pool_adapter_release_admission_terminal_json_and_digest(receipt)?;
    if receipt_digest != receipt.terminal_receipt_digest {
        bail!("external-pool Adapter admission terminal receipt digest is not canonical");
    }
    Ok(())
}

fn validate_terminal(terminal: &ComputeExternalPoolAdapterReleaseAdmissionTerminal) -> Result<()> {
    validate_admission(&terminal.admission)?;
    if terminal.prior_status != EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_STATUS_STAGED
        || terminal.actor_kind != EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_ACTOR_PLATFORM_ADMIN
        || terminal.currentness_effect != EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_CURRENTNESS_EFFECT
        || terminal.artifact_intake_effect
            != EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_ARTIFACT_INTAKE_EFFECT
        || terminal.existing_artifact_source_effect
            != EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_EXISTING_ARTIFACT_SOURCE_EFFECT
        || terminal.adapter_effect != EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_ADAPTER_EFFECT
        || terminal.route_effect != EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_ROUTE_EFFECT
    {
        bail!("external-pool Adapter admission terminal effects are not exact");
    }
    validate_identifier(&terminal.actor_id, "admission terminal actor ID", 160)?;
    validate_identifier(
        &terminal.idempotency_scope,
        "admission terminal idempotency scope",
        200,
    )?;
    validate_identifier(
        &terminal.idempotency_key,
        "admission terminal idempotency key",
        160,
    )?;
    validate_text(&terminal.reason, "admission terminal reason", 8, 2_000)?;

    match terminal.terminal_status.as_str() {
        EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_STATUS_WITHDRAWN => {
            require_without_successor(
                terminal,
                EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_WITHDRAWAL_CONFIRMATION,
            )?;
        }
        EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_STATUS_REVOKED => {
            require_without_successor(
                terminal,
                EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_REVOCATION_CONFIRMATION,
            )?;
        }
        EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_STATUS_SUPERSEDED => {
            if terminal.confirmation
                != EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_SUPERSESSION_CONFIRMATION
            {
                bail!("external-pool Adapter admission supersession confirmation is not exact");
            }
            let successor = terminal.successor_admission.as_ref().ok_or_else(|| {
                anyhow::anyhow!("external-pool Adapter admission supersession lacks a successor")
            })?;
            validate_successor(successor)?;
            if successor.admission_id == terminal.admission.admission_id
                || successor.release_version == terminal.admission.release_version
            {
                bail!("external-pool Adapter admission cannot supersede itself");
            }
        }
        _ => bail!("external-pool Adapter admission terminal status is not supported"),
    }

    let occurred_at = parse_timestamp(&terminal.occurred_at, "admission terminal occurred_at")?;
    let recorded_at = parse_timestamp(&terminal.recorded_at, "admission terminal recorded_at")?;
    if terminal.occurred_at != terminal.recorded_at || occurred_at != recorded_at {
        bail!("external-pool Adapter admission terminal timestamps must be identical");
    }
    Ok(())
}

fn require_without_successor(
    terminal: &ComputeExternalPoolAdapterReleaseAdmissionTerminal,
    confirmation: &str,
) -> Result<()> {
    if terminal.successor_admission.is_some() || terminal.confirmation != confirmation {
        bail!("external-pool Adapter admission terminal successor or confirmation is invalid");
    }
    Ok(())
}

fn validate_admission(binding: &ComputeExternalPoolAdapterReleaseAdmissionBinding) -> Result<()> {
    validate_identifier(&binding.admission_id, "admission ID", 160)?;
    validate_digest(&binding.admission_digest, "admission digest")?;
    validate_identifier(&binding.adapter_id, "Adapter ID", 160)?;
    validate_identifier(&binding.release_version, "release version", 80)
}

fn validate_successor(
    binding: &ComputeExternalPoolAdapterReleaseSuccessorAdmissionBinding,
) -> Result<()> {
    validate_identifier(&binding.admission_id, "successor admission ID", 160)?;
    validate_digest(&binding.admission_digest, "successor admission digest")?;
    validate_identifier(&binding.release_version, "successor release version", 80)
}

fn validate_identifier(value: &str, label: &str, limit: usize) -> Result<()> {
    validate_text(value, label, 1, limit)
}

fn validate_text(value: &str, label: &str, minimum: usize, maximum: usize) -> Result<()> {
    let length = value.chars().count();
    if value.trim() != value
        || !(minimum..=maximum).contains(&length)
        || value.chars().any(char::is_control)
    {
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
