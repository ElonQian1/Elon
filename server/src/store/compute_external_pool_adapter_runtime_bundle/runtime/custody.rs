//! Locked HMAC custody and epoch-local durable receipt seals.

mod atomic_activation_plan;
mod provider_active_successor;
mod support;
mod task_protocol_conformance;

use std::{collections::HashMap, net::SocketAddr, sync::Mutex};

use anyhow::{anyhow, bail, Result};
use chrono::{DateTime, Duration as ChronoDuration, SecondsFormat, Utc};

use support::{
    constant_time_equal, is_lower_hex_sha256, keyed_commitment, update_field,
    update_socket_address, update_u64, HmacSha256,
};

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
use super::super::secret_delivery::{
    CleanedExternalPoolAdapterEphemeralSecretDeliveryAuthority,
    ExternalPoolAdapterEphemeralSecretDeliveryBinding,
};
use super::super::{
    locked_bytes::LockedSensitiveBytes, types::CurrentExternalPoolAdapterRuntimeBundleAuthority,
};
use crate::compute_federation::external_pool_adapter_provider_runtime_readiness::{
    PROVIDER_RUNTIME_READINESS_BUNDLE_IDENTITY_COMMITMENT_DOMAIN,
    PROVIDER_RUNTIME_READINESS_CUSTODY_EPOCH_BYTES,
    PROVIDER_RUNTIME_READINESS_CUSTODY_EPOCH_DIGEST_DOMAIN,
    PROVIDER_RUNTIME_READINESS_HMAC_KEY_BYTES,
    PROVIDER_RUNTIME_READINESS_POST_CLEANUP_COMMITMENT_DOMAIN,
};
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
use elon_external_pool_adapter_session_core::ExternalPoolAdapterNoWorkProbeHostReceipt;

const MAX_READINESS_SEAL_TTL_MS: i64 = 15_000;
const MAX_LIVE_READINESS_SEALS: usize = 4_096;
pub(crate) use atomic_activation_plan::register_external_pool_adapter_atomic_activation_pending_plan_udf;
pub(in crate::store) use atomic_activation_plan::{
    install_external_pool_adapter_atomic_activation_pending_plan_on,
    ExternalPoolAdapterAtomicActivationPendingPlan,
    ExternalPoolAdapterAtomicActivationPendingPlanGuard,
    ExternalPoolAdapterAtomicActivationPendingWrite,
    ExternalPoolAdapterAtomicActivationPendingWriteKind,
};
pub(in crate::store) use provider_active_successor::{
    ExternalPoolAdapterProviderActiveSuccessorProcessSeal,
    ExternalPoolAdapterProviderActiveSuccessorProcessSealInput,
};
pub(in crate::store) use task_protocol_conformance::{
    ExternalPoolAdapterTaskProtocolConformanceSealInput, TaskProtocolConformanceProcessSeal,
};
/// Locked process secrets plus the only permitted purpose-specific commitment operations.
pub(in crate::store) struct ExternalPoolAdapterProviderRuntimeReadinessProcessCustody {
    secrets: Mutex<LockedProviderRuntimeReadinessSecrets>,
    readiness_seals: Mutex<ProviderRuntimeReadinessSealRegistry>,
    task_protocol_conformance_seals:
        Mutex<task_protocol_conformance::TaskProtocolConformanceSealRegistry>,
    provider_active_successor_seals:
        Mutex<provider_active_successor::ProviderActiveSuccessorSealRegistry>,
    custody_epoch_digest: String,
}

struct LockedProviderRuntimeReadinessSecrets {
    hmac_key: LockedSensitiveBytes,
    custody_epoch: LockedSensitiveBytes,
}

#[derive(Default)]
struct ProviderRuntimeReadinessSealRegistry {
    by_receipt_id: HashMap<String, ProviderRuntimeReadinessSeal>,
}

struct ProviderRuntimeReadinessSeal {
    receipt_digest: String,
    runtime_bundle_identity_commitment: String,
    post_cleanup_observation_commitment: String,
    expires_at: String,
    expires_at_utc: DateTime<Utc>,
    committed: bool,
}

// SAFETY: the exclusively owned allocations have no creator-thread affinity; every shared access
// is Mutex-serialized, granting Send only (never Sync) to the raw secret owner.
unsafe impl Send for LockedProviderRuntimeReadinessSecrets {}
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub(in crate::store) struct ExternalPoolAdapterPostCleanupCommitmentInput<'a> {
    pub(in crate::store) runtime_bundle_identity_commitment: &'a str,
    pub(in crate::store) receipt: &'a ExternalPoolAdapterNoWorkProbeHostReceipt,
    pub(in crate::store) binding: &'a ExternalPoolAdapterEphemeralSecretDeliveryBinding,
    pub(in crate::store) selected_address: SocketAddr,
    pub(in crate::store) cleaned: &'a CleanedExternalPoolAdapterEphemeralSecretDeliveryAuthority,
}

impl ExternalPoolAdapterProviderRuntimeReadinessProcessCustody {
    pub(in crate::store) fn generate() -> Result<Self> {
        Self::generate_for_epoch_domain(
            PROVIDER_RUNTIME_READINESS_CUSTODY_EPOCH_DIGEST_DOMAIN,
            "Provider readiness",
        )
    }

    fn generate_for_epoch_domain(epoch_domain: &[u8], authority: &str) -> Result<Self> {
        let hmac_key = LockedSensitiveBytes::random(PROVIDER_RUNTIME_READINESS_HMAC_KEY_BYTES)
            .map_err(|_| anyhow!("locked {authority} HMAC key is unavailable"))?;
        let custody_epoch =
            LockedSensitiveBytes::random(PROVIDER_RUNTIME_READINESS_CUSTODY_EPOCH_BYTES)
                .map_err(|_| anyhow!("locked {authority} custody epoch is unavailable"))?;
        let custody_epoch_digest = keyed_commitment(
            hmac_key.as_slice(),
            epoch_domain,
            custody_epoch.as_slice(),
            |_| {},
        )?;
        Ok(Self {
            secrets: Mutex::new(LockedProviderRuntimeReadinessSecrets {
                hmac_key,
                custody_epoch,
            }),
            readiness_seals: Mutex::new(ProviderRuntimeReadinessSealRegistry::default()),
            task_protocol_conformance_seals: Mutex::new(
                task_protocol_conformance::TaskProtocolConformanceSealRegistry::default(),
            ),
            provider_active_successor_seals: Mutex::new(
                provider_active_successor::ProviderActiveSuccessorSealRegistry::default(),
            ),
            custody_epoch_digest,
        })
    }

    pub(in crate::store) fn custody_epoch_digest(&self) -> &str {
        &self.custody_epoch_digest
    }

    pub(in crate::store) fn attests_custody_epoch_digest(&self, expected: &str) -> bool {
        is_lower_hex_sha256(expected) && constant_time_equal(&self.custody_epoch_digest, expected)
    }

    pub(in crate::store) fn runtime_bundle_identity_commitment(
        &self,
        bundle: &CurrentExternalPoolAdapterRuntimeBundleAuthority<'_, '_>,
    ) -> Result<String> {
        let roots = bundle.roots();
        let profile_receipt = bundle.launch_profile().profile();
        let profile = &profile_receipt.profile;
        let credential_receipt = bundle.credential().receipt();
        self.with_commitment(
            PROVIDER_RUNTIME_READINESS_BUNDLE_IDENTITY_COMMITMENT_DOMAIN,
            |mac| {
                update_u64(mac, roots.bundle_generation());
                update_u64(mac, roots.config_size_bytes());
                update_field(mac, roots.config_sha256());
                update_u64(mac, roots.credential_size_bytes());
                update_field(mac, roots.credential_sha256());
                update_field(mac, profile_receipt.profile_id.as_bytes());
                update_field(mac, profile_receipt.profile_digest.as_bytes());
                update_field(mac, profile.provider_binding_id.as_bytes());
                update_field(mac, profile.provider_binding_digest.as_bytes());
                update_field(mac, credential_receipt.reattestation_receipt_id.as_bytes());
                update_field(
                    mac,
                    credential_receipt.reattestation_receipt_digest.as_bytes(),
                );
            },
        )
    }

    pub(in crate::store) fn attests_runtime_bundle_identity_commitment(
        &self,
        bundle: &CurrentExternalPoolAdapterRuntimeBundleAuthority<'_, '_>,
        expected: &str,
    ) -> Result<bool> {
        if !is_lower_hex_sha256(expected) {
            return Ok(false);
        }
        let observed = self.runtime_bundle_identity_commitment(bundle)?;
        Ok(constant_time_equal(&observed, expected))
    }

    /// Remembers only a pending exact receipt seal for the life of this process custody epoch.
    /// A failed transaction may leave a harmless pending seal without a row; it expires within 15
    /// seconds and cannot attest until an exact post-commit promotion occurs.
    pub(in crate::store) fn remember_readiness_seal(
        &self,
        readiness_receipt_id: &str,
        readiness_receipt_digest: &str,
        runtime_bundle_identity_commitment: &str,
        post_cleanup_observation_commitment: &str,
        expires_at: &str,
    ) -> Result<()> {
        let now = Utc::now();
        let expires_at_utc = validate_readiness_seal_material(
            readiness_receipt_id,
            readiness_receipt_digest,
            runtime_bundle_identity_commitment,
            post_cleanup_observation_commitment,
            expires_at,
            now.clone(),
        )?;
        let mut registry = self
            .readiness_seals
            .lock()
            .map_err(|_| anyhow!("Provider readiness seal registry lock was poisoned"))?;
        registry.prune(now);
        if let Some(existing) = registry.by_receipt_id.get(readiness_receipt_id) {
            if existing.matches(
                readiness_receipt_digest,
                runtime_bundle_identity_commitment,
                post_cleanup_observation_commitment,
                expires_at,
            ) {
                return Ok(());
            }
            bail!("Provider readiness receipt identity already has a different process seal");
        }
        if registry.by_receipt_id.len() >= MAX_LIVE_READINESS_SEALS {
            bail!("Provider readiness process seal registry reached its fixed bound");
        }
        registry.by_receipt_id.insert(
            readiness_receipt_id.to_owned(),
            ProviderRuntimeReadinessSeal {
                receipt_digest: readiness_receipt_digest.to_owned(),
                runtime_bundle_identity_commitment: runtime_bundle_identity_commitment.to_owned(),
                post_cleanup_observation_commitment: post_cleanup_observation_commitment.to_owned(),
                expires_at: expires_at.to_owned(),
                expires_at_utc,
                committed: false,
            },
        );
        Ok(())
    }

    /// Promotes only an exact pending seal after its enclosing final IMMEDIATE transaction has
    /// committed. Absence never creates authority; exact already-committed state is idempotent.
    pub(in crate::store) fn commit_readiness_seal(
        &self,
        readiness_receipt_id: &str,
        readiness_receipt_digest: &str,
    ) -> Result<bool> {
        if readiness_receipt_id.is_empty()
            || readiness_receipt_id.len() > 512
            || !is_lower_hex_sha256(readiness_receipt_digest)
        {
            return Ok(false);
        }
        let now = Utc::now();
        let mut registry = self
            .readiness_seals
            .lock()
            .map_err(|_| anyhow!("Provider readiness seal registry lock was poisoned"))?;
        registry.prune(now);
        let Some(seal) = registry.by_receipt_id.get_mut(readiness_receipt_id) else {
            return Ok(false);
        };
        if !constant_time_equal(&seal.receipt_digest, readiness_receipt_digest) {
            return Ok(false);
        }
        seal.committed = true;
        Ok(true)
    }

    /// Attests only an exact seal minted in this process. Database contents alone cannot recreate
    /// this authority, and restart drops the registry together with the HMAC key and epoch.
    pub(in crate::store) fn attests_readiness_seal(
        &self,
        readiness_receipt_id: &str,
        readiness_receipt_digest: &str,
        runtime_bundle_identity_commitment: &str,
        post_cleanup_observation_commitment: &str,
        expires_at: &str,
    ) -> Result<bool> {
        let now = Utc::now();
        if validate_readiness_seal_material(
            readiness_receipt_id,
            readiness_receipt_digest,
            runtime_bundle_identity_commitment,
            post_cleanup_observation_commitment,
            expires_at,
            now.clone(),
        )
        .is_err()
        {
            return Ok(false);
        }
        let mut registry = self
            .readiness_seals
            .lock()
            .map_err(|_| anyhow!("Provider readiness seal registry lock was poisoned"))?;
        registry.prune(now);
        Ok(registry
            .by_receipt_id
            .get(readiness_receipt_id)
            .is_some_and(|seal| {
                seal.committed
                    && seal.matches(
                        readiness_receipt_digest,
                        runtime_bundle_identity_commitment,
                        post_cleanup_observation_commitment,
                        expires_at,
                    )
            }))
    }

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    pub(in crate::store) fn post_cleanup_observation_commitment(
        &self,
        input: &ExternalPoolAdapterPostCleanupCommitmentInput<'_>,
    ) -> Result<String> {
        if !input.cleaned.authenticated_shutdown_completed()
            || !input.cleaned.pidfd_reaped()
            || !input.cleaned.cgroup_cleaned()
            || !input.cleaned.scratch_cleaned()
        {
            bail!("post-cleanup commitment lacks terminal cleanup authority");
        }
        let nonce = input.receipt.probe_nonce_digest_hex();
        let root = input.receipt.probe_root_hex();
        let request = input.receipt.request_sha256_hex();
        let response = input.receipt.response_sha256_hex();
        self.with_commitment(
            PROVIDER_RUNTIME_READINESS_POST_CLEANUP_COMMITMENT_DOMAIN,
            |mac| {
                update_field(mac, input.runtime_bundle_identity_commitment.as_bytes());
                update_field(mac, nonce.as_bytes());
                update_field(mac, root.as_bytes());
                update_field(mac, request.as_bytes());
                update_field(mac, response.as_bytes());
                update_u64(mac, u64::from(input.receipt.request_bytes()));
                update_u64(mac, u64::from(input.receipt.response_bytes()));
                update_socket_address(mac, input.selected_address);
                update_field(mac, input.binding.target_digest().as_bytes());
                update_field(mac, input.binding.delivery_root().as_bytes());
                update_field(mac, input.binding.source_capsule_digest().as_bytes());
                update_field(mac, input.binding.launch_capsule_digest().as_bytes());
                update_field(mac, input.cleaned.delivery_checked_at().as_bytes());
                update_field(mac, b"authenticated_shutdown_completed");
                update_field(mac, b"pidfd_reaped");
                update_field(mac, b"cgroup_cleaned");
                update_field(mac, b"scratch_cleaned");
            },
        )
    }

    fn with_commitment(
        &self,
        domain: &[u8],
        update: impl FnOnce(&mut HmacSha256),
    ) -> Result<String> {
        let secrets = self
            .secrets
            .lock()
            .map_err(|_| anyhow!("Provider readiness process custody lock was poisoned"))?;
        keyed_commitment(
            secrets.hmac_key.as_slice(),
            domain,
            secrets.custody_epoch.as_slice(),
            update,
        )
    }
}

impl ProviderRuntimeReadinessSealRegistry {
    fn prune(&mut self, now: DateTime<Utc>) {
        self.by_receipt_id
            .retain(|_, seal| seal.expires_at_utc > now);
    }
}

impl ProviderRuntimeReadinessSeal {
    fn matches(
        &self,
        receipt_digest: &str,
        runtime_bundle_identity_commitment: &str,
        post_cleanup_observation_commitment: &str,
        expires_at: &str,
    ) -> bool {
        let receipt_matches = constant_time_equal(&self.receipt_digest, receipt_digest);
        let bundle_matches = constant_time_equal(
            &self.runtime_bundle_identity_commitment,
            runtime_bundle_identity_commitment,
        );
        let observation_matches = constant_time_equal(
            &self.post_cleanup_observation_commitment,
            post_cleanup_observation_commitment,
        );
        receipt_matches & bundle_matches & observation_matches & (self.expires_at == expires_at)
    }
}

fn validate_readiness_seal_material(
    readiness_receipt_id: &str,
    readiness_receipt_digest: &str,
    runtime_bundle_identity_commitment: &str,
    post_cleanup_observation_commitment: &str,
    expires_at: &str,
    now: DateTime<Utc>,
) -> Result<DateTime<Utc>> {
    if readiness_receipt_id.is_empty()
        || readiness_receipt_id.len() > 512
        || !is_lower_hex_sha256(readiness_receipt_digest)
        || !is_lower_hex_sha256(runtime_bundle_identity_commitment)
        || !is_lower_hex_sha256(post_cleanup_observation_commitment)
    {
        bail!("Provider readiness process seal material is invalid");
    }
    let parsed = DateTime::parse_from_rfc3339(expires_at)?;
    if parsed.offset().local_minus_utc() != 0
        || parsed.to_rfc3339_opts(SecondsFormat::Nanos, true) != expires_at
    {
        bail!("Provider readiness process seal expiry is not canonical UTC nanoseconds");
    }
    let expires_at_utc = parsed.with_timezone(&Utc);
    if expires_at_utc <= now
        || expires_at_utc > now + ChronoDuration::milliseconds(MAX_READINESS_SEAL_TTL_MS)
    {
        bail!("Provider readiness process seal expiry is outside its fixed live window");
    }
    Ok(expires_at_utc)
}
#[cfg(test)]
#[path = "custody_tests.rs"]
mod tests;
