const HOST_MODULE: &str = include_str!("mod.rs");
const FACADE: &str = include_str!("runtime_process_custody.rs");
const MODEL: &str = include_str!("runtime_process_custody/model.rs");
const POLICY: &str = include_str!("runtime_process_custody/policy.rs");
const ENCODING: &str = include_str!("runtime_process_custody/encoding.rs");
const LAUNCH_SECURITY: &str = include_str!("runtime_process_custody/launch_security.rs");
const WINDOWS_JOB: &str = include_str!("runtime_process_custody/windows_job.rs");
const WINDOWS: &str = include_str!("runtime_process_custody/windows.rs");
const SERVER_CARGO: &str = include_str!("../../Cargo.toml");

fn source_slice() -> String {
    [
        FACADE,
        MODEL,
        POLICY,
        ENCODING,
        LAUNCH_SECURITY,
        WINDOWS_JOB,
        WINDOWS,
    ]
    .join("\n")
}

#[test]
fn source_routes_one_private_windows_process_custody_owner() {
    assert!(HOST_MODULE.contains("mod runtime_process_custody;"));
    assert!(HOST_MODULE.contains("mod runtime_process_custody_source_contract_tests;"));
    for module in [
        "mod encoding;",
        "mod launch_security;",
        "mod model;",
        "mod policy;",
        "mod windows;",
        "mod windows_job;",
    ] {
        assert!(FACADE.contains(module), "missing module {module}");
    }
    assert_eq!(FACADE.matches("mod ").count(), 6);
    assert!(SERVER_CARGO.contains("\"Win32_Security\""));
    assert!(SERVER_CARGO.contains("\"Win32_System_JobObjects\""));
}

#[test]
fn sealed_launch_authorities_have_no_producer_or_scalar_escape() {
    let combined = source_slice();
    assert_eq!(
        MODEL
            .matches("struct SealedComputePluginRunnerImage {")
            .count(),
        1
    );
    assert_eq!(
        combined.matches("SealedComputePluginRunnerImage {").count(),
        2,
        "only the type definition and its current Debug impl may exist"
    );
    assert_eq!(
        LAUNCH_SECURITY
            .matches("struct SealedWindowsRunnerLaunchSecurity {")
            .count(),
        1
    );
    assert_eq!(
        combined
            .matches("SealedWindowsRunnerLaunchSecurity {")
            .count(),
        3,
        "only the type definition plus its current inherent and Debug impls may exist"
    );
    assert_eq!(combined.matches("from_sealed_authorities(").count(), 1);
    assert_eq!(
        combined
            .matches("prepare_suspended_windows_runner_process")
            .count(),
        1
    );
    assert!(!FACADE.contains("prepare_suspended_windows_runner_process"));
    assert!(MODEL.contains("executable: File"));
    assert!(MODEL.contains("working_directory: File"));
    assert!(MODEL.contains("loader_dependency_files: Vec<File>"));
    assert!(MODEL.contains("loader_namespace_directories: Vec<File>"));
    assert!(!combined.contains("pub(crate) fn path"));
    assert!(!combined.contains("pub(crate) fn files"));
    assert!(!combined.contains("try_clone"));
}

#[test]
fn preparation_consumes_linear_authorities_and_keeps_all_custody() {
    assert!(MODEL.contains("admitted: DurableWorkAdmittedPluginSlot<'root>"));
    assert!(MODEL.contains("runner_image: SealedComputePluginRunnerImage"));
    assert!(MODEL.contains("launch_security: SealedWindowsRunnerLaunchSecurity"));
    assert!(MODEL.contains("job: OwnedHandle"));
    assert!(MODEL.contains("process: OwnedHandle"));
    assert!(MODEL.contains("primary_thread: OwnedHandle"));
    assert!(!MODEL.contains("#[derive(Clone"));
    assert!(!MODEL.contains("Serialize, Deserialize"));
}

#[test]
fn restricted_token_and_empty_dacls_are_creation_prerequisites() {
    assert!(LAUNCH_SECURITY.contains("primary_token: OwnedHandle"));
    assert!(LAUNCH_SECURITY.contains("IsTokenRestricted"));
    assert!(LAUNCH_SECURITY.contains("TokenIsAppContainer"));
    assert!(LAUNCH_SECURITY.contains("TokenPrimary"));
    assert!(LAUNCH_SECURITY.contains("SE_SELF_RELATIVE"));
    assert!(LAUNCH_SECURITY.contains("dacl_present == 0 || dacl.is_null()"));
    assert!(LAUNCH_SECURITY.contains("acl_size.AceCount != 0"));
    assert!(LAUNCH_SECURITY.contains("bInheritHandle: 0"));
    assert!(WINDOWS.contains("CreateProcessAsUserW("));
    assert!(WINDOWS.contains("&create_security.process_attributes"));
    assert!(WINDOWS.contains("&create_security.thread_attributes"));
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
    assert!(WINDOWS.contains("IsProcessInJob(rollback.process_raw(), rollback.job_raw()"));
    assert!(!source_slice().contains("AssignProcessToJobObject"));
    assert!(!source_slice().contains("CREATE_BREAKAWAY_FROM_JOB"));
}

#[test]
fn process_never_resumes_and_failure_retains_uncertain_handles() {
    let combined = source_slice();
    assert!(!combined.contains("ResumeThread"));
    assert!(!combined.contains("Command::new"));
    assert!(WINDOWS.contains("uncertain_process: Option<SuspendedProcessRollback>"));
    assert!(WINDOWS.contains("fn terminate_and_confirm(&mut self) -> bool"));
    assert!(WINDOWS.contains("impl Drop for SuspendedProcessRollback"));
    assert!(WINDOWS.contains("impl Drop for PreparedComputePluginRunnerProcess"));
    assert!(WINDOWS.contains("TerminateJobObject"));
    assert!(WINDOWS.contains("TerminateProcess"));
    assert!(WINDOWS.contains("WaitForSingleObject"));
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
fn source_material_freezes_resume_blockers_and_zero_effects() {
    for blocker in [
        "authenticated_ipc_bootstrap",
        "cpu_enforcement",
        "disk_enforcement",
        "network_enforcement",
        "runtime_transition_store",
        "runtime_transition_recovery",
        "sidecar_uptime_enforcement",
        "vram_enforcement",
    ] {
        assert!(POLICY.contains(blocker), "missing blocker {blocker}");
    }
    for prerequisite in [
        "loader_dependency_closure_digest",
        "path_namespace_lock_digest",
        "launch_token_profile_digest",
        "process_security_descriptor_digest",
        "thread_security_descriptor_digest",
        "proc_thread_attribute_job_list",
    ] {
        assert!(
            POLICY.contains(prerequisite),
            "missing prerequisite {prerequisite}"
        );
    }
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
