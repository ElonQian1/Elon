//! Final post-cleanup currentness reproof and same-transaction observation callback.

use std::marker::PhantomData;

use anyhow::{bail, Result};
use chrono::{DateTime, Duration as ChronoDuration, SecondsFormat, Utc};
use rusqlite::{Transaction, TransactionBehavior};

use super::{
    audit_runtime_compatibility_static_roots,
    CurrentExternalPoolAdapterNoWorkProbeObservationAuthority,
    MAX_PROVIDER_READINESS_PROBE_TIMEOUT_MS,
};
use crate::{
    compute_federation::{
        external_pool_adapter_installation::PreparedExternalPoolAdapterInstallation,
        external_pool_adapter_runtime_compatibility_verification::server_runtime_compatibility_v2_profile_catalog,
    },
    store::{
        compute_external_pool_adapter_credential_reattestation::CurrentExternalPoolAdapterCredentialReattestationAuthority,
        compute_external_pool_adapter_runtime_compatibility_verification::{
            current_external_pool_adapter_runtime_compatibility_verification_authority_on,
            CurrentExternalPoolAdapterRuntimeCompatibilityVerificationAuthority,
        },
        compute_external_pool_adapter_sandbox_reattestation::CurrentExternalPoolAdapterSandboxReattestationAuthority,
        compute_external_pool_adapter_supervisor_session_policy_companion::{
            current_external_pool_adapter_supervisor_session_policy_companion_authority_on,
            CurrentExternalPoolAdapterSupervisorSessionPolicyCompanionAuthority,
        },
        compute_external_pool_adapter_vulnerability_reattestation::CurrentExternalPoolAdapterVulnerabilityReattestationAuthority,
        Store,
    },
};
use elon_external_pool_adapter_session_core::ExternalPoolAdapterNoWorkProbeHostReceipt;

use super::super::{
    current::current_external_pool_adapter_runtime_bundle_authority_on,
    probe_preparation::{materialize_probe_preparation, select_current_probe_preparation_roots_on},
    runtime::{
        ExternalPoolAdapterPostCleanupCommitmentInput,
        ExternalPoolAdapterProviderRuntimeReadinessRuntime,
    },
    secret_delivery::{
        audit_delivery_roots, delivery_binding,
        CleanedExternalPoolAdapterEphemeralSecretDeliveryAuthority,
        ExternalPoolAdapterEphemeralSecretDeliveryBinding,
    },
    types::CurrentExternalPoolAdapterProbePreparationAuthority,
};

impl Store {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn with_reproved_external_pool_adapter_no_work_roots<Pending, Output>(
        &self,
        profile_id: &str,
        companion_id: &str,
        expected_companion_digest: &str,
        runtime_compatibility_verification_receipt_id: &str,
        expected_runtime_compatibility_verification_receipt_digest: &str,
        bundle_prepared: PreparedExternalPoolAdapterInstallation,
        session_prepared: PreparedExternalPoolAdapterInstallation,
        runtime: &ExternalPoolAdapterProviderRuntimeReadinessRuntime,
        receipt: ExternalPoolAdapterNoWorkProbeHostReceipt,
        selected_address: std::net::SocketAddr,
        cleaned: CleanedExternalPoolAdapterEphemeralSecretDeliveryAuthority,
        consume: impl FnOnce(
            &Transaction<'_>,
            &CurrentExternalPoolAdapterNoWorkProbeObservationAuthority<'_, '_, '_>,
        ) -> Result<Pending>,
        postcommit: impl FnOnce(&rusqlite::Connection, Pending) -> Result<Output>,
    ) -> Result<Option<Output>> {
        let mut connection = self.conn()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let checked_at = Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true);
        let Some(bundle) = current_external_pool_adapter_runtime_bundle_authority_on(
            &transaction,
            profile_id,
            bundle_prepared,
            runtime.bundle_root(),
            &checked_at,
        )?
        else {
            return Ok(None);
        };
        let Some(companion) =
            current_external_pool_adapter_supervisor_session_policy_companion_authority_on(
                &transaction,
                companion_id,
                expected_companion_digest,
                session_prepared,
                &checked_at,
            )?
        else {
            return Ok(None);
        };
        audit_delivery_roots(&bundle, &companion, &checked_at)?;
        let selected =
            select_current_probe_preparation_roots_on(&transaction, &bundle, &checked_at)?;
        let compatibility =
            current_external_pool_adapter_runtime_compatibility_verification_authority_on(
                &transaction,
                runtime_compatibility_verification_receipt_id,
                expected_runtime_compatibility_verification_receipt_digest,
                &checked_at,
            )?
            .ok_or_else(|| anyhow::anyhow!("no-work reproof lacks exact current V268 authority"))?;
        if !runtime
            .process_custody()
            .attests_runtime_bundle_identity_commitment(
                &bundle,
                cleaned.binding().runtime_bundle_identity_commitment(),
            )?
        {
            bail!("no-work runtime bundle commitment changed before final reproof");
        }
        let final_bundle_commitment = runtime
            .process_custody()
            .runtime_bundle_identity_commitment(&bundle)?;

        let expected = cleaned.binding();
        let roots = expected.session_root_arguments();
        let mut pending = None;
        materialize_probe_preparation(&bundle, &selected, |preparation| {
            audit_runtime_compatibility_roots(preparation, &companion, &compatibility)?;
            let observed = delivery_binding(
                preparation,
                &companion,
                expected.delivery_root(),
                &roots,
                final_bundle_commitment.clone(),
            )?;
            if &observed != expected {
                bail!("no-work probe roots changed after application exchange and cleanup");
            }
            let checked = canonical_time(&checked_at)?;
            let expires = minimum_observation_expiry(
                checked,
                &observed,
                preparation.vulnerability(),
                preparation.sandbox(),
                preparation.bundle().credential(),
                &compatibility,
            )?;
            if Utc::now() >= expires {
                bail!("no-work post-cleanup observation expired before final insert");
            }
            let post_cleanup_observation_commitment = runtime
                .process_custody()
                .post_cleanup_observation_commitment(
                    &ExternalPoolAdapterPostCleanupCommitmentInput {
                        runtime_bundle_identity_commitment: &final_bundle_commitment,
                        receipt: &receipt,
                        binding: &observed,
                        selected_address,
                        cleaned: &cleaned,
                    },
                )?;
            let observation = CurrentExternalPoolAdapterNoWorkProbeObservationAuthority {
                receipt: &receipt,
                binding: &observed,
                selected_address,
                launch_profile: preparation.bundle().launch_profile().profile(),
                vulnerability: preparation.vulnerability(),
                sandbox: preparation.sandbox(),
                credential: preparation.bundle().credential(),
                companion: &companion,
                runtime_compatibility: &compatibility,
                cleaned: &cleaned,
                runtime_bundle_identity_commitment: &final_bundle_commitment,
                post_cleanup_observation_commitment,
                custody_epoch_digest: runtime.custody_epoch_digest(),
                checked_at: checked_at.clone(),
                expires_at: expires.to_rfc3339_opts(SecondsFormat::Nanos, true),
                transaction: PhantomData,
            };
            if !observation.no_work_observed() {
                bail!("no-work post-cleanup observation was not authoritative");
            }
            let created = consume(&transaction, &observation)?;
            if Utc::now() >= expires {
                bail!("no-work post-cleanup observation expired before transaction commit");
            }
            pending = Some(created);
            Ok(())
        })?;
        drop(compatibility);
        drop(selected);
        drop(companion);
        drop(bundle);
        transaction.commit()?;
        let pending = pending
            .ok_or_else(|| anyhow::anyhow!("no-work final callback returned no pending output"))?;
        postcommit(&connection, pending).map(Some)
    }
}

fn audit_runtime_compatibility_roots(
    preparation: &CurrentExternalPoolAdapterProbePreparationAuthority<'_, '_, '_>,
    companion: &CurrentExternalPoolAdapterSupervisorSessionPolicyCompanionAuthority,
    compatibility: &CurrentExternalPoolAdapterRuntimeCompatibilityVerificationAuthority<'_, '_>,
) -> Result<()> {
    audit_runtime_compatibility_static_roots(companion.target(), compatibility)?;
    let catalog = server_runtime_compatibility_v2_profile_catalog()?;
    let run = &compatibility.run_observation().observation;
    let capsule = preparation.capsule();
    let companion_material = &companion.companion().companion;
    if catalog.profile.supervisor_session_policy.policy_digest
        != companion_material.supervisor_session_policy_digest
        || catalog.profile.source_capsule_policy.policy_digest != capsule.policy_digest()
        || run.source_capsule_policy_digest != capsule.policy_digest()
        || run.source_capsule_sha256 != capsule.entrypoint_sha256()
        || run.source_capsule_size_bytes != capsule.entrypoint_size_bytes()
        || run.launch_image_sha256 != capsule.launch_sha256()
        || run.launch_image_size_bytes != capsule.launch_size_bytes()
        || compatibility.checked_at() != preparation.bundle().checked_at()
        || compatibility.checked_at() != companion.checked_at()
    {
        bail!("V268 source, launch image, or current policy roots diverged");
    }
    Ok(())
}

fn minimum_observation_expiry(
    checked_at: DateTime<Utc>,
    binding: &ExternalPoolAdapterEphemeralSecretDeliveryBinding,
    vulnerability: &CurrentExternalPoolAdapterVulnerabilityReattestationAuthority,
    sandbox: &CurrentExternalPoolAdapterSandboxReattestationAuthority,
    credential: &CurrentExternalPoolAdapterCredentialReattestationAuthority,
    compatibility: &CurrentExternalPoolAdapterRuntimeCompatibilityVerificationAuthority<'_, '_>,
) -> Result<DateTime<Utc>> {
    let timeout_ms = u64::try_from(binding.probe_timeout().as_millis())?;
    if timeout_ms == 0 || timeout_ms > MAX_PROVIDER_READINESS_PROBE_TIMEOUT_MS {
        bail!("Provider readiness probe timeout exceeds the fixed maximum");
    }
    let mut expires = checked_at + ChronoDuration::milliseconds(i64::try_from(timeout_ms)?);
    for candidate in [
        &vulnerability
            .receipt()
            .reattestation
            .binding
            .intelligence
            .expires_at,
        &sandbox.receipt().reattestation.binding.report_expires_at,
        &credential.receipt().reattestation.binding.report_expires_at,
        &compatibility.verification().verification.expires_at,
    ] {
        expires = expires.min(canonical_time(candidate)?);
    }
    if checked_at >= expires {
        bail!("Provider readiness evidence expired at the post-cleanup anchor");
    }
    Ok(expires)
}

fn canonical_time(value: &str) -> Result<DateTime<Utc>> {
    let parsed = DateTime::parse_from_rfc3339(value)?;
    if parsed.offset().local_minus_utc() != 0
        || parsed.to_rfc3339_opts(SecondsFormat::Nanos, true) != value
    {
        bail!("Provider readiness timestamp is not canonical UTC nanoseconds");
    }
    Ok(parsed.with_timezone(&Utc))
}
