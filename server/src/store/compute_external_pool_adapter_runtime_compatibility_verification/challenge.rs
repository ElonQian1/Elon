use anyhow::{bail, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use chrono::{Duration, SecondsFormat, Utc};
use ring::rand::{SecureRandom, SystemRandom};
use rusqlite::{params, TransactionBehavior};
use sha2::{Digest, Sha256};

use crate::{
    compute_federation::{
        external_pool_adapter_artifact_package::ARTIFACT_PACKAGE_ENTRYPOINT_ROLE,
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
    persistence::insert_challenge,
    read::{
        challenge_by_id_on, challenge_by_idempotency_on, identifier,
        verification_head_by_release_on,
    },
    types::ExternalPoolAdapterRuntimeCompatibilityChallengeWriteReceipt,
};

impl Store {
    pub(crate) fn issue_external_pool_adapter_runtime_compatibility_verification_challenge(
        &self,
        admin_user_id: &str,
        input: CreateExternalPoolAdapterRuntimeCompatibilityChallengeInput,
    ) -> std::result::Result<ExternalPoolAdapterRuntimeCompatibilityChallengeWriteReceipt, StoreError>
    {
        identifier(admin_user_id).map_err(StoreError::conflict)?;
        validate_create_runtime_compatibility_challenge_input(&input)
            .map_err(StoreError::conflict)?;
        let scope = format!("v268:runtime-compatibility-challenge:{admin_user_id}");
        identifier(&scope).map_err(StoreError::conflict)?;
        let mut conn = self.conn().map_err(StoreError::storage)?;
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(StoreError::storage)?;
        let result = (|| -> Result<_> {
            if let Some(stored) = challenge_by_idempotency_on(&tx, &scope, &input.idempotency_key)?
            {
                ensure_replay(admin_user_id, &scope, &input, &stored.receipt)?;
                return Ok(
                    ExternalPoolAdapterRuntimeCompatibilityChallengeWriteReceipt {
                        challenge: stored.receipt,
                        replayed: true,
                    },
                );
            }
            require_current_admin(&tx, admin_user_id)?;
            let issued = Utc::now();
            let checked_at = issued.to_rfc3339_opts(SecondsFormat::Nanos, true);
            let release = current_external_pool_adapter_registry_release_authority_on(
                &tx,
                &input.registry_release_id,
                &input.expected_registry_release_digest,
                &checked_at,
            )?
            .ok_or_else(|| anyhow::anyhow!("current exact V249 registry release was not found"))?;
            let key = current_sandbox_verifier_key_authority_on(
                &tx,
                &input.sandbox_verifier_key_record_id,
                &input.expected_sandbox_verifier_key_record_digest,
                &input.expected_sandbox_verifier_key_id,
            )?
            .ok_or_else(|| anyhow::anyhow!("current exact V237 verifier key was not found"))?;
            let profile = server_runtime_compatibility_v2_profile_catalog()?;
            let (_, runner_digest) = server_runtime_compatibility_runner_policy_catalog()?;
            let (fixtures, fixture_digest) = server_runtime_compatibility_public_fixture_catalog()?;
            if input.expected_profile_digest != profile.profile_digest
                || input.expected_runner_policy_digest != runner_digest
                || input.expected_fixture_catalog_digest != fixture_digest
            {
                bail!("V268 expected profile, runner, or fixture root is not current");
            }
            let predecessor = verification_head_by_release_on(&tx, &input.registry_release_id)?;
            ensure_predecessor(&input, predecessor.as_ref())?;
            let sequence = predecessor.as_ref().map_or(Ok(1), |stored| {
                stored
                    .receipt
                    .verification
                    .sequence
                    .checked_add(1)
                    .ok_or_else(|| anyhow::anyhow!("V268 verification sequence overflow"))
            })?;
            let issued_at = checked_at;
            let expires_at = (issued
                + Duration::minutes(RUNTIME_COMPATIBILITY_VERIFICATION_CHALLENGE_VALIDITY_MINUTES))
            .to_rfc3339_opts(SecondsFormat::Nanos, true);
            let nonce = random_nonce()?;
            let release_receipt = release.release().clone();
            let (entrypoint_path, entrypoint_sha256, entrypoint_size_bytes) =
                entrypoint(&release_receipt)?;
            let fixture_resources = fixture_identities(&fixtures, &release_receipt)?;
            let material = ExternalPoolAdapterRuntimeCompatibilityChallengeMaterial {
                schema: RUNTIME_COMPATIBILITY_VERIFICATION_CHALLENGE_SCHEMA.into(),
                challenge_id: new_id("external_pool_adapter_runtime_compatibility_challenge"),
                challenge_nonce_base64: STANDARD.encode(nonce),
                challenge_nonce_digest: hex::encode(Sha256::digest(nonce)),
                issued_at: issued_at.clone(),
                expires_at,
                registry_release: release_receipt,
                runtime_kind: "server_sidecar_v1".into(),
                entrypoint_path,
                entrypoint_sha256,
                entrypoint_size_bytes,
                profile_id: profile.profile.profile_id,
                profile_revision: profile.profile.profile_revision,
                profile_digest: profile.profile_digest,
                runtime_launch_policy: profile.profile.runtime_launch_policy,
                upstream_transport_policy: profile.profile.upstream_transport_policy,
                supervisor_session_policy: profile.profile.supervisor_session_policy,
                source_capsule_policy: profile.profile.source_capsule_policy,
                runner_policy: profile.profile.runner_policy,
                fixture_catalog: profile.profile.fixture_catalog,
                fixture_resources,
                sandbox_verifier_key_record_id: key.key_record_id().into(),
                sandbox_verifier_key_record_digest: key.key_record_digest().into(),
                sandbox_verifier_key_id: key.key_id().into(),
                sandbox_verifier_operator: key.verifier_operator().into(),
                sandbox_verifier_product: key.verifier_product().into(),
                signature_algorithm: RUNTIME_COMPATIBILITY_VERIFICATION_SIGNATURE_ALGORITHM.into(),
                sequence,
                predecessor_verification_receipt_id: input
                    .predecessor_verification_receipt_id
                    .clone(),
                predecessor_verification_receipt_digest: input
                    .predecessor_verification_receipt_digest
                    .clone(),
                created_by_admin_user_id: admin_user_id.into(),
                confirmation: input.confirmation.clone(),
                idempotency_scope: scope.clone(),
                idempotency_key: input.idempotency_key.clone(),
                recorded_at: issued_at,
            };
            let receipt = build_runtime_compatibility_challenge_receipt(material)?;
            insert_challenge(&tx, &receipt)?;
            let stored = challenge_by_id_on(&tx, &receipt.challenge.challenge_id)?
                .ok_or_else(|| anyhow::anyhow!("V268 challenge disappeared after insert"))?;
            if stored.receipt != receipt {
                bail!("V268 challenge readback drifted");
            }
            Ok(
                ExternalPoolAdapterRuntimeCompatibilityChallengeWriteReceipt {
                    challenge: stored.receipt,
                    replayed: false,
                },
            )
        })()
        .map_err(StoreError::classify_write)?;
        tx.commit().map_err(StoreError::storage)?;
        Ok(result)
    }
}

fn ensure_replay(
    admin: &str,
    scope: &str,
    input: &CreateExternalPoolAdapterRuntimeCompatibilityChallengeInput,
    receipt: &ExternalPoolAdapterRuntimeCompatibilityChallengeReceipt,
) -> Result<()> {
    let c = &receipt.challenge;
    if c.registry_release.registry_release_id != input.registry_release_id
        || c.registry_release.registry_release_digest != input.expected_registry_release_digest
        || c.profile_digest != input.expected_profile_digest
        || c.runner_policy.policy_digest != input.expected_runner_policy_digest
        || c.fixture_catalog.policy_digest != input.expected_fixture_catalog_digest
        || c.sandbox_verifier_key_record_id != input.sandbox_verifier_key_record_id
        || c.sandbox_verifier_key_record_digest != input.expected_sandbox_verifier_key_record_digest
        || c.sandbox_verifier_key_id != input.expected_sandbox_verifier_key_id
        || c.predecessor_verification_receipt_id != input.predecessor_verification_receipt_id
        || c.predecessor_verification_receipt_digest
            != input.predecessor_verification_receipt_digest
        || c.created_by_admin_user_id != admin
        || c.idempotency_scope != scope
        || c.idempotency_key != input.idempotency_key
        || c.confirmation != input.confirmation
    {
        bail!("V268 challenge idempotency replay conflicts with sealed input");
    }
    Ok(())
}

fn ensure_predecessor(
    input: &CreateExternalPoolAdapterRuntimeCompatibilityChallengeInput,
    predecessor: Option<&super::types::StoredRuntimeCompatibilityVerification>,
) -> Result<()> {
    match (
        predecessor,
        input.predecessor_verification_receipt_id.as_deref(),
        input.predecessor_verification_receipt_digest.as_deref(),
    ) {
        (None, None, None) => Ok(()),
        (Some(stored), Some(id), Some(digest))
            if stored.receipt.verification_receipt_id == id
                && stored.receipt.verification_receipt_digest == digest =>
        {
            Ok(())
        }
        _ => bail!("V268 challenge predecessor is missing, stale, or unexpected"),
    }
}

fn entrypoint(
    release: &crate::compute_federation::external_pool_adapter_registry::ExternalPoolAdapterRegistryReleaseReceipt,
) -> Result<(String, String, u64)> {
    let matches: Vec<_> = release
        .release
        .manifest
        .files
        .iter()
        .filter(|file| file.role == ARTIFACT_PACKAGE_ENTRYPOINT_ROLE)
        .collect();
    if matches.len() != 1 {
        bail!("V268 release does not have one exact entrypoint");
    }
    Ok((
        matches[0].path.clone(),
        matches[0].sha256.clone(),
        matches[0].size_bytes,
    ))
}

fn fixture_identities(
    catalog: &ExternalPoolAdapterRuntimeCompatibilityPublicFixtureCatalog,
    release: &crate::compute_federation::external_pool_adapter_registry::ExternalPoolAdapterRegistryReleaseReceipt,
) -> Result<Vec<ExternalPoolAdapterRuntimeCompatibilityFixtureResourceIdentity>> {
    catalog
        .resources
        .iter()
        .map(|requirement| {
            let matches: Vec<_> = release
                .release
                .manifest
                .files
                .iter()
                .filter(|file| file.path == requirement.path)
                .collect();
            if matches.len() != 1 {
                bail!("V268 release is missing an exact controlled public fixture");
            }
            let file = matches[0];
            Ok(
                ExternalPoolAdapterRuntimeCompatibilityFixtureResourceIdentity {
                    purpose: requirement.purpose.clone(),
                    path: file.path.clone(),
                    role: file.role.clone(),
                    sha256: file.sha256.clone(),
                    size_bytes: file.size_bytes,
                },
            )
        })
        .collect()
}

fn require_current_admin(conn: &rusqlite::Connection, admin: &str) -> Result<()> {
    let current: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM users WHERE id=?1 AND role IN ('admin','owner') AND status='active')",
        params![admin],
        |row| row.get(0),
    )?;
    if !current {
        bail!("V268 actor is not a current administrator");
    }
    Ok(())
}

fn random_nonce() -> Result<[u8; 32]> {
    let mut nonce = [0_u8; 32];
    SystemRandom::new()
        .fill(&mut nonce)
        .map_err(|_| anyhow::anyhow!("V268 challenge nonce generation failed"))?;
    Ok(nonce)
}
