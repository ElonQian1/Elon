use anyhow::{bail, Result};
use chrono::{SecondsFormat, Utc};
use rusqlite::{params, TransactionBehavior};

use crate::{
    compute_federation::external_pool_adapter_provider_runtime_readiness::*,
    store::{new_id, Store},
};

use super::{
    error::ExternalPoolAdapterProviderRuntimeReadinessStoreError as StoreError,
    persistence::insert_revocation,
    read::{
        readiness_by_id_on, readiness_head_by_binding_on, revocation_by_idempotency_on,
        revocation_by_readiness_on,
    },
    types::*,
};

impl Store {
    pub(crate) fn revoke_external_pool_adapter_provider_runtime_readiness(
        &self,
        input: RevokeExternalPoolAdapterProviderRuntimeReadiness,
    ) -> std::result::Result<
        ExternalPoolAdapterProviderRuntimeReadinessRevocationWriteReceipt,
        StoreError,
    > {
        let target = self
            .external_pool_provider_activation_candidate_audit_target(&input.candidate_id)
            .map_err(StoreError::storage)?
            .ok_or_else(|| StoreError::conflict(anyhow::anyhow!("candidate was not found")))?;
        authorize_actor(&input, &target.provider_owner_account_id)?;
        let mut conn = self.conn().map_err(StoreError::storage)?;
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| StoreError::storage(error.into()))?;
        let output = (|| -> Result<_> {
            if let Some(stored) =
                revocation_by_idempotency_on(&tx, &input.idempotency_scope, &input.idempotency_key)?
            {
                let readiness =
                    readiness_by_id_on(&tx, &stored.receipt.revocation.readiness_receipt_id)?
                        .ok_or_else(|| {
                            anyhow::anyhow!("readiness revocation replay lost its receipt")
                        })?;
                ensure_revocation_replay(&input, &readiness, &stored)?;
                tx.commit()?;
                return Ok(
                    ExternalPoolAdapterProviderRuntimeReadinessRevocationWriteReceipt {
                        readiness: provider_runtime_readiness_safe_summary(&readiness.receipt),
                        revocation: stored.receipt,
                        replayed: true,
                    },
                );
            }
            validate_actor_on(
                &tx,
                &input.revoked_by_actor_kind,
                &input.revoked_by_actor_user_id,
                &target.provider_owner_account_id,
            )?;
            let readiness = readiness_by_id_on(&tx, &input.readiness_receipt_id)?
                .ok_or_else(|| anyhow::anyhow!("readiness receipt was not found"))?;
            ensure_revocation_target(&input, &readiness)?;
            let head = readiness_head_by_binding_on(&tx, &input.provider_binding_id)?
                .ok_or_else(|| anyhow::anyhow!("readiness lineage head disappeared"))?;
            if head.receipt.readiness_receipt_id != readiness.receipt.readiness_receipt_id
                || revocation_by_readiness_on(&tx, &input.readiness_receipt_id)?.is_some()
            {
                bail!("only the latest unrevoked readiness receipt may be revoked")
            }
            let recorded_at = std::cmp::max(now(), readiness.receipt.readiness.checked_at.clone());
            let r = &readiness.receipt.readiness;
            let receipt =
                build_external_pool_adapter_provider_runtime_readiness_revocation_receipt(
                    new_id("external_pool_adapter_provider_runtime_readiness_revocation"),
                    ExternalPoolAdapterProviderRuntimeReadinessRevocationMaterial {
                        readiness_receipt_id: readiness.receipt.readiness_receipt_id.clone(),
                        readiness_receipt_digest: readiness
                            .receipt
                            .readiness_receipt_digest
                            .clone(),
                        provider_binding_id: r.provider_binding_id.clone(),
                        provider_binding_digest: r.provider_binding_digest.clone(),
                        candidate_id: r.candidate_id.clone(),
                        candidate_digest: r.candidate_digest.clone(),
                        profile_id: r.profile_id.clone(),
                        profile_digest: r.profile_digest.clone(),
                        target_id: r.target_id.clone(),
                        target_digest: r.target_digest.clone(),
                        companion_id: r.companion_id.clone(),
                        companion_digest: r.companion_digest.clone(),
                        provider_id: r.provider_id.clone(),
                        revoked_by_actor_kind: input.revoked_by_actor_kind.clone(),
                        revoked_by_actor_user_id: input.revoked_by_actor_user_id.clone(),
                        reason: input.reason.clone(),
                        revoked_at: recorded_at.clone(),
                        recorded_at,
                        idempotency_scope: input.idempotency_scope.clone(),
                        idempotency_key: input.idempotency_key.clone(),
                        confirmation: input.confirmation.clone(),
                        revocation_status: PROVIDER_RUNTIME_READINESS_REVOCATION_STATUS.into(),
                        effects: provider_runtime_readiness_no_effects(),
                        readiness: provider_runtime_readiness_no_readiness(),
                    },
                )?;
            insert_revocation(&tx, &receipt)?;
            let stored = revocation_by_readiness_on(&tx, &input.readiness_receipt_id)?
                .ok_or_else(|| anyhow::anyhow!("readiness revocation disappeared after insert"))?;
            let output = ExternalPoolAdapterProviderRuntimeReadinessRevocationWriteReceipt {
                readiness: provider_runtime_readiness_safe_summary(&readiness.receipt),
                revocation: stored.receipt,
                replayed: false,
            };
            tx.commit()?;
            Ok(output)
        })()
        .map_err(StoreError::classify_write)?;
        Ok(output)
    }
}

fn authorize_actor(
    input: &RevokeExternalPoolAdapterProviderRuntimeReadiness,
    owner_user_id: &str,
) -> std::result::Result<(), StoreError> {
    if input.revoked_by_actor_kind == PROVIDER_RUNTIME_READINESS_ACTOR_PROVIDER_OWNER
        && input.revoked_by_actor_user_id != owner_user_id
    {
        return Err(StoreError::conflict(anyhow::anyhow!(
            "readiness revocation actor is not the Provider owner"
        )));
    }
    if !matches!(
        input.revoked_by_actor_kind.as_str(),
        PROVIDER_RUNTIME_READINESS_ACTOR_PLATFORM_ADMIN
            | PROVIDER_RUNTIME_READINESS_ACTOR_PROVIDER_OWNER
    ) {
        return Err(StoreError::conflict(anyhow::anyhow!(
            "readiness revocation actor kind is invalid"
        )));
    }
    Ok(())
}

fn validate_actor_on(
    conn: &rusqlite::Connection,
    actor_kind: &str,
    actor_user_id: &str,
    owner_user_id: &str,
) -> Result<()> {
    let active: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM users WHERE id=?1 AND status='active')",
        params![actor_user_id],
        |row| row.get(0),
    )?;
    let platform: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM users
          WHERE id=?1 AND status='active' AND role IN ('admin','owner'))",
        params![actor_user_id],
        |row| row.get(0),
    )?;
    if !active
        || (actor_kind == PROVIDER_RUNTIME_READINESS_ACTOR_PLATFORM_ADMIN && !platform)
        || (actor_kind == PROVIDER_RUNTIME_READINESS_ACTOR_PROVIDER_OWNER
            && actor_user_id != owner_user_id)
    {
        bail!("readiness revocation actor is not authorized")
    }
    Ok(())
}

fn ensure_revocation_target(
    input: &RevokeExternalPoolAdapterProviderRuntimeReadiness,
    stored: &StoredProviderRuntimeReadiness,
) -> Result<()> {
    let r = &stored.receipt.readiness;
    if stored.receipt.readiness_receipt_id != input.readiness_receipt_id
        || stored.receipt.readiness_receipt_digest != input.expected_readiness_receipt_digest
        || r.provider_binding_id != input.provider_binding_id
        || r.candidate_id != input.candidate_id
        || r.profile_id != input.profile_id
        || r.target_id != input.target_id
        || r.companion_id != input.companion_id
    {
        bail!("readiness revocation target is not exact")
    }
    Ok(())
}

fn ensure_revocation_replay(
    input: &RevokeExternalPoolAdapterProviderRuntimeReadiness,
    readiness: &StoredProviderRuntimeReadiness,
    revocation: &StoredProviderRuntimeReadinessRevocation,
) -> Result<()> {
    ensure_revocation_target(input, readiness)?;
    let r = &revocation.receipt.revocation;
    let value = &readiness.receipt.readiness;
    if r.readiness_receipt_id != readiness.receipt.readiness_receipt_id
        || r.readiness_receipt_digest != readiness.receipt.readiness_receipt_digest
        || r.provider_binding_id != value.provider_binding_id
        || r.provider_binding_digest != value.provider_binding_digest
        || r.candidate_id != value.candidate_id
        || r.candidate_digest != value.candidate_digest
        || r.profile_id != value.profile_id
        || r.profile_digest != value.profile_digest
        || r.target_id != value.target_id
        || r.target_digest != value.target_digest
        || r.companion_id != value.companion_id
        || r.companion_digest != value.companion_digest
        || r.provider_id != value.provider_id
        || r.revoked_by_actor_kind != input.revoked_by_actor_kind
        || r.revoked_by_actor_user_id != input.revoked_by_actor_user_id
        || r.reason != input.reason
        || r.idempotency_scope != input.idempotency_scope
        || r.idempotency_key != input.idempotency_key
        || r.confirmation != input.confirmation
    {
        bail!("readiness revocation replay conflicts with sealed input")
    }
    Ok(())
}

fn now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true)
}
