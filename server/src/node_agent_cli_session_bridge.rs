use serde_json::{json, Value};
use std::collections::BTreeSet;

use crate::{
    node_agent_active_task::ActiveCliPromptView, node_agent_task_journal::TaskJournalRecord,
};

pub(crate) fn status_payload() -> Value {
    status_payload_for(&[], &[])
}

pub(crate) fn status_payload_for(
    active_controls: &[ActiveCliPromptView],
    recent_records: &[TaskJournalRecord],
) -> Value {
    let summary = ContinuitySummary::from_state(active_controls, recent_records);
    let current_state = summary.current_state();
    let latest_recoverable_task = latest_recoverable_task(active_controls, recent_records);

    json!({
        "status": "recoverable_continuity",
        "mode": "spawned_process_json_bridge_with_journal_recovery",
        "current_state": current_state,
        "tty_takeover_supported": false,
        "pty_takeover_supported": false,
        "process_handle_reconnect_supported": true,
        "restart_recovery_supported": true,
        "post_restart_approval_supported": false,
        "json_stream_supported": true,
        "task_journal_supported": true,
        "codex_resume_supported": true,
        "copilot_continue_supported": true,
        "backend_context_fallback_supported": true,
        "can_reconnect_live_control": summary.active_control_count > 0,
        "can_resume_after_node_restart": summary.can_resume_after_restart(),
        "can_resume_codex_session": summary.codex_session_count > 0,
        "can_continue_from_snapshot": summary.recent_record_count > 0,
        "can_approve_after_node_restart": false,
        "display_summary": display_summary(&summary),
        "summary": "已具备本机运行句柄、任务 journal、Codex session 和云端快照的恢复基础层；仍不重新接管已经打开的原 CLI TTY，节点重启后的旧审批卡不会继续批准。",
        "state_summary": {
            "active_control_count": summary.active_control_count,
            "recent_record_count": summary.recent_record_count,
            "detached_running_count": summary.detached_running_count,
            "terminal_record_count": summary.terminal_record_count,
            "codex_session_count": summary.codex_session_count,
            "route_a_record_count": summary.route_a_record_count,
            "route_b_record_count": summary.route_b_record_count,
            "route_c_record_count": summary.route_c_record_count,
            "last_updated_at_ms": summary.last_updated_at_ms,
        },
        "latest_recoverable_task": latest_recoverable_task,
        "restart_recovery": {
            "supported": true,
            "mode": "task_journal_snapshot_and_cli_native_resume",
            "safe_after_node_restart": true,
            "restores_prompt_or_api_key": false,
            "restores_original_tty": false,
            "restores_tool_approval_waiter": false,
            "next_action": restart_next_action(&summary),
            "reason": "节点重启后使用本机 journal 和云端任务快照恢复上下文；Codex 任务还可用已记录 session id 自动尝试 exec resume。"
        },
        "not_supported": [
            "attach_existing_cli_tty",
            "stream_pixels_or_terminal_buffer_from_original_cli",
            "approve_tool_after_node_restart"
        ],
        "continuity_modes": [
            "codex exec resume --json <thread>",
            "copilot --continue",
            "backend conversation continuity note"
        ],
        "resume_order": [
            {
                "kind": "live_control_handle",
                "label": "重连本机控制句柄",
                "available_when": "节点仍持有该任务 run_handle",
                "currently_available": summary.active_control_count > 0,
                "requires_new_task": false
            },
            {
                "kind": "journal_replay",
                "label": "回放本机 journal",
                "available_when": "本机仍有任务 journal",
                "currently_available": summary.recent_record_count > 0,
                "requires_new_task": false
            },
            {
                "kind": "codex_session_resume",
                "label": "自动续接 Codex session",
                "available_when": "journal 记录了 Codex session id 和 scope_key",
                "currently_available": summary.codex_session_count > 0,
                "requires_new_task": true
            },
            {
                "kind": "cloud_snapshot_continue",
                "label": "基于云端快照开启新任务",
                "available_when": "本机运行句柄或 journal 不存在",
                "currently_available": summary.recent_record_count == 0,
                "requires_new_task": true
            }
        ],
        "recommended_next_actions": [
            recommended_primary_action(&summary),
            "节点重启或任务 detached 后，回放 journal/快照并开新一轮继续，不再批准旧审批卡。",
            "有 Codex session 记录时由节点自动尝试 exec resume；失败时清理旧 session 并重新开始。",
            "真正原 TTY 接管仍需要后续 PTY/ConPTY attach 协议。"
        ],
        "future_work": [
            "为 Route A CLI 子进程建立可恢复 PTY/ConPTY 会话层。",
            "把 PTY 会话 id、生命周期和安全授权写入本机 journal。",
            "在 PC 前端接入 attach 协议和权限确认。"
        ],
        "routes": [
            {
                "name": "Codex CLI",
                "mode": "exec_json_resume",
                "tty_takeover_supported": false,
                "continuity": "codex exec resume --json <thread>"
            },
            {
                "name": "Copilot CLI",
                "mode": "continue_in_workspace",
                "tty_takeover_supported": false,
                "continuity": "copilot --continue"
            },
            {
                "name": "Fallback",
                "mode": "backend_context_handoff",
                "tty_takeover_supported": false,
                "continuity": "recent backend conversation records"
            }
        ]
    })
}

#[derive(Debug, Default)]
struct ContinuitySummary {
    active_control_count: usize,
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
    ) -> Self {
        let active_ids: BTreeSet<&str> = active_controls
            .iter()
            .map(|control| control.req_id.as_str())
            .collect();
        let mut seen_records = BTreeSet::new();
        let mut summary = Self {
            active_control_count: active_controls.len(),
            ..Self::default()
        };

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
        self.codex_session_count > 0 || self.recent_record_count > 0
    }
}

fn display_summary(summary: &ContinuitySummary) -> &'static str {
    match summary.current_state() {
        "live_control_available" => {
            "可重连本机运行句柄；仍不接管原 CLI 终端，输出通过 journal/JSON 桥接。"
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

fn recommended_primary_action(summary: &ContinuitySummary) -> &'static str {
    match summary.current_state() {
        "live_control_available" => {
            "仍是 live 任务时，使用本机控制句柄处理取消、状态查询和当前内存中的审批。"
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
    if summary.codex_session_count > 0 {
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
) -> Option<Value> {
    active_controls
        .first()
        .map(latest_task_from_active)
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
        node_agent_active_task::ActiveCliPromptView, node_agent_task_journal::TaskJournalRecord,
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
            .any(|item| item.as_str() == Some("attach_existing_cli_tty")));
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
                    .contains("不再批准旧审批卡")
            }));
        assert!(status["future_work"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item.as_str().unwrap_or_default().contains("ConPTY")));
    }

    #[test]
    fn live_active_control_is_exposed_as_current_reconnect_path() {
        let active = vec![active_control("req-live")];
        let status = status_payload_for(&active, &[]);

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
        let status = status_payload_for(&[], &records);

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
        let status = status_payload_for(&[], &records);

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
