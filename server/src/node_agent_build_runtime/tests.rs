use super::{
    cleanup::{cleanup_expired, remove_managed_path},
    paths::{ensure_within_root, resolve_run_paths},
    prepare_run, status, BuildCachePolicy, BuildEnvironment, BuildRunRequest,
};
use elon_pc_dev_runtime::NodeDataPaths;
use std::{
    fs,
    path::PathBuf,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

#[test]
fn environment_routes_large_outputs_under_node_data_root() {
    let root = unique_root("environment");
    let data_paths = NodeDataPaths::new(&root);
    let paths = resolve_run_paths(&data_paths, "task-1", "project/one", None).unwrap();
    let environment = BuildEnvironment::for_run(&paths, "project/one", "task-1");

    for key in [
        "CARGO_TARGET_DIR",
        "CARGO_HOME",
        "GRADLE_USER_HOME",
        "NPM_CONFIG_CACHE",
        "PNPM_STORE_DIR",
        "COREPACK_HOME",
        "TEMP",
        "TMP",
        "TMPDIR",
    ] {
        let value = environment
            .entries()
            .iter()
            .find_map(|(candidate, value)| (candidate == key).then_some(value))
            .unwrap();
        assert!(PathBuf::from(value).starts_with(&root));
    }
    assert!(!environment
        .entries()
        .iter()
        .any(|(key, _)| key.eq_ignore_ascii_case("PNPM_HOME")));
}

#[test]
fn rust_targets_share_project_and_split_toolchain_identity() {
    let root = unique_root("rust-target");
    let data_paths = NodeDataPaths::new(&root);
    let first = resolve_run_paths(&data_paths, "task-a", "project-a", None).unwrap();
    let second = resolve_run_paths(&data_paths, "task-b", "project-a", None).unwrap();
    let other = resolve_run_paths(&data_paths, "task-c", "project-b", None).unwrap();

    assert_eq!(first.cargo_target, second.cargo_target);
    assert_ne!(first.cargo_target, other.cargo_target);
}

#[test]
fn cleanup_refuses_paths_outside_managed_root() {
    let root = unique_root("cleanup-root");
    let outside = unique_root("cleanup-outside");
    fs::create_dir_all(&root).unwrap();
    fs::create_dir_all(&outside).unwrap();

    assert!(ensure_within_root(&root, &outside).is_err());
    assert!(remove_managed_path(&root, &outside).is_err());
    let _ = fs::remove_dir_all(root);
    let _ = fs::remove_dir_all(outside);
}

#[test]
fn prepared_run_creates_lease_and_removes_task_temp_on_drop() {
    let root = unique_root("lease");
    let data_paths = NodeDataPaths::new(&root);
    let mut run = prepare_run(
        &data_paths,
        BuildRunRequest {
            task_id: "task-lease",
            project_id: "project-lease",
            cwd: None,
        },
    )
    .unwrap();
    let temp = PathBuf::from(
        run.environment()
            .entries()
            .iter()
            .find_map(|(key, value)| (key == "TEMP").then_some(value))
            .unwrap(),
    );
    assert!(temp.is_dir());
    run.finish(true);
    drop(run);
    assert!(!temp.exists());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn failed_run_retains_task_temp_for_ttl_diagnostics() {
    let root = unique_root("failed-temp");
    let data_paths = NodeDataPaths::new(&root);
    let run = prepare_run(
        &data_paths,
        BuildRunRequest {
            task_id: "task-failed",
            project_id: "project-failed",
            cwd: None,
        },
    )
    .unwrap();
    let temp = PathBuf::from(
        run.environment()
            .entries()
            .iter()
            .find_map(|(key, value)| (key == "TEMP").then_some(value))
            .unwrap(),
    );
    drop(run);
    assert!(temp.is_dir());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn root_status_does_not_require_a_build_run_path() {
    let root = unique_root("root-status");
    let data_paths = NodeDataPaths::new(&root);
    fs::create_dir_all(data_paths.cache()).unwrap();
    fs::create_dir_all(data_paths.temp()).unwrap();
    fs::create_dir_all(data_paths.npm_cache()).unwrap();
    fs::write(data_paths.npm_cache().join("sample"), b"cache").unwrap();

    let snapshot = status(&data_paths);
    assert_eq!(snapshot.root, root.to_string_lossy().to_string());
    assert!(snapshot.cache_bytes >= 5);
    assert!(snapshot.max_total_cache_bytes > 0);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn expired_cleanup_keeps_current_run_lease() {
    let root = unique_root("active-cleanup");
    let data_paths = NodeDataPaths::new(&root);
    let run = prepare_run(
        &data_paths,
        BuildRunRequest {
            task_id: "task-active",
            project_id: "project-active",
            cwd: None,
        },
    )
    .unwrap();
    let paths = resolve_run_paths(&data_paths, "task-active", "project-active", None).unwrap();
    let report = cleanup_expired(
        &paths,
        &BuildCachePolicy {
            temp_ttl_secs: 1,
            cache_ttl_secs: 1,
            min_free_bytes: 1,
            max_total_cache_bytes: u64::MAX,
            max_project_rust_bytes: u64::MAX,
        },
    )
    .unwrap();
    assert!(paths.task_temp.exists());
    assert!(report.skipped_active_paths >= 1);
    drop(run);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn exec_project_context_remains_backward_compatible() {
    let legacy = r#"{"type":"exec","task_id":"task-1","cli":"cargo","args":["check"],"cwd":"D:/repo","env":[]}"#;
    let parsed: homecli_proto::ServerToAgent = serde_json::from_str(legacy).unwrap();
    match parsed {
        homecli_proto::ServerToAgent::Exec {
            project_context, ..
        } => assert!(project_context.is_none()),
        _ => panic!("expected exec"),
    }

    let current = homecli_proto::ServerToAgent::Exec {
        task_id: "task-2".into(),
        cli: "cargo".into(),
        args: vec!["check".into()],
        cwd: "D:/repo".into(),
        env: Vec::new(),
        project_context: Some(homecli_proto::CliProjectContext {
            project_id: "project-1".into(),
            conversation_id: "conversation-1".into(),
            runtime_permission: Some("project_write".into()),
        }),
    };
    let encoded = serde_json::to_string(&current).unwrap();
    assert!(encoded.contains("project-1"));
}

fn unique_root(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_nanos();
    std::env::temp_dir().join(format!("elon-build-runtime-{label}-{nanos}"))
}
