use crate::command_probe;
use std::{fs, io, path::Path, process::Output};

pub struct ProjectGitBaselineRequest<'a> {
    pub branch: Option<&'a str>,
}

pub fn ensure_project_git_baseline(
    repo: &Path,
    req: &ProjectGitBaselineRequest<'_>,
) -> io::Result<()> {
    fs::create_dir_all(repo)?;
    if !repo.join(".git").exists() {
        run_git(repo, &["init"])?;
    }

    if let Some(branch) = req.branch.and_then(clean_branch) {
        ensure_branch(repo, branch)?;
    }

    let _ = run_git(repo, &["config", "user.name", "Elon PC Node"]);
    let _ = run_git(repo, &["config", "user.email", "node@elon.local"]);
    run_git(repo, &["add", "."])?;
    let _ = run_git(
        repo,
        &["commit", "-m", "chore: initialize pc managed project"],
    );
    Ok(())
}

fn ensure_branch(repo: &Path, branch: &str) -> io::Result<()> {
    if git_has_head(repo) {
        if current_branch(repo).as_deref() == Some(branch) {
            return Ok(());
        }
        if local_branch_exists(repo, branch) {
            run_git(repo, &["checkout", branch])
        } else {
            run_git(repo, &["checkout", "-b", branch])
        }
    } else {
        run_git(repo, &["checkout", "-B", branch])
    }
}

fn clean_branch(value: &str) -> Option<&str> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

fn git_has_head(repo: &Path) -> bool {
    git_output(repo, &["rev-parse", "--verify", "HEAD"]).is_ok()
}

fn local_branch_exists(repo: &Path, branch: &str) -> bool {
    git_output(
        repo,
        &["rev-parse", "--verify", &format!("refs/heads/{branch}")],
    )
    .is_ok()
}

fn current_branch(repo: &Path) -> Option<String> {
    git_output(repo, &["rev-parse", "--abbrev-ref", "HEAD"])
        .ok()
        .filter(|branch| !branch.is_empty() && branch != "HEAD")
}

fn git_output(repo: &Path, args: &[&str]) -> io::Result<String> {
    let output = git_command(repo, args)?;
    if !output.status.success() {
        return Err(git_error(args, output));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn run_git(repo: &Path, args: &[&str]) -> io::Result<()> {
    let output = git_command(repo, args)?;
    if !output.status.success() {
        return Err(git_error(args, output));
    }
    Ok(())
}

fn git_command(repo: &Path, args: &[&str]) -> io::Result<Output> {
    command_probe::command_output("git", args, Some(repo))
}

fn git_error(args: &[&str], output: Output) -> io::Error {
    let stderr = String::from_utf8_lossy(&output.stderr);
    io::Error::new(
        io::ErrorKind::Other,
        format!("git {:?} failed: {}", args, stderr.trim()),
    )
}

#[cfg(test)]
mod tests {
    use super::{ensure_project_git_baseline, ProjectGitBaselineRequest};
    use std::{
        fs,
        path::Path,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn git_baseline_initializes_repo_identity_branch_and_commit() {
        if crate::command_probe::git_command()
            .arg("--version")
            .output()
            .is_err()
        {
            return;
        }
        let root = temp_dir("git_baseline_initializes_repo_identity_branch_and_commit");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("README.md"), "# Demo\n").unwrap();

        ensure_project_git_baseline(
            &root,
            &ProjectGitBaselineRequest {
                branch: Some("main"),
            },
        )
        .unwrap();

        assert!(root.join(".git").exists());
        assert_eq!(git_output(&root, &["config", "user.name"]), "Elon PC Node");
        assert_eq!(
            git_output(&root, &["config", "user.email"]),
            "node@elon.local"
        );
        assert_eq!(
            git_output(&root, &["rev-parse", "--abbrev-ref", "HEAD"]),
            "main"
        );
        assert!(!git_output(&root, &["rev-parse", "--short", "HEAD"]).is_empty());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn git_baseline_can_run_again_without_new_changes() {
        if crate::command_probe::git_command()
            .arg("--version")
            .output()
            .is_err()
        {
            return;
        }
        let root = temp_dir("git_baseline_can_run_again_without_new_changes");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("README.md"), "# Demo\n").unwrap();

        let req = ProjectGitBaselineRequest {
            branch: Some("main"),
        };
        ensure_project_git_baseline(&root, &req).unwrap();
        let before = git_output(&root, &["rev-parse", "HEAD"]);
        ensure_project_git_baseline(&root, &req).unwrap();
        let after = git_output(&root, &["rev-parse", "HEAD"]);

        assert_eq!(before, after);
        let _ = fs::remove_dir_all(root);
    }

    fn git_output(cwd: &Path, args: &[&str]) -> String {
        let output = crate::command_probe::git_command()
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
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    }

    fn temp_dir(label: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("elon-pc-dev-runtime-{label}-{nanos}"))
    }
}
