//! Canonical digest material for authenticated pre-lease PE observations.

use sha2::{Digest, Sha256};

use super::{
    AuthenticatedWindowsPreLeasePeMaterial, WindowsPreLeaseImportKind,
    WindowsPreLeasePackageImageRole,
};

impl AuthenticatedWindowsPreLeasePeMaterial {
    pub(super) fn recompute_digest(&self) -> String {
        let mut digest = MaterialDigest::new(b"ELON_WINDOWS_RUNNER_PRELEASE_PE_MATERIAL_V1");
        for value in [
            &self.admission_source_digest,
            &self.admission_receipt_digest,
            &self.extraction_plan_digest,
            &self.extraction_evidence_digest,
            &self.launch_path_candidate_set_digest,
            &self.runner_file_identity_digest,
            &self.target_architecture,
            &self.process_machine_context_digest,
            &self.parser_policy_digest,
            &self.authenticated_preloaded_module_set_digest,
            &self.canonical_merge_rule_digest,
        ] {
            digest.text(value);
        }
        digest.usize(self.runner_file_ordinal);
        for image in &self.package_images {
            digest.usize(image.parsed_image_ordinal);
            digest.usize(image.package_file_ordinal);
            digest.text(match image.image_role {
                WindowsPreLeasePackageImageRole::RunnerExecutable => "runner_executable",
                WindowsPreLeasePackageImageRole::LoadableDll => "loadable_dll",
            });
            digest.text(&image.relative_path);
            digest.text(&image.normalized_module_name);
            digest.text(&image.file_identity_digest);
            digest.text(&image.sealed_file_digest);
            digest.u64(image.size_bytes);
            digest.text(&image.machine_kind);
            digest.text(&image.pe_kind);
            digest.text(&image.parser_input_receipt_digest);
        }
        for edge in &self.import_edges {
            digest.usize(edge.edge_ordinal);
            digest.usize(edge.importer_image_ordinal);
            digest.usize(edge.importer_edge_ordinal);
            digest.text(match &edge.import_kind {
                WindowsPreLeaseImportKind::Normal => "normal",
                WindowsPreLeaseImportKind::Delay => "delay",
            });
            digest.text(&edge.normalized_module_name);
            digest.optional_text(edge.imported_symbol_name.as_deref());
            digest.optional_u16(edge.imported_symbol_ordinal);
            digest.usize(edge.descriptor_ordinal);
            digest.usize(edge.thunk_ordinal);
            digest.usize(edge.canonical_merge_ordinal);
            digest.text(&edge.edge_evidence_digest);
        }
        for hop in &self.forwarder_hops {
            digest.usize(hop.edge_ordinal);
            digest.usize(hop.hop_ordinal);
            digest.usize(hop.source_image_ordinal);
            digest.optional_text(hop.source_export_name.as_deref());
            digest.optional_u16(hop.source_export_ordinal);
            digest.text(&hop.target_module_name);
            digest.optional_text(hop.target_symbol_name.as_deref());
            digest.optional_u16(hop.target_symbol_ordinal);
            digest.text(&hop.hop_evidence_digest);
        }
        digest.usize(self.reachable_closure.root_parsed_image_ordinal);
        digest.usize(self.reachable_closure.maximum_forwarder_depth);
        for ordinal in &self.reachable_closure.reachable_image_ordinals {
            digest.usize(*ordinal);
        }
        for leaf in &self.reachable_closure.external_leaf_requests {
            digest.usize(leaf.leaf_ordinal);
            digest.usize(leaf.source_import_edge_ordinal);
            digest.optional_usize(leaf.forwarder_hop_ordinal);
            digest.text(&leaf.normalized_module_name);
            digest.optional_text(leaf.imported_symbol_name.as_deref());
            digest.optional_u16(leaf.imported_symbol_ordinal);
            digest.text(&leaf.leaf_binding_digest);
        }
        digest.text(&self.reachable_closure.cycle_check_receipt_digest);
        digest.text(&self.reachable_closure.reachable_set_digest);
        digest.text(&self.reachable_closure.module_cache_collision_closure_digest);
        digest.text(
            &self
                .reachable_closure
                .authenticated_module_cache_collision_receipt_digest,
        );
        digest.finish()
    }
}

pub(super) fn canonical_merge_rule_digest() -> String {
    let mut digest = MaterialDigest::new(b"ELON_WINDOWS_PE_EDGE_MERGE_RULE_V1");
    digest.text(super::CANONICAL_EDGE_MERGE_RULE);
    digest.finish()
}

pub(super) fn reachable_set_digest(
    root_ordinal: usize,
    ordinals: &[usize],
    external_leaves: &[super::WindowsPreLeaseExternalLeafRequest],
) -> String {
    let mut digest = MaterialDigest::new(b"ELON_WINDOWS_PE_REACHABLE_SET_V1");
    digest.usize(root_ordinal);
    for ordinal in ordinals {
        digest.usize(*ordinal);
    }
    for leaf in external_leaves {
        digest.text(&leaf.leaf_binding_digest);
    }
    digest.finish()
}

pub(super) fn external_leaf_binding_digest(
    leaf: &super::WindowsPreLeaseExternalLeafRequest,
) -> String {
    let mut digest = MaterialDigest::new(b"ELON_WINDOWS_PE_EXTERNAL_LEAF_REQUEST_V1");
    digest.usize(leaf.leaf_ordinal);
    digest.usize(leaf.source_import_edge_ordinal);
    digest.optional_usize(leaf.forwarder_hop_ordinal);
    digest.text(&leaf.normalized_module_name);
    digest.optional_text(leaf.imported_symbol_name.as_deref());
    digest.optional_u16(leaf.imported_symbol_ordinal);
    digest.finish()
}

pub(super) fn module_cache_collision_closure_digest(
    authenticated_preloaded_module_set_digest: &str,
    images: &[super::WindowsPreLeaseParsedPackageImage],
    edges: &[super::WindowsPreLeaseImportEdge],
    hops: &[super::WindowsPreLeaseForwarderHop],
) -> String {
    let mut keys = images
        .iter()
        .map(|image| image.normalized_module_name.as_str())
        .chain(
            edges
                .iter()
                .map(|edge| edge.normalized_module_name.as_str()),
        )
        .chain(hops.iter().map(|hop| hop.target_module_name.as_str()))
        .collect::<Vec<_>>();
    keys.sort_unstable();
    keys.dedup();
    let mut digest = MaterialDigest::new(b"ELON_WINDOWS_PE_MODULE_CACHE_COLLISION_CLOSURE_V1");
    digest.text(authenticated_preloaded_module_set_digest);
    for key in keys {
        digest.text(key);
    }
    digest.finish()
}

struct MaterialDigest(Sha256);

impl MaterialDigest {
    fn new(domain: &[u8]) -> Self {
        let mut value = Sha256::new();
        value.update(domain);
        Self(value)
    }

    fn text(&mut self, value: &str) {
        self.0.update((value.len() as u64).to_le_bytes());
        self.0.update(value.as_bytes());
    }

    fn optional_text(&mut self, value: Option<&str>) {
        match value {
            Some(value) => {
                self.0.update([1]);
                self.text(value);
            }
            None => self.0.update([0]),
        }
    }

    fn optional_u16(&mut self, value: Option<u16>) {
        match value {
            Some(value) => {
                self.0.update([1]);
                self.0.update(value.to_le_bytes());
            }
            None => self.0.update([0]),
        }
    }

    fn optional_usize(&mut self, value: Option<usize>) {
        match value {
            Some(value) => {
                self.0.update([1]);
                self.usize(value);
            }
            None => self.0.update([0]),
        }
    }

    fn usize(&mut self, value: usize) {
        self.0.update((value as u64).to_le_bytes());
    }

    fn u64(&mut self, value: u64) {
        self.0.update(value.to_le_bytes());
    }

    fn finish(self) -> String {
        hex::encode(self.0.finalize())
    }
}
