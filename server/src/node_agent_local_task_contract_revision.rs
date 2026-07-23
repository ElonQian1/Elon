//! Immutable requirement revisions for supervised task continuation.
//!
//! The root requirement remains unchanged. An explicit Desktop `Supersede`
//! request appends one validated revision to the child resume-context event.
//! Later Resume generations reconstruct the effective requirement by walking
//! the durable parent lineage, so they never fall back to the stale root text.

use std::{
    collections::BTreeSet,
    fs::File,
    io::{BufRead, BufReader},
};

use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::{
    node_agent_local_task_store::LocalTaskRecord,
    node_agent_local_task_supervision::{
        load_supervision_contract, SupervisionContract, SUPERVISION_PROTOCOL,
    },
    node_agent_task_journal_lock::with_task_journal_io_lock,
    NodeRuntime,
};

pub(crate) const CONTRACT_REVISION_SCHEMA: &str = "elon.supervision.contract_revision.v1";
const MAX_LINEAGE_DEPTH: usize = 64;
const MAX_REASON_CHARS: usize = 2_000;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ContractRevisionInput {
    pub schema: String,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct EffectiveRequirement {
    pub source_task_id: String,
    pub prompt: String,
    pub acceptance_criteria: Vec<String>,
    pub digest: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
struct ContractRevisionEvent {
    schema: String,
    root_task_id: String,
    parent_task_id: String,
    source_task_id: String,
    previous_source_task_id: String,
    previous_digest: String,
    reason: String,
    prompt: String,
    acceptance_criteria: Vec<String>,
    effective_digest: String,
}

pub(crate) fn resolve_and_apply(
    runtime: &NodeRuntime,
    root: &LocalTaskRecord,
    parent: &LocalTaskRecord,
    task_id: &str,
    requested_prompt: &str,
    contract: &mut SupervisionContract,
    revision: Option<&ContractRevisionInput>,
) -> Result<(EffectiveRequirement, Option<Value>)> {
    let effective = resolve_effective(runtime, root, parent)?;
    apply_revision(
        effective,
        task_id,
        &root.task_id,
        &parent.task_id,
        requested_prompt,
        contract,
        revision,
    )
}

fn resolve_effective(
    runtime: &NodeRuntime,
    root: &LocalTaskRecord,
    parent: &LocalTaskRecord,
) -> Result<EffectiveRequirement> {
    let root_contract = load_supervision_contract(&runtime.task_journal, &root.task_id)?
        .ok_or_else(|| anyhow!("resume root 缺少监督契约。"))?;
    if root_contract.protocol != SUPERVISION_PROTOCOL
        || root_contract.task_role != "requirement"
        || root_contract.parent_task_id.is_some()
    {
        bail!("resume root 不是不可变的权威 requirement 任务。")
    }
    let mut effective = requirement(
        &root.task_id,
        &root.task_id,
        &root.prompt,
        root_contract.acceptance_criteria.clone(),
    );
    if parent.task_id == root.task_id {
        return Ok(effective);
    }

    let mut chain = Vec::new();
    let mut current = parent.clone();
    let mut visited = BTreeSet::new();
    for _ in 0..MAX_LINEAGE_DEPTH {
        if !visited.insert(current.task_id.clone()) {
            bail!("resume contract revision lineage contains a cycle.")
        }
        if current.task_id == root.task_id {
            break;
        }
        let current_contract = load_supervision_contract(&runtime.task_journal, &current.task_id)?
            .ok_or_else(|| anyhow!("resume revision lineage 缺少监督契约。"))?;
        if current_contract.protocol != SUPERVISION_PROTOCOL
            || current_contract.task_role != "resume_original"
            || current_contract.root_task_id.as_deref() != Some(root.task_id.as_str())
        {
            bail!("resume revision lineage 身份或角色漂移。")
        }
        let parent_id = current_contract
            .parent_task_id
            .clone()
            .ok_or_else(|| anyhow!("resume revision lineage 缺少 parent_task_id。"))?;
        chain.push((current.clone(), current_contract));
        current = runtime
            .local_tasks
            .get_for_owner(&root.owner_user_id, &parent_id)?
            .ok_or_else(|| anyhow!("resume revision lineage 的父任务不存在。"))?;
    }
    if current.task_id != root.task_id {
        bail!("resume contract revision lineage 超过安全深度或没有回到根任务。")
    }

    chain.reverse();
    let mut previous_task_id = root.task_id.clone();
    for (task, task_contract) in chain {
        if task_contract.parent_task_id.as_deref() != Some(previous_task_id.as_str()) {
            bail!("resume revision lineage parent 顺序漂移。")
        }
        if let Some(event) = load_revision_event(runtime, &task.task_id)? {
            validate_event(
                &event,
                &root.task_id,
                &previous_task_id,
                &task.task_id,
                &effective,
            )?;
            effective = requirement(
                &root.task_id,
                &event.source_task_id,
                &event.prompt,
                event.acceptance_criteria,
            );
            if effective.digest != event.effective_digest {
                bail!("resume contract revision effective digest 漂移。")
            }
        } else if !task_contract.acceptance_criteria.is_empty()
            && task_contract.acceptance_criteria != effective.acceptance_criteria
        {
            bail!("resume acceptance_criteria 与有效合同发生漂移，且没有 Supersede 修订收据。")
        }
        previous_task_id = task.task_id;
    }
    Ok(effective)
}

fn apply_revision(
    effective: EffectiveRequirement,
    task_id: &str,
    root_task_id: &str,
    parent_task_id: &str,
    requested_prompt: &str,
    contract: &mut SupervisionContract,
    revision: Option<&ContractRevisionInput>,
) -> Result<(EffectiveRequirement, Option<Value>)> {
    let Some(revision) = revision else {
        if !contract.acceptance_criteria.is_empty()
            && contract.acceptance_criteria != effective.acceptance_criteria
        {
            bail!("resume acceptance_criteria 与有效合同发生漂移；需求确有变化时请显式使用 Supersede。")
        }
        contract.acceptance_criteria = effective.acceptance_criteria.clone();
        return Ok((effective, None));
    };
    if revision.schema.trim() != CONTRACT_REVISION_SCHEMA {
        bail!("contract_revision.schema 不受支持。")
    }
    let reason = revision.reason.trim();
    if reason.is_empty()
        || reason.chars().count() > MAX_REASON_CHARS
        || reason.chars().any(char::is_control)
    {
        bail!("Supersede 需要 1 到 2000 字且不含控制字符的变更原因。")
    }
    let prompt = requested_prompt.trim();
    if prompt.is_empty() {
        bail!("Supersede 需要新的完整用户需求。")
    }
    if contract.acceptance_criteria.is_empty() {
        bail!("Supersede 需要显式的新验收条件，不能从旧合同猜测。")
    }
    if prompt == effective.prompt && contract.acceptance_criteria == effective.acceptance_criteria {
        bail!("Supersede 没有改变需求或验收条件，请使用普通 Resume。")
    }
    let revised = requirement(
        root_task_id,
        task_id,
        prompt,
        contract.acceptance_criteria.clone(),
    );
    let event = ContractRevisionEvent {
        schema: CONTRACT_REVISION_SCHEMA.to_string(),
        root_task_id: root_task_id.to_string(),
        parent_task_id: parent_task_id.to_string(),
        source_task_id: task_id.to_string(),
        previous_source_task_id: effective.source_task_id.clone(),
        previous_digest: effective.digest.clone(),
        reason: reason.to_string(),
        prompt: revised.prompt.clone(),
        acceptance_criteria: revised.acceptance_criteria.clone(),
        effective_digest: revised.digest.clone(),
    };
    Ok((revised, Some(serde_json::to_value(event)?)))
}

fn validate_event(
    event: &ContractRevisionEvent,
    root_task_id: &str,
    parent_task_id: &str,
    source_task_id: &str,
    effective: &EffectiveRequirement,
) -> Result<()> {
    if event.schema != CONTRACT_REVISION_SCHEMA
        || event.root_task_id != root_task_id
        || event.parent_task_id != parent_task_id
        || event.source_task_id != source_task_id
        || event.previous_source_task_id != effective.source_task_id
        || event.previous_digest != effective.digest
        || event.reason.trim().is_empty()
        || event.reason.chars().count() > MAX_REASON_CHARS
    {
        bail!("resume contract revision provenance 漂移。")
    }
    Ok(())
}

fn load_revision_event(
    runtime: &NodeRuntime,
    task_id: &str,
) -> Result<Option<ContractRevisionEvent>> {
    with_task_journal_io_lock(|| {
        let path = runtime.task_journal.events_path();
        if !path.exists() {
            return Ok(None);
        }
        let file = File::open(&path).with_context(|| format!("读取 {:?}", path))?;
        let mut found: Option<ContractRevisionEvent> = None;
        for line in BufReader::new(file).lines() {
            let line = line.with_context(|| format!("读取 {:?}", path))?;
            let Ok(event) = serde_json::from_str::<Value>(&line) else {
                continue;
            };
            if event.get("req_id").and_then(Value::as_str) != Some(task_id)
                || event.get("type").and_then(Value::as_str) != Some("resume_context")
            {
                continue;
            }
            let Some(parsed) = parse_revision_event(&event)? else {
                continue;
            };
            if found.as_ref().is_some_and(|existing| existing != &parsed) {
                bail!("同一任务存在冲突的 contract revision 收据。")
            }
            found = Some(parsed);
        }
        Ok(found)
    })
}

fn parse_revision_event(event: &Value) -> Result<Option<ContractRevisionEvent>> {
    let Some(value) = event
        .get("payload")
        .and_then(|payload| payload.get("contract_revision"))
    else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    let parsed =
        serde_json::from_value(value.clone()).context("解析 contract revision journal event")?;
    Ok(Some(parsed))
}

pub(crate) fn task_has_revision(runtime: &NodeRuntime, task_id: &str) -> Result<bool> {
    Ok(load_revision_event(runtime, task_id)?.is_some())
}

fn requirement(
    root_task_id: &str,
    source_task_id: &str,
    prompt: &str,
    acceptance_criteria: Vec<String>,
) -> EffectiveRequirement {
    let digest = hex::encode(Sha256::digest(
        serde_json::to_vec(&json!({
            "schema": CONTRACT_REVISION_SCHEMA,
            "root_task_id": root_task_id,
            "source_task_id": source_task_id,
            "prompt": prompt,
            "acceptance_criteria": &acceptance_criteria,
        }))
        .expect("serialize effective requirement"),
    ));
    EffectiveRequirement {
        source_task_id: source_task_id.to_string(),
        prompt: prompt.to_string(),
        acceptance_criteria,
        digest,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contract(criteria: &[&str]) -> SupervisionContract {
        SupervisionContract {
            protocol: SUPERVISION_PROTOCOL.into(),
            supervisor: "codex_desktop".into(),
            task_role: "resume_original".into(),
            parent_task_id: Some("parent".into()),
            root_task_id: Some("root".into()),
            acceptance_criteria: criteria.iter().map(|value| value.to_string()).collect(),
            improvement_policy: "after_task_or_unblock".into(),
        }
    }

    #[test]
    fn ordinary_resume_inherits_effective_contract_and_rejects_silent_drift() {
        let base = requirement("root", "root", "old", vec!["old criterion".into()]);
        let mut inherited = contract(&[]);
        let (resolved, receipt) = apply_revision(
            base.clone(),
            "child",
            "root",
            "parent",
            "ignored",
            &mut inherited,
            None,
        )
        .unwrap();
        assert_eq!(resolved, base);
        assert_eq!(inherited.acceptance_criteria, vec!["old criterion"]);
        assert!(receipt.is_none());

        let mut drifted = contract(&["changed without authorization"]);
        assert!(apply_revision(
            base,
            "child",
            "root",
            "parent",
            "ignored",
            &mut drifted,
            None,
        )
        .unwrap_err()
        .to_string()
        .contains("Supersede"));
    }

    #[test]
    fn explicit_supersede_keeps_previous_digest_and_new_effective_requirement() {
        let base = requirement("root", "root", "old", vec!["old criterion".into()]);
        let mut revised_contract = contract(&["new criterion"]);
        let input = ContractRevisionInput {
            schema: CONTRACT_REVISION_SCHEMA.into(),
            reason: "用户改变了验收目标".into(),
        };
        let (revised, receipt) = apply_revision(
            base.clone(),
            "child",
            "root",
            "parent",
            "new requirement",
            &mut revised_contract,
            Some(&input),
        )
        .unwrap();
        assert_eq!(revised.source_task_id, "child");
        assert_eq!(revised.prompt, "new requirement");
        let receipt = receipt.unwrap();
        assert_eq!(receipt["previous_digest"], base.digest);
        assert_eq!(receipt["effective_digest"], revised.digest);
    }

    #[test]
    fn load_revision_event_skips_explicit_null_contract_revision() {
        let event = json!({
            "payload": {
                "contract_revision": null,
            },
        });

        assert!(parse_revision_event(&event).unwrap().is_none());
    }

    #[test]
    fn load_revision_event_rejects_malformed_non_null_contract_revision() {
        let event = json!({
            "payload": {
                "contract_revision": {
                    "schema": CONTRACT_REVISION_SCHEMA,
                },
            },
        });

        let error = parse_revision_event(&event).unwrap_err();
        assert!(error
            .to_string()
            .contains("解析 contract revision journal event"));
    }
}
