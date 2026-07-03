// server/src/node_agent_cli_session_bridge_capabilities.rs

use serde_json::{json, Value};

pub(crate) const MANAGED_SIDECAR_RECONNECT_SUPPORTED: bool = true;
pub(crate) const SIDECAR_TOOL_APPROVAL_RECOVERY_SUPPORTED: bool = true;

pub(crate) fn capability_summary(
    managed_sidecar_available: bool,
    pipe_json_sidecar_available: bool,
    sidecar_approval_available: bool,
    sidecar_attachable_count: usize,
    sidecar_stream_replay_count: usize,
    sidecar_approval_recoverable_count: usize,
    codex_session_count: usize,
    recent_record_count: usize,
) -> Value {
    json!({
        "managed_pty_conpty_sidecar": {
            "supported": MANAGED_SIDECAR_RECONNECT_SUPPORTED,
            "currently_available": managed_sidecar_available,
            "attachable_count": sidecar_attachable_count,
            "mode": "managed_pty_conpty_attach_read_write_resize",
            "requires": "task_started_by_elon_sidecar"
        },
        "managed_pipe_json_sidecar": {
            "supported": true,
            "currently_available": pipe_json_sidecar_available,
            "stream_replay_count": sidecar_stream_replay_count,
            "mode": "managed_pipe_json_output_replay_cancel",
            "requires": "codex_exec_json_started_by_elon_sidecar"
        },
        "post_restart_tool_approval": {
            "supported": SIDECAR_TOOL_APPROVAL_RECOVERY_SUPPORTED,
            "currently_available": sidecar_approval_available,
            "recoverable_count": sidecar_approval_recoverable_count,
            "requires": "active_managed_sidecar_waiting_approval"
        },
        "external_tty_takeover": {
            "supported": false,
            "currently_available": false,
            "reason": "只支持一龙托管 sidecar 会话重接，不接管任意外部终端。"
        },
        "codex_session_resume": {
            "supported": true,
            "currently_available": codex_session_count > 0,
            "session_count": codex_session_count
        },
        "journal_snapshot_continue": {
            "supported": true,
            "currently_available": recent_record_count > 0,
            "record_count": recent_record_count
        }
    })
}

pub(crate) fn insert_compat_fields(
    payload: &mut Value,
    capability_summary: Value,
    managed_sidecar_available: bool,
    sidecar_approval_available: bool,
) {
    let Some(object) = payload.as_object_mut() else {
        return;
    };
    object.insert(
        "managed_tty_reattach_capability_supported".to_string(),
        json!(MANAGED_SIDECAR_RECONNECT_SUPPORTED),
    );
    object.insert(
        "managed_tty_reattach_currently_available".to_string(),
        json!(managed_sidecar_available),
    );
    object.insert(
        "sidecar_tool_approval_recovery_supported".to_string(),
        json!(SIDECAR_TOOL_APPROVAL_RECOVERY_SUPPORTED),
    );
    object.insert(
        "sidecar_tool_approval_recovery_currently_available".to_string(),
        json!(sidecar_approval_available),
    );
    object.insert(
        "post_restart_approval_capability_supported".to_string(),
        json!(SIDECAR_TOOL_APPROVAL_RECOVERY_SUPPORTED),
    );
    object.insert(
        "post_restart_approval_currently_available".to_string(),
        json!(sidecar_approval_available),
    );
    object.insert("capability_summary".to_string(), capability_summary);
}
