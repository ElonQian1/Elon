const WORKER: &str = include_str!("../external_pool_adapter_task_worker.rs");
const FEDERATION_ROOT: &str = include_str!("../mod.rs");
const STARTUP: &str = include_str!("../../node_endpoint_session_startup.rs");
const BACKGROUND: &str = include_str!("../../server_background_workers.rs");
const RELEASE_API: &str = include_str!("../external_pool_adapter_release_api.rs");
const STORE_RECOVERY: &str =
    include_str!("../../store/compute_external_pool_adapter_task_delivery/recovery.rs");

#[test]
fn task_protocol_production_worker_is_default_off_and_requires_both_custodies() {
    assert_eq!(
        WORKER
            .matches("ELON_EXTERNAL_POOL_ADAPTER_ATTEMPT_DELIVERY_ENABLED")
            .count(),
        1
    );
    assert_ordered(
        source_block(WORKER, "fn configured_enabled()", "#[cfg(all("),
        &[
            "None => Ok(false)",
            "Some(\"true\") => Ok(true)",
            "Some(\"false\") => Ok(false)",
            "enabled value is invalid",
        ],
    );
    assert_ordered(
        WORKER,
        &[
            "if enabled {",
            "require_production_runtime_custody()?",
            "WORKER_ENABLED",
        ],
    );
    let linux_custody = source_block(
        WORKER,
        "#[cfg(all(target_os = \"linux\", target_arch = \"x86_64\"))]",
        "#[cfg(not(all(target_os = \"linux\", target_arch = \"x86_64\")))]",
    );
    assert_ordered(
        linux_custody,
        &[
            "external_pool_adapter_provider_runtime_readiness_runtime()",
            "external_pool_adapter_task_protocol_conformance_runtime()",
        ],
    );
    assert!(WORKER.contains("task delivery requires Linux x86-64"));
}

#[test]
fn task_protocol_production_worker_stays_dormant_and_wired_after_runtime_init() {
    assert!(FEDERATION_ROOT.contains("pub(crate) mod external_pool_adapter_task_worker;"));
    assert_ordered(
        STARTUP,
        &[
            "initialize_external_pool_adapter_provider_runtime_readiness_runtime()?",
            "initialize_external_pool_adapter_task_protocol_conformance_runtime()?",
            "initialize_external_pool_adapter_task_worker_runtime()?",
        ],
    );
    assert!(BACKGROUND.contains("external_pool_adapter_task_worker::spawn(state.clone())"));
    assert!(WORKER.contains("Duration::from_secs(60)"));
    assert!(WORKER.contains("ExternalPoolAdapterTaskWorkerCycleReport { eligible_rows: 0 }"));
    assert_ordered(
        WORKER,
        &[
            "recover_external_pool_adapter_task_delivery()",
            "eligible_rows != DORMANT_CYCLE_REPORT.eligible_rows",
            "continue;",
        ],
    );
    assert!(STORE_RECOVERY.contains("report.eligible_rows = 0;"));
    assert!(!WORKER.contains("Store::"));
    assert!(!WORKER.contains("begin_application_exchange"));
    assert!(!WORKER.contains("exchange_external_pool_adapter_broker_task"));
    for forbidden in [
        "PreparedStartSendRequest",
        "CommittedStartSendAuthority",
        "VerifiedComputeStartOutboxRemoteObservation",
        "persist_route_authority_on",
    ] {
        assert!(
            !WORKER.contains(forbidden),
            "dormant worker gained {forbidden}"
        );
    }
}

#[test]
fn task_protocol_production_adds_no_public_http_surface_or_panic_path() {
    for forbidden in [
        "task-protocol-production",
        "task-delivery",
        "authenticated-events",
        "external-pool-task",
    ] {
        assert!(
            !RELEASE_API.contains(forbidden),
            "V273 gained public API marker {forbidden}"
        );
    }
    for forbidden in [
        ".unwrap(",
        ".expect(",
        "panic!",
        "todo!",
        "unimplemented!",
        "unreachable!",
        "debug_assert!",
        "assert!",
    ] {
        assert!(!WORKER.contains(forbidden), "worker retained {forbidden}");
    }
}

fn source_block<'a>(source: &'a str, start: &str, end: &str) -> &'a str {
    source
        .split_once(start)
        .unwrap()
        .1
        .split_once(end)
        .unwrap()
        .0
}

fn assert_ordered(source: &str, needles: &[&str]) {
    let mut cursor = 0;
    for needle in needles {
        let offset = source[cursor..]
            .find(needle)
            .unwrap_or_else(|| panic!("missing ordered source marker {needle}"));
        cursor += offset + needle.len();
    }
}
