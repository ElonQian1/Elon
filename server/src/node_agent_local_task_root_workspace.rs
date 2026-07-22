//! Resolve chained supervision tasks back to the authoritative project root.

use std::{collections::HashSet, path::Path};

use anyhow::{bail, Context, Result};

use crate::{
    node_agent_local_task_store::{LocalTaskRecord, LocalTaskStore},
    node_agent_local_task_supervision::{SupervisionContract, SUPERVISION_PROTOCOL},
    node_agent_task_journal::TaskJournal,
};

pub(crate) struct ResolvedRequestWorkspace {
    pub(crate) request_path: String,
    pub(crate) grant_path: String,
}

pub(crate) fn resolve_request_workspace(
    runtime: &crate::NodeRuntime,
    creds: &crate::Credentials,
    project_id: &str,
    requested_workspace_path: &str,
    contract: Option<&SupervisionContract>,
) -> Result<ResolvedRequestWorkspace> {
    let Some(contract) = contract else {
        return Ok(same_request_and_grant(requested_workspace_path.to_string()));
    };
    match contract.task_role.as_str() {
        "resume_original" => {
            let parent_id = contract
                .parent_task_id
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .context("Resume 缺少 parent_task_id。")?;
            let parent = runtime
                .local_tasks
                .get_for_owner(&creds.owner_user_id, parent_id)?
                .context("Resume 的父任务不存在或不属于当前账号。")?;
            let grant_path = resolve_resume_authorized_workspace(
                &runtime.local_tasks,
                &runtime.task_journal,
                &runtime.update_recovery,
                creds,
                &runtime.install_id,
                project_id,
                &parent,
                contract,
            )?;
            Ok(ResolvedRequestWorkspace {
                request_path: requested_workspace_path.to_string(),
                grant_path,
            })
        }
        "capability_repair" | "post_task_improvement" => {
            let resolved = resolve_chained_authorized_workspace(
                &runtime.local_tasks,
                &runtime.task_journal,
                &runtime.update_recovery,
                creds,
                &runtime.install_id,
                project_id,
                requested_workspace_path,
                contract,
            )?;
            Ok(same_request_and_grant(resolved))
        }
        _ => Ok(same_request_and_grant(requested_workspace_path.to_string())),
    }
}

fn same_request_and_grant(path: String) -> ResolvedRequestWorkspace {
    ResolvedRequestWorkspace {
        request_path: path.clone(),
        grant_path: path,
    }
}

/// Resolve a resume parent through its persisted supervision ancestry.  Unlike
/// `record.workspace_path`, which is the active worktree for isolated tasks,
/// the returned path is anchored at the requirement root's durable base.
pub(crate) fn resolve_resume_authorized_workspace(
    tasks: &LocalTaskStore,
    journal: &TaskJournal,
    recovery: &crate::node_agent_update_recovery::UpdateRecoveryStore,
    creds: &crate::Credentials,
    install_id: &str,
    project_id: &str,
    parent: &LocalTaskRecord,
    contract: &SupervisionContract,
) -> Result<String> {
    anyhow::ensure!(
        contract.protocol == SUPERVISION_PROTOCOL && contract.task_role == "resume_original",
        "只有当前监督协议的 resume_original 可以解析根工作区。"
    );
    let parent_id = required_id(contract.parent_task_id.as_deref(), "parent_task_id")?;
    anyhow::ensure!(
        parent_id == parent.task_id,
        "resume parent identity mismatch"
    );
    let root_id = required_id(contract.root_task_id.as_deref(), "root_task_id")?;
    let immediate_parent_contract =
        crate::node_agent_local_task_supervision::load_supervision_contract(
            journal,
            &parent.task_id,
        )?
        .context("Resume 父任务缺少持久监督契约。")?;
    let mut current = parent.clone();
    let mut visited = HashSet::new();
    let mut recorded_bases = Vec::new();

    loop {
        validate_task_identity(&current, creds, install_id, project_id)?;
        anyhow::ensure!(
            visited.insert(current.task_id.clone()),
            "监督任务谱系包含循环"
        );
        let legacy_root_without_workspace_identity =
            current.task_id == root_id && current.workspace_status.is_none();
        if !legacy_root_without_workspace_identity {
            recorded_bases.push(recorded_base_workspace(&current)?);
        }
        let current_contract = crate::node_agent_local_task_supervision::load_supervision_contract(
            journal,
            &current.task_id,
        )?
        .context("监督任务谱系缺少持久契约。")?;
        anyhow::ensure!(
            current_contract.protocol == SUPERVISION_PROTOCOL
                && current_contract.supervisor == contract.supervisor,
            "监督任务谱系的协议或 supervisor 身份漂移"
        );
        let recorded_root = current_contract
            .root_task_id
            .as_deref()
            .unwrap_or(current.task_id.as_str());
        anyhow::ensure!(
            recorded_root == root_id,
            "resume root_task_id 与持久谱系不一致"
        );
        if current.task_id == root_id {
            anyhow::ensure!(
                current_contract.task_role == "requirement"
                    && current_contract.parent_task_id.is_none(),
                "监督谱系根任务不是可信的 requirement 根"
            );
            let root_base = if legacy_root_without_workspace_identity {
                super::resume_identity::validated_descendant_resume_base(
                    tasks,
                    parent,
                    &immediate_parent_contract,
                    root_id,
                    project_id,
                )?
            } else {
                recorded_base_workspace(&current)?
            };
            anyhow::ensure!(
                recorded_bases.iter().all(|base| {
                    crate::node_agent_update_checkpoint::same_path(
                        Path::new(base),
                        Path::new(&root_base),
                    )
                }),
                "监督任务谱系的基础工作区与持久 root 授权根不一致"
            );
            return Ok(root_base);
        }
        let receipt = recovery.receipt_for_task(&current.task_id)?;
        let ancestor_id = required_id(
            durable_parent_id(
                &current.task_id,
                root_id,
                &current_contract,
                receipt.as_ref(),
            ),
            "持久谱系 parent_task_id",
        )?;
        current = tasks
            .get_for_owner(&creds.owner_user_id, ancestor_id)?
            .context("监督任务谱系的祖先任务不存在。")?;
    }
}

pub(crate) fn resolve_chained_authorized_workspace(
    tasks: &LocalTaskStore,
    journal: &TaskJournal,
    recovery: &crate::node_agent_update_recovery::UpdateRecoveryStore,
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
        let recovery_receipt = recovery.receipt_for_task(&current.task_id)?;
        let durable_parent = durable_parent_id(
            &current.task_id,
            root_id,
            &current_contract,
            recovery_receipt.as_ref(),
        );
        let ancestor_id = required_id(durable_parent, "持久谱系 parent_task_id")?;
        current = tasks
            .get_for_owner(&creds.owner_user_id, ancestor_id)?
            .context("监督任务谱系的祖先任务不存在。")?;
    }

    let base = recorded_base_workspace(&current)?;
    validate_requested_workspace(requested_workspace_path, &immediate_parent, &base)?;
    Ok(base)
}

fn durable_parent_id<'a>(
    task_id: &str,
    root_id: &str,
    contract: &'a SupervisionContract,
    recovery: Option<&'a crate::node_agent_update_recovery::UpdateRecoveryReceipt>,
) -> Option<&'a str> {
    contract.parent_task_id.as_deref().or_else(|| {
        recovery.and_then(|receipt| {
            if receipt.root_task_id != root_id {
                return None;
            }
            if receipt.resume_task_id.as_deref() == Some(task_id) {
                Some(receipt.original_task_id.as_str())
            } else {
                receipt.parent_task_id.as_deref()
            }
        })
    })
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
    use crate::node_agent_local_task_store::{LocalTaskStart, LocalTaskStore};

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

    #[test]
    fn resume_request_resolves_inherited_worktree_to_authoritative_base_before_grant_check() {
        let root = std::env::temp_dir().join(format!(
            "elon-resume-request-root-{}",
            uuid::Uuid::new_v4().simple()
        ));
        let base = root.join("base");
        let active = root.join("conversation-worktrees/elon-self/conversation");
        std::fs::create_dir_all(&base).unwrap();
        std::fs::create_dir_all(&active).unwrap();
        let credentials = crate::node_agent_config::Credentials {
            agent_id: "agent".into(),
            agent_secret: "unused".into(),
            owner_user_id: "owner".into(),
            user_token: None,
        };
        let mut runtime = crate::NodeRuntime::new(
            crate::node_agent_config::NodeConfig {
                cloud_url: "ws://127.0.0.1".into(),
                cloud_http_url: "http://127.0.0.1".into(),
                ollama_url: "http://127.0.0.1".into(),
                lm_studio_url: None,
                custom_url: None,
                price_per_1k: 0.0,
            },
            Some(credentials.clone()),
            crate::pc_storage_repo::StorageSettings::default(),
            crate::node_agent_data_root::resolve(None, None, None),
            "install".into(),
        );
        runtime.local_tasks = LocalTaskStore::new(root.join("tasks.sqlite3"));
        runtime.task_journal = TaskJournal::new(root.join("journal"));
        runtime.update_recovery =
            crate::node_agent_update_recovery::UpdateRecoveryStore::new(root.join("recovery.json"));
        runtime
            .local_tasks
            .create(LocalTaskStart {
                task_id: "local-parent",
                owner_user_id: "owner",
                agent_id: "agent",
                install_id: "install",
                project_id: "elon-self",
                channel_id: None,
                conversation_id: "conversation",
                workspace_path: active.to_str().unwrap(),
                prompt: "work",
                cli: "codex",
                runtime_permission: "full_access",
            })
            .unwrap();
        runtime
            .local_tasks
            .record_initial_workspace_status(
                "local-parent",
                &serde_json::json!({
                    "isolated": true,
                    "platform_provenance": "elon.conversation_worktree.v1",
                    "project_id": "elon-self",
                    "root_task_id": "local-parent",
                    "base_workspace_path": base,
                    "active_workspace_path": active,
                    "branch": "ai/session/elon-self/conversation",
                }),
            )
            .unwrap();
        crate::node_agent_local_task_supervision::record_supervision_event(
            &runtime.task_journal,
            "local-parent",
            "supervision_contract",
            crate::node_agent_local_task_supervision::contract_payload(&SupervisionContract {
                protocol: SUPERVISION_PROTOCOL.into(),
                supervisor: "codex_desktop".into(),
                task_role: "requirement".into(),
                parent_task_id: None,
                root_task_id: Some("local-parent".into()),
                acceptance_criteria: vec![],
                improvement_policy: "observe_only".into(),
            }),
        )
        .unwrap();
        let resume = SupervisionContract {
            protocol: SUPERVISION_PROTOCOL.into(),
            supervisor: "codex_desktop".into(),
            task_role: "resume_original".into(),
            parent_task_id: Some("local-parent".into()),
            root_task_id: Some("local-parent".into()),
            acceptance_criteria: vec![],
            improvement_policy: "observe_only".into(),
        };

        let resolved = resolve_request_workspace(
            &runtime,
            &credentials,
            "elon-self",
            active.to_str().unwrap(),
            Some(&resume),
        )
        .unwrap();
        assert_eq!(resolved.request_path, active.to_string_lossy());
        assert!(crate::node_agent_update_checkpoint::same_path(
            Path::new(&resolved.grant_path),
            &base
        ));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn legacy_recovery_child_uses_durable_receipt_lineage() {
        let contract = SupervisionContract {
            protocol: SUPERVISION_PROTOCOL.to_string(),
            supervisor: "codex_desktop".into(),
            task_role: "resume_original".into(),
            parent_task_id: None,
            root_task_id: Some("local-root".into()),
            acceptance_criteria: Vec::new(),
            improvement_policy: "after_task_or_unblock".into(),
        };
        let mut receipt = crate::node_agent_update_recovery::UpdateRecoveryReceipt::planned(
            "node-update",
            "local-root",
            "local-parent",
        );
        receipt.resume_task_id = Some("local-recovery".into());
        assert_eq!(
            durable_parent_id("local-recovery", "local-root", &contract, Some(&receipt)),
            Some("local-parent")
        );
        assert_eq!(
            durable_parent_id("local-recovery", "another-root", &contract, Some(&receipt)),
            None
        );
    }
}
