//! Fail-closed revalidation for a modern descendant of a pre-provenance root.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::{
    node_agent_local_task_store::{LocalTaskRecord, LocalTaskStore},
    node_agent_local_task_supervision::{SupervisionContract, SUPERVISION_PROTOCOL},
};

/// A pre-platform requirement may lack `workspace_status` entirely. In that
/// one case, the immediate recovery descendant may authorize the base only
/// from its own complete platform identity after every mutable Git fact is
/// re-read. No malformed or partial root identity is silently replaced.
pub(super) fn validated_descendant_resume_base(
    tasks: &LocalTaskStore,
    parent: &LocalTaskRecord,
    parent_contract: &SupervisionContract,
    root_id: &str,
    project_id: &str,
) -> Result<String> {
    anyhow::ensure!(
        parent.task_id != root_id
            && parent_contract.protocol == SUPERVISION_PROTOCOL
            && matches!(
                parent_contract.task_role.as_str(),
                "capability_repair" | "resume_original"
            )
            && parent_contract.root_task_id.as_deref() == Some(root_id),
        "古老 root 缺少平台身份时，只允许当前协议的恢复后代提供后备身份"
    );
    let status = parent
        .workspace_status
        .as_ref()
        .context("恢复父任务缺少平台工作区身份")?;
    required_exact(
        status,
        "platform_provenance",
        "elon.conversation_worktree.v1",
    )?;
    anyhow::ensure!(
        status.get("isolated").and_then(serde_json::Value::as_bool) == Some(true),
        "恢复父任务不是平台隔离 worktree"
    );
    let status_project = required(status, "project_id")?;
    anyhow::ensure!(
        crate::node_agent_full_access::project_ids_equivalent(status_project, project_id)
            && crate::node_agent_full_access::project_ids_equivalent(
                status_project,
                &parent.project_id
            ),
        "恢复父任务平台项目身份漂移"
    );
    required_exact(status, "root_task_id", root_id)?;
    anyhow::ensure!(
        status.get("resume_blocked_reason").is_none()
            && status
                .get("terminal_snapshot_status")
                .and_then(serde_json::Value::as_str)
                != Some("rejected"),
        "恢复父任务终态工作区身份已被持久门禁拒绝"
    );

    let base = canonical_directory(required(status, "base_workspace_path")?, "基础工作区")?;
    let active = canonical_directory(required(status, "active_workspace_path")?, "活动 worktree")?;
    anyhow::ensure!(
        !same_path(&base, &active) && same_path(Path::new(&parent.workspace_path), &active),
        "恢复父任务活动路径与持久任务身份不一致"
    );
    validate_platform_shape(&active, &parent.project_id, required(status, "branch")?)?;

    required_exact(
        status,
        "branch",
        &git(&active, &["branch", "--show-current"])?,
    )?;
    let top = PathBuf::from(git(&active, &["rev-parse", "--show-toplevel"])?);
    anyhow::ensure!(
        same_path(&top, &active),
        "恢复父任务活动路径不是 worktree 根"
    );

    let current_common = PathBuf::from(git(
        &active,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )?);
    let base_common = PathBuf::from(git(
        &base,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )?);
    let recorded_common = PathBuf::from(required(status, "git_common_dir")?);
    anyhow::ensure!(
        same_path(&current_common, &base_common) && same_path(&current_common, &recorded_common),
        "恢复父任务 Git common-dir 身份漂移"
    );
    required_exact(
        status,
        "git_remote",
        &git(&base, &["config", "--get", "remote.origin.url"])?,
    )?;
    required_exact(
        status,
        "git_head",
        &git(&active, &["rev-parse", "--verify", "HEAD^{commit}"])?,
    )?;
    anyhow::ensure!(
        git(
            &active,
            &["status", "--porcelain=v1", "--untracked-files=all"]
        )?
        .is_empty(),
        "恢复父任务后备身份只允许 clean worktree"
    );

    let expected_lease = crate::node_agent_supervision_worktree_lease::lease_reason(root_id)?;
    let actual_lease =
        crate::node_agent_supervision_worktree_lease::worktree_lock_reason(&base, &active)?;
    anyhow::ensure!(
        actual_lease.as_deref() == Some(expected_lease.as_str()),
        "恢复父任务 root lease 身份不匹配"
    );
    anyhow::ensure!(
        !tasks.has_competing_workspace_occupancy(&parent.task_id, &parent.workspace_path)?,
        "恢复父任务 worktree 存在并发任务占用"
    );
    Ok(base.to_string_lossy().to_string())
}

fn validate_platform_shape(active: &Path, project_id: &str, branch: &str) -> Result<()> {
    let conversation = path_part(active, "会话")?;
    let project = active.parent().context("活动路径缺少项目目录")?;
    let project_part = path_part(project, "项目")?;
    let marker = path_part(project.parent().context("活动路径缺少平台目录")?, "平台")?;
    let expected_project = elon_pc_dev_runtime::safe_path_part(project_id, "project", 80);
    anyhow::ensure!(
        marker.eq_ignore_ascii_case("conversation-worktrees")
            && project_part.eq_ignore_ascii_case(&expected_project),
        "恢复父任务活动路径不是当前项目的平台 worktree"
    );
    anyhow::ensure!(
        branch == format!("ai/session/{expected_project}/{conversation}") && branch != "main",
        "恢复父任务平台分支身份无效"
    );
    Ok(())
}

fn required<'a>(status: &'a serde_json::Value, field: &str) -> Result<&'a str> {
    status
        .get(field)
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .with_context(|| format!("恢复父任务 workspace_status 缺少 {field}"))
}

fn required_exact(status: &serde_json::Value, field: &str, expected: &str) -> Result<()> {
    anyhow::ensure!(
        required(status, field)? == expected,
        "恢复父任务 workspace_status 的 {field} 身份漂移"
    );
    Ok(())
}

fn canonical_directory(value: &str, label: &str) -> Result<PathBuf> {
    let path = Path::new(value);
    anyhow::ensure!(path.is_absolute(), "{label}不是绝对路径");
    let path = path
        .canonicalize()
        .with_context(|| format!("{label}不存在或不可访问: {}", path.display()))?;
    anyhow::ensure!(path.is_dir(), "{label}不是目录");
    Ok(path)
}

fn path_part(path: &Path, label: &str) -> Result<String> {
    path.file_name()
        .and_then(|value| value.to_str())
        .map(str::to_string)
        .with_context(|| format!("活动路径缺少{label}身份"))
}

fn git(cwd: &Path, args: &[&str]) -> Result<String> {
    let output = crate::git_command_error::git_command()
        .args(args)
        .current_dir(cwd)
        .output()
        .with_context(|| crate::git_command_error::git_spawn_context(args))?;
    anyhow::ensure!(
        output.status.success(),
        "{}",
        crate::git_command_error::git_failure_message(cwd, args, &output)
    );
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn same_path(left: &Path, right: &Path) -> bool {
    crate::node_agent_update_checkpoint::same_path(left, right)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node_agent_local_task_store::LocalTaskStart;

    struct Fixture {
        root: PathBuf,
        base: PathBuf,
        active: PathBuf,
        tasks: LocalTaskStore,
        parent: LocalTaskRecord,
        contract: SupervisionContract,
    }

    impl Fixture {
        fn new(drift: Option<&str>) -> Self {
            let root = std::env::temp_dir().join(format!(
                "descendant-resume-{}",
                uuid::Uuid::new_v4().simple()
            ));
            let base = root.join("base");
            let active = root
                .join("conversation-worktrees")
                .join("elon-self")
                .join("repair-conversation");
            std::fs::create_dir_all(&base).unwrap();
            run(&base, &["init"]);
            run(&base, &["config", "user.email", "ai@example.test"]);
            run(&base, &["config", "user.name", "AI Test"]);
            run(
                &base,
                &[
                    "config",
                    "remote.origin.url",
                    "https://example.test/elon.git",
                ],
            );
            std::fs::write(base.join("seed.txt"), "seed\n").unwrap();
            run(&base, &["add", "seed.txt"]);
            run(&base, &["commit", "-m", "seed"]);
            std::fs::create_dir_all(active.parent().unwrap()).unwrap();
            let branch = "ai/session/elon-self/repair-conversation";
            run(
                &base,
                &[
                    "worktree",
                    "add",
                    "-b",
                    branch,
                    active.to_str().unwrap(),
                    "HEAD",
                ],
            );
            crate::node_agent_supervision_worktree_lease::acquire(&base, &active, "legacy-root")
                .unwrap();

            let tasks = LocalTaskStore::new(root.join("tasks.sqlite3"));
            tasks
                .create(LocalTaskStart {
                    task_id: "repair-child",
                    owner_user_id: "owner",
                    agent_id: "agent",
                    install_id: "install",
                    project_id: "elon-self",
                    channel_id: None,
                    conversation_id: "repair-conversation",
                    workspace_path: active.to_str().unwrap(),
                    prompt: "repair",
                    cli: "codex",
                    runtime_permission: "full_access",
                })
                .unwrap();
            let mut status = serde_json::json!({
                "platform_provenance": "elon.conversation_worktree.v1",
                "project_id": "elon-self",
                "root_task_id": "legacy-root",
                "base_workspace_path": base,
                "active_workspace_path": active,
                "isolated": true,
                "branch": branch,
                "git_head": output(&active, &["rev-parse", "HEAD"]),
                "git_common_dir": output(
                    &active,
                    &["rev-parse", "--path-format=absolute", "--git-common-dir"]
                ),
                "git_remote": "https://example.test/elon.git",
                "terminal_snapshot_status": "trusted"
            });
            match drift {
                Some("provenance") => status["platform_provenance"] = "unknown".into(),
                Some("common") => {
                    status["git_common_dir"] = root
                        .join("foreign.git")
                        .to_string_lossy()
                        .into_owned()
                        .into()
                }
                Some("base") => {
                    status["base_workspace_path"] =
                        root.join("missing").to_string_lossy().into_owned().into()
                }
                Some("malformed") => {
                    status.as_object_mut().unwrap().remove("git_remote");
                }
                _ => {}
            }
            assert!(tasks
                .record_initial_workspace_status("repair-child", &status)
                .unwrap());
            match drift {
                Some("remote") => run(
                    &base,
                    &["config", "remote.origin.url", "https://evil.test/elon.git"],
                ),
                Some("branch") => run(&active, &["branch", "-m", "ai/session/elon-self/drifted"]),
                Some("head") => {
                    std::fs::write(active.join("next.txt"), "next\n").unwrap();
                    run(&active, &["add", "next.txt"]);
                    run(&active, &["commit", "-m", "next"]);
                }
                Some("dirty") => {
                    std::fs::write(active.join("dirty.txt"), "dirty\n").unwrap();
                }
                Some("lease") => {
                    crate::node_agent_supervision_worktree_lease::release(
                        &base,
                        &active,
                        "legacy-root",
                    )
                    .unwrap();
                    run(
                        &base,
                        &[
                            "worktree",
                            "lock",
                            "--reason",
                            "foreign",
                            active.to_str().unwrap(),
                        ],
                    );
                }
                Some("occupancy") => {
                    tasks
                        .create(LocalTaskStart {
                            task_id: "other",
                            owner_user_id: "owner",
                            agent_id: "agent",
                            install_id: "install",
                            project_id: "elon-self",
                            channel_id: None,
                            conversation_id: "other",
                            workspace_path: active.to_str().unwrap(),
                            prompt: "other",
                            cli: "codex",
                            runtime_permission: "full_access",
                        })
                        .unwrap();
                }
                _ => {}
            }
            let parent = tasks.get("repair-child").unwrap().unwrap();
            let contract = SupervisionContract {
                protocol: SUPERVISION_PROTOCOL.to_string(),
                supervisor: "codex_desktop".to_string(),
                task_role: "capability_repair".to_string(),
                parent_task_id: Some("legacy-root".to_string()),
                root_task_id: Some("legacy-root".to_string()),
                acceptance_criteria: Vec::new(),
                improvement_policy: "after_task_only".to_string(),
            };
            Self {
                root,
                base,
                active,
                tasks,
                parent,
                contract,
            }
        }

        fn resolve(&self) -> Result<String> {
            validated_descendant_resume_base(
                &self.tasks,
                &self.parent,
                &self.contract,
                "legacy-root",
                "elon-self",
            )
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = crate::node_agent_supervision_worktree_lease::release(
                &self.base,
                &self.active,
                "legacy-root",
            );
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn complete_current_descendant_identity_can_authorize_a_legacy_root_resume() {
        let fixture = Fixture::new(None);
        let resolved = fixture.resolve().unwrap();
        assert!(same_path(Path::new(&resolved), &fixture.base));
    }

    #[test]
    fn descendant_identity_drift_dirty_content_and_occupancy_fail_closed() {
        for drift in [
            "common",
            "remote",
            "branch",
            "head",
            "provenance",
            "base",
            "dirty",
            "lease",
            "occupancy",
            "malformed",
        ] {
            let fixture = Fixture::new(Some(drift));
            assert!(fixture.resolve().is_err(), "{drift} must fail closed");
        }
    }

    fn run(cwd: &Path, args: &[&str]) {
        let result = crate::git_command_error::git_command()
            .args(args)
            .current_dir(cwd)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&result.stderr)
        );
    }

    fn output(cwd: &Path, args: &[&str]) -> String {
        let result = crate::git_command_error::git_command()
            .args(args)
            .current_dir(cwd)
            .output()
            .unwrap();
        assert!(result.status.success(), "git {args:?}");
        String::from_utf8_lossy(&result.stdout).trim().to_string()
    }
}
