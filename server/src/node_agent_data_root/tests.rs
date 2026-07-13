use super::*;
use uuid::Uuid;

#[test]
fn environment_root_wins_over_persisted_root() {
    let env_root = std::env::temp_dir().join("elon-env-root");
    let persisted_root = std::env::temp_dir().join("elon-persisted-root");
    let state = resolve_from_values(Some(env_root.clone()), Some(persisted_root), None, None);

    assert_eq!(state.source, NodeDataRootSource::Environment);
    assert_eq!(state.configured_root(), Some(env_root.as_path()));
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
    let paths = NodeDataPaths::new(&root);
    std::fs::create_dir_all(paths.cache()).expect("create cache");
    std::fs::create_dir_all(paths.temp()).expect("create temp");
    std::fs::create_dir_all(paths.workspaces()).expect("create workspaces");
    std::fs::create_dir_all(paths.storage()).expect("create storage");
    std::fs::write(paths.cache().join("cache.bin"), [1u8; 16]).expect("seed cache");
    std::fs::write(paths.workspaces().join("keep.txt"), "keep").expect("seed workspace");

    let result = cleanup(&paths, true).expect("cleanup managed cache and temp");

    assert_eq!(result.entries.len(), 2);
    assert!(paths.workspaces().join("keep.txt").is_file());
    assert!(paths.storage().is_dir());
    assert!(!paths.cache().join("cache.bin").exists());
    let _ = std::fs::remove_dir_all(root);
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

    let error = validate_no_root_overlap(legacy.to_string_lossy().as_ref(), &state)
        .expect_err("overlap must fail");
    assert!(error.to_string().contains("互相嵌套"));
}
