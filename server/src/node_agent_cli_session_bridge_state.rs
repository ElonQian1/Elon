// server/src/node_agent_cli_session_bridge_state.rs

use serde_json::{json, Value};
use std::collections::BTreeSet;

use crate::{
    node_agent_active_task::ActiveCliPromptView,
    node_agent_cli_sidecar::{now_ms, CliSidecarSessionRecord},
    node_agent_task_journal::TaskJournalRecord,
};

#[derive(Debug, Default)]
pub(crate) struct ContinuitySummary {
    pub(crate) active_control_count: usize,
    pub(crate) sidecar_session_count: usize,
    pub(crate) sidecar_stream_replay_count: usize,
    pub(crate) sidecar_attachable_count: usize,
    pub(crate) sidecar_approval_recoverable_count: usize,
    pub(crate) recent_record_count: usize,
    pub(crate) detached_running_count: usize,
    pub(crate) terminal_record_count: usize,
    pub(crate) codex_session_count: usize,
    pub(crate) route_a_record_count: usize,
    pub(crate) route_b_record_count: usize,
    pub(crate) route_c_record_count: usize,
    pub(crate) last_updated_at_ms: Option<u128>,
}

impl ContinuitySummary {
    pub(crate) fn from_state(
        active_controls: &[ActiveCliPromptView],
        recent_records: &[TaskJournalRecord],
        sidecar_sessions: &[CliSidecarSessionRecord],
    ) -> Self {
        let active_ids: BTreeSet<&str> = active_controls
            .iter()
            .map(|control| control.req_id.as_str())
            .collect();
        let mut seen_records = BTreeSet::new();
        let mut summary = Self {
            active_control_count: active_controls.len(),
            sidecar_session_count: sidecar_sessions.len(),
            ..Self::default()
        };
        let now = now_ms();
        for session in sidecar_sessions {
            summary.last_updated_at_ms = summary
                .last_updated_at_ms
                .map(|current| current.max(session.last_seen_at_ms))
                .or(Some(session.last_seen_at_ms));
            if session.can_replay_output_at(now) {
                summary.sidecar_stream_replay_count += 1;
            }
            if session.is_attachable_at(now) {
                summary.sidecar_attachable_count += 1;
            }
            if session.can_recover_tool_approval_after_restart(now) {
                summary.sidecar_approval_recoverable_count += 1;
            }
        }

        for record in recent_records {
            if !seen_records.insert(record.req_id.as_str()) {
                continue;
            }
            summary.recent_record_count += 1;
            summary.last_updated_at_ms = summary
                .last_updated_at_ms
                .map(|current| current.max(record.updated_at_ms))
                .or(Some(record.updated_at_ms));
            if record_has_codex_session(record) {
                summary.codex_session_count += 1;
            }
            match record.route.as_deref().unwrap_or_default() {
                "route_b_api_runtime" => summary.route_b_record_count += 1,
                "route_c_server_runtime" => summary.route_c_record_count += 1,
                _ => summary.route_a_record_count += 1,
            }
            if active_ids.contains(record.req_id.as_str()) {
                continue;
            }
            if is_running_status(&record.status) {
                summary.detached_running_count += 1;
            } else if is_terminal_status(&record.status) {
                summary.terminal_record_count += 1;
            }
        }

        summary
    }

    pub(crate) fn current_state(&self) -> &'static str {
        if self.active_control_count > 0 {
            "live_control_available"
        } else if self.sidecar_attachable_count > 0 {
            "managed_sidecar_attachable"
        } else if self.sidecar_stream_replay_count > 0 {
            "managed_pipe_json_sidecar_followable"
        } else if self.codex_session_count > 0 {
            "codex_session_resumable"
        } else if self.detached_running_count > 0 {
            "detached_journal_recoverable"
        } else if self.recent_record_count > 0 {
            "journal_snapshot_available"
        } else {
            "ready_no_session"
        }
    }

    pub(crate) fn can_resume_after_restart(&self) -> bool {
        self.sidecar_stream_replay_count > 0
            || self.codex_session_count > 0
            || self.recent_record_count > 0
    }
}

pub(crate) fn display_summary(summary: &ContinuitySummary) -> &'static str {
    match summary.current_state() {
        "live_control_available" => {
            "可重连本机运行句柄；仍不接管原 CLI 终端，输出通过 journal/JSON 桥接。"
        }
        "managed_sidecar_attachable" => {
            "可重接一龙 sidecar 持有的 PTY/ConPTY CLI 会话；节点重启后仍可读写终端、调整尺寸并恢复可验证审批。"
        }
        "managed_pipe_json_sidecar_followable" => {
            "可跟随一龙 pipe sidecar 持有的 Codex JSON 会话；节点重启后仍可回放输出和取消任务，但不支持终端输入。"
        }
        "codex_session_resumable" => {
            "可基于本机 journal 和 Codex session 续接；原 CLI 终端不可重接。"
        }
        "detached_journal_recoverable" => {
            "节点没有活动句柄，但本机 journal 可恢复快照并开启继续处理。"
        }
        "journal_snapshot_available" => "有本机任务 journal，可回放状态并从快照继续。",
        _ => "已准备本机 CLI journal 会话恢复层；暂无可恢复任务。",
    }
}

pub(crate) fn summary_text(summary: &ContinuitySummary) -> &'static str {
    if summary.sidecar_attachable_count > 0 {
        "已具备一龙托管 PTY/ConPTY sidecar 会话恢复层：可在节点重启后通过本机 attach API 读取输出、写入终端输入、调整尺寸，并恢复可验证审批；任意外部终端仍不可接管。"
    } else if summary.sidecar_stream_replay_count > 0 {
        "已具备一龙托管 pipe JSON sidecar 会话恢复层：可在节点重启后回放 Codex JSON 输出、取消任务并保留 journal；该模式没有终端输入或 resize。"
    } else {
        "已具备一龙托管 PTY/ConPTY sidecar 能力、任务 journal、Codex session 和云端快照恢复基础层；当前没有可重接 sidecar 会话时，仍不重新接管非 sidecar 管理的原 CLI TTY，节点重启后的非 sidecar 旧审批卡不会继续批准。"
    }
}

pub(crate) fn recommended_primary_action(summary: &ContinuitySummary) -> &'static str {
    match summary.current_state() {
        "live_control_available" => {
            "仍是 live 任务时，使用本机控制句柄处理取消、状态查询和当前内存中的审批。"
        }
        "managed_sidecar_attachable" => {
            "优先重接 sidecar PTY/ConPTY 会话；终端输入/resize/审批只能写入 sidecar mailbox 并由 sidecar 校验后执行。"
        }
        "managed_pipe_json_sidecar_followable" => {
            "优先跟随 pipe sidecar 的 JSON 输出和 journal；需要真人终端接管时再切到 PTY/ConPTY sidecar。"
        }
        "codex_session_resumable" => {
            "优先使用已记录的 Codex session 自动续接，并在失败时清理失效 session。"
        }
        "detached_journal_recoverable" => {
            "对 detached 任务先回放本机 journal，再基于快照开启新一轮继续。"
        }
        "journal_snapshot_available" => "展示最近任务快照，让用户从明确的继续入口恢复。",
        _ => "暂无旧任务时，直接启动新的本机 CLI 会话并写入 journal。",
    }
}

pub(crate) fn restart_next_action(summary: &ContinuitySummary) -> &'static str {
    if summary.sidecar_attachable_count > 0 {
        "managed_pty_conpty_sidecar_attach"
    } else if summary.sidecar_stream_replay_count > 0 {
        "managed_pipe_json_sidecar_follow"
    } else if summary.codex_session_count > 0 {
        "codex_exec_resume_or_snapshot_continue"
    } else if summary.recent_record_count > 0 {
        "journal_replay_then_snapshot_continue"
    } else {
        "start_new_journaled_cli_session"
    }
}

pub(crate) fn latest_recoverable_task(
    active_controls: &[ActiveCliPromptView],
    recent_records: &[TaskJournalRecord],
    sidecar_sessions: &[CliSidecarSessionRecord],
) -> Option<Value> {
    active_controls
        .first()
        .map(latest_task_from_active)
        .or_else(|| {
            sidecar_sessions
                .iter()
                .filter(|session| session.can_replay_output_at(now_ms()))
                .max_by(|left, right| {
                    left.last_seen_at_ms
                        .cmp(&right.last_seen_at_ms)
                        .then_with(|| left.started_at_ms.cmp(&right.started_at_ms))
                })
                .map(latest_task_from_sidecar)
        })
        .or_else(|| {
            recent_records
                .iter()
                .max_by(|left, right| {
                    left.updated_at_ms
                        .cmp(&right.updated_at_ms)
                        .then_with(|| left.started_at_ms.cmp(&right.started_at_ms))
                })
                .map(latest_task_from_record)
        })
}

fn latest_task_from_sidecar(session: &CliSidecarSessionRecord) -> Value {
    let can_attach_sidecar = session.capabilities.terminal_attach;
    json!({
        "task_id": session.task_id,
        "cli_name": session.cli_name,
        "route": session.route,
        "status": session.state,
        "recovery_kind": if can_attach_sidecar {
            "managed_pty_conpty_sidecar_attach"
        } else {
            "managed_pipe_json_sidecar_follow"
        },
        "can_cancel": session.capabilities.cancel,
        "can_continue_from_snapshot": false,
        "can_resume_codex_session": false,
        "can_attach_sidecar": can_attach_sidecar,
        "can_stream_live_output": session.capabilities.output_stream_replay,
        "can_write_terminal": session.capabilities.terminal_input,
        "can_resize_terminal": session.capabilities.terminal_resize,
        "can_approve_after_node_restart": session.can_recover_tool_approval_after_restart(now_ms()),
        "updated_at_ms": session.last_seen_at_ms,
    })
}

fn latest_task_from_active(control: &ActiveCliPromptView) -> Value {
    json!({
        "task_id": control.req_id,
        "cli_name": control.cli_name,
        "route": control.route,
        "status": "running",
        "recovery_kind": "live_control_handle",
        "can_cancel": control.control_handle_live,
        "can_continue_from_snapshot": false,
        "can_resume_codex_session": false,
        "updated_at_ms": control.last_heartbeat_ms,
    })
}

fn latest_task_from_record(record: &TaskJournalRecord) -> Value {
    json!({
        "task_id": record.req_id,
        "cli_name": record.cli_name,
        "route": record.route,
        "status": record.status,
        "recovery_kind": if record_has_codex_session(record) {
            "codex_session_resume"
        } else {
            "journal_snapshot"
        },
        "can_cancel": false,
        "can_continue_from_snapshot": true,
        "can_resume_codex_session": record_has_codex_session(record),
        "updated_at_ms": record.updated_at_ms,
    })
}

fn record_has_codex_session(record: &TaskJournalRecord) -> bool {
    record
        .codex_session_id
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty())
        && record
            .codex_session_scope_key
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
}

fn is_running_status(status: &str) -> bool {
    matches!(
        status.trim().to_ascii_lowercase().as_str(),
        "running" | "cancel_requested"
    )
}

fn is_terminal_status(status: &str) -> bool {
    matches!(
        status.trim().to_ascii_lowercase().as_str(),
        "finished"
            | "done"
            | "failed"
            | "canceled"
            | "cancelled"
            | "interrupted"
            | "resume_required"
    )
}
