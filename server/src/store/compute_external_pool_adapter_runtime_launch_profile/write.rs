use anyhow::{bail, Result};
use chrono::{SecondsFormat, Utc};
use rusqlite::TransactionBehavior;

use crate::store::{
    compute_external_pool_provider_activation_candidate::current_external_pool_provider_activation_candidate_static_authority_on,
    Store,
};

use super::{
    build::*, input::*, persistence::*, policy::runtime_launch_policy_catalog, read::*, roots::*,
    types::*,
};

impl Store {
    pub(crate) fn create_external_pool_adapter_runtime_launch_profile(
        &self,
        input: CreateExternalPoolAdapterRuntimeLaunchProfile,
    ) -> Result<ExternalPoolAdapterRuntimeLaunchProfileWriteReceipt> {
        validate_create_input(&input)?;
        validate_create_actor(&input)?;
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(profile) =
            profile_by_idempotency_on(&tx, &input.idempotency_scope, &input.idempotency_key)?
        {
            ensure_create_replay(&tx, &input, &profile)?;
            let output = write_output(&profile, true);
            tx.commit()?;
            return Ok(output);
        }
        validate_platform_actor_on(
            &tx,
            &input.recorded_by_actor_kind,
            &input.recorded_by_actor_user_id,
        )?;
        let policy = runtime_launch_policy_catalog()?;
        if policy.digest != input.expected_launch_policy_digest {
            bail!("runtime launch profile expected policy digest is not exact");
        }
        let candidate_binding_id = candidate_binding_on(&tx, &input.candidate_id)?;
        if candidate_binding_id.0 != input.expected_provider_binding_digest {
            bail!("runtime launch profile expected Provider binding is not exact");
        }
        let previous = profile_head_for_fresh(&tx, &candidate_binding_id.1, &input)?;
        let sequence = next_sequence(previous.as_ref())?;
        let checked_at = monotonic_now(previous.as_ref());
        let CreateExternalPoolAdapterRuntimeLaunchProfile {
            prepared,
            candidate_id,
            expected_candidate_digest,
            recorded_by_actor_kind,
            recorded_by_actor_user_id,
            idempotency_scope,
            idempotency_key,
            confirmation,
            ..
        } = input;
        let authority = current_external_pool_provider_activation_candidate_static_authority_on(
            &tx,
            prepared,
            &candidate_id,
            &expected_candidate_digest,
            &checked_at,
        )?
        .ok_or_else(|| anyhow::anyhow!("current exact V254 candidate was not found"))?;
        if authority.candidate().candidate.provider_binding_digest != candidate_binding_id.0 {
            bail!("runtime launch profile expected Provider binding is not exact");
        }
        let (scheme, commitment) = credential_subject_on(&tx, authority.registry())?;
        let profile = build_profile(
            &authority,
            previous.as_ref(),
            &scheme,
            &commitment,
            sequence,
            &checked_at,
            &recorded_by_actor_kind,
            &recorded_by_actor_user_id,
            &idempotency_scope,
            &idempotency_key,
            &confirmation,
        )?;
        audit_current_roots(
            authority.registry(),
            authority.candidate(),
            &profile,
            &scheme,
            &commitment,
        )?;
        insert_profile(&tx, &profile)?;
        let stored = profile_by_id_on(&tx, &profile.profile_id)?
            .ok_or_else(|| anyhow::anyhow!("runtime launch profile disappeared after insert"))?;
        let output = write_output(&stored, false);
        tx.commit()?;
        Ok(output)
    }

    pub(crate) fn revoke_external_pool_adapter_runtime_launch_profile(
        &self,
        input: RevokeExternalPoolAdapterRuntimeLaunchProfile,
    ) -> Result<ExternalPoolAdapterRuntimeLaunchProfileRevocationWriteReceipt> {
        validate_revoke_input(&input)?;
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(revocation) =
            revocation_by_idempotency_on(&tx, &input.idempotency_scope, &input.idempotency_key)?
        {
            let profile = profile_by_id_on(&tx, &revocation.receipt.revocation.profile_id)?
                .ok_or_else(|| anyhow::anyhow!("revocation replay lost historical profile"))?;
            ensure_revocation_replay(&input, &profile, &revocation)?;
            let output = revocation_output(&profile, &revocation, true);
            tx.commit()?;
            return Ok(output);
        }
        validate_platform_actor_on(
            &tx,
            &input.revoked_by_actor_kind,
            &input.revoked_by_actor_user_id,
        )?;
        let profile = profile_by_id_on(&tx, &input.profile_id)?
            .ok_or_else(|| anyhow::anyhow!("runtime launch profile was not found"))?;
        ensure_revocation_target(&input, &profile)?;
        let head = profile_head_by_binding_on(&tx, &profile.receipt.profile.provider_binding_id)?
            .ok_or_else(|| {
            anyhow::anyhow!("runtime launch profile lineage head disappeared")
        })?;
        if head.receipt.profile_id != profile.receipt.profile_id {
            bail!("only the latest runtime launch profile may be revoked");
        }
        if revocation_by_profile_on(&tx, &input.profile_id)?.is_some() {
            bail!("runtime launch profile is already revoked under another idempotency key");
        }
        let revoked_at = std::cmp::max(now(), profile.receipt.profile.recorded_at.clone());
        let receipt = build_revocation(&input, &profile, &revoked_at)?;
        insert_revocation(&tx, &receipt)?;
        let stored = revocation_by_profile_on(&tx, &input.profile_id)?
            .ok_or_else(|| anyhow::anyhow!("runtime launch revocation disappeared after insert"))?;
        let output = revocation_output(&profile, &stored, false);
        tx.commit()?;
        Ok(output)
    }
}

fn profile_head_for_fresh(
    conn: &rusqlite::Connection,
    provider_binding_id: &str,
    input: &CreateExternalPoolAdapterRuntimeLaunchProfile,
) -> Result<Option<StoredRuntimeLaunchProfile>> {
    let previous = profile_head_by_binding_on(conn, provider_binding_id)?;
    match (
        previous,
        &input.predecessor_profile_id,
        &input.expected_predecessor_profile_digest,
    ) {
        (None, None, None) => Ok(None),
        (Some(previous), Some(id), Some(digest))
            if previous.receipt.profile_id == *id && previous.receipt.profile_digest == *digest =>
        {
            Ok(Some(previous))
        }
        _ => bail!("runtime launch profile predecessor is missing, stale, or unexpected"),
    }
}

fn candidate_binding_on(
    conn: &rusqlite::Connection,
    candidate_id: &str,
) -> Result<(String, String)> {
    Ok(conn.query_row(
        "SELECT provider_binding_digest,provider_binding_id
           FROM compute_external_pool_provider_activation_candidates
          WHERE candidate_id=?1",
        rusqlite::params![candidate_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?)
}

fn ensure_create_replay(
    conn: &rusqlite::Connection,
    input: &CreateExternalPoolAdapterRuntimeLaunchProfile,
    profile: &StoredRuntimeLaunchProfile,
) -> Result<()> {
    let p = &profile.receipt.profile;
    if p.candidate_id != input.candidate_id
        || p.candidate_digest != input.expected_candidate_digest
        || p.provider_binding_digest != input.expected_provider_binding_digest
        || p.launch_policy_digest != input.expected_launch_policy_digest
        || p.predecessor_profile_id != input.predecessor_profile_id
        || p.predecessor_profile_digest != input.expected_predecessor_profile_digest
        || p.recorded_by_actor_kind != input.recorded_by_actor_kind
        || p.recorded_by_actor_user_id != input.recorded_by_actor_user_id
        || p.confirmation != input.confirmation
    {
        bail!("runtime launch profile replay conflicts with sealed input");
    }
    let historical = crate::store::compute_external_pool_provider_activation_candidate::historical_external_pool_provider_activation_candidate_authority_on(
        conn,
        &p.candidate_id,
        &p.candidate_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("runtime launch profile replay lost V254 history"))?;
    if historical.candidate().candidate.delegation_id != p.delegation_id
        || historical.candidate().candidate.delegation_digest != p.delegation_digest
        || historical.delegation().delegation_id != p.delegation_id
        || historical.delegation().delegation_digest != p.delegation_digest
    {
        bail!("runtime launch profile replay V254 history is not exact");
    }
    let installation = crate::store::compute_external_pool_adapter_installation::external_pool_adapter_installation_receipt_authority_on(
        conn,
        &p.installation_receipt_id,
        &p.installation_receipt_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("runtime launch profile replay lost installation history"))?;
    if input.prepared.binding() != &installation.receipt().installation.binding {
        bail!("runtime launch profile replay Prepared installation root is not exact");
    }
    audit_replay_prepared(&input.prepared, &profile.receipt)
}

fn ensure_revocation_target(
    input: &RevokeExternalPoolAdapterRuntimeLaunchProfile,
    profile: &StoredRuntimeLaunchProfile,
) -> Result<()> {
    if profile.receipt.profile_id != input.profile_id
        || profile.receipt.profile_digest != input.expected_profile_digest
        || profile.receipt.profile.candidate_digest != input.expected_candidate_digest
    {
        bail!("runtime launch profile revocation target is not exact");
    }
    validate_actor_for_owner(
        &input.revoked_by_actor_kind,
        &input.revoked_by_actor_user_id,
        &profile.receipt.profile.provider_owner_account_id,
    )
}

fn ensure_revocation_replay(
    input: &RevokeExternalPoolAdapterRuntimeLaunchProfile,
    profile: &StoredRuntimeLaunchProfile,
    revocation: &StoredRuntimeLaunchProfileRevocation,
) -> Result<()> {
    ensure_revocation_target(input, profile)?;
    let r = &revocation.receipt.revocation;
    if r.reason != input.reason
        || r.idempotency_scope != input.idempotency_scope
        || r.idempotency_key != input.idempotency_key
        || r.revoked_by_actor_kind != input.revoked_by_actor_kind
        || r.revoked_by_actor_user_id != input.revoked_by_actor_user_id
    {
        bail!("runtime launch revocation replay conflicts with sealed input");
    }
    Ok(())
}

fn next_sequence(previous: Option<&StoredRuntimeLaunchProfile>) -> Result<u64> {
    match previous {
        None => Ok(1),
        Some(profile) => profile
            .receipt
            .profile
            .sequence
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("runtime launch profile sequence overflow")),
    }
}

fn monotonic_now(previous: Option<&StoredRuntimeLaunchProfile>) -> String {
    previous.map_or_else(now, |p| {
        std::cmp::max(now(), p.receipt.profile.recorded_at.clone())
    })
}

fn write_output(
    profile: &StoredRuntimeLaunchProfile,
    replayed: bool,
) -> ExternalPoolAdapterRuntimeLaunchProfileWriteReceipt {
    ExternalPoolAdapterRuntimeLaunchProfileWriteReceipt {
        profile: profile.summary(),
        replayed,
    }
}

fn revocation_output(
    profile: &StoredRuntimeLaunchProfile,
    revocation: &StoredRuntimeLaunchProfileRevocation,
    replayed: bool,
) -> ExternalPoolAdapterRuntimeLaunchProfileRevocationWriteReceipt {
    ExternalPoolAdapterRuntimeLaunchProfileRevocationWriteReceipt {
        profile: profile.summary(),
        revocation: revocation.summary(),
        replayed,
    }
}

fn now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true)
}
