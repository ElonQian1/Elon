//! Background refresh for PC project workspace health snapshots.
//!
//! User-facing health checks still do live inspection on demand. This monitor
//! keeps archive and node-capacity views useful even before a user opens the
//! health page.

use homecli_proto::{AgentToServer, ProjectWorkspaceInspectStatus};
use std::{sync::Arc, time::Duration};

use crate::{
    node_runtime::node_runtime_by_id,
    project_workspace_lifecycle::workspace_lifecycle,
    store::{ProjectWorkspaceHealthSnapshotWrite, ProjectWorkspaceHealthTarget},
    types::AppState,
};

pub fn spawn_project_workspace_health_monitor(state: Arc<AppState>) {
    if std::env::var("PC_WORKSPACE_HEALTH_MONITOR_DISABLED")
        .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
    {
        tracing::info!("PC workspace health monitor disabled");
        return;
    }

    let interval_secs = std::env::var("PC_WORKSPACE_HEALTH_INTERVAL_SECS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value >= 60)
        .unwrap_or(600);
    let limit = std::env::var("PC_WORKSPACE_HEALTH_SCAN_LIMIT")
        .ok()
        .and_then(|value| value.parse::<i64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(100);

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));
        loop {
            interval.tick().await;
            match refresh_workspace_health_snapshots(&state, limit).await {
                Ok(count) if count > 0 => {
                    tracing::info!(count, "PC workspace health snapshots refreshed");
                }
                Ok(_) => {}
                Err(e) => tracing::warn!(error = %e, "PC workspace health monitor failed"),
            }
        }
    });
}

async fn refresh_workspace_health_snapshots(state: &AppState, limit: i64) -> anyhow::Result<usize> {
    let targets = state.store.list_project_workspace_health_targets(limit)?;
    let mut refreshed = 0;
    for target in targets {
        if let Err(e) = refresh_one_workspace_health_snapshot(state, &target).await {
            tracing::warn!(
                project_id = %target.project_id,
                node_id = %target.node_id,
                error = %e,
                "failed to refresh PC workspace health snapshot"
            );
        } else {
            refreshed += 1;
        }
    }
    Ok(refreshed)
}

async fn refresh_one_workspace_health_snapshot(
    state: &AppState,
    target: &ProjectWorkspaceHealthTarget,
) -> anyhow::Result<()> {
    let node = node_runtime_by_id(state, &target.node_id).await?;
    let node_online = node.as_ref().map(|node| node.online).unwrap_or(false);
    let node_can_run_project_cli = node
        .as_ref()
        .map(|node| node.cli_connected && node.cli_project_ready())
        .unwrap_or(false);
    let can_run_on_pc = node_can_run_project_cli && !target.workspace_path.trim().is_empty();
    let (live_inspect, inspect_error) =
        inspect_target_workspace(state, target, node_can_run_project_cli).await;

    let mut warnings = warnings_for_target(target, node.as_ref());
    if let Some(error) = inspect_error.as_deref() {
        warnings.push(format!("PC 工作区后台巡检失败：{error}"));
    }
    if let Some(status) = live_inspect.as_ref() {
        append_inspect_warnings(status, &mut warnings);
    }

    let verified_can_run_on_pc = live_inspect
        .as_ref()
        .map(|status| status.path_exists && status.is_dir && cli_available(status))
        .or_else(|| (!node_can_run_project_cli && node.is_some()).then_some(false));
    let lifecycle = workspace_lifecycle(
        "pc_node_workspace",
        Some(&target.node_id),
        Some(&target.workspace_path),
        node_online,
        can_run_on_pc,
        verified_can_run_on_pc,
        live_inspect.as_ref(),
        warnings.len(),
    );

    state
        .store
        .upsert_project_workspace_health_snapshot(ProjectWorkspaceHealthSnapshotWrite {
            project_id: &target.project_id,
            node_id: Some(&target.node_id),
            workspace_path: Some(&target.workspace_path),
            can_run_on_pc,
            verified_can_run_on_pc,
            health_label: &lifecycle.health_label,
            health_tone: lifecycle.health_tone,
            recommended_action: &lifecycle.recommended_action,
            warnings: &warnings,
            live_inspect: live_inspect.as_ref(),
            inspect_error: inspect_error.as_deref(),
        })?;
    Ok(())
}

async fn inspect_target_workspace(
    state: &AppState,
    target: &ProjectWorkspaceHealthTarget,
    node_can_run_project_cli: bool,
) -> (Option<ProjectWorkspaceInspectStatus>, Option<String>) {
    if !node_can_run_project_cli {
        return (None, None);
    }
    match state
        .agent_manager
        .dispatch_project_workspace_inspect(&target.node_id, target.workspace_path.clone())
        .await
    {
        Ok(AgentToServer::ProjectWorkspaceInspected { status, .. }) => (Some(status), None),
        Ok(AgentToServer::ProjectWorkspaceInspectError { message, .. }) => (None, Some(message)),
        Ok(other) => (
            None,
            Some(format!("unexpected inspect response: {other:?}")),
        ),
        Err(e) => (None, Some(e.to_string())),
    }
}

fn warnings_for_target(
    target: &ProjectWorkspaceHealthTarget,
    node: Option<&crate::node_runtime::NodeRuntime>,
) -> Vec<String> {
    let mut warnings = Vec::new();
    if target.source_type == "local_path" {
        warnings.push("外部本地项目按登记路径执行，请确认该路径仍属于当前 PC 节点".to_string());
    }
    if !node.map(|node| node.online).unwrap_or(false) {
        warnings.push("PC 节点当前不在线".to_string());
    } else if !node.map(|node| node.cli_connected).unwrap_or(false) {
        warnings.push("PC CLI 通道未连接，无法执行项目代码任务".to_string());
    } else if !node.map(|node| node.cli_project_ready()).unwrap_or(false) {
        warnings.push("PC 节点未上报 Codex/Copilot CLI 能力".to_string());
    }
    warnings
}

fn append_inspect_warnings(status: &ProjectWorkspaceInspectStatus, warnings: &mut Vec<String>) {
    if !status.path_exists {
        warnings.push("PC 工作区目录不存在，项目无法在该节点继续执行".to_string());
    } else if !status.is_dir {
        warnings.push("PC workspace_path 不是目录".to_string());
    } else if !status.is_git_worktree {
        warnings.push("PC 工作区不是 Git worktree，后续合并/回收能力受限".to_string());
    }

    if status.has_uncommitted_changes {
        let count = status.uncommitted_count.unwrap_or(0);
        warnings.push(format!("PC 工作区存在 {count} 个未提交改动"));
    }
    if !cli_available(status) {
        warnings.push("PC 节点未检测到 codex 或 copilot CLI".to_string());
    }
    if matches!(status.disk_free_bytes, Some(bytes) if bytes < 2 * 1024 * 1024 * 1024) {
        warnings.push("PC 工作区所在磁盘剩余空间低于 2GB".to_string());
    }
}

fn cli_available(status: &ProjectWorkspaceInspectStatus) -> bool {
    status.codex_available || status.copilot_available
}
