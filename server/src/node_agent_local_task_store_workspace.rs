//! Early persistence of the isolated workspace identity used by supervised recovery.

use anyhow::{Context, Result};
use rusqlite::{params, OptionalExtension, Transaction};

use super::{now_ms, read_record, select_sql, LocalTaskRecord, LocalTaskStore};

#[path = "node_agent_local_task_store_orphan_migration.rs"]
mod orphan_migration;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TerminalLeaseCursor {
    pub(crate) terminal_at_ms: i64,
    pub(crate) task_id: String,
}

impl TerminalLeaseCursor {
    pub(crate) fn from_record(record: &LocalTaskRecord) -> Self {
        Self {
            terminal_at_ms: record.finished_at_ms.unwrap_or(record.started_at_ms),
            task_id: record.task_id.clone(),
        }
    }
}

impl LocalTaskStore {
    pub(crate) fn list_terminal_repair_candidates(
        &self,
        limit: usize,
    ) -> Result<Vec<LocalTaskRecord>> {
        let conn = self.open()?;
        let mut stmt = conn.prepare(&format!(
            "{} WHERE ((status = 'done' AND completion_event_id IS NOT NULL)
                    OR (status = 'resume_required' AND completion_event_id IS NULL AND error = ?2))
                 AND sync_state IN ('local_only','pending','retrying','rejected')
             ORDER BY COALESCE(finished_at_ms, started_at_ms) DESC, task_id DESC
             LIMIT ?1",
            select_sql()
        ))?;
        let records = stmt
            .query_map(
                params![
                    limit.clamp(1, 1_000) as i64,
                    super::ORPHAN_RUNTIME_RESUME_REQUIRED_REASON
                ],
                read_record,
            )?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(anyhow::Error::from)?;
        Ok(records)
    }

    pub(crate) fn list_stale_runtime_candidates(
        &self,
        started_before_ms: i64,
        limit: usize,
    ) -> Result<Vec<LocalTaskRecord>> {
        let conn = self.open()?;
        let mut stmt = conn.prepare(&format!(
            "{} WHERE status IN ('running','recovering','reattaching','cancel_requested')
                   AND completion_event_id IS NULL AND started_at_ms <= ?1
             ORDER BY started_at_ms, task_id
             LIMIT ?2",
            select_sql()
        ))?;
        let records = stmt
            .query_map(
                params![started_before_ms, limit.clamp(1, 1_000) as i64],
                read_record,
            )?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(anyhow::Error::from)?;
        Ok(records)
    }

    pub(crate) fn list_terminal_journal_drift_candidates(
        &self,
        limit: usize,
    ) -> Result<Vec<LocalTaskRecord>> {
        let conn = self.open()?;
        let mut stmt = conn.prepare(&format!(
            "{} WHERE status = 'resume_required'
                   AND finished_at_ms IS NOT NULL
                   AND completion_event_id IS NULL
             ORDER BY finished_at_ms, task_id
             LIMIT ?1",
            select_sql()
        ))?;
        let records = stmt
            .query_map(params![limit.clamp(1, 1_000) as i64], read_record)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(anyhow::Error::from)?;
        Ok(records)
    }

    /// Installer-only view: persisted cancellation and Resume states need a
    /// fresh executor recheck, but must not widen the live drain candidate set.
    pub(crate) fn list_update_install_candidates(&self) -> Result<Vec<LocalTaskRecord>> {
        let conn = self.open()?;
        let mut stmt = conn.prepare(&format!(
            "{} WHERE status IN ('running','recovering','reattaching','cancel_requested','resume_required')
             ORDER BY started_at_ms",
            select_sql()
        ))?;
        let records = stmt
            .query_map([], read_record)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(anyhow::Error::from)?;
        Ok(records)
    }

    /// Atomically preserve the workspace/root recovery context while moving a
    /// dead sidecar task out of the misleading running state.
    pub(crate) fn mark_stale_sidecar_resume_required(
        &self,
        task_id: &str,
        reason: &str,
        recovery_context: &serde_json::Value,
    ) -> Result<bool> {
        let mut conn = self.open()?;
        let tx = conn.transaction()?;
        let workspace_status: Option<Option<String>> = tx
            .query_row(
                "SELECT workspace_status_json FROM local_tasks WHERE task_id = ?1",
                params![task_id],
                |row| row.get(0),
            )
            .optional()?;
        let encoded = workspace_status
            .context("stale sidecar task does not exist")?
            .context("stale sidecar task is missing workspace_status_json")?;
        let mut workspace_status: serde_json::Value = serde_json::from_str(&encoded)
            .context("stale sidecar task has invalid workspace_status_json")?;
        let object = workspace_status
            .as_object_mut()
            .context("stale sidecar workspace_status_json is not an object")?;
        object.insert("restart_recovery".to_string(), recovery_context.clone());
        let encoded = serde_json::to_string(&workspace_status)?;
        let changed = tx.execute(
            "UPDATE local_tasks
                SET status = 'resume_required', error = ?2,
                    finished_at_ms = ?3, sync_state = 'local_only',
                    workspace_status_json = ?4
              WHERE task_id = ?1
                AND completion_event_id IS NULL
                AND status IN ('running','recovering','reattaching')",
            params![task_id, reason, now_ms(), encoded],
        )?;
        tx.commit()?;
        Ok(changed > 0)
    }

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

    pub(crate) fn has_competing_workspace_occupancy(
        &self,
        task_id: &str,
        workspace_path: &str,
    ) -> Result<bool> {
        let count: i64 = self.open()?.query_row(
            "SELECT COUNT(*) FROM local_tasks
              WHERE task_id <> ?1 AND workspace_path = ?2
                AND status IN ('running','recovering','reattaching','interrupted','cancel_requested','resume_required')",
            params![task_id, workspace_path],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    pub(crate) fn list_terminal_lease_candidates_page(
        &self,
        cursor: Option<&TerminalLeaseCursor>,
        limit: usize,
    ) -> Result<Vec<LocalTaskRecord>> {
        let conn = self.open()?;
        let limit = limit.clamp(1, 100) as i64;
        let records = if let Some(cursor) = cursor {
            let mut stmt = conn.prepare(&format!(
                "{} WHERE status IN ('done','failed','canceled','cancelled','finished','cancel_requested')
                   AND (COALESCE(finished_at_ms, started_at_ms) < ?1
                     OR (COALESCE(finished_at_ms, started_at_ms) = ?1 AND task_id < ?2))
                 ORDER BY COALESCE(finished_at_ms, started_at_ms) DESC, task_id DESC
                 LIMIT ?3",
                select_sql()
            ))?;
            let records = stmt
                .query_map(
                    params![cursor.terminal_at_ms, &cursor.task_id, limit],
                    read_record,
                )?
                .collect::<rusqlite::Result<Vec<_>>>()
                .map_err(anyhow::Error::from)?;
            records
        } else {
            let mut stmt = conn.prepare(&format!(
                "{} WHERE status IN ('done','failed','canceled','cancelled','finished','cancel_requested')
                 ORDER BY COALESCE(finished_at_ms, started_at_ms) DESC, task_id DESC
                 LIMIT ?1",
                select_sql()
            ))?;
            let records = stmt
                .query_map(params![limit], read_record)?
                .collect::<rusqlite::Result<Vec<_>>>()
                .map_err(anyhow::Error::from)?;
            records
        };
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
    trusted: Option<
        &crate::node_agent_supervision_terminal_lease_safety::VerifiedTerminalLeaseIdentity,
    >,
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
    let status: serde_json::Value = serde_json::from_str(&encoded)?;
    if status
        .get("platform_provenance")
        .and_then(serde_json::Value::as_str)
        != Some("elon.conversation_worktree.v1")
    {
        anyhow::ensure!(
            trusted.is_none(),
            "ordinary task received a supervised terminal snapshot"
        );
        return Ok(None);
    }

    if let Some(trusted) = trusted {
        return trusted
            .trusted_workspace_status(task_id, &project, &record_path, &status)
            .map(Some);
    }
    if completed.is_some() {
        return Ok(Some(encoded));
    }

    anyhow::bail!("supervised terminal refresh requires guarded reconciliation")
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
    };

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
    fn generic_completion_cannot_create_supervised_trusted_snapshot() {
        let fixture = Fixture::new("generic-rejected");
        let error = fixture
            .store
            .finish("owner", &completion("event-generic"))
            .expect_err("generic completion must use the guarded reconciler");
        assert!(error.to_string().contains("guarded reconciliation"));
        let record = fixture.store.get("task").unwrap().unwrap();
        assert_eq!(record.status, "running");
        assert_eq!(record.completion_event_id, None);
        assert_eq!(fixture.status()["git_head"], fixture.initial_head);
    }

    #[test]
    fn normal_task_losing_its_runtime_handle_preserves_safe_resume_identity() {
        let fixture = Fixture::new("stale-sidecar");
        let context = json!({
            "root_task_id": "root", "sidecar_session_id": "sidecar-1",
            "journal_cursor": 42, "sidecar_output_offset": 84,
            "sidecar_output_sequence": 21, "runtime_handle_present": false,
            "journal_preserved": true, "workspace_preserved": true,
            "root_lease_preserved": true
        });
        assert!(fixture
            .store
            .mark_stale_sidecar_resume_required("task", "dead sidecar", &context)
            .unwrap());
        let record = fixture.store.get("task").unwrap().unwrap();
        assert_eq!(record.status, "resume_required");
        let status = record.workspace_status.unwrap();
        assert_eq!(
            status["platform_provenance"],
            "elon.conversation_worktree.v1"
        );
        assert_eq!(status["root_task_id"], "root");
        assert_eq!(status["git_head"], fixture.initial_head);
        assert_eq!(status["restart_recovery"], context);
        assert!(fixture.active.is_dir());
        assert!(
            crate::node_agent_supervision_worktree_lease::worktree_lock_reason(
                &fixture.base,
                &fixture.active,
            )
            .unwrap()
            .is_some()
        );
    }

    #[test]
    fn orphan_resume_required_is_a_terminal_repair_candidate_but_other_resume_is_not() {
        let fixture = Fixture::new("orphan-terminal-repair-candidate");
        assert!(fixture
            .store
            .mark_one_stale_without_runtime("task", i64::MAX)
            .unwrap());
        let candidates = fixture.store.list_terminal_repair_candidates(10).unwrap();
        assert_eq!(
            candidates
                .iter()
                .map(|record| record.task_id.as_str())
                .collect::<Vec<_>>(),
            vec!["task"]
        );

        fixture
            .store
            .open()
            .unwrap()
            .execute(
                "UPDATE local_tasks SET error = 'manual resume still required' WHERE task_id = 'task'",
                [],
            )
            .unwrap();
        assert!(fixture
            .store
            .list_terminal_repair_candidates(10)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn stale_sidecar_rejects_missing_or_corrupt_workspace_identity_without_mutation() {
        for (label, raw) in [("missing", None), ("corrupt", Some("{not-json"))] {
            let fixture = Fixture::new(label);
            fixture
                .store
                .open()
                .unwrap()
                .execute(
                    "UPDATE local_tasks SET workspace_status_json = ?1 WHERE task_id = 'task'",
                    [raw],
                )
                .unwrap();
            let before: (String, Option<String>) = fixture
                .store
                .open()
                .unwrap()
                .query_row(
                    "SELECT status, workspace_status_json FROM local_tasks WHERE task_id = 'task'",
                    [],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .unwrap();
            assert!(fixture
                .store
                .mark_stale_sidecar_resume_required(
                    "task",
                    "dead sidecar",
                    &json!({"state":"resume_required"}),
                )
                .is_err());
            let after: (String, Option<String>) = fixture
                .store
                .open()
                .unwrap()
                .query_row(
                    "SELECT status, workspace_status_json FROM local_tasks WHERE task_id = 'task'",
                    [],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .unwrap();
            assert_eq!(
                after, before,
                "{label} identity must remain byte-for-byte intact"
            );
        }
    }

    #[test]
    fn stale_sidecar_does_not_rewrite_cancel_requested_semantics() {
        let fixture = Fixture::new("stale-cancel");
        assert!(fixture.store.mark_cancel_requested("task").unwrap());
        let before = fixture.status();
        assert!(!fixture
            .store
            .mark_stale_sidecar_resume_required(
                "task",
                "dead sidecar",
                &json!({"state":"resume_required"}),
            )
            .unwrap());
        let record = fixture.store.get("task").unwrap().unwrap();
        assert_eq!(record.status, "cancel_requested");
        assert_eq!(record.workspace_status.unwrap(), before);
    }

    #[test]
    fn terminal_pages_use_finish_time_include_cancelled_and_reach_empty_page() {
        let root = std::env::temp_dir().join(format!("terminal-pages-{}", Uuid::new_v4().simple()));
        let store = LocalTaskStore::new(root.join("tasks.sqlite3"));
        for task_id in [
            "old-start-new-finish",
            "recent-start-old-finish",
            "cancelled",
        ] {
            store
                .create(LocalTaskStart {
                    task_id,
                    owner_user_id: "owner",
                    agent_id: "node",
                    install_id: "install",
                    project_id: "project",
                    channel_id: None,
                    conversation_id: task_id,
                    workspace_path: root.to_str().unwrap(),
                    prompt: "work",
                    cli: "codex",
                    runtime_permission: "full_access",
                })
                .unwrap();
        }
        let conn = store.open().unwrap();
        conn.execute(
            "UPDATE local_tasks SET status='done', started_at_ms=1, finished_at_ms=300 WHERE task_id='old-start-new-finish'",
            [],
        ).unwrap();
        conn.execute(
            "UPDATE local_tasks SET status='done', started_at_ms=200, finished_at_ms=100 WHERE task_id='recent-start-old-finish'",
            [],
        ).unwrap();
        conn.execute(
            "UPDATE local_tasks SET status='cancelled', started_at_ms=150, finished_at_ms=200 WHERE task_id='cancelled'",
            [],
        ).unwrap();
        drop(conn);

        let first = store.list_terminal_lease_candidates_page(None, 2).unwrap();
        assert_eq!(first[0].task_id, "old-start-new-finish");
        assert_eq!(first[1].task_id, "cancelled");
        let cursor = TerminalLeaseCursor::from_record(first.last().unwrap());
        let second = store
            .list_terminal_lease_candidates_page(Some(&cursor), 2)
            .unwrap();
        assert_eq!(second[0].task_id, "recent-start-old-finish");
        let cursor = TerminalLeaseCursor::from_record(second.last().unwrap());
        assert!(store
            .list_terminal_lease_candidates_page(Some(&cursor), 2)
            .unwrap()
            .is_empty());
        let _ = fs::remove_dir_all(root);
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
