use sha2::{Digest, Sha256};
use zeroize::Zeroize;

use super::{
    locked_bytes::LockedSensitiveBytes,
    manifest::{parse_and_validate_manifest, MAX_MANIFEST_BYTES},
    types::{
        ExpectedExternalPoolAdapterRuntimeBundle, ExternalPoolAdapterRuntimeBundleError,
        ExternalPoolAdapterRuntimeBundleRoot, PreparedExternalPoolAdapterRuntimeBundle,
    },
};

#[cfg(target_os = "linux")]
#[path = "filesystem/linux.rs"]
mod linux;
#[cfg(windows)]
#[path = "filesystem/windows.rs"]
mod windows;

const MANIFEST_FILE: &str = "manifest.jcs";
const CONFIG_FILE: &str = "config.bin";
const CREDENTIAL_FILE: &str = "credential.bin";

pub(super) fn resolve_external_pool_adapter_runtime_bundle(
    root: &ExternalPoolAdapterRuntimeBundleRoot,
    expected: &ExpectedExternalPoolAdapterRuntimeBundle,
) -> Result<PreparedExternalPoolAdapterRuntimeBundle, ExternalPoolAdapterRuntimeBundleError> {
    validate_profile_digest(&expected.profile_digest)?;
    let mut opened = platform_open_bundle(root, &expected.profile_digest)?;
    let manifest_bytes = opened.read_manifest(MAX_MANIFEST_BYTES as u64)?;
    let manifest = parse_and_validate_manifest(manifest_bytes.as_slice(), expected)?;
    let config = opened.read_sensitive(CONFIG_FILE, manifest.config_size_bytes)?;
    let config_sha256 = exact_sha256(config.as_slice(), &manifest.config_sha256)?;
    let credential = opened.read_sensitive(CREDENTIAL_FILE, manifest.credential_size_bytes)?;
    let credential_sha256 = exact_sha256(credential.as_slice(), &manifest.credential_sha256)?;
    opened.revalidate()?;
    Ok(PreparedExternalPoolAdapterRuntimeBundle {
        config,
        credential,
        bundle_generation: manifest.bundle_generation,
        config_sha256,
        config_size_bytes: manifest.config_size_bytes,
        credential_sha256,
        credential_size_bytes: manifest.credential_size_bytes,
        retained_bundle: Box::new(opened),
    })
}

fn exact_sha256(
    bytes: &[u8],
    expected: &str,
) -> Result<[u8; 32], ExternalPoolAdapterRuntimeBundleError> {
    let mut digest: [u8; 32] = Sha256::digest(bytes).into();
    let mut expected_digest = [0_u8; 32];
    if hex::decode_to_slice(expected, &mut expected_digest).is_err() {
        digest.zeroize();
        expected_digest.zeroize();
        return Err(ExternalPoolAdapterRuntimeBundleError::InvalidAuthority);
    }
    let matches = digest == expected_digest;
    expected_digest.zeroize();
    if matches {
        Ok(digest)
    } else {
        digest.zeroize();
        Err(ExternalPoolAdapterRuntimeBundleError::ContentDrift)
    }
}

fn validate_profile_digest(value: &str) -> Result<(), ExternalPoolAdapterRuntimeBundleError> {
    if value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        Ok(())
    } else {
        Err(ExternalPoolAdapterRuntimeBundleError::InvalidAuthority)
    }
}

pub(super) trait OpenedRuntimeBundle {
    fn read_manifest(
        &mut self,
        max_bytes: u64,
    ) -> Result<LockedSensitiveBytes, ExternalPoolAdapterRuntimeBundleError>;
    fn read_sensitive(
        &mut self,
        name: &'static str,
        expected_size: u64,
    ) -> Result<LockedSensitiveBytes, ExternalPoolAdapterRuntimeBundleError>;
    fn revalidate(&self) -> Result<(), ExternalPoolAdapterRuntimeBundleError>;
}

#[cfg(target_os = "linux")]
fn platform_open_bundle(
    root: &ExternalPoolAdapterRuntimeBundleRoot,
    digest: &str,
) -> Result<impl OpenedRuntimeBundle, ExternalPoolAdapterRuntimeBundleError> {
    linux::LinuxOpenedRuntimeBundle::open(root.as_path(), digest)
}

#[cfg(windows)]
fn platform_open_bundle(
    root: &ExternalPoolAdapterRuntimeBundleRoot,
    digest: &str,
) -> Result<impl OpenedRuntimeBundle, ExternalPoolAdapterRuntimeBundleError> {
    windows::WindowsOpenedRuntimeBundle::open(root.as_path(), digest)
}

#[cfg(not(any(target_os = "linux", windows)))]
fn platform_open_bundle(
    _root: &ExternalPoolAdapterRuntimeBundleRoot,
    _digest: &str,
) -> Result<UnsupportedOpenedRuntimeBundle, ExternalPoolAdapterRuntimeBundleError> {
    Err(ExternalPoolAdapterRuntimeBundleError::UnsafeCustody)
}

#[cfg(not(any(target_os = "linux", windows)))]
struct UnsupportedOpenedRuntimeBundle;

#[cfg(not(any(target_os = "linux", windows)))]
impl OpenedRuntimeBundle for UnsupportedOpenedRuntimeBundle {
    fn read_manifest(
        &mut self,
        _max_bytes: u64,
    ) -> Result<LockedSensitiveBytes, ExternalPoolAdapterRuntimeBundleError> {
        Err(ExternalPoolAdapterRuntimeBundleError::UnsafeCustody)
    }
    fn read_sensitive(
        &mut self,
        _name: &'static str,
        _size: u64,
    ) -> Result<LockedSensitiveBytes, ExternalPoolAdapterRuntimeBundleError> {
        Err(ExternalPoolAdapterRuntimeBundleError::UnsafeCustody)
    }
    fn revalidate(&self) -> Result<(), ExternalPoolAdapterRuntimeBundleError> {
        Err(ExternalPoolAdapterRuntimeBundleError::UnsafeCustody)
    }
}

#[cfg(test)]
#[path = "filesystem_tests.rs"]
mod tests;
