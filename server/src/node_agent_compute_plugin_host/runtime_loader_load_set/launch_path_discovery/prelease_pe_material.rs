//! Authenticated pre-lease PE material for a retained Windows Runner package.
//!
//! The material deliberately excludes grants, content leases and lease generations. Its producer
//! remains uninhabited until a retained-handle parser and authenticated bootstrap projection exist.

#![allow(dead_code)]

mod closure;
mod digest;

use std::{collections::HashSet, convert::Infallible, fmt};

use anyhow::{bail, Result};

use crate::node_agent_compute_plugin_host::manifest_validation::is_sha256;

use closure::{WindowsPreLeaseExternalLeafRequest, WindowsPreLeaseReachableClosureReceipt};
use digest::{
    canonical_merge_rule_digest, module_cache_collision_closure_digest, reachable_set_digest,
};

const CANONICAL_EDGE_MERGE_RULE: &str =
    "normal_then_delay_import_edges_then_forwarder_hops_by_source_edge_and_hop_v2";

pub(super) enum WindowsPreLeaseImportKind {
    Normal,
    Delay,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum WindowsPreLeasePackageImageRole {
    RunnerExecutable,
    LoadableDll,
}

pub(super) struct WindowsPreLeaseParsedPackageImage {
    parsed_image_ordinal: usize,
    package_file_ordinal: usize,
    image_role: WindowsPreLeasePackageImageRole,
    relative_path: String,
    normalized_module_name: String,
    file_identity_digest: String,
    sealed_file_digest: String,
    size_bytes: u64,
    machine_kind: String,
    pe_kind: String,
    parser_input_receipt_digest: String,
}

pub(super) struct WindowsPreLeaseImportEdge {
    edge_ordinal: usize,
    importer_image_ordinal: usize,
    importer_edge_ordinal: usize,
    import_kind: WindowsPreLeaseImportKind,
    normalized_module_name: String,
    imported_symbol_name: Option<String>,
    imported_symbol_ordinal: Option<u16>,
    descriptor_ordinal: usize,
    thunk_ordinal: usize,
    canonical_merge_ordinal: usize,
    edge_evidence_digest: String,
}

pub(super) struct WindowsPreLeaseForwarderHop {
    edge_ordinal: usize,
    hop_ordinal: usize,
    source_image_ordinal: usize,
    source_export_name: Option<String>,
    source_export_ordinal: Option<u16>,
    target_module_name: String,
    target_symbol_name: Option<String>,
    target_symbol_ordinal: Option<u16>,
    hop_evidence_digest: String,
}

/// Exact parser projection before any namespace grant or immutable-content lease dispatch.
pub(super) struct AuthenticatedWindowsPreLeasePeMaterial {
    admission_source_digest: String,
    admission_receipt_digest: String,
    extraction_plan_digest: String,
    extraction_evidence_digest: String,
    launch_path_candidate_set_digest: String,
    runner_file_ordinal: usize,
    runner_file_identity_digest: String,
    target_architecture: String,
    process_machine_context_digest: String,
    parser_policy_digest: String,
    authenticated_preloaded_module_set_digest: String,
    package_images: Vec<WindowsPreLeaseParsedPackageImage>,
    import_edges: Vec<WindowsPreLeaseImportEdge>,
    forwarder_hops: Vec<WindowsPreLeaseForwarderHop>,
    canonical_merge_rule_digest: String,
    reachable_closure: WindowsPreLeaseReachableClosureReceipt,
    material_set_digest: String,
    _authenticated_prelease_pe_parser_producer_unavailable: Infallible,
}

impl AuthenticatedWindowsPreLeasePeMaterial {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn validate_binding(
        &self,
        admission_source_digest: &str,
        admission_receipt_digest: &str,
        extraction_plan_digest: &str,
        extraction_evidence_digest: &str,
        launch_path_candidate_set_digest: &str,
        runner_file_ordinal: usize,
        runner_file_identity_digest: &str,
        target_architecture: &str,
        process_machine_context_digest: &str,
    ) -> Result<()> {
        let runner_image = self
            .package_images
            .iter()
            .find(|image| image.package_file_ordinal == runner_file_ordinal)
            .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_WINDOWS_RUNNER_PE_IMAGE_MISSING"))?;
        if self.admission_source_digest != admission_source_digest
            || self.admission_receipt_digest != admission_receipt_digest
            || self.extraction_plan_digest != extraction_plan_digest
            || self.extraction_evidence_digest != extraction_evidence_digest
            || self.launch_path_candidate_set_digest != launch_path_candidate_set_digest
            || self.runner_file_ordinal != runner_file_ordinal
            || self.runner_file_identity_digest != runner_file_identity_digest
            || self.target_architecture != target_architecture
            || self.process_machine_context_digest != process_machine_context_digest
            || self.package_images.is_empty()
            || runner_image.file_identity_digest != runner_file_identity_digest
            || runner_image.parsed_image_ordinal != 0
        {
            bail!("COMPUTE_PLUGIN_WINDOWS_PRELEASE_PE_BINDING_CHANGED");
        }
        self.validate_shape(runner_image.parsed_image_ordinal)?;
        if self.recompute_digest() != self.material_set_digest {
            bail!("COMPUTE_PLUGIN_WINDOWS_PRELEASE_PE_DIGEST_CHANGED");
        }
        Ok(())
    }

    fn forwarders_are_contiguous(&self) -> bool {
        let mut hops = self.forwarder_hops.iter().peekable();
        for edge in &self.import_edges {
            let mut expected_hop_ordinal = 0;
            let mut previous_target_module: Option<&str> = None;
            let mut previous_target_symbol: Option<(Option<&str>, Option<u16>)> = None;
            while hops
                .peek()
                .is_some_and(|hop| hop.edge_ordinal == edge.edge_ordinal)
            {
                let Some(hop) = hops.next() else {
                    return false;
                };
                if hop.hop_ordinal != expected_hop_ordinal
                    || (expected_hop_ordinal == 0
                        && (self.package_images[hop.source_image_ordinal].normalized_module_name
                            != edge.normalized_module_name
                            || !same_symbol(
                                edge.imported_symbol_name.as_deref(),
                                edge.imported_symbol_ordinal,
                                hop.source_export_name.as_deref(),
                                hop.source_export_ordinal,
                            )))
                    || previous_target_module.is_some_and(|target| {
                        self.package_images[hop.source_image_ordinal]
                            .normalized_module_name
                            .as_str()
                            != target
                    })
                    || previous_target_symbol.is_some_and(|(name, ordinal)| {
                        !same_symbol(
                            name,
                            ordinal,
                            hop.source_export_name.as_deref(),
                            hop.source_export_ordinal,
                        )
                    })
                {
                    return false;
                }
                expected_hop_ordinal += 1;
                previous_target_module = Some(&hop.target_module_name);
                previous_target_symbol =
                    Some((hop.target_symbol_name.as_deref(), hop.target_symbol_ordinal));
            }
        }
        hops.next().is_none()
    }

    fn maximum_observed_forwarder_depth(&self) -> usize {
        self.forwarder_hops
            .iter()
            .map(|hop| hop.hop_ordinal + 1)
            .max()
            .unwrap_or(0)
    }

    fn validate_shape(&self, runner_parsed_image_ordinal: usize) -> Result<()> {
        let mut package_file_ordinals = HashSet::new();
        let mut package_module_cache_keys = HashSet::new();
        if [
            &self.admission_source_digest,
            &self.admission_receipt_digest,
            &self.extraction_plan_digest,
            &self.extraction_evidence_digest,
            &self.launch_path_candidate_set_digest,
            &self.runner_file_identity_digest,
            &self.process_machine_context_digest,
            &self.parser_policy_digest,
            &self.authenticated_preloaded_module_set_digest,
            &self.canonical_merge_rule_digest,
            &self.material_set_digest,
        ]
        .into_iter()
        .any(|value| !is_sha256(value))
            || self
                .package_images
                .iter()
                .enumerate()
                .any(|(ordinal, image)| {
                    image.parsed_image_ordinal != ordinal
                        || !package_file_ordinals.insert(image.package_file_ordinal)
                        || !package_module_cache_keys.insert(&image.normalized_module_name)
                        || image.relative_path.is_empty()
                        || image.relative_path.contains('\\')
                        || !package_image_role_and_name_are_canonical(
                            image.image_role,
                            image.parsed_image_ordinal == runner_parsed_image_ordinal,
                            &image.normalized_module_name,
                        )
                        || !module_name_matches_path(
                            &image.normalized_module_name,
                            &image.relative_path,
                        )
                        || image.size_bytes == 0
                        || image.machine_kind != self.target_architecture
                        || !matches!(image.pe_kind.as_str(), "pe32" | "pe32_plus")
                        || !is_sha256(&image.file_identity_digest)
                        || !is_sha256(&image.sealed_file_digest)
                        || !is_sha256(&image.parser_input_receipt_digest)
                })
            || self.import_edges.iter().enumerate().any(|(ordinal, edge)| {
                edge.edge_ordinal != ordinal
                    || edge.canonical_merge_ordinal != ordinal
                    || edge.importer_edge_ordinal
                        != self.import_edges[..ordinal]
                            .iter()
                            .filter(|prior| {
                                prior.importer_image_ordinal == edge.importer_image_ordinal
                            })
                            .count()
                    || edge.importer_image_ordinal >= self.package_images.len()
                    || !dll_module_name_is_canonical(&edge.normalized_module_name)
                    || !symbol_is_exact(
                        edge.imported_symbol_name.as_deref(),
                        edge.imported_symbol_ordinal,
                    )
                    || !is_sha256(&edge.edge_evidence_digest)
            })
            || self.forwarder_hops.iter().any(|hop| {
                hop.edge_ordinal >= self.import_edges.len()
                    || hop.source_image_ordinal >= self.package_images.len()
                    || !dll_module_name_is_canonical(&hop.target_module_name)
                    || !symbol_is_exact(
                        hop.source_export_name.as_deref(),
                        hop.source_export_ordinal,
                    )
                    || !symbol_is_exact(
                        hop.target_symbol_name.as_deref(),
                        hop.target_symbol_ordinal,
                    )
                    || !is_sha256(&hop.hop_evidence_digest)
            })
            || self.forwarder_hops.windows(2).any(|pair| {
                (pair[0].edge_ordinal, pair[0].hop_ordinal)
                    >= (pair[1].edge_ordinal, pair[1].hop_ordinal)
            })
            || self
                .import_edges
                .windows(2)
                .any(|pair| pair[0].canonical_sort_key() >= pair[1].canonical_sort_key())
            || !self.forwarders_are_contiguous()
            || self.canonical_merge_rule_digest != canonical_merge_rule_digest()
        {
            bail!("COMPUTE_PLUGIN_WINDOWS_PRELEASE_PE_SHAPE_INVALID");
        }
        self.reachable_closure.validate(
            self.package_images.len(),
            runner_parsed_image_ordinal,
            self.maximum_observed_forwarder_depth(),
        )?;
        if self.reachable_closure.reachable_set_digest
            != reachable_set_digest(
                self.reachable_closure.root_parsed_image_ordinal,
                &self.reachable_closure.reachable_image_ordinals,
                &self.reachable_closure.external_leaf_requests,
            )
            || self.reachable_closure.module_cache_collision_closure_digest
                != module_cache_collision_closure_digest(
                    &self.authenticated_preloaded_module_set_digest,
                    &self.package_images,
                    &self.import_edges,
                    &self.forwarder_hops,
                )
        {
            bail!("COMPUTE_PLUGIN_WINDOWS_PRELEASE_PE_CLOSURE_DIGEST_CHANGED");
        }
        self.validate_external_leaf_coverage()?;
        if self.import_edges.iter().any(|edge| {
            self.reachable_closure
                .reachable_image_ordinals
                .binary_search(&edge.importer_image_ordinal)
                .is_err()
        }) || self.forwarder_hops.iter().any(|hop| {
            self.reachable_closure
                .reachable_image_ordinals
                .binary_search(&hop.source_image_ordinal)
                .is_err()
        }) {
            bail!("COMPUTE_PLUGIN_WINDOWS_PRELEASE_PE_REACHABLE_EDGE_CHANGED");
        }
        Ok(())
    }

    pub(super) fn material_set_digest(&self) -> &str {
        &self.material_set_digest
    }

    pub(super) fn parser_policy_digest(&self) -> &str {
        &self.parser_policy_digest
    }

    pub(super) fn preloaded_module_set_digest(&self) -> &str {
        &self.authenticated_preloaded_module_set_digest
    }

    pub(super) fn package_images(&self) -> &[WindowsPreLeaseParsedPackageImage] {
        &self.package_images
    }

    pub(super) fn import_edges(&self) -> &[WindowsPreLeaseImportEdge] {
        &self.import_edges
    }

    pub(super) fn forwarder_hops(&self) -> &[WindowsPreLeaseForwarderHop] {
        &self.forwarder_hops
    }
}

impl WindowsPreLeaseParsedPackageImage {
    pub(super) fn parsed_image_ordinal(&self) -> usize {
        self.parsed_image_ordinal
    }

    pub(super) fn package_file_ordinal(&self) -> usize {
        self.package_file_ordinal
    }

    pub(super) fn relative_path(&self) -> &str {
        &self.relative_path
    }

    pub(super) fn normalized_module_name(&self) -> &str {
        &self.normalized_module_name
    }

    pub(super) fn file_identity_digest(&self) -> &str {
        &self.file_identity_digest
    }

    pub(super) fn sealed_file_digest(&self) -> &str {
        &self.sealed_file_digest
    }

    pub(super) fn size_bytes(&self) -> u64 {
        self.size_bytes
    }
}

impl WindowsPreLeaseImportEdge {
    pub(super) fn edge_ordinal(&self) -> usize {
        self.edge_ordinal
    }

    pub(super) fn importer_image_ordinal(&self) -> usize {
        self.importer_image_ordinal
    }

    pub(super) fn importer_edge_ordinal(&self) -> usize {
        self.importer_edge_ordinal
    }

    pub(super) fn import_kind(&self) -> &WindowsPreLeaseImportKind {
        &self.import_kind
    }

    pub(super) fn imported_symbol_binding(&self) -> (Option<&str>, Option<u16>) {
        (
            self.imported_symbol_name.as_deref(),
            self.imported_symbol_ordinal,
        )
    }

    pub(super) fn descriptor_and_thunk_ordinals(&self) -> (usize, usize) {
        (self.descriptor_ordinal, self.thunk_ordinal)
    }

    pub(super) fn edge_evidence_digest(&self) -> &str {
        &self.edge_evidence_digest
    }

    pub(super) fn normalized_module_name(&self) -> &str {
        &self.normalized_module_name
    }

    fn canonical_sort_key(&self) -> (u8, usize, usize, usize) {
        (
            self.import_kind.canonical_rank(),
            self.importer_image_ordinal,
            self.descriptor_ordinal,
            self.thunk_ordinal,
        )
    }
}

impl WindowsPreLeaseForwarderHop {
    pub(super) fn source_edge_and_hop_ordinals(&self) -> (usize, usize) {
        (self.edge_ordinal, self.hop_ordinal)
    }

    pub(super) fn source_image_ordinal(&self) -> usize {
        self.source_image_ordinal
    }

    pub(super) fn source_symbol_binding(&self) -> (Option<&str>, Option<u16>) {
        (
            self.source_export_name.as_deref(),
            self.source_export_ordinal,
        )
    }

    pub(super) fn target_module_name(&self) -> &str {
        &self.target_module_name
    }

    pub(super) fn target_symbol_binding(&self) -> (Option<&str>, Option<u16>) {
        (
            self.target_symbol_name.as_deref(),
            self.target_symbol_ordinal,
        )
    }

    pub(super) fn hop_evidence_digest(&self) -> &str {
        &self.hop_evidence_digest
    }
}

impl WindowsPreLeaseImportKind {
    fn canonical_rank(&self) -> u8 {
        match self {
            Self::Normal => 0,
            Self::Delay => 1,
        }
    }
}

fn symbol_is_exact(name: Option<&str>, ordinal: Option<u16>) -> bool {
    matches!((name, ordinal), (Some(value), None) if !value.is_empty() && !value.contains('\0'))
        || matches!((name, ordinal), (None, Some(_)))
}

fn same_symbol(
    left_name: Option<&str>,
    left_ordinal: Option<u16>,
    right_name: Option<&str>,
    right_ordinal: Option<u16>,
) -> bool {
    left_name == right_name && left_ordinal == right_ordinal
}

fn package_image_role_and_name_are_canonical(
    role: WindowsPreLeasePackageImageRole,
    is_runner: bool,
    value: &str,
) -> bool {
    match (role, is_runner) {
        (WindowsPreLeasePackageImageRole::RunnerExecutable, true) => {
            canonical_module_name_with_suffix(value, ".exe")
        }
        (WindowsPreLeasePackageImageRole::LoadableDll, false) => {
            canonical_module_name_with_suffix(value, ".dll")
        }
        _ => false,
    }
}

fn dll_module_name_is_canonical(value: &str) -> bool {
    canonical_module_name_with_suffix(value, ".dll")
}

fn canonical_module_name_with_suffix(value: &str, suffix: &str) -> bool {
    !value.is_empty()
        && value.ends_with(suffix)
        && value.bytes().all(|unit| {
            unit.is_ascii_lowercase() || unit.is_ascii_digit() || matches!(unit, b'.' | b'-' | b'_')
        })
}

fn module_name_matches_path(module_name: &str, relative_path: &str) -> bool {
    relative_path
        .rsplit('/')
        .next()
        .is_some_and(|basename| basename.to_ascii_lowercase() == module_name)
}

fn strictly_sorted_unique(values: &[usize]) -> bool {
    values.windows(2).all(|pair| pair[0] < pair[1])
}

impl fmt::Debug for AuthenticatedWindowsPreLeasePeMaterial {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AuthenticatedWindowsPreLeasePeMaterial")
            .field("package_image_count", &self.package_images.len())
            .field("import_edge_count", &self.import_edges.len())
            .field("forwarder_hop_count", &self.forwarder_hops.len())
            .field("material_set_digest", &"<redacted>")
            .finish()
    }
}
