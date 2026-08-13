use anyhow::{bail, Result};
use chrono::{SecondsFormat, Utc};
use rusqlite::TransactionBehavior;

use crate::store::{
    compute_external_pool_adapter_installation::external_pool_adapter_installation_receipt_authority_on,
    compute_external_pool_adapter_runtime_bundle::external_pool_adapter_entrypoint_capsule_policy_root,
    compute_external_pool_adapter_runtime_launch_profile::historical_external_pool_adapter_runtime_launch_profile_authority_on,
    compute_external_pool_adapter_upstream_transport_target::{
        audit_replay_prepared as audit_target_replay_prepared,
        current_external_pool_adapter_upstream_transport_target_authority_on,
        historical_external_pool_adapter_upstream_transport_target_authority_on,
    },
    Store,
};

use super::{
    build::*, input::*, persistence::*, policy::supervisor_session_policy_catalog, read::*,
    roots::*, types::*,
};

impl Store {
    pub(crate) fn create_external_pool_adapter_supervisor_session_policy_companion(
        &self,
        input: CreateExternalPoolAdapterSupervisorSessionPolicyCompanion,
    ) -> Result<ExternalPoolAdapterSupervisorSessionPolicyCompanionWriteReceipt> {
        validate_create_input(&input)?;
        validate_actor_for_owner(
            &input.recorded_by_actor_kind,
            &input.recorded_by_actor_user_id,
            &input.prepared.binding().provider_owner_account_id,
        )?;
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(stored) =
            companion_by_idempotency_on(&tx, &input.idempotency_scope, &input.idempotency_key)?
        {
            ensure_create_replay(&tx, &input, &stored)?;
            let output = ExternalPoolAdapterSupervisorSessionPolicyCompanionWriteReceipt {
                companion: stored.summary(),
                replayed: true,
            };
            tx.commit()?;
            return Ok(output);
        }
        validate_platform_actor_on(
            &tx,
            &input.recorded_by_actor_kind,
            &input.recorded_by_actor_user_id,
        )?;
        let policy = supervisor_session_policy_catalog()?;
        if policy.digest != input.expected_supervisor_session_policy_digest {
            bail!("supervisor session companion expected policy digest is not exact")
        }
        let historical_target =
            historical_external_pool_adapter_upstream_transport_target_authority_on(
                &tx,
                &input.target_id,
                &input.expected_target_digest,
            )?
            .ok_or_else(|| anyhow::anyhow!("historical exact V258 target was not found"))?;
        let binding_id = historical_target.target.provider_binding_id;
        let previous = head_for_fresh(&tx, &binding_id, &input)?;
        let sequence =
            previous.as_ref().map_or(Ok(1), |p| {
                p.receipt.companion.sequence.checked_add(1).ok_or_else(|| {
                    anyhow::anyhow!("supervisor session companion sequence overflow")
                })
            })?;
        let recorded_at = previous.as_ref().map_or_else(now, |p| {
            std::cmp::max(now(), p.receipt.companion.recorded_at.clone())
        });
        let CreateExternalPoolAdapterSupervisorSessionPolicyCompanion {
            prepared,
            target_id,
            expected_target_digest,
            expected_profile_digest,
            expected_candidate_digest,
            expected_provider_binding_digest,
            recorded_by_actor_kind,
            recorded_by_actor_user_id,
            idempotency_scope,
            idempotency_key,
            confirmation,
            ..
        } = input;
        let authority = current_external_pool_adapter_upstream_transport_target_authority_on(
            &tx,
            &target_id,
            &expected_target_digest,
            prepared,
            &recorded_at,
        )?
        .ok_or_else(|| anyhow::anyhow!("current exact V258 target was not found"))?;
        let t = &authority.target().target;
        if t.profile_digest != expected_profile_digest
            || t.candidate_digest != expected_candidate_digest
            || t.provider_binding_digest != expected_provider_binding_digest
            || t.provider_binding_id != binding_id
        {
            bail!("supervisor session companion current V258/V255 roots are not exact")
        }
        validate_actor_for_owner(
            &recorded_by_actor_kind,
            &recorded_by_actor_user_id,
            &t.provider_owner_account_id,
        )?;
        let capsule = external_pool_adapter_entrypoint_capsule_policy_root()?;
        let receipt = build_companion(
            &authority,
            &capsule.0,
            capsule.1,
            &capsule.2,
            previous.as_ref(),
            sequence,
            &recorded_at,
            &recorded_by_actor_kind,
            &recorded_by_actor_user_id,
            &idempotency_scope,
            &idempotency_key,
            &confirmation,
        )?;
        audit_current_roots(&authority, &receipt)?;
        insert_companion(&tx, &receipt)?;
        let stored = companion_by_id_on(&tx, &receipt.companion_id)?.ok_or_else(|| {
            anyhow::anyhow!("supervisor session companion disappeared after insert")
        })?;
        let output = ExternalPoolAdapterSupervisorSessionPolicyCompanionWriteReceipt {
            companion: stored.summary(),
            replayed: false,
        };
        tx.commit()?;
        Ok(output)
    }
    pub(crate) fn revoke_external_pool_adapter_supervisor_session_policy_companion(
        &self,
        input: RevokeExternalPoolAdapterSupervisorSessionPolicyCompanion,
    ) -> Result<ExternalPoolAdapterSupervisorSessionPolicyCompanionRevocationWriteReceipt> {
        validate_revoke_input(&input)?;
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(r) =
            revocation_by_idempotency_on(&tx, &input.idempotency_scope, &input.idempotency_key)?
        {
            let c = companion_by_id_on(&tx, &r.receipt.revocation.companion_id)?
                .ok_or_else(|| anyhow::anyhow!("companion revocation replay lost companion"))?;
            audit_revoke_historical_roots(&tx, &c)?;
            ensure_revoke(&input, &c, &r)?;
            let output =
                ExternalPoolAdapterSupervisorSessionPolicyCompanionRevocationWriteReceipt {
                    companion: c.summary(),
                    revocation: r.summary(),
                    replayed: true,
                };
            tx.commit()?;
            return Ok(output);
        }
        validate_platform_actor_on(
            &tx,
            &input.revoked_by_actor_kind,
            &input.revoked_by_actor_user_id,
        )?;
        let c = companion_by_id_on(&tx, &input.companion_id)?
            .ok_or_else(|| anyhow::anyhow!("supervisor session companion was not found"))?;
        audit_revoke_historical_roots(&tx, &c)?;
        ensure_target(&input, &c)?;
        let head = companion_head_by_binding_on(&tx, &c.receipt.companion.provider_binding_id)?
            .ok_or_else(|| anyhow::anyhow!("supervisor session companion head disappeared"))?;
        if head.receipt.companion_id != c.receipt.companion_id
            || revocation_by_companion_on(&tx, &input.companion_id)?.is_some()
        {
            bail!("only latest unrevoked supervisor session companion may be revoked")
        }
        let at = std::cmp::max(now(), c.receipt.companion.recorded_at.clone());
        let receipt = build_revocation(&input, &c, &at)?;
        insert_revocation(&tx, &receipt)?;
        let r = revocation_by_companion_on(&tx, &input.companion_id)?.ok_or_else(|| {
            anyhow::anyhow!("supervisor session companion revocation disappeared")
        })?;
        let output = ExternalPoolAdapterSupervisorSessionPolicyCompanionRevocationWriteReceipt {
            companion: c.summary(),
            revocation: r.summary(),
            replayed: false,
        };
        tx.commit()?;
        Ok(output)
    }
}
fn head_for_fresh(
    conn: &rusqlite::Connection,
    binding: &str,
    input: &CreateExternalPoolAdapterSupervisorSessionPolicyCompanion,
) -> Result<Option<StoredSupervisorSessionPolicyCompanion>> {
    let p = companion_head_by_binding_on(conn, binding)?;
    match (
        p,
        &input.predecessor_companion_id,
        &input.expected_predecessor_companion_digest,
    ) {
        (None, None, None) => Ok(None),
        (Some(p), Some(id), Some(digest))
            if p.receipt.companion_id == *id && p.receipt.companion_digest == *digest =>
        {
            Ok(Some(p))
        }
        _ => bail!("supervisor session companion predecessor is missing, stale, or unexpected"),
    }
}
fn ensure_create_replay(
    conn: &rusqlite::Connection,
    input: &CreateExternalPoolAdapterSupervisorSessionPolicyCompanion,
    stored: &StoredSupervisorSessionPolicyCompanion,
) -> Result<()> {
    let c = &stored.receipt.companion;
    if c.target_id != input.target_id
        || c.target_digest != input.expected_target_digest
        || c.profile_digest != input.expected_profile_digest
        || c.candidate_digest != input.expected_candidate_digest
        || c.provider_binding_digest != input.expected_provider_binding_digest
        || c.supervisor_session_policy_digest != input.expected_supervisor_session_policy_digest
        || c.predecessor_companion_id != input.predecessor_companion_id
        || c.predecessor_companion_digest != input.expected_predecessor_companion_digest
        || c.recorded_by_actor_kind != input.recorded_by_actor_kind
        || c.recorded_by_actor_user_id != input.recorded_by_actor_user_id
        || c.idempotency_scope != input.idempotency_scope
        || c.idempotency_key != input.idempotency_key
        || c.confirmation != input.confirmation
    {
        bail!("supervisor session companion replay conflicts with sealed input")
    }
    let target = historical_external_pool_adapter_upstream_transport_target_authority_on(
        conn,
        &c.target_id,
        &c.target_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("companion replay lost historical V258 target"))?;
    let profile = historical_external_pool_adapter_runtime_launch_profile_authority_on(
        conn,
        &c.profile_id,
        &c.profile_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("companion replay lost historical V255 profile"))?;
    audit_historical_roots(&target, &profile, &stored.receipt)?;
    let installation = external_pool_adapter_installation_receipt_authority_on(
        conn,
        &c.installation_receipt_id,
        &c.installation_receipt_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("companion replay lost historical installation"))?;
    if input.prepared.binding() != &installation.receipt().installation.binding {
        bail!("companion replay Prepared installation binding is not exact")
    }
    audit_target_replay_prepared(&input.prepared, &profile)
}
fn ensure_target(
    input: &RevokeExternalPoolAdapterSupervisorSessionPolicyCompanion,
    c: &StoredSupervisorSessionPolicyCompanion,
) -> Result<()> {
    let x = &c.receipt.companion;
    if c.receipt.companion_id != input.companion_id
        || c.receipt.companion_digest != input.expected_companion_digest
        || x.target_digest != input.expected_target_digest
        || x.profile_digest != input.expected_profile_digest
    {
        bail!("supervisor session companion revocation target is not exact")
    }
    validate_actor_for_owner(
        &input.revoked_by_actor_kind,
        &input.revoked_by_actor_user_id,
        &x.provider_owner_account_id,
    )
}
fn audit_revoke_historical_roots(
    conn: &rusqlite::Connection,
    stored: &StoredSupervisorSessionPolicyCompanion,
) -> Result<()> {
    let c = &stored.receipt.companion;
    let target = historical_external_pool_adapter_upstream_transport_target_authority_on(
        conn,
        &c.target_id,
        &c.target_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("companion revocation lost historical V258 target"))?;
    let profile = historical_external_pool_adapter_runtime_launch_profile_authority_on(
        conn,
        &c.profile_id,
        &c.profile_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("companion revocation lost historical V255 profile"))?;
    audit_historical_roots(&target, &profile, &stored.receipt)
}
fn ensure_revoke(
    input: &RevokeExternalPoolAdapterSupervisorSessionPolicyCompanion,
    c: &StoredSupervisorSessionPolicyCompanion,
    r: &StoredSupervisorSessionPolicyCompanionRevocation,
) -> Result<()> {
    ensure_target(input, c)?;
    let x = &r.receipt.revocation;
    let companion = &c.receipt.companion;
    if x.companion_id != c.receipt.companion_id
        || x.companion_digest != c.receipt.companion_digest
        || x.target_id != companion.target_id
        || x.target_digest != companion.target_digest
        || x.profile_id != companion.profile_id
        || x.profile_digest != companion.profile_digest
        || x.provider_binding_id != companion.provider_binding_id
        || x.provider_binding_digest != companion.provider_binding_digest
        || x.provider_id != companion.provider_id
        || x.reason != input.reason
        || x.revoked_by_actor_kind != input.revoked_by_actor_kind
        || x.revoked_by_actor_user_id != input.revoked_by_actor_user_id
        || x.idempotency_scope != input.idempotency_scope
        || x.idempotency_key != input.idempotency_key
        || x.confirmation != input.confirmation
    {
        bail!("supervisor session companion revocation replay conflicts")
    };
    Ok(())
}
fn now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true)
}
