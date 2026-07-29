//! Durable, project-scoped diagnostics for the document organization MCP workflow.

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
    time::{SystemTime, UNIX_EPOCH},
};

const TRACE_VERSION: u8 = 1;
const MAX_EVENTS: usize = 40;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct DocumentOrganizationTrace {
    pub version: u8,
    pub operation_id: String,
    pub status: String,
    pub current_stage: String,
    pub created_at: u64,
    pub updated_at: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub catalog_revision: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manifest_revision: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub discussion_graph_revision: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suggestions_revision: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git_baseline_commit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git_result_commit: Option<String>,
    #[serde(default)]
    pub documents_cataloged: u64,
    #[serde(default)]
    pub ambiguous_documents: u64,
    #[serde(default)]
    pub documents_read: u64,
    #[serde(default)]
    pub estimated_tokens_used: u64,
    #[serde(default)]
    pub events: Vec<DocumentOrganizationEvent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<DocumentOrganizationError>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct DocumentOrganizationEvent {
    pub stage: String,
    pub status: String,
    pub label: String,
    pub detail: String,
    pub at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct DocumentOrganizationError {
    pub code: String,
    pub message: String,
    pub recovery: String,
    pub at: u64,
}

pub(crate) fn start_operation(workspace: &Path, operation_id: Option<&str>) -> Result<Value> {
    let workspace = validate_workspace(workspace)?;
    let operation_id = normalize_operation_id(operation_id.unwrap_or(""))?;
    with_trace_lock(|| {
        if let Some(existing) = read_trace_unlocked(&workspace)? {
            if existing.operation_id == operation_id {
                return serde_json::to_value(existing).context("序列化文档整理状态失败");
            }
        }
        let now = unix_seconds();
        let mut trace = DocumentOrganizationTrace {
            version: TRACE_VERSION,
            operation_id,
            status: "pending".to_string(),
            current_stage: "requested".to_string(),
            created_at: now,
            updated_at: now,
            task_id: None,
            session_id: None,
            catalog_revision: None,
            manifest_revision: None,
            discussion_graph_revision: None,
            suggestions_revision: None,
            git_baseline_commit: None,
            git_result_commit: None,
            documents_cataloged: 0,
            ambiguous_documents: 0,
            documents_read: 0,
            estimated_tokens_used: 0,
            events: Vec::new(),
            error: None,
        };
        push_event(
            &mut trace,
            "requested",
            "pending",
            "整理请求已创建",
            "等待 AI 任务连接项目文档 MCP。",
        );
        write_trace_unlocked(&workspace, &trace)?;
        serde_json::to_value(trace).context("序列化文档整理状态失败")
    })
}

pub(crate) fn mark_session_ready(workspace: &Path, session_id: &str) -> Result<String> {
    let workspace = validate_workspace(workspace)?;
    with_trace_lock(|| {
        let mut trace = read_trace_unlocked(&workspace)?.unwrap_or_else(new_direct_trace);
        trace.session_id = Some(session_id.to_string());
        if can_advance_work(&trace) {
            advance(
                &mut trace,
                "session_ready",
                "running",
                "MCP 会话已就绪",
                "AI 可直接调用供应商无关的项目文档工具。",
            );
        }
        let operation_id = trace.operation_id.clone();
        write_trace_unlocked(&workspace, &trace)?;
        Ok(operation_id)
    })
}

pub(crate) fn get_status(workspace: &Path, operation_id: Option<&str>) -> Result<Value> {
    let workspace = validate_workspace(workspace)?;
    with_trace_lock(|| {
        let trace = read_trace_unlocked(&workspace)?
            .ok_or_else(|| anyhow::anyhow!("当前项目还没有文档整理运行记录"))?;
        verify_operation(&trace, operation_id)?;
        serde_json::to_value(trace).context("序列化文档整理状态失败")
    })
}

pub(crate) fn record_tool_success(workspace: &Path, tool: &str, value: &Value) {
    let Ok(workspace) = validate_workspace(workspace) else {
        return;
    };
    let _ = with_trace_lock(|| {
        let mut trace = read_trace_unlocked(&workspace)?.unwrap_or_else(new_direct_trace);
        match tool {
            "project_docs_analyze" => {
                trace.catalog_revision = string_field(value, "catalog_revision");
                trace.documents_cataloged = value["pagination"]["matching_documents"]
                    .as_u64()
                    .unwrap_or_default();
                trace.ambiguous_documents = value["budget"]["ambiguous_documents"]
                    .as_u64()
                    .unwrap_or_default();
                if can_advance_work(&trace) {
                    advance(
                        &mut trace,
                        "catalog_analyzed",
                        "running",
                        "目录分析完成",
                        "只读取了路径、标题、标题层级、哈希和生命周期元数据。",
                    );
                }
            }
            "project_docs_read" => {
                trace.documents_read = trace
                    .documents_read
                    .saturating_add(value["documents_read"].as_u64().unwrap_or_default());
                trace.estimated_tokens_used = trace.estimated_tokens_used.saturating_add(
                    value["estimated_tokens_returned"]
                        .as_u64()
                        .unwrap_or_default(),
                );
                if can_advance_work(&trace) {
                    advance(
                        &mut trace,
                        "documents_read",
                        "running",
                        "按需正文读取完成",
                        "只读取了 AI 明确选择的歧义或任务相关文档。",
                    );
                }
            }
            "project_docs_read_sections" => {
                trace.documents_read = trace
                    .documents_read
                    .saturating_add(value["sections_read"].as_u64().unwrap_or_default());
                trace.estimated_tokens_used = trace.estimated_tokens_used.saturating_add(
                    value["estimated_tokens_returned"]
                        .as_u64()
                        .unwrap_or_default(),
                );
                if can_advance_work(&trace) {
                    advance(
                        &mut trace,
                        "document_sections_read",
                        "running",
                        "文档章节已按需读取",
                        "只读取了任务指定的标题范围，没有展开整份大文档。",
                    );
                }
            }
            "project_docs_get_map"
            | "project_docs_get_node"
            | "project_docs_review_map"
            | "project_docs_plan_context"
            | "project_docs_review_modularity"
            | "project_docs_test_retrieval"
                if can_advance_work(&trace) =>
            {
                advance(
                    &mut trace,
                    "knowledge_map_inspected",
                    "running",
                    "项目知识图谱已检查",
                    "只读取图谱、目录和实现引用元数据，没有读取 Markdown 正文。",
                )
            }
            "project_docs_get_map"
            | "project_docs_get_node"
            | "project_docs_review_map"
            | "project_docs_plan_context"
            | "project_docs_review_modularity"
            | "project_docs_test_retrieval" => {}
            "project_docs_get_suggestions" if can_advance_work(&trace) => advance(
                &mut trace,
                "suggestions_checked",
                "running",
                "现有建议已检查",
                "已读取建议 revision，没有读取 Markdown 正文。",
            ),
            "project_docs_get_suggestions" => {}
            "project_discussions_read_source_chunk" => {
                trace.documents_read = trace.documents_read.saturating_add(1);
                trace.estimated_tokens_used = trace.estimated_tokens_used.saturating_add(
                    value["budget"]["estimated_model_tokens"]
                        .as_u64()
                        .unwrap_or_default(),
                );
                if can_advance_work(&trace) {
                    advance(
                        &mut trace,
                        "discussion_source_chunk_read",
                        "running",
                        "聊天来源已按块读取",
                        "只读取当前来源的一个稳定 chunk；已记录本次返回字符数和估算 token。",
                    );
                }
            }
            "project_discussions_get_graph"
            | "project_discussions_get_node"
            | "project_discussions_get_history"
            | "project_discussions_get_graph_at_version"
            | "project_discussions_compare_versions"
            | "project_discussions_trace_node"
            | "project_discussions_review_graph"
            | "project_discussions_prepare_safe_repair"
            | "project_discussions_get_source_manifest"
            | "project_discussions_get_suggestions"
                if can_advance_work(&trace) =>
            {
                advance(
                    &mut trace,
                    "discussion_graph_inspected",
                    "running",
                    "讨论推理图已检查",
                    "只读取讨论节点、来源锚点和关系元数据；没有展开原始聊天正文。",
                )
            }
            "project_discussions_get_graph"
            | "project_discussions_get_node"
            | "project_discussions_get_history"
            | "project_discussions_get_graph_at_version"
            | "project_discussions_compare_versions"
            | "project_discussions_trace_node"
            | "project_discussions_review_graph"
            | "project_discussions_prepare_safe_repair"
            | "project_discussions_get_source_manifest"
            | "project_discussions_get_suggestions" => {}
            "project_discussions_save_proposal" => {
                trace.suggestions_revision = string_field(value, "suggestions_revision");
                match value["authorization_mode"]
                    .as_str()
                    .unwrap_or("git_backed_full")
                {
                    "review_all" => advance(
                        &mut trace,
                        "awaiting_discussion_review",
                        "awaiting_review",
                        "讨论图建议已生成",
                        "等待用户审核节点、分支和拟晋升文档。",
                    ),
                    "suggestions_only" => advance(
                        &mut trace,
                        "discussion_suggestions_saved",
                        "succeeded",
                        "讨论图建议已生成",
                        "当前为仅建议模式，没有修改讨论图或创建文档。",
                    ),
                    _ => advance(
                        &mut trace,
                        "discussion_suggestions_ready",
                        "running",
                        "讨论图建议已生成",
                        "来源、节点、分支和晋升建议已校验，继续执行可逆应用。",
                    ),
                }
            }
            "project_discussions_apply" => {
                trace.discussion_graph_revision = string_field(value, "graph_revision");
                trace.suggestions_revision = string_field(value, "suggestions_revision");
                trace.git_baseline_commit = string_field(value, "git_baseline_commit");
                trace.git_result_commit = string_field(value, "git_result_commit");
                advance(
                    &mut trace,
                    "discussion_graph_applied",
                    "succeeded",
                    "讨论推理图已应用",
                    "讨论节点和分支已更新；确认晋升的文档已创建，并保存可用的版本记录。",
                );
            }
            "project_docs_save_suggestions" => {
                trace.catalog_revision = string_field(value, "catalog_revision");
                trace.suggestions_revision = string_field(value, "suggestions_revision");
                if let Some(suggestions) = value.get("suggestions") {
                    trace.documents_read = suggestions["documents_read"]
                        .as_u64()
                        .unwrap_or(trace.documents_read);
                    trace.estimated_tokens_used = suggestions["estimated_tokens_used"]
                        .as_u64()
                        .unwrap_or(trace.estimated_tokens_used);
                }
                match value["authorization_mode"]
                    .as_str()
                    .unwrap_or("git_backed_full")
                {
                    "review_all" => advance(
                        &mut trace,
                        "awaiting_review",
                        "awaiting_review",
                        "整理建议已生成",
                        "当前为逐项审核模式，等待用户确认应用。",
                    ),
                    "suggestions_only" => advance(
                        &mut trace,
                        "suggestions_saved",
                        "succeeded",
                        "整理建议已生成",
                        "当前为仅建议模式，没有应用任何分区或文件操作。",
                    ),
                    _ => advance(
                        &mut trace,
                        "suggestions_ready",
                        "running",
                        "整理建议已生成",
                        "默认 Git 备份后完全整理权限已开放，继续应用文档操作。",
                    ),
                }
            }
            "project_docs_apply_suggestions" => {
                trace.manifest_revision = string_field(value, "manifest_revision");
                trace.suggestions_revision = string_field(value, "suggestions_revision");
                trace.git_baseline_commit = string_field(value, "git_baseline_commit");
                trace.git_result_commit = string_field(value, "git_result_commit");
                let pending_files = value["suggestions"]["file_operations"]
                    .as_array()
                    .is_some_and(|operations| {
                        operations
                            .iter()
                            .any(|operation| operation["status"] == "proposed")
                    });
                advance(
                    &mut trace,
                    if pending_files {
                        "virtual_applied"
                    } else {
                        "applied"
                    },
                    if pending_files {
                        "running"
                    } else {
                        "succeeded"
                    },
                    "分区建议已应用",
                    if pending_files {
                        "整理前 Git 基线已保存，虚拟分区已更新；继续执行 Markdown 路径操作。"
                    } else {
                        "虚拟分区已更新，整理前和整理后 Git 提交均已保存。"
                    },
                );
            }
            "project_docs_apply_file_operations" => {
                trace.catalog_revision = string_field(value, "catalog_revision");
                trace.manifest_revision = string_field(value, "manifest_revision");
                trace.suggestions_revision = string_field(value, "suggestions_revision");
                trace.git_baseline_commit = string_field(value, "git_baseline_commit");
                trace.git_result_commit = string_field(value, "git_result_commit");
                advance(
                    &mut trace,
                    "files_applied",
                    "succeeded",
                    "安全文件整理已应用",
                    "已完成 Markdown 重命名/移动，并保存整理前、整理后两个仅文档 Git 提交。",
                );
            }
            _ => return Ok(()),
        }
        write_trace_unlocked(&workspace, &trace)
    });
}

pub(crate) fn record_tool_failure(workspace: &Path, tool: &str, error: &anyhow::Error) {
    let recovery = match tool {
        "project_docs_analyze" => "确认项目目录存在且是 Git 工作区，然后重新分析。",
        "project_docs_read" | "project_docs_read_sections" => {
            "重新 analyze 获取最新目录 revision，只读取目录中存在的路径和标题。"
        }
        "project_docs_get_map"
        | "project_docs_get_node"
        | "project_docs_review_map"
        | "project_docs_plan_context"
        | "project_docs_review_modularity"
        | "project_docs_test_retrieval" => {
            "重新获取 overview 图，确认 view、node_id 和 token 预算后重试。"
        }
        "project_docs_save_suggestions" => {
            "重新 analyze，合并最新建议 revision，并移除不存在的路径或未知分区。"
        }
        "project_docs_apply_suggestions" => {
            "确认权限模式并刷新建议；若 revision 已变化，重新 analyze 后再应用。"
        }
        "project_docs_apply_file_operations" => {
            "重新 analyze 获取最新文件哈希和目录 revision，确认目标未占用后重试。"
        }
        "project_discussions_get_graph"
        | "project_discussions_get_node"
        | "project_discussions_get_history"
        | "project_discussions_get_graph_at_version"
        | "project_discussions_compare_versions"
        | "project_discussions_trace_node"
        | "project_discussions_review_graph"
        | "project_discussions_prepare_safe_repair"
        | "project_discussions_get_source_manifest"
        | "project_discussions_get_suggestions" => {
            "刷新讨论图 revision，确认提交仍在当前分支历史且 root_id 或 node_id 存在后重试。"
        }
        "project_discussions_read_source_chunk" => {
            "重新获取来源 manifest，使用最新 source revision 和其中存在的 chunk id 重试。"
        }
        "project_discussions_save_proposal" => {
            "重新读取讨论图和建议 revision，修正未知来源、循环父子关系或无效晋升目标后重试。"
        }
        "project_discussions_apply" => {
            "重新读取讨论图建议；确认权限、revision 和晋升文档目标未被占用后重试。"
        }
        _ => "查看错误详情并重试当前步骤。",
    };
    let code = format!(
        "{}_failed",
        tool.trim_start_matches("project_docs_")
            .trim_start_matches("project_discussions_")
    );
    let _ = mark_failure(workspace, None, &code, &format!("{error:#}"), recovery);
}

pub(crate) fn mark_dispatched(
    workspace: &Path,
    operation_id: Option<&str>,
    task_id: Option<&str>,
) -> Result<Value> {
    update_trace(workspace, operation_id, |trace| {
        trace.task_id = task_id
            .filter(|value| !value.trim().is_empty())
            .map(str::to_string);
        if can_advance_work(trace) {
            advance(
                trace,
                "task_dispatched",
                "running",
                "AI 整理任务已发送",
                "等待所选 AI 供应商连接项目文档 MCP。",
            );
        }
    })
}

pub(crate) fn mark_applied(
    workspace: &Path,
    operation_id: Option<&str>,
    manifest_revision: Option<&str>,
    suggestions_revision: Option<&str>,
) -> Result<Value> {
    update_trace(workspace, operation_id, |trace| {
        trace.manifest_revision = manifest_revision.map(str::to_string);
        trace.suggestions_revision = suggestions_revision.map(str::to_string);
        advance(
            trace,
            "applied",
            "succeeded",
            "分区建议已应用",
            "PC 审核接口已更新虚拟分区；Markdown 未移动、删除或改写。",
        );
    })
}

pub(crate) fn mark_failure(
    workspace: &Path,
    operation_id: Option<&str>,
    code: &str,
    message: &str,
    recovery: &str,
) -> Result<Value> {
    update_trace(workspace, operation_id, |trace| {
        let now = unix_seconds();
        trace.status = "failed".to_string();
        trace.current_stage = "failed".to_string();
        trace.updated_at = now;
        trace.error = Some(DocumentOrganizationError {
            code: clean_bounded(code, 80),
            message: clean_bounded(message, 2_000),
            recovery: clean_bounded(recovery, 1_000),
            at: now,
        });
        push_event(trace, "failed", "failed", "整理流程在当前步骤失败", message);
    })
}

fn update_trace(
    workspace: &Path,
    operation_id: Option<&str>,
    update: impl FnOnce(&mut DocumentOrganizationTrace),
) -> Result<Value> {
    let workspace = validate_workspace(workspace)?;
    with_trace_lock(|| {
        let mut trace = read_trace_unlocked(&workspace)?
            .ok_or_else(|| anyhow::anyhow!("当前项目还没有文档整理运行记录"))?;
        verify_operation(&trace, operation_id)?;
        update(&mut trace);
        write_trace_unlocked(&workspace, &trace)?;
        serde_json::to_value(trace).context("序列化文档整理状态失败")
    })
}

fn advance(
    trace: &mut DocumentOrganizationTrace,
    stage: &str,
    status: &str,
    label: &str,
    detail: &str,
) {
    trace.current_stage = stage.to_string();
    trace.status = status.to_string();
    trace.updated_at = unix_seconds();
    trace.error = None;
    push_event(trace, stage, status, label, detail);
}

fn can_advance_work(trace: &DocumentOrganizationTrace) -> bool {
    !matches!(trace.status.as_str(), "awaiting_review" | "succeeded")
}

fn push_event(
    trace: &mut DocumentOrganizationTrace,
    stage: &str,
    status: &str,
    label: &str,
    detail: &str,
) {
    let event = DocumentOrganizationEvent {
        stage: stage.to_string(),
        status: status.to_string(),
        label: clean_bounded(label, 120),
        detail: clean_bounded(detail, 1_000),
        at: unix_seconds(),
    };
    if trace
        .events
        .last()
        .is_some_and(|last| last.stage == event.stage && last.status == event.status)
    {
        trace.events.pop();
    }
    trace.events.push(event);
    if trace.events.len() > MAX_EVENTS {
        trace.events.drain(..trace.events.len() - MAX_EVENTS);
    }
}

fn new_direct_trace() -> DocumentOrganizationTrace {
    let now = unix_seconds();
    DocumentOrganizationTrace {
        version: TRACE_VERSION,
        operation_id: format!("mcp_{}", uuid::Uuid::new_v4().simple()),
        status: "pending".to_string(),
        current_stage: "requested".to_string(),
        created_at: now,
        updated_at: now,
        task_id: None,
        session_id: None,
        catalog_revision: None,
        manifest_revision: None,
        discussion_graph_revision: None,
        suggestions_revision: None,
        git_baseline_commit: None,
        git_result_commit: None,
        documents_cataloged: 0,
        ambiguous_documents: 0,
        documents_read: 0,
        estimated_tokens_used: 0,
        events: Vec::new(),
        error: None,
    }
}

fn validate_workspace(workspace: &Path) -> Result<PathBuf> {
    let root = workspace
        .canonicalize()
        .context("projectRoot 不存在或不可访问")?;
    if !root.is_dir() || !root.join(".git").exists() {
        bail!("projectRoot 必须是现存 Git 工作区");
    }
    Ok(root)
}

fn normalize_operation_id(value: &str) -> Result<String> {
    let value = if value.trim().is_empty() {
        format!("docs_{}", uuid::Uuid::new_v4().simple())
    } else {
        value.trim().to_string()
    };
    if value.len() > 96
        || !value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-'))
    {
        bail!("operation_id 格式无效");
    }
    Ok(value)
}

fn verify_operation(trace: &DocumentOrganizationTrace, expected: Option<&str>) -> Result<()> {
    if expected
        .filter(|value| !value.trim().is_empty())
        .is_some_and(|value| value != trace.operation_id)
    {
        bail!("该文档整理运行已被更新的操作替代");
    }
    Ok(())
}

fn trace_path(workspace: &Path) -> PathBuf {
    let mut key = workspace.to_string_lossy().replace('\\', "/");
    if cfg!(windows) {
        key = key.to_ascii_lowercase();
    }
    let hash = format!("{:x}", Sha256::digest(key.as_bytes()));
    std::env::temp_dir()
        .join("elon-project-docs-organization")
        .join(format!("{hash}.json"))
}

fn read_trace_unlocked(workspace: &Path) -> Result<Option<DocumentOrganizationTrace>> {
    let path = trace_path(workspace);
    if !path.is_file() {
        return Ok(None);
    }
    let bytes =
        fs::read(&path).with_context(|| format!("读取文档整理状态失败：{}", path.display()))?;
    serde_json::from_slice(&bytes)
        .map(Some)
        .with_context(|| format!("文档整理状态损坏：{}", path.display()))
}

fn write_trace_unlocked(workspace: &Path, trace: &DocumentOrganizationTrace) -> Result<()> {
    let path = trace_path(workspace);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let bytes = serde_json::to_vec_pretty(trace)?;
    crate::node_agent_atomic_file::write(&path, &bytes)
        .with_context(|| format!("写入文档整理状态失败：{}", path.display()))
}

fn with_trace_lock<T>(operation: impl FnOnce() -> Result<T>) -> Result<T> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    let _guard = LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    operation()
}

fn string_field(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
}

fn clean_bounded(value: &str, limit: usize) -> String {
    value.trim().chars().take(limit).collect()
}

fn unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

#[cfg(test)]
#[path = "project_document_observability_tests.rs"]
mod tests;
