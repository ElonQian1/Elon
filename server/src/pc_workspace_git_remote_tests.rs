use super::{clean_git_remote, current_branch, ensure_git_remote_workspace, git_remote_origin};
use std::fs;
use std::path::Path;
use uuid::Uuid;

#[test]
fn git_remote_workspace_clones_requested_branch() {
    if crate::git_command_error::git_command()
        .arg("--version")
        .output()
        .is_err()
    {
        return;
    }
    let root = std::env::temp_dir().join(format!(
        "elon_git_remote_workspace_{}",
        Uuid::new_v4().simple()
    ));
    let remote = root.join("remote.git");
    let seed = root.join("seed");
    let repo = root.join("managed").join("repo");
    fs::create_dir_all(&seed).expect("seed dir should create");
    run_git(&root, &["init", "--bare", remote.to_str().unwrap()]);
    run_git(&seed, &["init"]);
    run_git(&seed, &["config", "user.name", "Test"]);
    run_git(&seed, &["config", "user.email", "test@example.com"]);
    run_git(&seed, &["checkout", "-B", "main"]);
    fs::write(seed.join("README.md"), "# Portable\n").expect("readme should write");
    run_git(&seed, &["add", "."]);
    run_git(&seed, &["commit", "-m", "init"]);
    run_git(
        &seed,
        &["remote", "add", "origin", remote.to_str().unwrap()],
    );
    run_git(&seed, &["push", "-u", "origin", "main"]);

    let remote_cfg = clean_git_remote(Some(&remote.to_string_lossy()), Some("main"))
        .expect("remote config should clean");
    ensure_git_remote_workspace(&repo, &remote_cfg, |repo| {
        fs::write(repo.join("AGENTS.md"), "# Project\n")?;
        Ok(())
    })
    .expect("workspace should clone remote");

    assert!(repo.join("README.md").exists());
    assert_eq!(current_branch(&repo).as_deref(), Some("main"));
    assert_eq!(
        git_remote_origin(&repo).expect("origin should exist"),
        remote.to_string_lossy()
    );
    let _ = fs::remove_dir_all(root);
}

fn run_git(cwd: &Path, args: &[&str]) {
    let output = crate::git_command_error::git_command()
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("git should run");
    assert!(
        output.status.success(),
        "git {:?} failed: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
}
