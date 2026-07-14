// server/src/node_agent_task_resume.rs

#[path = "node_agent_task_resume_tty.rs"]
mod tty;

use serde::Serialize;

use crate::{
    node_agent_active_task::ActiveCliPromptView,
    node_agent_cli_sidecar::CliSidecarSessionRecord,
    node_agent_task_approval_snapshot::TaskApprovalJournalSnapshot,
    node_agent_task_journal::TaskJournalRecord,
    node_agent_task_resume_sidecar::{
        sidecar_limitations, sidecar_session_from_record, TaskResumeSidecarSession,
    },
};
use tty::{pipe_sidecar_tty_reattach_status, sidecar_tty_reattach_status, tty_reattach_status};

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
    sidecar_session: Option<TaskResumeSidecarSession>,
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
    tool_approval_recovery: TaskResumeToolApprovalRecovery,
    can_resume_codex_session: bool,
    codex_session: Option<TaskResumeCodexSession>,
    continue_mode: &'static str,
    tty_reattach: TaskResumeTtyReattach,
    sidecar_session: Option<TaskResumeSidecarSession>,
    run_handle: Option<TaskResumeRunHandle>,
    strategy: TaskResumeStrategy,
    limitations: Vec<&'static str>,
    next_action: &'static str,
    reason: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct TaskResumeTtyReattach {
    pub(crate) status: &'static str,
    pub(crate) supported: bool,
    pub(crate) mode: &'static str,
    pub(crate) fallback: &'static str,
    pub(crate) reason: &'static str,
    pub(crate) required_future_work: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
struct TaskResumeToolApprovalRecovery {
    status: &'static str,
    can_approve_now: bool,
    active_approval_ids: Vec<String>,
    journal_pending_approval_ids: Vec<String>,
    journal_pending_count: usize,
    replay_source: &'static str,
    pending_after_restart_action: &'static str,
    reason: &'static str,
    required_future_work: Vec<&'static str>,
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
    task_attach_state_with_sidecar(record, active, None)
}

pub(crate) fn task_attach_state_with_sidecar(
    record: Option<&TaskJournalRecord>,
    active: Option<ActiveCliPromptView>,
    sidecar: Option<CliSidecarSessionRecord>,
) -> TaskAttachState {
    let codex_session = codex_session_from_record(record);
    let sidecar_session = sidecar.and_then(sidecar_session_from_record);
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
            sidecar_session,
        };
    }
    if let Some(sidecar_session) = sidecar_session {
        return TaskAttachState {
            status: "sidecar_recoverable",
            live: false,
            can_reconnect: true,
            continue_mode: "managed_sidecar_attach",
            source: "sidecar_registry",
            reason: "该任务由一龙 sidecar 持有，node-agent 重启后可以重接 sidecar 控制面并恢复可验证审批。",
            run_handle: None,
            codex_session,
            sidecar_session: Some(sidecar_session),
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
            sidecar_session: None,
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
            sidecar_session: None,
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
            sidecar_session: None,
        },
    }
}

pub(crate) fn task_resume_contract(attach: &TaskAttachState) -> TaskResumeContract {
    task_resume_contract_with_journal_pending(attach, Vec::new())
}

pub(crate) fn task_resume_contract_with_journal_approvals(
    attach: &TaskAttachState,
    approvals: &TaskApprovalJournalSnapshot,
) -> TaskResumeContract {
    task_resume_contract_with_journal_pending(attach, approvals.pending_approval_ids())
}

fn task_resume_contract_with_journal_pending(
    attach: &TaskAttachState,
    journal_pending_approval_ids: Vec<String>,
) -> TaskResumeContract {
    let active_approval_ids = active_approval_ids(attach);
    let can_approve_tools = !active_approval_ids.is_empty();
    let sidecar_pending_approval_ids = journal_pending_approval_ids.clone();
    let tool_approval_recovery = tool_approval_recovery_status(
        attach.status,
        active_approval_ids.clone(),
        journal_pending_approval_ids,
    );
    let run_handle = resume_run_handle(attach);
    let codex_session = attach.codex_session.clone();
    let sidecar_session = attach.sidecar_session.clone();
    let can_resume_codex_session = codex_session.is_some();
    let sidecar_can_attach_terminal = attach
        .sidecar_session
        .as_ref()
        .is_some_and(|session| session.can_attach_terminal);
    let sidecar_can_cancel = attach
        .sidecar_session
        .as_ref()
        .is_some_and(|session| session.can_cancel);
    let sidecar_strategy_kind = if sidecar_can_attach_terminal {
        "managed_pty_conpty_sidecar_attach"
    } else {
        "managed_pipe_json_sidecar_follow"
    };
    let sidecar_strategy_label = if sidecar_can_attach_terminal {
        "重接 PTY/ConPTY sidecar 会话"
    } else {
        "跟随 pipe JSON sidecar 会话"
    };
    let sidecar_strategy_reason = if sidecar_can_attach_terminal {
        "任务由一龙 sidecar 持有 PTY/ConPTY，node-agent 重启后可重新连接 sidecar 控制面；终端输入、resize 和审批决定写入 sidecar mailbox，由 sidecar 复核后执行。"
    } else {
        "任务由一龙 pipe sidecar 持有，node-agent 重启后可重新读取结构化输出、发送取消命令，并继续依赖 Codex JSON 事件展示公开过程；该模式没有终端输入或 resize。"
    };
    match attach.status {
        "live" => TaskResumeContract {
            status: attach.status,
            can_reconnect: true,
            can_cancel: true,
            can_stream_live_output: false,
            can_replay_journal_events: true,
            can_approve_tools,
            active_approval_ids,
            tool_approval_recovery,
            can_resume_codex_session,
            codex_session,
            continue_mode: attach.continue_mode,
            tty_reattach: tty_reattach_status(),
            sidecar_session,
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
        "sidecar_recoverable" => TaskResumeContract {
            status: attach.status,
            can_reconnect: true,
            can_cancel: sidecar_can_cancel,
            can_stream_live_output: attach
                .sidecar_session
                .as_ref()
                .is_some_and(|session| session.can_stream_live_output),
            can_replay_journal_events: true,
            can_approve_tools: attach
                .sidecar_session
                .as_ref()
                .is_some_and(|session| {
                    session.can_recover_tool_approval_after_restart
                        && !sidecar_pending_approval_ids.is_empty()
                }),
            active_approval_ids: sidecar_pending_approval_ids,
            tool_approval_recovery,
            can_resume_codex_session,
            codex_session,
            continue_mode: attach.continue_mode,
            tty_reattach: if sidecar_can_attach_terminal {
                sidecar_tty_reattach_status()
            } else {
                pipe_sidecar_tty_reattach_status()
            },
            sidecar_session,
            run_handle: None,
            strategy: TaskResumeStrategy {
                kind: sidecar_strategy_kind,
                label: sidecar_strategy_label,
                reason: sidecar_strategy_reason,
                requires_new_task: false,
                uses_cloud_snapshot: false,
                uses_local_journal: true,
            },
            limitations: sidecar_limitations(),
            next_action: "attach_sidecar",
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
            tool_approval_recovery,
            can_resume_codex_session,
            codex_session,
            continue_mode: attach.continue_mode,
            tty_reattach: tty_reattach_status(),
            sidecar_session,
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
            tool_approval_recovery,
            can_resume_codex_session,
            codex_session,
            continue_mode: attach.continue_mode,
            tty_reattach: tty_reattach_status(),
            sidecar_session,
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
            tool_approval_recovery,
            can_resume_codex_session: false,
            codex_session: None,
            continue_mode: "snapshot_continue",
            tty_reattach: tty_reattach_status(),
            sidecar_session,
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

impl TaskResumeContract {
    pub(crate) fn status(&self) -> &'static str {
        self.status
    }

    pub(crate) fn can_approve_tools(&self) -> bool {
        self.can_approve_tools
    }

    pub(crate) fn active_approval_ids(&self) -> &[String] {
        &self.active_approval_ids
    }

    pub(crate) fn next_action(&self) -> &'static str {
        self.next_action
    }

    pub(crate) fn reason(&self) -> &'static str {
        self.reason
    }
}

fn tool_approval_recovery_status(
    attach_status: &'static str,
    active_approval_ids: Vec<String>,
    journal_pending_approval_ids: Vec<String>,
) -> TaskResumeToolApprovalRecovery {
    let journal_pending_count = journal_pending_approval_ids.len();
    match attach_status {
        "live" if !active_approval_ids.is_empty() => TaskResumeToolApprovalRecovery {
            status: "active_waiter",
            can_approve_now: true,
            active_approval_ids,
            journal_pending_approval_ids,
            journal_pending_count,
            replay_source: "local_journal_and_memory_waiter",
            pending_after_restart_action: "approve_or_deny_current_waiter",
            reason:
                "当前节点进程仍持有工具审批 waiter，PC 端可以继续批准或拒绝这些审批。",
            required_future_work: Vec::new(),
        },
        "live" => TaskResumeToolApprovalRecovery {
            status: "no_active_waiter",
            can_approve_now: false,
            active_approval_ids: Vec::new(),
            journal_pending_approval_ids,
            journal_pending_count,
            replay_source: "local_journal",
            pending_after_restart_action: "wait_refresh_or_continue_from_snapshot",
            reason:
                "本机任务仍有运行句柄，但当前没有活动工具审批 waiter；历史审批卡只能回放，不能继续点击。",
            required_future_work: vec![
                "将工具审批 waiter 状态写入可恢复本机存储。",
                "恢复时校验任务进程、文件 hash 和审批请求仍然一致。",
            ],
        },
        "sidecar_recoverable" if !journal_pending_approval_ids.is_empty() => {
            TaskResumeToolApprovalRecovery {
                status: "sidecar_waiter_recoverable",
                can_approve_now: true,
                active_approval_ids: Vec::new(),
                journal_pending_approval_ids,
                journal_pending_count,
                replay_source: "sidecar_mailbox_and_local_journal",
                pending_after_restart_action: "approve_or_deny_sidecar_waiter",
                reason: "审批 waiter 由 sidecar 持有；node-agent 重启后只写入 sidecar mailbox，由 sidecar 校验任务、审批 id 和安全指纹后继续执行。",
                required_future_work: Vec::new(),
            }
        }
        "sidecar_recoverable" => TaskResumeToolApprovalRecovery {
            status: "sidecar_no_pending_waiter",
            can_approve_now: false,
            active_approval_ids: Vec::new(),
            journal_pending_approval_ids,
            journal_pending_count,
            replay_source: "sidecar_mailbox_and_local_journal",
            pending_after_restart_action: "wait_refresh_or_attach_sidecar",
            reason: "sidecar 会话可重接，但本机 journal 当前没有未决审批 id；前端应先刷新 sidecar 状态。",
            required_future_work: Vec::new(),
        },
        "detached" => TaskResumeToolApprovalRecovery {
            status: "lost_after_restart",
            can_approve_now: false,
            active_approval_ids: Vec::new(),
            journal_pending_approval_ids,
            journal_pending_count,
            replay_source: "local_journal",
            pending_after_restart_action: "continue_from_snapshot",
            reason:
                "节点重启或任务进程脱离后，内存中的审批 waiter 已丢失；历史审批卡必须失效，只能基于快照开启新任务。",
            required_future_work: vec![
                "落库审批请求、审批到期时间和文件安全指纹。",
                "恢复审批前重新校验工作区状态和工具请求仍可安全执行。",
            ],
        },
        "terminal" => TaskResumeToolApprovalRecovery {
            status: "closed_by_terminal_task",
            can_approve_now: false,
            active_approval_ids: Vec::new(),
            journal_pending_approval_ids,
            journal_pending_count,
            replay_source: "local_journal",
            pending_after_restart_action: "none",
            reason: "任务已进入终态，所有未处理工具审批都应显示为已关闭或已失效。",
            required_future_work: Vec::new(),
        },
        _ => TaskResumeToolApprovalRecovery {
            status: "unavailable",
            can_approve_now: false,
            active_approval_ids: Vec::new(),
            journal_pending_approval_ids,
            journal_pending_count,
            replay_source: "cloud_snapshot_only",
            pending_after_restart_action: "refresh_snapshot",
            reason: "当前 PC 节点没有本机 journal 现场，不能判断或继续任何工具审批。",
            required_future_work: Vec::new(),
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
#[path = "node_agent_task_resume_tests.rs"]
mod tests;
