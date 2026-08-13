use anyhow::{bail, Result};
use chrono::{SecondsFormat, Utc};
use rusqlite::TransactionBehavior;

use crate::store::{
    compute_external_pool_adapter_runtime_launch_profile::{
        current_external_pool_adapter_runtime_launch_profile_authority_on,
        historical_external_pool_adapter_runtime_launch_profile_authority_on,
    },
    Store,
};

use super::{
    build::*, input::*, persistence::*, policy::upstream_transport_target_policy_catalog, read::*,
    roots::*, types::*,
};

impl Store {
    pub(crate) fn create_external_pool_adapter_upstream_transport_target(
        &self,
        input: CreateExternalPoolAdapterUpstreamTransportTarget,
    ) -> Result<ExternalPoolAdapterUpstreamTransportTargetWriteReceipt> {
        validate_create_input(&input)?;
        validate_actor_for_owner(
            &input.recorded_by_actor_kind,
            &input.recorded_by_actor_user_id,
            &input.prepared.binding().provider_owner_account_id,
        )?;
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(target) =
            target_by_idempotency_on(&tx, &input.idempotency_scope, &input.idempotency_key)?
        {
            ensure_create_replay(&tx, &input, &target)?;
            let output = write_output(&target, true);
            tx.commit()?;
            return Ok(output);
        }
        validate_platform_actor_on(
            &tx,
            &input.recorded_by_actor_kind,
            &input.recorded_by_actor_user_id,
        )?;
        let policy = upstream_transport_target_policy_catalog()?;
        if policy.digest != input.expected_target_policy_digest {
            bail!("upstream transport target expected policy digest is not exact");
        }
        let (binding_id, binding_digest) = profile_binding_on(&tx, &input.profile_id)?;
        if binding_digest != input.expected_provider_binding_digest {
            bail!("upstream transport target expected Provider binding is not exact");
        }
        let previous = target_head_for_fresh(&tx, &binding_id, &input)?;
        let sequence = next_sequence(previous.as_ref())?;
        let checked_at = monotonic_now(previous.as_ref());
        let CreateExternalPoolAdapterUpstreamTransportTarget {
            prepared,
            profile_id,
            expected_profile_digest,
            expected_candidate_digest,
            recorded_by_actor_kind,
            recorded_by_actor_user_id,
            idempotency_scope,
            idempotency_key,
            confirmation,
            target,
            ..
        } = input;
        let authority = current_external_pool_adapter_runtime_launch_profile_authority_on(
            &tx,
            &profile_id,
            prepared,
            &checked_at,
        )?
        .ok_or_else(|| {
            anyhow::anyhow!("current exact V255 runtime launch profile was not found")
        })?;
        let profile = authority.profile();
        if profile.profile_digest != expected_profile_digest
            || profile.profile.candidate_digest != expected_candidate_digest
            || profile.profile.provider_binding_id != binding_id
            || profile.profile.provider_binding_digest != binding_digest
        {
            bail!("upstream transport target current V255 roots are not exact");
        }
        validate_actor_for_owner(
            &recorded_by_actor_kind,
            &recorded_by_actor_user_id,
            &profile.profile.provider_owner_account_id,
        )?;
        let receipt = build_target(
            &authority,
            previous.as_ref(),
            target,
            sequence,
            &checked_at,
            &recorded_by_actor_kind,
            &recorded_by_actor_user_id,
            &idempotency_scope,
            &idempotency_key,
            &confirmation,
        )?;
        audit_current_roots(&authority, &receipt)?;
        if authority.checked_at() != receipt.target.recorded_at {
            bail!("upstream transport target fresh roots were not recorded at one instant");
        }
        insert_target(&tx, &receipt)?;
        let stored = target_by_id_on(&tx, &receipt.target_id)?
            .ok_or_else(|| anyhow::anyhow!("upstream transport target disappeared after insert"))?;
        let output = write_output(&stored, false);
        tx.commit()?;
        Ok(output)
    }

    pub(crate) fn revoke_external_pool_adapter_upstream_transport_target(
        &self,
        input: RevokeExternalPoolAdapterUpstreamTransportTarget,
    ) -> Result<ExternalPoolAdapterUpstreamTransportTargetRevocationWriteReceipt> {
        validate_revoke_input(&input)?;
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(revocation) =
            revocation_by_idempotency_on(&tx, &input.idempotency_scope, &input.idempotency_key)?
        {
            let target = target_by_id_on(&tx, &revocation.receipt.revocation.target_id)?
                .ok_or_else(|| {
                    anyhow::anyhow!("target revocation replay lost historical target")
                })?;
            ensure_revocation_replay(&input, &target, &revocation)?;
            let output = revocation_output(&target, &revocation, true);
            tx.commit()?;
            return Ok(output);
        }
        validate_platform_actor_on(
            &tx,
            &input.revoked_by_actor_kind,
            &input.revoked_by_actor_user_id,
        )?;
        let target = target_by_id_on(&tx, &input.target_id)?
            .ok_or_else(|| anyhow::anyhow!("upstream transport target was not found"))?;
        ensure_revocation_target(&input, &target)?;
        let head = target_head_by_binding_on(&tx, &target.receipt.target.provider_binding_id)?
            .ok_or_else(|| anyhow::anyhow!("upstream transport target lineage head disappeared"))?;
        if head.receipt.target_id != target.receipt.target_id {
            bail!("only the latest upstream transport target may be revoked");
        }
        if revocation_by_target_on(&tx, &input.target_id)?.is_some() {
            bail!("upstream transport target is already revoked under another idempotency key");
        }
        let revoked_at = std::cmp::max(now(), target.receipt.target.recorded_at.clone());
        let receipt = build_revocation(&input, &target, &revoked_at)?;
        insert_revocation(&tx, &receipt)?;
        let stored = revocation_by_target_on(&tx, &input.target_id)?
            .ok_or_else(|| anyhow::anyhow!("target revocation disappeared after insert"))?;
        let output = revocation_output(&target, &stored, false);
        tx.commit()?;
        Ok(output)
    }
}

fn target_head_for_fresh(
    conn: &rusqlite::Connection,
    provider_binding_id: &str,
    input: &CreateExternalPoolAdapterUpstreamTransportTarget,
) -> Result<Option<StoredUpstreamTransportTarget>> {
    let previous = target_head_by_binding_on(conn, provider_binding_id)?;
    match (
        previous,
        &input.predecessor_target_id,
        &input.expected_predecessor_target_digest,
    ) {
        (None, None, None) => Ok(None),
        (Some(previous), Some(id), Some(digest))
            if previous.receipt.target_id == *id && previous.receipt.target_digest == *digest =>
        {
            Ok(Some(previous))
        }
        _ => bail!("upstream transport target predecessor is missing, stale, or unexpected"),
    }
}

fn profile_binding_on(conn: &rusqlite::Connection, profile_id: &str) -> Result<(String, String)> {
    Ok(conn.query_row(
        "SELECT provider_binding_id,provider_binding_digest
           FROM compute_external_pool_adapter_runtime_launch_profiles WHERE profile_id=?1",
        rusqlite::params![profile_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?)
}

fn ensure_create_replay(
    conn: &rusqlite::Connection,
    input: &CreateExternalPoolAdapterUpstreamTransportTarget,
    target: &StoredUpstreamTransportTarget,
) -> Result<()> {
    let t = &target.receipt.target;
    if t.profile_id != input.profile_id
        || t.profile_digest != input.expected_profile_digest
        || t.candidate_digest != input.expected_candidate_digest
        || t.provider_binding_digest != input.expected_provider_binding_digest
        || t.target_policy_digest != input.expected_target_policy_digest
        || t.dns_hostname != input.target.dns_hostname
        || t.port != input.target.port
        || t.tls_server_name != input.target.dns_hostname
        || t.expected_tls_leaf_spki_sha256 != input.target.expected_tls_leaf_spki_sha256
        || t.predecessor_target_id != input.predecessor_target_id
        || t.predecessor_target_digest != input.expected_predecessor_target_digest
        || t.recorded_by_actor_kind != input.recorded_by_actor_kind
        || t.recorded_by_actor_user_id != input.recorded_by_actor_user_id
        || t.confirmation != input.confirmation
    {
        bail!("upstream transport target replay conflicts with sealed input");
    }
    let profile = historical_external_pool_adapter_runtime_launch_profile_authority_on(
        conn,
        &t.profile_id,
        &t.profile_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("target replay lost historical V255 profile"))?;
    audit_historical_roots(&profile, &target.receipt)?;
    let installation = crate::store::compute_external_pool_adapter_installation::external_pool_adapter_installation_receipt_authority_on(
        conn,
        &t.installation_receipt_id,
        &t.installation_receipt_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("target replay lost historical installation"))?;
    if input.prepared.binding() != &installation.receipt().installation.binding {
        bail!("target replay Prepared installation binding is not exact");
    }
    audit_replay_prepared(&input.prepared, &profile)
}

fn ensure_revocation_target(
    input: &RevokeExternalPoolAdapterUpstreamTransportTarget,
    target: &StoredUpstreamTransportTarget,
) -> Result<()> {
    if target.receipt.target_id != input.target_id
        || target.receipt.target_digest != input.expected_target_digest
        || target.receipt.target.profile_digest != input.expected_profile_digest
    {
        bail!("upstream transport target revocation target is not exact");
    }
    validate_actor_for_owner(
        &input.revoked_by_actor_kind,
        &input.revoked_by_actor_user_id,
        &target.receipt.target.provider_owner_account_id,
    )
}

fn ensure_revocation_replay(
    input: &RevokeExternalPoolAdapterUpstreamTransportTarget,
    target: &StoredUpstreamTransportTarget,
    revocation: &StoredUpstreamTransportTargetRevocation,
) -> Result<()> {
    ensure_revocation_target(input, target)?;
    let r = &revocation.receipt.revocation;
    if r.reason != input.reason
        || r.idempotency_scope != input.idempotency_scope
        || r.idempotency_key != input.idempotency_key
        || r.revoked_by_actor_kind != input.revoked_by_actor_kind
        || r.revoked_by_actor_user_id != input.revoked_by_actor_user_id
        || r.confirmation != input.confirmation
    {
        bail!("upstream transport target revocation replay conflicts with sealed input");
    }
    Ok(())
}

fn next_sequence(previous: Option<&StoredUpstreamTransportTarget>) -> Result<u64> {
    match previous {
        None => Ok(1),
        Some(target) => target
            .receipt
            .target
            .sequence
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("upstream transport target sequence overflow")),
    }
}

fn monotonic_now(previous: Option<&StoredUpstreamTransportTarget>) -> String {
    previous.map_or_else(now, |target| {
        std::cmp::max(now(), target.receipt.target.recorded_at.clone())
    })
}

fn write_output(
    target: &StoredUpstreamTransportTarget,
    replayed: bool,
) -> ExternalPoolAdapterUpstreamTransportTargetWriteReceipt {
    ExternalPoolAdapterUpstreamTransportTargetWriteReceipt {
        target: target.summary(),
        replayed,
    }
}

fn revocation_output(
    target: &StoredUpstreamTransportTarget,
    revocation: &StoredUpstreamTransportTargetRevocation,
    replayed: bool,
) -> ExternalPoolAdapterUpstreamTransportTargetRevocationWriteReceipt {
    ExternalPoolAdapterUpstreamTransportTargetRevocationWriteReceipt {
        target: target.summary(),
        revocation: revocation.summary(),
        replayed,
    }
}

fn now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true)
}
