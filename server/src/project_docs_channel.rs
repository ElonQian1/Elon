//! Server-side adapter for the fixed project documentation channel.
use chrono::Utc;
use homecli_proto::{AgentToServer, ProjectDocumentEntry, ProjectDocumentsSnapshot};
use std::{path::Path, sync::Arc};

use crate::{
    project_docs_scan::collect_project_documents,
    store::{ProjectAccess, ProjectChannelMessage},
    types::AppState,
};

pub(crate) async fn load_project_doc_messages(
    state: Arc<AppState>,
    user_id: &str,
    project: &ProjectAccess,
    channel_id: &str,
) -> Vec<ProjectChannelMessage> {
    let workspace =
        state.resolve_project_workspace(&project.workspace_key, project.workspace_path.as_deref());
    let snapshot = read_project_documents(&state, project, &workspace).await;
    snapshot_to_messages(&project.id, channel_id, user_id, snapshot)
}

async fn read_project_documents(
    state: &AppState,
    project: &ProjectAccess,
    workspace: &Path,
) -> ProjectDocumentsSnapshot {
    if let (Some(node_id), Some(workspace_path)) = (
        project
            .node_id
            .as_deref()
            .filter(|value| !value.trim().is_empty()),
        project
            .workspace_path
            .as_deref()
            .filter(|value| !value.trim().is_empty()),
    ) {
        match state
            .agent_manager
            .dispatch_project_documents_read(node_id, workspace_path.to_string())
            .await
        {
            Ok(AgentToServer::ProjectDocumentsRead { snapshot, .. }) => return snapshot,
            Ok(AgentToServer::ProjectDocumentsReadError { message, .. }) => {
                return fallback_snapshot(workspace, format!("PC 节点读取文档失败：{message}"));
            }
            Ok(other) => {
                return fallback_snapshot(workspace, format!("PC 节点返回了非文档响应：{other:?}"));
            }
            Err(error) => {
                return fallback_snapshot(workspace, format!("PC 节点暂不可读取文档：{error}"));
            }
        }
    }

    collect_project_documents(workspace).unwrap_or_else(|error| ProjectDocumentsSnapshot {
        workspace_path: workspace.to_string_lossy().to_string(),
        documents: Vec::new(),
        warnings: vec![format!("读取项目文档失败：{error}")],
    })
}

fn fallback_snapshot(workspace: &Path, warning: String) -> ProjectDocumentsSnapshot {
    let mut snapshot =
        collect_project_documents(workspace).unwrap_or_else(|error| ProjectDocumentsSnapshot {
            workspace_path: workspace.to_string_lossy().to_string(),
            documents: Vec::new(),
            warnings: vec![format!("服务器本地读取也失败：{error}")],
        });
    snapshot.warnings.insert(0, warning);
    snapshot
}

fn snapshot_to_messages(
    project_id: &str,
    channel_id: &str,
    _user_id: &str,
    snapshot: ProjectDocumentsSnapshot,
) -> Vec<ProjectChannelMessage> {
    let created_at = Utc::now().to_rfc3339();
    let mut messages = Vec::new();
    messages.push(synthetic_message(
        "project-docs-summary",
        project_id,
        channel_id,
        format_summary(&snapshot),
        &created_at,
    ));
    for (index, doc) in snapshot.documents.into_iter().enumerate() {
        messages.push(synthetic_message(
            &format!("project-docs-{index}"),
            project_id,
            channel_id,
            format_document(&doc),
            &created_at,
        ));
    }
    messages
}

fn synthetic_message(
    id: &str,
    project_id: &str,
    channel_id: &str,
    content: String,
    created_at: &str,
) -> ProjectChannelMessage {
    ProjectChannelMessage {
        id: id.to_string(),
        project_id: project_id.to_string(),
        channel_id: channel_id.to_string(),
        sender_user_id: None,
        sender_name: Some("项目文档".to_string()),
        sender_avatar_data_url: None,
        reply_to_message_id: None,
        kind: "system".to_string(),
        content,
        task_id: None,
        suggestion_status: None,
        suggestion_resolved_by: None,
        suggestion_resolved_by_name: None,
        suggestion_resolved_at: None,
        created_at: created_at.to_string(),
        outgoing: false,
    }
}

fn format_summary(snapshot: &ProjectDocumentsSnapshot) -> String {
    let mut text = format!(
        "# 项目文档频道\n\n工作区：`{}`\n\n这里固定汇总当前项目内的 AI 代理入口、说明文档、GitHub Copilot 指令、按需加载指令和 docs Markdown。AI 开发前也会优先读取这些项目文档。",
        snapshot.workspace_path
    );
    if !snapshot.warnings.is_empty() {
        text.push_str("\n\n## 注意\n");
        for warning in &snapshot.warnings {
            text.push_str("- ");
            text.push_str(warning);
            text.push('\n');
        }
    }
    if snapshot.documents.is_empty() {
        text.push_str("\n\n未发现可展示的固定文档。");
    }
    text
}

fn format_document(doc: &ProjectDocumentEntry) -> String {
    let mut text = format!(
        "## {}\n\n路径：`{}`\n\n{}",
        doc.title, doc.path, doc.content
    );
    if doc.truncated {
        text.push_str("\n\n[文档较长，频道中已显示前半部分；AI 执行任务时仍可按需读取完整文件。]");
    }
    text
}
