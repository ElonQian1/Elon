//! Resolve chained supervision tasks back to the authoritative project root.

use std::{collections::HashSet, path::Path};

use anyhow::{bail, Context, Result};

use crate::{
    node_agent_local_task_store::{LocalTaskRecord, LocalTaskStore},
    node_agent_local_task_supervision::{SupervisionContract, SUPERVISION_PROTOCOL},
    node_agent_task_journal::TaskJournal,
};

pub(crate) fn resolve_request_workspace(
    runtime: &crate::NodeRuntime,
    creds: &crate::Credentials,
    project_id: &str,
    requested_workspace_path: &str,
    contract: Option<&SupervisionContract>,
) -> Result<String> {
    let Some(contract) = contract.filter(|contract| {
        matches!(
            contract.task_role.as_str(),
            "capability_repair" | "post_task_improvement"
        )
    }) else {
        return Ok(requested_workspace_path.to_string());
    };
    resolve_chained_authorized_workspace(
        &runtime.local_tasks,
        &runtime.task_journal,
        creds,
        &runtime.install_id,
        project_id,
        requested_workspace_path,
        contract,
    )
}

pub(crate) fn resolve_chained_authorized_workspace(
    tasks: &LocalTaskStore,
    journal: &TaskJournal,
    creds: &crate::Credentials,
    install_id: &str,
    project_id: &str,
    requested_workspace_path: &str,
    contract: &SupervisionContract,
) -> Result<String> {
    if contract.protocol != SUPERVISION_PROTOCOL
        || !matches!(
            contract.task_role.as_str(),
            "capability_repair" | "post_task_improvement"
        )
    {
        bail!("只有当前监督协议的链式 Improve 可以解析根工作区。");
    }
    let parent_id = required_id(contract.parent_task_id.as_deref(), "parent_task_id")?;
    let root_id = required_id(contract.root_task_id.as_deref(), "root_task_id")?;
    let mut current = tasks
        .get_for_owner(&creds.owner_user_id, parent_id)?
        .context("链式 Improve 的父任务不存在或不属于当前账号。")?;
    let immediate_parent = current.clone();
    let mut visited = HashSet::new();

    loop {
        validate_task_identity(&current, creds, install_id, project_id)?;
        if !visited.insert(current.task_id.clone()) {
            bail!("监督任务谱系包含循环，已拒绝继承工作区。");
        }
        let current_contract = crate::node_agent_local_task_supervision::load_supervision_contract(
            journal,
            &current.task_id,
        )?
        .context("监督任务谱系缺少持久契约。")?;
        if current_contract.protocol != SUPERVISION_PROTOCOL {
            bail!("监督任务谱系包含非当前协议任务。");
        }
        let recorded_root = current_contract
            .root_task_id
            .as_deref()
            .unwrap_or(&current.task_id);
        if recorded_root != root_id {
            bail!("链式 Improve 的 root_task_id 与持久任务谱系不一致。");
        }
        if current.task_id == root_id {
            break;
        }
        let ancestor_id = required_id(
            current_contract.parent_task_id.as_deref(),
            "持久谱系 parent_task_id",
        )?;
        current = tasks
            .get_for_owner(&creds.owner_user_id, ancestor_id)?
            .context("监督任务谱系的祖先任务不存在。")?;
    }

    let base = recorded_base_workspace(&current)?;
    validate_requested_workspace(requested_workspace_path, &immediate_parent, &base)?;
    Ok(base)
}

fn validate_task_identity(
    task: &LocalTaskRecord,
    creds: &crate::Credentials,
    install_id: &str,
    project_id: &str,
) -> Result<()> {
    if task.agent_id != creds.agent_id
        || task.install_id != install_id
        || task.owner_user_id != creds.owner_user_id
        || !crate::node_agent_full_access::project_ids_equivalent(&task.project_id, project_id)
    {
        bail!("链式 Improve 不能跨 owner、节点、安装实例或项目继承授权。");
    }
    Ok(())
}

fn recorded_base_workspace(root: &LocalTaskRecord) -> Result<String> {
    let status = root
        .workspace_status
        .as_ref()
        .context("root task 缺少平台工作区身份。")?;
    if status.get("isolated").and_then(serde_json::Value::as_bool) != Some(true) {
        bail!("root task 不是平台隔离任务，已拒绝链式继承。");
    }
    let base = status
        .get("base_workspace_path")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .context("root task 缺少权威 base_workspace_path。")?;
    let base = Path::new(base);
    if !base.is_absolute() || !base.is_dir() {
        bail!("root task 的权威基础工作区不存在或不是绝对目录。");
    }
    Ok(base.to_string_lossy().to_string())
}

fn validate_requested_workspace(
    requested: &str,
    parent: &LocalTaskRecord,
    root_base: &str,
) -> Result<()> {
    let requested = Path::new(requested);
    let mut allowed = vec![Path::new(&parent.workspace_path), Path::new(root_base)];
    if let Some(status) = parent.workspace_status.as_ref() {
        for field in ["base_workspace_path", "active_workspace_path"] {
            if let Some(path) = status.get(field).and_then(serde_json::Value::as_str) {
                allowed.push(Path::new(path));
            }
        }
    }
    if !allowed
        .into_iter()
        .any(|path| crate::node_agent_update_checkpoint::same_path(requested, path))
    {
        bail!("链式 Improve 请求路径不属于父任务或 root 项目绑定。");
    }
    Ok(())
}

fn required_id<'a>(value: Option<&'a str>, name: &str) -> Result<&'a str> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .with_context(|| format!("链式 Improve 缺少 {name}。"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(
        workspace_path: &str,
        workspace_status: Option<serde_json::Value>,
    ) -> LocalTaskRecord {
        LocalTaskRecord {
            task_id: "local-parent".into(),
            owner_user_id: "owner".into(),
            agent_id: "agent".into(),
            install_id: "install".into(),
            project_id: "elon-self".into(),
            channel_id: None,
            conversation_id: "conversation".into(),
            workspace_path: workspace_path.into(),
            prompt: "prompt".into(),
            cli: "codex".into(),
            runtime_permission: "full_access".into(),
            execution_origin: "local_offline".into(),
            billing_source: "own_codex".into(),
            status: "failed".into(),
            error: None,
            final_reply: None,
            model: None,
            codex_session_id: None,
            input_tokens: None,
            cached_input_tokens: None,
            output_tokens: None,
            reasoning_tokens: None,
            total_tokens: None,
            workspace_status,
            sync_state: "local_only".into(),
            completion_event_id: None,
            started_at_ms: 1,
            finished_at_ms: Some(2),
            server_ack_at_ms: None,
        }
    }

    #[test]
    fn chained_request_accepts_parent_worktree_but_resolves_root_base() {
        let base =
            std::env::temp_dir().join(format!("elon-chain-base-{}", uuid::Uuid::new_v4().simple()));
        let active = base.join("conversation-worktrees").join("child");
        std::fs::create_dir_all(&active).unwrap();
        let root = record(
            active.to_string_lossy().as_ref(),
            Some(serde_json::json!({
                "isolated": true,
                "base_workspace_path": base,
                "active_workspace_path": active,
            })),
        );

        let resolved = recorded_base_workspace(&root).unwrap();
        assert!(crate::node_agent_update_checkpoint::same_path(
            Path::new(&resolved),
            &base
        ));
        validate_requested_workspace(active.to_string_lossy().as_ref(), &root, &resolved).unwrap();
        assert!(validate_requested_workspace("C:/unrelated-root", &root, &resolved).is_err());
        let _ = std::fs::remove_dir_all(base);
    }
}
