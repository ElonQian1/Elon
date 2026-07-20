// server/src/node_agent_active_task.rs

use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::watch;

use crate::node_agent_tool_approval::PendingToolApprovalView;

const CONTROL_LEASE_MS: u128 = 45_000;

#[derive(Clone)]
pub(crate) struct ActiveCliPromptHandle {
    cancel_tx: watch::Sender<bool>,
    req_id: String,
    cli_name: String,
    route: String,
    cwd: Option<String>,
    runtime_permission: Option<String>,
    exclusive_workspace: bool,
    requires_cloud_control: bool,
    started_at_ms: u128,
    last_heartbeat_ms: u128,
    os_pid: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ActiveCliPromptView {
    pub req_id: String,
    pub run_handle_id: String,
    pub cli_name: String,
    pub route: String,
    pub cwd: Option<String>,
    pub runtime_permission: Option<String>,
    pub requires_cloud_control: bool,
    pub started_at_ms: u128,
    pub last_heartbeat_ms: u128,
    pub control_lease_expires_at_ms: u128,
    pub os_pid: Option<u32>,
    pub control_handle_live: bool,
    pub pending_approvals: Vec<PendingToolApprovalView>,
}

#[derive(Clone)]
pub(crate) struct ActiveCliCancelTarget {
    pub(crate) cancel_tx: watch::Sender<bool>,
    pub(crate) run_handle_id: String,
    pub(crate) started_at_ms: u128,
}

impl ActiveCliPromptHandle {
    pub(crate) fn new(
        req_id: impl Into<String>,
        cli_name: impl Into<String>,
        route: impl Into<String>,
        cwd: Option<String>,
        runtime_permission: Option<String>,
        cancel_tx: watch::Sender<bool>,
    ) -> Self {
        let now = now_ms();
        Self {
            cancel_tx,
            req_id: req_id.into(),
            cli_name: cli_name.into(),
            route: route.into(),
            cwd,
            runtime_permission,
            exclusive_workspace: false,
            requires_cloud_control: false,
            started_at_ms: now,
            last_heartbeat_ms: now,
            os_pid: None,
        }
    }

    pub(crate) fn cancel_tx(&self) -> watch::Sender<bool> {
        self.cancel_tx.clone()
    }

    pub(crate) fn cancel_target(&self) -> ActiveCliCancelTarget {
        ActiveCliCancelTarget {
            cancel_tx: self.cancel_tx(),
            run_handle_id: self.req_id.clone(),
            started_at_ms: self.started_at_ms,
        }
    }

    pub(crate) fn with_requires_cloud_control(mut self, required: bool) -> Self {
        self.requires_cloud_control = required;
        self
    }

    pub(crate) fn with_exclusive_workspace(mut self, exclusive: bool) -> Self {
        self.exclusive_workspace = exclusive;
        self
    }

    pub(crate) fn cwd(&self) -> Option<&str> {
        self.cwd.as_deref()
    }

    pub(crate) fn exclusive_workspace(&self) -> bool {
        self.exclusive_workspace
    }

    pub(crate) fn requires_cloud_control(&self) -> bool {
        self.requires_cloud_control
    }

    pub(crate) fn set_requires_cloud_control(&mut self, required: bool) {
        // Once a task has adopted cloud-managed credentials it remains cloud
        // controlled for the rest of that task, even if a later retry changes
        // credential homes again.
        self.requires_cloud_control |= required;
        self.touch();
    }

    pub(crate) fn req_id(&self) -> &str {
        &self.req_id
    }

    pub(crate) fn set_os_pid(&mut self, pid: Option<u32>) {
        self.os_pid = pid;
        self.touch();
    }

    pub(crate) fn touch(&mut self) {
        self.last_heartbeat_ms = now_ms();
    }

    pub(crate) fn view(
        &self,
        pending_approvals: Vec<PendingToolApprovalView>,
    ) -> ActiveCliPromptView {
        // 这个 lease 只声明“当前节点进程仍握有控制句柄”，不是跨重启恢复承诺。
        ActiveCliPromptView {
            req_id: self.req_id.clone(),
            run_handle_id: self.req_id.clone(),
            cli_name: self.cli_name.clone(),
            route: self.route.clone(),
            cwd: self.cwd.clone(),
            runtime_permission: self.runtime_permission.clone(),
            requires_cloud_control: self.requires_cloud_control,
            started_at_ms: self.started_at_ms,
            last_heartbeat_ms: self.last_heartbeat_ms,
            control_lease_expires_at_ms: now_ms() + CONTROL_LEASE_MS,
            os_pid: self.os_pid,
            control_handle_live: true,
            pending_approvals,
        }
    }
}

pub(crate) fn route_for_cli(cli_name: &str) -> &'static str {
    match cli_name.trim().to_ascii_lowercase().as_str() {
        "api-runtime" => "route_b_api_runtime",
        "server-runtime" => "route_c_server_runtime",
        _ => "route_a_external_cli",
    }
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}
