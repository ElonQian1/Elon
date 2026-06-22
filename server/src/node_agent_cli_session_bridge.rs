use serde_json::{json, Value};

pub(crate) fn status_payload() -> Value {
    json!({
        "mode": "spawned_process_json_bridge",
        "tty_takeover_supported": false,
        "pty_takeover_supported": false,
        "json_stream_supported": true,
        "codex_resume_supported": true,
        "copilot_continue_supported": true,
        "backend_context_fallback_supported": true,
        "summary": "不能重新接管已经打开的原 CLI TTY；当前通过新 CLI 子进程、JSON 输出流、Codex resume / Copilot continue 和后端对话上下文兜底来保持连续性。",
        "continuity_modes": [
            "codex exec resume --json <thread>",
            "copilot --continue",
            "backend conversation continuity note"
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
        assert_eq!(status["json_stream_supported"], true);
        assert_eq!(status["codex_resume_supported"], true);
        assert!(status["summary"]
            .as_str()
            .unwrap_or_default()
            .contains("不能重新接管"));
        assert!(status["continuity_modes"]
            .as_array()
            .is_some_and(|items| items.len() >= 3));
    }
}
