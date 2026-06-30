// server/src/node_agent_task_resume_sidecar.rs

use serde::Serialize;

use crate::node_agent_cli_sidecar::{now_ms, CliSidecarSessionRecord};

#[derive(Debug, Clone, Serialize)]
pub(crate) struct TaskResumeSidecarSession {
    pub(crate) session_id: String,
    pub(crate) task_id: String,
    pub(crate) cli_name: String,
    pub(crate) route: String,
    pub(crate) state: String,
    pub(crate) transport: String,
    pub(crate) endpoint: Option<String>,
    pub(crate) sidecar_pid: Option<u32>,
    pub(crate) child_pid: Option<u32>,
    pub(crate) last_seen_at_ms: u128,
    pub(crate) can_attach_terminal: bool,
    pub(crate) can_stream_live_output: bool,
    pub(crate) can_recover_tool_approval_after_restart: bool,
}

pub(crate) fn sidecar_limitations() -> Vec<&'static str> {
    vec![
        "sidecar 恢复文件不保存 prompt 或 API key。",
        "只支持一龙启动并登记的 managed sidecar 会话；不接管用户在外部终端手动启动的任意 CLI。",
        "审批恢复必须通过 sidecar mailbox，并由 sidecar 重新校验审批 id、工具请求和安全指纹。",
        "当前前端仍需要接入终端 attach 面板后，才能展示完整交互式 TTY。",
    ]
}

pub(crate) fn sidecar_session_from_record(
    session: CliSidecarSessionRecord,
) -> Option<TaskResumeSidecarSession> {
    let now = now_ms();
    if !session.is_attachable_at(now) {
        return None;
    }
    let can_recover_tool_approval_after_restart =
        session.can_recover_tool_approval_after_restart(now);
    Some(TaskResumeSidecarSession {
        session_id: session.session_id,
        task_id: session.task_id,
        cli_name: session.cli_name,
        route: session.route,
        state: session.state,
        transport: session.transport,
        endpoint: session.endpoint,
        sidecar_pid: session.sidecar_pid,
        child_pid: session.child_pid,
        last_seen_at_ms: session.last_seen_at_ms,
        can_attach_terminal: session.capabilities.terminal_attach,
        can_stream_live_output: session.capabilities.output_stream_replay,
        can_recover_tool_approval_after_restart,
    })
}
