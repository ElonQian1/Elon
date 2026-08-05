//! Full-governance MCP tools for the Git-backed feature registry.

use anyhow::{anyhow, Result};
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::Path;

use crate::{
    project_feature_registry::ProjectFeatureStatus,
    project_feature_registry_service::{
        check_drift, claim_feature, feature_history, list_features, plan_feature, record_evidence,
        register_feature, release_claim, transition_feature, RegisterFeatureRequest,
    },
    project_feature_registry_store::FeatureEvidenceInput,
    project_feature_registry_update::{
        rebind_requirement, update_feature, RebindRequirementRequest, UpdateFeatureRequest,
    },
};

#[derive(Debug, Deserialize)]
struct ListArguments {
    #[serde(default)]
    statuses: Vec<ProjectFeatureStatus>,
    #[serde(default)]
    query: String,
    #[serde(default)]
    offset: usize,
    #[serde(default = "default_limit")]
    limit: usize,
}

#[derive(Debug, Deserialize)]
struct FeatureArguments {
    feature_id: String,
}

#[derive(Debug, Deserialize)]
struct ClaimArguments {
    feature_id: String,
    agent_id: String,
    #[serde(default = "default_lease_minutes")]
    lease_minutes: u64,
    #[serde(default)]
    expected_registry_revision: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ReleaseClaimArguments {
    feature_id: String,
    claim_id: String,
    #[serde(default)]
    reason: String,
    #[serde(default)]
    expected_registry_revision: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TransitionArguments {
    feature_id: String,
    to_status: ProjectFeatureStatus,
    actor: String,
    #[serde(default)]
    reason: String,
    #[serde(default)]
    claim_id: String,
    #[serde(default)]
    expected_registry_revision: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RecordEvidenceArguments {
    feature_id: String,
    #[serde(default)]
    claim_id: String,
    actor: String,
    evidence: Vec<FeatureEvidenceInput>,
    #[serde(default)]
    expected_registry_revision: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DriftArguments {
    #[serde(default)]
    feature_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct HistoryArguments {
    #[serde(default)]
    feature_id: Option<String>,
    #[serde(default)]
    offset: usize,
    #[serde(default = "default_limit")]
    limit: usize,
}

pub(crate) fn definitions() -> Vec<Value> {
    vec![
        tool(
            "project_features_register",
            "在 Codex 已创建正式需求 Markdown 后，将其登记为 Git 共享功能模块。服务端绑定需求当前 SHA-256/Git 对象，只保存短摘要、状态、依赖和验收标准，不复制正文；accepted/ready 拒绝草稿、收件箱、历史、归档和非 current 文档。",
            json!({
                "type":"object",
                "required":["id","title","summary","requirement_path","actor"],
                "properties":{
                    "id":{"type":"string","pattern":"^[A-Za-z0-9._-]{1,96}$"},
                    "title":{"type":"string","minLength":1,"maxLength":160},
                    "summary":{"type":"string","minLength":1,"maxLength":800},
                    "status":{"type":"string","enum":["draft","proposed","accepted","ready"],"default":"proposed"},
                    "priority":priority_schema(),
                    "requirement_path":{"type":"string","description":"现存的工作区相对 Markdown 路径；正文由 Codex 原生编辑工具创建。"},
                    "knowledge_node_id":{"type":"string","maxLength":96,"description":"可选；提供时必须已存在于知识图谱。"},
                    "owner":{"type":"string","maxLength":80},
                    "tags":{"type":"array","maxItems":12,"items":{"type":"string","maxLength":48}},
                    "task_paths":{"type":"array","maxItems":24,"items":{"type":"string"}},
                    "dependencies":{"type":"array","maxItems":32,"items":{"type":"string","pattern":"^[A-Za-z0-9._-]{1,96}$"}},
                    "acceptance_criteria":{"type":"array","maxItems":32,"items":{"type":"string","maxLength":500}},
                    "actor":{"type":"string","minLength":1,"maxLength":120},
                    "reason":{"type":"string","maxLength":500},
                    "expected_registry_revision":{"type":"string","description":"注册表已存在时必传；首次创建时省略。"}
                }
            }),
        ),
        tool(
            "project_features_list",
            "分页列出功能需求、状态、优先级、认领、依赖阻塞和需求漂移；为保持低延迟只计数实现证据，精确实现证据漂移由单功能 plan/check_drift 校验。不返回正文、不修改文件。",
            json!({
                "type":"object","properties":{
                    "statuses":{"type":"array","maxItems":11,"items":status_schema()},
                    "query":{"type":"string","maxLength":200},
                    "offset":{"type":"integer","minimum":0,"default":0},
                    "limit":{"type":"integer","minimum":1,"maximum":100,"default":50}
                }
            }),
        ),
        tool(
            "project_features_update",
            "显式更新尚未完成的功能元数据。只有传入字段会被替换；更改任务路径、依赖或验收标准会把 accepted/ready/blocked 回退到 proposed 并清除旧实现证据，避免旧范围冒充已评审范围。",
            json!({
                "type":"object","required":["feature_id","actor","expected_registry_revision"],
                "properties":{
                    "feature_id":{"type":"string","maxLength":96},
                    "title":{"type":"string","minLength":1,"maxLength":160},
                    "summary":{"type":"string","minLength":1,"maxLength":800},
                    "priority":priority_schema(),
                    "knowledge_node_id":{"type":"string","maxLength":96},
                    "owner":{"type":"string","maxLength":80},
                    "tags":{"type":"array","maxItems":12,"items":{"type":"string","maxLength":48}},
                    "task_paths":{"type":"array","maxItems":24,"items":{"type":"string"}},
                    "dependencies":{"type":"array","maxItems":32,"items":{"type":"string","pattern":"^[A-Za-z0-9._-]{1,96}$"}},
                    "acceptance_criteria":{"type":"array","maxItems":32,"items":{"type":"string","maxLength":500}},
                    "actor":{"type":"string","minLength":1,"maxLength":120},
                    "reason":{"type":"string","maxLength":500},
                    "expected_registry_revision":{"type":"string","minLength":1}
                }
            }),
        ),
        tool(
            "project_features_rebind_requirement",
            "需求 Markdown 被有意修改或移动后，重新绑定当前哈希和 Git 身份。accepted/ready/blocked 会回退到 proposed、清除旧实现证据并要求重新评审；不得用此工具静默确认漂移。",
            json!({
                "type":"object","required":["feature_id","actor","expected_registry_revision"],
                "properties":{
                    "feature_id":{"type":"string","maxLength":96},
                    "requirement_path":{"type":"string","description":"可选新路径；省略或空字符串时沿用已登记路径。"},
                    "actor":{"type":"string","minLength":1,"maxLength":120},
                    "reason":{"type":"string","maxLength":500},
                    "expected_registry_revision":{"type":"string","minLength":1}
                }
            }),
        ),
        tool(
            "project_features_plan",
            "为一个功能生成低 token 任务包：需求路径与哈希、验收标准、依赖、任务路径、当前认领和实现证据；不返回正文，代理再用原生工具只读命中路径。",
            json!({"type":"object","required":["feature_id"],"properties":{"feature_id":{"type":"string","maxLength":96}}}),
        ),
        tool(
            "project_features_claim",
            "使用 optimistic registry revision 认领 ready 功能并创建有期限 claim。新 claim 会清除上一轮实现证据；需求漂移、依赖未完成、未过期的其他认领都会失败关闭。",
            json!({
                "type":"object","required":["feature_id","agent_id","expected_registry_revision"],
                "properties":{
                    "feature_id":{"type":"string","maxLength":96},"agent_id":{"type":"string","minLength":1,"maxLength":120},
                    "lease_minutes":{"type":"integer","minimum":5,"maximum":1440,"default":120},
                    "expected_registry_revision":{"type":"string","minLength":1}
                }
            }),
        ),
        tool(
            "project_features_release_claim",
            "显式释放 claimed/in_progress 功能；必须匹配 claim_id 和 registry revision，允许清理已过期认领。需求或依赖仍无效时进入 blocked，否则退回 ready。",
            json!({
                "type":"object","required":["feature_id","claim_id","expected_registry_revision"],
                "properties":{"feature_id":{"type":"string","maxLength":96},"claim_id":{"type":"string","maxLength":96},"reason":{"type":"string","maxLength":500},"expected_registry_revision":{"type":"string","minLength":1}}
            }),
        ),
        tool(
            "project_features_transition",
            "按固定状态机推进功能。claim 绑定阶段必须传 claim_id；implemented 需要当前实现证据，verified/released 还需要 test 证据。不能跳过验收或用注册表覆盖当前源码事实。",
            json!({
                "type":"object","required":["feature_id","to_status","actor","expected_registry_revision"],
                "properties":{"feature_id":{"type":"string","maxLength":96},"to_status":transition_status_schema(),"actor":{"type":"string","minLength":1,"maxLength":120},"reason":{"type":"string","maxLength":500},"claim_id":{"type":"string","maxLength":96},"expected_registry_revision":{"type":"string","minLength":1}}
            }),
        ),
        tool(
            "project_features_record_evidence",
            "为 claimed/in_progress/blocked/implemented 功能记录 1–16 条当前工作区证据。调用方只传相对路径、定位符和类型；服务端绑定当前 SHA-256/Git 对象，不保存源码正文。",
            json!({
                "type":"object","required":["feature_id","actor","evidence","expected_registry_revision"],
                "properties":{"feature_id":{"type":"string","maxLength":96},"claim_id":{"type":"string","maxLength":96},"actor":{"type":"string","minLength":1,"maxLength":120},"expected_registry_revision":{"type":"string","minLength":1},
                    "evidence":{"type":"array","minItems":1,"maxItems":16,"items":{"type":"object","required":["path"],"properties":{"path":{"type":"string"},"locator":{"type":"string","maxLength":160},"evidence_kind":{"type":"string","enum":["source","test","document","configuration"],"default":"source"}}}}
                }
            }),
        ),
        tool(
            "project_features_check_drift",
            "检查需求和实现证据的当前 SHA-256/Git 身份、依赖阻塞与认领过期状态，返回非自动 repair plan；不读取正文、不修改仓库。",
            json!({"type":"object","properties":{"feature_id":{"type":"string","maxLength":96}}}),
        ),
        tool(
            "project_features_history",
            "按时间倒序分页返回功能登记审计事件，可限定单个 feature。只含 actor、原因、状态和时间等有界元数据，不返回需求、源码或聊天正文。",
            json!({"type":"object","properties":{"feature_id":{"type":"string","maxLength":96},"offset":{"type":"integer","minimum":0,"default":0},"limit":{"type":"integer","minimum":1,"maximum":100,"default":50}}}),
        ),
    ]
}

pub(crate) fn try_call(workspace: &Path, name: &str, arguments: Value) -> Result<Option<Value>> {
    let result = match name {
        "project_features_register" => register_feature(workspace, decode(arguments, name)?)?,
        "project_features_list" => {
            let input: ListArguments = decode(arguments, name)?;
            list_features(
                workspace,
                &input.statuses,
                &input.query,
                input.offset,
                input.limit,
            )?
        }
        "project_features_update" => {
            let input: UpdateFeatureRequest = decode(arguments, name)?;
            update_feature(workspace, input)?
        }
        "project_features_rebind_requirement" => {
            let input: RebindRequirementRequest = decode(arguments, name)?;
            rebind_requirement(workspace, input)?
        }
        "project_features_plan" => {
            let input: FeatureArguments = decode(arguments, name)?;
            plan_feature(workspace, &input.feature_id)?
        }
        "project_features_claim" => {
            let input: ClaimArguments = decode(arguments, name)?;
            claim_feature(
                workspace,
                &input.feature_id,
                &input.agent_id,
                input.lease_minutes,
                input.expected_registry_revision.as_deref(),
            )?
        }
        "project_features_release_claim" => {
            let input: ReleaseClaimArguments = decode(arguments, name)?;
            release_claim(
                workspace,
                &input.feature_id,
                &input.claim_id,
                &input.reason,
                input.expected_registry_revision.as_deref(),
            )?
        }
        "project_features_transition" => {
            let input: TransitionArguments = decode(arguments, name)?;
            transition_feature(
                workspace,
                &input.feature_id,
                input.to_status,
                &input.actor,
                &input.reason,
                &input.claim_id,
                input.expected_registry_revision.as_deref(),
            )?
        }
        "project_features_record_evidence" => {
            let input: RecordEvidenceArguments = decode(arguments, name)?;
            record_evidence(
                workspace,
                &input.feature_id,
                &input.claim_id,
                &input.actor,
                input.evidence,
                input.expected_registry_revision.as_deref(),
            )?
        }
        "project_features_check_drift" => {
            let input: DriftArguments = decode(arguments, name)?;
            check_drift(workspace, input.feature_id.as_deref())?
        }
        "project_features_history" => {
            let input: HistoryArguments = decode(arguments, name)?;
            feature_history(
                workspace,
                input.feature_id.as_deref(),
                input.offset,
                input.limit,
            )?
        }
        _ => return Ok(None),
    };
    Ok(Some(result))
}

fn tool(name: &str, description: &str, input_schema: Value) -> Value {
    json!({"name":name,"description":description,"inputSchema":input_schema})
}

fn status_schema() -> Value {
    json!({"type":"string","enum":["draft","proposed","accepted","ready","claimed","in_progress","blocked","implemented","verified","released","retired"]})
}

fn transition_status_schema() -> Value {
    json!({"type":"string","enum":["draft","proposed","accepted","ready","in_progress","blocked","implemented","verified","released","retired"]})
}

fn priority_schema() -> Value {
    json!({"type":"string","enum":["p0","p1","p2","p3"],"default":"p2"})
}

fn decode<T: serde::de::DeserializeOwned>(value: Value, name: &str) -> Result<T> {
    serde_json::from_value(value).map_err(|error| anyhow!("{name} arguments 无效：{error}"))
}

fn default_limit() -> usize {
    50
}

fn default_lease_minutes() -> u64 {
    120
}
