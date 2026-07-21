//! Resolve a UI worktree's durable supervision identity without trusting the
//! transient Git worktree `locked` file.

use std::path::Path;

use anyhow::{bail, Context, Result};

use crate::{
    node_agent_cli_sidecar::CliSidecarRegistry,
    node_agent_local_task_resume_lineage::{validate_full_lineage, LegacyLeaseMigration},
    node_agent_local_task_store::{LocalTaskRecord, LocalTaskStore},
    node_agent_local_task_supervision::{load_supervision_contract, SUPERVISION_PROTOCOL},
    node_agent_task_journal::TaskJournal,
};

pub(crate) fn resolve_root_task_id(project_root: &Path) -> Result<String> {
    resolve_with(
        project_root,
        &LocalTaskStore::default(),
        &TaskJournal::default(),
        &CliSidecarRegistry::default(),
    )
}

pub(crate) fn validate_task_root(
    project_root: &Path,
    task_id: &str,
    expected_root: &str,
) -> Result<()> {
    let tasks = LocalTaskStore::default();
    let journal = TaskJournal::default();
    validate_task_root_with(project_root, task_id, expected_root, &tasks, &journal)
}

fn validate_task_root_with(
    project_root: &Path,
    task_id: &str,
    expected_root: &str,
    tasks: &LocalTaskStore,
    journal: &TaskJournal,
) -> Result<()> {
    let task = tasks
        .get(task_id)?
        .ok_or_else(|| anyhow::anyhow!("RUNTIME_BINDING_LEGACY_TASK_MISSING: task={task_id}"))?;
    anyhow::ensure!(
        same_path(Path::new(&task.workspace_path), project_root),
        "RUNTIME_BINDING_LEGACY_TASK_STALE: task={task_id} project root 不一致"
    );
    let root = validated_root(&tasks, &journal, &task)?;
    anyhow::ensure!(
        root == expected_root,
        "RUNTIME_BINDING_LEGACY_TASK_STALE: task={task_id} root={root} expected={expected_root}"
    );
    Ok(())
}

fn resolve_with(
    project_root: &Path,
    tasks: &LocalTaskStore,
    journal: &TaskJournal,
    sidecars: &CliSidecarRegistry,
) -> Result<String> {
    let canonical = project_root.canonicalize().with_context(|| {
        format!(
            "RUNTIME_BINDING_MISSING_ROOT: 项目目录不存在: {}",
            project_root.display()
        )
    })?;
    let records = tasks
        .list_identity_candidates()
        .context("RUNTIME_BINDING_ROOT_STORE_INVALID: 读取持久 task 元数据失败")?;
    let sidecar_task_ids = sidecars
        .all_sessions()
        .context("RUNTIME_BINDING_ROOT_SIDECAR_INVALID: 读取 sidecar 元数据失败")?
        .into_iter()
        .filter(|item| {
            item.cwd
                .as_deref()
                .is_some_and(|cwd| same_path(Path::new(cwd), &canonical))
        })
        .map(|item| item.task_id)
        .collect::<std::collections::HashSet<_>>();

    let mut roots = std::collections::BTreeSet::new();
    let mut candidates = Vec::new();
    for task in records
        .into_iter()
        .filter(|task| same_path(Path::new(&task.workspace_path), &canonical))
    {
        let root = match validated_root(tasks, journal, &task) {
            Ok(root) => root,
            Err(error) => {
                candidates.push(format!("{}:invalid:{error:#}", task.task_id));
                continue;
            }
        };
        candidates.push(format!(
            "{}->{}:{}:{}",
            task.task_id,
            root,
            task.status,
            if sidecar_task_ids.contains(&task.task_id) {
                "sidecar"
            } else {
                "task"
            }
        ));
        roots.insert(root);
    }

    match roots.len() {
        1 => Ok(roots.into_iter().next().expect("one root")),
        0 => match crate::node_agent_codex_task_contract_identity::resolve(&canonical)? {
            Some(root) => Ok(root),
            None => bail!(
                "RUNTIME_BINDING_MISSING_ROOT: 持久 task/lineage 元数据没有可信 root identity；候选={}",
                candidates.join(",")
            ),
        },
        count => bail!(
            "RUNTIME_BINDING_AMBIGUOUS_ROOT: 持久监督元数据包含 {count} 个 root identity；候选={}",
            candidates.join(",")
        ),
    }
}

fn validated_root(
    tasks: &LocalTaskStore,
    journal: &TaskJournal,
    task: &LocalTaskRecord,
) -> Result<String> {
    let contract = load_supervision_contract(journal, &task.task_id)?
        .ok_or_else(|| anyhow::anyhow!("missing_contract"))?;
    anyhow::ensure!(contract.protocol == SUPERVISION_PROTOCOL, "wrong_protocol");
    if contract.task_role == "requirement" {
        anyhow::ensure!(
            contract.parent_task_id.is_none()
                && contract
                    .root_task_id
                    .as_deref()
                    .is_none_or(|id| id == task.task_id),
            "invalid_requirement_root"
        );
        return Ok(task.task_id.clone());
    }
    let root = contract
        .root_task_id
        .as_deref()
        .filter(|id| !id.trim().is_empty())
        .ok_or_else(|| anyhow::anyhow!("missing_root"))?;
    validate_full_lineage(
        tasks,
        journal,
        task,
        &contract,
        &LegacyLeaseMigration {
            legacy_task_id: task.task_id.clone(),
            root_task_id: root.to_string(),
        },
    )?;
    Ok(root.to_string())
}

fn same_path(left: &Path, right: &Path) -> bool {
    let left = left.canonicalize().unwrap_or_else(|_| left.to_path_buf());
    let right = right.canonicalize().unwrap_or_else(|_| right.to_path_buf());
    if cfg!(windows) {
        left.to_string_lossy()
            .eq_ignore_ascii_case(&right.to_string_lossy())
    } else {
        left == right
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        node_agent_local_task_store::LocalTaskStart,
        node_agent_local_task_supervision::{
            contract_payload, record_supervision_event, SupervisionContract,
        },
    };

    fn add_task(
        store: &LocalTaskStore,
        journal: &TaskJournal,
        root: &Path,
        id: &str,
        role: &str,
        parent: Option<&str>,
        root_id: Option<&str>,
    ) {
        store
            .create(LocalTaskStart {
                task_id: id,
                owner_user_id: "owner",
                agent_id: "node",
                install_id: "install",
                project_id: "project",
                channel_id: None,
                conversation_id: id,
                workspace_path: &root.to_string_lossy(),
                prompt: "test",
                cli: "codex",
                runtime_permission: "full_access",
            })
            .unwrap();
        let contract = SupervisionContract {
            protocol: SUPERVISION_PROTOCOL.into(),
            supervisor: "codex_desktop".into(),
            task_role: role.into(),
            parent_task_id: parent.map(str::to_string),
            root_task_id: root_id.map(str::to_string),
            acceptance_criteria: vec!["test".into()],
            improvement_policy: "after_task_only".into(),
        };
        record_supervision_event(
            journal,
            id,
            "supervision_contract",
            contract_payload(&contract),
        )
        .unwrap();
    }

    #[test]
    fn restores_root_from_persisted_descendant_lineage_after_git_lock_is_gone() {
        let temp =
            std::env::temp_dir().join(format!("root-identity-{}", uuid::Uuid::new_v4().simple()));
        let workspace = temp.join("workspace");
        std::fs::create_dir_all(&workspace).unwrap();
        let tasks = LocalTaskStore::new(temp.join("tasks.sqlite3"));
        let journal = TaskJournal::new(temp.join("journal"));
        let sidecars = CliSidecarRegistry::new(temp.join("sidecars"));
        add_task(
            &tasks,
            &journal,
            &workspace,
            "root",
            "requirement",
            None,
            None,
        );
        add_task(
            &tasks,
            &journal,
            &workspace,
            "repair",
            "capability_repair",
            Some("root"),
            Some("root"),
        );
        assert_eq!(
            resolve_with(&workspace, &tasks, &journal, &sidecars).unwrap(),
            "root"
        );
        std::fs::remove_dir_all(temp).unwrap();
    }

    #[test]
    fn fails_closed_for_missing_and_conflicting_roots_with_candidates() {
        let temp =
            std::env::temp_dir().join(format!("root-identity-{}", uuid::Uuid::new_v4().simple()));
        let workspace = temp.join("workspace");
        std::fs::create_dir_all(&workspace).unwrap();
        let tasks = LocalTaskStore::new(temp.join("tasks.sqlite3"));
        let journal = TaskJournal::new(temp.join("journal"));
        let sidecars = CliSidecarRegistry::new(temp.join("sidecars"));
        assert!(resolve_with(&workspace, &tasks, &journal, &sidecars)
            .unwrap_err()
            .to_string()
            .contains("RUNTIME_BINDING_MISSING_ROOT"));
        add_task(
            &tasks,
            &journal,
            &workspace,
            "root-a",
            "requirement",
            None,
            None,
        );
        add_task(
            &tasks,
            &journal,
            &workspace,
            "root-b",
            "requirement",
            None,
            None,
        );
        let error = resolve_with(&workspace, &tasks, &journal, &sidecars)
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("RUNTIME_BINDING_AMBIGUOUS_ROOT")
                && error.contains("root-a")
                && error.contains("root-b")
        );
        std::fs::remove_dir_all(temp).unwrap();
    }

    #[test]
    fn legacy_fit_run_task_must_preserve_project_and_root_identity() {
        let temp =
            std::env::temp_dir().join(format!("root-identity-{}", uuid::Uuid::new_v4().simple()));
        let workspace = temp.join("workspace");
        let other = temp.join("other");
        std::fs::create_dir_all(&workspace).unwrap();
        std::fs::create_dir_all(&other).unwrap();
        let tasks = LocalTaskStore::new(temp.join("tasks.sqlite3"));
        let journal = TaskJournal::new(temp.join("journal"));
        add_task(
            &tasks,
            &journal,
            &workspace,
            "root",
            "requirement",
            None,
            None,
        );
        add_task(
            &tasks,
            &journal,
            &workspace,
            "fit-task",
            "resume_original",
            Some("root"),
            Some("root"),
        );
        validate_task_root_with(&workspace, "fit-task", "root", &tasks, &journal).unwrap();
        assert!(
            validate_task_root_with(&other, "fit-task", "root", &tasks, &journal)
                .unwrap_err()
                .to_string()
                .contains("project root")
        );
        assert!(
            validate_task_root_with(&workspace, "fit-task", "other-root", &tasks, &journal)
                .unwrap_err()
                .to_string()
                .contains("expected=other-root")
        );
        std::fs::remove_dir_all(temp).unwrap();
    }
}
