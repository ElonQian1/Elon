use crate::store::{
    compute_external_pool_adapter_credential_reattestation::CurrentExternalPoolAdapterCredentialReattestationAuthority,
    compute_external_pool_adapter_runtime_launch_profile::CurrentExternalPoolAdapterRuntimeLaunchProfileAuthority,
    compute_external_pool_adapter_sandbox_reattestation::CurrentExternalPoolAdapterSandboxReattestationAuthority,
    compute_external_pool_adapter_vulnerability_reattestation::CurrentExternalPoolAdapterVulnerabilityReattestationAuthority,
};
use anyhow::Result;
use rusqlite::Transaction;
use sha2::{Digest, Sha256};
#[cfg(target_os = "linux")]
use std::fs::File;
#[cfg(windows)]
use std::path::Path;
use std::{marker::PhantomData, path::PathBuf};
use zeroize::Zeroize;

use super::entrypoint_capsule::{
    PreparedExternalPoolAdapterEntrypointCapsule, ACTIVATION_READY, ENTRYPOINT_CAPSULE_EFFECT,
    PROBE_OBSERVED, RUNTIME_LAUNCH_READY as CAPSULE_RUNTIME_LAUNCH_READY,
};
use super::locked_bytes::LockedSensitiveBytes;

pub(super) const RUNTIME_BUNDLE_EFFECT: &str = "resolved_ephemeral";
pub(super) const CONFIG_ACCESS_EFFECT: &str = "memory_only";
pub(super) const SECRET_ACCESS_EFFECT: &str = "memory_only";
pub(super) const RUNTIME_LAUNCH_READY: bool = false;

#[derive(Clone)]
pub(super) struct ExpectedExternalPoolAdapterRuntimeBundle {
    pub(super) profile_id: String,
    pub(super) profile_digest: String,
    pub(super) launch_policy_digest: String,
    pub(super) candidate_id: String,
    pub(super) candidate_digest: String,
    pub(super) provider_binding_id: String,
    pub(super) provider_binding_digest: String,
    pub(super) provider_id: String,
    pub(super) provider_owner_account_id: String,
    pub(super) logical_adapter_id: String,
    pub(super) release_version: String,
    pub(super) adapter_config_revision: i64,
    pub(super) adapter_config_digest: String,
    pub(super) credential_locator_commitment: String,
    pub(super) credential_reattestation_receipt_id: String,
    pub(super) credential_reattestation_receipt_digest: String,
    pub(super) credential_reattestation_material_digest: String,
    pub(super) credential_report_expires_at: String,
}

/// Startup-retained operator custody root.
///
/// Linux keeps the no-follow directory handle, so a later rename or path replacement cannot
/// redirect a bundle resolution. Windows retains the path until its protected-DACL implementation
/// becomes available. The type deliberately exposes neither representation.
pub(in crate::store) struct ExternalPoolAdapterRuntimeBundleRoot {
    #[cfg(target_os = "linux")]
    retained_directory: File,
    #[cfg(windows)]
    path: PathBuf,
    #[cfg(not(any(target_os = "linux", windows)))]
    unavailable: (),
}

impl ExternalPoolAdapterRuntimeBundleRoot {
    pub(in crate::store) fn new(
        path: PathBuf,
    ) -> std::result::Result<Self, ExternalPoolAdapterRuntimeBundleError> {
        if !path.is_absolute() {
            return Err(ExternalPoolAdapterRuntimeBundleError::UnsafeCustody);
        }

        #[cfg(target_os = "linux")]
        {
            let retained_directory =
                super::filesystem::open_external_pool_adapter_runtime_bundle_root(&path)?;
            return Ok(Self { retained_directory });
        }

        #[cfg(windows)]
        {
            return Ok(Self { path });
        }

        #[cfg(not(any(target_os = "linux", windows)))]
        {
            let _ = path;
            Err(ExternalPoolAdapterRuntimeBundleError::Unavailable)
        }
    }

    #[cfg(target_os = "linux")]
    pub(super) fn retained_directory(&self) -> &File {
        &self.retained_directory
    }

    #[cfg(windows)]
    pub(super) fn as_path(&self) -> &Path {
        &self.path
    }
}

pub(super) struct PreparedExternalPoolAdapterRuntimeBundle {
    pub(super) config: LockedSensitiveBytes,
    pub(super) credential: LockedSensitiveBytes,
    pub(super) bundle_generation: u64,
    pub(super) config_sha256: [u8; 32],
    pub(super) config_size_bytes: u64,
    pub(super) credential_sha256: [u8; 32],
    pub(super) credential_size_bytes: u64,
    pub(super) retained_bundle: Box<dyn super::filesystem::OpenedRuntimeBundle>,
}

impl PreparedExternalPoolAdapterRuntimeBundle {
    pub(super) fn with_sensitive_bytes(
        &self,
        consume: impl FnOnce(&[u8], &[u8]) -> Result<()>,
    ) -> Result<()> {
        let _retained_filesystem_authority = &self.retained_bundle;
        consume(self.config.as_slice(), self.credential.as_slice())
    }

    pub(super) fn roots(&self) -> ExternalPoolAdapterRuntimeBundleRoots<'_> {
        ExternalPoolAdapterRuntimeBundleRoots {
            bundle_generation: self.bundle_generation,
            config_sha256: &self.config_sha256,
            config_size_bytes: self.config_size_bytes,
            credential_sha256: &self.credential_sha256,
            credential_size_bytes: self.credential_size_bytes,
        }
    }

    pub(super) fn revalidate(&self) -> Result<()> {
        self.retained_bundle
            .revalidate()
            .map_err(|_| anyhow::anyhow!("runtime bundle retained authority is no longer exact"))?;
        let mut config_sha256: [u8; 32] = Sha256::digest(self.config.as_slice()).into();
        let mut credential_sha256: [u8; 32] = Sha256::digest(self.credential.as_slice()).into();
        let exact = self.config.as_slice().len() as u64 == self.config_size_bytes
            && self.credential.as_slice().len() as u64 == self.credential_size_bytes
            && config_sha256 == self.config_sha256
            && credential_sha256 == self.credential_sha256;
        config_sha256.zeroize();
        credential_sha256.zeroize();
        if !exact {
            anyhow::bail!("runtime bundle locked content roots are no longer exact");
        }
        Ok(())
    }
}

impl Drop for PreparedExternalPoolAdapterRuntimeBundle {
    fn drop(&mut self) {
        self.bundle_generation.zeroize();
        self.config_sha256.zeroize();
        self.config_size_bytes.zeroize();
        self.credential_sha256.zeroize();
        self.credential_size_bytes.zeroize();
    }
}

/// Borrowed binary roots for one Store-owned preparation callback.
///
/// It is intentionally neither Clone, Debug, nor serializable.
pub(super) struct ExternalPoolAdapterRuntimeBundleRoots<'a> {
    bundle_generation: u64,
    config_sha256: &'a [u8; 32],
    config_size_bytes: u64,
    credential_sha256: &'a [u8; 32],
    credential_size_bytes: u64,
}

impl ExternalPoolAdapterRuntimeBundleRoots<'_> {
    pub(super) fn bundle_generation(&self) -> u64 {
        self.bundle_generation
    }

    pub(super) fn config_sha256(&self) -> &[u8; 32] {
        self.config_sha256
    }

    pub(super) fn config_size_bytes(&self) -> u64 {
        self.config_size_bytes
    }

    pub(super) fn credential_sha256(&self) -> &[u8; 32] {
        self.credential_sha256
    }

    pub(super) fn credential_size_bytes(&self) -> u64 {
        self.credential_size_bytes
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ExternalPoolAdapterRuntimeBundleError {
    InvalidAuthority,
    Unavailable,
    UnsafeCustody,
    ContentDrift,
    MemoryCustodyUnavailable,
}

/// Sealed, Store-only snapshot. It is intentionally neither Clone, Debug, nor serializable.
pub(in crate::store) struct CurrentExternalPoolAdapterRuntimeBundleAuthority<'tx, 'conn> {
    launch_profile: CurrentExternalPoolAdapterRuntimeLaunchProfileAuthority,
    credential: CurrentExternalPoolAdapterCredentialReattestationAuthority,
    bundle: PreparedExternalPoolAdapterRuntimeBundle,
    checked_at: String,
    transaction: PhantomData<&'tx Transaction<'conn>>,
}

impl<'tx, 'conn> CurrentExternalPoolAdapterRuntimeBundleAuthority<'tx, 'conn> {
    pub(super) fn new(
        _transaction: &'tx Transaction<'conn>,
        launch_profile: CurrentExternalPoolAdapterRuntimeLaunchProfileAuthority,
        credential: CurrentExternalPoolAdapterCredentialReattestationAuthority,
        bundle: PreparedExternalPoolAdapterRuntimeBundle,
        checked_at: String,
    ) -> Self {
        Self {
            launch_profile,
            credential,
            bundle,
            checked_at,
            transaction: PhantomData,
        }
    }

    #[allow(dead_code)]
    pub(in crate::store) fn launch_profile(
        &self,
    ) -> &CurrentExternalPoolAdapterRuntimeLaunchProfileAuthority {
        &self.launch_profile
    }

    #[allow(dead_code)]
    pub(in crate::store) fn credential(
        &self,
    ) -> &CurrentExternalPoolAdapterCredentialReattestationAuthority {
        &self.credential
    }

    /// Keeps sensitive slices borrowed only for the duration of one Store-owned callback.
    #[allow(dead_code)]
    pub(in crate::store) fn with_sensitive_bytes(
        &self,
        consume: impl FnOnce(&[u8], &[u8]) -> Result<()>,
    ) -> Result<()> {
        self.bundle.with_sensitive_bytes(consume)
    }

    /// Borrows non-secret binary content roots without releasing bundle custody.
    #[allow(dead_code)]
    pub(super) fn roots(&self) -> ExternalPoolAdapterRuntimeBundleRoots<'_> {
        self.bundle.roots()
    }

    /// Rechecks the retained directory and file identities before a side effect.
    #[allow(dead_code)]
    pub(super) fn revalidate(&self) -> Result<()> {
        self.bundle.revalidate()
    }

    #[allow(dead_code)]
    pub(in crate::store) fn checked_at(&self) -> &str {
        &self.checked_at
    }

    /// Removes every database/installation authority while retaining only the already-resolved
    /// locked V256 material and its filesystem handles.
    pub(super) fn into_prepared_bundle(self) -> PreparedExternalPoolAdapterRuntimeBundle {
        self.bundle
    }
}

/// Store-only borrowed view over one exact pre-probe preparation.
///
/// It is intentionally neither Clone, Debug, nor serializable and exposes no receipt roots,
/// transaction, secret bytes, content hashes, path, or raw descriptor.
pub(in crate::store) struct CurrentExternalPoolAdapterProbePreparationAuthority<'a, 'tx, 'conn> {
    capsule: &'a PreparedExternalPoolAdapterEntrypointCapsule,
    vulnerability: &'a CurrentExternalPoolAdapterVulnerabilityReattestationAuthority,
    sandbox: &'a CurrentExternalPoolAdapterSandboxReattestationAuthority,
    bundle: &'a CurrentExternalPoolAdapterRuntimeBundleAuthority<'tx, 'conn>,
    policy_id: &'static str,
    policy_revision: u64,
    policy_digest: &'a str,
}

impl<'a, 'tx, 'conn> CurrentExternalPoolAdapterProbePreparationAuthority<'a, 'tx, 'conn> {
    pub(super) fn new(
        capsule: &'a PreparedExternalPoolAdapterEntrypointCapsule,
        vulnerability: &'a CurrentExternalPoolAdapterVulnerabilityReattestationAuthority,
        sandbox: &'a CurrentExternalPoolAdapterSandboxReattestationAuthority,
        bundle: &'a CurrentExternalPoolAdapterRuntimeBundleAuthority<'tx, 'conn>,
        policy_id: &'static str,
        policy_revision: u64,
        policy_digest: &'a str,
    ) -> Self {
        Self {
            capsule,
            vulnerability,
            sandbox,
            bundle,
            policy_id,
            policy_revision,
            policy_digest,
        }
    }

    pub(in crate::store) fn preparation_effect(&self) -> &'static str {
        self.retain_complete_authority();
        ENTRYPOINT_CAPSULE_EFFECT
    }

    fn retain_complete_authority(&self) {
        let _retained_authorities = (
            &self.vulnerability,
            &self.sandbox,
            &self.bundle,
            &self.capsule,
            self.policy_id,
            self.policy_revision,
            self.policy_digest,
        );
    }

    pub(in crate::store) fn probe_observed(&self) -> bool {
        self.retain_complete_authority();
        PROBE_OBSERVED
    }

    pub(in crate::store) fn runtime_launch_ready(&self) -> bool {
        self.retain_complete_authority();
        CAPSULE_RUNTIME_LAUNCH_READY
    }

    pub(in crate::store) fn activation_ready(&self) -> bool {
        self.retain_complete_authority();
        ACTIVATION_READY
    }

    pub(super) fn capsule(&self) -> &PreparedExternalPoolAdapterEntrypointCapsule {
        self.capsule
    }

    pub(super) fn bundle(&self) -> &CurrentExternalPoolAdapterRuntimeBundleAuthority<'tx, 'conn> {
        self.bundle
    }

    pub(in crate::store) fn vulnerability(
        &self,
    ) -> &CurrentExternalPoolAdapterVulnerabilityReattestationAuthority {
        self.vulnerability
    }

    pub(in crate::store) fn sandbox(
        &self,
    ) -> &CurrentExternalPoolAdapterSandboxReattestationAuthority {
        self.sandbox
    }
}
