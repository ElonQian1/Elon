//! Early persistence of the isolated workspace identity used by supervised recovery.

use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use rusqlite::{params, OptionalExtension, Transaction};

use super::{read_record, select_sql, LocalTaskRecord, LocalTaskStore};

impl LocalTaskStore {
    pub(crate) fn record_initial_workspace_status(
        &self,
        task_id: &str,
        workspace_status: &serde_json::Value,
    ) -> Result<bool> {
        let encoded = serde_json::to_string(workspace_status)?;
        Ok(self.open()?.execute(
            "UPDATE local_tasks SET workspace_status_json = ?2
              WHERE task_id = ?1 AND workspace_status_json IS NULL
                AND completion_event_id IS NULL",
            params![task_id, encoded],
        )? > 0)
    }

    pub(crate) fn list_identity_candidates(&self) -> Result<Vec<LocalTaskRecord>> {
        let conn = self.open()?;
        let mut stmt = conn.prepare(&format!("{} ORDER BY started_at_ms DESC", select_sql()))?;
        let records = stmt
            .query_map([], read_record)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(anyhow::Error::from)?;
        Ok(records)
    }

    pub(crate) fn list_terminal_lease_candidates(&self) -> Result<Vec<LocalTaskRecord>> {
        let conn = self.open()?;
        let mut stmt = conn.prepare(&format!(
            "{} WHERE status IN ('done','failed','canceled','finished','cancel_requested')
             ORDER BY started_at_ms",
            select_sql()
        ))?;
        let records = stmt
            .query_map([], read_record)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(anyhow::Error::from)?;
        Ok(records)
    }
}

/// Refresh a supervised task's durable workspace identity while the caller's
/// terminal-state transaction is still open. Ordinary local tasks are left
/// untouched. A failed refresh preserves every startup identity field and adds
/// a durable fail-closed reason instead of allowing the old HEAD to look final.
pub(super) fn terminal_workspace_status(
    tx: &Transaction<'_>,
    task_id: &str,
) -> Result<Option<String>> {
    let row = tx
        .query_row(
            "SELECT owner_user_id, agent_id, install_id, project_id, workspace_path,
                    workspace_status_json, completion_event_id
               FROM local_tasks WHERE task_id = ?1",
            [task_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, Option<String>>(6)?,
                ))
            },
        )
        .optional()?;
    let Some((_owner, _agent, _install, project, record_path, encoded, completed)) = row else {
        return Ok(None);
    };
    let Some(encoded) = encoded else {
        return Ok(None);
    };
    if completed.is_some() {
        return Ok(Some(encoded));
    }
    let mut status: serde_json::Value = serde_json::from_str(&encoded)?;
    if status
        .get("platform_provenance")
        .and_then(serde_json::Value::as_str)
        != Some("elon.conversation_worktree.v1")
    {
        return Ok(None);
    }

    match validate_terminal_snapshot(tx, task_id, &project, &record_path, &status) {
        Ok(head) => {
            let object = status
                .as_object_mut()
                .ok_or_else(|| anyhow!("supervised workspace_status is not an object"))?;
            object.insert("git_head".into(), serde_json::Value::String(head));
            object.insert(
                "terminal_snapshot_status".into(),
                serde_json::Value::String("trusted".into()),
            );
            object.remove("resume_blocked_reason");
        }
        Err(error) => {
            let object = status
                .as_object_mut()
                .ok_or_else(|| anyhow!("supervised workspace_status is not an object"))?;
            object.insert(
                "terminal_snapshot_status".into(),
                serde_json::Value::String("rejected".into()),
            );
            object.insert(
                "resume_blocked_reason".into(),
                serde_json::Value::String(format!(
                    "终态工作区快照刷新失败，禁止 Resume：{error:#}"
                )),
            );
        }
    }
    serde_json::to_string(&status).map(Some).map_err(Into::into)
}

fn validate_terminal_snapshot(
    tx: &Transaction<'_>,
    task_id: &str,
    project: &str,
    record_path: &str,
    status: &serde_json::Value,
) -> Result<String> {
    required_eq(status, "project_id", project)?;
    let root = required(status, "root_task_id")?;
    let base = PathBuf::from(required(status, "base_workspace_path")?);
    let active = PathBuf::from(required(status, "active_workspace_path")?);
    anyhow::ensure!(
        status.get("isolated").and_then(serde_json::Value::as_bool) == Some(true),
        "启动记录不是隔离 worktree"
    );
    anyhow::ensure!(
        same_path(Path::new(record_path), &active),
        "任务活动路径与启动记录漂移"
    );
    anyhow::ensure!(base.is_dir() && active.is_dir(), "基础或活动工作区不可读取");
    anyhow::ensure!(!same_path(&base, &active), "活动工作区退化为基础工作区");

    let expected_branch = required(status, "branch")?;
    validate_platform_shape(&active, project, expected_branch)?;
    let branch = git(&active, &["branch", "--show-current"])?;
    anyhow::ensure!(
        branch == expected_branch && branch != "main",
        "活动 worktree 分支漂移"
    );
    let top = PathBuf::from(git(&active, &["rev-parse", "--show-toplevel"])?);
    anyhow::ensure!(same_path(&top, &active), "活动路径不是 worktree 根");

    let recorded_common = PathBuf::from(required(status, "git_common_dir")?);
    let current_common = PathBuf::from(git(
        &active,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )?);
    anyhow::ensure!(
        same_path(&recorded_common, &current_common),
        "Git common-dir 漂移"
    );
    let base_common = PathBuf::from(git(
        &base,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )?);
    anyhow::ensure!(
        same_path(&base_common, &current_common),
        "活动 worktree 不属于启动仓库"
    );
    required_eq(
        status,
        "git_remote",
        &git(&base, &["config", "--get", "remote.origin.url"])?,
    )?;

    let expected_lease = crate::node_agent_supervision_worktree_lease::lease_reason(root)?;
    let actual_lease =
        crate::node_agent_supervision_worktree_lease::worktree_lock_reason(&base, &active)?;
    anyhow::ensure!(
        actual_lease.as_deref() == Some(expected_lease.as_str()),
        "root lease 缺失或漂移"
    );

    let occupied: i64 = tx.query_row(
        "SELECT COUNT(*) FROM local_tasks
          WHERE task_id <> ?1 AND workspace_path = ?2
            AND status IN ('running','recovering','reattaching','interrupted','cancel_requested','resume_required')",
        params![task_id, record_path],
        |row| row.get(0),
    )?;
    anyhow::ensure!(occupied == 0, "活动 worktree 存在跨身份任务占用漂移");
    git(&active, &["rev-parse", "--verify", "HEAD^{commit}"])
}

fn validate_platform_shape(active: &Path, project: &str, branch: &str) -> Result<()> {
    let conversation = active
        .file_name()
        .and_then(|part| part.to_str())
        .context("活动路径缺少会话目录")?;
    let project_dir = active
        .parent()
        .and_then(Path::file_name)
        .and_then(|part| part.to_str())
        .context("活动路径缺少项目目录")?;
    let marker = active
        .parent()
        .and_then(Path::parent)
        .and_then(Path::file_name)
        .and_then(|part| part.to_str())
        .context("活动路径缺少平台目录")?;
    let project_part = elon_pc_dev_runtime::safe_path_part(project, "project", 80);
    anyhow::ensure!(
        marker.eq_ignore_ascii_case("conversation-worktrees")
            && project_dir.eq_ignore_ascii_case(&project_part),
        "活动路径不是当前项目的平台隔离 worktree"
    );
    anyhow::ensure!(
        branch == format!("ai/session/{project_part}/{conversation}"),
        "活动路径与平台分支身份不一致"
    );
    Ok(())
}

fn required<'a>(status: &'a serde_json::Value, field: &str) -> Result<&'a str> {
    status
        .get(field)
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("启动记录缺少 {field}"))
}

fn required_eq(status: &serde_json::Value, field: &str, expected: &str) -> Result<()> {
    anyhow::ensure!(
        required(status, field)? == expected,
        "启动记录 {field} 身份漂移"
    );
    Ok(())
}

fn git(cwd: &Path, args: &[&str]) -> Result<String> {
    crate::node_agent_update_checkpoint::git_output(cwd, args)
        .map(|value| value.trim().to_string())
        .with_context(|| format!("无法读取 Git 现场: git {}", args.join(" ")))
}

fn same_path(left: &Path, right: &Path) -> bool {
    crate::node_agent_update_checkpoint::same_path(left, right)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use homecli_proto::{CliCompletionEnvelope, CliCompletionProducerIdentity, CliProjectContext};
    use serde_json::json;
    use uuid::Uuid;

    use super::*;
    use crate::node_agent_local_task_store::LocalTaskStart;

    struct Fixture {
        store: LocalTaskStore,
        root: PathBuf,
        base: PathBuf,
        active: PathBuf,
        initial_head: String,
    }

    impl Fixture {
        fn new(label: &str) -> Self {
            let root = std::env::temp_dir().join(format!(
                "terminal-workspace-{label}-{}",
                Uuid::new_v4().simple()
            ));
            let base = root.join("base");
            let active = root
                .join("conversation-worktrees")
                .join("project")
                .join("root-conversation");
            fs::create_dir_all(&base).unwrap();
            run(&base, &["init"]);
            run(&base, &["config", "user.email", "ai@example.test"]);
            run(&base, &["config", "user.name", "AI Test"]);
            run(
                &base,
                &[
                    "config",
                    "remote.origin.url",
                    "https://example.test/repo.git",
                ],
            );
            fs::write(base.join("seed.txt"), "seed\n").unwrap();
            run(&base, &["add", "seed.txt"]);
            run(&base, &["commit", "-m", "seed"]);
            let branch = "ai/session/project/root-conversation";
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
            crate::node_agent_supervision_worktree_lease::acquire(&base, &active, "root").unwrap();
            let initial_head = output(&active, &["rev-parse", "--verify", "HEAD^{commit}"]);
            let store = LocalTaskStore::new(root.join("tasks.sqlite3"));
            store
                .create(LocalTaskStart {
                    task_id: "task",
                    owner_user_id: "owner",
                    agent_id: "node",
                    install_id: "install",
                    project_id: "project",
                    channel_id: None,
                    conversation_id: "root-conversation",
                    workspace_path: active.to_str().unwrap(),
                    prompt: "work",
                    cli: "codex",
                    runtime_permission: "full_access",
                })
                .unwrap();
            let status = json!({
                "platform_provenance": "elon.conversation_worktree.v1",
                "project_id": "project", "root_task_id": "root",
                "base_workspace_path": base, "active_workspace_path": active,
                "isolated": true, "branch": branch, "git_head": initial_head,
                "git_common_dir": output(&active, &["rev-parse", "--path-format=absolute", "--git-common-dir"]),
                "git_remote": "https://example.test/repo.git",
                "prepare_status": "provisioned_supervised_worktree", "merge_status": "preserved"
            });
            assert!(store
                .record_initial_workspace_status("task", &status)
                .unwrap());
            Self {
                store,
                root,
                base,
                active,
                initial_head,
            }
        }

        fn finish(&self, event: &str) {
            assert!(self.store.finish("owner", &completion(event)).unwrap());
        }

        fn status(&self) -> serde_json::Value {
            self.store
                .get("task")
                .unwrap()
                .unwrap()
                .workspace_status
                .unwrap()
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = crate::node_agent_supervision_worktree_lease::release(
                &self.base,
                &self.active,
                "root",
            );
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn committed_terminal_head_refreshes_atomically_and_replays_idempotently() {
        let fixture = Fixture::new("commit");
        fs::write(fixture.active.join("committed.txt"), "B\n").unwrap();
        run(&fixture.active, &["add", "committed.txt"]);
        run(&fixture.active, &["commit", "-m", "B"]);
        fs::write(fixture.active.join("dirty.txt"), "preserve\n").unwrap();
        let head_b = output(&fixture.active, &["rev-parse", "HEAD"]);
        assert_ne!(head_b, fixture.initial_head);
        fixture.finish("event-B");
        let first = fixture.status();
        assert_eq!(first["git_head"], head_b);
        assert_eq!(first["terminal_snapshot_status"], "trusted");
        assert!(fixture.active.join("dirty.txt").exists());
        fixture.finish("event-B");
        assert_eq!(fixture.status(), first);
    }

    #[test]
    fn identity_and_occupancy_drift_fail_closed_without_replacing_start_head() {
        for drift in ["remote", "branch", "lease", "occupancy"] {
            let fixture = Fixture::new(drift);
            fs::write(fixture.active.join("next.txt"), drift).unwrap();
            run(&fixture.active, &["add", "next.txt"]);
            run(&fixture.active, &["commit", "-m", "next"]);
            match drift {
                "remote" => run(
                    &fixture.base,
                    &["config", "remote.origin.url", "https://evil.test/repo.git"],
                ),
                "branch" => run(
                    &fixture.active,
                    &["branch", "-m", "ai/session/project/drifted"],
                ),
                "lease" => {
                    crate::node_agent_supervision_worktree_lease::release(
                        &fixture.base,
                        &fixture.active,
                        "root",
                    )
                    .unwrap();
                    run(
                        &fixture.base,
                        &[
                            "worktree",
                            "lock",
                            "--reason",
                            "foreign",
                            fixture.active.to_str().unwrap(),
                        ],
                    );
                }
                "occupancy" => {
                    fixture
                        .store
                        .create(LocalTaskStart {
                            task_id: "other",
                            owner_user_id: "other",
                            agent_id: "other-node",
                            install_id: "other-install",
                            project_id: "project",
                            channel_id: None,
                            conversation_id: "other",
                            workspace_path: fixture.active.to_str().unwrap(),
                            prompt: "other",
                            cli: "codex",
                            runtime_permission: "full_access",
                        })
                        .unwrap();
                }
                _ => unreachable!(),
            }
            fixture.finish(&format!("event-{drift}"));
            let status = fixture.status();
            assert_eq!(status["git_head"], fixture.initial_head, "{drift}");
            assert_eq!(status["terminal_snapshot_status"], "rejected", "{drift}");
            assert!(status["resume_blocked_reason"]
                .as_str()
                .unwrap()
                .contains("禁止 Resume"));
        }
    }

    fn completion(event: &str) -> CliCompletionEnvelope {
        CliCompletionEnvelope {
            event_id: event.into(),
            req_id: "task".into(),
            cli: "codex".into(),
            origin: crate::node_agent_completion_outbox::LOCAL_OFFLINE_ORIGIN.into(),
            producer_identity: Some(CliCompletionProducerIdentity {
                owner_user_id: "owner".into(),
                agent_id: "node".into(),
                install_id: "install".into(),
            }),
            project_context: Some(CliProjectContext {
                project_id: "project".into(),
                conversation_id: "root-conversation".into(),
                runtime_permission: Some("full_access".into()),
            }),
            channel_id: None,
            prompt: None,
            final_output: "done".into(),
            exit_ok: true,
            error: None,
            session_id: None,
            prompt_tokens: None,
            cached_input_tokens: None,
            completion_tokens: None,
            reasoning_tokens: None,
            total_tokens: None,
            model: None,
            workspace_status: None,
            created_at_ms: 123,
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
        assert!(result.status.success());
        String::from_utf8_lossy(&result.stdout).trim().to_string()
    }
}
