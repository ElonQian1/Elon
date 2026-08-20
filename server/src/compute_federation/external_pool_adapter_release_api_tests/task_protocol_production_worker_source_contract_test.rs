const WORKER_ROOT: &str = include_str!("../external_pool_adapter_task_worker.rs");
const WORKER_LIFECYCLE: &str = include_str!("../external_pool_adapter_task_worker/lifecycle.rs");
const WORKER_CYCLE: &str = include_str!("../external_pool_adapter_task_worker/cycle.rs");
const WORKER_REPORT: &str = include_str!("../external_pool_adapter_task_worker/report.rs");
const FEDERATION_ROOT: &str = include_str!("../mod.rs");
const STARTUP: &str = include_str!("../../node_endpoint_session_startup.rs");
const BACKGROUND: &str = include_str!("../../server_background_workers.rs");
const RELEASE_API: &str = include_str!("../external_pool_adapter_release_api.rs");

#[test]
fn task_protocol_production_worker_is_default_off_and_requires_both_custodies() {
    for module in ["cycle", "lifecycle", "report"] {
        assert!(
            WORKER_ROOT.contains(&format!("mod {module};")),
            "worker root lost {module}"
        );
    }
    assert_eq!(
        WORKER_LIFECYCLE
            .matches("ELON_EXTERNAL_POOL_ADAPTER_ATTEMPT_DELIVERY_ENABLED")
            .count(),
        1
    );
    assert_ordered(
        source_block(
            WORKER_LIFECYCLE,
            "fn configured_enabled()",
            "#[cfg(all(target_os",
        ),
        &[
            "None => Ok(false)",
            "Some(\"true\") => Ok(true)",
            "Some(\"false\") => Ok(false)",
            "enabled value is invalid",
        ],
    );
    assert_ordered(
        WORKER_LIFECYCLE,
        &[
            "if enabled {",
            "require_production_runtime_custody()?",
            "WORKER_ENABLED",
        ],
    );
    let linux_custody = source_block(
        WORKER_LIFECYCLE,
        "#[cfg(all(target_os = \"linux\", target_arch = \"x86_64\"))]\nfn require_production_runtime_custody()",
        "#[cfg(not(all(target_os",
    );
    assert_ordered(
        linux_custody,
        &[
            "external_pool_adapter_provider_runtime_readiness_runtime()",
            "external_pool_adapter_task_protocol_conformance_runtime()",
        ],
    );
    assert!(WORKER_LIFECYCLE.contains("Duration::from_secs(60)"));
    assert!(WORKER_LIFECYCLE.contains("task delivery requires Linux x86-64"));
}

#[test]
fn task_protocol_worker_runs_preparation_before_the_honest_source_stage() {
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
    assert_ordered(
        WORKER_CYCLE,
        &[
            "run_external_pool_adapter_active_preparation_cycle(",
            "run_external_pool_adapter_task_delivery_source_cycle(&checked_at)",
            "if let Some(provider_id) = observed_provider",
            "reprove_external_pool_adapter_task_delivery_source(",
            "eligible_rows: 0",
            "delivery_attempted: false",
        ],
    );
    assert!(WORKER_REPORT.contains("if report.eligible_rows == 0"));
    assert!(WORKER_REPORT.contains("observed rows without a V278 producer"));
    for forbidden in [
        "record_external_pool_adapter_task_outbound_on",
        "insert_external_pool_adapter_task_exchange_receipt_on",
        "apply_external_pool_adapter_task_terminal_ack_on",
        "exchange_external_pool_adapter_broker_task",
        "relay_external_pool_adapter_task",
    ] {
        assert!(
            !WORKER_CYCLE.contains(forbidden),
            "source-stage worker gained positive producer {forbidden}"
        );
    }
}

#[test]
fn task_protocol_production_adds_no_public_http_or_panic_surface() {
    for forbidden in [
        "task-protocol-production",
        "task-delivery",
        "authenticated-events",
        "external-pool-task",
    ] {
        assert!(
            !RELEASE_API.contains(forbidden),
            "V278 gained public API marker {forbidden}"
        );
    }
    let worker = format!("{WORKER_ROOT}{WORKER_LIFECYCLE}{WORKER_CYCLE}{WORKER_REPORT}");
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
        assert!(!worker.contains(forbidden), "worker retained {forbidden}");
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
