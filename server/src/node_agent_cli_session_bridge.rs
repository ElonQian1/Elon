use serde_json::{json, Value};

pub(crate) fn status_payload() -> Value {
    json!({
        "status": "limited_continuity",
        "mode": "spawned_process_json_bridge",
        "tty_takeover_supported": false,
        "pty_takeover_supported": false,
        "json_stream_supported": true,
        "codex_resume_supported": true,
        "copilot_continue_supported": true,
        "backend_context_fallback_supported": true,
        "display_summary": "不接管原 CLI 终端；优先通过运行句柄、journal 回放、Codex session 或云端快照继续。",
        "summary": "不能重新接管已经打开的原 CLI TTY；当前通过新 CLI 子进程、JSON 输出流、Codex resume / Copilot continue 和后端对话上下文兜底来保持连续性。",
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
                "requires_new_task": false
            },
            {
                "kind": "journal_replay",
                "label": "回放本机 journal",
                "available_when": "本机仍有任务 journal",
                "requires_new_task": false
            },
            {
                "kind": "codex_session_resume",
                "label": "自动续接 Codex session",
                "available_when": "journal 记录了 Codex session id 和 scope_key",
                "requires_new_task": true
            },
            {
                "kind": "cloud_snapshot_continue",
                "label": "基于云端快照开启新任务",
                "available_when": "本机运行句柄或 journal 不存在",
                "requires_new_task": true
            }
        ],
        "recommended_next_actions": [
            "仍是 live 任务时，使用本机控制句柄处理取消、状态查询和当前内存中的审批。",
            "节点重启或任务 detached 后，只回放 journal/快照，不再批准旧审批卡。",
            "有 Codex session 记录时由节点自动尝试 resume；失败时清理旧 session 并重新开始。",
            "未来要实现真正 TTY 接管，需要先建设可恢复 PTY/ConPTY 会话层。"
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

#[cfg(test)]
mod tests {
    use super::status_payload;

    #[test]
    fn status_declares_tty_takeover_limitation_and_resume_bridge() {
        let status = status_payload();

        assert_eq!(status["tty_takeover_supported"], false);
        assert_eq!(status["status"], "limited_continuity");
        assert_eq!(status["json_stream_supported"], true);
        assert_eq!(status["codex_resume_supported"], true);
        assert!(status["display_summary"]
            .as_str()
            .unwrap_or_default()
            .contains("journal"));
        assert!(status["summary"]
            .as_str()
            .unwrap_or_default()
            .contains("不能重新接管"));
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
}
