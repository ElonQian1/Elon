//! Transactional provenance migration for a proved orphaned root workspace.

use std::path::Path;

use anyhow::{Context, Result};
use rusqlite::{params, OptionalExtension};

use super::super::{read_record, select_sql, LocalTaskRecord, LocalTaskStore};

impl LocalTaskStore {
    pub(crate) fn migrate_orphaned_root_workspace(
        &self,
        parent: &LocalTaskRecord,
        workspace: &crate::pc_workspace_provisioner::ConversationWorkspaceResult,
        migration: &crate::node_agent_local_task_resume::OrphanedWorkspaceMigration,
    ) -> Result<LocalTaskRecord> {
        let mut conn = self.open()?;
        let tx = conn.transaction()?;
        let current = tx
            .query_row(
                &format!("{} WHERE task_id = ?1", select_sql()),
                [&parent.task_id],
                read_record,
            )
            .optional()?
            .context("孤儿迁移根任务已从 registry 消失")?;
        anyhow::ensure!(
            current.owner_user_id == parent.owner_user_id
                && current.agent_id == parent.agent_id
                && current.install_id == parent.install_id
                && current.project_id == parent.project_id
                && current.workspace_path == parent.workspace_path
                && current.workspace_status == parent.workspace_status,
            "孤儿迁移前根任务 registry 身份已发生变化"
        );
        anyhow::ensure!(
            current.finished_at_ms.is_some()
                && matches!(
                    current.status.as_str(),
                    "done" | "failed" | "canceled" | "interrupted" | "resume_required"
                ),
            "孤儿迁移根任务不再处于可靠终态"
        );
        let root_id = workspace
            .supervision_root_task_id
            .as_deref()
            .context("新 worktree 缺少 root lease 身份")?;
        anyhow::ensure!(
            root_id == current.task_id,
            "孤儿迁移只允许 requirement 根任务取得自身 root lease"
        );
        let base = workspace
            .base_workspace_path
            .as_deref()
            .context("新 worktree 缺少 base identity")?;
        let branch = workspace
            .branch
            .as_deref()
            .context("新 worktree 缺少 branch identity")?;
        let head = crate::node_agent_update_checkpoint::git_output(
            Path::new(&workspace.workspace_path),
            &["rev-parse", "--verify", "HEAD^{commit}"],
        )
        .context("无法读取新 worktree HEAD")?;
        anyhow::ensure!(
            head.eq_ignore_ascii_case(&migration.target_head),
            "新 worktree HEAD 与迁移目标不一致"
        );

        let mut status = current
            .workspace_status
            .clone()
            .context("孤儿迁移根任务缺少 workspace_status")?;
        let object = status
            .as_object_mut()
            .context("孤儿迁移 workspace_status 不是对象")?;
        object.insert(
            "platform_provenance".into(),
            "elon.conversation_worktree.v1".into(),
        );
        object.insert("project_id".into(), current.project_id.clone().into());
        object.insert("root_task_id".into(), root_id.into());
        object.insert("base_workspace_path".into(), base.into());
        object.insert(
            "active_workspace_path".into(),
            workspace.workspace_path.clone().into(),
        );
        object.insert("branch".into(), branch.into());
        object.insert("git_head".into(), head.into());
        object.insert(
            "base_revision".into(),
            crate::node_agent_update_checkpoint::git_output(
                Path::new(base),
                &["rev-parse", "--verify", "HEAD^{commit}"],
            )
            .context("无法读取授权 base revision")?
            .into(),
        );
        object.insert(
            "git_common_dir".into(),
            crate::node_agent_update_checkpoint::git_output(
                Path::new(&workspace.workspace_path),
                &["rev-parse", "--path-format=absolute", "--git-common-dir"],
            )
            .context("无法读取新 worktree common-dir")?
            .into(),
        );
        object.insert(
            "git_remote".into(),
            crate::node_agent_update_checkpoint::git_output(
                Path::new(base),
                &["config", "--get", "remote.origin.url"],
            )
            .context("无法读取授权 base remote")?
            .into(),
        );
        object.insert(
            "prepare_status".into(),
            "orphaned_workspace_controlled_migration".into(),
        );
        object.insert(
            "provenance_derivation".into(),
            "orphaned_workspace_controlled_migration_branch_head".into(),
        );
        object.insert(
            "migration_source_active_workspace_path".into(),
            migration.source_path.clone().into(),
        );
        object.insert(
            "migration_source_branch".into(),
            migration.source_branch.clone().into(),
        );
        object.insert(
            "migration_source_recorded_head".into(),
            migration.recorded_head.clone().into(),
        );
        object.insert("terminal_snapshot_status".into(), "trusted".into());
        object.remove("resume_blocked_reason");
        let encoded = serde_json::to_string(&status)?;
        let changed = tx.execute(
            "UPDATE local_tasks SET workspace_path = ?2, workspace_status_json = ?3
              WHERE task_id = ?1 AND workspace_path = ?4 AND workspace_status_json = ?5",
            params![
                current.task_id,
                workspace.workspace_path,
                encoded,
                current.workspace_path,
                serde_json::to_string(current.workspace_status.as_ref().unwrap())?,
            ],
        )?;
        anyhow::ensure!(changed == 1, "孤儿迁移根任务 provenance CAS 失败");
        tx.commit()?;
        self.get(&parent.task_id)?
            .context("孤儿迁移根任务更新后不可读")
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use serde_json::json;
    use uuid::Uuid;

    use super::*;
    use crate::node_agent_local_task_store::LocalTaskStart;

    #[test]
    fn controlled_migration_rebinds_root_provenance_and_remains_resumable() {
        let root = std::env::temp_dir().join(format!(
            "store-orphan-migration-{}",
            Uuid::new_v4().simple()
        ));
        let base = root.join("base");
        let source = root.join("conversation-worktrees/project/root-conversation");
        let migrated = root.join("conversation-worktrees/project/resume-conversation");
        fs::create_dir_all(source.parent().unwrap()).unwrap();
        fs::create_dir_all(&base).unwrap();
        git(&base, &["init"]);
        git(&base, &["config", "user.email", "tests@example.invalid"]);
        git(&base, &["config", "user.name", "Tests"]);
        git(
            &base,
            &[
                "config",
                "remote.origin.url",
                "https://example.test/elon.git",
            ],
        );
        fs::write(base.join("seed.txt"), "seed\n").unwrap();
        git(&base, &["add", "seed.txt"]);
        git(&base, &["commit", "-m", "seed"]);
        let head = output(&base, &["rev-parse", "HEAD"]);
        git(
            &base,
            &[
                "worktree",
                "add",
                "-b",
                "ai/session/project/root-conversation",
                source.to_str().unwrap(),
                &head,
            ],
        );

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
                workspace_path: source.to_str().unwrap(),
                prompt: "task",
                cli: "codex",
                runtime_permission: "full_access",
            })
            .unwrap();
        store
            .record_initial_workspace_status(
                "task",
                &json!({
                    "platform_provenance":"elon.conversation_worktree.v1",
                    "project_id":"project", "root_task_id":"task",
                    "base_workspace_path":base, "active_workspace_path":source,
                    "isolated":true, "branch":"ai/session/project/root-conversation",
                    "git_head":head
                }),
            )
            .unwrap();
        store
            .open()
            .unwrap()
            .execute(
                "UPDATE local_tasks SET status='done', finished_at_ms=2 WHERE task_id='task'",
                [],
            )
            .unwrap();
        let parent = store.get("task").unwrap().unwrap();

        git(
            &base,
            &[
                "worktree",
                "add",
                "-b",
                "ai/session/project/resume-conversation",
                migrated.to_str().unwrap(),
                &head,
            ],
        );
        crate::node_agent_supervision_worktree_lease::acquire(&base, &migrated, "task").unwrap();
        let workspace = crate::pc_workspace_provisioner::ConversationWorkspaceResult {
            base_workspace_path: Some(base.to_string_lossy().into_owned()),
            workspace_path: migrated.to_string_lossy().into_owned(),
            isolated: true,
            branch: Some("ai/session/project/resume-conversation".into()),
            supervision_root_task_id: Some("task".into()),
        };
        let migration = crate::node_agent_local_task_resume::OrphanedWorkspaceMigration {
            source_path: source.to_string_lossy().into_owned(),
            source_branch: "ai/session/project/root-conversation".into(),
            recorded_head: head.clone(),
            target_head: head,
        };
        let record = store
            .migrate_orphaned_root_workspace(&parent, &workspace, &migration)
            .unwrap();
        assert!(crate::node_agent_update_checkpoint::same_path(
            Path::new(&record.workspace_path),
            &migrated
        ));
        let status = record.workspace_status.as_ref().unwrap();
        assert_eq!(
            status["prepare_status"],
            "orphaned_workspace_controlled_migration"
        );
        assert_eq!(
            status["migration_source_active_workspace_path"].as_str(),
            Some(source.to_string_lossy().as_ref())
        );
        assert!(source.is_dir(), "migration must preserve the source");

        let parent_contract = contract("requirement", None);
        let resume_contract = contract("resume_original", Some("task"));
        crate::node_agent_local_task_resume::inspect_resume_workspace(
            &resume_contract,
            &record,
            Some(&parent_contract),
            None,
            "project",
            base.to_str().unwrap(),
        )
        .expect("migrated root provenance should remain resumable");
        crate::node_agent_supervision_worktree_lease::release(&base, &migrated, "task").unwrap();
        let _ = fs::remove_dir_all(root);
    }

    fn contract(
        role: &str,
        parent: Option<&str>,
    ) -> crate::node_agent_local_task_supervision::SupervisionContract {
        crate::node_agent_local_task_supervision::SupervisionContract {
            protocol: crate::node_agent_local_task_supervision::SUPERVISION_PROTOCOL.into(),
            supervisor: "codex_desktop".into(),
            task_role: role.into(),
            parent_task_id: parent.map(str::to_string),
            root_task_id: Some("task".into()),
            acceptance_criteria: Vec::new(),
            improvement_policy: "after_task_only".into(),
        }
    }

    fn git(cwd: &Path, args: &[&str]) {
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
