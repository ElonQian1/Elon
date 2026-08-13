use anyhow::{bail, Result};
use chrono::{SecondsFormat, Utc};
use rusqlite::{Transaction, TransactionBehavior};

use crate::{
    compute_federation::external_pool_adapter_credential_reattestation::*,
    store::{new_id, Store},
};

use super::{persistence::insert_revocation, read::*, types::*};

impl Store {
    pub(crate) fn revoke_external_pool_adapter_credential_reattestation(
        &self,
        input: RevokeExternalPoolAdapterCredentialReattestation,
    ) -> Result<ExternalPoolAdapterCredentialReattestationRevocationWriteReceipt> {
        validate_input(&input)?;
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(revocation) =
            revocation_by_idempotency_on(&tx, &input.idempotency_scope, &input.idempotency_key)?
        {
            ensure_replay(&revocation, &input)?;
            let target = receipt_by_id_on(&tx, &input.reattestation_receipt_id)?
                .ok_or_else(|| anyhow::anyhow!("credential re-attestation replay lost target"))?;
            let output = ExternalPoolAdapterCredentialReattestationRevocationWriteReceipt {
                reattestation: target.summary(),
                revocation: revocation.summary(),
                replayed: true,
            };
            tx.commit()?;
            return Ok(output);
        }

        let target = receipt_by_id_on(&tx, &input.reattestation_receipt_id)?
            .ok_or_else(|| anyhow::anyhow!("credential re-attestation was not found"))?;
        if target.receipt.reattestation_receipt_digest
            != input.expected_reattestation_receipt_digest
            || revocation_by_receipt_on(&tx, &input.reattestation_receipt_id)?.is_some()
        {
            bail!("credential re-attestation revocation target is not exact");
        }
        let binding = &target.receipt.reattestation.binding;
        let head = head_by_provider_binding_on(&tx, &binding.provider_binding_id)?
            .ok_or_else(|| anyhow::anyhow!("credential re-attestation head was not found"))?;
        if head.receipt.reattestation_receipt_id != input.reattestation_receipt_id
            || head.receipt.reattestation_receipt_digest
                != input.expected_reattestation_receipt_digest
        {
            bail!("only the exact credential re-attestation head can be revoked");
        }

        let timestamp = Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true);
        let material = ExternalPoolAdapterCredentialReattestationRevocationMaterial {
            reattestation_receipt_id: target.receipt.reattestation_receipt_id.clone(),
            reattestation_receipt_digest: target.receipt.reattestation_receipt_digest.clone(),
            provider_binding_id: binding.provider_binding_id.clone(),
            provider_binding_digest: binding.provider_binding_digest.clone(),
            revoked_by_admin_user_id: input.revoked_by_admin_user_id,
            reason: input.reason,
            confirmation: input.confirmation,
            idempotency_scope: input.idempotency_scope,
            idempotency_key: input.idempotency_key,
            revoked_at: timestamp.clone(),
            recorded_at: timestamp,
            revocation_effect: CREDENTIAL_REATTESTATION_REVOCATION_EFFECT.into(),
            adapter_effect: CREDENTIAL_REATTESTATION_NO_EFFECT.into(),
            provider_effect: CREDENTIAL_REATTESTATION_NO_EFFECT.into(),
            route_effect: CREDENTIAL_REATTESTATION_NO_EFFECT.into(),
            execution_effect: CREDENTIAL_REATTESTATION_NO_EFFECT.into(),
            usage_effect: CREDENTIAL_REATTESTATION_NO_EFFECT.into(),
            settlement_effect: CREDENTIAL_REATTESTATION_NO_EFFECT.into(),
        };
        let mut receipt = ExternalPoolAdapterCredentialReattestationRevocationReceipt {
            schema: CREDENTIAL_REATTESTATION_REVOCATION_RECEIPT_SCHEMA.into(),
            revocation_receipt_id: new_id(
                "external_pool_adapter_credential_reattestation_revocation",
            ),
            revocation_receipt_digest: String::new(),
            revocation_material_digest: credential_reattestation_revocation_material_digest(
                &material,
            )?,
            canonicalization: CREDENTIAL_REATTESTATION_CANONICALIZATION.into(),
            digest_algorithm: CREDENTIAL_REATTESTATION_DIGEST_ALGORITHM.into(),
            revocation: material,
        };
        receipt.revocation_receipt_digest =
            credential_reattestation_revocation_receipt_json_and_digest(&receipt)?.1;
        validate_credential_reattestation_revocation_receipt(&receipt)?;
        let (json, _) = credential_reattestation_revocation_receipt_json_and_digest(&receipt)?;
        insert_revocation(&tx, &receipt, &json)?;
        let stored = revocation_by_receipt_on(&tx, &input.reattestation_receipt_id)?
            .ok_or_else(|| anyhow::anyhow!("credential re-attestation revocation disappeared"))?;
        if stored.receipt != receipt || stored.receipt_json != json {
            bail!("credential re-attestation revocation changed during exact readback");
        }
        let output = ExternalPoolAdapterCredentialReattestationRevocationWriteReceipt {
            reattestation: target.summary(),
            revocation: stored.summary(),
            replayed: false,
        };
        tx.commit()?;
        Ok(output)
    }
}

fn validate_input(input: &RevokeExternalPoolAdapterCredentialReattestation) -> Result<()> {
    identifier(&input.reattestation_receipt_id, 200)?;
    digest(&input.expected_reattestation_receipt_digest)?;
    identifier(&input.revoked_by_admin_user_id, 200)?;
    identifier(&input.idempotency_scope, 240)?;
    identifier(&input.idempotency_key, 240)?;
    if input.confirmation != CREDENTIAL_REATTESTATION_REVOCATION_CONFIRMATION
        || input.reason.trim() != input.reason
        || !(12..=500).contains(&input.reason.chars().count())
        || input.reason.chars().any(char::is_control)
    {
        bail!("credential re-attestation revocation input is invalid");
    }
    Ok(())
}

fn ensure_replay(
    stored: &StoredCredentialReattestationRevocation,
    input: &RevokeExternalPoolAdapterCredentialReattestation,
) -> Result<()> {
    let item = &stored.receipt.revocation;
    if item.reattestation_receipt_id != input.reattestation_receipt_id
        || item.reattestation_receipt_digest != input.expected_reattestation_receipt_digest
        || item.revoked_by_admin_user_id != input.revoked_by_admin_user_id
        || item.reason != input.reason
        || item.confirmation != input.confirmation
        || item.idempotency_scope != input.idempotency_scope
        || item.idempotency_key != input.idempotency_key
    {
        bail!("credential re-attestation revocation idempotency conflicts with history");
    }
    Ok(())
}

fn identifier(value: &str, max: usize) -> Result<()> {
    if value.is_empty()
        || value.trim() != value
        || value.chars().count() > max
        || value.chars().any(char::is_control)
    {
        bail!("credential re-attestation revocation identifier is invalid");
    }
    Ok(())
}

fn digest(value: &str) -> Result<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        bail!("credential re-attestation revocation digest is invalid");
    }
    Ok(())
}
