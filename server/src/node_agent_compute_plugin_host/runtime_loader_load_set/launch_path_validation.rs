use anyhow::{bail, Result};
use serde_json::json;

use crate::node_agent_compute_plugin_host::{
    manifest_validation::is_sha256, signed_artifact_verification::jcs_sha256_hex,
};

use super::{
    model::SealedComputePluginRunnerImage,
    resolution::{
        SealedWindowsLoaderNamespacePrerequisite, SealedWindowsLoaderResolutionAuthority,
        WindowsLoaderLaunchPathComponentBinding, WindowsLoaderLaunchPathGrantCustody,
        WindowsLoaderLaunchPathKind,
    },
};

pub(super) fn validate_launch_path_authority(
    image: &SealedComputePluginRunnerImage,
    resolution: &SealedWindowsLoaderResolutionAuthority,
    namespace: &SealedWindowsLoaderNamespacePrerequisite,
) -> Result<()> {
    let runner = image
        .runner_file()
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_LOADER_RUNNER_ORDINAL_MISSING"))?;
    let working_directory = image
        .working_directory()
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_LOADER_CWD_ORDINAL_MISSING"))?;
    let application_path = runner.handle_path_binding();
    let working_directory_path = working_directory.handle_path_binding();
    validate_path_receipt(
        application_path,
        &image.root_identity_digest,
        &image.file_identity_digest,
        &resolution
            .launch_path_authority
            .application_component_set_digest,
    )?;
    validate_path_receipt(
        working_directory_path,
        &image.root_identity_digest,
        &image.working_directory_identity_digest,
        &resolution
            .launch_path_authority
            .working_directory_component_set_digest,
    )?;

    let share_contract_material = json!([
        {
            "path_kind": "application",
            "retained_parent_chain_share_contract_digest": application_path.4,
            "observation_receipt_digest": application_path.5,
        },
        {
            "path_kind": "working_directory",
            "retained_parent_chain_share_contract_digest": working_directory_path.4,
            "observation_receipt_digest": working_directory_path.5,
        }
    ]);
    if jcs_sha256_hex(&share_contract_material)?
        != resolution
            .launch_path_authority
            .retained_parent_chain_share_contract_set_digest
    {
        bail!("COMPUTE_PLUGIN_LOADER_LAUNCH_PATH_SHARE_CONTRACT_CHANGED");
    }

    if resolution.launch_path_authority.components.len()
        != namespace.launch_path_component_grants.len()
    {
        bail!("COMPUTE_PLUGIN_LOADER_LAUNCH_PATH_GRANT_CARDINALITY_CHANGED");
    }
    validate_path_kind(
        WindowsLoaderLaunchPathKind::Application,
        &resolution.launch_path_authority.components,
        &namespace.launch_path_component_grants,
        &namespace.session,
        application_path.0,
        application_path.1,
    )?;
    validate_path_kind(
        WindowsLoaderLaunchPathKind::WorkingDirectory,
        &resolution.launch_path_authority.components,
        &namespace.launch_path_component_grants,
        &namespace.session,
        working_directory_path.0,
        working_directory_path.1,
    )?;
    Ok(())
}

fn validate_path_receipt(
    binding: (&str, &str, &str, &str, &str, &str),
    expected_root_identity: &str,
    expected_final_identity: &str,
    expected_component_set_digest: &str,
) -> Result<()> {
    if binding.0 != expected_root_identity
        || binding.1 != expected_final_identity
        || binding.3 != expected_component_set_digest
        || [
            binding.0, binding.1, binding.2, binding.3, binding.4, binding.5,
        ]
        .iter()
        .any(|digest| !is_sha256(digest))
    {
        bail!("COMPUTE_PLUGIN_LOADER_HANDLE_PATH_RECEIPT_CHANGED");
    }
    Ok(())
}

fn validate_path_kind(
    kind: WindowsLoaderLaunchPathKind,
    all_components: &[WindowsLoaderLaunchPathComponentBinding],
    all_grants: &[WindowsLoaderLaunchPathGrantCustody],
    session: &crate::node_agent_managed_fs::ManagedLoaderNamespaceSession,
    root_identity: &str,
    final_identity: &str,
) -> Result<()> {
    let components = all_components
        .iter()
        .filter(|entry| entry.path_kind == kind)
        .collect::<Vec<_>>();
    let grants = all_grants
        .iter()
        .filter(|entry| entry.path_kind == kind)
        .collect::<Vec<_>>();
    if components.is_empty() || components.len() != grants.len() {
        bail!("COMPUTE_PLUGIN_LOADER_LAUNCH_PATH_KIND_CARDINALITY_CHANGED");
    }
    let mut expected_parent = root_identity;
    for (ordinal, (component, grant)) in components.iter().zip(grants).enumerate() {
        let (grant_generation, parent, name, disposition, fence_generation) = grant.grant.binding();
        let expected_disposition = jcs_sha256_hex(&json!({
            "kind": "expected_exact_object",
            "identity_digest": component.expected_object_identity_digest,
        }))?;
        if component.component_ordinal != ordinal
            || grant.component_ordinal != ordinal
            || component.parent_directory_identity_digest != expected_parent
            || component.normalized_component.trim().is_empty()
            || !grant.grant.matches_session(session)
            || grant_generation != session.binding().1
            || parent != component.parent_directory_identity_digest
            || name != component.normalized_component
            || disposition != expected_disposition
            || [
                &component.parent_directory_identity_digest,
                &component.expected_object_identity_digest,
                fence_generation,
            ]
            .iter()
            .any(|digest| !is_sha256(digest))
        {
            bail!("COMPUTE_PLUGIN_LOADER_LAUNCH_PATH_COMPONENT_CHANGED");
        }
        expected_parent = &component.expected_object_identity_digest;
    }
    if expected_parent != final_identity {
        bail!("COMPUTE_PLUGIN_LOADER_LAUNCH_PATH_FINAL_IDENTITY_CHANGED");
    }
    Ok(())
}
