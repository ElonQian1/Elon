//! Durable task/lease admission for a controlled orphaned-worktree migration.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Context, Result};

use crate::{
    node_agent_local_task_resume::OrphanedWorkspaceMigration,
    node_agent_local_task_store::{LocalTaskRecord, LocalTaskStore},
    node_agent_local_task_supervision::{SupervisionContract, SUPERVISION_PROTOCOL},
    node_agent_task_journal::TaskJournal,
};

#[derive(Clone, Debug)]
pub(crate) struct ReclaimableLease {
    pub(crate) path: PathBuf,
    pub(crate) lease_task_id: String,
}

#[derive(Clone, Debug)]
pub(crate) struct OrphanMigrationOccupancy {
    pub(crate) task_paths: Vec<PathBuf>,
    pub(crate) reclaimable_leases: Vec<ReclaimableLease>,
}

pub(crate) fn validate_occupancy(
    tasks: &LocalTaskStore,
    journal: &TaskJournal,
    parent: &LocalTaskRecord,
    parent_contract: &SupervisionContract,
    migration: &OrphanedWorkspaceMigration,
    authorized_base: &Path,
) -> Result<OrphanMigrationOccupancy> {
    anyhow::ensure!(
        parent_contract.protocol == SUPERVISION_PROTOCOL,
        "孤儿迁移父任务监督协议漂移"
    );
    let root_id = parent_contract
        .root_task_id
        .as_deref()
        .unwrap_or(parent.task_id.as_str());
    anyhow::ensure!(
        parent.task_id == root_id && parent_contract.task_role == "requirement",
        "孤儿 workspace provenance 迁移只允许权威 requirement 根任务"
    );
    let mut lineage = HashMap::new();
    for task in tasks.list_identity_candidates()? {
        let Some(contract) = crate::node_agent_local_task_supervision::load_supervision_contract(
            journal,
            &task.task_id,
        )?
        else {
            continue;
        };
        let task_root = contract
            .root_task_id
            .as_deref()
            .unwrap_or(task.task_id.as_str());
        if task_root != root_id {
            continue;
        }
        validate_task_identity(&task, parent, authorized_base, root_id)?;
        anyhow::ensure!(
            is_terminal(&task),
            "同根任务 {} 仍为非终态 {}，禁止迁移",
            task.task_id,
            task.status
        );
        lineage.insert(task.task_id.clone(), task);
    }
    anyhow::ensure!(
        lineage.contains_key(root_id),
        "本机 registry 缺少同根 requirement 任务"
    );
    anyhow::ensure!(
        lineage.contains_key(&parent.task_id),
        "本机 registry 缺少当前 Resume 父任务"
    );

    let mut task_paths = lineage
        .values()
        .map(recorded_active_path)
        .collect::<Result<Vec<_>>>()?;
    task_paths.sort();
    task_paths.dedup();
    anyhow::ensure!(
        task_paths
            .iter()
            .any(|path| same_path(path, Path::new(&migration.source_path))),
        "孤儿来源路径未关联到同根持久任务"
    );

    let mut reclaimable = Vec::new();
    for lock in crate::node_agent_supervision_worktree_lease::list_worktree_locks(authorized_base)?
    {
        let associated = lineage
            .values()
            .any(|task| recorded_active_path(task).is_ok_and(|path| same_path(&path, &lock.path)));
        let lease_task_id =
            crate::node_agent_supervision_worktree_lease::supervision_lease_task_id(&lock.reason);
        if !associated {
            if lease_task_id == Some(root_id)
                || lease_task_id.is_some_and(|id| lineage.contains_key(id))
            {
                anyhow::bail!(
                    "同根 lease {} 没有匹配的 registry workspace provenance",
                    lock.reason
                );
            }
            continue;
        }
        let lease_task_id = lease_task_id
            .ok_or_else(|| anyhow!("同根历史 worktree 使用未知或通用 lease：{}", lock.reason))?;
        anyhow::ensure!(
            lease_task_id == root_id || lineage.contains_key(lease_task_id),
            "同根历史 worktree lease 无法关联到持久任务：{}",
            lock.reason
        );
        if lease_task_id != root_id {
            let lease_task = lineage.get(lease_task_id).context("历史 lease 任务缺失")?;
            anyhow::ensure!(
                same_path(&recorded_active_path(lease_task)?, &lock.path),
                "历史 lease 路径与任务 provenance 不一致"
            );
        }
        reclaimable.push(ReclaimableLease {
            path: lock.path,
            lease_task_id: lease_task_id.to_string(),
        });
    }
    Ok(OrphanMigrationOccupancy {
        task_paths,
        reclaimable_leases: reclaimable,
    })
}

pub(crate) fn reclaim_terminal_leases(
    base: &Path,
    new_workspace: &Path,
    occupancy: &OrphanMigrationOccupancy,
) -> Result<()> {
    let mut released: Vec<ReclaimableLease> = Vec::new();
    for lease in &occupancy.reclaimable_leases {
        if same_path(&lease.path, new_workspace) {
            continue;
        }
        if let Err(error) = crate::node_agent_supervision_worktree_lease::release(
            base,
            &lease.path,
            &lease.lease_task_id,
        ) {
            for prior in released.iter().rev() {
                let _ = crate::node_agent_supervision_worktree_lease::acquire(
                    base,
                    &prior.path,
                    &prior.lease_task_id,
                );
            }
            return Err(error).context("受控回收同根历史终态 lease 失败");
        }
        released.push(lease.clone());
    }
    Ok(())
}

fn validate_task_identity(
    task: &LocalTaskRecord,
    parent: &LocalTaskRecord,
    authorized_base: &Path,
    root_id: &str,
) -> Result<()> {
    anyhow::ensure!(
        task.owner_user_id == parent.owner_user_id
            && task.agent_id == parent.agent_id
            && task.install_id == parent.install_id,
        "同根任务 {} 的 owner/node/install 身份漂移",
        task.task_id
    );
    anyhow::ensure!(
        crate::node_agent_full_access::project_ids_equivalent(&task.project_id, &parent.project_id),
        "同根任务 {} 的 project 身份漂移",
        task.task_id
    );
    let status = task
        .workspace_status
        .as_ref()
        .context("同根任务缺少 workspace provenance")?;
    if let Some(provenance) = status
        .get("platform_provenance")
        .and_then(serde_json::Value::as_str)
    {
        anyhow::ensure!(
            provenance == "elon.conversation_worktree.v1",
            "同根任务平台 provenance 漂移"
        );
    }
    if let Some(recorded_root) = status
        .get("root_task_id")
        .and_then(serde_json::Value::as_str)
    {
        anyhow::ensure!(recorded_root == root_id, "同根任务 workspace root 身份漂移");
    }
    if let Some(recorded_project) = status.get("project_id").and_then(serde_json::Value::as_str) {
        anyhow::ensure!(
            crate::node_agent_full_access::project_ids_equivalent(
                recorded_project,
                &task.project_id
            ),
            "同根任务 workspace project 身份漂移"
        );
    }
    let base = status
        .get("base_workspace_path")
        .and_then(serde_json::Value::as_str)
        .context("同根任务缺少 base_workspace_path")?;
    anyhow::ensure!(
        same_path(Path::new(base), authorized_base),
        "同根任务授权 base repo 漂移"
    );
    Ok(())
}

fn recorded_active_path(task: &LocalTaskRecord) -> Result<PathBuf> {
    let path = task
        .workspace_status
        .as_ref()
        .and_then(|status| status.get("active_workspace_path"))
        .and_then(serde_json::Value::as_str)
        .context("同根任务缺少 active_workspace_path")?;
    let path = PathBuf::from(path);
    anyhow::ensure!(path.is_absolute(), "同根任务 active workspace 不是绝对路径");
    anyhow::ensure!(
        same_path(Path::new(&task.workspace_path), &path),
        "同根任务 record/workspace provenance 路径漂移"
    );
    Ok(std::fs::canonicalize(&path).unwrap_or(path))
}

fn is_terminal(task: &LocalTaskRecord) -> bool {
    task.finished_at_ms.is_some()
        && matches!(
            task.status.as_str(),
            "done" | "failed" | "canceled" | "interrupted" | "resume_required"
        )
}

fn same_path(left: &Path, right: &Path) -> bool {
    crate::node_agent_update_checkpoint::same_path(left, right)
}

#[cfg(test)]
#[path = "node_agent_local_task_orphan_migration_tests.rs"]
mod tests;
