use super::{
    git_output, prepare_project_storage_repo, validate_repo_access_token, StorageRepoRequest,
    StorageSettings,
};
use std::path::PathBuf;
use uuid::Uuid;

#[test]
fn prepare_repo_uses_user_scoped_url_and_token() {
    if crate::git_command_error::git_command()
        .arg("--version")
        .output()
        .is_err()
    {
        return;
    }
    let root = std::env::temp_dir().join(format!("elon_storage_repo_{}", Uuid::new_v4().simple()));
    let settings = StorageSettings {
        enabled: true,
        root_path: Some(root.to_string_lossy().to_string()),
        git_base_url: Some("https://git.example.test/elon".into()),
    };

    let result = prepare_project_storage_repo(
        &settings,
        StorageRepoRequest {
            project_id: "project:one".into(),
            user_id: "user/one".into(),
            name: "Project One".into(),
            branch: Some("main".into()),
            access_token: Some("abcdefghijklmnopqrstuvwxyz0123456789".into()),
            prepare_worktree: false,
        },
    )
    .expect("storage repo should prepare");

    assert_eq!(
        result.storage_repo_url.as_deref(),
        Some("https://git.example.test/elon/projects/user-one/project-one.git")
    );
    assert!(validate_repo_access_token(
        &PathBuf::from(&result.storage_repo_path),
        "abcdefghijklmnopqrstuvwxyz0123456789"
    ));
    assert!(result.storage_worktree_path.is_none());
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn prepare_repo_can_create_owner_worktree() {
    if crate::git_command_error::git_command()
        .arg("--version")
        .output()
        .is_err()
    {
        return;
    }
    let root = std::env::temp_dir().join(format!(
        "elon_storage_repo_worktree_{}",
        Uuid::new_v4().simple()
    ));
    let settings = StorageSettings {
        enabled: true,
        root_path: Some(root.to_string_lossy().to_string()),
        git_base_url: None,
    };

    let result = prepare_project_storage_repo(
        &settings,
        StorageRepoRequest {
            project_id: "project:two".into(),
            user_id: "user/two".into(),
            name: "Project Two".into(),
            branch: Some("main".into()),
            access_token: Some("abcdefghijklmnopqrstuvwxyz0123456789".into()),
            prepare_worktree: true,
        },
    )
    .expect("storage repo and owner worktree should prepare");

    let worktree = PathBuf::from(
        result
            .storage_worktree_path
            .as_ref()
            .expect("owner worktree path should return"),
    );
    assert!(worktree.join("README.md").exists());
    assert_eq!(
        git_output(&worktree, &["remote", "get-url", "origin"]).expect("origin should exist"),
        result.storage_repo_path
    );
    assert_eq!(
        git_output(&worktree, &["rev-parse", "--abbrev-ref", "HEAD"]).expect("branch should exist"),
        "main"
    );
    let _ = std::fs::remove_dir_all(root);
}
