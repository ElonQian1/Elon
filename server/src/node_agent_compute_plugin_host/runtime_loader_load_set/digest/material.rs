use anyhow::Result;
use serde_json::{json, Value};

use crate::node_agent_compute_plugin_host::signed_artifact_verification::jcs_sha256_hex;

use super::super::resolution::{
    WindowsLoaderFilesystemSearchDirectoryTarget, WindowsLoaderImportBindingRef,
    WindowsLoaderImportEdgeKind, WindowsLoaderLaunchPathKind, WindowsLoaderModuleNode,
    WindowsLoaderSearchedNameDisposition, WindowsLoaderSystemResolutionOrigin,
};

pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) fn searched_name_disposition_digest(
    disposition: &WindowsLoaderSearchedNameDisposition,
) -> Result<String> {
    jcs_sha256_hex(&disposition_material(disposition))
}

pub(super) fn disposition_material(disposition: &WindowsLoaderSearchedNameDisposition) -> Value {
    match disposition {
        WindowsLoaderSearchedNameDisposition::ExpectedPackage {
            package_file_ordinal,
            image_file_identity_digest,
        } => json!({
            "kind": "expected_package",
            "package_file_ordinal": package_file_ordinal,
            "image_file_identity_digest": image_file_identity_digest,
        }),
        WindowsLoaderSearchedNameDisposition::ExpectedSystem {
            resolved_component_identity_digest,
            image_file_identity_digest,
            immutable_section_identity_digest,
            servicing_generation_digest,
        } => json!({
            "kind": "expected_system",
            "resolved_component_identity_digest": resolved_component_identity_digest,
            "image_file_identity_digest": image_file_identity_digest,
            "immutable_section_identity_digest": immutable_section_identity_digest,
            "servicing_generation_digest": servicing_generation_digest,
        }),
        WindowsLoaderSearchedNameDisposition::MustRemainAbsent => {
            json!({ "kind": "must_remain_absent" })
        }
        WindowsLoaderSearchedNameDisposition::ShadowedByEarlierName {
            earlier_searched_name_ordinal,
        } => json!({
            "kind": "shadowed_by_earlier_name",
            "earlier_searched_name_ordinal": earlier_searched_name_ordinal,
        }),
    }
}

pub(super) fn search_target_material(
    target: &WindowsLoaderFilesystemSearchDirectoryTarget,
) -> Value {
    match target {
        WindowsLoaderFilesystemSearchDirectoryTarget::PackageRoot => {
            json!({ "kind": "package_root" })
        }
        WindowsLoaderFilesystemSearchDirectoryTarget::PackageWorkingDirectory => {
            json!({ "kind": "package_working_directory" })
        }
        WindowsLoaderFilesystemSearchDirectoryTarget::PackagePlanDirectory {
            directory_ordinal,
        } => json!({
            "kind": "package_plan_directory",
            "directory_ordinal": directory_ordinal,
        }),
        WindowsLoaderFilesystemSearchDirectoryTarget::SystemDirectory { directory } => {
            external_search_directory_material("system_directory", directory)
        }
        WindowsLoaderFilesystemSearchDirectoryTarget::WindowsDirectory { directory } => {
            external_search_directory_material("windows_directory", directory)
        }
        WindowsLoaderFilesystemSearchDirectoryTarget::SideBySideAssemblyDirectory { directory } => {
            external_search_directory_material("side_by_side_assembly_directory", directory)
        }
    }
}

fn external_search_directory_material(
    kind: &str,
    directory: &crate::node_agent_managed_fs::PinnedWindowsLoaderSearchDirectory,
) -> Value {
    let (
        root_identity_digest,
        final_identity_digest,
        canonical_path_digest,
        component_set_digest,
        retained_parent_chain_share_contract_digest,
        observation_receipt_digest,
        namespace_alias_currentness_receipt_digest,
    ) = directory.path_currentness_binding();
    json!({
        "kind": kind,
        "root_identity_digest": root_identity_digest,
        "final_identity_digest": final_identity_digest,
        "canonical_path_digest": canonical_path_digest,
        "component_set_digest": component_set_digest,
        "retained_parent_chain_share_contract_digest": retained_parent_chain_share_contract_digest,
        "observation_receipt_digest": observation_receipt_digest,
        "namespace_alias_currentness_receipt_digest": namespace_alias_currentness_receipt_digest,
    })
}

pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) fn edge_kind_name(
    kind: &WindowsLoaderImportEdgeKind,
) -> &'static str {
    match kind {
        WindowsLoaderImportEdgeKind::NormalImport => "normal_import",
        WindowsLoaderImportEdgeKind::DelayImport => "delay_import",
        WindowsLoaderImportEdgeKind::Forwarder => "forwarder",
    }
}

pub(super) fn launch_path_kind_name(kind: WindowsLoaderLaunchPathKind) -> &'static str {
    match kind {
        WindowsLoaderLaunchPathKind::Application => "application",
        WindowsLoaderLaunchPathKind::WorkingDirectory => "working_directory",
    }
}

pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) fn module_node_material(
    node: &WindowsLoaderModuleNode,
) -> Value {
    match node {
        WindowsLoaderModuleNode::PackageFile {
            package_file_ordinal,
        } => json!({
            "kind": "package_file",
            "package_file_ordinal": package_file_ordinal,
        }),
        WindowsLoaderModuleNode::SystemComponent {
            component_identity_digest,
        } => json!({
            "kind": "system_component",
            "component_identity_digest": component_identity_digest,
        }),
        WindowsLoaderModuleNode::KnownDllSection {
            section_identity_digest,
        } => json!({
            "kind": "known_dll_section",
            "section_identity_digest": section_identity_digest,
        }),
        WindowsLoaderModuleNode::ApiSetHost {
            component_identity_digest,
        } => json!({
            "kind": "api_set_host",
            "component_identity_digest": component_identity_digest,
        }),
        WindowsLoaderModuleNode::SideBySideAssembly {
            assembly_identity_digest,
        } => json!({
            "kind": "side_by_side_assembly",
            "assembly_identity_digest": assembly_identity_digest,
        }),
    }
}

pub(super) fn import_binding_ref_material(binding: &WindowsLoaderImportBindingRef) -> Value {
    match binding {
        WindowsLoaderImportBindingRef::Package { binding_ordinal } => json!({
            "kind": "package",
            "binding_ordinal": binding_ordinal,
        }),
        WindowsLoaderImportBindingRef::System { binding_ordinal } => json!({
            "kind": "system",
            "binding_ordinal": binding_ordinal,
        }),
    }
}

pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) fn system_resolution_origin_material(
    origin: &WindowsLoaderSystemResolutionOrigin,
) -> Value {
    match origin {
        WindowsLoaderSystemResolutionOrigin::Preloaded {
            preloaded_module_ordinal,
        } => json!({
            "kind": "preloaded",
            "preloaded_module_ordinal": preloaded_module_ordinal,
        }),
        WindowsLoaderSystemResolutionOrigin::KnownDll {
            section_identity_digest,
        } => json!({
            "kind": "known_dll",
            "section_identity_digest": section_identity_digest,
        }),
        WindowsLoaderSystemResolutionOrigin::ApiSet {
            normalized_contract_name,
            host_component_identity_digest,
            host_resolution,
        } => json!({
            "kind": "api_set",
            "normalized_contract_name": normalized_contract_name,
            "host_component_identity_digest": host_component_identity_digest,
            "host_resolution": api_set_host_resolution_material(host_resolution),
        }),
        WindowsLoaderSystemResolutionOrigin::SideBySide {
            assembly_identity_digest,
            search_directory_ordinal,
        } => json!({
            "kind": "side_by_side",
            "assembly_identity_digest": assembly_identity_digest,
            "search_directory_ordinal": search_directory_ordinal,
        }),
        WindowsLoaderSystemResolutionOrigin::FilesystemSearch {
            search_directory_ordinal,
        } => json!({
            "kind": "filesystem_search",
            "search_directory_ordinal": search_directory_ordinal,
        }),
    }
}

fn api_set_host_resolution_material(
    resolution: &super::super::resolution::WindowsLoaderApiSetHostResolution,
) -> Value {
    use super::super::resolution::WindowsLoaderApiSetHostResolution;
    match resolution {
        WindowsLoaderApiSetHostResolution::Preloaded {
            preloaded_module_ordinal,
        } => json!({
            "kind": "preloaded",
            "preloaded_module_ordinal": preloaded_module_ordinal,
        }),
        WindowsLoaderApiSetHostResolution::KnownDll {
            section_identity_digest,
        } => json!({
            "kind": "known_dll",
            "section_identity_digest": section_identity_digest,
        }),
        WindowsLoaderApiSetHostResolution::FilesystemSearch {
            search_directory_ordinal,
        } => json!({
            "kind": "filesystem_search",
            "search_directory_ordinal": search_directory_ordinal,
        }),
        WindowsLoaderApiSetHostResolution::SideBySide {
            assembly_identity_digest,
            search_directory_ordinal,
        } => json!({
            "kind": "side_by_side",
            "assembly_identity_digest": assembly_identity_digest,
            "search_directory_ordinal": search_directory_ordinal,
        }),
    }
}

pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) fn filesystem_system_image_ref_material(
    image_ref: Option<&super::super::resolution::WindowsLoaderResolvedFilesystemSystemImageRef>,
) -> Value {
    let Some(image_ref) = image_ref else {
        return Value::Null;
    };
    json!({
        "resolution_request_ordinal": image_ref.resolution_request_ordinal,
    })
}
