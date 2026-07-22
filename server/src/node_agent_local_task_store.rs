use crate::node_agent_task_journal_events::completion_terminal_status;
use anyhow::{bail, Context, Result};
use homecli_proto::CliCompletionEnvelope;
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use std::{collections::HashSet, path::PathBuf};

#[path = "node_agent_local_post_idempotency.rs"]
pub(crate) mod idempotency;
#[path = "node_agent_local_task_store_reconcile.rs"]
pub(crate) mod reconcile;
#[path = "node_agent_local_task_store_safety.rs"]
mod safety;
#[path = "node_agent_local_task_store_terminal.rs"]
mod terminal;
#[path = "node_agent_local_task_store_update.rs"]
mod update;
#[path = "node_agent_local_task_store_workspace.rs"]
pub(crate) mod workspace;
#[derive(Clone, Debug)]
pub(crate) struct LocalTaskStore {
    path: PathBuf,
}

#[derive(Debug)]
pub(crate) struct LocalTaskStart<'a> {
    pub task_id: &'a str,
    pub owner_user_id: &'a str,
    pub agent_id: &'a str,
    pub install_id: &'a str,
    pub project_id: &'a str,
    pub channel_id: Option<&'a str>,
    pub conversation_id: &'a str,
    pub workspace_path: &'a str,
    pub prompt: &'a str,
    pub cli: &'a str,
    pub runtime_permission: &'a str,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct LocalTaskRecord {
    pub task_id: String,
    pub owner_user_id: String,
    pub agent_id: String,
    pub install_id: String,
    pub project_id: String,
    pub channel_id: Option<String>,
    pub conversation_id: String,
    pub workspace_path: String,
    pub prompt: String,
    pub cli: String,
    pub runtime_permission: String,
    pub execution_origin: String,
    pub billing_source: String,
    pub status: String,
    pub error: Option<String>,
    pub final_reply: Option<String>,
    pub model: Option<String>,
    pub codex_session_id: Option<String>,
    pub input_tokens: Option<i64>,
    pub cached_input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub reasoning_tokens: Option<i64>,
    pub total_tokens: Option<i64>,
    pub workspace_status: Option<serde_json::Value>,
    pub sync_state: String,
    pub completion_event_id: Option<String>,
    pub started_at_ms: i64,
    pub finished_at_ms: Option<i64>,
    pub server_ack_at_ms: Option<i64>,
}

impl LocalTaskStore {
    pub(crate) fn default() -> Self {
        Self {
            path: super::state_path().with_file_name("local-tasks.sqlite3"),
        }
    }

    #[cfg(test)]
    pub(crate) fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub(crate) fn create(&self, start: LocalTaskStart<'_>) -> Result<LocalTaskRecord> {
        let conn = self.open()?;
        let now = now_ms();
        conn.execute(
            "INSERT OR IGNORE INTO local_tasks (
               task_id, owner_user_id, agent_id, install_id, project_id, channel_id,
               conversation_id, workspace_path, prompt, cli, runtime_permission,
               execution_origin, billing_source, status, sync_state, started_at_ms
             ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,'local_offline','own_codex','running','local_only',?12)",
            params![
                start.task_id,
                start.owner_user_id,
                start.agent_id,
                start.install_id,
                start.project_id,
                clean_optional(start.channel_id),
                start.conversation_id,
                start.workspace_path,
                start.prompt,
                start.cli,
                start.runtime_permission,
                now,
            ],
        )?;
        let record = self
            .get_for_owner(start.owner_user_id, start.task_id)?
            .context("local task inserted but not readable")?;
        if record.agent_id != start.agent_id
            || record.install_id != start.install_id
            || record.project_id != start.project_id
            || record.conversation_id != start.conversation_id
        {
            bail!("local task id already belongs to a different durable identity");
        }
        Ok(record)
    }

    pub(crate) fn finish(
        &self,
        owner_user_id: &str,
        completion: &CliCompletionEnvelope,
    ) -> Result<bool> {
        let producer = completion
            .producer_identity
            .as_ref()
            .context("local completion is missing producer identity")?;
        if producer.owner_user_id != owner_user_id {
            bail!("local completion producer owner does not match task owner");
        }
        self.finish_scoped(Some(owner_user_id), completion)
    }

    fn finish_scoped(
        &self,
        owner_user_id: Option<&str>,
        completion: &CliCompletionEnvelope,
    ) -> Result<bool> {
        let project_id = completion
            .project_context
            .as_ref()
            .map(|context| context.project_id.as_str());
        let conversation_id = completion
            .project_context
            .as_ref()
            .map(|context| context.conversation_id.as_str());
        let producer = completion
            .producer_identity
            .as_ref()
            .context("local completion is missing producer identity")?;
        self.finish_scoped_with_context(
            owner_user_id,
            Some(producer.agent_id.as_str()),
            Some(producer.install_id.as_str()),
            project_id,
            conversation_id,
            completion,
            None,
        )
    }

    fn finish_scoped_with_context(
        &self,
        owner_user_id: Option<&str>,
        agent_id: Option<&str>,
        install_id: Option<&str>,
        project_id: Option<&str>,
        conversation_id: Option<&str>,
        completion: &CliCompletionEnvelope,
        trusted: Option<
            &crate::node_agent_supervision_terminal_lease_safety::VerifiedTerminalLeaseIdentity,
        >,
    ) -> Result<bool> {
        let mut conn = self.open()?;
        let completion_workspace_status = completion
            .workspace_status
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?;
        let tx = conn.transaction()?;
        reconcile::ensure_same_event_is_immutable(&tx, completion)?;
        let terminal_workspace_status =
            workspace::terminal_workspace_status(&tx, &completion.req_id, trusted)?;
        let supervised_workspace_refresh = terminal_workspace_status.is_some();
        let workspace_status = terminal_workspace_status.or(completion_workspace_status);
        let terminal_status =
            completion_terminal_status(completion.exit_ok, completion.error.as_deref());
        let changed = tx.execute(
            "UPDATE local_tasks
                SET status = CASE
                        WHEN status IN ('done','failed','canceled') THEN status
                        ELSE ?1
                    END,
                    error = CASE
                        WHEN status IN ('done','failed','canceled') THEN error
                        ELSE ?2
                    END,
                    final_reply = COALESCE(final_reply, ?3),
                    model = COALESCE(model, ?4),
                    codex_session_id = COALESCE(codex_session_id, ?5),
                    input_tokens = COALESCE(input_tokens, ?6),
                    cached_input_tokens = COALESCE(cached_input_tokens, ?7),
                    output_tokens = COALESCE(output_tokens, ?8),
                    reasoning_tokens = COALESCE(reasoning_tokens, ?9),
                    total_tokens = COALESCE(total_tokens, ?10),
                    workspace_status_json = CASE WHEN ?20 THEN ?11
                        ELSE COALESCE(workspace_status_json, ?11) END,
                    sync_state = CASE
                        WHEN sync_state IN ('synced','rejected') THEN sync_state
                        ELSE 'pending'
                    END,
                    completion_event_id = COALESCE(completion_event_id, ?12),
                    finished_at_ms = COALESCE(finished_at_ms, ?13)
              WHERE task_id = ?15
                AND execution_origin = 'local_offline'
                AND (?14 IS NULL OR owner_user_id = ?14)
                AND (?16 IS NULL OR project_id = ?16)
                AND (?17 IS NULL OR conversation_id = ?17)
                AND (?18 IS NULL OR agent_id = ?18)
                AND (?19 IS NULL OR install_id = ?19)
                AND (completion_event_id IS NULL OR completion_event_id = ?12)
                AND status IN ('running','recovering','reattaching','interrupted','cancel_requested','canceled','done','failed','resume_required')",
            params![
                terminal_status,
                completion.error,
                trim_to_option(&completion.final_output),
                completion.model,
                completion.session_id,
                completion.prompt_tokens.map(clamp_u64),
                completion.cached_input_tokens.map(clamp_u64),
                completion.completion_tokens.map(clamp_u64),
                completion.reasoning_tokens.map(clamp_u64),
                completion.total_tokens.map(clamp_u64),
                workspace_status,
                completion.event_id,
                completion.created_at_ms.min(i64::MAX as u64) as i64,
                owner_user_id,
                completion.req_id,
                project_id,
                conversation_id,
                agent_id,
                install_id,
                supervised_workspace_refresh,
            ],
        )?;
        tx.commit()?;
        Ok(changed > 0)
    }

    /// Preserve one-click recovery when a restart loses in-memory child handles.
    pub(crate) fn interrupt_lingering_running(
        &self,
        durable_req_ids: &HashSet<String>,
        started_before_ms: i64,
    ) -> Result<usize> {
        let mut conn = self.open()?;
        let tx = conn.transaction()?;
        let running_ids = {
            let mut stmt = tx.prepare(
                "SELECT task_id FROM local_tasks
                  WHERE status = 'running' AND completion_event_id IS NULL
                    AND started_at_ms <= ?1",
            )?;
            let ids = stmt
                .query_map([started_before_ms], |row| row.get::<_, String>(0))?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            ids
        };
        let mut changed = 0;
        for task_id in running_ids {
            if durable_req_ids.contains(&task_id) {
                continue;
            }
            changed += tx.execute(
                "UPDATE local_tasks
                    SET status = 'resume_required',
                        error = '节点进程重启后需要继续：工作区与 journal 已保留，请点击 Resume 让 Codex 检查现场后续跑',
                        sync_state = 'local_only',
                        finished_at_ms = ?1
                  WHERE task_id = ?2
                    AND status = 'running'
                    AND completion_event_id IS NULL
                    AND started_at_ms <= ?3",
                params![now_ms(), task_id, started_before_ms],
            )?;
        }
        tx.commit()?;
        Ok(changed)
    }

    pub(crate) fn get(&self, task_id: &str) -> Result<Option<LocalTaskRecord>> {
        self.open()?
            .query_row(
                &format!("{} WHERE task_id = ?1", select_sql()),
                params![task_id],
                read_record,
            )
            .optional()
            .map_err(Into::into)
    }

    pub(crate) fn mark_recovering(&self, task_id: &str, reason: &str) -> Result<bool> {
        Ok(self.open()?.execute(
            "UPDATE local_tasks SET status = 'recovering', error = ?2
              WHERE task_id = ?1
                AND completion_event_id IS NULL
                AND status IN ('running','recovering','reattaching')",
            params![task_id, reason],
        )? > 0)
    }

    pub(crate) fn mark_recovery_running(&self, task_id: &str) -> Result<bool> {
        Ok(self.open()?.execute(
            "UPDATE local_tasks
                SET status = 'running', error = NULL, finished_at_ms = NULL
              WHERE task_id = ?1
                AND completion_event_id IS NULL
                AND status IN ('recovering','reattaching','resume_required')",
            params![task_id],
        )? > 0)
    }

    pub(crate) fn mark_recovery_blocked(&self, task_id: &str, reason: &str) -> Result<bool> {
        Ok(self.open()?.execute(
            "UPDATE local_tasks
                SET status = 'resume_required', error = ?2,
                    finished_at_ms = ?3, sync_state = 'local_only'
              WHERE task_id = ?1
                AND completion_event_id IS NULL
                AND status IN ('running','recovering','reattaching')",
            params![task_id, reason, now_ms()],
        )? > 0)
    }

    pub(crate) fn mark_cancel_requested(&self, task_id: &str) -> Result<bool> {
        Ok(self.open()?.execute(
            "UPDATE local_tasks
                SET status = 'cancel_requested', error = '取消请求已持久化，正在等待执行器确认终态',
                    finished_at_ms = NULL, sync_state = 'local_only'
              WHERE task_id = ?1 AND completion_event_id IS NULL
                AND status IN ('running','recovering','reattaching','interrupted','resume_required')",
            params![task_id],
        )? > 0)
    }

    pub(crate) fn mark_synced(&self, event_id: &str) -> Result<bool> {
        let changed = self.open()?.execute(
            "UPDATE local_tasks SET sync_state = 'synced', server_ack_at_ms = ?1
              WHERE completion_event_id = ?2",
            params![now_ms(), event_id],
        )?;
        Ok(changed > 0)
    }

    pub(crate) fn mark_sync_error(&self, event_id: &str, retryable: bool) -> Result<bool> {
        let changed = self.open()?.execute(
            "UPDATE local_tasks SET sync_state = ?1 WHERE completion_event_id = ?2",
            params![if retryable { "retrying" } else { "rejected" }, event_id],
        )?;
        Ok(changed > 0)
    }

    pub(crate) fn list_for_owner(
        &self,
        owner_user_id: &str,
        limit: usize,
    ) -> Result<Vec<LocalTaskRecord>> {
        let conn = self.open()?;
        let mut stmt = conn.prepare(&format!(
            "{} WHERE owner_user_id = ?1 ORDER BY started_at_ms DESC LIMIT ?2",
            select_sql()
        ))?;
        let records = stmt
            .query_map(
                params![owner_user_id, limit.clamp(1, 100) as i64],
                read_record,
            )?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(anyhow::Error::from)?;
        Ok(records)
    }

    pub(crate) fn get_for_owner(
        &self,
        owner_user_id: &str,
        task_id: &str,
    ) -> Result<Option<LocalTaskRecord>> {
        self.open()?
            .query_row(
                &format!("{} WHERE owner_user_id = ?1 AND task_id = ?2", select_sql()),
                params![owner_user_id, task_id],
                read_record,
            )
            .optional()
            .map_err(Into::into)
    }

    pub(crate) fn pending_count(&self, owner_user_id: &str) -> Result<i64> {
        self.open()?
            .query_row(
                "SELECT COUNT(*) FROM local_tasks
                  WHERE owner_user_id = ?1 AND sync_state IN ('pending','retrying')",
                params![owner_user_id],
                |row| row.get(0),
            )
            .map_err(Into::into)
    }

    fn open(&self) -> Result<Connection> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(&self.path)
            .with_context(|| format!("open local task store {}", self.path.display()))?;
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "FULL")?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS local_tasks (
                task_id TEXT PRIMARY KEY,
                owner_user_id TEXT NOT NULL,
                agent_id TEXT NOT NULL,
                install_id TEXT NOT NULL,
                project_id TEXT NOT NULL,
                channel_id TEXT,
                conversation_id TEXT NOT NULL,
                workspace_path TEXT NOT NULL,
                prompt TEXT NOT NULL,
                cli TEXT NOT NULL,
                runtime_permission TEXT NOT NULL,
                execution_origin TEXT NOT NULL,
                billing_source TEXT NOT NULL,
                status TEXT NOT NULL,
                error TEXT,
                final_reply TEXT,
                model TEXT,
                codex_session_id TEXT,
                input_tokens INTEGER,
                cached_input_tokens INTEGER,
                output_tokens INTEGER,
                reasoning_tokens INTEGER,
                total_tokens INTEGER,
                workspace_status_json TEXT,
                sync_state TEXT NOT NULL,
                completion_event_id TEXT UNIQUE,
                started_at_ms INTEGER NOT NULL,
                finished_at_ms INTEGER,
                server_ack_at_ms INTEGER
             );
             CREATE INDEX IF NOT EXISTS idx_local_tasks_owner_started
                ON local_tasks(owner_user_id, started_at_ms DESC);
             CREATE INDEX IF NOT EXISTS idx_local_tasks_sync
                ON local_tasks(sync_state, started_at_ms);",
        )?;
        Ok(conn)
    }
}

fn select_sql() -> &'static str {
    "SELECT task_id, owner_user_id, agent_id, install_id, project_id, channel_id,
            conversation_id, workspace_path, prompt, cli, runtime_permission,
            execution_origin, billing_source, status, error, final_reply, model,
            codex_session_id, input_tokens, cached_input_tokens, output_tokens,
            reasoning_tokens, total_tokens, workspace_status_json, sync_state,
            completion_event_id, started_at_ms, finished_at_ms, server_ack_at_ms
       FROM local_tasks"
}

fn read_record(row: &rusqlite::Row<'_>) -> rusqlite::Result<LocalTaskRecord> {
    let workspace_status_json: Option<String> = row.get(23)?;
    Ok(LocalTaskRecord {
        task_id: row.get(0)?,
        owner_user_id: row.get(1)?,
        agent_id: row.get(2)?,
        install_id: row.get(3)?,
        project_id: row.get(4)?,
        channel_id: row.get(5)?,
        conversation_id: row.get(6)?,
        workspace_path: row.get(7)?,
        prompt: row.get(8)?,
        cli: row.get(9)?,
        runtime_permission: row.get(10)?,
        execution_origin: row.get(11)?,
        billing_source: row.get(12)?,
        status: row.get(13)?,
        error: row.get(14)?,
        final_reply: row.get(15)?,
        model: row.get(16)?,
        codex_session_id: row.get(17)?,
        input_tokens: row.get(18)?,
        cached_input_tokens: row.get(19)?,
        output_tokens: row.get(20)?,
        reasoning_tokens: row.get(21)?,
        total_tokens: row.get(22)?,
        workspace_status: workspace_status_json
            .as_deref()
            .and_then(|value| serde_json::from_str(value).ok()),
        sync_state: row.get(24)?,
        completion_event_id: row.get(25)?,
        started_at_ms: row.get(26)?,
        finished_at_ms: row.get(27)?,
        server_ack_at_ms: row.get(28)?,
    })
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(i64::MAX as u128) as i64)
        .unwrap_or(0)
}

fn clamp_u64(value: u64) -> i64 {
    value.min(i64::MAX as u64) as i64
}

fn trim_to_option(value: &str) -> Option<&str> {
    let value = value.trim();
    (!value.is_empty()).then_some(value)
}

fn clean_optional(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_store(name: &str) -> LocalTaskStore {
        let path = std::env::temp_dir().join(format!(
            "elon-local-task-{name}-{}-{}.sqlite3",
            std::process::id(),
            now_ms()
        ));
        LocalTaskStore::new(path)
    }

    fn create_task(store: &LocalTaskStore, task_id: &str) {
        store
            .create(LocalTaskStart {
                task_id,
                owner_user_id: "usr-a",
                agent_id: "node-a",
                install_id: "install-a",
                project_id: "prj-a",
                channel_id: Some("dev"),
                conversation_id: "conv-a",
                workspace_path: "D:/demo",
                prompt: "finish work",
                cli: "codex",
                runtime_permission: "full_access",
            })
            .unwrap();
    }

    fn completion(
        task_id: &str,
        event_id: &str,
        exit_ok: bool,
        error: Option<&str>,
        final_output: &str,
    ) -> CliCompletionEnvelope {
        CliCompletionEnvelope {
            event_id: event_id.to_string(),
            req_id: task_id.to_string(),
            cli: "codex".to_string(),
            origin: crate::node_agent_completion_outbox::LOCAL_OFFLINE_ORIGIN.to_string(),
            producer_identity: Some(homecli_proto::CliCompletionProducerIdentity {
                owner_user_id: "usr-a".to_string(),
                agent_id: "node-a".to_string(),
                install_id: "install-a".to_string(),
            }),
            project_context: Some(homecli_proto::CliProjectContext {
                project_id: "prj-a".to_string(),
                conversation_id: "conv-a".to_string(),
                runtime_permission: Some("full_access".to_string()),
            }),
            channel_id: Some("dev".to_string()),
            prompt: Some("finish work".to_string()),
            final_output: final_output.to_string(),
            exit_ok,
            error: error.map(str::to_string),
            session_id: Some("session-a".to_string()),
            prompt_tokens: Some(4),
            cached_input_tokens: Some(1),
            completion_tokens: Some(2),
            reasoning_tokens: Some(0),
            total_tokens: Some(6),
            model: Some("gpt-5".to_string()),
            workspace_status: None,
            created_at_ms: 123,
        }
    }

    #[test]
    fn recovering_task_atomically_returns_to_running_and_clears_update_error() {
        let store = test_store("recovery-running");
        create_task(&store, "local-running");
        assert!(store
            .mark_recovering("local-running", "节点更新完成，正在重接原 CLI 会话")
            .unwrap());
        assert!(store.mark_recovery_running("local-running").unwrap());
        let record = store.get("local-running").unwrap().unwrap();
        assert_eq!(record.status, "running");
        assert!(record.error.is_none());
        assert!(!store.mark_recovery_running("local-running").unwrap());
        assert_eq!(
            store
                .interrupt_lingering_running(&HashSet::new(), i64::MAX)
                .unwrap(),
            1
        );
        assert!(store.mark_recovery_running("local-running").unwrap());
        let recovered = store.get("local-running").unwrap().unwrap();
        assert_eq!(recovered.status, "running");
        assert!(recovered.finished_at_ms.is_none());
    }

    #[test]
    fn recovered_cancel_and_timeout_bind_terminal_state_idempotently() {
        let store = test_store("recovery-terminal");
        create_task(&store, "local-canceled");
        store
            .mark_recovering("local-canceled", "reattaching")
            .unwrap();
        let canceled = completion(
            "local-canceled",
            "event-canceled",
            false,
            Some("任务在节点更新恢复期间被取消"),
            "partial reply",
        );
        assert!(store.reconcile_completion(&canceled).unwrap());
        assert!(store.reconcile_completion(&canceled).unwrap());
        assert_eq!(
            store.get("local-canceled").unwrap().unwrap().status,
            "canceled"
        );

        create_task(&store, "local-timeout");
        store
            .mark_recovering("local-timeout", "reattaching")
            .unwrap();
        let timeout = completion(
            "local-timeout",
            "event-timeout",
            false,
            Some("codex pipe sidecar 执行超时（超过 3600 秒）"),
            "timeout evidence",
        );
        assert!(store.reconcile_completion(&timeout).unwrap());
        let timed_out = store.get("local-timeout").unwrap().unwrap();
        assert_eq!(timed_out.status, "failed");
        assert!(timed_out.error.unwrap().contains("超时"));
    }

    #[test]
    fn late_terminal_race_never_downgrades_done_or_loses_final_reply() {
        let store = test_store("terminal-race");
        create_task(&store, "local-race");
        store.mark_recovering("local-race", "reattaching").unwrap();
        let success = completion("local-race", "event-race", true, None, "final answer");
        assert!(store.reconcile_completion(&success).unwrap());

        let mut late_failure = success;
        late_failure.exit_ok = false;
        late_failure.error = Some("late timeout".to_string());
        late_failure.final_output.clear();
        assert!(store.reconcile_completion(&late_failure).unwrap());
        assert!(!store
            .mark_recovering("local-race", "late recovery")
            .unwrap());
        assert!(!store.mark_recovery_running("local-race").unwrap());

        let record = store.get("local-race").unwrap().unwrap();
        assert_eq!(record.status, "done");
        assert!(record.error.is_none());
        assert_eq!(record.final_reply.as_deref(), Some("final answer"));
    }

    #[test]
    fn local_tasks_are_partitioned_and_cancel_intent_stays_nonterminal() {
        let store = test_store("partition");
        store
            .create(LocalTaskStart {
                task_id: "local-1",
                owner_user_id: "usr-a",
                agent_id: "node-a",
                install_id: "install-a",
                project_id: "prj-a",
                channel_id: Some("dev"),
                conversation_id: "conv-a",
                workspace_path: "D:/demo",
                prompt: "finish work",
                cli: "codex",
                runtime_permission: "full_access",
            })
            .unwrap();
        assert!(store.get_for_owner("usr-b", "local-1").unwrap().is_none());
        assert!(store.mark_cancel_requested("local-1").unwrap());
        assert!(!store.mark_cancel_requested("local-1").unwrap());
        let pending = store.get_for_owner("usr-a", "local-1").unwrap().unwrap();
        assert_eq!(pending.status, "cancel_requested");
        assert!(pending.finished_at_ms.is_none());
    }

    #[test]
    fn durable_completion_reconciles_terminal_binding_idempotently() {
        let store = test_store("reconcile");
        store
            .create(LocalTaskStart {
                task_id: "local-reconcile",
                owner_user_id: "usr-a",
                agent_id: "node-a",
                install_id: "install-a",
                project_id: "prj-a",
                channel_id: Some("dev"),
                conversation_id: "conv-a",
                workspace_path: "D:/demo",
                prompt: "finish work",
                cli: "codex",
                runtime_permission: "full_access",
            })
            .unwrap();
        let completion = CliCompletionEnvelope {
            event_id: "event-reconcile".to_string(),
            req_id: "local-reconcile".to_string(),
            cli: "codex".to_string(),
            origin: crate::node_agent_completion_outbox::LOCAL_OFFLINE_ORIGIN.to_string(),
            producer_identity: Some(homecli_proto::CliCompletionProducerIdentity {
                owner_user_id: "usr-a".to_string(),
                agent_id: "node-a".to_string(),
                install_id: "install-a".to_string(),
            }),
            project_context: Some(homecli_proto::CliProjectContext {
                project_id: "prj-a".to_string(),
                conversation_id: "conv-a".to_string(),
                runtime_permission: Some("full_access".to_string()),
            }),
            channel_id: Some("dev".to_string()),
            prompt: Some("finish work".to_string()),
            final_output: "done".to_string(),
            exit_ok: true,
            error: None,
            session_id: Some("session-a".to_string()),
            prompt_tokens: Some(4),
            cached_input_tokens: Some(1),
            completion_tokens: Some(2),
            reasoning_tokens: Some(0),
            total_tokens: Some(6),
            model: Some("gpt-5".to_string()),
            workspace_status: None,
            created_at_ms: 123,
        };

        assert!(store.reconcile_completion(&completion).unwrap());
        assert!(store.reconcile_completion(&completion).unwrap());
        let record = store
            .get_for_owner("usr-a", "local-reconcile")
            .unwrap()
            .unwrap();
        assert_eq!(record.status, "done");
        assert_eq!(
            record.completion_event_id.as_deref(),
            Some("event-reconcile")
        );
        assert_eq!(record.total_tokens, Some(6));

        let mut wrong_event = completion.clone();
        wrong_event.event_id = "event-conflict".to_string();
        let error = store
            .reconcile_completion(&wrong_event)
            .expect_err("a terminal row cannot bind a second completion event");
        assert!(error.to_string().contains("different completion event"));

        let mut wrong_scope = completion;
        wrong_scope.project_context.as_mut().unwrap().project_id = "prj-other".to_string();
        assert!(!store.reconcile_completion(&wrong_scope).unwrap());
    }
}
