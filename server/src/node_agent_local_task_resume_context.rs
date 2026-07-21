//! Authoritative, bounded context for Desktop-supervised resume tasks.
//!
//! The helper sends only task references. The node resolves the root request,
//! validates the durable lineage and Git identity, and compiles exactly one
//! executor prompt. This prevents recursive parent-prompt growth.

use anyhow::{anyhow, bail, Context, Result};
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::{
    node_agent_local_task_resume::ResolvedResumeWorkspace,
    node_agent_local_task_store::LocalTaskRecord,
    node_agent_local_task_supervision::{
        load_supervision_contract, load_supervision_state, SupervisionContract,
        SUPERVISION_PROTOCOL,
    },
    Credentials, NodeRuntime,
};

pub(crate) const RESUME_CONTEXT_SCHEMA: &str = "elon.resume_context.v1";

pub(crate) struct ResumeContextSeed {
    root: LocalTaskRecord,
    parent: LocalTaskRecord,
    parent_summary: Value,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RequirementRef<'a> {
    task_id: &'a str,
    sha256: String,
    chars: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ParentSummary<'a> {
    task_id: &'a str,
    status: &'a str,
    error: Option<String>,
    final_reply: Option<String>,
    supervision: &'a Value,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WorkspaceIdentity<'a> {
    project_id: &'a str,
    authorized_workspace_path: &'a str,
    active_workspace_path: &'a str,
    branch: &'a str,
    git_head: &'a str,
    derivation: &'a str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ResumeContextPacket<'a> {
    schema: &'static str,
    root_task_id: &'a str,
    parent_task_id: &'a str,
    requirement: RequirementRef<'a>,
    acceptance_criteria: &'a [String],
    parent: ParentSummary<'a>,
    workspace: WorkspaceIdentity<'a>,
    must_not_repeat: [&'static str; 3],
}

pub(crate) fn resolve_seed(
    runtime: &NodeRuntime,
    creds: &Credentials,
    project_id: &str,
    contract: &mut SupervisionContract,
) -> Result<ResumeContextSeed> {
    if contract.protocol != SUPERVISION_PROTOCOL || contract.task_role != "resume_original" {
        bail!("resume context 只接受当前监督协议的 resume_original。")
    }
    let parent_task_id = contract
        .parent_task_id
        .as_deref()
        .ok_or_else(|| anyhow!("resume_original 缺少 parent_task_id。"))?;
    let root_task_id = contract
        .root_task_id
        .as_deref()
        .ok_or_else(|| anyhow!("resume_original 缺少 root_task_id。"))?;
    let parent = runtime
        .local_tasks
        .get_for_owner(&creds.owner_user_id, parent_task_id)?
        .ok_or_else(|| anyhow!("resume parent 不存在或不属于当前账号。"))?;
    let root = runtime
        .local_tasks
        .get_for_owner(&creds.owner_user_id, root_task_id)?
        .ok_or_else(|| anyhow!("resume root 不存在或不属于当前账号。"))?;
    for task in [&parent, &root] {
        if task.agent_id != creds.agent_id || task.install_id != runtime.install_id {
            bail!("resume lineage 不属于当前节点和安装实例。")
        }
        if !crate::node_agent_full_access::project_ids_equivalent(project_id, &task.project_id) {
            bail!("resume lineage 不能跨项目。")
        }
    }

    let parent_contract = load_supervision_contract(&runtime.task_journal, &parent.task_id)?
        .ok_or_else(|| anyhow!("resume parent 缺少监督契约。"))?;
    let parent_root = parent_contract
        .root_task_id
        .as_deref()
        .unwrap_or(parent.task_id.as_str());
    if parent_root != root.task_id {
        bail!("resume parent 的 root_task_id 与请求不一致。")
    }
    let root_contract = load_supervision_contract(&runtime.task_journal, &root.task_id)?
        .ok_or_else(|| anyhow!("resume root 缺少监督契约。"))?;
    if root_contract.protocol != SUPERVISION_PROTOCOL
        || root_contract.task_role != "requirement"
        || root_contract
            .root_task_id
            .as_deref()
            .is_some_and(|value| value != root.task_id)
    {
        bail!("resume root 不是当前协议的权威 requirement 任务。")
    }
    if !contract.acceptance_criteria.is_empty()
        && contract.acceptance_criteria != root_contract.acceptance_criteria
    {
        bail!("resume acceptance_criteria 与根任务发生漂移。")
    }
    contract.acceptance_criteria = root_contract.acceptance_criteria;
    let parent_summary =
        load_supervision_state(&runtime.task_journal, &parent.task_id)?.resume_summary_payload();
    Ok(ResumeContextSeed {
        root,
        parent,
        parent_summary,
    })
}

pub(crate) struct CompiledResumeContext {
    pub(crate) record_prompt: String,
    pub(crate) executor_prompt: String,
    pub(crate) journal_payload: Value,
    pub(crate) digest: String,
}

pub(crate) fn compile(
    seed: &ResumeContextSeed,
    contract: &SupervisionContract,
    workspace: &ResolvedResumeWorkspace,
) -> Result<CompiledResumeContext> {
    let active_workspace_path = workspace.inherited_workspace.workspace_path.as_str();
    let branch = workspace
        .inherited_workspace
        .branch
        .as_deref()
        .ok_or_else(|| anyhow!("resume workspace 缺少隔离分支。"))?;
    let packet = ResumeContextPacket {
        schema: RESUME_CONTEXT_SCHEMA,
        root_task_id: &seed.root.task_id,
        parent_task_id: &seed.parent.task_id,
        requirement: RequirementRef {
            task_id: &seed.root.task_id,
            sha256: sha256_hex(seed.root.prompt.as_bytes()),
            chars: seed.root.prompt.chars().count(),
        },
        acceptance_criteria: &contract.acceptance_criteria,
        parent: ParentSummary {
            task_id: &seed.parent.task_id,
            status: &seed.parent.status,
            error: bounded(seed.parent.error.as_deref(), 4_000),
            final_reply: bounded(seed.parent.final_reply.as_deref(), 12_000),
            supervision: &seed.parent_summary,
        },
        workspace: WorkspaceIdentity {
            project_id: &seed.root.project_id,
            authorized_workspace_path: &workspace.authorized_workspace_path,
            active_workspace_path,
            branch,
            git_head: &workspace.git_head,
            derivation: &workspace.derivation,
        },
        must_not_repeat: [
            "Inspect current workspace and evidence before repeating completed work.",
            "Do not redispatch this write task to the PC node.",
            "Do not infer missing context; fail closed on identity or source drift.",
        ],
    };
    let packet_json = serde_json::to_string(&packet).context("serialize resume context")?;
    let digest = sha256_hex(packet_json.as_bytes());
    let contract_json =
        serde_json::to_string(contract).context("serialize supervision contract")?;
    let executor_prompt = format!(
        r#"<elon-pc-executor version="2" protocol="{protocol}">
你是由一龙 PC 本机节点启动的执行者，不是桌面监督者。
1. 直接在当前项目续跑任务；不得再次派发给 PC 节点。
2. 先检查现有工作区、父任务证据和 Git 身份，只补做尚未完成的工作。
3. 遵守项目 AGENTS.md、验证、提交、发布与统一收尾要求。
resume_context_digest={digest}
supervision_contract={contract_json}
resume_context={packet_json}
</elon-pc-executor>

<authoritative-root-request>
{root_prompt}
</authoritative-root-request>"#,
        protocol = SUPERVISION_PROTOCOL,
        root_prompt = seed.root.prompt,
    );
    let record_prompt = format!(
        "Resolve {RESUME_CONTEXT_SCHEMA}: root_task_id={}, parent_task_id={}, digest={digest}",
        seed.root.task_id, seed.parent.task_id
    );
    Ok(CompiledResumeContext {
        record_prompt,
        executor_prompt,
        journal_payload: json!({
            "schema": RESUME_CONTEXT_SCHEMA,
            "root_task_id": seed.root.task_id,
            "parent_task_id": seed.parent.task_id,
            "digest": digest,
            "requirement_sha256": sha256_hex(seed.root.prompt.as_bytes()),
            "workspace_git_head": workspace.git_head,
        }),
        digest,
    })
}

fn bounded(value: Option<&str>, max_chars: usize) -> Option<String> {
    value.map(|value| value.chars().take(max_chars).collect())
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node_agent_local_task_supervision::SupervisionContract;
    use crate::pc_workspace_provisioner::ConversationWorkspaceResult;

    #[test]
    fn bounded_summary_is_unicode_safe() {
        assert_eq!(bounded(Some("一二三四"), 3).as_deref(), Some("一二三"));
        assert_eq!(bounded(None, 3), None);
    }

    #[test]
    fn five_generation_resume_keeps_one_authoritative_root_prompt() {
        let root_prompt = "ROOT REQUIREMENT UNIQUE";
        let root = record("root", root_prompt);
        let mut parent = record("parent-0", "OLD PARENT PROMPT MUST NOT LEAK");
        let contract = SupervisionContract {
            protocol: SUPERVISION_PROTOCOL.into(),
            supervisor: "codex_desktop".into(),
            task_role: "resume_original".into(),
            parent_task_id: Some(parent.task_id.clone()),
            root_task_id: Some(root.task_id.clone()),
            acceptance_criteria: vec!["same acceptance".into()],
            improvement_policy: "after_task_or_unblock".into(),
        };
        let workspace = ResolvedResumeWorkspace {
            authorized_workspace_path: "C:/repo".into(),
            inherited_workspace: ConversationWorkspaceResult {
                base_workspace_path: Some("C:/repo".into()),
                workspace_path: "C:/worktree".into(),
                isolated: true,
                branch: Some("ai/session/project/root".into()),
                supervision_root_task_id: Some("root".into()),
            },
            derivation: "workspace_status".into(),
            git_head: "0123456789abcdef".into(),
            requires_recreation: false,
            snapshot_continue_required: false,
            lease_migration: None,
            resume_admission: None,
        };
        for generation in 1..=5 {
            let seed = ResumeContextSeed {
                root: root.clone(),
                parent: parent.clone(),
                parent_summary: json!({"generation":generation}),
            };
            let compiled = compile(&seed, &contract, &workspace).unwrap();
            assert_eq!(compiled.executor_prompt.matches(root_prompt).count(), 1);
            assert!(!compiled
                .executor_prompt
                .contains("OLD PARENT PROMPT MUST NOT LEAK"));
            assert!(!compiled
                .executor_prompt
                .contains("Resume the original task"));
            parent = record(&format!("parent-{generation}"), &compiled.executor_prompt);
        }
    }

    fn record(task_id: &str, prompt: &str) -> LocalTaskRecord {
        LocalTaskRecord {
            task_id: task_id.into(),
            owner_user_id: "owner".into(),
            agent_id: "agent".into(),
            install_id: "install".into(),
            project_id: "project".into(),
            channel_id: None,
            conversation_id: task_id.into(),
            workspace_path: "C:/repo".into(),
            prompt: prompt.into(),
            cli: "codex".into(),
            runtime_permission: "full_access".into(),
            execution_origin: "local_offline".into(),
            billing_source: "own_codex".into(),
            status: "done".into(),
            error: None,
            final_reply: Some("done".into()),
            model: None,
            codex_session_id: None,
            input_tokens: None,
            cached_input_tokens: None,
            output_tokens: None,
            reasoning_tokens: None,
            total_tokens: None,
            workspace_status: None,
            sync_state: "local_only".into(),
            completion_event_id: None,
            started_at_ms: 1,
            finished_at_ms: Some(2),
            server_ack_at_ms: None,
        }
    }
}
