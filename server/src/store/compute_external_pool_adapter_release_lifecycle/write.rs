use anyhow::{bail, Result};
use chrono::{SecondsFormat, Utc};
use rusqlite::{params, Transaction, TransactionBehavior};

use crate::{
    compute_federation::external_pool_adapter_release_lifecycle::{
        validate_external_pool_adapter_release_admission_terminal_receipt,
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
        EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_ROUTE_EFFECT,
        EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_STATUS_REVOKED,
        EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_STATUS_STAGED,
        EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_STATUS_SUPERSEDED,
        EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_STATUS_WITHDRAWN,
    },
    store::{new_id, Store},
};

use super::{
    canonical::{canonical_terminal_json_and_digest, terminal_request_digest},
    read::{
        current_external_pool_adapter_release_admission_authority_on, expected_confirmation,
        historical_admission_on, lifecycle_on, terminal_by_admission_on,
        terminal_by_idempotency_on, validate_digest, validate_exact, validate_reason,
    },
    types::{
        CreateExternalPoolAdapterReleaseAdmissionTerminal,
        ExternalPoolAdapterReleaseAdmissionTerminalWriteReceipt, StoredTerminalReceipt,
    },
};

impl Store {
    pub(crate) fn create_external_pool_adapter_release_admission_terminal(
        &self,
        input: CreateExternalPoolAdapterReleaseAdmissionTerminal,
    ) -> Result<ExternalPoolAdapterReleaseAdmissionTerminalWriteReceipt> {
        validate_input(&input)?;
        let mut connection = self.conn()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;

        if let Some(stored) = terminal_by_idempotency_on(
            &transaction,
            &input.idempotency_scope,
            &input.idempotency_key,
        )? {
            ensure_replay(&stored, &input)?;
            ensure_terminal_currentness(&transaction, &stored.terminal_receipt)?;
            let receipt = stored.into_write_receipt(true);
            transaction.commit()?;
            return Ok(receipt);
        }

        let base = historical_admission_on(&transaction, &input.admission_id)?
            .ok_or_else(|| anyhow::anyhow!("external-pool Adapter release admission is absent"))?;
        if base.admission.admission_digest != input.expected_admission_digest {
            bail!("external-pool Adapter release admission digest is not exact");
        }
        if terminal_by_admission_on(&transaction, &input.admission_id)?.is_some() {
            bail!("external-pool Adapter release admission already has another terminal");
        }

        let successor = successor_binding_on(&transaction, &base, &input)?;
        let recorded_at = Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true);
        if successor
            .as_ref()
            .is_some_and(|value| value.applied_at.as_str() > recorded_at.as_str())
        {
            bail!("external-pool Adapter release successor was applied after terminal time");
        }
        let terminal = ComputeExternalPoolAdapterReleaseAdmissionTerminal {
            admission: ComputeExternalPoolAdapterReleaseAdmissionBinding {
                admission_id: base.admission.admission_id.clone(),
                admission_digest: base.admission.admission_digest.clone(),
                adapter_id: base.admission.adapter_id.clone(),
                release_version: base.admission.release_version.clone(),
            },
            prior_status: EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_STATUS_STAGED.to_string(),
            terminal_status: input.terminal_status.clone(),
            successor_admission: successor.map(|value| value.binding),
            actor_kind: EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_ACTOR_PLATFORM_ADMIN.to_string(),
            actor_id: input.actor_id.clone(),
            reason: input.reason.clone(),
            confirmation: input.confirmation.clone(),
            idempotency_scope: input.idempotency_scope.clone(),
            idempotency_key: input.idempotency_key.clone(),
            occurred_at: recorded_at.clone(),
            recorded_at,
            currentness_effect: EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_CURRENTNESS_EFFECT
                .to_string(),
            artifact_intake_effect: EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_ARTIFACT_INTAKE_EFFECT
                .to_string(),
            existing_artifact_source_effect:
                EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_EXISTING_ARTIFACT_SOURCE_EFFECT.to_string(),
            adapter_effect: EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_ADAPTER_EFFECT.to_string(),
            route_effect: EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_ROUTE_EFFECT.to_string(),
        };
        let request_digest = terminal_request_digest(&terminal)?;
        let mut terminal_receipt = ComputeExternalPoolAdapterReleaseAdmissionTerminalReceipt {
            schema: COMPUTE_EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_TERMINAL_RECEIPT_SCHEMA
                .to_string(),
            terminal_receipt_id: new_id(
                "compute_external_pool_adapter_release_admission_terminal_receipt",
            ),
            terminal_receipt_digest: String::new(),
            request_digest,
            canonicalization:
                COMPUTE_EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_TERMINAL_CANONICALIZATION
                    .to_string(),
            digest_algorithm:
                COMPUTE_EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_TERMINAL_DIGEST_ALGORITHM
                    .to_string(),
            terminal,
        };
        let (_, digest) = canonical_terminal_json_and_digest(&terminal_receipt)?;
        terminal_receipt.terminal_receipt_digest = digest;
        let (terminal_json, digest) = canonical_terminal_json_and_digest(&terminal_receipt)?;
        if digest != terminal_receipt.terminal_receipt_digest {
            bail!("external-pool Adapter release terminal digest changed before persistence");
        }
        validate_external_pool_adapter_release_admission_terminal_receipt(&terminal_receipt)?;

        insert_terminal(&transaction, &terminal_receipt, &terminal_json)?;
        let stored = terminal_by_admission_on(&transaction, &input.admission_id)?
            .ok_or_else(|| anyhow::anyhow!("Adapter release terminal is absent after insert"))?;
        if stored.terminal_receipt != terminal_receipt
            || stored.terminal_receipt_json != terminal_json
        {
            bail!("Adapter release terminal changed during exact readback");
        }
        ensure_terminal_currentness(&transaction, &terminal_receipt)?;
        let receipt = stored.into_write_receipt(false);
        transaction.commit()?;
        Ok(receipt)
    }
}

struct AuditedSuccessor {
    binding: ComputeExternalPoolAdapterReleaseSuccessorAdmissionBinding,
    applied_at: String,
}

fn successor_binding_on(
    transaction: &Transaction<'_>,
    base: &super::types::AuditedAdmission,
    input: &CreateExternalPoolAdapterReleaseAdmissionTerminal,
) -> Result<Option<AuditedSuccessor>> {
    if input.terminal_status != EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_STATUS_SUPERSEDED {
        return Ok(None);
    }
    let successor_id = input.successor_admission_id.as_deref().ok_or_else(|| {
        anyhow::anyhow!("external-pool Adapter release supersession lacks successor ID")
    })?;
    let successor_digest = input
        .expected_successor_admission_digest
        .as_deref()
        .ok_or_else(|| {
            anyhow::anyhow!("external-pool Adapter release supersession lacks successor digest")
        })?;
    let authority = current_external_pool_adapter_release_admission_authority_on(
        transaction,
        successor_id,
        successor_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("external-pool Adapter release successor is absent"))?;
    if authority.adapter_id() != base.admission.adapter_id
        || authority.admission_id() == base.admission.admission_id
        || authority.release_version() == base.admission.release_version
        || authority.applied_at() < base.applied_at.as_str()
    {
        bail!("external-pool Adapter release successor lineage is not exact");
    }
    Ok(Some(AuditedSuccessor {
        binding: ComputeExternalPoolAdapterReleaseSuccessorAdmissionBinding {
            admission_id: authority.admission_id().to_string(),
            admission_digest: authority.admission_digest().to_string(),
            release_version: authority.release_version().to_string(),
        },
        applied_at: authority.applied_at().to_string(),
    }))
}

fn insert_terminal(
    transaction: &Transaction<'_>,
    receipt: &ComputeExternalPoolAdapterReleaseAdmissionTerminalReceipt,
    receipt_json: &str,
) -> Result<()> {
    let terminal = &receipt.terminal;
    let successor = terminal.successor_admission.as_ref();
    transaction.execute(
        "INSERT INTO compute_external_pool_adapter_release_admission_terminal_receipts (
            terminal_receipt_id, terminal_receipt_schema, terminal_receipt_digest,
            terminal_receipt_json, canonicalization, digest_algorithm, request_digest,
            admission_id, admission_digest, adapter_id, release_version, prior_status,
            terminal_status, successor_admission_id, successor_admission_digest,
            successor_release_version, actor_kind, actor_id, reason, confirmation,
            idempotency_scope, idempotency_key, occurred_at, recorded_at,
            currentness_effect, artifact_intake_effect, existing_artifact_source_effect,
            adapter_effect, route_effect
         ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10,
            ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20,
            ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29
         )",
        params![
            receipt.terminal_receipt_id,
            receipt.schema,
            receipt.terminal_receipt_digest,
            receipt_json,
            receipt.canonicalization,
            receipt.digest_algorithm,
            receipt.request_digest,
            terminal.admission.admission_id,
            terminal.admission.admission_digest,
            terminal.admission.adapter_id,
            terminal.admission.release_version,
            terminal.prior_status,
            terminal.terminal_status,
            successor.map(|value| value.admission_id.as_str()),
            successor.map(|value| value.admission_digest.as_str()),
            successor.map(|value| value.release_version.as_str()),
            terminal.actor_kind,
            terminal.actor_id,
            terminal.reason,
            terminal.confirmation,
            terminal.idempotency_scope,
            terminal.idempotency_key,
            terminal.occurred_at,
            terminal.recorded_at,
            terminal.currentness_effect,
            terminal.artifact_intake_effect,
            terminal.existing_artifact_source_effect,
            terminal.adapter_effect,
            terminal.route_effect,
        ],
    )?;
    Ok(())
}

fn ensure_terminal_currentness(
    transaction: &Transaction<'_>,
    receipt: &ComputeExternalPoolAdapterReleaseAdmissionTerminalReceipt,
) -> Result<()> {
    let currentness = lifecycle_on(
        transaction,
        &receipt.terminal.admission.admission_id,
        &receipt.terminal.admission.admission_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("terminal admission disappeared from current view"))?
    .1;
    if currentness.current_status != receipt.terminal.terminal_status
        || currentness.terminal_receipt_id.as_deref() != Some(receipt.terminal_receipt_id.as_str())
        || currentness.terminal_receipt_digest.as_deref()
            != Some(receipt.terminal_receipt_digest.as_str())
    {
        bail!("Adapter release terminal currentness readback is not exact");
    }
    Ok(())
}

fn ensure_replay(
    stored: &StoredTerminalReceipt,
    input: &CreateExternalPoolAdapterReleaseAdmissionTerminal,
) -> Result<()> {
    let terminal = &stored.terminal_receipt.terminal;
    let successor = terminal.successor_admission.as_ref();
    if terminal.admission.admission_id != input.admission_id
        || terminal.admission.admission_digest != input.expected_admission_digest
        || terminal.terminal_status != input.terminal_status
        || successor.map(|value| value.admission_id.as_str())
            != input.successor_admission_id.as_deref()
        || successor.map(|value| value.admission_digest.as_str())
            != input.expected_successor_admission_digest.as_deref()
        || terminal.actor_id != input.actor_id
        || terminal.reason != input.reason
        || terminal.confirmation != input.confirmation
        || terminal.idempotency_scope != input.idempotency_scope
        || terminal.idempotency_key != input.idempotency_key
    {
        bail!("external-pool Adapter release terminal replay conflicts with immutable history");
    }
    Ok(())
}

fn validate_input(input: &CreateExternalPoolAdapterReleaseAdmissionTerminal) -> Result<()> {
    validate_exact(&input.admission_id, "terminal admission ID", 160)?;
    validate_digest(
        &input.expected_admission_digest,
        "terminal admission digest",
    )?;
    validate_exact(&input.actor_id, "terminal actor ID", 160)?;
    validate_exact(&input.idempotency_scope, "terminal idempotency scope", 200)?;
    validate_exact(&input.idempotency_key, "terminal idempotency key", 160)?;
    validate_reason(&input.reason)?;
    if input.confirmation != expected_confirmation(&input.terminal_status)? {
        bail!("external-pool Adapter release terminal confirmation is not exact");
    }
    match input.terminal_status.as_str() {
        EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_STATUS_WITHDRAWN
        | EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_STATUS_REVOKED => {
            if input.successor_admission_id.is_some()
                || input.expected_successor_admission_digest.is_some()
            {
                bail!("withdrawn or revoked Adapter release cannot carry a successor");
            }
        }
        EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_STATUS_SUPERSEDED => {
            let successor_id = input.successor_admission_id.as_deref().ok_or_else(|| {
                anyhow::anyhow!("Adapter release supersession requires successor ID")
            })?;
            let successor_digest = input
                .expected_successor_admission_digest
                .as_deref()
                .ok_or_else(|| {
                    anyhow::anyhow!("Adapter release supersession requires successor digest")
                })?;
            validate_exact(successor_id, "successor admission ID", 160)?;
            validate_digest(successor_digest, "successor admission digest")?;
            if successor_id == input.admission_id {
                bail!("Adapter release admission cannot supersede itself");
            }
        }
        _ => bail!("external-pool Adapter release terminal status is unsupported"),
    }
    Ok(())
}
