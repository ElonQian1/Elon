//! Trust rules for upgrading a historical descendant lease to its root lease.

use std::{collections::HashSet, path::Path};

use anyhow::{anyhow, bail, Context, Result};

use crate::{
    node_agent_local_task_store::{LocalTaskRecord, LocalTaskStore},
    node_agent_local_task_supervision::{SupervisionContract, SUPERVISION_PROTOCOL},
    node_agent_task_journal::TaskJournal,
};

const MAX_LINEAGE_DEPTH: usize = 64;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct LegacyLeaseMigration {
    pub(crate) legacy_task_id: String,
    pub(crate) root_task_id: String,
}

pub(crate) fn assess_legacy_lease(
    actual_reason: Option<&str>,
    expected_reason: &str,
    contract: &SupervisionContract,
    parent: &LocalTaskRecord,
    parent_contract: Option<&SupervisionContract>,
) -> Result<Option<LegacyLeaseMigration>> {
    if actual_reason == Some(expected_reason) {
        return Ok(None);
    }
    let root_task_id = contract
        .root_task_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            anyhow!("旧子任务 lease 迁移要求 resume_original 显式携带 root_task_id。")
        })?;
    let legacy_reason =
        crate::node_agent_supervision_worktree_lease::lease_reason(&parent.task_id)?;
    if actual_reason != Some(legacy_reason.as_str()) {
        bail!(
            "父任务 worktree root lease 身份不匹配：expected {expected_reason}, actual {}",
            actual_reason.unwrap_or("<unlocked>")
        );
    }
    anyhow::ensure!(
        parent.task_id != root_task_id,
        "旧 lease 与 root_task_id 相同但 reason 校验不一致"
    );
    let parent_contract =
        parent_contract.ok_or_else(|| anyhow!("旧子任务 lease 缺少可验证的父监督契约。"))?;
    anyhow::ensure!(
        parent_contract.protocol == SUPERVISION_PROTOCOL
            && parent_contract.supervisor == contract.supervisor
            && parent_contract.root_task_id.as_deref() == Some(root_task_id)
            && parent_contract.parent_task_id.is_some()
            && is_descendant_role(&parent_contract.task_role),
        "旧子任务 lease 的监督谱系身份不可信，已拒绝迁移。"
    );
    Ok(Some(LegacyLeaseMigration {
        legacy_task_id: parent.task_id.clone(),
        root_task_id: root_task_id.to_string(),
    }))
}

pub(crate) fn validate_full_lineage(
    local_tasks: &LocalTaskStore,
    task_journal: &TaskJournal,
    parent: &LocalTaskRecord,
    parent_contract: &SupervisionContract,
    migration: &LegacyLeaseMigration,
) -> Result<()> {
    anyhow::ensure!(
        parent.task_id == migration.legacy_task_id,
        "lease 迁移候选与父任务身份不一致"
    );
    let owner = parent.owner_user_id.as_str();
    let agent = parent.agent_id.as_str();
    let install = parent.install_id.as_str();
    let project = parent.project_id.as_str();
    let workspace = parent.workspace_path.as_str();
    let supervisor = parent_contract.supervisor.as_str();
    let mut current = parent.clone();
    let mut contract = parent_contract.clone();
    let mut visited = HashSet::new();

    for _ in 0..MAX_LINEAGE_DEPTH {
        anyhow::ensure!(
            visited.insert(current.task_id.clone()),
            "监督任务谱系存在循环引用"
        );
        validate_task_identity(&current, owner, agent, install, project, workspace)?;
        anyhow::ensure!(
            contract.protocol == SUPERVISION_PROTOCOL && contract.supervisor == supervisor,
            "监督任务谱系的协议或 supervisor 身份漂移"
        );

        if current.task_id == migration.root_task_id {
            anyhow::ensure!(
                contract.task_role == "requirement"
                    && contract.parent_task_id.is_none()
                    && contract
                        .root_task_id
                        .as_deref()
                        .is_none_or(|root| root == migration.root_task_id),
                "监督谱系根任务不是可信的 requirement 根"
            );
            return Ok(());
        }

        anyhow::ensure!(
            is_descendant_role(&contract.task_role)
                && contract.root_task_id.as_deref() == Some(migration.root_task_id.as_str()),
            "监督子任务没有绑定同一 root_task_id"
        );
        let ancestor_id = contract
            .parent_task_id
            .as_deref()
            .ok_or_else(|| anyhow!("监督子任务谱系提前中断"))?;
        anyhow::ensure!(
            ancestor_id != current.task_id,
            "监督任务不能把自身作为父任务"
        );
        current = local_tasks
            .get_for_owner(owner, ancestor_id)
            .with_context(|| format!("读取监督谱系父任务 {ancestor_id}"))?
            .ok_or_else(|| anyhow!("监督谱系父任务不存在或不属于同一 owner: {ancestor_id}"))?;
        contract = crate::node_agent_local_task_supervision::load_supervision_contract(
            task_journal,
            ancestor_id,
        )?
        .ok_or_else(|| anyhow!("监督谱系父任务缺少持久契约: {ancestor_id}"))?;
    }
    bail!("监督任务谱系超过最大深度 {MAX_LINEAGE_DEPTH}")
}

fn validate_task_identity(
    task: &LocalTaskRecord,
    owner: &str,
    agent: &str,
    install: &str,
    project: &str,
    workspace: &str,
) -> Result<()> {
    anyhow::ensure!(
        task.owner_user_id == owner && task.agent_id == agent && task.install_id == install,
        "监督任务谱系跨越 owner、节点或安装实例"
    );
    anyhow::ensure!(
        crate::node_agent_full_access::project_ids_equivalent(&task.project_id, project),
        "监督任务谱系跨项目"
    );
    anyhow::ensure!(
        same_path(Path::new(&task.workspace_path), Path::new(workspace)),
        "监督任务谱系的授权基础工作区漂移"
    );
    Ok(())
}

fn is_descendant_role(role: &str) -> bool {
    matches!(
        role,
        "capability_repair" | "resume_original" | "post_task_improvement"
    )
}

fn same_path(left: &Path, right: &Path) -> bool {
    let left = std::fs::canonicalize(left).unwrap_or_else(|_| left.to_path_buf());
    let right = std::fs::canonicalize(right).unwrap_or_else(|_| right.to_path_buf());
    if cfg!(windows) {
        left.to_string_lossy()
            .eq_ignore_ascii_case(&right.to_string_lossy())
    } else {
        left == right
    }
}

#[cfg(test)]
#[path = "node_agent_local_task_resume_lease_migration_tests.rs"]
mod tests;
