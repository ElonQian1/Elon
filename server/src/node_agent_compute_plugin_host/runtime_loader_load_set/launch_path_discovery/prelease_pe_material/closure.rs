//! Reachable package-image closure and typed external dependency leaves.

use anyhow::{bail, Result};

use crate::node_agent_compute_plugin_host::manifest_validation::is_sha256;

use super::{
    digest::external_leaf_binding_digest, dll_module_name_is_canonical, strictly_sorted_unique,
    symbol_is_exact, AuthenticatedWindowsPreLeasePeMaterial, WindowsPreLeaseImportKind,
};

pub(super) struct WindowsPreLeaseExternalLeafRequest {
    pub(super) leaf_ordinal: usize,
    pub(super) source_import_edge_ordinal: usize,
    pub(super) forwarder_hop_ordinal: Option<usize>,
    pub(super) normalized_module_name: String,
    pub(super) imported_symbol_name: Option<String>,
    pub(super) imported_symbol_ordinal: Option<u16>,
    pub(super) leaf_binding_digest: String,
}

pub(super) struct WindowsPreLeaseReachableClosureReceipt {
    pub(super) root_parsed_image_ordinal: usize,
    pub(super) reachable_image_ordinals: Vec<usize>,
    pub(super) external_leaf_requests: Vec<WindowsPreLeaseExternalLeafRequest>,
    pub(super) maximum_forwarder_depth: usize,
    pub(super) cycle_check_receipt_digest: String,
    pub(super) reachable_set_digest: String,
    pub(super) module_cache_collision_closure_digest: String,
    pub(super) authenticated_module_cache_collision_receipt_digest: String,
}

impl AuthenticatedWindowsPreLeasePeMaterial {
    pub(super) fn validate_external_leaf_coverage(&self) -> Result<()> {
        let mut expected = Vec::new();
        for edge in &self.import_edges {
            if let Some(target) = self
                .package_images
                .iter()
                .find(|image| image.normalized_module_name == edge.normalized_module_name)
            {
                if self
                    .reachable_closure
                    .reachable_image_ordinals
                    .binary_search(&target.parsed_image_ordinal)
                    .is_err()
                {
                    bail!("COMPUTE_PLUGIN_WINDOWS_PRELEASE_PE_PACKAGE_LEAF_NOT_REACHABLE");
                }
            } else {
                expected.push((
                    edge.edge_ordinal,
                    None,
                    edge.normalized_module_name.as_str(),
                    edge.imported_symbol_name.as_deref(),
                    edge.imported_symbol_ordinal,
                ));
            }
        }
        for hop in &self.forwarder_hops {
            if let Some(target) = self
                .package_images
                .iter()
                .find(|image| image.normalized_module_name == hop.target_module_name)
            {
                if self
                    .reachable_closure
                    .reachable_image_ordinals
                    .binary_search(&target.parsed_image_ordinal)
                    .is_err()
                {
                    bail!("COMPUTE_PLUGIN_WINDOWS_PRELEASE_PE_FORWARDER_LEAF_NOT_REACHABLE");
                }
            } else {
                expected.push((
                    hop.edge_ordinal,
                    Some(hop.hop_ordinal),
                    hop.target_module_name.as_str(),
                    hop.target_symbol_name.as_deref(),
                    hop.target_symbol_ordinal,
                ));
            }
        }
        if expected.len() != self.reachable_closure.external_leaf_requests.len() {
            bail!("COMPUTE_PLUGIN_WINDOWS_PRELEASE_PE_EXTERNAL_LEAF_COVERAGE_CHANGED");
        }
        for (ordinal, (leaf, expected)) in self
            .reachable_closure
            .external_leaf_requests
            .iter()
            .zip(expected)
            .enumerate()
        {
            if leaf.leaf_ordinal != ordinal
                || leaf.source_import_edge_ordinal != expected.0
                || leaf.forwarder_hop_ordinal != expected.1
                || leaf.normalized_module_name != expected.2
                || leaf.imported_symbol_name.as_deref() != expected.3
                || leaf.imported_symbol_ordinal != expected.4
                || !dll_module_name_is_canonical(&leaf.normalized_module_name)
                || !symbol_is_exact(
                    leaf.imported_symbol_name.as_deref(),
                    leaf.imported_symbol_ordinal,
                )
                || !is_sha256(&leaf.leaf_binding_digest)
                || leaf.leaf_binding_digest != external_leaf_binding_digest(leaf)
            {
                bail!("COMPUTE_PLUGIN_WINDOWS_PRELEASE_PE_EXTERNAL_LEAF_CHANGED");
            }
        }
        Ok(())
    }
}

impl WindowsPreLeaseReachableClosureReceipt {
    pub(super) fn validate(
        &self,
        image_count: usize,
        runner_ordinal: usize,
        observed_forwarder_depth: usize,
    ) -> Result<()> {
        if self.root_parsed_image_ordinal != runner_ordinal
            || self.maximum_forwarder_depth != observed_forwarder_depth
            || self
                .reachable_image_ordinals
                .binary_search(&runner_ordinal)
                .is_err()
            || !strictly_sorted_unique(&self.reachable_image_ordinals)
            || self
                .reachable_image_ordinals
                .iter()
                .any(|ordinal| *ordinal >= image_count)
            || [
                &self.cycle_check_receipt_digest,
                &self.reachable_set_digest,
                &self.module_cache_collision_closure_digest,
                &self.authenticated_module_cache_collision_receipt_digest,
            ]
            .into_iter()
            .any(|value| !is_sha256(value))
        {
            bail!("COMPUTE_PLUGIN_WINDOWS_PRELEASE_PE_CLOSURE_INVALID");
        }
        Ok(())
    }
}
