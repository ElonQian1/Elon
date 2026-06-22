// server/src/node_agent_task_resume.rs

use serde::Serialize;

use crate::{
    node_agent_active_task::ActiveCliPromptView, node_agent_task_journal::TaskJournalRecord,
};

#[derive(Debug, Clone, Serialize)]
pub(crate) struct TaskAttachState {
    status: &'static str,
    live: bool,
    can_reconnect: bool,
    continue_mode: &'static str,
    source: &'static str,
    reason: &'static str,
    run_handle: Option<ActiveCliPromptView>,
    codex_session: Option<TaskResumeCodexSession>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct TaskResumeContract {
    status: &'static str,
    can_reconnect: bool,
    can_cancel: bool,
    can_stream_live_output: bool,
    can_replay_journal_events: bool,
    can_approve_tools: bool,
    active_approval_ids: Vec<String>,
    can_resume_codex_session: bool,
    codex_session: Option<TaskResumeCodexSession>,
    continue_mode: &'static str,
    run_handle: Option<TaskResumeRunHandle>,
    strategy: TaskResumeStrategy,
    limitations: Vec<&'static str>,
    next_action: &'static str,
    reason: &'static str,
}

#[derive(Debug, Clone, Serialize)]
struct TaskResumeStrategy {
    kind: &'static str,
    label: &'static str,
    reason: &'static str,
    requires_new_task: bool,
    uses_cloud_snapshot: bool,
    uses_local_journal: bool,
}

#[derive(Debug, Clone, Serialize)]
struct TaskResumeRunHandle {
    id: String,
    route: String,
    os_pid: Option<u32>,
    control_lease_expires_at_ms: u128,
}

#[derive(Debug, Clone, Serialize)]
struct TaskResumeCodexSession {
    id: String,
    scope_key: String,
    updated_at_ms: Option<u128>,
}

pub(crate) fn task_attach_state(
    record: Option<&TaskJournalRecord>,
    active: Option<ActiveCliPromptView>,
) -> TaskAttachState {
    let codex_session = codex_session_from_record(record);
    if let Some(handle) = active {
        return TaskAttachState {
            status: "live",
            live: true,
            can_reconnect: true,
            continue_mode: "reconnect_original_process",
            source: "local_journal",
            reason:
                "本机节点仍持有该任务的运行控制句柄，可以重连控制面并处理当前内存中的审批 waiter。",
            run_handle: Some(handle),
            codex_session,
        };
    }
    match record.map(|record| record.status.as_str()) {
        Some("running" | "cancel_requested") => TaskAttachState {
            status: "detached",
            live: false,
            can_reconnect: false,
            continue_mode: "snapshot_continue",
            source: "local_journal",
            reason: "本机 journal 显示任务未终态，但当前节点已没有运行句柄，只能基于快照继续。",
            run_handle: None,
            codex_session,
        },
        Some(_) => TaskAttachState {
            status: "terminal",
            live: false,
            can_reconnect: false,
            continue_mode: "snapshot_continue",
            source: "local_journal",
            reason: "本机进程已经结束，只能基于任务快照继续新一轮处理。",
            run_handle: None,
            codex_session,
        },
        None => TaskAttachState {
            status: "missing",
            live: false,
            can_reconnect: false,
            continue_mode: "snapshot_continue",
            source: "local_journal",
            reason: "本机没有该任务的 journal 记录，前端只能使用云端任务快照。",
            run_handle: None,
            codex_session: None,
        },
    }
}

pub(crate) fn task_resume_contract(attach: &TaskAttachState) -> TaskResumeContract {
    let active_approval_ids = active_approval_ids(attach);
    let can_approve_tools = !active_approval_ids.is_empty();
    let run_handle = resume_run_handle(attach);
    let codex_session = attach.codex_session.clone();
    let can_resume_codex_session = codex_session.is_some();
    match attach.status {
        "live" => TaskResumeContract {
            status: attach.status,
            can_reconnect: true,
            can_cancel: true,
            can_stream_live_output: false,
            can_replay_journal_events: true,
            can_approve_tools,
            active_approval_ids,
            can_resume_codex_session,
            codex_session,
            continue_mode: attach.continue_mode,
            run_handle,
            strategy: TaskResumeStrategy {
                kind: "control_handle_reconnect",
                label: "重连本机控制句柄",
                reason: "当前本机节点还保留运行句柄，可查询状态、停止任务、处理仍在内存中的工具审批，并通过本机 journal 轮询回放输出事件；直接接管 CLI TTY 仍需后续 attach 协议。",
                requires_new_task: false,
                uses_cloud_snapshot: true,
                uses_local_journal: true,
            },
            limitations: shared_limitations(),
            next_action: "wait_or_cancel",
            reason: attach.reason,
        },
        "detached" => TaskResumeContract {
            status: attach.status,
            can_reconnect: false,
            can_cancel: false,
            can_stream_live_output: false,
            can_replay_journal_events: true,
            can_approve_tools: false,
            active_approval_ids: Vec::new(),
            can_resume_codex_session,
            codex_session,
            continue_mode: attach.continue_mode,
            run_handle: None,
            strategy: TaskResumeStrategy {
                kind: "snapshot_continue",
                label: "基于快照继续",
                reason: "原进程控制句柄已经丢失，需要新开一轮任务并先检查工作区状态。",
                requires_new_task: true,
                uses_cloud_snapshot: true,
                uses_local_journal: true,
            },
            limitations: shared_limitations(),
            next_action: "continue_from_snapshot",
            reason: attach.reason,
        },
        "terminal" => TaskResumeContract {
            status: attach.status,
            can_reconnect: false,
            can_cancel: false,
            can_stream_live_output: false,
            can_replay_journal_events: true,
            can_approve_tools: false,
            active_approval_ids: Vec::new(),
            can_resume_codex_session,
            codex_session,
            continue_mode: attach.continue_mode,
            run_handle: None,
            strategy: TaskResumeStrategy {
                kind: "snapshot_continue",
                label: "基于终态快照继续",
                reason: "任务已经结束，只能读取云端消息和本机 journal 后开启新任务继续迭代。",
                requires_new_task: true,
                uses_cloud_snapshot: true,
                uses_local_journal: true,
            },
            limitations: shared_limitations(),
            next_action: "continue_from_snapshot",
            reason: attach.reason,
        },
        _ => TaskResumeContract {
            status: "missing",
            can_reconnect: false,
            can_cancel: false,
            can_stream_live_output: false,
            can_replay_journal_events: false,
            can_approve_tools: false,
            active_approval_ids: Vec::new(),
            can_resume_codex_session: false,
            codex_session: None,
            continue_mode: "snapshot_continue",
            run_handle: None,
            strategy: TaskResumeStrategy {
                kind: "cloud_snapshot_only",
                label: "仅使用云端快照",
                reason: "当前 PC 节点没有对应 journal，不能判断本机进程现场。",
                requires_new_task: true,
                uses_cloud_snapshot: true,
                uses_local_journal: false,
            },
            limitations: shared_limitations(),
            next_action: "refresh_snapshot",
            reason: attach.reason,
        },
    }
}

fn shared_limitations() -> Vec<&'static str> {
    vec![
        "本机 journal 不保存 prompt 或 API key。",
        "输出回放来自本机 journal 快照/轮询，不是直接接管原 CLI TTY。",
        "Codex session id 只用于本机 Codex CLI 续接，不包含 prompt 或 API key。",
        "审批 waiter 只在当前节点进程内有效；节点重启后历史审批卡会失效。",
        "节点重启后不能重新绑定原进程控制句柄，只能基于快照新开任务继续。",
    ]
}

fn codex_session_from_record(record: Option<&TaskJournalRecord>) -> Option<TaskResumeCodexSession> {
    let record = record?;
    let id = record
        .codex_session_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    let scope_key = record
        .codex_session_scope_key
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    Some(TaskResumeCodexSession {
        id: id.to_string(),
        scope_key: scope_key.to_string(),
        updated_at_ms: record.codex_session_updated_at_ms,
    })
}

fn active_approval_ids(attach: &TaskAttachState) -> Vec<String> {
    attach
        .run_handle
        .as_ref()
        .map(|handle| {
            handle
                .pending_approvals
                .iter()
                .map(|approval| approval.approval_id.clone())
                .collect()
        })
        .unwrap_or_default()
}

fn resume_run_handle(attach: &TaskAttachState) -> Option<TaskResumeRunHandle> {
    attach
        .run_handle
        .as_ref()
        .map(|handle| TaskResumeRunHandle {
            id: handle.run_handle_id.clone(),
            route: handle.route.clone(),
            os_pid: handle.os_pid,
            control_lease_expires_at_ms: handle.control_lease_expires_at_ms,
        })
}

#[cfg(test)]
mod tests {
    use super::{task_attach_state, task_resume_contract};
    use crate::{
        node_agent_active_task::ActiveCliPromptView, node_agent_task_journal::TaskJournalRecord,
        node_agent_tool_approval::PendingToolApprovalView,
    };

    fn record(status: &str) -> TaskJournalRecord {
        TaskJournalRecord {
            req_id: "task-1".to_string(),
            cli_name: "codex".to_string(),
            route: Some("route_a_external_cli".to_string()),
            run_handle_id: Some("task-1".to_string()),
            cwd: Some("D:/demo".to_string()),
            runtime_permission: Some("project_write".to_string()),
            os_pid: Some(4242),
            process_started_at_ms: Some(1),
            codex_session_id: None,
            codex_session_scope_key: None,
            codex_session_updated_at_ms: None,
            status: status.to_string(),
            started_at_ms: 1,
            updated_at_ms: 2,
            cancel_requested_at_ms: None,
        }
    }

    fn codex_record(status: &str) -> TaskJournalRecord {
        let mut record = record(status);
        record.codex_session_id = Some("session-uuid".to_string());
        record.codex_session_scope_key = Some("scope-a".to_string());
        record.codex_session_updated_at_ms = Some(9);
        record
    }

    fn active_handle() -> ActiveCliPromptView {
        ActiveCliPromptView {
            req_id: "task-1".to_string(),
            run_handle_id: "task-1".to_string(),
            cli_name: "server-runtime".to_string(),
            route: "route_c_server_runtime".to_string(),
            cwd: Some("D:/demo".to_string()),
            runtime_permission: Some("project_write".to_string()),
            started_at_ms: 1,
            last_heartbeat_ms: 2,
            control_lease_expires_at_ms: 47_000,
            os_pid: None,
            control_handle_live: true,
            pending_approvals: vec![PendingToolApprovalView {
                approval_id: "tap_1_1".to_string(),
                registered_at_ms: 3,
            }],
        }
    }

    #[test]
    fn live_contract_is_honest_about_stream_replay() {
        let running = record("running");
        let attach = task_attach_state(Some(&running), Some(active_handle()));
        let resume = task_resume_contract(&attach);

        assert_eq!(resume.status, "live");
        assert!(resume.can_reconnect);
        assert!(resume.can_cancel);
        assert!(!resume.can_stream_live_output);
        assert!(resume.can_replay_journal_events);
        assert!(resume.can_approve_tools);
        assert!(!resume.can_resume_codex_session);
        assert!(resume.codex_session.is_none());
        assert_eq!(resume.active_approval_ids, vec!["tap_1_1"]);
        assert_eq!(
            resume
                .run_handle
                .as_ref()
                .map(|handle| handle.route.as_str()),
            Some("route_c_server_runtime")
        );
        assert_eq!(resume.next_action, "wait_or_cancel");
        assert_eq!(resume.strategy.kind, "control_handle_reconnect");
    }

    #[test]
    fn detached_contract_requires_snapshot_continue() {
        let running = record("running");
        let attach = task_attach_state(Some(&running), None);
        let resume = task_resume_contract(&attach);

        assert_eq!(attach.status, "detached");
        assert!(!resume.can_reconnect);
        assert!(!resume.can_cancel);
        assert!(!resume.can_approve_tools);
        assert!(resume.active_approval_ids.is_empty());
        assert_eq!(resume.next_action, "continue_from_snapshot");
        assert_eq!(resume.strategy.kind, "snapshot_continue");
        assert!(resume.strategy.requires_new_task);
    }

    #[test]
    fn cancel_requested_without_live_handle_cannot_be_canceled_again() {
        let canceling = record("cancel_requested");
        let attach = task_attach_state(Some(&canceling), None);
        let resume = task_resume_contract(&attach);

        assert_eq!(attach.status, "detached");
        assert!(!resume.can_reconnect);
        assert!(!resume.can_cancel);
        assert!(!resume.can_approve_tools);
        assert_eq!(resume.next_action, "continue_from_snapshot");
        assert!(resume
            .limitations
            .iter()
            .any(|item| item.contains("节点重启后不能重新绑定原进程控制句柄")));
    }

    #[test]
    fn live_codex_task_exposes_control_and_session_continuity() {
        let running = codex_record("running");
        let attach = task_attach_state(Some(&running), Some(active_handle()));
        let resume = task_resume_contract(&attach);

        assert_eq!(resume.status, "live");
        assert!(resume.can_reconnect);
        assert!(resume.can_cancel);
        assert!(resume.can_resume_codex_session);
        assert_eq!(resume.next_action, "wait_or_cancel");
        assert_eq!(
            resume
                .codex_session
                .as_ref()
                .map(|session| session.id.as_str()),
            Some("session-uuid")
        );
    }

    #[test]
    fn codex_session_is_exposed_for_snapshot_continue() {
        let running = codex_record("running");
        let resume = task_resume_contract(&task_attach_state(Some(&running), None));

        assert_eq!(resume.status, "detached");
        assert!(resume.can_resume_codex_session);
        assert_eq!(
            resume
                .codex_session
                .as_ref()
                .map(|session| session.id.as_str()),
            Some("session-uuid")
        );
        assert_eq!(
            resume
                .codex_session
                .as_ref()
                .map(|session| session.scope_key.as_str()),
            Some("scope-a")
        );
    }

    #[test]
    fn terminal_and_missing_contracts_do_not_claim_reconnect() {
        let finished = record("finished");
        let terminal = task_resume_contract(&task_attach_state(Some(&finished), None));
        let missing = task_resume_contract(&task_attach_state(None, None));

        assert_eq!(terminal.status, "terminal");
        assert_eq!(terminal.next_action, "continue_from_snapshot");
        assert_eq!(missing.status, "missing");
        assert_eq!(missing.strategy.kind, "cloud_snapshot_only");
        assert!(!missing.strategy.uses_local_journal);
    }
}
