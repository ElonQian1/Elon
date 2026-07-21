//! Shared production provision -> journal -> local-record transaction boundary.

use anyhow::{Context, Result};
use homecli_proto::CliProjectContext;

use crate::{
    node_agent_local_task_store::{LocalTaskRecord, LocalTaskStart, LocalTaskStore},
    node_agent_local_task_supervision::{
        contract_payload, record_supervision_event, SupervisionContract,
    },
    node_agent_task_journal::TaskJournal,
    pc_workspace_provisioner::ConversationWorkspaceResult,
};

pub(crate) struct SupervisedLocalTaskProvision<'a> {
    pub task_id: &'a str,
    pub owner_user_id: &'a str,
    pub agent_id: &'a str,
    pub install_id: &'a str,
    pub project_id: &'a str,
    pub channel_id: Option<&'a str>,
    pub conversation_id: &'a str,
    pub base_workspace_path: &'a str,
    pub prompt: &'a str,
    pub runtime_permission: &'a str,
    pub root_task_id: &'a str,
    pub contract: &'a SupervisionContract,
}

pub(crate) fn provision_and_record_supervised_task(
    data_paths: Option<&elon_pc_dev_runtime::NodeDataPaths>,
    local_tasks: &LocalTaskStore,
    journal: &TaskJournal,
    request: SupervisedLocalTaskProvision<'_>,
) -> Result<(LocalTaskRecord, ConversationWorkspaceResult)> {
    let prepared = crate::node_agent_cli_runner::prepare_cli_prompt_cwd_in_with_supervision(
        data_paths,
        Some(request.base_workspace_path.to_string()),
        Some(CliProjectContext {
            project_id: request.project_id.to_string(),
            conversation_id: request.conversation_id.to_string(),
            runtime_permission: Some(request.runtime_permission.to_string()),
        }),
        Some(request.root_task_id),
    )?;
    let workspace = prepared
        .conversation_workspace
        .context("supervised task did not receive an isolated worktree")?;
    let record = record_provisioned_supervised_task(local_tasks, journal, &workspace, &request)?;
    Ok((record, workspace))
}

pub(crate) fn provision_record_and_dispatch_supervised_task(
    data_paths: Option<&elon_pc_dev_runtime::NodeDataPaths>,
    local_tasks: &LocalTaskStore,
    journal: &TaskJournal,
    request: SupervisedLocalTaskProvision<'_>,
    dispatch: impl FnOnce(&LocalTaskRecord, &ConversationWorkspaceResult) -> Result<()>,
) -> Result<(LocalTaskRecord, ConversationWorkspaceResult)> {
    let (record, workspace) =
        provision_and_record_supervised_task(data_paths, local_tasks, journal, request)?;
    dispatch(&record, &workspace)?;
    Ok((record, workspace))
}

pub(super) fn record_provisioned_supervised_task(
    local_tasks: &LocalTaskStore,
    journal: &TaskJournal,
    workspace: &ConversationWorkspaceResult,
    request: &SupervisedLocalTaskProvision<'_>,
) -> Result<LocalTaskRecord> {
    record_supervision_event(
        journal,
        request.task_id,
        "supervision_contract",
        contract_payload(request.contract),
    )?;
    record_supervised_task_workspace(
        local_tasks,
        workspace,
        request,
        "provisioned_supervised_worktree",
    )
}

pub(super) fn record_resumed_supervised_task(
    local_tasks: &LocalTaskStore,
    record: &LocalTaskRecord,
    workspace: &ConversationWorkspaceResult,
    base_workspace_path: &str,
    root_task_id: &str,
) -> Result<LocalTaskRecord> {
    persist_supervised_task_workspace(
        local_tasks,
        record,
        workspace,
        base_workspace_path,
        &record.project_id,
        root_task_id,
        "inherited_supervised_worktree",
    )
}

pub(super) fn record_local_or_resumed_task(
    local_tasks: &LocalTaskStore,
    start: LocalTaskStart<'_>,
    resume: Option<&crate::node_agent_local_task_resume::ResolvedResumeWorkspace>,
    contract: Option<&SupervisionContract>,
) -> Result<LocalTaskRecord> {
    let record = local_tasks.create(start)?;
    let (Some(resume), Some(contract)) = (resume, contract) else {
        return Ok(record);
    };
    let root_task_id = contract
        .root_task_id
        .as_deref()
        .context("resume task is missing root_task_id")?;
    record_resumed_supervised_task(
        local_tasks,
        &record,
        &resume.inherited_workspace,
        &resume.authorized_workspace_path,
        root_task_id,
    )
}

fn record_supervised_task_workspace(
    local_tasks: &LocalTaskStore,
    workspace: &ConversationWorkspaceResult,
    request: &SupervisedLocalTaskProvision<'_>,
    prepare_status: &str,
) -> Result<LocalTaskRecord> {
    anyhow::ensure!(
        workspace.isolated,
        "supervised task worktree is not isolated"
    );
    anyhow::ensure!(
        !crate::node_agent_update_checkpoint::same_path(
            std::path::Path::new(request.base_workspace_path),
            std::path::Path::new(&workspace.workspace_path),
        ),
        "supervised task execution path equals its base workspace"
    );
    let record = local_tasks.create(LocalTaskStart {
        task_id: request.task_id,
        owner_user_id: request.owner_user_id,
        agent_id: request.agent_id,
        install_id: request.install_id,
        project_id: request.project_id,
        channel_id: request.channel_id,
        conversation_id: request.conversation_id,
        workspace_path: &workspace.workspace_path,
        prompt: request.prompt,
        cli: "codex",
        runtime_permission: request.runtime_permission,
    })?;
    persist_supervised_task_workspace(
        local_tasks,
        &record,
        workspace,
        request.base_workspace_path,
        request.project_id,
        request.root_task_id,
        prepare_status,
    )
}

fn persist_supervised_task_workspace(
    local_tasks: &LocalTaskStore,
    record: &LocalTaskRecord,
    workspace: &ConversationWorkspaceResult,
    base_workspace_path: &str,
    project_id: &str,
    root_task_id: &str,
    prepare_status: &str,
) -> Result<LocalTaskRecord> {
    anyhow::ensure!(
        workspace.isolated,
        "supervised task worktree is not isolated"
    );
    anyhow::ensure!(
        crate::node_agent_update_checkpoint::same_path(
            std::path::Path::new(&record.workspace_path),
            std::path::Path::new(&workspace.workspace_path),
        ),
        "local task record did not preserve the supervised execution worktree"
    );
    if let Some(recorded_base) = workspace.base_workspace_path.as_deref() {
        anyhow::ensure!(
            crate::node_agent_update_checkpoint::same_path(
                std::path::Path::new(base_workspace_path),
                std::path::Path::new(recorded_base),
            ),
            "supervised task workspace base identity drifted"
        );
    }
    anyhow::ensure!(
        workspace.supervision_root_task_id.as_deref() == Some(root_task_id),
        "supervised task workspace root lease identity drifted"
    );
    let status = serde_json::json!({
        "platform_provenance": "elon.conversation_worktree.v1",
        "project_id": project_id,
        "root_task_id": root_task_id,
        "base_workspace_path": workspace.base_workspace_path,
        "active_workspace_path": workspace.workspace_path,
        "isolated": workspace.isolated,
        "branch": workspace.branch,
        "git_head": crate::node_agent_update_checkpoint::git_output(
            std::path::Path::new(&workspace.workspace_path),
            &["rev-parse", "--verify", "HEAD^{commit}"],
        ),
        "base_revision": crate::node_agent_update_checkpoint::git_output(
            std::path::Path::new(base_workspace_path),
            &["rev-parse", "--verify", "HEAD^{commit}"],
        ),
        "git_common_dir": crate::node_agent_update_checkpoint::git_output(
            std::path::Path::new(&workspace.workspace_path),
            &["rev-parse", "--path-format=absolute", "--git-common-dir"],
        ),
        "git_remote": crate::node_agent_update_checkpoint::git_output(
            std::path::Path::new(base_workspace_path),
            &["config", "--get", "remote.origin.url"],
        ),
        "prepare_status": prepare_status,
        "merge_status": "preserved",
    });
    anyhow::ensure!(
        local_tasks.record_initial_workspace_status(&record.task_id, &status)?,
        "local task record did not preserve initial isolated workspace identity"
    );
    local_tasks
        .get(&record.task_id)?
        .context("local task missing after workspace identity persistence")
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use uuid::Uuid;

    use super::*;

    #[test]
    fn resumed_task_persists_the_inherited_provenance_git_identity_and_root_lease() {
        let root =
            std::env::temp_dir().join(format!("resume-provision-{}", Uuid::new_v4().simple()));
        let base = root.join("repo");
        let active = root.join("conversation-worktrees/project/resume");
        std::fs::create_dir_all(&base).unwrap();
        git(&base, &["init"]);
        git(&base, &["config", "user.email", "tests@example.invalid"]);
        git(&base, &["config", "user.name", "Tests"]);
        git(
            &base,
            &[
                "config",
                "remote.origin.url",
                "https://example.invalid/repo.git",
            ],
        );
        std::fs::write(base.join("seed.txt"), "seed\n").unwrap();
        git(&base, &["add", "seed.txt"]);
        git(&base, &["commit", "-m", "seed"]);
        let branch = "ai/session/project/resume";
        git(
            &base,
            &[
                "worktree",
                "add",
                "-b",
                branch,
                active.to_str().unwrap(),
                "HEAD",
            ],
        );
        crate::node_agent_supervision_worktree_lease::acquire(&base, &active, "local-root")
            .unwrap();

        let store = LocalTaskStore::new(root.join("tasks.sqlite3"));
        let contract = SupervisionContract {
            protocol: crate::node_agent_local_task_supervision::SUPERVISION_PROTOCOL.into(),
            supervisor: "codex_desktop".into(),
            task_role: "resume_original".into(),
            parent_task_id: Some("local-parent".into()),
            root_task_id: Some("local-root".into()),
            acceptance_criteria: vec!["resume".into()],
            improvement_policy: "after_task_only".into(),
        };
        let workspace = ConversationWorkspaceResult {
            base_workspace_path: Some(base.to_string_lossy().into_owned()),
            workspace_path: active.to_string_lossy().into_owned(),
            isolated: true,
            branch: Some(branch.into()),
            supervision_root_task_id: Some("local-root".into()),
        };
        let request = SupervisedLocalTaskProvision {
            task_id: "local-child",
            owner_user_id: "owner",
            agent_id: "agent",
            install_id: "install",
            project_id: "project",
            channel_id: None,
            conversation_id: "resume",
            base_workspace_path: base.to_str().unwrap(),
            prompt: "resume",
            runtime_permission: "full_access",
            root_task_id: "local-root",
            contract: &contract,
        };
        let record = store
            .create(LocalTaskStart {
                task_id: request.task_id,
                owner_user_id: request.owner_user_id,
                agent_id: request.agent_id,
                install_id: request.install_id,
                project_id: request.project_id,
                channel_id: request.channel_id,
                conversation_id: request.conversation_id,
                workspace_path: &workspace.workspace_path,
                prompt: request.prompt,
                cli: "codex",
                runtime_permission: request.runtime_permission,
            })
            .unwrap();
        let record = record_resumed_supervised_task(
            &store,
            &record,
            &workspace,
            request.base_workspace_path,
            request.root_task_id,
        )
        .unwrap();
        let status = record.workspace_status.unwrap();
        assert!(crate::node_agent_update_checkpoint::same_path(
            Path::new(&record.workspace_path),
            &active
        ));
        assert_eq!(
            status["platform_provenance"],
            "elon.conversation_worktree.v1"
        );
        assert_eq!(status["root_task_id"], "local-root");
        assert_eq!(status["branch"], branch);
        assert_eq!(status["prepare_status"], "inherited_supervised_worktree");
        let git_head = crate::node_agent_update_checkpoint::git_output(
            &active,
            &["rev-parse", "--verify", "HEAD^{commit}"],
        )
        .unwrap();
        assert_eq!(status["git_head"].as_str(), Some(git_head.as_str()));
        assert_eq!(
            crate::node_agent_supervision_worktree_lease::worktree_lock_reason(&base, &active)
                .unwrap()
                .as_deref(),
            Some("elon-supervision:local-root")
        );

        crate::node_agent_supervision_worktree_lease::release(&base, &active, "local-root")
            .unwrap();
        let _ = std::fs::remove_dir_all(root);
    }

    fn git(cwd: &Path, args: &[&str]) {
        let output = crate::git_command_error::git_command()
            .args(args)
            .current_dir(cwd)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
