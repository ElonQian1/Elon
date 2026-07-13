use super::*;
use uuid::Uuid;

#[test]
fn persisted_root_wins_over_bootstrap_environment_root() {
    let env_root = std::env::temp_dir().join("elon-env-root");
    let persisted_root = std::env::temp_dir().join("elon-persisted-root");
    let state = resolve_from_values(Some(env_root), Some(persisted_root.clone()), None, None);

    assert_eq!(state.source, NodeDataRootSource::Persisted);
    assert_eq!(state.configured_root(), Some(persisted_root.as_path()));
}

#[test]
fn corrupted_root_marker_fails_closed() {
    let root = std::env::temp_dir().join(format!("elon-marker-corrupt-{}", Uuid::new_v4()));
    std::fs::create_dir_all(&root).expect("create root");
    std::fs::write(root.join(ROOT_MARKER_FILE), b"{broken").expect("write corrupt marker");

    let error = validate_and_prepare(root.to_string_lossy().as_ref(), "ins_current")
        .expect_err("corrupt marker must block startup");

    assert!(error.to_string().contains("标记损坏"));
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn root_marker_requires_nonempty_install_id() {
    let root = std::env::temp_dir().join(format!("elon-marker-empty-id-{}", Uuid::new_v4()));

    let error = validate_and_prepare(root.to_string_lossy().as_ref(), "  ")
        .expect_err("empty install id must not bind root");

    assert!(error.to_string().contains("安装 ID 不能为空"));
    assert!(!root.join(ROOT_MARKER_FILE).exists());
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn existing_marker_must_contain_nonempty_install_id() {
    let root = std::env::temp_dir().join(format!("elon-marker-missing-id-{}", Uuid::new_v4()));
    std::fs::create_dir_all(&root).expect("create root");
    std::fs::write(root.join(ROOT_MARKER_FILE), br#"{"schema_version":1}"#)
        .expect("write marker without install id");

    let error = validate_and_prepare(root.to_string_lossy().as_ref(), "ins_current")
        .expect_err("marker without identity must block startup");

    assert!(error.to_string().contains("缺少有效 install_id"));
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn existing_marker_must_use_supported_schema() {
    let root = std::env::temp_dir().join(format!("elon-marker-schema-{}", Uuid::new_v4()));
    std::fs::create_dir_all(&root).expect("create root");
    std::fs::write(
        root.join(ROOT_MARKER_FILE),
        br#"{"schema_version":2,"install_id":"ins_current"}"#,
    )
    .expect("write future marker");

    let error = validate_and_prepare(root.to_string_lossy().as_ref(), "ins_current")
        .expect_err("unknown marker schema must block root reuse");

    assert!(error.to_string().contains("schema_version 不受支持"));
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn fresh_unmarked_root_must_be_empty() {
    let root = std::env::temp_dir().join(format!("elon-root-not-empty-{}", Uuid::new_v4()));
    std::fs::create_dir_all(&root).expect("create root");
    std::fs::write(root.join("keep.txt"), b"user data").expect("seed unrelated data");

    let error = validate_and_prepare(root.to_string_lossy().as_ref(), "ins_current")
        .expect_err("non-empty unmarked root must be rejected");

    assert!(error.to_string().contains("必须是空目录"));
    assert!(root.join("keep.txt").is_file());
    assert!(!root.join(ROOT_MARKER_FILE).exists());
    assert!(!root.join("cache").exists());
    assert!(!root.join("temp").exists());
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn concurrent_different_installations_cannot_both_claim_a_root() {
    let root = std::env::temp_dir().join(format!("elon-marker-race-{}", Uuid::new_v4()));
    std::fs::create_dir_all(&root).expect("create empty root");
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let claims = ["ins_first", "ins_second"]
        .into_iter()
        .map(|install_id| {
            let root = root.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                validate_and_prepare(root.to_string_lossy().as_ref(), install_id)
            })
        })
        .collect::<Vec<_>>();

    let results = claims
        .into_iter()
        .map(|claim| claim.join().expect("claim thread"))
        .collect::<Vec<_>>();

    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(results.iter().filter(|result| result.is_err()).count(), 1);
    let marker = read_existing_root_marker(&root.join(ROOT_MARKER_FILE))
        .expect("read marker")
        .expect("marker exists");
    assert!(marker == "ins_first" || marker == "ins_second");
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn marker_owned_by_another_installation_is_never_rebound() {
    let root = std::env::temp_dir().join(format!("elon-marker-foreign-{}", Uuid::new_v4()));
    std::fs::create_dir_all(&root).expect("create root");
    let marker = root.join(ROOT_MARKER_FILE);
    std::fs::write(
        &marker,
        br#"{"schema_version":1,"install_id":"ins_other"}"#,
    )
    .expect("write foreign marker");
    let original = std::fs::read(&marker).expect("read original marker");

    let error = validate_and_prepare(root.to_string_lossy().as_ref(), "ins_current")
        .expect_err("foreign marker must block root reuse");

    assert!(error.to_string().contains("另一台"));
    assert_eq!(std::fs::read(&marker).expect("marker remains"), original);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn same_installation_reuses_existing_marker_without_rewriting_it() {
    let root = std::env::temp_dir().join(format!("elon-marker-reuse-{}", Uuid::new_v4()));
    let paths = validate_and_prepare(root.to_string_lossy().as_ref(), "ins_current")
        .expect("bind root first time");
    let marker = paths.root().join(ROOT_MARKER_FILE);
    let original = std::fs::read(&marker).expect("read marker");

    validate_and_prepare(root.to_string_lossy().as_ref(), "ins_current")
        .expect("same install reuses marker");

    assert_eq!(std::fs::read(&marker).expect("read marker again"), original);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn legacy_roots_are_reported_without_being_moved() {
    let root = std::env::temp_dir().join(format!("elon-data-root-test-{}", Uuid::new_v4()));
    let legacy = root.join("legacy-workspaces");
    std::fs::create_dir_all(&legacy).expect("create legacy root");
    std::fs::write(legacy.join("project.txt"), "legacy").expect("seed legacy root");

    let state = resolve_from_values(
        Some(root.join("new-root")),
        None,
        Some(legacy.clone()),
        None,
    );
    let plan = state.migration_plan();

    assert!(state.migration_required());
    assert_eq!(plan.len(), 1);
    assert_eq!(plan[0].source_path, path_text(&legacy));
    assert!(plan[0].read_only_compatibility);
    assert!(legacy.join("project.txt").is_file());
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn cleanup_never_includes_workspace_or_storage() {
    let root = std::env::temp_dir().join(format!("elon-data-cleanup-test-{}", Uuid::new_v4()));
    let paths = validate_and_prepare(root.to_string_lossy().as_ref(), "ins_current")
        .expect("prepare owned root");
    std::fs::write(paths.cache().join("cache.bin"), [1u8; 16]).expect("seed cache");
    std::fs::write(paths.workspaces().join("keep.txt"), "keep").expect("seed workspace");

    let result =
        cleanup(&paths, "ins_current", true).expect("cleanup managed cache and temp");

    assert_eq!(result.entries.len(), 2);
    assert!(paths.workspaces().join("keep.txt").is_file());
    assert!(paths.storage().is_dir());
    assert!(!paths.cache().join("cache.bin").exists());
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn cleanup_rejects_unmarked_or_foreign_root() {
    let unmarked_root =
        std::env::temp_dir().join(format!("elon-cleanup-unmarked-{}", Uuid::new_v4()));
    let unmarked_paths = NodeDataPaths::new(&unmarked_root);
    std::fs::create_dir_all(unmarked_paths.cache()).expect("create unmarked cache");
    let error = cleanup(&unmarked_paths, "ins_current", true)
        .expect_err("unmarked cleanup must fail");
    assert!(error.to_string().contains("缺少所有权标记"));
    assert!(unmarked_paths.cache().is_dir());

    let foreign_root =
        std::env::temp_dir().join(format!("elon-cleanup-foreign-{}", Uuid::new_v4()));
    let foreign_paths = validate_and_prepare(
        foreign_root.to_string_lossy().as_ref(),
        "ins_foreign",
    )
    .expect("prepare foreign root");
    let sentinel = foreign_paths.cache().join("keep.bin");
    std::fs::write(&sentinel, b"foreign-owner-data").expect("seed foreign cache");
    let error = cleanup(&foreign_paths, "ins_current", true)
        .expect_err("foreign marker must block cleanup");
    assert!(error.to_string().contains("另一台"));
    assert!(sentinel.is_file(), "foreign cache must remain untouched");

    let _ = std::fs::remove_dir_all(unmarked_root);
    let _ = std::fs::remove_dir_all(foreign_root);
}

#[test]
fn rejects_data_root_nested_with_legacy_git_data() {
    let legacy = std::env::temp_dir().join("elon-overlap-legacy");
    let state = resolve_from_values(
        None,
        None,
        Some(legacy.join("workspaces")),
        Some(legacy.join("storage")),
    );

    let error = validate_no_root_overlap(legacy.to_string_lossy().as_ref(), &state, "ins_current")
        .expect_err("overlap must fail");
    assert!(error.to_string().contains("互相嵌套"));
}

#[test]
fn normalizes_parent_segments_before_overlap_checks() {
    let base = std::env::temp_dir().join("elon-overlap-normalized");
    let legacy = base.join("legacy");
    let state = resolve_from_values(None, None, Some(legacy.clone()), None);
    let candidate = base.join("other").join("..").join("legacy").join("child");

    let error =
        validate_no_root_overlap(candidate.to_string_lossy().as_ref(), &state, "ins_current")
        .expect_err("normalized overlap must fail");
    assert!(error.to_string().contains("互相嵌套"));
}

#[test]
fn owned_previous_data_root_can_be_selected_again() {
    let base =
        std::env::temp_dir().join(format!("elon-data-root-rollback-{}", Uuid::new_v4()));
    let previous = validate_and_prepare(
        base.join("previous").to_string_lossy().as_ref(),
        "ins_current",
    )
    .expect("prepare previous root");
    let current = validate_and_prepare(
        base.join("current").to_string_lossy().as_ref(),
        "ins_current",
    )
    .expect("prepare current root");
    let state = NodeDataRootState::from_prepared_paths(
        current,
        NodeDataRootSource::Persisted,
        Some(previous.workspaces()),
        Some(previous.storage()),
    );

    validate_no_root_overlap(
        previous.root().to_string_lossy().as_ref(),
        &state,
        "ins_current",
    )
    .expect("owned previous root may bypass only its exact managed children");
    validate_no_canonical_root_overlap(previous.root(), &state, "ins_current")
        .expect("canonical rollback overlap is allowed for the same owner");

    let _ = std::fs::remove_dir_all(base);
}

#[cfg(windows)]
#[test]
fn overlap_checks_are_case_insensitive_on_windows() {
    let state = resolve_from_values(
        None,
        None,
        Some(std::path::PathBuf::from(r"D:\ElonLegacy\workspaces")),
        None,
    );

    let error = validate_no_root_overlap(r"d:\elonlegacy", &state, "ins_current")
        .expect_err("case-only overlap must fail on Windows");
    assert!(error.to_string().contains("互相嵌套"));
}

#[cfg(unix)]
#[test]
fn canonical_overlap_detects_legacy_symlink_target() {
    use std::os::unix::fs::symlink;

    let base = std::env::temp_dir().join(format!("elon-canonical-overlap-{}", Uuid::new_v4()));
    let actual = base.join("actual");
    let candidate = actual.join("node-data");
    let legacy_alias = base.join("legacy-link");
    std::fs::create_dir_all(&candidate).expect("create canonical candidate");
    symlink(&actual, &legacy_alias).expect("create legacy symlink");
    let state = resolve_from_values(None, None, Some(legacy_alias), None);

    let error = validate_no_canonical_root_overlap(&candidate, &state, "ins_current")
        .expect_err("canonical overlap must fail");

    assert!(error.to_string().contains("规范化后"));
    let _ = std::fs::remove_dir_all(base);
}

#[cfg(windows)]
#[test]
fn canonical_overlap_detects_legacy_junction_target() {
    let base = std::env::temp_dir().join(format!("elon-canonical-overlap-{}", Uuid::new_v4()));
    let actual = base.join("actual");
    let candidate = actual.join("node-data");
    let legacy_alias = base.join("legacy-junction");
    std::fs::create_dir_all(&candidate).expect("create canonical candidate");
    let status = std::process::Command::new("cmd")
        .args(["/c", "mklink", "/J"])
        .arg(&legacy_alias)
        .arg(&actual)
        .status()
        .expect("invoke mklink");
    if !status.success() {
        let _ = std::fs::remove_dir_all(base);
        return;
    }
    let state = resolve_from_values(None, None, Some(legacy_alias.clone()), None);

    let error = validate_no_canonical_root_overlap(&candidate, &state, "ins_current")
        .expect_err("canonical overlap must fail");

    assert!(error.to_string().contains("规范化后"));
    let _ = std::fs::remove_dir(&legacy_alias);
    let _ = std::fs::remove_dir_all(base);
}

#[cfg(windows)]
#[test]
fn rejects_junction_ancestor_before_creating_data_root() {
    let base = std::env::temp_dir().join(format!("elon-junction-root-{}", Uuid::new_v4()));
    let actual = base.join("actual");
    let junction = base.join("junction");
    std::fs::create_dir_all(&actual).expect("create junction target");
    let status = std::process::Command::new("cmd")
        .args(["/c", "mklink", "/J"])
        .arg(&junction)
        .arg(&actual)
        .status()
        .expect("invoke mklink");
    if !status.success() {
        let _ = std::fs::remove_dir_all(base);
        return;
    }

    let error = validate_and_prepare(
        junction.join("node-data").to_string_lossy().as_ref(),
        "ins_current",
    )
    .expect_err("junction ancestor must be rejected");

    assert!(error.to_string().contains("重解析点"));
    let _ = std::fs::remove_dir(&junction);
    let _ = std::fs::remove_dir_all(base);
}

#[cfg(windows)]
#[test]
fn cleanup_rejects_junction_inside_managed_tree() {
    let root = std::env::temp_dir().join(format!("elon-junction-cleanup-{}", Uuid::new_v4()));
    let outside = std::env::temp_dir().join(format!("elon-junction-outside-{}", Uuid::new_v4()));
    let paths =
        validate_and_prepare(root.to_string_lossy().as_ref(), "ins_current").expect("prepare root");
    std::fs::create_dir_all(&outside).expect("create outside target");
    std::fs::write(outside.join("keep.txt"), b"keep").expect("seed outside file");
    let junction = paths.cache().join("escape");
    let status = std::process::Command::new("cmd")
        .args(["/c", "mklink", "/J"])
        .arg(&junction)
        .arg(&outside)
        .status()
        .expect("invoke mklink");
    if !status.success() {
        let _ = std::fs::remove_dir_all(root);
        let _ = std::fs::remove_dir_all(outside);
        return;
    }

    let error = cleanup(&paths, "ins_current", true)
        .expect_err("cleanup must reject nested junction");

    assert!(error.to_string().contains("重解析点"));
    assert!(outside.join("keep.txt").is_file());
    let _ = std::fs::remove_dir(&junction);
    let _ = std::fs::remove_dir_all(root);
    let _ = std::fs::remove_dir_all(outside);
}
