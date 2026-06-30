use serde_json::{json, Value};
use std::collections::BTreeSet;

use crate::{
    node_agent_active_task::ActiveCliPromptView,
    node_agent_cli_sidecar::{now_ms, sidecar_status_view, CliSidecarSessionRecord},
    node_agent_task_journal::TaskJournalRecord,
};

pub(crate) fn status_payload() -> Value {
    status_payload_for(&[], &[], &[])
}

pub(crate) fn status_payload_for(
    active_controls: &[ActiveCliPromptView],
    recent_records: &[TaskJournalRecord],
    sidecar_sessions: &[CliSidecarSessionRecord],
) -> Value {
    let summary = ContinuitySummary::from_state(active_controls, recent_records, sidecar_sessions);
    let current_state = summary.current_state();
    let latest_recoverable_task =
        latest_recoverable_task(active_controls, recent_records, sidecar_sessions);
    let latest_sidecar_session = sidecar_sessions
        .iter()
        .filter(|session| session.is_attachable_at(now_ms()))
        .max_by(|left, right| {
            left.last_seen_at_ms
                .cmp(&right.last_seen_at_ms)
                .then_with(|| left.started_at_ms.cmp(&right.started_at_ms))
        })
        .map(sidecar_status_view);
    let managed_sidecar_available = summary.sidecar_attachable_count > 0;
    let sidecar_approval_available = summary.sidecar_approval_recoverable_count > 0;
    let status = if managed_sidecar_available {
        "sidecar_recoverable_continuity"
    } else {
        "recoverable_continuity"
    };
    let mode = if managed_sidecar_available {
        "managed_pty_conpty_sidecar_attach"
    } else {
        "spawned_process_json_bridge_with_journal_recovery"
    };
    let restart_mode = if managed_sidecar_available {
        "managed_pty_conpty_sidecar_attach"
    } else {
        "task_journal_snapshot_and_cli_native_resume"
    };
    let restart_reason = if managed_sidecar_available {
        "由一龙启动并由 sidecar 持有的 PTY/ConPTY CLI 会话可以在节点重启后通过本机 attach API 重新读写；prompt/API key 仍不写入恢复文件。"
    } else {
        "节点重启后使用本机 journal 和云端任务快照恢复上下文；Codex 任务还可用已记录 session id 自动尝试 exec resume。"
    };
    let second_recommended_action = if managed_sidecar_available {
        "节点重启后先 attach sidecar 的 PTY/ConPTY 会话；只有 sidecar mailbox 仍可验证的审批才允许继续批准。"
    } else {
        "节点重启或任务 detached 后，回放 journal/快照并开新一轮继续，不再批准非 sidecar 的旧审批卡。"
    };
    let state_summary = json!({
        "active_control_count": summary.active_control_count,
        "sidecar_session_count": summary.sidecar_session_count,
        "sidecar_attachable_count": summary.sidecar_attachable_count,
        "sidecar_approval_recoverable_count": summary.sidecar_approval_recoverable_count,
        "recent_record_count": summary.recent_record_count,
        "detached_running_count": summary.detached_running_count,
        "terminal_record_count": summary.terminal_record_count,
        "codex_session_count": summary.codex_session_count,
        "route_a_record_count": summary.route_a_record_count,
        "route_b_record_count": summary.route_b_record_count,
        "route_c_record_count": summary.route_c_record_count,
        "last_updated_at_ms": summary.last_updated_at_ms,
    });
    let restart_recovery = json!({
        "supported": true,
        "mode": restart_mode,
        "safe_after_node_restart": true,
        "restores_prompt_or_api_key": false,
        "restores_original_tty": managed_sidecar_available,
        "restores_tool_approval_waiter": sidecar_approval_available,
        "next_action": restart_next_action(&summary),
        "reason": restart_reason
    });
    let resume_order = vec![
        json!({
            "kind": "managed_pty_conpty_sidecar_attach",
            "label": "重接一龙 sidecar 持有的 PTY/ConPTY CLI 会话",
            "available_when": "任务由一龙 sidecar 启动且 sidecar 心跳仍有效",
            "currently_available": managed_sidecar_available,
            "requires_new_task": false
        }),
        json!({
            "kind": "live_control_handle",
            "label": "重连本机控制句柄",
            "available_when": "节点仍持有该任务 run_handle",
            "currently_available": summary.active_control_count > 0,
            "requires_new_task": false
        }),
        json!({
            "kind": "journal_replay",
            "label": "回放本机 journal",
            "available_when": "本机仍有任务 journal",
            "currently_available": summary.recent_record_count > 0,
            "requires_new_task": false
        }),
        json!({
            "kind": "codex_session_resume",
            "label": "自动续接 Codex session",
            "available_when": "journal 记录了 Codex session id 和 scope_key",
            "currently_available": summary.codex_session_count > 0,
            "requires_new_task": true
        }),
        json!({
            "kind": "cloud_snapshot_continue",
            "label": "基于云端快照开启新任务",
            "available_when": "本机运行句柄或 journal 不存在",
            "currently_available": summary.recent_record_count == 0,
            "requires_new_task": true
        }),
    ];
    let routes = vec![
        json!({
            "name": "Codex CLI",
            "mode": "managed_pty_conpty_sidecar_or_exec_json_resume",
            "tty_takeover_supported": managed_sidecar_available,
            "continuity": "managed PTY/ConPTY sidecar attach/read/write/resize; fallback codex exec resume --json <thread>"
        }),
        json!({
            "name": "Copilot CLI",
            "mode": "managed_pty_conpty_sidecar_or_continue_in_workspace",
            "tty_takeover_supported": managed_sidecar_available,
            "continuity": "managed PTY/ConPTY sidecar attach/read/write/resize; fallback copilot --continue"
        }),
        json!({
            "name": "Fallback",
            "mode": "backend_context_handoff",
            "tty_takeover_supported": false,
            "continuity": "recent backend conversation records"
        }),
    ];

    json!({
        "status": status,
        "mode": mode,
        "current_state": current_state,
        "tty_takeover_supported": false,
        "pty_takeover_supported": false,
        "managed_tty_reattach_supported": managed_sidecar_available,
        "managed_conpty_sidecar_supported": true,
        "managed_conpty_sidecar_active": managed_sidecar_available,
        "sidecar_protocol_supported": true,
        "sidecar_protocol_mode": "managed_pty_conpty_attach_read_write_resize",
        "sidecar_attach_api": {
            "read": "/api/cli-sidecars/:task_id/attach?since=<offset>",
            "write": "/api/cli-sidecars/:task_id/input",
            "resize": "/api/cli-sidecars/:task_id/resize"
        },
        "sidecar_attachable_count": summary.sidecar_attachable_count,
        "sidecar_approval_recoverable_count": summary.sidecar_approval_recoverable_count,
        "process_handle_reconnect_supported": true,
        "restart_recovery_supported": true,
        "post_restart_approval_supported": sidecar_approval_available,
        "json_stream_supported": true,
        "task_journal_supported": true,
        "codex_resume_supported": true,
        "copilot_continue_supported": true,
        "backend_context_fallback_supported": true,
        "can_reconnect_live_control": summary.active_control_count > 0,
        "can_resume_after_node_restart": summary.can_resume_after_restart(),
        "can_resume_codex_session": summary.codex_session_count > 0,
        "can_continue_from_snapshot": summary.recent_record_count > 0,
        "can_approve_after_node_restart": sidecar_approval_available,
        "display_summary": display_summary(&summary),
        "summary": summary_text(&summary),
        "state_summary": state_summary,
        "latest_recoverable_task": latest_recoverable_task,
        "latest_sidecar_session": latest_sidecar_session,
        "restart_recovery": restart_recovery,
        "not_supported": [
            "attach_external_cli_tty_not_started_by_elon_sidecar",
            "attach_non_sidecar_external_cli_tty",
            "approve_tool_after_node_restart_without_managed_sidecar"
        ],
        "continuity_modes": [
            "managed PTY/ConPTY sidecar attach/read/write/resize",
            "codex exec resume --json <thread>",
            "copilot --continue",
            "backend conversation continuity note"
        ],
        "resume_order": resume_order,
        "recommended_next_actions": [
            recommended_primary_action(&summary),
            second_recommended_action,
            "有 Codex session 记录时由节点自动尝试 exec resume；失败时清理旧 session 并重新开始。",
            "任意外部终端仍不可接管；只有一龙管理的 sidecar 会话进入恢复协议。"
        ],
        "future_work": [
            "PC 网页端已接入项目级 sidecar 终端面板，可重接、读取输出、写入输入、同步尺寸并渲染 ANSI SGR 颜色。",
            "补充屏幕级终端 buffer；当前前端显示 PTY 字节流的 ANSI 可读视图。"
        ],
        "routes": routes
    })
}

#[derive(Debug, Default)]
struct ContinuitySummary {
    active_control_count: usize,
    sidecar_session_count: usize,
    sidecar_attachable_count: usize,
    sidecar_approval_recoverable_count: usize,
    recent_record_count: usize,
    detached_running_count: usize,
    terminal_record_count: usize,
    codex_session_count: usize,
    route_a_record_count: usize,
    route_b_record_count: usize,
    route_c_record_count: usize,
    last_updated_at_ms: Option<u128>,
}

impl ContinuitySummary {
    fn from_state(
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

    fn current_state(&self) -> &'static str {
        if self.active_control_count > 0 {
            "live_control_available"
        } else if self.sidecar_attachable_count > 0 {
            "managed_sidecar_attachable"
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

    fn can_resume_after_restart(&self) -> bool {
        self.sidecar_attachable_count > 0
            || self.codex_session_count > 0
            || self.recent_record_count > 0
    }
}

fn display_summary(summary: &ContinuitySummary) -> &'static str {
    match summary.current_state() {
        "live_control_available" => {
            "可重连本机运行句柄；仍不接管原 CLI 终端，输出通过 journal/JSON 桥接。"
        }
        "managed_sidecar_attachable" => {
            "可重接一龙 sidecar 持有的 PTY/ConPTY CLI 会话；节点重启后仍可读写终端、调整尺寸并恢复可验证审批。"
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

fn summary_text(summary: &ContinuitySummary) -> &'static str {
    if summary.sidecar_attachable_count > 0 {
        "已具备一龙托管 PTY/ConPTY sidecar 会话恢复层：可在节点重启后通过本机 attach API 读取输出、写入终端输入、调整尺寸，并恢复可验证审批；任意外部终端仍不可接管。"
    } else {
        "已具备本机运行句柄、任务 journal、Codex session 和云端快照的恢复基础层；仍不重新接管非 sidecar 管理的原 CLI TTY，节点重启后的非 sidecar 旧审批卡不会继续批准。"
    }
}

fn recommended_primary_action(summary: &ContinuitySummary) -> &'static str {
    match summary.current_state() {
        "live_control_available" => {
            "仍是 live 任务时，使用本机控制句柄处理取消、状态查询和当前内存中的审批。"
        }
        "managed_sidecar_attachable" => {
            "优先重接 sidecar PTY/ConPTY 会话；终端输入/resize/审批只能写入 sidecar mailbox 并由 sidecar 校验后执行。"
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

fn restart_next_action(summary: &ContinuitySummary) -> &'static str {
    if summary.sidecar_attachable_count > 0 {
        "managed_pty_conpty_sidecar_attach"
    } else if summary.codex_session_count > 0 {
        "codex_exec_resume_or_snapshot_continue"
    } else if summary.recent_record_count > 0 {
        "journal_replay_then_snapshot_continue"
    } else {
        "start_new_journaled_cli_session"
    }
}

fn latest_recoverable_task(
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
                .filter(|session| session.is_attachable_at(now_ms()))
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
    json!({
        "task_id": session.task_id,
        "cli_name": session.cli_name,
        "route": session.route,
        "status": session.state,
        "recovery_kind": "managed_pty_conpty_sidecar_attach",
        "can_cancel": session.capabilities.cancel,
        "can_continue_from_snapshot": false,
        "can_resume_codex_session": false,
        "can_attach_sidecar": true,
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
        "finished" | "done" | "failed" | "canceled" | "cancelled" | "interrupted"
    )
}

#[cfg(test)]
mod tests {
    use super::{status_payload, status_payload_for};
    use crate::{
        node_agent_active_task::ActiveCliPromptView,
        node_agent_cli_sidecar::{now_ms, CliSidecarSessionRecord},
        node_agent_task_journal::TaskJournalRecord,
    };

    #[test]
    fn static_status_declares_recoverable_bridge_without_tty_takeover() {
        let status = status_payload();

        assert_eq!(status["tty_takeover_supported"], false);
        assert_eq!(status["status"], "recoverable_continuity");
        assert_eq!(status["current_state"], "ready_no_session");
        assert_eq!(status["restart_recovery_supported"], true);
        assert_eq!(status["post_restart_approval_supported"], false);
        assert_eq!(status["can_resume_after_node_restart"], false);
        assert_eq!(status["json_stream_supported"], true);
        assert_eq!(status["codex_resume_supported"], true);
        assert!(status["display_summary"]
            .as_str()
            .unwrap_or_default()
            .contains("journal"));
        assert!(status["summary"]
            .as_str()
            .unwrap_or_default()
            .contains("恢复基础层"));
        assert!(status["continuity_modes"]
            .as_array()
            .is_some_and(|items| items.len() >= 3));
        assert!(status["not_supported"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| {
                item.as_str() == Some("attach_external_cli_tty_not_started_by_elon_sidecar")
            }));
        assert!(status["resume_order"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| {
                item["kind"].as_str() == Some("codex_session_resume")
                    && item["requires_new_task"].as_bool() == Some(true)
                    && item["currently_available"].as_bool() == Some(false)
            }));
        assert!(status["recommended_next_actions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| {
                item.as_str()
                    .unwrap_or_default()
                    .contains("不再批准非 sidecar")
            }));
        assert_eq!(
            status["sidecar_protocol_mode"],
            "managed_pty_conpty_attach_read_write_resize"
        );
        assert_eq!(status["managed_conpty_sidecar_active"], false);
        assert_eq!(
            status["sidecar_attach_api"]["write"],
            "/api/cli-sidecars/:task_id/input"
        );
    }

    #[test]
    fn live_active_control_is_exposed_as_current_reconnect_path() {
        let active = vec![active_control("req-live")];
        let status = status_payload_for(&active, &[], &[]);

        assert_eq!(status["current_state"], "live_control_available");
        assert_eq!(status["can_reconnect_live_control"], true);
        assert_eq!(status["can_resume_after_node_restart"], false);
        assert_eq!(
            status["latest_recoverable_task"]["recovery_kind"],
            "live_control_handle"
        );
        assert_eq!(status["latest_recoverable_task"]["can_cancel"], true);
        assert_eq!(status["state_summary"]["active_control_count"], 1);
    }

    #[test]
    fn detached_codex_record_makes_restart_resume_available() {
        let records = vec![codex_record("req-codex", "running", 200)];
        let status = status_payload_for(&[], &records, &[]);

        assert_eq!(status["current_state"], "codex_session_resumable");
        assert_eq!(status["can_resume_after_node_restart"], true);
        assert_eq!(status["can_resume_codex_session"], true);
        assert_eq!(
            status["restart_recovery"]["next_action"],
            "codex_exec_resume_or_snapshot_continue"
        );
        assert_eq!(
            status["latest_recoverable_task"]["recovery_kind"],
            "codex_session_resume"
        );
        assert_eq!(
            status["latest_recoverable_task"]["can_resume_codex_session"],
            true
        );
        assert_eq!(status["state_summary"]["detached_running_count"], 1);
        assert_eq!(status["state_summary"]["route_a_record_count"], 1);
    }

    #[test]
    fn route_b_and_c_records_are_counted_for_snapshot_continue() {
        let records = vec![
            record(
                "req-b",
                "api-runtime",
                "route_b_api_runtime",
                "finished",
                100,
            ),
            record(
                "req-c",
                "server-runtime",
                "route_c_server_runtime",
                "cancel_requested",
                300,
            ),
        ];
        let status = status_payload_for(&[], &records, &[]);

        assert_eq!(status["current_state"], "detached_journal_recoverable");
        assert_eq!(status["can_continue_from_snapshot"], true);
        assert_eq!(
            status["restart_recovery"]["next_action"],
            "journal_replay_then_snapshot_continue"
        );
        assert_eq!(status["state_summary"]["route_b_record_count"], 1);
        assert_eq!(status["state_summary"]["route_c_record_count"], 1);
        assert_eq!(status["state_summary"]["detached_running_count"], 1);
        assert_eq!(status["state_summary"]["terminal_record_count"], 1);
        assert_eq!(status["latest_recoverable_task"]["task_id"], "req-c");
    }

    #[test]
    fn sidecar_session_takes_priority_for_restart_attach_and_approval_recovery() {
        let sidecars = vec![sidecar("sidecar-1", "req-sidecar", now_ms())];
        let records = vec![record(
            "req-sidecar",
            "codex",
            "route_a_external_cli",
            "running",
            100,
        )];
        let status = status_payload_for(&[], &records, &sidecars);

        assert_eq!(status["status"], "sidecar_recoverable_continuity");
        assert_eq!(status["current_state"], "managed_sidecar_attachable");
        assert_eq!(status["managed_tty_reattach_supported"], true);
        assert_eq!(status["can_resume_after_node_restart"], true);
        assert_eq!(status["can_approve_after_node_restart"], true);
        assert_eq!(
            status["restart_recovery"]["mode"],
            "managed_pty_conpty_sidecar_attach"
        );
        assert_eq!(status["restart_recovery"]["restores_original_tty"], true);
        assert_eq!(
            status["restart_recovery"]["restores_tool_approval_waiter"],
            true
        );
        assert_eq!(
            status["latest_recoverable_task"]["recovery_kind"],
            "managed_pty_conpty_sidecar_attach"
        );
        assert_eq!(
            status["latest_recoverable_task"]["can_write_terminal"],
            true
        );
        assert_eq!(
            status["latest_recoverable_task"]["can_resize_terminal"],
            true
        );
        assert_eq!(status["latest_sidecar_session"]["session_id"], "sidecar-1");
        assert_eq!(status["state_summary"]["sidecar_attachable_count"], 1);
        assert_eq!(
            status["resume_order"].as_array().unwrap().first().unwrap()["kind"],
            "managed_pty_conpty_sidecar_attach"
        );
    }

    fn active_control(req_id: &str) -> ActiveCliPromptView {
        ActiveCliPromptView {
            req_id: req_id.to_string(),
            run_handle_id: req_id.to_string(),
            cli_name: "codex".to_string(),
            route: "route_a_external_cli".to_string(),
            cwd: Some("D:/demo".to_string()),
            runtime_permission: Some("project_write".to_string()),
            started_at_ms: 1,
            last_heartbeat_ms: 2,
            control_lease_expires_at_ms: 47_000,
            os_pid: Some(4242),
            control_handle_live: true,
            pending_approvals: Vec::new(),
        }
    }

    fn codex_record(req_id: &str, status: &str, updated_at_ms: u128) -> TaskJournalRecord {
        let mut record = record(
            req_id,
            "codex",
            "route_a_external_cli",
            status,
            updated_at_ms,
        );
        record.codex_session_id = Some("codex-session-uuid".to_string());
        record.codex_session_scope_key = Some("scope-key".to_string());
        record.codex_session_updated_at_ms = Some(updated_at_ms);
        record
    }

    fn sidecar(session_id: &str, task_id: &str, last_seen_at_ms: u128) -> CliSidecarSessionRecord {
        let mut session = CliSidecarSessionRecord::managed_conpty(
            session_id,
            task_id,
            "codex",
            "route_a_external_cli",
            Some("D:/demo".to_string()),
            Some("npipe://elon/sidecar-1".to_string()),
            Some(100),
            Some(200),
            last_seen_at_ms,
        );
        session.last_seen_at_ms = last_seen_at_ms;
        session
    }

    fn record(
        req_id: &str,
        cli_name: &str,
        route: &str,
        status: &str,
        updated_at_ms: u128,
    ) -> TaskJournalRecord {
        TaskJournalRecord {
            req_id: req_id.to_string(),
            cli_name: cli_name.to_string(),
            route: Some(route.to_string()),
            run_handle_id: Some(req_id.to_string()),
            cwd: Some("D:/demo".to_string()),
            runtime_permission: Some("project_write".to_string()),
            os_pid: None,
            process_started_at_ms: None,
            codex_session_id: None,
            codex_session_scope_key: None,
            codex_session_updated_at_ms: None,
            status: status.to_string(),
            started_at_ms: 1,
            updated_at_ms,
            cancel_requested_at_ms: None,
        }
    }
}
