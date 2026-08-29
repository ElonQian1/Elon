const HOST_MODULE: &str = include_str!("mod.rs");
const REGISTRY_ROOT: &str = include_str!("local_authority/sqlite_vfs_policy/registry.rs");
const POLICY_ROOT: &str = include_str!("local_authority/sqlite_vfs_policy.rs");
const LOCAL_AUTHORITY_ROOT: &str = include_str!("local_authority.rs");
const OPEN_ATTEMPT: &str =
    include_str!("local_authority/sqlite_vfs_policy/registry/open_attempt.rs");
const PROCESS_OWNER: &str =
    include_str!("local_authority/sqlite_vfs_policy/registry/process_owner.rs");
const REGISTRY_OWNER: &str = include_str!("local_authority/sqlite_vfs_policy/registry/owner.rs");
const REGISTRY_STATE: &str = include_str!("local_authority/sqlite_vfs_policy/registry/state.rs");
const OPENED_AUTHORITY: &str = include_str!("local_authority/opened_authority.rs");
const AUTHORITY: &str = include_str!(
    "../../../docs/distributed-compute/node-plugin-handle-bound-open-attempt-authority.md"
);
const ACCEPTANCE: &str = include_str!(
    "../../../docs/distributed-compute/node-plugin-handle-bound-open-attempt-acceptance.md"
);

fn ordered(source: &str, needles: &[&str]) {
    let mut cursor = 0;
    for needle in needles {
        let offset = source[cursor..]
            .find(needle)
            .unwrap_or_else(|| panic!("missing source marker: {needle}"));
        cursor += offset + needle.len();
    }
}

fn between<'a>(source: &'a str, start: &str, end: &str) -> &'a str {
    let start = source.find(start).expect("missing section start");
    let tail = &source[start..];
    let end = tail.find(end).expect("missing section end");
    &tail[..end]
}

fn has_cfg_test_module(source: &str, module: &str) -> bool {
    let expected = format!("mod {module};");
    let lines = source.lines().map(str::trim).collect::<Vec<_>>();
    lines
        .windows(2)
        .any(|pair| pair == ["#[cfg(test)]", expected.as_str()])
}

#[test]
fn open_attempt_typestates_are_private_linear_and_have_no_safe_producer() {
    assert!(has_cfg_test_module(
        HOST_MODULE,
        "handle_bound_open_attempt_source_contract_tests"
    ));
    assert!(REGISTRY_ROOT
        .lines()
        .any(|line| line.trim() == "mod open_attempt;"));
    assert!(!REGISTRY_ROOT.contains("pub mod open_attempt;"));
    assert!(!REGISTRY_ROOT.contains("pub use open_attempt"));
    assert!(!POLICY_ROOT.contains("ComputePluginHandleBoundOpenAttemptProcess"));
    assert!(!LOCAL_AUTHORITY_ROOT.contains("ComputePluginHandleBoundOpenAttemptProcess"));
    for marker in [
        "struct ComputePluginHandleBoundOpenAttemptProcess",
        "struct RegisteredComputePluginHandleBoundAuthorityOpenAttempt",
        "struct OpeningComputePluginHandleBoundAuthorityOpenAttempt",
        "struct ComputePluginHandleBoundOpenAttemptRegistrationFailure",
        "struct ComputePluginHandleBoundOpenAttemptBeginFailure",
        "PhantomData<Rc<()>>",
    ] {
        assert!(OPEN_ATTEMPT.contains(marker), "missing {marker}");
    }
    for forbidden in [
        "Serialize",
        "Deserialize",
        "OnceLock",
        "LazyLock",
        "leak_with_system_nonce_source",
        "from_process_owner",
        "fn new(",
        "fn seal(",
        "::leak(",
        "Box::leak",
        "Clone for",
        "Copy for",
        "unsafe impl",
    ] {
        assert!(!OPEN_ATTEMPT.contains(forbidden), "forbidden {forbidden}");
    }
    assert_eq!(
        OPEN_ATTEMPT
            .matches("ComputePluginHandleBoundOpenAttemptProcess {")
            .count(),
        3
    );
    let process_seal = between(
        OPEN_ATTEMPT,
        "struct ComputePluginHandleBoundOpenAttemptProcess {",
        "impl ComputePluginHandleBoundOpenAttemptProcess",
    );
    assert!(process_seal.contains("owner: &'static ComputePluginHandleBoundSqliteProcessOwner"));
    assert!(process_seal.contains("_not_send_or_sync: PhantomData<Rc<()>>"));
    let process_impl = between(
        OPEN_ATTEMPT,
        "impl ComputePluginHandleBoundOpenAttemptProcess",
        "impl fmt::Debug for ComputePluginHandleBoundOpenAttemptProcess",
    );
    assert!(!process_impl.contains("Self {"));
    for typestate in [
        "struct RegisteredComputePluginHandleBoundAuthorityOpenAttempt",
        "struct OpeningComputePluginHandleBoundAuthorityOpenAttempt",
    ] {
        let body = between(OPEN_ATTEMPT, typestate, "impl ");
        assert!(body.contains("_not_send_or_sync: PhantomData<Rc<()>>"));
    }
    assert!(OPEN_ATTEMPT
        .lines()
        .filter(|line| line.trim_start().starts_with("#[derive"))
        .all(|line| line.trim() == "#[derive(Debug)]"));
    assert!(!OPEN_ATTEMPT.contains("pub(crate)"));
    assert!(!OPEN_ATTEMPT.contains("pub(in crate"));
    assert_eq!(
        OPEN_ATTEMPT
            .matches("ComputePluginHandleBoundOpenAttemptProcess")
            .count(),
        4
    );
}

#[test]
fn registration_begin_and_drop_keep_exact_custody() {
    ordered(
        OPEN_ATTEMPT,
        &[
            "let identity = ComputePluginHandleBoundOpenIdentity::from_intent(&intent);",
            "self.owner.register(intent)",
            "failure.into_parts()",
            "RegisteredComputePluginHandleBoundAuthorityOpenAttempt",
        ],
    );
    assert!(between(
        OPEN_ATTEMPT,
        "struct ComputePluginHandleBoundOpenAttemptRegistrationFailure",
        "/// Exact registry PendingMain custody"
    )
    .contains("intent: ComputePluginHandleBoundAuthorityOpenIntent"));
    let registration_failure = between(
        OPEN_ATTEMPT,
        "impl ComputePluginHandleBoundOpenAttemptRegistrationFailure",
        "impl fmt::Debug for ComputePluginHandleBoundOpenAttemptRegistrationFailure",
    );
    for marker in [
        "pub(super) fn into_parts(",
        "ManagedSqliteRegistryProcessRegistrationRejection",
        "ComputePluginHandleBoundAuthorityOpenIntent",
        "(self.reason, self.intent)",
    ] {
        assert!(registration_failure.contains(marker), "missing {marker}");
    }
    assert!(!registration_failure.contains("into_intent"));
    ordered(
        between(
            OPEN_ATTEMPT,
            "pub(super) fn begin_open(",
            "fn exact_route(&self)",
        ),
        &[
            "main_logical_name_owned(route)",
            ".identity",
            ".take()",
            ".route",
            ".take()",
            "match self.owner.begin_open_attempt(route)",
            "Ok(()) => Ok(OpeningComputePluginHandleBoundAuthorityOpenAttempt",
            "Err(reason) => {",
            "self.route = Some(route);",
            "self.identity = Some(identity);",
        ],
    );
    let success_arm = between(
        OPEN_ATTEMPT,
        "Ok(()) => Ok(OpeningComputePluginHandleBoundAuthorityOpenAttempt",
        "Err(reason) => {",
    );
    for forbidden in ["?", "expect(", "unwrap(", "map_err", "return"] {
        assert!(
            !success_arm.contains(forbidden),
            "fallible success tail: {forbidden}"
        );
    }
    let begin_failure = between(
        OPEN_ATTEMPT,
        "struct ComputePluginHandleBoundOpenAttemptBeginFailure",
        "impl fmt::Debug for ComputePluginHandleBoundOpenAttemptBeginFailure",
    );
    assert!(!begin_failure.contains("OpenIntent"));
    assert!(!begin_failure.contains("RouteHandle"));

    let pending_drop = between(
        OPEN_ATTEMPT,
        "impl Drop for RegisteredComputePluginHandleBoundAuthorityOpenAttempt",
        "impl fmt::Debug for RegisteredComputePluginHandleBoundAuthorityOpenAttempt",
    );
    ordered(
        pending_drop,
        &["self.route.take()", "self.owner.retire_pending(route)"],
    );
    assert!(!pending_drop.contains("retain_terminal_custody"));

    let opening_drop = between(
        OPEN_ATTEMPT,
        "impl Drop for OpeningComputePluginHandleBoundAuthorityOpenAttempt",
        "impl fmt::Debug for OpeningComputePluginHandleBoundAuthorityOpenAttempt",
    );
    ordered(
        opening_drop,
        &[
            "self.route.take()",
            "self.owner.retain_terminal_custody(",
            "ManagedSqliteRegistryTerminalReason::FailureCustodyRetained",
        ],
    );
    assert!(opening_drop.contains("ManagedSqliteRegistryTerminalReason::FailureCustodyRetained"));
    assert!(!opening_drop.contains("retire_pending"));
    let opening_impl = between(
        OPEN_ATTEMPT,
        "impl OpeningComputePluginHandleBoundAuthorityOpenAttempt",
        "impl Drop for OpeningComputePluginHandleBoundAuthorityOpenAttempt",
    );
    assert_eq!(opening_impl.matches("pub(super) fn ").count(), 2);
    assert!(opening_impl.contains("fn authority_instance_binding(&self)"));
    assert!(opening_impl.contains("fn main_logical_name(&self)"));

    ordered(
        between(
            REGISTRY_OWNER,
            "pub(super) fn register(",
            "pub(super) fn main_logical_name(",
        ),
        &["custody.ensure_registry_current()", "self.routes.insert("],
    );
    ordered(
        between(
            REGISTRY_OWNER,
            "pub(super) fn main_logical_name(",
            "pub(super) fn phase(",
        ),
        &[
            "self.exact_entry(handle)?",
            "ManagedSqliteLogicalFileRole::Main",
        ],
    );
    ordered(
        between(
            REGISTRY_STATE,
            "pub(super) fn begin_open_attempt(",
            "pub(super) fn begin_callback(",
        ),
        &[
            "ManagedSqliteRegistrySessionPhase::PendingMain",
            "self.connection_owner = true",
            "self.phase = ManagedSqliteRegistrySessionPhase::Opening",
        ],
    );
}

#[test]
fn production_open_and_downstream_effects_remain_absent() {
    let production_open = between(
        OPENED_AUTHORITY,
        "fn open(",
        "impl Drop for ComputePluginHandleBoundAuthorityOpenIntent",
    );
    ordered(
        production_open,
        &[
            "self.ensure_current()?",
            "bail!(HANDLE_BOUND_SQLITE_VFS_UNAVAILABLE)",
        ],
    );
    assert_eq!(
        production_open.matches("self.ensure_current()?;").count(),
        1
    );
    assert_eq!(
        production_open
            .matches("bail!(HANDLE_BOUND_SQLITE_VFS_UNAVAILABLE)")
            .count(),
        1
    );
    for forbidden in [
        "Ok(",
        "match ",
        "if ",
        "else ",
        "Connection::",
        "from_verified_backend",
    ] {
        assert!(
            !production_open.contains(forbidden),
            "open contains {forbidden}"
        );
    }
    assert_eq!(
        OPENED_AUTHORITY.matches("from_verified_backend(").count(),
        1
    );
    assert_eq!(
        PROCESS_OWNER
            .matches("leak_with_system_nonce_source")
            .count()
            + REGISTRY_ROOT
                .matches("leak_with_system_nonce_source")
                .count()
            + POLICY_ROOT.matches("leak_with_system_nonce_source").count()
            + LOCAL_AUTHORITY_ROOT
                .matches("leak_with_system_nonce_source")
                .count()
            + OPEN_ATTEMPT
                .matches("leak_with_system_nonce_source")
                .count(),
        1
    );
    for forbidden in [
        "sqlite3_vfs_register",
        "sqlite3_open",
        "rusqlite::Connection",
        "Connection::open",
        "from_verified_backend",
        "SealedHandleBoundSqliteBackend",
        "activate_connection(",
        "begin_connection_close(",
        "observe_connection_closed(",
        "retire_closed(",
        "test_vfs_bridge",
        "canonicalize(",
        "std::fs",
    ] {
        assert!(!OPEN_ATTEMPT.contains(forbidden), "forbidden {forbidden}");
    }
}

#[test]
fn authority_records_verified_source_unwired_production_and_zero_effect_boundary() {
    let authority_a2_gate = between(AUTHORITY, "## 5. A2 与生产启用门", "## 6. 零效果");
    for marker in [
        "Barrier 与 Registration 各 `WindowsDynamic=8/8`",
        "A2b2 `WindowsDynamic=16/117`",
        "clean wide regression `121/121`",
    ] {
        assert!(
            authority_a2_gate.contains(marker),
            "authority A2 gate missing {marker}"
        );
    }
    for marker in [
        "registration_status: unregistered_feature_workflow_unavailable",
        "source_compiled_production_unwired",
        "targeted_local_source_and_registry_verified",
        "source-contract `4/4`",
        "registry lifecycle 回归 `42/42`",
        "open-attempt 两态没有行为运行证据",
        "migration/table/view/trigger/writer = none/none/none/none/none",
        "vfs_registration/sqlite_open/connection/opened_authority = none/none/none/none",
        "COMPUTE_PLUGIN_HANDLE_BOUND_SQLITE_VFS_UNAVAILABLE",
        "service/api/http/mcp/pc/wire = none/none/none/none/none/none",
        "plan_apply/runtime/ready/provider/route/offer/capacity = none",
        "job/attempt/lease/receipt/usage/settlement/money = none",
    ] {
        assert!(AUTHORITY.contains(marker), "authority missing {marker}");
    }

    let current_evidence = between(ACCEPTANCE, "## 1. 当前证据强度", "## 2. Source review 清单");
    for marker in [
        "a2_barrier_windows_dynamic=8/8",
        "a2_registration_windows_dynamic=8/8",
        "a2b2_windows_dynamic=16/117",
        "production_acceptance=deferred",
    ] {
        assert!(
            current_evidence.contains(marker),
            "current acceptance evidence missing {marker}"
        );
    }
    assert!(!current_evidence.contains("a2b2_windows_dynamic=117/117"));

    let barrier_row = ACCEPTANCE
        .lines()
        .find(|line| line.starts_with("| A2 Barrier WindowsDynamic |"))
        .expect("missing A2 Barrier WindowsDynamic matrix row");
    assert!(barrier_row.contains("| 8 | 0 | 0 |"));
    assert!(barrier_row.contains("`8/8`"));

    let registration_row = ACCEPTANCE
        .lines()
        .find(|line| line.starts_with("| A2 Registration WindowsDynamic |"))
        .expect("missing A2 Registration WindowsDynamic matrix row");
    assert!(registration_row.contains("| 8 | 0 | 0 |"));
    assert!(registration_row.contains("`8/8`"));

    let a2b2_row = ACCEPTANCE
        .lines()
        .find(|line| line.starts_with("| A2b2 WindowsDynamic |"))
        .expect("missing A2b2 WindowsDynamic matrix row");
    assert!(a2b2_row.contains("| 16 | 0 | 101 |"));
    assert!(a2b2_row.contains("`16/117`"));
    assert!(!a2b2_row.contains("117/117"));

    let promotion_gate = &ACCEPTANCE[ACCEPTANCE
        .find("## 5. 晋级门")
        .expect("missing promotion gate")..];
    for marker in [
        "Barrier/Registration 各 `8/8`",
        "`117/117` WindowsDynamic",
        "宽回归",
    ] {
        assert!(
            promotion_gate.contains(marker),
            "promotion gate missing {marker}"
        );
    }
    for marker in [
        "source_contract_guard=4/4",
        "registry_lifecycle_regression=42/42",
        "compiled_targets=1 test_cases_run=46 passed=46 failed=0",
        "open_attempt_runtime_unrun",
        "production_acceptance=deferred",
    ] {
        assert!(ACCEPTANCE.contains(marker), "acceptance missing {marker}");
    }
}
