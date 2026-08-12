use anyhow::{bail, Result};
use chrono::{SecondsFormat, Utc};
use rusqlite::{params, Transaction, TransactionBehavior};

use crate::{
    compute_federation::external_pool_adapter_adoption::*,
    store::{
        compute_external_pool_adapter_artifact_sandbox_conformance::current_external_pool_adapter_sandbox_conformance_authority_on,
        compute_external_pool_adapter_credential_verification::current_external_pool_adapter_credential_verification_authority_on,
        new_id, Store,
    },
};

use super::{read::*, types::*};

impl Store {
    pub(crate) fn adopt_external_pool_adapter(
        &self,
        input: AdoptExternalPoolAdapter,
    ) -> Result<ExternalPoolAdapterAdoptionWriteReceipt> {
        validate_adopt_input(&input)?;
        let mut connection = self.conn()?;
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(stored) =
            adoption_by_idempotency_on(&tx, &input.idempotency_scope, &input.idempotency_key)?
        {
            ensure_adoption_replay(&stored.receipt, &input)?;
            let output = adoption_write_receipt(&stored, None, true);
            tx.commit()?;
            return Ok(output);
        }
        if adoption_by_lineage_on(&tx, &input.application_id, &input.admission_id)?.is_some() {
            bail!("exact onboarding and admission lineage already has an adoption receipt");
        }
        let binding = exact_binding(&tx, &input)?;
        let timestamp = Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true);
        let material = ExternalPoolAdapterAdoptionMaterial {
            binding,
            adopted_by_admin_user_id: input.adopted_by_admin_user_id,
            confirmation: input.confirmation,
            idempotency_scope: input.idempotency_scope,
            idempotency_key: input.idempotency_key,
            adopted_at: timestamp.clone(),
            recorded_at: timestamp,
            adoption_effect: ADOPTION_AUTHORITY_EFFECT.to_string(),
            install_effect: ADOPTION_INSTALL_EFFECT.to_string(),
            provider_effect: ADOPTION_NO_EFFECT.to_string(),
            route_effect: ADOPTION_NO_EFFECT.to_string(),
            execution_effect: ADOPTION_NO_EFFECT.to_string(),
            settlement_effect: ADOPTION_NO_EFFECT.to_string(),
        };
        let mut receipt = ExternalPoolAdapterAdoptionReceipt {
            schema: ADOPTION_RECEIPT_SCHEMA.to_string(),
            adoption_receipt_id: new_id("external_pool_adapter_adoption"),
            adoption_receipt_digest: String::new(),
            adoption_material_digest: adoption_material_digest(&material)?,
            canonicalization: ADOPTION_CANONICALIZATION.to_string(),
            digest_algorithm: ADOPTION_DIGEST_ALGORITHM.to_string(),
            adoption: material,
        };
        receipt.adoption_receipt_digest = canonical_adoption_receipt_json_and_digest(&receipt)?.1;
        validate_adoption_receipt(&receipt)?;
        let (json, digest) = canonical_adoption_receipt_json_and_digest(&receipt)?;
        if digest != receipt.adoption_receipt_digest {
            bail!("Adapter adoption digest changed before persistence");
        }
        insert_adoption(&tx, &receipt, &json)?;
        let stored = adoption_by_id_on(&tx, &receipt.adoption_receipt_id)?
            .ok_or_else(|| anyhow::anyhow!("Adapter adoption disappeared after insert"))?;
        if stored.receipt != receipt || stored.receipt_json != json {
            bail!("Adapter adoption changed during exact readback");
        }
        let output = adoption_write_receipt(&stored, None, false);
        tx.commit()?;
        Ok(output)
    }

    pub(crate) fn revoke_external_pool_adapter_adoption(
        &self,
        input: RevokeExternalPoolAdapterAdoption,
    ) -> Result<ExternalPoolAdapterAdoptionWriteReceipt> {
        validate_revoke_input(&input)?;
        let mut connection = self.conn()?;
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(terminal) =
            terminal_by_idempotency_on(&tx, &input.idempotency_scope, &input.idempotency_key)?
        {
            ensure_terminal_replay(&terminal.receipt, &input)?;
            let adoption = adoption_by_id_on(&tx, &input.adoption_receipt_id)?
                .ok_or_else(|| anyhow::anyhow!("Adapter adoption was not found"))?;
            let output = adoption_write_receipt(&adoption, Some(&terminal), true);
            tx.commit()?;
            return Ok(output);
        }
        let adoption = adoption_by_id_on(&tx, &input.adoption_receipt_id)?
            .ok_or_else(|| anyhow::anyhow!("Adapter adoption was not found"))?;
        if adoption.receipt.adoption_receipt_digest != input.expected_adoption_receipt_digest {
            bail!("Adapter adoption revocation digest is stale");
        }
        if terminal_by_adoption_on(&tx, &input.adoption_receipt_id)?.is_some() {
            bail!("Adapter adoption already has an immutable terminal receipt");
        }
        let timestamp = Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true);
        let material = ExternalPoolAdapterAdoptionTerminalMaterial {
            adoption_receipt_id: input.adoption_receipt_id,
            adoption_receipt_digest: input.expected_adoption_receipt_digest,
            revoked_by_admin_user_id: input.revoked_by_admin_user_id,
            reason: input.reason,
            confirmation: input.confirmation,
            idempotency_scope: input.idempotency_scope,
            idempotency_key: input.idempotency_key,
            revoked_at: timestamp.clone(),
            recorded_at: timestamp,
            adoption_effect: ADOPTION_REVOKED_EFFECT.to_string(),
            provider_effect: ADOPTION_NO_EFFECT.to_string(),
            route_effect: ADOPTION_NO_EFFECT.to_string(),
            execution_effect: ADOPTION_NO_EFFECT.to_string(),
            settlement_effect: ADOPTION_NO_EFFECT.to_string(),
        };
        let mut receipt = ExternalPoolAdapterAdoptionTerminalReceipt {
            schema: ADOPTION_TERMINAL_RECEIPT_SCHEMA.to_string(),
            terminal_receipt_id: new_id("external_pool_adapter_adoption_terminal"),
            terminal_receipt_digest: String::new(),
            terminal_material_digest: adoption_terminal_material_digest(&material)?,
            canonicalization: ADOPTION_CANONICALIZATION.to_string(),
            digest_algorithm: ADOPTION_DIGEST_ALGORITHM.to_string(),
            terminal: material,
        };
        receipt.terminal_receipt_digest =
            canonical_adoption_terminal_receipt_json_and_digest(&receipt)?.1;
        validate_adoption_terminal_receipt(&receipt)?;
        let (json, digest) = canonical_adoption_terminal_receipt_json_and_digest(&receipt)?;
        if digest != receipt.terminal_receipt_digest {
            bail!("Adapter adoption terminal digest changed before persistence");
        }
        insert_terminal(&tx, &receipt, &json)?;
        let terminal = terminal_by_adoption_on(&tx, &receipt.terminal.adoption_receipt_id)?
            .ok_or_else(|| anyhow::anyhow!("Adapter adoption terminal disappeared after insert"))?;
        if terminal.receipt != receipt || terminal.receipt_json != json {
            bail!("Adapter adoption terminal changed during exact readback");
        }
        let output = adoption_write_receipt(&adoption, Some(&terminal), false);
        tx.commit()?;
        Ok(output)
    }
}

fn exact_binding(
    tx: &Transaction<'_>,
    input: &AdoptExternalPoolAdapter,
) -> Result<ExternalPoolAdapterAdoptionBinding> {
    let sandbox = current_external_pool_adapter_sandbox_conformance_authority_on(
        tx,
        &input.admission_id,
        &input.expected_sandbox_conformance_receipt_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("current sandbox conformance authority was not found"))?;
    let credential = current_external_pool_adapter_credential_verification_authority_on(
        tx,
        &input.credential_verification_receipt_id,
        &input.expected_credential_verification_receipt_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("current credential verification authority was not found"))?;
    let sandbox_receipt = sandbox.receipt();
    let sandbox_binding = &sandbox_receipt.conformance.binding;
    let credential_receipt = credential.receipt();
    let credential_binding = &credential_receipt.verification.binding;
    if credential_binding.application_id != input.application_id
        || credential_binding.application_digest != input.expected_application_digest
        || credential_binding.admission_id != input.admission_id
        || credential_binding.admission_digest != input.expected_admission_digest
        || sandbox_binding.admission_id != credential_binding.admission_id
        || sandbox_binding.admission_digest != credential_binding.admission_digest
        || sandbox_binding.adapter_id != credential_binding.adapter_id
        || sandbox_binding.release_version != credential_binding.adapter_release_version
        || sandbox_binding.declared_implementation_sha256
            != credential_binding.declared_implementation_sha256
        || sandbox_binding.capability_set_digest != credential_binding.capability_set_digest
        || sandbox_binding.expected_credential_verifier
            != credential_binding.expected_credential_verifier
    {
        bail!("Adapter adoption upstream lineage is incompatible");
    }
    Ok(ExternalPoolAdapterAdoptionBinding {
        application_id: credential_binding.application_id.clone(),
        application_digest: credential_binding.application_digest.clone(),
        provider_id: credential_binding.provider_id.clone(),
        provider_owner_account_id: credential_binding.provider_owner_account_id.clone(),
        provider_policy_revision: credential_binding.provider_policy_revision,
        provider_digest: credential_binding.provider_digest.clone(),
        admission_id: credential_binding.admission_id.clone(),
        admission_digest: credential_binding.admission_digest.clone(),
        adapter_id: credential_binding.adapter_id.clone(),
        adapter_release_version: credential_binding.adapter_release_version.clone(),
        adapter_config_revision: credential_binding.adapter_config_revision,
        adapter_config_digest: credential_binding.adapter_config_digest.clone(),
        declared_implementation_sha256: credential_binding.declared_implementation_sha256.clone(),
        capability_set_digest: credential_binding.capability_set_digest.clone(),
        sandbox_conformance_receipt_id: sandbox_receipt.sandbox_conformance_receipt_id.clone(),
        sandbox_conformance_receipt_digest: sandbox_receipt
            .sandbox_conformance_receipt_digest
            .clone(),
        sandbox_report_expires_at: sandbox_binding.report_expires_at.clone(),
        credential_verification_receipt_id: credential_receipt
            .credential_verification_receipt_id
            .clone(),
        credential_verification_receipt_digest: credential_receipt
            .credential_verification_receipt_digest
            .clone(),
        credential_locator_commitment: credential_binding.credential_locator_commitment.clone(),
        credential_report_expires_at: credential_binding.report_expires_at.clone(),
    })
}

fn adoption_write_receipt(
    adoption: &StoredExternalPoolAdapterAdoption,
    terminal: Option<&StoredExternalPoolAdapterAdoptionTerminal>,
    replayed: bool,
) -> ExternalPoolAdapterAdoptionWriteReceipt {
    ExternalPoolAdapterAdoptionWriteReceipt {
        adoption: adoption.summary(),
        terminal: terminal.map(StoredExternalPoolAdapterAdoptionTerminal::summary),
        replayed,
    }
}

fn validate_adopt_input(input: &AdoptExternalPoolAdapter) -> Result<()> {
    for value in [
        &input.application_id,
        &input.admission_id,
        &input.credential_verification_receipt_id,
        &input.adopted_by_admin_user_id,
        &input.idempotency_scope,
        &input.idempotency_key,
    ] {
        identifier(value)?;
    }
    for value in [
        &input.expected_application_digest,
        &input.expected_admission_digest,
        &input.expected_sandbox_conformance_receipt_digest,
        &input.expected_credential_verification_receipt_digest,
    ] {
        digest(value)?;
    }
    if input.confirmation != ADOPTION_CONFIRMATION {
        bail!("Adapter adoption requires confirmation");
    }
    Ok(())
}

fn validate_revoke_input(input: &RevokeExternalPoolAdapterAdoption) -> Result<()> {
    for value in [
        &input.adoption_receipt_id,
        &input.revoked_by_admin_user_id,
        &input.reason,
        &input.idempotency_scope,
        &input.idempotency_key,
    ] {
        identifier(value)?;
    }
    digest(&input.expected_adoption_receipt_digest)?;
    if input.reason.len() > 1000 || input.confirmation != ADOPTION_REVOCATION_CONFIRMATION {
        bail!("Adapter adoption revocation requires a bounded reason and confirmation");
    }
    Ok(())
}

fn identifier(value: &str) -> Result<()> {
    if value.is_empty()
        || value.trim() != value
        || value.len() > 1000
        || value.chars().any(char::is_control)
    {
        bail!("Adapter adoption input identifier is invalid");
    }
    Ok(())
}

fn digest(value: &str) -> Result<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        bail!("Adapter adoption input digest is invalid");
    }
    Ok(())
}

fn ensure_adoption_replay(
    receipt: &ExternalPoolAdapterAdoptionReceipt,
    input: &AdoptExternalPoolAdapter,
) -> Result<()> {
    let item = &receipt.adoption;
    let binding = &item.binding;
    if binding.application_id != input.application_id
        || binding.application_digest != input.expected_application_digest
        || binding.admission_id != input.admission_id
        || binding.admission_digest != input.expected_admission_digest
        || binding.sandbox_conformance_receipt_digest
            != input.expected_sandbox_conformance_receipt_digest
        || binding.credential_verification_receipt_id != input.credential_verification_receipt_id
        || binding.credential_verification_receipt_digest
            != input.expected_credential_verification_receipt_digest
        || item.adopted_by_admin_user_id != input.adopted_by_admin_user_id
        || item.confirmation != input.confirmation
        || item.idempotency_scope != input.idempotency_scope
        || item.idempotency_key != input.idempotency_key
    {
        bail!("Adapter adoption idempotency key conflicts with immutable history");
    }
    Ok(())
}

fn ensure_terminal_replay(
    receipt: &ExternalPoolAdapterAdoptionTerminalReceipt,
    input: &RevokeExternalPoolAdapterAdoption,
) -> Result<()> {
    let item = &receipt.terminal;
    if item.adoption_receipt_id != input.adoption_receipt_id
        || item.adoption_receipt_digest != input.expected_adoption_receipt_digest
        || item.revoked_by_admin_user_id != input.revoked_by_admin_user_id
        || item.reason != input.reason
        || item.confirmation != input.confirmation
        || item.idempotency_scope != input.idempotency_scope
        || item.idempotency_key != input.idempotency_key
    {
        bail!("Adapter adoption revocation idempotency key conflicts with immutable history");
    }
    Ok(())
}

fn insert_adoption(
    tx: &Transaction<'_>,
    receipt: &ExternalPoolAdapterAdoptionReceipt,
    json: &str,
) -> Result<()> {
    let item = &receipt.adoption;
    let b = &item.binding;
    tx.execute("INSERT INTO compute_external_pool_adapter_adoption_receipts(
        adoption_receipt_id,adoption_receipt_digest,receipt_json,adoption_material_digest,
        application_id,application_digest,provider_id,provider_owner_account_id,provider_policy_revision,
        provider_digest,admission_id,admission_digest,adapter_id,adapter_release_version,
        adapter_config_revision,adapter_config_digest,declared_implementation_sha256,capability_set_digest,
        sandbox_conformance_receipt_id,sandbox_conformance_receipt_digest,sandbox_report_expires_at,
        credential_verification_receipt_id,credential_verification_receipt_digest,
        credential_locator_commitment,credential_report_expires_at,adopted_by_admin_user_id,
        confirmation,idempotency_scope,idempotency_key,adopted_at,recorded_at,adoption_effect,
        install_effect,provider_effect,route_effect,execution_effect,settlement_effect)
        VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,
                ?19,?20,?21,?22,?23,?24,?25,?26,?27,?28,?29,?30,?31,?32,?33,?34,?35,?36,?37)",
        params![receipt.adoption_receipt_id,receipt.adoption_receipt_digest,json,receipt.adoption_material_digest,
        b.application_id,b.application_digest,b.provider_id,b.provider_owner_account_id,b.provider_policy_revision,
        b.provider_digest,b.admission_id,b.admission_digest,b.adapter_id,b.adapter_release_version,b.adapter_config_revision,
        b.adapter_config_digest,b.declared_implementation_sha256,b.capability_set_digest,b.sandbox_conformance_receipt_id,
        b.sandbox_conformance_receipt_digest,b.sandbox_report_expires_at,b.credential_verification_receipt_id,
        b.credential_verification_receipt_digest,b.credential_locator_commitment,b.credential_report_expires_at,
        item.adopted_by_admin_user_id,item.confirmation,item.idempotency_scope,item.idempotency_key,item.adopted_at,
        item.recorded_at,item.adoption_effect,item.install_effect,item.provider_effect,item.route_effect,
        item.execution_effect,item.settlement_effect])?;
    Ok(())
}

fn insert_terminal(
    tx: &Transaction<'_>,
    receipt: &ExternalPoolAdapterAdoptionTerminalReceipt,
    json: &str,
) -> Result<()> {
    let item = &receipt.terminal;
    tx.execute(
        "INSERT INTO compute_external_pool_adapter_adoption_terminal_receipts(
        terminal_receipt_id,terminal_receipt_digest,receipt_json,terminal_material_digest,
        adoption_receipt_id,adoption_receipt_digest,revoked_by_admin_user_id,reason,confirmation,
        idempotency_scope,idempotency_key,revoked_at,recorded_at,adoption_effect,provider_effect,
        route_effect,execution_effect,settlement_effect)
        VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18)",
        params![
            receipt.terminal_receipt_id,
            receipt.terminal_receipt_digest,
            json,
            receipt.terminal_material_digest,
            item.adoption_receipt_id,
            item.adoption_receipt_digest,
            item.revoked_by_admin_user_id,
            item.reason,
            item.confirmation,
            item.idempotency_scope,
            item.idempotency_key,
            item.revoked_at,
            item.recorded_at,
            item.adoption_effect,
            item.provider_effect,
            item.route_effect,
            item.execution_effect,
            item.settlement_effect
        ],
    )?;
    Ok(())
}
