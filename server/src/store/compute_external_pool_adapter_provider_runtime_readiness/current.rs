use anyhow::{bail, Result};
use chrono::{DateTime, SecondsFormat, Utc};
use rusqlite::{params, OptionalExtension, Transaction, TransactionBehavior};

use crate::{
    compute_federation::{
        external_pool_adapter_installation::PreparedExternalPoolAdapterInstallation,
        external_pool_adapter_provider_runtime_readiness::*,
    },
    store::{
        compute_external_pool_adapter_credential_reattestation::current_external_pool_adapter_credential_reattestation_authority_on,
        compute_external_pool_adapter_runtime_bundle::{
            current_external_pool_adapter_runtime_bundle_authority_on,
            ExternalPoolAdapterProviderRuntimeReadinessRuntime,
        },
        compute_external_pool_adapter_runtime_compatibility_verification::current_external_pool_adapter_runtime_compatibility_verification_authority_on,
        compute_external_pool_adapter_sandbox_reattestation::current_external_pool_adapter_sandbox_reattestation_authority_on,
        compute_external_pool_adapter_supervisor_session_policy_companion::current_external_pool_adapter_supervisor_session_policy_companion_authority_on,
        compute_external_pool_adapter_vulnerability_reattestation::current_external_pool_adapter_vulnerability_reattestation_authority_on,
        Store,
    },
};

use super::{
    error::ExternalPoolAdapterProviderRuntimeReadinessStoreError as StoreError,
    read::{identifier, readiness_by_id_on},
    types::*,
};

struct RelationalCurrentness {
    head_status: String,
    revocation_status: String,
    ttl_status: String,
    provider_binding_status: String,
    provider_status: String,
    candidate_status: String,
    profile_status: String,
    target_status: String,
    companion_status: String,
    vulnerability_status: String,
    sandbox_status: String,
    credential_status: String,
    compatibility_status: String,
    current_status: String,
}

impl Store {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn external_pool_adapter_provider_runtime_readiness_currentness(
        &self,
        provider_binding_id: &str,
        candidate_id: &str,
        profile_id: &str,
        target_id: &str,
        companion_id: &str,
        readiness_receipt_id: &str,
        runtime: Option<&ExternalPoolAdapterProviderRuntimeReadinessRuntime>,
    ) -> std::result::Result<
        Option<ExternalPoolAdapterProviderRuntimeReadinessCurrentnessSummary>,
        StoreError,
    > {
        for value in [
            provider_binding_id,
            candidate_id,
            profile_id,
            target_id,
            companion_id,
            readiness_receipt_id,
        ] {
            identifier(value).map_err(StoreError::conflict)?;
        }
        let mut conn = self.conn().map_err(StoreError::storage)?;
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Deferred)
            .map_err(|error| StoreError::storage(error.into()))?;
        let output = (|| -> Result<_> {
            let Some(stored) = readiness_by_id_on(&tx, readiness_receipt_id)? else {
                return Ok(None);
            };
            let r = &stored.receipt.readiness;
            if r.provider_binding_id != provider_binding_id
                || r.candidate_id != candidate_id
                || r.profile_id != profile_id
                || r.target_id != target_id
                || r.companion_id != companion_id
            {
                return Ok(None);
            }
            let relational = relational_currentness_on(&tx, readiness_receipt_id)?
                .ok_or_else(|| anyhow::anyhow!("readiness currentness view lost its receipt"))?;
            let checked_at = Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true);
            let process_seal = match runtime {
                Some(runtime)
                    if runtime.process_custody().attests_custody_epoch_digest(
                        &r.sealed_bindings.runtime_custody_epoch_digest,
                    ) =>
                {
                    runtime.process_custody().attests_readiness_seal(
                        &stored.receipt.readiness_receipt_id,
                        &stored.receipt.readiness_receipt_digest,
                        &r.sealed_bindings.runtime_bundle_identity_commitment,
                        &r.sealed_bindings.post_cleanup_observation_commitment,
                        &r.expires_at,
                    )?
                }
                _ => false,
            };
            let relational_current = relational.current_status
                == "relationally_current_requires_process_custody_reproof";
            let currentness_status = if relational_current && process_seal {
                "relationally_current_same_process_private_bundle_reproof_required"
            } else {
                PROVIDER_RUNTIME_READINESS_HISTORICAL_STATUS
            };
            Ok(Some(
                ExternalPoolAdapterProviderRuntimeReadinessCurrentnessSummary {
                    schema: PROVIDER_RUNTIME_READINESS_CURRENTNESS_SCHEMA.into(),
                    readiness: provider_runtime_readiness_safe_summary(&stored.receipt),
                    currentness_status: currentness_status.into(),
                    head_status: relational.head_status,
                    provider_binding_status: relational.provider_binding_status,
                    provider_status: relational.provider_status,
                    candidate_status: relational.candidate_status,
                    profile_status: relational.profile_status,
                    target_status: relational.target_status,
                    companion_status: relational.companion_status,
                    vulnerability_reattestation_status: relational.vulnerability_status,
                    sandbox_reattestation_status: relational.sandbox_status,
                    credential_reattestation_status: relational.credential_status,
                    runtime_compatibility_verification_status: relational.compatibility_status,
                    runtime_custody_epoch_status: if process_seal {
                        "same_process_epoch_and_observation_seal_present"
                    } else {
                        "unavailable_restarted_or_unregistered"
                    }
                    .into(),
                    runtime_bundle_identity_status: if process_seal {
                        "private_reproof_required"
                    } else {
                        "historical_only"
                    }
                    .into(),
                    ttl_status: relational.ttl_status,
                    revocation_status: relational.revocation_status,
                    checked_at,
                    effects: r.effects.clone(),
                    current_readiness: provider_runtime_readiness_no_readiness(),
                },
            ))
        })()
        .map_err(StoreError::storage)?;
        tx.commit()
            .map_err(|error| StoreError::storage(error.into()))?;
        Ok(output)
    }
}

#[allow(clippy::too_many_arguments)]
pub(in crate::store) fn current_external_pool_adapter_provider_runtime_readiness_authority_on<
    'tx,
    'conn,
>(
    transaction: &'tx Transaction<'conn>,
    readiness_receipt_id: &str,
    expected_readiness_receipt_digest: &str,
    bundle_prepared: PreparedExternalPoolAdapterInstallation,
    session_prepared: PreparedExternalPoolAdapterInstallation,
    runtime: &ExternalPoolAdapterProviderRuntimeReadinessRuntime,
    checked_at: &str,
) -> Result<Option<CurrentExternalPoolAdapterProviderRuntimeReadinessAuthority<'tx, 'conn>>> {
    let Some(stored) = readiness_by_id_on(transaction, readiness_receipt_id)? else {
        return Ok(None);
    };
    if stored.receipt.readiness_receipt_digest != expected_readiness_receipt_digest {
        bail!("provider runtime readiness expected receipt digest is not exact")
    }
    let r = &stored.receipt.readiness;
    let relational = relational_currentness_on(transaction, readiness_receipt_id)?
        .ok_or_else(|| anyhow::anyhow!("readiness current view disappeared"))?;
    if relational.current_status != "relationally_current_requires_process_custody_reproof"
        || canonical_time(&r.expires_at)? <= canonical_time(checked_at)?
        || !runtime
            .process_custody()
            .attests_custody_epoch_digest(&r.sealed_bindings.runtime_custody_epoch_digest)
        || !runtime.process_custody().attests_readiness_seal(
            &stored.receipt.readiness_receipt_id,
            &stored.receipt.readiness_receipt_digest,
            &r.sealed_bindings.runtime_bundle_identity_commitment,
            &r.sealed_bindings.post_cleanup_observation_commitment,
            &r.expires_at,
        )?
    {
        bail!("provider runtime readiness is historical or lacks exact process custody")
    }
    let bundle = current_external_pool_adapter_runtime_bundle_authority_on(
        transaction,
        &r.profile_id,
        bundle_prepared,
        runtime.bundle_root(),
        checked_at,
    )?
    .ok_or_else(|| anyhow::anyhow!("readiness lost its current V256 bundle"))?;
    let companion = current_external_pool_adapter_supervisor_session_policy_companion_authority_on(
        transaction,
        &r.companion_id,
        &r.companion_digest,
        session_prepared,
        checked_at,
    )?
    .ok_or_else(|| anyhow::anyhow!("readiness lost its current V259 companion"))?;
    let vulnerability = current_external_pool_adapter_vulnerability_reattestation_authority_on(
        transaction,
        &r.registry_release_id,
        &r.vulnerability_reattestation_receipt_id,
        &r.vulnerability_reattestation_receipt_digest,
        checked_at,
    )?
    .ok_or_else(|| anyhow::anyhow!("readiness lost exact current V250"))?;
    let sandbox = current_external_pool_adapter_sandbox_reattestation_authority_on(
        transaction,
        &r.registry_release_id,
        &r.sandbox_reattestation_receipt_id,
        &r.sandbox_reattestation_receipt_digest,
        checked_at,
    )?
    .ok_or_else(|| anyhow::anyhow!("readiness lost exact current V252"))?;
    let credential = current_external_pool_adapter_credential_reattestation_authority_on(
        transaction,
        &r.provider_binding_id,
        &r.credential_reattestation_receipt_id,
        &r.credential_reattestation_receipt_digest,
        checked_at,
    )?
    .ok_or_else(|| anyhow::anyhow!("readiness lost exact current V253"))?;
    let runtime_compatibility =
        current_external_pool_adapter_runtime_compatibility_verification_authority_on(
            transaction,
            &r.runtime_compatibility_verification_receipt_id,
            &r.runtime_compatibility_verification_receipt_digest,
            checked_at,
        )?
        .ok_or_else(|| anyhow::anyhow!("readiness lost exact current V268"))?;
    if !runtime
        .process_custody()
        .attests_runtime_bundle_identity_commitment(
            &bundle,
            &r.sealed_bindings.runtime_bundle_identity_commitment,
        )?
    {
        bail!("provider runtime readiness bundle identity commitment changed")
    }
    audit_current_roots(
        &stored.receipt,
        &bundle,
        &companion,
        &vulnerability,
        &sandbox,
        &credential,
        &runtime_compatibility,
        checked_at,
    )?;
    Ok(Some(
        CurrentExternalPoolAdapterProviderRuntimeReadinessAuthority::new(
            transaction,
            stored.receipt,
            bundle,
            companion,
            vulnerability,
            sandbox,
            credential,
            runtime_compatibility,
            checked_at.into(),
        ),
    ))
}

fn relational_currentness_on(
    conn: &rusqlite::Connection,
    readiness_receipt_id: &str,
) -> Result<Option<RelationalCurrentness>> {
    conn.query_row(
        "SELECT head_status,revocation_status,ttl_status,provider_binding_status,
                provider_status,candidate_status,profile_status,target_status,companion_status,
                vulnerability_reattestation_status,sandbox_reattestation_status,
                credential_reattestation_status,runtime_compatibility_verification_status,
                current_status
           FROM compute_external_pool_adapter_provider_runtime_readiness_current
          WHERE readiness_receipt_id=?1",
        params![readiness_receipt_id],
        |row| {
            Ok(RelationalCurrentness {
                head_status: row.get(0)?,
                revocation_status: row.get(1)?,
                ttl_status: row.get(2)?,
                provider_binding_status: row.get(3)?,
                provider_status: row.get(4)?,
                candidate_status: row.get(5)?,
                profile_status: row.get(6)?,
                target_status: row.get(7)?,
                companion_status: row.get(8)?,
                vulnerability_status: row.get(9)?,
                sandbox_status: row.get(10)?,
                credential_status: row.get(11)?,
                compatibility_status: row.get(12)?,
                current_status: row.get(13)?,
            })
        },
    )
    .optional()
    .map_err(Into::into)
}

#[allow(clippy::too_many_arguments)]
fn audit_current_roots(
    receipt: &ExternalPoolAdapterProviderRuntimeReadinessReceipt,
    bundle: &crate::store::compute_external_pool_adapter_runtime_bundle::CurrentExternalPoolAdapterRuntimeBundleAuthority<'_, '_>,
    companion: &crate::store::compute_external_pool_adapter_supervisor_session_policy_companion::CurrentExternalPoolAdapterSupervisorSessionPolicyCompanionAuthority,
    vulnerability: &crate::store::compute_external_pool_adapter_vulnerability_reattestation::CurrentExternalPoolAdapterVulnerabilityReattestationAuthority,
    sandbox: &crate::store::compute_external_pool_adapter_sandbox_reattestation::CurrentExternalPoolAdapterSandboxReattestationAuthority,
    credential: &crate::store::compute_external_pool_adapter_credential_reattestation::CurrentExternalPoolAdapterCredentialReattestationAuthority,
    compatibility: &crate::store::compute_external_pool_adapter_runtime_compatibility_verification::CurrentExternalPoolAdapterRuntimeCompatibilityVerificationAuthority<'_, '_>,
    checked_at: &str,
) -> Result<()> {
    let r = &receipt.readiness;
    let c = &companion.companion().companion;
    let profile = bundle.launch_profile().profile();
    let verification = compatibility.verification();
    let run = compatibility.run_observation();
    let release = compatibility.release();
    if bundle.checked_at() != checked_at
        || companion.checked_at() != checked_at
        || compatibility.checked_at() != checked_at
        || vulnerability.checked_at() != checked_at
        || sandbox.checked_at() != checked_at
        || credential.checked_at() != checked_at
        || c.provider_binding_id != r.provider_binding_id
        || c.provider_binding_digest != r.provider_binding_digest
        || c.registry_release_id != r.registry_release_id
        || c.registry_release_digest != r.registry_release_digest
        || release.registry_release_material_digest != r.registry_release_material_digest
        || c.installation_receipt_id != r.installation_receipt_id
        || c.installation_receipt_digest != r.installation_receipt_digest
        || c.installation_content_digest != r.installation_content_digest
        || c.candidate_id != r.candidate_id
        || c.candidate_digest != r.candidate_digest
        || c.delegation_id != r.delegation_id
        || c.delegation_digest != r.delegation_digest
        || profile.profile_id != r.profile_id
        || profile.profile_digest != r.profile_digest
        || c.target_id != r.target_id
        || c.target_digest != r.target_digest
        || companion.companion().companion_id != r.companion_id
        || companion.companion().companion_digest != r.companion_digest
        || c.provider_id != r.provider_id
        || c.provider_policy_revision != r.provider_policy_revision
        || c.provider_digest != r.provider_digest
        || c.provider_status != r.provider_status
        || vulnerability.receipt().reattestation_receipt_id
            != r.vulnerability_reattestation_receipt_id
        || vulnerability.receipt().reattestation_receipt_digest
            != r.vulnerability_reattestation_receipt_digest
        || sandbox.receipt().reattestation_receipt_id != r.sandbox_reattestation_receipt_id
        || sandbox.receipt().reattestation_receipt_digest != r.sandbox_reattestation_receipt_digest
        || credential.receipt().reattestation_receipt_id != r.credential_reattestation_receipt_id
        || credential.receipt().reattestation_receipt_digest
            != r.credential_reattestation_receipt_digest
        || verification.verification_receipt_id != r.runtime_compatibility_verification_receipt_id
        || verification.verification_receipt_digest
            != r.runtime_compatibility_verification_receipt_digest
        || c.launch_policy_digest != r.launch_policy_digest
        || c.target_policy_digest != r.target_policy_digest
        || c.entrypoint_capsule_policy_digest != r.entrypoint_capsule_policy_digest
        || c.supervisor_session_policy_digest != r.supervisor_session_policy_digest
        || run.observation.source_capsule_sha256 != r.source_capsule_sha256
        || run.observation.source_capsule_size_bytes != r.source_capsule_size_bytes
        || run.observation.launch_image_sha256 != r.launch_image_sha256
        || run.observation.launch_image_size_bytes != r.launch_image_size_bytes
    {
        bail!("provider runtime readiness current roots are not exact")
    }
    Ok(())
}

fn canonical_time(value: &str) -> Result<DateTime<Utc>> {
    let parsed = DateTime::parse_from_rfc3339(value)?;
    if parsed.offset().local_minus_utc() != 0
        || parsed.to_rfc3339_opts(SecondsFormat::Nanos, true) != value
    {
        bail!("provider runtime readiness timestamp is not canonical UTC nanos")
    }
    Ok(parsed.with_timezone(&Utc))
}
