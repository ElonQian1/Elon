// server/src/node_agent_cli_sidecar.rs

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    collections::BTreeMap,
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::node_agent_task_journal_lock::with_task_journal_io_lock;

const SIDECAR_STALE_AFTER_MS: u128 = 2 * 60 * 1_000;

#[derive(Clone, Debug)]
pub(crate) struct CliSidecarRegistry {
    dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct CliSidecarCapabilities {
    #[serde(default)]
    pub terminal_attach: bool,
    #[serde(default)]
    pub output_stream_replay: bool,
    #[serde(default)]
    pub tool_approval_recovery: bool,
    #[serde(default)]
    pub cancel: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct CliSidecarSessionRecord {
    pub session_id: String,
    pub task_id: String,
    pub cli_name: String,
    pub route: String,
    #[serde(default)]
    pub cwd: Option<String>,
    pub state: String,
    pub transport: String,
    #[serde(default)]
    pub endpoint: Option<String>,
    #[serde(default)]
    pub sidecar_pid: Option<u32>,
    #[serde(default)]
    pub child_pid: Option<u32>,
    pub started_at_ms: u128,
    pub last_seen_at_ms: u128,
    pub capabilities: CliSidecarCapabilities,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct CliSidecarCommandRecord {
    pub task_id: String,
    pub command: String,
    #[serde(default)]
    pub approval_id: Option<String>,
    #[serde(default)]
    pub decision: Option<String>,
    pub at_ms: u128,
}

impl CliSidecarRegistry {
    pub(crate) fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    pub(crate) fn default() -> Self {
        Self::new(super::state_path().with_file_name("cli-sidecars"))
    }

    pub(crate) fn upsert_session(&self, session: CliSidecarSessionRecord) -> Result<()> {
        with_task_journal_io_lock(|| {
            let mut sessions = self.load_sessions()?;
            sessions.insert(session.session_id.clone(), session);
            self.save_sessions(&sessions)
        })
    }

    pub(crate) fn touch_session(
        &self,
        session_id: &str,
        state: Option<&str>,
        child_pid: Option<u32>,
    ) -> Result<bool> {
        let session_id = session_id.trim();
        if session_id.is_empty() {
            return Ok(false);
        }
        with_task_journal_io_lock(|| {
            let mut sessions = self.load_sessions()?;
            let Some(session) = sessions.get_mut(session_id) else {
                return Ok(false);
            };
            if let Some(state) = state.map(str::trim).filter(|value| !value.is_empty()) {
                session.state = state.to_string();
            }
            if child_pid.is_some() {
                session.child_pid = child_pid;
            }
            session.last_seen_at_ms = now_ms();
            self.save_sessions(&sessions)?;
            Ok(true)
        })
    }

    pub(crate) fn mark_task_terminal(&self, task_id: &str, state: &str) -> Result<bool> {
        let task_id = task_id.trim();
        if task_id.is_empty() {
            return Ok(false);
        }
        with_task_journal_io_lock(|| {
            let mut sessions = self.load_sessions()?;
            let Some((_, session)) = sessions
                .iter_mut()
                .filter(|(_, session)| session.task_id == task_id)
                .max_by(|(_, left), (_, right)| {
                    left.last_seen_at_ms
                        .cmp(&right.last_seen_at_ms)
                        .then_with(|| left.started_at_ms.cmp(&right.started_at_ms))
                })
            else {
                return Ok(false);
            };
            session.state = state.to_string();
            session.last_seen_at_ms = now_ms();
            self.save_sessions(&sessions)?;
            Ok(true)
        })
    }

    pub(crate) fn latest_sessions(&self, limit: usize) -> Result<Vec<CliSidecarSessionRecord>> {
        with_task_journal_io_lock(|| {
            let mut sessions: Vec<_> = self.load_sessions()?.into_values().collect();
            sessions.sort_by(|left, right| {
                right
                    .last_seen_at_ms
                    .cmp(&left.last_seen_at_ms)
                    .then_with(|| right.started_at_ms.cmp(&left.started_at_ms))
            });
            sessions.truncate(limit.min(100));
            Ok(sessions)
        })
    }

    pub(crate) fn session_for_task(
        &self,
        task_id: &str,
    ) -> Result<Option<CliSidecarSessionRecord>> {
        let task_id = task_id.trim();
        if task_id.is_empty() {
            return Ok(None);
        }
        with_task_journal_io_lock(|| {
            Ok(self
                .load_sessions()?
                .into_values()
                .filter(|session| session.task_id == task_id)
                .max_by(|left, right| {
                    left.last_seen_at_ms
                        .cmp(&right.last_seen_at_ms)
                        .then_with(|| left.started_at_ms.cmp(&right.started_at_ms))
                }))
        })
    }

    pub(crate) fn record_cancel_command(&self, task_id: &str) -> Result<bool> {
        let Some(session) = self.session_for_task(task_id)? else {
            return Ok(false);
        };
        if !session.is_attachable_at(now_ms()) || !session.capabilities.cancel {
            return Ok(false);
        }
        self.append_sidecar_command(task_id, "cancel", None, None)
    }

    pub(crate) fn record_tool_approval_decision(
        &self,
        task_id: &str,
        approval_id: &str,
        decision: &str,
    ) -> Result<bool> {
        if !valid_approval_decision(decision) {
            return Ok(false);
        }
        let Some(session) = self.session_for_task(task_id)? else {
            return Ok(false);
        };
        if !session.can_recover_tool_approval_after_restart(now_ms()) {
            return Ok(false);
        }
        self.append_sidecar_command(
            task_id,
            "tool_approval_decision",
            Some(approval_id),
            Some(decision),
        )
    }

    fn append_sidecar_command(
        &self,
        task_id: &str,
        command: &str,
        approval_id: Option<&str>,
        decision: Option<&str>,
    ) -> Result<bool> {
        let task_id = task_id.trim();
        if task_id.is_empty() {
            return Ok(false);
        }
        self.ensure_dir()?;
        let path = self.command_path(task_id);
        let record = CliSidecarCommandRecord {
            task_id: task_id.to_string(),
            command: command.to_string(),
            approval_id: approval_id.map(str::to_string),
            decision: decision.map(str::to_string),
            at_ms: now_ms(),
        };
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .with_context(|| format!("打开 sidecar command mailbox {:?}", path))?;
        writeln!(file, "{}", serde_json::to_string(&record)?)
            .with_context(|| format!("写入 sidecar command mailbox {:?}", path))?;
        Ok(true)
    }

    fn load_sessions(&self) -> Result<BTreeMap<String, CliSidecarSessionRecord>> {
        let path = self.sessions_path();
        if !path.exists() {
            return Ok(BTreeMap::new());
        }
        let text = fs::read_to_string(&path).with_context(|| format!("读取 {:?}", path))?;
        serde_json::from_str(&text).with_context(|| format!("解析 {:?}", path))
    }

    fn save_sessions(&self, sessions: &BTreeMap<String, CliSidecarSessionRecord>) -> Result<()> {
        self.ensure_dir()?;
        let path = self.sessions_path();
        fs::write(&path, serde_json::to_string_pretty(sessions)?)
            .with_context(|| format!("写入 {:?}", path))
    }

    fn ensure_dir(&self) -> Result<()> {
        fs::create_dir_all(&self.dir).with_context(|| format!("创建 {:?}", self.dir))
    }

    fn sessions_path(&self) -> PathBuf {
        self.dir.join("sessions.json")
    }

    fn command_path(&self, task_id: &str) -> PathBuf {
        self.dir
            .join(format!("commands-{}.jsonl", safe_file_component(task_id)))
    }

    pub(crate) fn command_mailbox_path(&self, task_id: &str) -> PathBuf {
        self.command_path(task_id)
    }

    pub(crate) fn output_path(&self, task_id: &str, session_id: &str) -> PathBuf {
        self.dir.join(format!(
            "output-{}-{}.jsonl",
            safe_file_component(task_id),
            safe_file_component(session_id)
        ))
    }

    pub(crate) fn dir(&self) -> PathBuf {
        self.dir.clone()
    }
}

impl CliSidecarSessionRecord {
    pub(crate) fn managed_conpty(
        session_id: impl Into<String>,
        task_id: impl Into<String>,
        cli_name: impl Into<String>,
        route: impl Into<String>,
        cwd: Option<String>,
        endpoint: Option<String>,
        sidecar_pid: Option<u32>,
        child_pid: Option<u32>,
        now_ms: u128,
    ) -> Self {
        Self {
            session_id: session_id.into(),
            task_id: task_id.into(),
            cli_name: cli_name.into(),
            route: route.into(),
            cwd,
            state: "running".to_string(),
            transport: "managed_conpty_sidecar".to_string(),
            endpoint,
            sidecar_pid,
            child_pid,
            started_at_ms: now_ms,
            last_seen_at_ms: now_ms,
            capabilities: CliSidecarCapabilities {
                terminal_attach: true,
                output_stream_replay: true,
                tool_approval_recovery: true,
                cancel: true,
            },
        }
    }

    pub(crate) fn is_attachable_at(&self, now_ms: u128) -> bool {
        matches!(
            self.state.trim().to_ascii_lowercase().as_str(),
            "running" | "waiting_approval" | "cancel_requested"
        ) && now_ms.saturating_sub(self.last_seen_at_ms) <= SIDECAR_STALE_AFTER_MS
            && self.capabilities.terminal_attach
    }

    pub(crate) fn can_recover_tool_approval_after_restart(&self, now_ms: u128) -> bool {
        self.is_attachable_at(now_ms) && self.capabilities.tool_approval_recovery
    }
}

pub(crate) fn sidecar_status_view(session: &CliSidecarSessionRecord) -> serde_json::Value {
    let now = now_ms();
    json!({
        "session_id": session.session_id,
        "task_id": session.task_id,
        "cli_name": session.cli_name,
        "route": session.route,
        "state": session.state,
        "transport": session.transport,
        "endpoint": session.endpoint,
        "sidecar_pid": session.sidecar_pid,
        "child_pid": session.child_pid,
        "started_at_ms": session.started_at_ms,
        "last_seen_at_ms": session.last_seen_at_ms,
        "attachable_after_restart": session.is_attachable_at(now),
        "approval_recoverable_after_restart": session.can_recover_tool_approval_after_restart(now),
        "capabilities": session.capabilities,
    })
}

pub(crate) fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

fn safe_file_component(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

fn valid_approval_decision(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "approve" | "approved" | "deny" | "denied" | "reject" | "rejected"
    )
}

#[cfg(test)]
mod tests {
    use super::{now_ms, CliSidecarRegistry, CliSidecarSessionRecord};
    use std::{fs, path::PathBuf};

    #[test]
    fn sidecar_session_survives_registry_reload_and_accepts_recovered_approval() {
        let dir = unique_test_dir("approval");
        let _ = fs::remove_dir_all(&dir);
        let registry = CliSidecarRegistry::new(&dir);
        registry
            .upsert_session(CliSidecarSessionRecord::managed_conpty(
                "sidecar-1",
                "task-1",
                "codex",
                "route_a_external_cli",
                Some("D:/demo".to_string()),
                Some("npipe://elon/sidecar-1".to_string()),
                Some(100),
                Some(200),
                now_ms(),
            ))
            .expect("sidecar session should persist");

        let reloaded = CliSidecarRegistry::new(&dir)
            .session_for_task("task-1")
            .expect("session lookup should read")
            .expect("session should exist");
        assert!(reloaded.is_attachable_at(now_ms()));
        assert!(reloaded.can_recover_tool_approval_after_restart(now_ms()));
        assert!(CliSidecarRegistry::new(&dir)
            .record_tool_approval_decision("task-1", "tap_1_1", "approve")
            .expect("sidecar command should persist"));
        assert!(CliSidecarRegistry::new(&dir)
            .record_cancel_command("task-1")
            .expect("sidecar cancel command should persist"));
        let commands =
            fs::read_to_string(dir.join("commands-task-1.jsonl")).expect("mailbox should exist");
        assert!(commands.contains(r#""command":"tool_approval_decision""#));
        assert!(commands.contains(r#""approval_id":"tap_1_1""#));
        assert!(commands.contains(r#""command":"cancel""#));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn stale_sidecar_session_does_not_claim_restart_approval() {
        let now = now_ms();
        let mut session = CliSidecarSessionRecord::managed_conpty(
            "sidecar-1",
            "task-1",
            "codex",
            "route_a_external_cli",
            None,
            None,
            None,
            None,
            now.saturating_sub(10 * 60 * 1_000),
        );
        session.last_seen_at_ms = now.saturating_sub(10 * 60 * 1_000);

        assert!(!session.is_attachable_at(now));
        assert!(!session.can_recover_tool_approval_after_restart(now));
    }

    #[test]
    fn sidecar_commands_fail_closed_without_attachable_session_or_valid_decision() {
        let dir = unique_test_dir("fail-closed");
        let _ = fs::remove_dir_all(&dir);
        let registry = CliSidecarRegistry::new(&dir);

        assert!(!registry
            .record_cancel_command("missing-task")
            .expect("missing sidecar should be handled"));
        assert!(!registry
            .record_tool_approval_decision("missing-task", "tap_1_1", "approve")
            .expect("missing sidecar should be handled"));

        registry
            .upsert_session(CliSidecarSessionRecord::managed_conpty(
                "sidecar-1",
                "task-1",
                "codex",
                "route_a_external_cli",
                None,
                None,
                None,
                None,
                now_ms(),
            ))
            .expect("sidecar session should persist");
        assert!(!registry
            .record_tool_approval_decision("task-1", "tap_1_1", "maybe")
            .expect("invalid decision should be rejected"));
        let _ = fs::remove_dir_all(dir);
    }

    fn unique_test_dir(suffix: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "elon-cli-sidecar-test-{}-{}",
            std::process::id(),
            suffix
        ))
    }
}
