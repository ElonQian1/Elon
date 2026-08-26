const HOST_MODULE: &str = include_str!("mod.rs");
const LOADER_MODEL: &str = include_str!("runtime_loader_load_set/model.rs");
const LOADER_RESOLUTION: &str = include_str!("runtime_loader_load_set/resolution.rs");
const LOADER_VALIDATION: &str = include_str!("runtime_loader_load_set/validation.rs");
const EXACT_CONTEXT_LINEAGE: &str =
    include_str!("runtime_loader_load_set/launch_path_discovery/exact_context_plan/lineage.rs");
const EXACT_CONTEXT_INTENT: &str =
    include_str!("runtime_loader_load_set/launch_path_discovery/exact_context_plan/intent.rs");
const MANAGED_LOADER: &str = include_str!("../node_agent_managed_fs/loader.rs");
const FACADE: &str = include_str!("runtime_process_custody.rs");
const MODEL: &str = include_str!("runtime_process_custody/model.rs");
const LOADER_CURRENTNESS: &str = include_str!("runtime_process_custody/namespace_query.rs");
const POLICY: &str = include_str!("runtime_process_custody/policy.rs");
const ENCODING: &str = include_str!("runtime_process_custody/encoding.rs");
const LAUNCH_SECURITY: &str = include_str!("runtime_process_custody/launch_security.rs");
const WINDOWS_JOB: &str = include_str!("runtime_process_custody/windows_job.rs");
const WINDOWS: &str = include_str!("runtime_process_custody/windows.rs");
const WINDOWS_ROLLBACK: &str = include_str!("runtime_process_custody/windows_rollback.rs");
const SERVER_CARGO: &str = include_str!("../../Cargo.toml");

fn source_slice() -> String {
    [
        FACADE,
        LOADER_MODEL,
        LOADER_RESOLUTION,
        LOADER_VALIDATION,
        EXACT_CONTEXT_LINEAGE,
        EXACT_CONTEXT_INTENT,
        MANAGED_LOADER,
        MODEL,
        LOADER_CURRENTNESS,
        POLICY,
        ENCODING,
        LAUNCH_SECURITY,
        WINDOWS_JOB,
        WINDOWS,
        WINDOWS_ROLLBACK,
    ]
    .join("\n")
}

#[test]
fn source_routes_eight_private_windows_process_custody_modules() {
    assert!(HOST_MODULE.contains("mod runtime_process_custody;"));
    assert!(HOST_MODULE.contains("mod runtime_process_custody_source_contract_tests;"));
    for module in [
        "mod encoding;",
        "mod launch_security;",
        "mod model;",
        "mod namespace_query;",
        "mod policy;",
        "mod windows;",
        "mod windows_job;",
        "mod windows_rollback;",
    ] {
        assert!(FACADE.contains(module), "missing module {module}");
    }
    assert_eq!(FACADE.matches("mod ").count(), 8);
    assert!(SERVER_CARGO.contains("\"Win32_Security\""));
    assert!(SERVER_CARGO.contains("\"Win32_System_JobObjects\""));
    assert!(SERVER_CARGO.contains("\"Win32_System_StationsAndDesktops\""));
}

#[test]
fn sealed_launch_authorities_have_no_producer_clone_or_scalar_escape() {
    let combined = source_slice();
    assert!(LOADER_MODEL.contains("struct SealedComputePluginRunnerImage"));
    assert!(LAUNCH_SECURITY.contains("struct SealedWindowsRunnerLaunchSecurity"));
    assert!(MODEL.contains("WindowsRunnerProcessPreparationValidationFailure<'root>"));
    assert_eq!(combined.matches("from_sealed_authorities(").count(), 1);
    assert_eq!(
        combined
            .matches("prepare_suspended_windows_runner_process")
            .count(),
        1
    );
    assert!(!FACADE.contains("prepare_suspended_windows_runner_process"));
    assert!(LOADER_MODEL.contains("load_set_authority: SealedWindowsRunnerLoadSetAuthority"));
    assert!(LOADER_MODEL.contains("package_files: Vec<WindowsLoaderPackageFileCustody>"));
    assert!(LOADER_MODEL.contains("package_root_directory: PinnedManagedLoaderDirectory"));
    assert!(
        LOADER_MODEL.contains("working_directory_location: WindowsLoaderWorkingDirectoryLocation")
    );
    assert!(LOADER_RESOLUTION
        .contains("package_content_leases: Vec<WindowsLoaderPackageContentLeaseCustody>"));
    assert!(MANAGED_LOADER.contains("struct ManagedLoaderFileContentLease"));
    assert!(MANAGED_LOADER.contains("struct ManagedLoaderFileReopenReceipt"));
    assert!(MANAGED_LOADER.contains("struct PinnedManagedLoaderFile"));
    assert!(!combined.contains("transition_admitted_runner_to_loader_load_set"));
    assert!(!MODEL.contains("executable: File"));
    assert!(!MODEL.contains("loader_dependency_files: Vec<File>"));
    assert!(!combined.contains("pub(crate) fn files"));
    assert!(!combined.contains("try_clone"));
    assert!(MANAGED_LOADER.contains("handle_derived_canonical_path"));
}

#[test]
fn preparation_consumes_linear_authorities_and_typed_failure_keeps_both_owners() {
    assert!(LOADER_MODEL.contains("struct LoaderLockedWorkAdmittedPluginSlot<'root>"));
    assert!(LOADER_MODEL.contains("authority: LoaderTransitionAuthorityCustody<'root>"));
    assert!(LOADER_MODEL.contains("authenticated_launch_lineage:"));
    assert!(LOADER_MODEL.contains("image: SealedComputePluginRunnerImage"));
    assert!(MODEL.contains("loader_locked: LoaderLockedWorkAdmittedPluginSlot<'root>"));
    assert!(!MODEL.contains("DurableWorkAdmittedPluginSlot"));
    assert!(!MODEL.contains("runner_image: SealedComputePluginRunnerImage"));
    assert!(MODEL.contains("launch_security: SealedWindowsRunnerLaunchSecurity"));
    assert!(MODEL.contains("application_path: PathBuf"));
    assert!(MODEL.contains("working_directory_path: PathBuf"));
    assert!(MODEL.contains("loader_currentness:"));
    assert!(MODEL.contains("job: OwnedHandle"));
    assert!(MODEL.contains("process: OwnedHandle"));
    assert!(MODEL.contains("primary_thread: OwnedHandle"));
    assert!(LOADER_MODEL.contains("_staging_root_lock_lease: ComputePluginRootLockLease"));
    assert!(!MODEL.contains("#[derive(Clone"));
    assert!(!MODEL.contains("Serialize, Deserialize"));
    assert!(LOADER_VALIDATION.contains("fn validate_internal_binding"));
    assert!(MODEL.contains("_loader_locked: LoaderLockedWorkAdmittedPluginSlot<'root>"));
    assert!(MODEL.contains("_launch_security: SealedWindowsRunnerLaunchSecurity"));
    assert!(MODEL.contains(
        "std::result::Result<Self, WindowsRunnerProcessPreparationValidationFailure<'root>>"
    ));
}

#[test]
fn private_desktop_custody_is_bound_to_startup_info_and_live_process() {
    assert!(LAUNCH_SECURITY.contains("private_desktop: SealedWindowsRunnerPrivateDesktopCustody"));
    assert!(LAUNCH_SECURITY.contains("struct SealedWindowsRunnerPrivateDesktopCustody"));
    assert!(LAUNCH_SECURITY.contains("struct SealedWindowsUserObjectQueryReceipt"));
    assert!(LAUNCH_SECURITY.contains("name_utf16: Box<[u16]>"));
    assert!(LAUNCH_SECURITY.contains("object_type_utf16: Box<[u16]>"));
    assert!(LAUNCH_SECURITY.contains("security_descriptor: Box<[u8]>"));
    assert!(LAUNCH_SECURITY
        .contains("_authenticated_user_object_query_producer_unavailable: Infallible"));
    assert!(LAUNCH_SECURITY.contains("GetUserObjectInformationW"));
    assert!(LAUNCH_SECURITY.contains("UOI_NAME"));
    assert!(LAUNCH_SECURITY.contains("UOI_TYPE"));
    assert!(LAUNCH_SECURITY.contains("GetUserObjectSecurity"));
    assert!(LAUNCH_SECURITY.contains("struct SealedWindowsDesktopParentBindingReceipt"));
    assert!(LAUNCH_SECURITY.contains("struct SealedWindowsDesktopTokenSessionBindingReceipt"));
    assert!(LAUNCH_SECURITY.contains("primary_token_session_id: u32"));
    assert!(LAUNCH_SECURITY.contains("primary_token_logon_session_identity_digest: String"));
    assert!(LAUNCH_SECURITY.contains("TokenSessionId"));
    assert!(LAUNCH_SECURITY.contains("TokenStatistics"));
    assert!(LAUNCH_SECURITY.contains("window_station_handle_value: usize"));
    assert!(LAUNCH_SECURITY.contains("desktop_handle_value: usize"));
    assert!(
        LAUNCH_SECURITY.contains("_authenticated_desktop_parent_backend_unavailable: Infallible")
    );
    assert!(LAUNCH_SECURITY.contains("self._window_station.0"));
    assert!(LAUNCH_SECURITY.contains("self._desktop.0"));
    assert!(LAUNCH_SECURITY.contains("struct OwnedPrivateDesktop(HDESK)"));
    assert!(LAUNCH_SECURITY.contains("struct OwnedPrivateWindowStation(HWINSTA)"));
    assert!(LAUNCH_SECURITY.contains("impl Drop for OwnedPrivateDesktop"));
    assert!(LAUNCH_SECURITY.contains("unsafe { CloseDesktop(self.0) }"));
    assert!(LAUNCH_SECURITY.contains("impl Drop for OwnedPrivateWindowStation"));
    assert!(LAUNCH_SECURITY.contains("unsafe { CloseWindowStation(self.0) }"));
    assert!(LAUNCH_SECURITY.contains("self.private_desktop.validate("));
    assert!(LAUNCH_SECURITY
        .contains("_authenticated_token_session_namespace_backend_unavailable: Infallible"));
    assert!(LAUNCH_SECURITY.contains("fn private_desktop_name_ptr(&self) -> *mut u16"));
    assert!(WINDOWS_JOB.contains("struct ConfiguredRunnerStartupInfo<'owner>"));
    assert!(WINDOWS_JOB.contains("&'owner SealedWindowsRunnerLaunchSecurity"));
    assert!(WINDOWS_JOB
        .contains("raw.StartupInfo.lpDesktop = launch_security.private_desktop_name_ptr()"));
    assert!(WINDOWS.contains("job.startup_info(&loader_current.preparation.launch_security)"));
    assert!(MODEL.contains("launch_security: SealedWindowsRunnerLaunchSecurity"));
}

#[test]
fn loader_currentness_is_consumed_immediately_before_process_create() {
    assert!(LOADER_CURRENTNESS.contains("trait WindowsRunnerPreCreateLoaderCurrentnessBackend"));
    assert!(!LOADER_CURRENTNESS.contains("impl WindowsRunnerPreCreateLoaderCurrentnessBackend for"));
    assert!(LOADER_CURRENTNESS.contains("LoaderCurrentWindowsRunnerProcessPreparation<'root>"));
    assert!(LOADER_CURRENTNESS.contains("DefinitiveRejected"));
    assert!(LOADER_CURRENTNESS.contains("OutcomeUncertain"));
    assert!(LOADER_CURRENTNESS
        .contains("returned_positive: Option<ManagedLoaderNamespaceQueryReceipt>"));
    assert!(LOADER_CURRENTNESS.contains("returned_positive.is_none()"));
    assert!(LOADER_CURRENTNESS.contains("authenticated_response_is_bound()"));
    assert!(LOADER_CURRENTNESS.contains("fence_generation_set_digest"));
    assert!(LOADER_CURRENTNESS.contains("content_lease_generation_set_digest"));
    assert!(LOADER_CURRENTNESS.contains("resolution_profile_digest"));
    assert!(LOADER_CURRENTNESS.contains("known_dll_section_generation_digest"));
    assert!(LOADER_CURRENTNESS.contains("api_set_schema_identity_digest"));
    assert!(LOADER_CURRENTNESS.contains("activation_context_identity_digest"));
    assert!(LOADER_CURRENTNESS.contains("driver_session_identity_digest"));
    assert!(
        LOADER_CURRENTNESS.contains("query_generation <= image.final_namespace_query_generation()")
    );
    assert!(LOADER_CURRENTNESS
        .contains("receipt_request_digest == image.final_namespace_query_request_digest()"));
    assert!(LOADER_CURRENTNESS
        .contains("receipt_nonce_digest == image.final_namespace_query_nonce_digest()"));
    assert!(LOADER_CURRENTNESS.contains("ExplicitAuthorizedReleaseRequiredButUnavailable"));
    assert!(LOADER_CURRENTNESS.contains("jcs_sha256_hex(&material)?"));
    assert!(WINDOWS.contains("loader_currentness_backend.query_current_and_seal(preparation)"));
    assert!(WINDOWS.contains("loader_current.validate_binding()"));
    assert_eq!(WINDOWS.matches("launch_security.validate()").count(), 2);
    let query = WINDOWS
        .find("loader_currentness_backend.query_current_and_seal(preparation)")
        .expect("loader-currentness query missing");
    let create_frame = WINDOWS
        .find("let mut post_create = match create_suspended_process_with_custody(")
        .expect("whole-custody CreateProcess frame missing");
    let startup = WINDOWS
        .find("job.startup_info(&loader_current.preparation.launch_security)")
        .expect("Job and desktop startup-info setup missing");
    let create = WINDOWS
        .find("CreateProcessAsUserW(")
        .expect("process create missing");
    assert!(query < create_frame);
    assert!(startup < create);
    assert!(MODEL.contains("WindowsRunnerPreCreateLoaderCurrentness"));
}

#[test]
fn restricted_token_empty_dacls_and_private_desktop_are_creation_prerequisites() {
    assert!(LAUNCH_SECURITY.contains("primary_token: OwnedHandle"));
    assert!(LAUNCH_SECURITY.contains("IsTokenRestricted"));
    assert!(LAUNCH_SECURITY.contains("TokenIsAppContainer"));
    assert!(LAUNCH_SECURITY.contains("TokenPrimary"));
    assert!(LAUNCH_SECURITY.contains("SE_SELF_RELATIVE"));
    assert!(LAUNCH_SECURITY.contains("dacl_present == 0 || dacl.is_null()"));
    assert!(LAUNCH_SECURITY.contains("acl_size.AceCount != 0"));
    assert!(LAUNCH_SECURITY.contains("bInheritHandle: 0"));
    assert!(LAUNCH_SECURITY.contains("_private_desktop_isolation_producer_unavailable: Infallible"));
    assert!(WINDOWS.contains("CreateProcessAsUserW("));
    assert!(WINDOWS.contains("&create_security.process_attributes"));
    assert!(WINDOWS.contains("&create_security.thread_attributes"));
    assert!(WINDOWS.contains("startup.as_ptr()"));
    assert!(!WINDOWS.contains("CreateProcessW("));
}

#[test]
fn job_is_attached_atomically_and_query_verified_without_fallback() {
    assert!(WINDOWS_JOB.contains("CreateJobObjectW"));
    assert!(WINDOWS_JOB.contains("JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE"));
    assert!(WINDOWS_JOB.contains("JOB_OBJECT_LIMIT_ACTIVE_PROCESS"));
    assert!(WINDOWS_JOB.contains("JOB_OBJECT_LIMIT_JOB_MEMORY"));
    assert!(WINDOWS_JOB
        .contains("JOB_OBJECT_LIMIT_BREAKAWAY_OK | JOB_OBJECT_LIMIT_SILENT_BREAKAWAY_OK"));
    assert!(WINDOWS_JOB.contains("InitializeProcThreadAttributeList"));
    assert!(WINDOWS_JOB.contains("PROC_THREAD_ATTRIBUTE_JOB_LIST"));
    assert!(WINDOWS_JOB.contains("DeleteProcThreadAttributeList"));
    assert!(WINDOWS.contains("EXTENDED_STARTUPINFO_PRESENT"));
    assert!(WINDOWS.contains("CREATE_SUSPENDED"));
    assert!(WINDOWS.contains("CREATE_UNICODE_ENVIRONMENT"));
    assert!(WINDOWS.contains("CREATE_NO_WINDOW"));
    assert!(WINDOWS.contains("IsProcessInJob("));
    assert!(WINDOWS.contains("post_create.rollback().process_raw()"));
    assert!(WINDOWS.contains("post_create.rollback().job_raw()"));
    assert!(!source_slice().contains("AssignProcessToJobObject"));
    assert!(!source_slice().contains("CREATE_BREAKAWAY_FROM_JOB"));
}

#[test]
fn failures_and_drop_quarantine_the_whole_process_custody_until_termination_is_confirmed() {
    let combined = source_slice();
    assert!(!combined.contains("ResumeThread"));
    assert!(!combined.contains("Command::new"));
    assert!(WINDOWS_ROLLBACK.contains("struct SuspendedProcessRollback"));
    assert!(WINDOWS_ROLLBACK.contains("fn terminate_and_confirm(&mut self) -> bool"));
    assert!(WINDOWS_ROLLBACK.contains("fn terminate_and_confirm_owned("));
    assert!(WINDOWS_ROLLBACK.contains("fn terminate_and_confirm_raw("));
    assert!(WINDOWS_ROLLBACK.contains("impl Drop for SuspendedProcessRollback"));
    assert!(WINDOWS_ROLLBACK.contains("TerminateJobObject"));
    assert!(WINDOWS_ROLLBACK.contains("TerminateProcess"));
    assert!(WINDOWS_ROLLBACK.contains("WaitForSingleObject"));
    assert!(WINDOWS_ROLLBACK.contains("QueryInformationJobObject"));
    assert!(WINDOWS_ROLLBACK.contains("JobObjectBasicAccountingInformation"));
    assert!(WINDOWS_ROLLBACK.contains("accounting.ActiveProcesses == 0"));
    assert!(WINDOWS_ROLLBACK.contains("process_signaled && job_signaled"));
    assert!(MODEL.contains("struct WindowsRunnerLiveProcessCustody<'root>"));
    assert!(MODEL.contains("custody: ManuallyDrop<WindowsRunnerLiveProcessCustody<'root>>"));
    assert!(WINDOWS_ROLLBACK.contains("struct WindowsRunnerPostCreateCustody<'root>"));
    assert!(WINDOWS_ROLLBACK.contains("rollback: ManuallyDrop<SuspendedProcessRollback>"));
    assert!(WINDOWS_ROLLBACK.contains(
        "preparation: ManuallyDrop<LoaderCurrentWindowsRunnerProcessPreparation<'root>>"
    ));
    assert!(WINDOWS_ROLLBACK.contains("impl Drop for WindowsRunnerPostCreateCustody"));
    assert!(WINDOWS_ROLLBACK.contains("if self.rollback_mut().terminate_and_confirm()"));
    assert!(WINDOWS_ROLLBACK.contains("fn into_prepared_process("));
    assert!(WINDOWS_ROLLBACK.contains("rollback.into_handles_if_complete()"));
    assert!(WINDOWS.contains("fn create_suspended_process_with_custody<'root>("));
    assert!(WINDOWS.contains("SuspendedProcessRollback::from_created(job, process_information)"));
    assert!(WINDOWS.contains("WindowsRunnerPostCreateCustody::new("));
    assert!(WINDOWS.contains("match post_create.into_prepared_process(identity)"));
    assert!(WINDOWS.contains(
        "PostCreateUnconfirmed(ManuallyDrop<WindowsRunnerUnconfirmedProcessCustody<'root>>)"
    ));
    assert!(WINDOWS.contains("struct WindowsRunnerUnconfirmedProcessCustody<'root>"));
    assert!(WINDOWS.contains("_post_create: WindowsRunnerPostCreateCustody<'root>"));
    assert!(WINDOWS.contains("if post_create.rollback_mut().terminate_and_confirm()"));
    assert!(WINDOWS.contains("PostCreateUnconfirmed(ManuallyDrop::new("));
    assert!(WINDOWS.contains("impl Drop for PreparedComputePluginRunnerProcess"));
    assert!(WINDOWS.contains("ManuallyDrop::drop(&mut self.custody)"));
}

#[test]
fn command_material_is_explicit_and_does_not_inherit_node_secrets() {
    assert!(ENCODING.contains("nul_terminated_path"));
    assert!(ENCODING.contains("quote_argument"));
    assert!(ENCODING.contains("empty_environment_block"));
    assert!(ENCODING.contains("[0, 0]"));
    assert!(WINDOWS.contains("application.as_ptr()"));
    assert!(WINDOWS.contains("command.as_mut_ptr()"));
    assert!(WINDOWS.contains("environment.as_ptr().cast()"));
    assert!(WINDOWS.contains("current_directory.as_ptr()"));
    assert!(WINDOWS.contains("create_security.primary_token"));
}

#[test]
fn source_material_freezes_resume_blockers_launch_binding_and_zero_effects() {
    for blocker in [
        "authenticated_ipc_bootstrap",
        "cpu_enforcement",
        "disk_enforcement",
        "dynamic_module_load_enforcement",
        "launch_path_parent_chain_currentness",
        "live_windows_resolution_currentness",
        "namespace_fence_explicit_release_and_recovery",
        "network_enforcement",
        "pre_resume_loader_currentness",
        "private_noninteractive_window_station_desktop_isolation",
        "runtime_transition_store",
        "runtime_transition_recovery",
        "sidecar_uptime_enforcement",
        "termination_outcome_recovery",
        "vram_enforcement",
    ] {
        assert!(POLICY.contains(blocker), "missing blocker {blocker}");
    }
    for prerequisite in [
        "launch_context_selector_digest",
        "startup_import_resolution_profile_digest",
        "startup_import_namespace_authority_digest",
        "required_launch_context_digest",
        "launch_token_profile_digest",
        "private_desktop_isolation_digest",
        "process_security_descriptor_digest",
        "thread_security_descriptor_digest",
        "proc_thread_attribute_job_list",
        "retained_handle_derived",
    ] {
        assert!(
            POLICY.contains(prerequisite),
            "missing prerequisite {prerequisite}"
        );
    }
    assert!(POLICY.contains("windows_runner_required_launch_context.v3"));
    assert!(POLICY.contains("elon.compute_plugin.windows_runner_process_preparation.v3"));
    assert!(POLICY.contains("entrypoint_arguments_digest: profile.entrypoint_arguments_digest()"));
    assert!(POLICY.contains("explicit_empty_unicode_environment_block_v1"));
    assert!(POLICY.contains("process_creation_flags: PROCESS_CREATION_FLAGS"));
    for authenticated_bridge in [
        "launch_context_selector_digest: String",
        "startup_import_resolution_profile_digest: String",
        "expected_required_launch_context_digest: String",
        "fn launch_context_selector_digest(&self) -> &str",
        "fn startup_import_resolution_profile_digest(&self) -> &str",
        "fn expected_required_launch_context_digest(&self) -> &str",
    ] {
        assert!(
            LAUNCH_SECURITY.contains(authenticated_bridge),
            "missing launch-security context bridge {authenticated_bridge}"
        );
    }
    assert!(MODEL.contains(
        "launch_security.launch_context_selector_digest() != image.launch_context_selector_digest()"
    ));
    assert!(MODEL.contains("launch_security.startup_import_resolution_profile_digest()"));
    assert!(MODEL.contains("!= image.startup_import_resolution_profile_digest()"));
    assert!(MODEL.contains("WindowsRunnerLaunchContextPreCreateProjection::new("));
    assert!(MODEL.contains("profile.entrypoint_arguments_digest()"));
    assert!(MODEL.contains("loader_locked.validate_authenticated_launch_context_projection("));
    assert!(LOADER_VALIDATION.contains("fn validate_authenticated_launch_context_projection("));
    assert!(LOADER_VALIDATION.contains(".authenticated_launch_lineage"));
    assert!(LOADER_VALIDATION.contains(".validate_loader_image_binding("));
    assert!(LOADER_VALIDATION.contains(".validate_process_projection(profile, expected)"));
    assert!(EXACT_CONTEXT_LINEAGE.contains("struct WindowsRunnerLaunchContextPreCreateProjection"));
    assert!(EXACT_CONTEXT_LINEAGE.contains("fn validate_process_projection("));
    for exact_field in [
        "expected.restricted_token",
        "expected.app_container",
        "expected.inherited_handles",
        "expected.environment_policy",
        "expected.process_creation_flags",
        "expected.entrypoint_arguments_digest",
    ] {
        assert!(
            EXACT_CONTEXT_INTENT.contains(exact_field),
            "missing exact process projection field {exact_field}"
        );
    }
    let loader_internal = MODEL
        .find("loader_locked.validate_internal_binding()?")
        .expect("loader internal binding validation missing");
    let security = MODEL
        .find("launch_security.validate()?")
        .expect("launch-security validation missing");
    let projection = MODEL
        .find("let precreate_launch_context = WindowsRunnerLaunchContextPreCreateProjection::new(")
        .expect("pre-create launch-context projection missing");
    let typed_projection = MODEL
        .find("loader_locked.validate_authenticated_launch_context_projection(")
        .expect("typed launch-context validation missing");
    let policy_projection = MODEL
        .find("WindowsRunnerProcessPolicy::from_sources(")
        .expect("required process-policy projection missing");
    assert!(loader_internal < security);
    assert!(security < projection);
    assert!(projection < typed_projection);
    assert!(typed_projection < policy_projection);
    assert!(POLICY.contains("expected_required_launch_context_digest()"));
    assert!(POLICY.contains("post_create_live_process_machine_context_queryback"));
    assert!(POLICY.contains("bail!(\"COMPUTE_PLUGIN_WINDOWS_REQUIRED_LAUNCH_CONTEXT_CHANGED\")"));
    assert!(!POLICY.contains("required_launch_context_digest != image."));
    for effect in [
        "runtime_phase_effect",
        "runtime_generation_effect",
        "health_effect",
        "readiness_effect",
        "provider_effect",
        "route_effect",
        "offer_effect",
        "capacity_effect",
        "execution_effect",
        "attempt_effect",
        "lease_effect",
        "usage_effect",
        "settlement_effect",
        "money_effect",
    ] {
        assert!(POLICY.contains(&format!("{effect}: \"none\"")));
    }
}

#[test]
fn process_custody_source_does_not_write_runtime_or_market_authority() {
    let combined = source_slice();
    for forbidden in [
        "rusqlite",
        "INSERT INTO",
        "UPDATE compute_plugin",
        "RUNTIME_STARTING",
        "ComputeReadyCapability",
        "ValidatedComputeReadyPublication",
        "ComputeOffer",
        "ComputeLease",
        "runner_events::Started",
    ] {
        assert!(
            !combined.contains(forbidden),
            "forbidden source {forbidden}"
        );
    }
}
