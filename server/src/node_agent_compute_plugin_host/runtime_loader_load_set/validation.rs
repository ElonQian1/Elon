use std::path::Path;

use anyhow::{bail, Result};
use serde_json::json;

use crate::node_agent_compute_plugin_host::{
    manifest_validation::is_sha256, signed_artifact_verification::jcs_sha256_hex,
};

use super::{
    digest::{searched_name_disposition_digest, validate_aggregate_digests},
    launch_path_discovery::WindowsRunnerLaunchContextPreCreateProjection,
    launch_path_validation::validate_launch_path_authority,
    model::{LoaderLockedWorkAdmittedPluginSlot, WindowsLoaderWorkingDirectoryLocation},
    namespace_validation::validate_namespace_queries,
    pe_graph_validation::validate_pe_import_graph,
    resolution::{
        SealedWindowsLoaderResolutionAuthority, WindowsLoaderFilesystemSearchDirectoryTarget,
        WindowsLoaderImportBindingRef, WindowsLoaderSearchedNameDisposition,
    },
    system_resolution_validation::{
        canonical_loader_module_basename, module_node_valid, normalized_loader_module_key_valid,
        resolved_filesystem_system_image, system_resolution_origin_valid,
        system_terminal_search_binding, validate_system_dependencies,
    },
};

/// Borrowed launch paths are returned only after structural cross-binding of the retained owner
/// graph against the extraction plan, admission receipts, sealed resolution envelope, and fence
/// map. Full PE closure and OS-policy proof remain obligations of the uninhabited producer.
pub(in crate::node_agent_compute_plugin_host) struct ValidatedLoaderLockedRunnerBinding<'owner> {
    application_path: &'owner Path,
    working_directory_path: &'owner Path,
}

impl ValidatedLoaderLockedRunnerBinding<'_> {
    pub(in crate::node_agent_compute_plugin_host) fn application_path(&self) -> &Path {
        self.application_path
    }

    pub(in crate::node_agent_compute_plugin_host) fn working_directory_path(&self) -> &Path {
        self.working_directory_path
    }
}

impl LoaderLockedWorkAdmittedPluginSlot<'_> {
    pub(in crate::node_agent_compute_plugin_host) fn validate_authenticated_launch_context_projection(
        &self,
        expected: &WindowsRunnerLaunchContextPreCreateProjection<'_>,
    ) -> Result<()> {
        let receipts = &self.authority.work_admission_receipts;
        receipts.validate()?;
        let profile = receipts.source().source().launch_profile();
        profile.validate()?;
        let image = &self.image;
        self.authority
            .authenticated_launch_lineage
            .validate_loader_image_binding(
                receipts.source().source_digest(),
                receipts.receipt().receipt_digest(),
                profile,
                image,
            )?;
        let (selector, machine, resolution_profile, working_directory, runner, arguments) =
            expected.loader_binding();
        if selector != image.launch_context_selector_digest()
            || machine != image.process_machine_context_digest()
            || resolution_profile != image.startup_import_resolution_profile_digest()
            || working_directory != image.working_directory_identity_digest()
            || runner != profile.runner_relative_path()
            || arguments != profile.entrypoint_arguments_digest()
        {
            bail!("COMPUTE_PLUGIN_WINDOWS_LOADER_ACTUAL_CONTEXT_PROJECTION_CHANGED");
        }
        self.authority
            .authenticated_launch_lineage
            .validate_process_projection(profile, expected)
    }

    pub(in crate::node_agent_compute_plugin_host) fn validate_internal_binding(
        &self,
    ) -> Result<ValidatedLoaderLockedRunnerBinding<'_>> {
        let image = &self.image;
        let plan_envelope = self.authority.extraction_plan.envelope();
        let plan = &plan_envelope.plan;
        let extraction_evidence = &self.authority.extraction_evidence.evidence;
        let resolution = &image.load_set_authority.resolution;
        let namespace = &image.load_set_authority.namespace;
        let namespace_prerequisite = &namespace.prerequisite;
        let receipts = &self.authority.work_admission_receipts;
        let launch_profile = receipts.source().source().launch_profile();

        receipts.validate()?;
        if image.installation_id_digest != self.authority.staging_root.installation_id_digest()
            || image.root_identity_digest != self.authority.staging_root.root_identity_digest()
            || image.release != plan.release
            || image.plugin_id != plan.release.plugin_id
            || plan_envelope.plan_digest != resolution.extraction_plan_digest
            || self.authority.extraction_evidence.evidence_digest
                != resolution.extraction_evidence_digest
            || receipts.source().source_digest() != resolution.admission_source_digest
            || receipts.receipt().receipt_digest() != resolution.admission_receipt_digest
            || resolution.resolution_profile_digest
                != namespace_prerequisite.resolution_profile_digest
            || resolution.working_directory_identity_digest
                != image.working_directory_identity_digest
            || resolution.signed_system_dependencies.manifest_digest
                != launch_profile.manifest_digest()
            || resolution
                .signed_system_dependencies
                .signed_manifest_envelope_digest
                != launch_profile.signed_manifest_envelope_digest()
            || extraction_evidence.installation_id_digest != image.installation_id_digest
            || extraction_evidence.root_identity_digest != image.root_identity_digest
            || extraction_evidence.staging_run_digest != self.authority.staging_run_digest
            || extraction_evidence.extraction_plan_digest != plan_envelope.plan_digest
        {
            bail!("COMPUTE_PLUGIN_LOADER_AUTHORITY_BINDING_CHANGED");
        }

        if image.package_files.len() != plan.files.len()
            || extraction_evidence.files.len() != plan.files.len()
            || image.namespace_directories.len() != plan.directories.len()
            || image.runner_ordinal >= plan.files.len()
        {
            bail!("COMPUTE_PLUGIN_LOADER_ORDINAL_CARDINALITY_CHANGED");
        }
        for (ordinal, entry) in image.package_files.iter().enumerate() {
            let expected = &plan.files[ordinal];
            let evidence = &extraction_evidence.files[ordinal];
            let expected_size = u64::try_from(expected.expected_size_bytes)?;
            let expected_loader_executable = expected.executable
                || ordinal == image.runner_ordinal
                || resolution
                    .package_module_bindings
                    .iter()
                    .any(|binding| binding.resolved_package_file_ordinal == ordinal);
            if entry.package_file_ordinal != ordinal
                || entry.relative_path != expected.relative_path
                || evidence.relative_path != expected.relative_path
                || evidence.digest != expected.expected_digest
                || evidence.size_bytes != expected.expected_size_bytes
                || !entry.file.matches_plan_file(
                    &expected.expected_digest,
                    expected_size,
                    expected_loader_executable,
                    &expected.relative_path,
                    &image.root_identity_digest,
                    &evidence.file_identity_digest,
                )
            {
                bail!("COMPUTE_PLUGIN_LOADER_PACKAGE_FILE_BINDING_CHANGED");
            }
        }
        let package_content_lease_material = image
            .package_files
            .iter()
            .map(|entry| {
                let (file_identity, sealed_digest, generation, policy) =
                    entry.file.content_lease_binding();
                json!({
                    "package_file_ordinal": entry.package_file_ordinal,
                    "relative_path": entry.relative_path,
                    "file_identity_digest": file_identity,
                    "sealed_digest": sealed_digest,
                    "lease_generation_digest": generation,
                    "immutable_content_policy_digest": policy,
                })
            })
            .collect::<Vec<_>>();
        if image
            .package_files
            .iter()
            .flat_map(|entry| {
                let binding = entry.file.content_lease_binding();
                [binding.0, binding.1, binding.2, binding.3]
            })
            .any(|digest| !is_sha256(digest))
            || jcs_sha256_hex(&package_content_lease_material)?
                != resolution.package_content_lease_set_digest
        {
            bail!("COMPUTE_PLUGIN_LOADER_PACKAGE_CONTENT_LEASE_SET_CHANGED");
        }
        let system_content_lease_material = resolution
            .resolved_filesystem_system_images
            .iter()
            .map(|custody| {
                let (request, candidate, session, lease_request, nonce, response, receipt) =
                    custody.outcome.binding();
                let (parent, name, file, section, open_receipt, mapping_receipt) =
                    custody.outcome.image().binding();
                let (_, _, servicing, generation, policy) =
                    custody.outcome.image().content_lease_binding();
                json!({
                    "resolution_request_ordinal": custody.resolution_request_ordinal,
                    "outcome_request_ordinal": request,
                    "candidate_binding_digest": candidate,
                    "lease_session_identity_digest": session,
                    "lease_request_digest": lease_request,
                    "query_nonce_digest": nonce,
                    "authenticated_response_digest": response,
                    "positive_receipt_digest": receipt,
                    "parent_directory_identity_digest": parent,
                    "normalized_name": name,
                    "image_file_identity_digest": file,
                    "immutable_section_identity_digest": section,
                    "parent_relative_open_receipt_digest": open_receipt,
                    "section_mapping_receipt_digest": mapping_receipt,
                    "servicing_generation_digest": servicing,
                    "lease_generation_digest": generation,
                    "immutable_content_policy_digest": policy,
                })
            })
            .collect::<Vec<_>>();
        let system_owner_graph_invalid = resolution
            .resolved_filesystem_system_images
            .iter()
            .enumerate()
            .any(|(ordinal, custody)| {
                let owner_binding = custody.outcome.image().binding();
                let lease_binding = custody.outcome.image().content_lease_binding();
                custody.resolution_request_ordinal != ordinal
                    || owner_binding.2 != lease_binding.0
                    || owner_binding.3 != lease_binding.1
                    || resolution.resolved_filesystem_system_images[..ordinal]
                        .iter()
                        .any(|prior| prior.outcome.image().binding().2 == owner_binding.2)
                    || !resolution.system_module_bindings.iter().any(|binding| {
                        binding
                            .filesystem_image_ref
                            .as_ref()
                            .is_some_and(|image_ref| {
                                image_ref.resolution_request_ordinal == ordinal
                            })
                    })
                    || [
                        owner_binding.0,
                        owner_binding.2,
                        owner_binding.3,
                        owner_binding.4,
                        owner_binding.5,
                        lease_binding.2,
                        lease_binding.3,
                        lease_binding.4,
                    ]
                    .into_iter()
                    .any(|digest| !is_sha256(digest))
            })
            || resolution.system_module_bindings.iter().any(|binding| {
                binding
                    .filesystem_image_ref
                    .as_ref()
                    .is_some_and(|image_ref| {
                        resolution
                            .resolved_filesystem_system_images
                            .get(image_ref.resolution_request_ordinal)
                            .is_none_or(|custody| {
                                custody.resolution_request_ordinal
                                    != image_ref.resolution_request_ordinal
                            })
                    })
            });
        if system_owner_graph_invalid
            || jcs_sha256_hex(&system_content_lease_material)?
                != resolution.system_content_lease_set_digest
        {
            bail!("COMPUTE_PLUGIN_LOADER_SYSTEM_CONTENT_LEASE_SET_CHANGED");
        }
        let immutable_content_lease_set_digest = jcs_sha256_hex(&json!({
            "schema": "elon.compute_plugin.windows_immutable_content_lease_set.v1",
            "package_content_lease_set_digest": resolution.package_content_lease_set_digest,
            "system_content_lease_set_digest": resolution.system_content_lease_set_digest,
        }))?;
        if immutable_content_lease_set_digest != resolution.immutable_content_lease_set_digest {
            bail!("COMPUTE_PLUGIN_LOADER_IMMUTABLE_CONTENT_LEASE_SET_CHANGED");
        }
        for (ordinal, entry) in image.namespace_directories.iter().enumerate() {
            let expected_managed_relative_path = format!(
                "{}/{}",
                self.authority.staging_relative_root, plan.directories[ordinal]
            );
            if entry.directory_ordinal != ordinal
                || entry.relative_path != plan.directories[ordinal]
                || !entry
                    .directory
                    .matches_root_identity(&image.root_identity_digest)
                || !entry
                    .directory
                    .matches_managed_relative_path(&expected_managed_relative_path)
            {
                bail!("COMPUTE_PLUGIN_LOADER_DIRECTORY_BINDING_CHANGED");
            }
        }
        if !image
            .package_root_directory
            .matches_root_identity(&image.root_identity_digest)
            || !image
                .package_root_directory
                .matches_managed_relative_path(&self.authority.staging_relative_root)
        {
            bail!("COMPUTE_PLUGIN_LOADER_PACKAGE_ROOT_BINDING_CHANGED");
        }

        let expected_working_directory_relative_path = match image.working_directory_location {
            WindowsLoaderWorkingDirectoryLocation::PackageRoot => ".",
            WindowsLoaderWorkingDirectoryLocation::PlanDirectory { directory_ordinal } => image
                .namespace_directories
                .get(directory_ordinal)
                .filter(|entry| entry.directory_ordinal == directory_ordinal)
                .map(|entry| entry.relative_path.as_str())
                .ok_or_else(|| {
                    anyhow::anyhow!("COMPUTE_PLUGIN_LOADER_WORKING_DIRECTORY_ORDINAL_CHANGED")
                })?,
        };
        let runner_plan_entry = &plan.files[image.runner_ordinal];
        let runner_evidence = &extraction_evidence.files[image.runner_ordinal];
        if resolution.working_directory_relative_path != expected_working_directory_relative_path
            || resolution.runner_relative_path != image.relative_path
            || launch_profile.runner_relative_path() != image.relative_path
            || runner_plan_entry.relative_path != image.relative_path
            || runner_evidence.relative_path != image.relative_path
            || !runner_plan_entry.executable
            || !image.retained_runner_matches()
            || !image.retained_working_directory_matches()
            || image.file_identity_digest
                != extraction_evidence.files[image.runner_ordinal].file_identity_digest
        {
            bail!("COMPUTE_PLUGIN_LOADER_RUNNER_OR_CWD_BINDING_CHANGED");
        }

        validate_resolution_bindings(image, plan.files.len())?;
        let application_path = image
            .application_path()
            .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_LOADER_RUNNER_ORDINAL_MISSING"))?;
        let working_directory_path = image
            .working_directory_path()
            .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_LOADER_CWD_ORDINAL_MISSING"))?;
        if !application_path.is_absolute() || !working_directory_path.is_absolute() {
            bail!("COMPUTE_PLUGIN_LOADER_HANDLE_PATH_NOT_ABSOLUTE");
        }
        Ok(ValidatedLoaderLockedRunnerBinding {
            application_path,
            working_directory_path,
        })
    }
}

fn validate_resolution_bindings(
    image: &super::model::SealedComputePluginRunnerImage,
    package_file_count: usize,
) -> Result<()> {
    let resolution = &image.load_set_authority.resolution;
    let namespace = &image.load_set_authority.namespace;
    let namespace_prerequisite = &namespace.prerequisite;
    for (ordinal, directory) in resolution.search_directories.iter().enumerate() {
        let (retained_directory, external_binding_valid) = match &directory.target {
            WindowsLoaderFilesystemSearchDirectoryTarget::PackageRoot => {
                (Some(&image.package_root_directory), true)
            }
            WindowsLoaderFilesystemSearchDirectoryTarget::PackageWorkingDirectory => {
                (image.working_directory(), true)
            }
            WindowsLoaderFilesystemSearchDirectoryTarget::PackagePlanDirectory {
                directory_ordinal,
            } => (
                image
                    .namespace_directories
                    .get(*directory_ordinal)
                    .filter(|entry| entry.directory_ordinal == *directory_ordinal)
                    .map(|entry| &entry.directory),
                true,
            ),
            WindowsLoaderFilesystemSearchDirectoryTarget::SystemDirectory {
                directory: external_directory,
            }
            | WindowsLoaderFilesystemSearchDirectoryTarget::WindowsDirectory {
                directory: external_directory,
            }
            | WindowsLoaderFilesystemSearchDirectoryTarget::SideBySideAssemblyDirectory {
                directory: external_directory,
            } => (
                None,
                external_directory.matches_handle_binding(
                    directory.canonical_path.as_path(),
                    &directory.canonical_path_digest,
                    &directory.directory_identity_digest,
                ),
            ),
        };
        let retained_binding_valid = match retained_directory {
            Some(retained) => {
                retained.handle_derived_canonical_path() == directory.canonical_path.as_path()
                    && retained.canonical_path_digest() == directory.canonical_path_digest
                    && retained.matches_sealed_identity(&directory.directory_identity_digest)
                    && retained.matches_root_identity(&image.root_identity_digest)
            }
            None => true,
        };
        if directory.search_directory_ordinal != ordinal
            || !directory.canonical_path.is_absolute()
            || !is_sha256(&directory.canonical_path_digest)
            || !is_sha256(&directory.directory_identity_digest)
            || !is_sha256(&directory.policy_source_digest)
            || !retained_binding_valid
            || !external_binding_valid
        {
            bail!("COMPUTE_PLUGIN_LOADER_SEARCH_DIRECTORY_BINDING_CHANGED");
        }
    }
    for binding in &resolution.package_module_bindings {
        let Some(target) = image
            .package_files
            .get(binding.resolved_package_file_ordinal)
        else {
            bail!("COMPUTE_PLUGIN_LOADER_PACKAGE_MODULE_TARGET_MISSING");
        };
        let expected_search_parent = resolution
            .search_directories
            .get(binding.resolved_search_directory_ordinal)
            .map(|directory| directory.directory_identity_digest.as_str());
        let expected_cache_key = canonical_loader_module_basename(&binding.relative_path);
        let target_search_binding_valid =
            target
                .file
                .loader_search_binding()
                .is_some_and(|(parent, name)| {
                    Some(parent) == expected_search_parent
                        && normalized_loader_module_key_valid(&name.to_ascii_lowercase())
                        && name.eq_ignore_ascii_case(&binding.normalized_import_name)
                });
        if !module_node_valid(&binding.importer, resolution, package_file_count)
            || !normalized_loader_module_key_valid(&binding.normalized_import_name)
            || !normalized_loader_module_key_valid(&binding.resolved_module_cache_key)
            || expected_cache_key.as_deref() != Some(binding.resolved_module_cache_key.as_str())
            || target.package_file_ordinal != binding.resolved_package_file_ordinal
            || target.relative_path != binding.relative_path
            || binding.resolved_search_directory_ordinal >= resolution.search_directories.len()
            || !target_search_binding_valid
            || binding.digest
                != image.package_files[binding.resolved_package_file_ordinal]
                    .file
                    .digest_for_binding()
        {
            bail!("COMPUTE_PLUGIN_LOADER_PACKAGE_MODULE_BINDING_CHANGED");
        }
    }
    for binding in &resolution.system_module_bindings {
        let resolved_dependency = resolution
            .resolved_system_dependencies
            .get(binding.resolved_dependency_ordinal);
        let resolved_binding_valid = match resolved_dependency {
            Some(dependency) => dependency
                .component_identity_digests
                .contains(&binding.resolved_component_identity_digest),
            None => false,
        };
        let image_binding_valid =
            resolution
                .system_module_images
                .component_images
                .iter()
                .any(|image| {
                    image.component_identity_digest == binding.resolved_component_identity_digest
                        && image.immutable_section_identity_digest
                            == binding.resolved_image_section_identity_digest
                });
        if !module_node_valid(&binding.importer, resolution, package_file_count)
            || !normalized_loader_module_key_valid(&binding.normalized_import_name)
            || !normalized_loader_module_key_valid(&binding.resolved_module_cache_key)
            || !is_sha256(&binding.resolved_component_identity_digest)
            || !is_sha256(&binding.resolved_image_section_identity_digest)
            || !resolved_binding_valid
            || !image_binding_valid
            || !system_resolution_origin_valid(binding, resolution)
        {
            bail!("COMPUTE_PLUGIN_LOADER_SYSTEM_MODULE_BINDING_CHANGED");
        }
    }
    validate_system_dependencies(resolution)?;
    if resolution.searched_names.len() != namespace_prerequisite.searched_name_grants.len() {
        bail!("COMPUTE_PLUGIN_LOADER_SEARCHED_NAME_FENCE_CARDINALITY_CHANGED");
    }
    for (ordinal, searched) in resolution.searched_names.iter().enumerate() {
        let fence = &namespace_prerequisite.searched_name_grants[ordinal];
        let linked_import_valid = match &searched.import_binding {
            WindowsLoaderImportBindingRef::Package { binding_ordinal } => resolution
                .package_module_bindings
                .get(*binding_ordinal)
                .is_some_and(|binding| {
                    binding.normalized_import_name == searched.normalized_name
                        && match &searched.disposition {
                            WindowsLoaderSearchedNameDisposition::ExpectedPackage {
                                package_file_ordinal,
                                image_file_identity_digest,
                            } => {
                                *package_file_ordinal == binding.resolved_package_file_ordinal
                                    && searched.search_directory_ordinal
                                        == binding.resolved_search_directory_ordinal
                                    && package_present_disposition_matches(
                                        image,
                                        binding,
                                        *package_file_ordinal,
                                        image_file_identity_digest,
                                    )
                            }
                            WindowsLoaderSearchedNameDisposition::MustRemainAbsent
                            | WindowsLoaderSearchedNameDisposition::ShadowedByEarlierName { .. } => true,
                            WindowsLoaderSearchedNameDisposition::ExpectedSystem { .. } => false,
                        }
                }),
            WindowsLoaderImportBindingRef::System { binding_ordinal } => resolution
                .system_module_bindings
                .get(*binding_ordinal)
                .is_some_and(|binding| {
                    system_terminal_search_binding(binding).is_some_and(
                        |(terminal_directory_ordinal, terminal_name)| {
                            terminal_name == searched.normalized_name
                        && match &searched.disposition {
                            WindowsLoaderSearchedNameDisposition::ExpectedSystem {
                                resolved_component_identity_digest,
                                image_file_identity_digest,
                                immutable_section_identity_digest,
                                servicing_generation_digest,
                            } => {
                                resolved_component_identity_digest
                                    == &binding.resolved_component_identity_digest
                                    && binding.resolved_search_directory_ordinal
                                        == Some(searched.search_directory_ordinal)
                                    && terminal_directory_ordinal
                                        == searched.search_directory_ordinal
                                    && system_present_disposition_matches(
                                        resolution,
                                        binding,
                                        image_file_identity_digest,
                                        immutable_section_identity_digest,
                                        servicing_generation_digest,
                                    )
                            }
                            WindowsLoaderSearchedNameDisposition::MustRemainAbsent
                            | WindowsLoaderSearchedNameDisposition::ShadowedByEarlierName { .. } => true,
                            WindowsLoaderSearchedNameDisposition::ExpectedPackage { .. } => false,
                        }
                        },
                    )
                }),
        };
        let disposition_valid = match &searched.disposition {
            WindowsLoaderSearchedNameDisposition::ExpectedPackage {
                package_file_ordinal,
                image_file_identity_digest,
            } => {
                *package_file_ordinal < package_file_count
                    && resolution.package_module_bindings.iter().any(|binding| {
                        binding.normalized_import_name.as_str() == searched.normalized_name.as_str()
                            && binding.resolved_package_file_ordinal == *package_file_ordinal
                            && package_present_disposition_matches(
                                image,
                                binding,
                                *package_file_ordinal,
                                image_file_identity_digest,
                            )
                    })
            }
            WindowsLoaderSearchedNameDisposition::ExpectedSystem {
                resolved_component_identity_digest,
                image_file_identity_digest,
                immutable_section_identity_digest,
                servicing_generation_digest,
            } => {
                is_sha256(resolved_component_identity_digest)
                    && resolution.system_module_bindings.iter().any(|binding| {
                        binding.resolved_component_identity_digest
                            == resolved_component_identity_digest.as_str()
                            && system_terminal_search_binding(binding).is_some_and(
                                |(terminal_directory_ordinal, terminal_name)| {
                                    terminal_directory_ordinal == searched.search_directory_ordinal
                                        && terminal_name == searched.normalized_name
                                },
                            )
                            && system_present_disposition_matches(
                                resolution,
                                binding,
                                image_file_identity_digest,
                                immutable_section_identity_digest,
                                servicing_generation_digest,
                            )
                    })
            }
            WindowsLoaderSearchedNameDisposition::MustRemainAbsent => true,
            WindowsLoaderSearchedNameDisposition::ShadowedByEarlierName {
                earlier_searched_name_ordinal: _,
            } => false,
        };
        let expected_parent_identity = resolution
            .search_directories
            .get(searched.search_directory_ordinal)
            .map(|directory| directory.directory_identity_digest.as_str());
        let expected_disposition_digest = searched_name_disposition_digest(&searched.disposition)?;
        let expected_search_step_ordinal = resolution.searched_names[..ordinal]
            .iter()
            .filter(|prior| same_import_binding(&prior.import_binding, &searched.import_binding))
            .count();
        let (grant_generation, parent_identity, normalized_name, disposition_digest, fence_digest) =
            fence.grant.binding();
        let (grant_request, _, _, _) = fence.grant.authenticated_positive_binding();
        if searched.searched_name_ordinal != ordinal
            || !normalized_loader_module_key_valid(&searched.normalized_name)
            || searched.search_directory_ordinal >= resolution.search_directories.len()
            || searched.search_step_ordinal != expected_search_step_ordinal
            || fence.searched_name_ordinal != ordinal
            || fence.search_directory_ordinal != searched.search_directory_ordinal
            || !fence.grant.matches_session(&namespace_prerequisite.session)
            || grant_generation != namespace_prerequisite.session.binding().1
            || Some(parent_identity) != expected_parent_identity
            || normalized_name != searched.normalized_name
            || disposition_digest != expected_disposition_digest
            || !is_sha256(fence_digest)
            || !fence.grant.authenticated_positive_is_bound()
            || grant_request != searched.grant_request_digest
            || !disposition_valid
            || !linked_import_valid
        {
            bail!("COMPUTE_PLUGIN_LOADER_SEARCHED_NAME_BINDING_CHANGED");
        }
    }
    validate_launch_path_authority(image, resolution, namespace_prerequisite)?;
    for digest in [
        &resolution.known_dll_authority.os_build_identity_digest,
        &resolution
            .known_dll_authority
            .object_manager_directory_identity_digest,
        &resolution
            .known_dll_authority
            .section_namespace_generation_digest,
        &resolution.known_dll_authority.section_binding_set_digest,
        &resolution.api_set_authority.os_build_identity_digest,
        &resolution.api_set_authority.schema_identity_digest,
        &resolution
            .api_set_authority
            .contract_host_binding_set_digest,
        &resolution
            .side_by_side_authority
            .activation_context_identity_digest,
        &resolution.side_by_side_authority.manifest_set_digest,
        &resolution
            .side_by_side_authority
            .assembly_binding_set_digest,
        &resolution.launch_context_selector_digest,
        &resolution.selected_context_binding_digest,
        &resolution.preliminary_resolution_request_plan_digest,
        &resolution.grant_ready_resolution_plan_digest,
        &resolution.process_machine_context_digest,
        &resolution.system_module_images.component_image_set_digest,
        &resolution.package_content_lease_set_digest,
        &resolution.system_content_lease_set_digest,
        &resolution.immutable_content_lease_set_digest,
        &resolution
            .preloaded_module_authority
            .process_machine_context_digest,
        &resolution.preloaded_module_authority.module_set_digest,
        &resolution.launch_path_authority.component_set_digest,
        &resolution
            .launch_path_authority
            .application_component_set_digest,
        &resolution
            .launch_path_authority
            .working_directory_component_set_digest,
        &resolution
            .launch_path_authority
            .retained_parent_chain_share_contract_set_digest,
        &namespace_prerequisite.searched_name_set_digest,
        &namespace_prerequisite.fence_generation_set_digest,
        &namespace.namespace_authority_digest,
        &resolution.pe_import_graph.parsed_image_set_digest,
        &resolution.pe_import_graph.import_edge_set_digest,
        &resolution.pe_import_graph.reachable_node_set_digest,
        &resolution.pe_import_graph.search_sequence_set_digest,
    ] {
        if !is_sha256(digest) {
            bail!("COMPUTE_PLUGIN_LOADER_RESOLUTION_DIGEST_INVALID");
        }
    }
    if resolution.known_dll_authority.os_build_identity_digest
        != resolution.api_set_authority.os_build_identity_digest
    {
        bail!("COMPUTE_PLUGIN_LOADER_OS_BUILD_AUTHORITY_CHANGED");
    }
    if resolution
        .preloaded_module_authority
        .process_machine_context_digest
        != resolution.process_machine_context_digest
    {
        bail!("COMPUTE_PLUGIN_LOADER_PRELOADED_PROCESS_CONTEXT_CHANGED");
    }
    validate_pe_import_graph(image, resolution)?;
    let (session_identity_digest, _, generation_domain_digest) =
        namespace_prerequisite.session.binding();
    if !is_sha256(session_identity_digest) || !is_sha256(generation_domain_digest) {
        bail!("COMPUTE_PLUGIN_LOADER_NAMESPACE_SESSION_BINDING_INVALID");
    }
    validate_namespace_queries(
        namespace_prerequisite,
        namespace,
        &resolution.immutable_content_lease_set_digest,
    )?;
    validate_aggregate_digests(resolution, namespace)?;
    Ok(())
}

fn same_import_binding(
    left: &WindowsLoaderImportBindingRef,
    right: &WindowsLoaderImportBindingRef,
) -> bool {
    match (left, right) {
        (
            WindowsLoaderImportBindingRef::Package {
                binding_ordinal: left,
            },
            WindowsLoaderImportBindingRef::Package {
                binding_ordinal: right,
            },
        )
        | (
            WindowsLoaderImportBindingRef::System {
                binding_ordinal: left,
            },
            WindowsLoaderImportBindingRef::System {
                binding_ordinal: right,
            },
        ) => left == right,
        _ => false,
    }
}

fn package_present_disposition_matches(
    image: &super::model::SealedComputePluginRunnerImage,
    binding: &super::resolution::WindowsLoaderPackageModuleBinding,
    package_file_ordinal: usize,
    image_file_identity_digest: &str,
) -> bool {
    image
        .package_files
        .get(package_file_ordinal)
        .filter(|target| target.package_file_ordinal == binding.resolved_package_file_ordinal)
        .is_some_and(|target| {
            let (file_identity, _, _, _) = target.file.content_lease_binding();
            file_identity == image_file_identity_digest && is_sha256(image_file_identity_digest)
        })
}

fn system_present_disposition_matches(
    resolution: &SealedWindowsLoaderResolutionAuthority,
    binding: &super::resolution::WindowsLoaderSystemModuleBinding,
    image_file_identity_digest: &str,
    immutable_section_identity_digest: &str,
    servicing_generation_digest: &str,
) -> bool {
    resolved_filesystem_system_image(binding, resolution).is_some_and(|file| {
        let (file_identity, section_identity, retained_servicing_generation, _, _) =
            file.content_lease_binding();
        file_identity == image_file_identity_digest
            && section_identity == immutable_section_identity_digest
            && retained_servicing_generation == servicing_generation_digest
            && section_identity == binding.resolved_image_section_identity_digest
            && is_sha256(image_file_identity_digest)
            && is_sha256(immutable_section_identity_digest)
            && is_sha256(servicing_generation_digest)
    })
}
