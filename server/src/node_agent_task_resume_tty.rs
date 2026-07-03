// server/src/node_agent_task_resume_tty.rs

use crate::node_agent_task_resume::TaskResumeTtyReattach;

pub(crate) fn tty_reattach_status() -> TaskResumeTtyReattach {
    TaskResumeTtyReattach {
        status: "not_supported",
        supported: false,
        mode: "no_original_cli_tty_reattach",
        fallback: "journal_replay_snapshot_continue_and_codex_session_resume",
        reason: "当前节点只能重连本机控制句柄、回放 journal、处理仍在内存中的审批 waiter，不能重新接管已经打开的原 CLI 终端 TTY。",
        required_future_work: vec![
            "外部 CLI 终端仍不能被接管；需要从一龙 sidecar 启动的任务才有 PTY/ConPTY attach。",
            "非 sidecar 任务继续使用 journal 回放、Codex session resume 和云端快照续跑。",
        ],
    }
}

pub(crate) fn sidecar_tty_reattach_status() -> TaskResumeTtyReattach {
    TaskResumeTtyReattach {
        status: "supported",
        supported: true,
        mode: "managed_pty_conpty_sidecar_reattach",
        fallback: "journal_replay_snapshot_continue_and_codex_session_resume",
        reason: "该任务由一龙 sidecar 启动并持有 PTY/ConPTY 与控制 mailbox，node-agent 重启后可以重接 sidecar、读写终端和 resize，而不是接管任意外部终端。",
        required_future_work: vec![
            "在 PC 前端接入真实终端 attach 面板。",
            "为 sidecar 输出补充屏幕级 buffer/ANSI 视图；当前恢复协议回放 PTY 字节流。",
        ],
    }
}

pub(crate) fn pipe_sidecar_tty_reattach_status() -> TaskResumeTtyReattach {
    TaskResumeTtyReattach {
        status: "not_supported",
        supported: false,
        mode: "managed_pipe_json_sidecar_no_tty",
        fallback: "json_output_replay_cancel_and_codex_session_resume",
        reason: "该任务由一龙 pipe JSON sidecar 启动，stdout/stderr 仍是干净 pipe；可以回放输出和取消任务，但没有 PTY/ConPTY 终端可接管。",
        required_future_work: vec![
            "前端应把该模式展示为结构化过程回放，而不是终端 attach。",
            "如果用户需要真人接管终端，应显式切到 PTY/ConPTY sidecar。",
        ],
    }
}
