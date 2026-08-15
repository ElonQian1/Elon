use std::{fs::File, os::unix::fs::FileExt};

use anyhow::{bail, Result};
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

use crate::compute_federation::{
    external_pool_adapter_installation::PreparedExternalPoolAdapterInstallation,
    external_pool_adapter_runtime_compatibility_verification::*,
};

pub(super) struct RuntimeCompatibilityFixtureBytes {
    pub(super) config: Zeroizing<Vec<u8>>,
    pub(super) credential: Zeroizing<Vec<u8>>,
    pub(super) request: Zeroizing<Vec<u8>>,
    pub(super) response: Zeroizing<Vec<u8>>,
}

pub(super) fn audit_prepared_installation(
    prepared: &PreparedExternalPoolAdapterInstallation,
    challenge: &ExternalPoolAdapterRuntimeCompatibilityChallengeReceipt,
) -> Result<()> {
    let expected = &challenge.challenge;
    let release = &expected.registry_release.release;
    let binding = prepared.binding();
    if prepared.installation_content_digest() != release.installation_content_digest
        || binding.admission_id != release.admission_id
        || binding.admission_digest != release.admission_digest
        || binding.package_receipt_id != release.package_receipt_id
        || binding.package_receipt_digest != release.package_receipt_digest
        || binding.package_material_digest != release.package_material_digest
        || binding.source_receipt_id != release.source_receipt_id
        || binding.source_receipt_digest != release.source_receipt_digest
        || binding.adapter_id != release.adapter_id
        || binding.adapter_release_version != release.release_version
        || binding.declared_implementation_sha256 != release.declared_implementation_sha256
        || binding.capability_set_digest != release.capability_set_digest
        || binding.archive_sha256 != release.archive_sha256
        || binding.archive_size_bytes != release.archive_size_bytes
        || binding.manifest_digest != release.manifest_digest
        || binding.entry_inventory_digest != release.entry_inventory_digest
        || binding.entry_count != release.entry_count
        || binding.total_uncompressed_bytes != release.total_uncompressed_bytes
        || binding.runtime_kind != expected.runtime_kind
        || binding.entrypoint_path != expected.entrypoint_path
        || binding.entrypoint_sha256 != expected.entrypoint_sha256
        || binding.entrypoint_size_bytes != expected.entrypoint_size_bytes
        || binding.installed_files.len() != release.manifest.files.len()
        || binding
            .installed_files
            .iter()
            .zip(&release.manifest.files)
            .any(|(installed, manifest)| {
                installed.path != manifest.path
                    || installed.sha256 != manifest.sha256
                    || installed.size_bytes != manifest.size_bytes
                    || installed.role != manifest.role
            })
    {
        bail!("V268 Prepared installation is not the exact provider-neutral release content");
    }
    let (_, entrypoint_sha, entrypoint_size) = prepared.retained_entrypoint()?;
    if entrypoint_sha != expected.entrypoint_sha256
        || entrypoint_size != expected.entrypoint_size_bytes
    {
        bail!("V268 retained entrypoint is not exact");
    }
    for resource in &expected.fixture_resources {
        let (_, sha256, size_bytes) = prepared.retained_resource(&resource.path)?;
        if sha256 != resource.sha256 || size_bytes != resource.size_bytes {
            bail!("V268 retained public fixture root drifted");
        }
    }
    Ok(())
}

pub(super) fn load_public_fixtures(
    prepared: &PreparedExternalPoolAdapterInstallation,
    challenge: &ExternalPoolAdapterRuntimeCompatibilityChallengeReceipt,
) -> Result<RuntimeCompatibilityFixtureBytes> {
    let resources = &challenge.challenge.fixture_resources;
    Ok(RuntimeCompatibilityFixtureBytes {
        config: load_fixture(prepared, resource(resources, "config")?)?,
        credential: load_fixture(prepared, resource(resources, "credential")?)?,
        request: load_fixture(prepared, resource(resources, "no_work_request")?)?,
        response: load_fixture(prepared, resource(resources, "no_work_response")?)?,
    })
}

fn resource<'a>(
    resources: &'a [ExternalPoolAdapterRuntimeCompatibilityFixtureResourceIdentity],
    purpose: &str,
) -> Result<&'a ExternalPoolAdapterRuntimeCompatibilityFixtureResourceIdentity> {
    let matches: Vec<_> = resources
        .iter()
        .filter(|resource| resource.purpose == purpose)
        .collect();
    if matches.len() != 1 {
        bail!("V268 controlled public fixture inventory is not exact");
    }
    Ok(matches[0])
}

fn load_fixture(
    prepared: &PreparedExternalPoolAdapterInstallation,
    expected: &ExternalPoolAdapterRuntimeCompatibilityFixtureResourceIdentity,
) -> Result<Zeroizing<Vec<u8>>> {
    let (file, sha256, size_bytes) = prepared.retained_resource(&expected.path)?;
    if sha256 != expected.sha256 || size_bytes != expected.size_bytes {
        bail!("V268 retained public fixture identity drifted");
    }
    let length = usize::try_from(size_bytes)?;
    let mut bytes = Zeroizing::new(vec![0_u8; length]);
    read_exact_at(file, &mut bytes, size_bytes)?;
    if hex::encode(Sha256::digest(&bytes[..])) != expected.sha256 {
        bail!("V268 retained public fixture bytes drifted");
    }
    Ok(bytes)
}

fn read_exact_at(file: &File, mut output: &mut [u8], expected_size: u64) -> Result<()> {
    if file.metadata()?.len() != expected_size {
        bail!("V268 retained public fixture length drifted");
    }
    let mut offset = 0_u64;
    while !output.is_empty() {
        match file.read_at(output, offset)? {
            0 => bail!("V268 retained public fixture ended early"),
            read => {
                output = &mut output[read..];
                offset += u64::try_from(read)?;
            }
        }
    }
    Ok(())
}

pub(super) fn ordered_observations(
    duration_ms: u64,
) -> Vec<ExternalPoolAdapterRuntimeCompatibilityObservation> {
    REQUIRED_RUNTIME_COMPATIBILITY_OBSERVATIONS
        .into_iter()
        .map(
            |observation_id| ExternalPoolAdapterRuntimeCompatibilityObservation {
                observation_id: observation_id.into(),
                observation_revision: 1,
                outcome: "passed".into(),
                duration_ms,
                policy_violation_count: 0,
            },
        )
        .collect()
}
