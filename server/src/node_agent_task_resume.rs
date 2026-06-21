// server/src/node_agent_task_resume.rs

use serde::Serialize;

use crate::node_agent_task_journal::TaskJournalRecord;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct TaskAttachState {
    status: &'static str,
    live: bool,
    can_reconnect: bool,
    continue_mode: &'static str,
    source: &'static str,
    reason: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct TaskResumeContract {
    status: &'static str,
    can_reconnect: bool,
    can_cancel: bool,
    can_stream_live_output: bool,
    continue_mode: &'static str,
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

pub(crate) fn task_attach_state(
    record: Option<&TaskJournalRecord>,
    active: bool,
) -> TaskAttachState {
    if active {
        return TaskAttachState {
            status: "live",
            live: true,
            can_reconnect: true,
            continue_mode: "reconnect_original_process",
            source: "local_journal",
            reason: "本机节点仍持有该任务的运行控制句柄，可以重连控制面，但当前版本还不能回放 stdout/stderr。",
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
        },
        Some(_) => TaskAttachState {
            status: "terminal",
            live: false,
            can_reconnect: false,
            continue_mode: "snapshot_continue",
            source: "local_journal",
            reason: "本机进程已经结束，只能基于任务快照继续新一轮处理。",
        },
        None => TaskAttachState {
            status: "missing",
            live: false,
            can_reconnect: false,
            continue_mode: "snapshot_continue",
            source: "local_journal",
            reason: "本机没有该任务的 journal 记录，前端只能使用云端任务快照。",
        },
    }
}

pub(crate) fn task_resume_contract(attach: &TaskAttachState) -> TaskResumeContract {
    match attach.status {
        "live" => TaskResumeContract {
            status: attach.status,
            can_reconnect: true,
            can_cancel: true,
            can_stream_live_output: false,
            continue_mode: attach.continue_mode,
            strategy: TaskResumeStrategy {
                kind: "control_handle_reconnect",
                label: "重连本机控制句柄",
                reason: "当前本机节点还保留运行句柄，可继续查询状态或停止任务；stdout/stderr 回放仍需后续 attach 协议。",
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
            continue_mode: attach.continue_mode,
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
            continue_mode: attach.continue_mode,
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
            continue_mode: "snapshot_continue",
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
        "当前版本不回放 CLI stdout/stderr 历史流。",
        "当前版本不持久化审批 waiter，页面刷新后只能从历史事件重建审批卡。",
        "节点重启后不能恢复原进程 pid，只能基于快照新开任务继续。",
    ]
}

#[cfg(test)]
mod tests {
    use super::{task_attach_state, task_resume_contract};
    use crate::node_agent_task_journal::TaskJournalRecord;

    fn record(status: &str) -> TaskJournalRecord {
        TaskJournalRecord {
            req_id: "task-1".to_string(),
            cli_name: "codex".to_string(),
            cwd: Some("D:/demo".to_string()),
            runtime_permission: Some("project_write".to_string()),
            status: status.to_string(),
            started_at_ms: 1,
            updated_at_ms: 2,
            cancel_requested_at_ms: None,
        }
    }

    #[test]
    fn live_contract_is_honest_about_stream_replay() {
        let running = record("running");
        let attach = task_attach_state(Some(&running), true);
        let resume = task_resume_contract(&attach);

        assert_eq!(resume.status, "live");
        assert!(resume.can_reconnect);
        assert!(resume.can_cancel);
        assert!(!resume.can_stream_live_output);
        assert_eq!(resume.next_action, "wait_or_cancel");
        assert_eq!(resume.strategy.kind, "control_handle_reconnect");
    }

    #[test]
    fn detached_contract_requires_snapshot_continue() {
        let running = record("running");
        let attach = task_attach_state(Some(&running), false);
        let resume = task_resume_contract(&attach);

        assert_eq!(attach.status, "detached");
        assert!(!resume.can_reconnect);
        assert!(!resume.can_cancel);
        assert_eq!(resume.next_action, "continue_from_snapshot");
        assert_eq!(resume.strategy.kind, "snapshot_continue");
        assert!(resume.strategy.requires_new_task);
    }

    #[test]
    fn terminal_and_missing_contracts_do_not_claim_reconnect() {
        let finished = record("finished");
        let terminal = task_resume_contract(&task_attach_state(Some(&finished), false));
        let missing = task_resume_contract(&task_attach_state(None, false));

        assert_eq!(terminal.status, "terminal");
        assert_eq!(terminal.next_action, "continue_from_snapshot");
        assert_eq!(missing.status, "missing");
        assert_eq!(missing.strategy.kind, "cloud_snapshot_only");
        assert!(!missing.strategy.uses_local_journal);
    }
}
