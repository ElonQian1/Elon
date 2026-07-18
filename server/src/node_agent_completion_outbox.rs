//! Durable node-side outbox for terminal PC CLI results.
//!
//! A completion must be inserted here before its WebSocket replay is queued.
//! SQLite WAL plus `synchronous=FULL` makes the terminal record survive both a
//! cloud disconnect and a node-agent restart. The producer removes (or retains
//! as acknowledged for later compaction) a row only after the server confirms
//! durable receipt with `ServerToAgent::CliCompletionAck`.

use std::{collections::HashSet, fs, path::PathBuf, time::Duration};

use anyhow::{bail, Context, Result};
use homecli_proto::{CliCompletionEnvelope, CliCompletionProducerIdentity};
use rusqlite::{params, Connection, OptionalExtension};

#[path = "node_agent_completion_outbox_support.rs"]
mod support;
use support::*;

const OUTBOX_FILE_NAME: &str = "cli-completion-outbox.sqlite3";
const STATUS_PENDING: &str = "pending";
const STATUS_ACKED: &str = "acked";
const STATUS_DEAD_LETTER: &str = "dead_letter";
pub(crate) const LOCAL_OFFLINE_ORIGIN: &str = "local_offline";
const BUSY_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_LAST_ERROR_CHARS: usize = 2_000;

#[derive(Clone, Debug)]
pub(crate) struct CliCompletionOutbox {
    path: PathBuf,
}

#[derive(Clone, Debug)]
pub(crate) struct PendingCliCompletion {
    pub(crate) completion: CliCompletionEnvelope,
    pub(crate) attempt_count: u32,
    pub(crate) last_attempt_at_ms: Option<u64>,
    pub(crate) last_error: Option<String>,
}

impl CliCompletionOutbox {
    pub(crate) fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub(crate) fn default() -> Self {
        Self::new(crate::state_path().with_file_name(OUTBOX_FILE_NAME))
    }

    /// Persist a terminal event before any network send.
    ///
    /// Returns `true` for a newly inserted completion and `false` when an
    /// identical `event_id`/`req_id` payload was already present. Reusing either
    /// id with different content is rejected instead of silently changing the
    /// accounting/result payload.
    pub(crate) fn enqueue(&self, completion: &CliCompletionEnvelope) -> Result<bool> {
        validate_completion(completion)?;
        let payload_json = serde_json::to_string(completion).context("序列化 CLI completion")?;
        let conn = self.connect()?;
        let tx = conn.unchecked_transaction()?;
        let inserted = tx.execute(
            "INSERT OR IGNORE INTO cli_completion_outbox (
                event_id, req_id, payload_json, producer_owner_user_id,
                producer_agent_id, producer_install_id, status, created_at_ms,
                attempt_count, last_attempt_at_ms, last_error, acked_at_ms
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 0, NULL, NULL, NULL)",
            params![
                completion.event_id,
                completion.req_id,
                payload_json,
                completion
                    .producer_identity
                    .as_ref()
                    .map(|value| &value.owner_user_id),
                completion
                    .producer_identity
                    .as_ref()
                    .map(|value| &value.agent_id),
                completion
                    .producer_identity
                    .as_ref()
                    .map(|value| &value.install_id),
                STATUS_PENDING,
                sqlite_ms(completion.created_at_ms),
            ],
        )?;
        if inserted == 0 {
            let existing = tx
                .query_row(
                    "SELECT event_id, req_id, payload_json
                       FROM cli_completion_outbox
                      WHERE event_id = ?1 OR req_id = ?2
                      ORDER BY CASE WHEN event_id = ?1 THEN 0 ELSE 1 END
                      LIMIT 1",
                    params![completion.event_id, completion.req_id],
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                        ))
                    },
                )
                .optional()?;
            let Some((existing_event_id, existing_req_id, existing_payload)) = existing else {
                bail!("CLI completion insert was ignored without an existing idempotency row");
            };
            if existing_event_id != completion.event_id
                || existing_req_id != completion.req_id
                || existing_payload != payload_json
            {
                bail!(
                    "CLI completion idempotency conflict: event_id={} req_id={}",
                    completion.event_id,
                    completion.req_id
                );
            }
            tx.commit()?;
            return Ok(false);
        }
        tx.commit()?;
        Ok(true)
    }

    pub(crate) fn list_pending(&self, limit: usize) -> Result<Vec<CliCompletionEnvelope>> {
        Ok(self
            .list_pending_with_metadata(limit)?
            .into_iter()
            .map(|row| row.completion)
            .collect())
    }

    /// Read the full durable envelope associated with a legacy `CliDone`.
    pub(crate) fn latest_for_req_id(&self, req_id: &str) -> Result<Option<CliCompletionEnvelope>> {
        let req_id = required_id(req_id, "req_id")?;
        let conn = self.connect()?;
        let payload = conn
            .query_row(
                "SELECT payload_json FROM cli_completion_outbox WHERE req_id = ?1",
                params![req_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        payload
            .map(|json| serde_json::from_str(&json).context("解析 CLI completion outbox payload"))
            .transpose()
    }

    pub(crate) fn latest_for_req_id_for_producer(
        &self,
        req_id: &str,
        producer: &CliCompletionProducerIdentity,
    ) -> Result<Option<CliCompletionEnvelope>> {
        let req_id = required_id(req_id, "req_id")?;
        validate_producer_identity(producer)?;
        let conn = self.connect()?;
        let payload = conn
            .query_row(
                "SELECT payload_json FROM cli_completion_outbox
                  WHERE req_id = ?1
                    AND producer_owner_user_id = ?2
                    AND producer_agent_id = ?3
                    AND producer_install_id = ?4",
                params![
                    req_id,
                    producer.owner_user_id.as_str(),
                    producer.agent_id.as_str(),
                    producer.install_id.as_str()
                ],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        payload
            .map(|json| serde_json::from_str(&json).context("解析 CLI completion outbox payload"))
            .transpose()
    }

    /// Resolve an ACK/rejection only when both durable binding keys match.
    pub(crate) fn completion_for_binding(
        &self,
        event_id: &str,
        req_id: &str,
    ) -> Result<Option<CliCompletionEnvelope>> {
        let event_id = required_id(event_id, "event_id")?;
        let req_id = required_id(req_id, "req_id")?;
        let conn = self.connect()?;
        let payload = conn
            .query_row(
                "SELECT payload_json FROM cli_completion_outbox
                  WHERE event_id = ?1 AND req_id = ?2",
                params![event_id, req_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        payload
            .map(|json| serde_json::from_str(&json).context("解析绑定的 CLI completion payload"))
            .transpose()
    }

    pub(crate) fn list_pending_with_metadata(
        &self,
        limit: usize,
    ) -> Result<Vec<PendingCliCompletion>> {
        self.list_pending_matching(None, limit)
    }

    /// Only replay rows created by the exact login/node/installation currently
    /// authenticated on this WebSocket. Rows from a previous binding remain
    /// durable but cannot be reinterpreted by the new account.
    pub(crate) fn list_pending_for_producer(
        &self,
        producer: &CliCompletionProducerIdentity,
        limit: usize,
    ) -> Result<Vec<PendingCliCompletion>> {
        validate_producer_identity(producer)?;
        self.list_pending_matching(Some(producer), limit)
    }

    fn list_pending_matching(
        &self,
        producer: Option<&CliCompletionProducerIdentity>,
        limit: usize,
    ) -> Result<Vec<PendingCliCompletion>> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare(
            "SELECT event_id, payload_json, attempt_count, last_attempt_at_ms, last_error
               FROM cli_completion_outbox
              WHERE status = ?1
                AND (?2 IS NULL OR (
                    producer_owner_user_id = ?2
                    AND producer_agent_id = ?3
                    AND producer_install_id = ?4
                ))
              ORDER BY (last_attempt_at_ms IS NOT NULL) ASC,
                       COALESCE(last_attempt_at_ms, 0) ASC,
                       created_at_ms ASC,
                       rowid ASC
              LIMIT ?5",
        )?;
        let owner_user_id = producer.map(|value| value.owner_user_id.as_str());
        let agent_id = producer.map(|value| value.agent_id.as_str());
        let install_id = producer.map(|value| value.install_id.as_str());
        let rows = stmt.query_map(
            params![
                STATUS_PENDING,
                owner_user_id,
                agent_id,
                install_id,
                limit.clamp(1, 1_000) as i64
            ],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, Option<i64>>(3)?,
                    row.get::<_, Option<String>>(4)?,
                ))
            },
        )?;
        let raw_rows = rows.collect::<rusqlite::Result<Vec<_>>>()?;
        drop(stmt);

        let mut pending = Vec::new();
        for (event_id, payload_json, attempt_count, last_attempt_at_ms, last_error) in raw_rows {
            match serde_json::from_str::<CliCompletionEnvelope>(&payload_json) {
                Ok(completion) => pending.push(PendingCliCompletion {
                    completion,
                    attempt_count: attempt_count.clamp(0, u32::MAX as i64) as u32,
                    last_attempt_at_ms: last_attempt_at_ms.map(nonnegative_u64),
                    last_error,
                }),
                Err(error) => {
                    let message = format!("损坏的 CLI completion payload：{error}");
                    conn.execute(
                        "UPDATE cli_completion_outbox
                            SET status = ?2,
                                attempt_count = attempt_count + 1,
                                last_attempt_at_ms = ?3,
                                last_error = ?4
                          WHERE event_id = ?1 AND status = ?5",
                        params![
                            event_id,
                            STATUS_DEAD_LETTER,
                            sqlite_ms(now_ms()),
                            truncate_optional(Some(&message), MAX_LAST_ERROR_CHARS),
                            STATUS_PENDING,
                        ],
                    )?;
                    tracing::error!(%event_id, %error, "损坏的 CLI completion 已移入 dead letter");
                }
            }
        }
        Ok(pending)
    }

    /// Returns raw request bindings without deserializing payloads. Startup
    /// recovery uses this to avoid interrupting a task whose terminal envelope
    /// is durable but was outside the bounded reconciliation batch.
    pub(crate) fn pending_req_ids(&self) -> Result<HashSet<String>> {
        let conn = self.connect()?;
        let mut stmt =
            conn.prepare("SELECT req_id FROM cli_completion_outbox WHERE status = ?1")?;
        let rows = stmt.query_map(params![STATUS_PENDING], |row| row.get::<_, String>(0))?;
        rows.collect::<rusqlite::Result<HashSet<_>>>()
            .map_err(Into::into)
    }

    /// Record a send/replay attempt without changing the durable pending state.
    pub(crate) fn record_attempt(&self, event_id: &str, error: Option<&str>) -> Result<bool> {
        let event_id = required_id(event_id, "event_id")?;
        let conn = self.connect()?;
        let changed = conn.execute(
            "UPDATE cli_completion_outbox
                SET attempt_count = attempt_count + 1,
                    last_attempt_at_ms = ?2,
                    last_error = ?3
              WHERE event_id = ?1 AND status = ?4",
            params![
                event_id,
                sqlite_ms(now_ms()),
                truncate_optional(error, MAX_LAST_ERROR_CHARS),
                STATUS_PENDING,
            ],
        )?;
        Ok(changed > 0)
    }

    /// Mark a matching row acknowledged. Repeating the same ACK remains safe.
    pub(crate) fn acknowledge(&self, event_id: &str, req_id: &str) -> Result<bool> {
        let event_id = required_id(event_id, "event_id")?;
        let req_id = required_id(req_id, "req_id")?;
        let conn = self.connect()?;
        let changed = conn.execute(
            "UPDATE cli_completion_outbox
                SET status = ?3,
                    acked_at_ms = ?4,
                    last_error = NULL
              WHERE event_id = ?1 AND req_id = ?2",
            params![event_id, req_id, STATUS_ACKED, sqlite_ms(now_ms())],
        )?;
        Ok(changed > 0)
    }

    /// Keep a transient rejection pending, or move a permanent rejection to a
    /// local dead letter so it cannot spin forever on every reconnect.
    pub(crate) fn reject(
        &self,
        event_id: &str,
        req_id: &str,
        retryable: bool,
        error: &str,
    ) -> Result<bool> {
        let event_id = required_id(event_id, "event_id")?;
        let req_id = required_id(req_id, "req_id")?;
        let conn = self.connect()?;
        let status = if retryable {
            STATUS_PENDING
        } else {
            STATUS_DEAD_LETTER
        };
        let changed = conn.execute(
            "UPDATE cli_completion_outbox
                SET status = ?3,
                    last_error = ?4,
                    last_attempt_at_ms = ?5
              WHERE event_id = ?1 AND req_id = ?2 AND status = ?6",
            params![
                event_id,
                req_id,
                status,
                truncate_optional(Some(error), MAX_LAST_ERROR_CHARS),
                sqlite_ms(now_ms()),
                STATUS_PENDING,
            ],
        )?;
        Ok(changed > 0)
    }

    /// Physically remove one row after it has been marked acknowledged.
    pub(crate) fn delete_acked(&self, event_id: &str) -> Result<bool> {
        let event_id = required_id(event_id, "event_id")?;
        let conn = self.connect()?;
        let changed = conn.execute(
            "DELETE FROM cli_completion_outbox WHERE event_id = ?1 AND status = ?2",
            params![event_id, STATUS_ACKED],
        )?;
        Ok(changed > 0)
    }

    pub(crate) fn pending_count(&self) -> Result<usize> {
        self.count_status(STATUS_PENDING)
    }

    #[cfg(test)]
    fn acked_count(&self) -> Result<usize> {
        self.count_status(STATUS_ACKED)
    }

    #[cfg(test)]
    fn dead_letter_count(&self) -> Result<usize> {
        self.count_status(STATUS_DEAD_LETTER)
    }

    fn count_status(&self, status: &str) -> Result<usize> {
        let conn = self.connect()?;
        let count = conn.query_row(
            "SELECT COUNT(*) FROM cli_completion_outbox WHERE status = ?1",
            params![status],
            |row| row.get::<_, i64>(0),
        )?;
        Ok(count.max(0) as usize)
    }

    fn connect(&self) -> Result<Connection> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("创建 CLI completion outbox 目录 {:?}", parent))?;
        }
        let conn = Connection::open(&self.path)
            .with_context(|| format!("打开 CLI completion outbox {:?}", self.path))?;
        conn.busy_timeout(BUSY_TIMEOUT)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "FULL")?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS cli_completion_outbox (
                event_id          TEXT PRIMARY KEY,
                req_id            TEXT NOT NULL UNIQUE,
                payload_json      TEXT NOT NULL,
                producer_owner_user_id TEXT,
                producer_agent_id TEXT,
                producer_install_id TEXT,
                status            TEXT NOT NULL DEFAULT 'pending'
                                  CHECK (status IN ('pending', 'acked', 'dead_letter')),
                created_at_ms     INTEGER NOT NULL,
                attempt_count     INTEGER NOT NULL DEFAULT 0,
                last_attempt_at_ms INTEGER,
                last_error        TEXT,
                acked_at_ms       INTEGER
             );",
        )?;
        ensure_identity_column(&conn, "producer_owner_user_id")?;
        ensure_identity_column(&conn, "producer_agent_id")?;
        ensure_identity_column(&conn, "producer_install_id")?;
        conn.execute_batch(
            "CREATE INDEX IF NOT EXISTS idx_cli_completion_outbox_pending
                 ON cli_completion_outbox(status, created_at_ms, event_id);
             CREATE INDEX IF NOT EXISTS idx_cli_completion_outbox_producer
                 ON cli_completion_outbox(
                    status, producer_owner_user_id, producer_agent_id,
                    producer_install_id, created_at_ms
                 );",
        )?;
        Ok(conn)
    }
}

#[cfg(test)]
#[path = "node_agent_completion_outbox_migration_tests.rs"]
mod migration_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use homecli_proto::{CliProjectContext, CliWorkspaceStatus};
    use std::path::Path;

    fn temp_outbox() -> (CliCompletionOutbox, PathBuf) {
        let path = std::env::temp_dir().join(format!(
            "elon-cli-completion-outbox-test-{}.sqlite3",
            uuid::Uuid::new_v4().simple()
        ));
        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(path.with_extension("sqlite3-wal"));
        let _ = fs::remove_file(path.with_extension("sqlite3-shm"));
        (CliCompletionOutbox::new(&path), path)
    }

    fn completion(event_id: &str, req_id: &str) -> CliCompletionEnvelope {
        CliCompletionEnvelope {
            event_id: event_id.to_string(),
            req_id: req_id.to_string(),
            cli: "codex".to_string(),
            origin: "cloud_dispatch".to_string(),
            producer_identity: Some(CliCompletionProducerIdentity {
                owner_user_id: "owner-a".to_string(),
                agent_id: "node-a".to_string(),
                install_id: "install-a".to_string(),
            }),
            project_context: Some(CliProjectContext {
                project_id: "project-a".to_string(),
                conversation_id: "conversation-a".to_string(),
                runtime_permission: Some("project_write".to_string()),
            }),
            channel_id: None,
            prompt: None,
            final_output: "codex\n任务完成。\n".to_string(),
            exit_ok: true,
            error: None,
            session_id: Some("codex-session-a".to_string()),
            prompt_tokens: Some(100),
            cached_input_tokens: Some(20),
            completion_tokens: Some(30),
            reasoning_tokens: Some(5),
            total_tokens: Some(130),
            model: Some("gpt-5.4".to_string()),
            workspace_status: Some(CliWorkspaceStatus {
                base_workspace_path: Some("D:\\repo".to_string()),
                active_workspace_path: "D:\\repo-worktree".to_string(),
                isolated: true,
                branch: Some("ai/session/project-a/conversation-a".to_string()),
                git_head: Some("0123456789012345678901234567890123456789".to_string()),
                prepare_status: "prepared".to_string(),
                merge_status: Some("merged".to_string()),
                merge_message: None,
            }),
            created_at_ms: 1_783_920_000_000,
        }
    }

    fn cleanup(path: &Path) {
        let _ = fs::remove_file(path);
        let _ = fs::remove_file(path.with_extension("sqlite3-wal"));
        let _ = fs::remove_file(path.with_extension("sqlite3-shm"));
    }

    #[test]
    fn pending_completion_survives_reopen_and_duplicate_enqueue_is_idempotent() {
        let (outbox, path) = temp_outbox();
        let value = completion("event-1", "req-1");
        assert!(outbox.enqueue(&value).expect("insert completion"));
        assert!(!outbox.enqueue(&value).expect("duplicate completion"));
        drop(outbox);

        let reopened = CliCompletionOutbox::new(&path);
        let pending = reopened.list_pending(10).expect("list after reopen");
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].event_id, "event-1");
        assert_eq!(pending[0].req_id, "req-1");
        assert_eq!(pending[0].completion_tokens, Some(30));
        assert_eq!(pending[0].final_output, "codex\n任务完成。\n");
        cleanup(&path);
    }

    #[test]
    fn reused_event_or_request_id_cannot_change_the_completion_payload() {
        let (outbox, path) = temp_outbox();
        let original = completion("event-1", "req-1");
        outbox.enqueue(&original).unwrap();

        let mut changed_payload = original.clone();
        changed_payload.final_output = "different".to_string();
        assert!(outbox.enqueue(&changed_payload).is_err());

        assert!(outbox.enqueue(&completion("event-2", "req-1")).is_err());
        assert!(outbox.enqueue(&completion("event-1", "req-2")).is_err());
        assert_eq!(outbox.pending_count().unwrap(), 1);
        cleanup(&path);
    }

    #[test]
    fn producer_scoped_replay_cannot_cross_account_or_installation() {
        let (outbox, path) = temp_outbox();
        let original = completion("event-a", "req-a");
        outbox.enqueue(&original).unwrap();

        let identity = original.producer_identity.as_ref().unwrap();
        assert_eq!(
            outbox
                .list_pending_for_producer(identity, 10)
                .unwrap()
                .len(),
            1
        );
        for different in [
            CliCompletionProducerIdentity {
                owner_user_id: "owner-b".to_string(),
                ..identity.clone()
            },
            CliCompletionProducerIdentity {
                agent_id: "node-b".to_string(),
                ..identity.clone()
            },
            CliCompletionProducerIdentity {
                install_id: "install-b".to_string(),
                ..identity.clone()
            },
        ] {
            assert!(outbox
                .list_pending_for_producer(&different, 10)
                .unwrap()
                .is_empty());
        }
        assert_eq!(outbox.pending_count().unwrap(), 1);
        cleanup(&path);
    }

    #[test]
    fn mismatched_ack_binding_cannot_transition_pending_completion() {
        let (outbox, path) = temp_outbox();
        outbox.enqueue(&completion("event-1", "req-1")).unwrap();

        assert!(outbox
            .completion_for_binding("event-1", "req-1")
            .unwrap()
            .is_some());
        assert!(outbox
            .completion_for_binding("event-other", "req-1")
            .unwrap()
            .is_none());
        assert!(outbox
            .completion_for_binding("event-1", "req-other")
            .unwrap()
            .is_none());
        assert!(!outbox.acknowledge("event-1", "req-other").unwrap());
        assert!(!outbox
            .reject("event-other", "req-1", false, "wrong binding")
            .unwrap());
        assert_eq!(outbox.pending_count().unwrap(), 1);
        cleanup(&path);
    }

    #[test]
    fn ack_hides_pending_row_and_allows_explicit_compaction() {
        let (outbox, path) = temp_outbox();
        outbox.enqueue(&completion("event-1", "req-1")).unwrap();
        assert_eq!(
            outbox
                .latest_for_req_id("req-1")
                .unwrap()
                .expect("completion before ACK")
                .event_id,
            "event-1"
        );
        assert!(outbox.acknowledge("event-1", "req-1").unwrap());
        assert!(outbox.acknowledge("event-1", "req-1").unwrap());
        assert_eq!(outbox.pending_count().unwrap(), 0);
        assert_eq!(outbox.acked_count().unwrap(), 1);
        assert!(outbox.latest_for_req_id("req-1").unwrap().is_some());
        assert!(outbox.delete_acked("event-1").unwrap());
        assert!(!outbox.delete_acked("event-1").unwrap());
        assert_eq!(outbox.acked_count().unwrap(), 0);
        assert!(outbox.latest_for_req_id("req-1").unwrap().is_none());
        cleanup(&path);
    }

    #[test]
    fn retryable_rejection_stays_pending_and_permanent_rejection_is_dead_lettered() {
        let (outbox, path) = temp_outbox();
        outbox
            .enqueue(&completion("event-retry", "req-retry"))
            .unwrap();
        outbox
            .enqueue(&completion("event-dead", "req-dead"))
            .unwrap();

        assert!(outbox
            .reject(
                "event-retry",
                "req-retry",
                true,
                "server temporarily unavailable"
            )
            .unwrap());
        assert!(outbox
            .reject(
                "event-dead",
                "req-dead",
                false,
                "authenticated node does not own request"
            )
            .unwrap());
        assert_eq!(outbox.pending_count().unwrap(), 1);
        assert_eq!(outbox.dead_letter_count().unwrap(), 1);
        let pending = outbox.list_pending_with_metadata(10).unwrap();
        assert_eq!(pending[0].completion.event_id, "event-retry");
        assert_eq!(
            pending[0].last_error.as_deref(),
            Some("server temporarily unavailable")
        );
        cleanup(&path);
    }

    #[test]
    fn send_attempt_metadata_is_persisted_without_removing_completion() {
        let (outbox, path) = temp_outbox();
        outbox.enqueue(&completion("event-1", "req-1")).unwrap();
        assert!(outbox
            .record_attempt("event-1", Some("connection reset"))
            .unwrap());
        drop(outbox);

        let reopened = CliCompletionOutbox::new(&path);
        let pending = reopened.list_pending_with_metadata(10).unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].attempt_count, 1);
        assert!(pending[0].last_attempt_at_ms.is_some());
        assert_eq!(pending[0].last_error.as_deref(), Some("connection reset"));
        cleanup(&path);
    }

    #[test]
    fn unattempted_completion_is_listed_before_recently_attempted_rows() {
        let (outbox, path) = temp_outbox();
        outbox.enqueue(&completion("event-old", "req-old")).unwrap();
        outbox.enqueue(&completion("event-new", "req-new")).unwrap();
        outbox.record_attempt("event-old", None).unwrap();

        let pending = outbox.list_pending_with_metadata(10).unwrap();
        assert_eq!(pending.len(), 2);
        assert_eq!(pending[0].completion.event_id, "event-new");
        assert_eq!(pending[1].completion.event_id, "event-old");
        cleanup(&path);
    }

    #[test]
    fn corrupt_pending_payload_is_dead_lettered_without_blocking_later_rows() {
        let (outbox, path) = temp_outbox();
        outbox.enqueue(&completion("event-bad", "req-bad")).unwrap();
        outbox
            .enqueue(&completion("event-good", "req-good"))
            .unwrap();
        outbox
            .connect()
            .unwrap()
            .execute(
                "UPDATE cli_completion_outbox SET payload_json = '{' WHERE event_id = ?1",
                params!["event-bad"],
            )
            .unwrap();

        let pending = outbox.list_pending_with_metadata(10).unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].completion.event_id, "event-good");
        assert_eq!(outbox.pending_count().unwrap(), 1);
        assert_eq!(outbox.dead_letter_count().unwrap(), 1);
        cleanup(&path);
    }

    #[test]
    fn prompt_is_only_accepted_for_local_offline_origin() {
        let (outbox, path) = temp_outbox();
        let mut cloud = completion("event-cloud", "req-cloud");
        cloud.prompt = Some("must not persist".to_string());
        assert!(outbox.enqueue(&cloud).is_err());

        let mut local = completion("event-local", "req-local");
        local.origin = LOCAL_OFFLINE_ORIGIN.to_string();
        local.channel_id = Some("channel-ai".to_string());
        local.prompt = Some("离线发起的新任务".to_string());
        assert!(outbox.enqueue(&local).unwrap());
        let pending = outbox.list_pending(10).unwrap();
        assert_eq!(pending[0].channel_id.as_deref(), Some("channel-ai"));
        assert_eq!(pending[0].prompt.as_deref(), Some("离线发起的新任务"));
        cleanup(&path);
    }

    #[test]
    fn sqlite_uses_wal_and_full_synchronous_mode() {
        let (outbox, path) = temp_outbox();
        outbox.enqueue(&completion("event-1", "req-1")).unwrap();
        let conn = outbox.connect().unwrap();
        let journal_mode: String = conn
            .pragma_query_value(None, "journal_mode", |row| row.get(0))
            .unwrap();
        let synchronous: i64 = conn
            .pragma_query_value(None, "synchronous", |row| row.get(0))
            .unwrap();
        assert_eq!(journal_mode.to_ascii_lowercase(), "wal");
        assert_eq!(synchronous, 2, "SQLite FULL synchronous mode is 2");
        drop(conn);
        cleanup(&path);
    }
}
