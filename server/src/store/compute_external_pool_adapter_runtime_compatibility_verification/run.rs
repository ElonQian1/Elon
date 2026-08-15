use anyhow::{bail, Result};
use chrono::{DateTime, SecondsFormat, Utc};
use rusqlite::TransactionBehavior;

use crate::{
    compute_federation::{
        external_pool_adapter_installation::PreparedExternalPoolAdapterInstallation,
        external_pool_adapter_linux_supervisor::ExternalPoolAdapterSupervisorCgroupParent,
        external_pool_adapter_runtime_compatibility_verification::*,
    },
    store::{
        compute_external_pool_adapter_registry::current_external_pool_adapter_registry_release_authority_on,
        compute_external_pool_adapter_sandbox_verifier_key::current_sandbox_verifier_key_authority_on,
        new_id, Store,
    },
};

use super::{
    error::ExternalPoolAdapterRuntimeCompatibilityVerificationStoreError as StoreError,
    persistence::insert_run_observation,
    read::{challenge_by_id_on, run_observation_by_challenge_on, run_observation_by_id_on},
    types::ExternalPoolAdapterRuntimeCompatibilityRunObservationWriteReceipt,
};

mod execution;
mod support;

enum RunPreflight {
    Fresh(ExternalPoolAdapterRuntimeCompatibilityChallengeReceipt),
    Replay {
        challenge: ExternalPoolAdapterRuntimeCompatibilityChallengeReceipt,
        run_observation: ExternalPoolAdapterRuntimeCompatibilityRunObservationReceipt,
    },
}

impl Store {
    #[allow(clippy::too_many_lines)]
    pub(in crate::store) fn run_external_pool_adapter_runtime_compatibility_verification_challenge(
        &self,
        challenge_id: &str,
        expected_challenge_digest: &str,
        prepared: PreparedExternalPoolAdapterInstallation,
        cgroup_parent: &ExternalPoolAdapterSupervisorCgroupParent,
    ) -> std::result::Result<
        ExternalPoolAdapterRuntimeCompatibilityRunObservationWriteReceipt,
        StoreError,
    > {
        super::read::identifier(challenge_id).map_err(StoreError::conflict)?;
        let audited_challenge = {
            let challenge = {
                let conn = self.conn().map_err(StoreError::storage)?;
                challenge_by_id_on(&conn, challenge_id)
                    .map_err(StoreError::storage)?
                    .ok_or_else(|| {
                        StoreError::conflict(anyhow::anyhow!("V268 challenge was not found"))
                    })?
            };
            if challenge.receipt.challenge_digest != expected_challenge_digest {
                return Err(StoreError::conflict(anyhow::anyhow!(
                    "V268 expected challenge digest is not exact"
                )));
            }
            support::audit_prepared_installation(&prepared, &challenge.receipt)
                .map_err(classify_runtime_file_audit_error)?;
            challenge.receipt
        };
        let preflight = {
            let mut conn = self.conn().map_err(StoreError::storage)?;
            let tx = conn
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .map_err(StoreError::storage)?;
            let current = challenge_by_id_on(&tx, challenge_id)
                .map_err(StoreError::storage)?
                .ok_or_else(|| {
                    StoreError::storage(anyhow::anyhow!(
                        "V268 challenge disappeared before preflight commit"
                    ))
                })?;
            if current.receipt != audited_challenge {
                return Err(StoreError::conflict(anyhow::anyhow!(
                    "V268 challenge drifted during preflight audit"
                )));
            }
            let state = if let Some(observation) =
                run_observation_by_challenge_on(&tx, challenge_id).map_err(StoreError::storage)?
            {
                RunPreflight::Replay {
                    challenge: current.receipt,
                    run_observation: observation.receipt,
                }
            } else {
                require_fresh_current_authority(&tx, &current.receipt, &now())
                    .map_err(StoreError::classify_write)?;
                RunPreflight::Fresh(current.receipt)
            };
            tx.commit()?;
            state
        };
        let challenge = match preflight {
            RunPreflight::Replay {
                challenge,
                run_observation,
            } => {
                let signature_challenge =
                    runtime_compatibility_signature_challenge(&challenge, &run_observation)
                        .map_err(StoreError::storage)?;
                return Ok(
                    ExternalPoolAdapterRuntimeCompatibilityRunObservationWriteReceipt {
                        run_observation,
                        signature_challenge,
                        replayed: true,
                    },
                );
            }
            RunPreflight::Fresh(challenge) => challenge,
        };
        let fixtures = support::load_public_fixtures(&prepared, &challenge)
            .map_err(classify_runtime_file_audit_error)?;
        let runner_execution_id =
            new_id("external_pool_adapter_runtime_compatibility_runner_execution");
        let run_started_at = now();
        let evidence = execution::execute(&challenge, &prepared, &fixtures, cgroup_parent)
            .map_err(StoreError::storage)?;
        let run_completed_at = now();
        let started = DateTime::parse_from_rfc3339(&run_started_at).map_err(StoreError::storage)?;
        let completed =
            DateTime::parse_from_rfc3339(&run_completed_at).map_err(StoreError::storage)?;
        if completed - started
            > chrono::Duration::seconds(RUNTIME_COMPATIBILITY_MAX_RUN_SECONDS as i64)
        {
            return Err(StoreError::storage(anyhow::anyhow!(
                "V268 server-owned run exceeded its exact duration window"
            )));
        }
        support::audit_prepared_installation(&prepared, &challenge)
            .map_err(classify_runtime_file_audit_error)?;
        let mut conn = self.conn().map_err(StoreError::storage)?;
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(StoreError::storage)?;
        let current = challenge_by_id_on(&tx, challenge_id)
            .map_err(StoreError::storage)?
            .ok_or_else(|| {
                StoreError::storage(anyhow::anyhow!(
                    "V268 challenge disappeared after execution"
                ))
            })?;
        if current.receipt != challenge {
            return Err(StoreError::conflict(anyhow::anyhow!(
                "V268 challenge drifted during execution"
            )));
        }
        if let Some(stored) =
            run_observation_by_challenge_on(&tx, challenge_id).map_err(StoreError::storage)?
        {
            let signature_challenge =
                runtime_compatibility_signature_challenge(&current.receipt, &stored.receipt)
                    .map_err(StoreError::storage)?;
            let output = ExternalPoolAdapterRuntimeCompatibilityRunObservationWriteReceipt {
                run_observation: stored.receipt,
                signature_challenge,
                replayed: true,
            };
            tx.commit()?;
            return Ok(output);
        }
        let checked_at = now();
        require_fresh_current_authority(&tx, &current.receipt, &checked_at)
            .map_err(StoreError::classify_write)?;
        if run_completed_at >= current.receipt.challenge.expires_at {
            return Err(StoreError::conflict(anyhow::anyhow!(
                "V268 challenge expired before durable observation commit"
            )));
        }
        let selected = &current.receipt.challenge;
        let material = ExternalPoolAdapterRuntimeCompatibilityServerRunObservationMaterial {
            runner_execution_id,
            challenge_id: selected.challenge_id.clone(),
            challenge_digest: current.receipt.challenge_digest.clone(),
            challenge_nonce_digest: selected.challenge_nonce_digest.clone(),
            registry_release: selected.registry_release.clone(),
            profile_id: selected.profile_id.clone(),
            profile_revision: selected.profile_revision,
            profile_digest: selected.profile_digest.clone(),
            runner_policy_digest: selected.runner_policy.policy_digest.clone(),
            fixture_catalog_digest: selected.fixture_catalog.policy_digest.clone(),
            source_capsule_sha256: evidence.source_capsule_sha256,
            source_capsule_size_bytes: evidence.source_capsule_size_bytes,
            source_capsule_policy_digest: selected.source_capsule_policy.policy_digest.clone(),
            launch_image_sha256: evidence.launch_image_sha256,
            launch_image_size_bytes: evidence.launch_image_size_bytes,
            public_fixture_delivery_root: evidence.public_fixture_delivery_root,
            run_started_at,
            run_completed_at: run_completed_at.clone(),
            recorded_at: run_completed_at,
            fixture_resources: selected.fixture_resources.clone(),
            observations: support::ordered_observations(evidence.duration_ms),
            no_work: evidence.no_work,
            child_network_attempt_count: 0,
            upstream_connect_attempt_count: 0,
            write_outside_ephemeral_count: 0,
            additional_process_attempt_count: 0,
            policy_violation_count: 0,
            observation_status: RUNTIME_COMPATIBILITY_VERIFICATION_OBSERVATION_STATUS.into(),
            effects: runtime_compatibility_no_effects(),
            readiness: runtime_compatibility_no_readiness(),
        };
        let prepared_observation =
            prepare_runtime_compatibility_server_run_observation(&current.receipt, material)
                .map_err(StoreError::storage)?;
        let receipt = build_runtime_compatibility_run_observation_receipt(
            new_id("external_pool_adapter_runtime_compatibility_run_observation"),
            prepared_observation,
        )
        .map_err(StoreError::storage)?;
        insert_run_observation(&tx, &receipt).map_err(StoreError::classify_write)?;
        let stored = run_observation_by_id_on(&tx, &receipt.run_observation_id)
            .map_err(StoreError::storage)?
            .ok_or_else(|| {
                StoreError::storage(anyhow::anyhow!("V268 observation disappeared after insert"))
            })?;
        if stored.receipt != receipt {
            return Err(StoreError::storage(anyhow::anyhow!(
                "V268 observation readback drifted"
            )));
        }
        let signature_challenge =
            runtime_compatibility_signature_challenge(&current.receipt, &stored.receipt)
                .map_err(StoreError::storage)?;
        let output = ExternalPoolAdapterRuntimeCompatibilityRunObservationWriteReceipt {
            run_observation: stored.receipt,
            signature_challenge,
            replayed: false,
        };
        tx.commit()?;
        Ok(output)
    }
}

pub(super) fn require_fresh_current_authority(
    conn: &rusqlite::Connection,
    challenge: &ExternalPoolAdapterRuntimeCompatibilityChallengeReceipt,
    checked_at: &str,
) -> Result<()> {
    let selected = &challenge.challenge;
    if checked_at >= selected.expires_at.as_str() {
        bail!("V268 challenge is expired");
    }
    current_external_pool_adapter_registry_release_authority_on(
        conn,
        &selected.registry_release.registry_release_id,
        &selected.registry_release.registry_release_digest,
        checked_at,
    )?
    .ok_or_else(|| anyhow::anyhow!("V268 challenge lost current V249 release"))?;
    let key = current_sandbox_verifier_key_authority_on(
        conn,
        &selected.sandbox_verifier_key_record_id,
        &selected.sandbox_verifier_key_record_digest,
        &selected.sandbox_verifier_key_id,
    )?
    .ok_or_else(|| anyhow::anyhow!("V268 challenge lost current V237 key"))?;
    if key.verifier_operator() != selected.sandbox_verifier_operator
        || key.verifier_product() != selected.sandbox_verifier_product
    {
        bail!("V268 challenge V237 operator/product roots drifted");
    }
    validate_runtime_compatibility_challenge_current_roots(selected)
}

fn now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true)
}

fn classify_runtime_file_audit_error(error: anyhow::Error) -> StoreError {
    if error.chain().any(|cause| {
        cause.downcast_ref::<std::io::Error>().is_some()
            || cause.downcast_ref::<std::num::TryFromIntError>().is_some()
    }) {
        StoreError::storage(error)
    } else {
        StoreError::conflict(error)
    }
}
