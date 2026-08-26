//! Borrow-only discovery of retained Windows launch-path candidates.
//!
//! Receipts stay attached to typed owners by the caller. They are not CWD selection, namespace
//! grants, loader authority, or permission to reconstruct an owner from scalar material.

#![allow(dead_code)]

use std::{collections::HashSet, fs::File, path::Path, sync::Arc};

use anyhow::{anyhow, bail, Result};
use sha2::{Digest, Sha256};

use super::{
    platform, ManagedObjectBinding, PinnedManagedDirectory, PinnedManagedExtractionLoaderDirectory,
    PinnedManagedFile,
};

pub(super) const DIRECTORY_DISCOVERY_CLASS_PROVENANCE: &str =
    "managed_fs_directory_delete_share_denied_discovery_minimum_v1";
pub(super) const FILE_DISCOVERY_CLASS_PROVENANCE: &str =
    "managed_fs_file_share_none_discovery_minimum_v1";

#[repr(u8)]
pub(crate) enum ManagedLoaderLaunchPathObjectKind {
    Directory,
    File,
}

pub(crate) struct ManagedLoaderLaunchPathComponentDiscovery {
    pub(super) ordinal: usize,
    pub(super) parent_identity_digest: String,
    pub(super) normalized_component: String,
    pub(super) object_identity_digest: String,
    pub(super) object_kind: ManagedLoaderLaunchPathObjectKind,
    pub(super) granted_access: u32,
    /// Windows cannot query existing share flags. This is only a broad static opener class plus
    /// queried discovery-minimum access, not an exact opener recipe or dynamic share evidence.
    pub(super) discovery_class_provenance: &'static str,
}

pub(crate) struct ManagedLoaderLaunchPathDiscoveryReceipt {
    pub(super) managed_root_identity_digest: String,
    pub(super) final_identity_digest: String,
    pub(super) canonical_path_digest: String,
    pub(super) component_set_digest: String,
    pub(super) retained_chain_discovery_class_digest: String,
    pub(super) observation_receipt_digest: String,
    pub(super) components: Vec<ManagedLoaderLaunchPathComponentDiscovery>,
}

pub(crate) struct ManagedLoaderPlanDirectoryLaunchPathDiscovery {
    directory_ordinal: usize,
    receipt: ManagedLoaderLaunchPathDiscoveryReceipt,
}

pub(crate) struct ManagedLoaderLaunchPathDiscoverySet {
    application: ManagedLoaderLaunchPathDiscoveryReceipt,
    package_root: ManagedLoaderLaunchPathDiscoveryReceipt,
    plan_directories: Vec<ManagedLoaderPlanDirectoryLaunchPathDiscovery>,
    aggregate_receipt_digest: String,
}

/// Discovers every admissible CWD candidate without selecting one. All inputs remain borrowed, so
/// any error leaves the exact retained owner graph with the caller.
pub(crate) fn discover_loader_launch_path_candidates(
    application: &PinnedManagedFile,
    package_root: &PinnedManagedExtractionLoaderDirectory,
    plan_directories: &[PinnedManagedDirectory],
) -> Result<ManagedLoaderLaunchPathDiscoverySet> {
    let package_root = package_root.discovery_directory();
    require_common_owner(package_root, application)?;
    require_handle_prefix(
        &package_root.directory_handles,
        &application._directory_handles,
    )?;

    let package_root_receipt = platform::discover_loader_directory_launch_path(package_root)?;
    let application_receipt = platform::discover_loader_file_launch_path(application)?;
    let mut seen = HashSet::with_capacity(plan_directories.len() + 1);
    seen.insert(package_root_receipt.final_identity_digest.clone());
    let mut discovered = Vec::with_capacity(plan_directories.len());
    for (directory_ordinal, directory) in plan_directories.iter().enumerate() {
        require_common_directory(package_root, directory)?;
        require_handle_prefix(
            &package_root.directory_handles,
            &directory.directory_handles,
        )?;
        let receipt = platform::discover_loader_directory_launch_path(directory)?;
        if !seen.insert(receipt.final_identity_digest.clone()) {
            bail!("NODE_MANAGED_LOADER_LAUNCH_PATH_DIRECTORY_DUPLICATED");
        }
        discovered.push(ManagedLoaderPlanDirectoryLaunchPathDiscovery {
            directory_ordinal,
            receipt,
        });
    }

    let mut aggregate = ReceiptDigest::new(b"ELON_MANAGED_LOADER_LAUNCH_PATH_SET_V1");
    aggregate.text(&application_receipt.observation_receipt_digest);
    aggregate.text(&package_root_receipt.observation_receipt_digest);
    for entry in &discovered {
        aggregate.usize(entry.directory_ordinal);
        aggregate.text(&entry.receipt.observation_receipt_digest);
    }
    Ok(ManagedLoaderLaunchPathDiscoverySet {
        application: application_receipt,
        package_root: package_root_receipt,
        plan_directories: discovered,
        aggregate_receipt_digest: aggregate.finish(),
    })
}

fn require_common_owner(root: &PinnedManagedDirectory, file: &PinnedManagedFile) -> Result<()> {
    if root.root_volume_serial != file.root_volume_serial
        || root.root_identity_digest != file.root_identity_digest
    {
        bail!("NODE_MANAGED_LOADER_LAUNCH_PATH_ROOT_CHANGED");
    }
    Ok(())
}

fn require_common_directory(
    left: &PinnedManagedDirectory,
    right: &PinnedManagedDirectory,
) -> Result<()> {
    if left.root_volume_serial != right.root_volume_serial
        || left.root_identity_digest != right.root_identity_digest
    {
        bail!("NODE_MANAGED_LOADER_LAUNCH_PATH_ROOT_CHANGED");
    }
    Ok(())
}

fn require_handle_prefix(expected: &[Arc<File>], actual: &[Arc<File>]) -> Result<()> {
    if expected.len() > actual.len()
        || !expected
            .iter()
            .zip(actual)
            .all(|(left, right)| Arc::ptr_eq(left, right))
    {
        bail!("NODE_MANAGED_LOADER_LAUNCH_PATH_OWNER_CHAIN_CHANGED");
    }
    Ok(())
}

pub(super) fn seal_discovered_path(
    root: &str,
    final_identity: String,
    canonical_path: &Path,
    anchor_access: u32,
    final_is_file: bool,
    binding: &ManagedObjectBinding,
    components: Vec<ManagedLoaderLaunchPathComponentDiscovery>,
) -> Result<ManagedLoaderLaunchPathDiscoveryReceipt> {
    let canonical_path = canonical_path
        .to_str()
        .filter(|value| !value.contains('\0'))
        .ok_or_else(|| anyhow!("NODE_MANAGED_LOADER_LAUNCH_PATH_CANONICAL_NOT_UTF8"))?;
    let final_component = components
        .last()
        .ok_or_else(|| anyhow!("NODE_MANAGED_LOADER_LAUNCH_PATH_FINAL_MISSING"))?;
    if binding.is_directory() == final_is_file
        || binding.identity_digest() != final_component.object_identity_digest
        || binding.parent_identity_digest() != final_component.parent_identity_digest
        || binding.relative_name().to_str() != Some(&final_component.normalized_component)
    {
        bail!("NODE_MANAGED_LOADER_LAUNCH_PATH_FINAL_BINDING_CHANGED");
    }
    let canonical_path_digest =
        digest_text(b"ELON_MANAGED_LOADER_CANONICAL_PATH_V1", canonical_path);
    let mut component_set = ReceiptDigest::new(b"ELON_MANAGED_LOADER_PATH_COMPONENTS_V1");
    let mut discovery_class = ReceiptDigest::new(b"ELON_MANAGED_LOADER_PATH_DISCOVERY_CLASS_V1");
    discovery_class.u32(anchor_access);
    discovery_class.text(DIRECTORY_DISCOVERY_CLASS_PROVENANCE);
    for entry in &components {
        component_set.usize(entry.ordinal);
        component_set.text(&entry.parent_identity_digest);
        component_set.text(&entry.normalized_component);
        component_set.text(&entry.object_identity_digest);
        component_set.u32(match &entry.object_kind {
            ManagedLoaderLaunchPathObjectKind::Directory => 0,
            ManagedLoaderLaunchPathObjectKind::File => 1,
        });
        discovery_class.u32(entry.granted_access);
        discovery_class.text(entry.discovery_class_provenance);
    }
    let component_set_digest = component_set.finish();
    let retained_chain_discovery_class_digest = discovery_class.finish();
    let mut observation = ReceiptDigest::new(b"ELON_MANAGED_LOADER_PATH_OBSERVATION_V1");
    for value in [
        root,
        &final_identity,
        &canonical_path_digest,
        &component_set_digest,
        &retained_chain_discovery_class_digest,
    ] {
        observation.text(value);
    }
    Ok(ManagedLoaderLaunchPathDiscoveryReceipt {
        managed_root_identity_digest: root.to_owned(),
        final_identity_digest: final_identity,
        canonical_path_digest,
        component_set_digest,
        retained_chain_discovery_class_digest,
        observation_receipt_digest: observation.finish(),
        components,
    })
}

fn digest_text(domain: &[u8], value: &str) -> String {
    let mut digest = ReceiptDigest::new(domain);
    digest.text(value);
    digest.finish()
}

struct ReceiptDigest(Sha256);

impl ReceiptDigest {
    fn new(domain: &[u8]) -> Self {
        let mut value = Sha256::new();
        value.update(domain);
        Self(value)
    }
    fn text(&mut self, value: &str) {
        self.0.update((value.len() as u64).to_le_bytes());
        self.0.update(value.as_bytes());
    }
    fn u32(&mut self, value: u32) {
        self.0.update(value.to_le_bytes());
    }
    fn usize(&mut self, value: usize) {
        self.0.update((value as u64).to_le_bytes());
    }
    fn finish(self) -> String {
        hex::encode(self.0.finalize())
    }
}

impl ManagedLoaderLaunchPathDiscoverySet {
    pub(crate) fn application(&self) -> &ManagedLoaderLaunchPathDiscoveryReceipt {
        &self.application
    }
    pub(crate) fn package_root(&self) -> &ManagedLoaderLaunchPathDiscoveryReceipt {
        &self.package_root
    }
    pub(crate) fn plan_directories(&self) -> &[ManagedLoaderPlanDirectoryLaunchPathDiscovery] {
        &self.plan_directories
    }

    pub(crate) fn binding(&self) -> (&str, usize) {
        (&self.aggregate_receipt_digest, self.plan_directories.len())
    }
}

impl ManagedLoaderPlanDirectoryLaunchPathDiscovery {
    pub(crate) fn binding(&self) -> (usize, &ManagedLoaderLaunchPathDiscoveryReceipt) {
        (self.directory_ordinal, &self.receipt)
    }
}

impl ManagedLoaderLaunchPathDiscoveryReceipt {
    pub(crate) fn binding(&self) -> (&str, &str, &str, &str, &str, &str) {
        (
            &self.managed_root_identity_digest,
            &self.final_identity_digest,
            &self.canonical_path_digest,
            &self.component_set_digest,
            &self.retained_chain_discovery_class_digest,
            &self.observation_receipt_digest,
        )
    }

    pub(crate) fn components(&self) -> &[ManagedLoaderLaunchPathComponentDiscovery] {
        &self.components
    }
}

impl ManagedLoaderLaunchPathComponentDiscovery {
    pub(crate) fn binding(
        &self,
    ) -> (
        usize,
        &str,
        &str,
        &str,
        &ManagedLoaderLaunchPathObjectKind,
        u32,
        &str,
    ) {
        (
            self.ordinal,
            &self.parent_identity_digest,
            &self.normalized_component,
            &self.object_identity_digest,
            &self.object_kind,
            self.granted_access,
            self.discovery_class_provenance,
        )
    }
}
