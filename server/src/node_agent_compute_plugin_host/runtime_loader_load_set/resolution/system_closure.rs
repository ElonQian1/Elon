//! Post-lease recursive PE closure for images first reached through system resolution.
//!
//! Wave zero remains the package-only preliminary/GrantReady plan. This module freezes the
//! disjoint provenance, signed recursive limits, exact per-producer-wave acquisition custody,
//! source-owner parse receipts and terminal fixpoint required for every later wave. It deliberately
//! provides no signature, parser, resolver, grant, candidate, lease, advancer or sealing producer.

mod acquisition;
mod digest;
mod edge_order;
mod edge_projection;
mod projection_digest;
mod source_projection;
mod validation;

use std::convert::Infallible;

use super::WindowsLoaderModuleNode;

pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) use acquisition::{
    AuthenticatedWindowsRecursiveWaveResolutionPlan,
    SealedWindowsRecursiveResolutionAcquisitionChain, TerminalWindowsRecursiveResolutionCustody,
    WindowsRecursiveResolutionAccumulatedCustody, WindowsRecursiveWaveAcquisitionReceipt,
    WindowsRecursiveWaveAdvanceFailureClass, WindowsRecursiveWaveAdvanceFailureCustody,
    WindowsRecursiveWaveCandidateAcquisitionCustody, WindowsRecursiveWaveCompletedCustody,
    WindowsRecursiveWaveGrantAcquisitionCustody, WindowsRecursiveWaveLeaseAcquisitionCustody,
    WindowsRecursiveWaveRequestCustody, WindowsRecursiveWaveRequestPlan,
    WindowsRecursiveWaveSameOwnerParseCustody,
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) enum WindowsRecursiveImageOwnerRef
{
    PackageContentLease {
        package_file_ordinal: usize,
    },
    AuthenticatedPreloadedModule {
        preloaded_module_ordinal: usize,
    },
    KnownDllSection {
        known_dll_authority_record_ordinal: usize,
    },
    ResolvedFilesystemSystemImage {
        resolution_request_ordinal: usize,
    },
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) enum WindowsPeParsedImageSource
{
    BasePreleasePackage {
        prelease_parsed_image_ordinal: usize,
    },
    RecursiveExpansion {
        parse_receipt_ordinal: usize,
    },
}

pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) struct WindowsPostLeaseSystemImageParseReceipt
{
    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) parse_receipt_ordinal:
        usize,
    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) wave_ordinal: usize,
    /// Index of the producer-wave acquisition receipt that retained the exact owner parsed here.
    /// It is always `wave_ordinal - 1`; the receipt digest cannot be embedded without a digest
    /// cycle because that receipt commits the completed parse-receipt set.
    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) producer_acquisition_receipt_ordinal:
        usize,
    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) producer_module_request_ordinal:
        usize,
    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) parsed_image_ordinal:
        usize,
    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) node:
        WindowsLoaderModuleNode,
    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) source_owner:
        WindowsRecursiveImageOwnerRef,
    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) source_owner_binding_digest:
        String,
    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) image_material_identity_digest:
        String,
    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) parser_policy_digest:
        String,
    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) import_table_digest:
        String,
    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) normal_import_count:
        usize,
    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) delay_import_count:
        usize,
    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) forwarder_count: usize,
    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) same_owner_parse_receipt_digest:
        String,
    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) receipt_digest: String,
}

pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) struct WindowsRecursiveResolutionWavePlan
{
    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) wave_ordinal: usize,
    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) source_parse_receipt_ordinals:
        Vec<usize>,
    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) first_module_request_ordinal:
        usize,
    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) module_request_count:
        usize,
    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) first_searched_name_ordinal:
        usize,
    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) searched_name_count:
        usize,
    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) first_system_image_request_ordinal:
        usize,
    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) system_image_request_count:
        usize,
    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) next_frontier_parse_receipt_ordinals:
        Vec<usize>,
    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) parsed_edge_set_digest:
        String,
    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) searched_name_disposition_set_digest:
        String,
    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) acquired_system_image_set_digest:
        String,
    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) wave_digest: String,
}

/// Source-only final projection envelope. The uninhabited producer keeps the current runtime
/// unable to claim recursive system-image custody or execution from a detached final graph.
pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) struct SealedWindowsRecursiveResolutionClosure
{
    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) base_prelease_parsed_image_count:
        usize,
    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) base_module_request_count:
        usize,
    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) base_searched_name_count:
        usize,
    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) base_system_image_request_count:
        usize,
    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) parse_receipts:
        Vec<WindowsPostLeaseSystemImageParseReceipt>,
    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) waves:
        Vec<WindowsRecursiveResolutionWavePlan>,
    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) acquisition_chain:
        SealedWindowsRecursiveResolutionAcquisitionChain,
    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) file_identity_dedupe_receipt_digest:
        String,
    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) module_cache_collision_closure_receipt_digest:
        String,
    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) forwarder_cycle_closure_receipt_digest:
        String,
    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) terminal_empty_frontier_receipt_digest:
        String,
    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) closure_digest: String,
    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) _recursive_system_import_closure_producer_unavailable:
        Infallible,
}

impl SealedWindowsRecursiveResolutionClosure {
    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) fn digest(
        &self,
    ) -> &str {
        &self.closure_digest
    }
}
