use anyhow::{bail, Result};
use chrono::{SecondsFormat, Utc};
use rusqlite::{params, Transaction, TransactionBehavior};

use crate::{
    compute_federation::external_pool_adapter_sandbox_reattestation::*,
    store::{new_id, Store},
};

use super::{read::*, types::*};

impl Store {
    pub(crate) fn revoke_external_pool_adapter_sandbox_reattestation(
        &self,
        input: RevokeExternalPoolAdapterSandboxReattestation,
    ) -> Result<ExternalPoolAdapterSandboxReattestationRevocationWriteReceipt> {
        validate_input(&input)?;
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(revocation) =
            revocation_by_idempotency_on(&tx, &input.idempotency_scope, &input.idempotency_key)?
        {
            ensure_replay(&tx, &revocation, &input)?;
            let target = receipt_by_id_on(&tx, &input.reattestation_receipt_id)?
                .ok_or_else(|| anyhow::anyhow!("sandbox re-attestation target disappeared"))?;
            let output = ExternalPoolAdapterSandboxReattestationRevocationWriteReceipt {
                reattestation: target.summary(),
                revocation: revocation.summary(),
                replayed: true,
            };
            tx.commit()?;
            return Ok(output);
        }
        let target = receipt_by_id_on(&tx, &input.reattestation_receipt_id)?
            .ok_or_else(|| anyhow::anyhow!("sandbox re-attestation target was not found"))?;
        if target.receipt.reattestation_receipt_digest
            != input.expected_reattestation_receipt_digest
            || revocation_by_receipt_on(&tx, &input.reattestation_receipt_id)?.is_some()
        {
            bail!("sandbox re-attestation revocation target is not exact");
        }
        let binding = &target.receipt.reattestation.binding;
        let head = head_by_release_on(&tx, &binding.registry_release_id)?
            .ok_or_else(|| anyhow::anyhow!("sandbox re-attestation head was not found"))?;
        if head.receipt.reattestation_receipt_id != input.reattestation_receipt_id {
            bail!("only the sandbox re-attestation head can be revoked");
        }
        let timestamp = Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true);
        let material = ExternalPoolAdapterSandboxReattestationRevocationMaterial {
            reattestation_receipt_id: target.receipt.reattestation_receipt_id.clone(),
            reattestation_receipt_digest: target.receipt.reattestation_receipt_digest.clone(),
            registry_release_id: binding.registry_release_id.clone(),
            registry_release_digest: binding.registry_release_digest.clone(),
            revoked_by_admin_user_id: input.revoked_by_admin_user_id,
            reason: input.reason,
            confirmation: input.confirmation,
            idempotency_scope: input.idempotency_scope,
            idempotency_key: input.idempotency_key,
            revoked_at: timestamp.clone(),
            recorded_at: timestamp,
            revocation_effect: SANDBOX_REATTESTATION_REVOCATION_EFFECT.into(),
            adapter_effect: SANDBOX_REATTESTATION_NO_EFFECT.into(),
            provider_effect: SANDBOX_REATTESTATION_NO_EFFECT.into(),
            credential_effect: SANDBOX_REATTESTATION_NO_EFFECT.into(),
            route_effect: SANDBOX_REATTESTATION_NO_EFFECT.into(),
            execution_effect: SANDBOX_REATTESTATION_NO_EFFECT.into(),
            settlement_effect: SANDBOX_REATTESTATION_NO_EFFECT.into(),
        };
        let mut receipt = ExternalPoolAdapterSandboxReattestationRevocationReceipt {
            schema: SANDBOX_REATTESTATION_REVOCATION_RECEIPT_SCHEMA.into(),
            revocation_receipt_id: new_id("external_pool_adapter_sandbox_reattestation_revocation"),
            revocation_receipt_digest: String::new(),
            revocation_material_digest: sandbox_reattestation_revocation_material_digest(
                &material,
            )?,
            canonicalization: SANDBOX_REATTESTATION_CANONICALIZATION.into(),
            digest_algorithm: SANDBOX_REATTESTATION_DIGEST_ALGORITHM.into(),
            revocation: material,
        };
        receipt.revocation_receipt_digest =
            sandbox_reattestation_revocation_receipt_json_and_digest(&receipt)?.1;
        validate_sandbox_reattestation_revocation_receipt(&receipt)?;
        let (json, _) = sandbox_reattestation_revocation_receipt_json_and_digest(&receipt)?;
        insert(&tx, &receipt, &json)?;
        let stored = revocation_by_receipt_on(&tx, &input.reattestation_receipt_id)?
            .ok_or_else(|| anyhow::anyhow!("sandbox re-attestation revocation disappeared"))?;
        let output = ExternalPoolAdapterSandboxReattestationRevocationWriteReceipt {
            reattestation: target.summary(),
            revocation: stored.summary(),
            replayed: false,
        };
        tx.commit()?;
        Ok(output)
    }
}

fn validate_input(input: &RevokeExternalPoolAdapterSandboxReattestation) -> Result<()> {
    for value in [
        &input.reattestation_receipt_id,
        &input.revoked_by_admin_user_id,
        &input.idempotency_scope,
        &input.idempotency_key,
    ] {
        if value.trim() != value || value.is_empty() || value.chars().count() > 240 {
            bail!("sandbox re-attestation revocation identifier is invalid");
        }
    }
    if input.expected_reattestation_receipt_digest.len() != 64
        || input.reason.trim() != input.reason
        || !(12..=500).contains(&input.reason.chars().count())
        || input.confirmation != SANDBOX_REATTESTATION_REVOCATION_CONFIRMATION
    {
        bail!("sandbox re-attestation revocation input is invalid");
    }
    Ok(())
}

fn ensure_replay(
    tx: &Transaction<'_>,
    stored: &StoredSandboxReattestationRevocation,
    input: &RevokeExternalPoolAdapterSandboxReattestation,
) -> Result<()> {
    let item = &stored.receipt.revocation;
    let target = receipt_by_id_on(tx, &item.reattestation_receipt_id)?
        .ok_or_else(|| anyhow::anyhow!("sandbox re-attestation replay lost target"))?;
    if item.reattestation_receipt_id != input.reattestation_receipt_id
        || item.reattestation_receipt_digest != input.expected_reattestation_receipt_digest
        || item.revoked_by_admin_user_id != input.revoked_by_admin_user_id
        || item.reason != input.reason
        || item.confirmation != input.confirmation
        || item.idempotency_scope != input.idempotency_scope
        || item.idempotency_key != input.idempotency_key
        || target.receipt.reattestation_receipt_digest != item.reattestation_receipt_digest
    {
        bail!("sandbox re-attestation revocation idempotency conflicts with history");
    }
    Ok(())
}

fn insert(
    tx: &Transaction<'_>,
    receipt: &ExternalPoolAdapterSandboxReattestationRevocationReceipt,
    json: &str,
) -> Result<()> {
    let item = &receipt.revocation;
    tx.execute(
        "INSERT INTO compute_external_pool_adapter_sandbox_reattestation_revocations(
          revocation_receipt_id,revocation_receipt_digest,receipt_json,revocation_material_digest,
          reattestation_receipt_id,reattestation_receipt_digest,registry_release_id,
          registry_release_digest,revoked_by_admin_user_id,reason,confirmation,idempotency_scope,
          idempotency_key,revoked_at,recorded_at,revocation_effect,adapter_effect,provider_effect,
          credential_effect,route_effect,execution_effect,settlement_effect
        ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22)",
        params![
            receipt.revocation_receipt_id,
            receipt.revocation_receipt_digest,
            json,
            receipt.revocation_material_digest,
            item.reattestation_receipt_id,
            item.reattestation_receipt_digest,
            item.registry_release_id,
            item.registry_release_digest,
            item.revoked_by_admin_user_id,
            item.reason,
            item.confirmation,
            item.idempotency_scope,
            item.idempotency_key,
            item.revoked_at,
            item.recorded_at,
            item.revocation_effect,
            item.adapter_effect,
            item.provider_effect,
            item.credential_effect,
            item.route_effect,
            item.execution_effect,
            item.settlement_effect
        ],
    )?;
    Ok(())
}
