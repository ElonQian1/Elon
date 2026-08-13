use crate::store::{
    compute_external_pool_adapter_credential_reattestation::CurrentExternalPoolAdapterCredentialReattestationAuthority,
    compute_external_pool_adapter_runtime_launch_profile::CurrentExternalPoolAdapterRuntimeLaunchProfileAuthority,
};
use anyhow::Result;
use rusqlite::Transaction;
use std::{
    marker::PhantomData,
    path::{Path, PathBuf},
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

pub(in crate::store) struct ExternalPoolAdapterRuntimeBundleRoot(PathBuf);

impl ExternalPoolAdapterRuntimeBundleRoot {
    pub(in crate::store) fn new(
        path: PathBuf,
    ) -> std::result::Result<Self, ExternalPoolAdapterRuntimeBundleError> {
        if !path.is_absolute() {
            return Err(ExternalPoolAdapterRuntimeBundleError::UnsafeCustody);
        }
        Ok(Self(path))
    }

    pub(super) fn as_path(&self) -> &Path {
        &self.0
    }
}

pub(super) struct PreparedExternalPoolAdapterRuntimeBundle {
    pub(super) config: LockedSensitiveBytes,
    pub(super) credential: LockedSensitiveBytes,
    pub(super) retained_handles: Vec<std::fs::File>,
}

impl PreparedExternalPoolAdapterRuntimeBundle {
    pub(super) fn with_sensitive_bytes(
        &self,
        consume: impl FnOnce(&[u8], &[u8]) -> Result<()>,
    ) -> Result<()> {
        let _retained_filesystem_handles = &self.retained_handles;
        consume(self.config.as_slice(), self.credential.as_slice())
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

    #[allow(dead_code)]
    pub(in crate::store) fn checked_at(&self) -> &str {
        &self.checked_at
    }
}
