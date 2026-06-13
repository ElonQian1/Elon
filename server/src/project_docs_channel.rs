//! Server-side adapter for the fixed project documentation channel.
use chrono::Utc;
use homecli_proto::{ProjectDocumentEntry, ProjectDocumentsSnapshot};
use std::sync::Arc;

use crate::{
    project_docs_snapshot::load_project_documents_snapshot,
    store::{ProjectAccess, ProjectChannelMessage},
    types::AppState,
};

pub(crate) async fn load_project_doc_messages(
    state: Arc<AppState>,
    user_id: &str,
    project: &ProjectAccess,
    channel_id: &str,
) -> Vec<ProjectChannelMessage> {
    let snapshot = load_project_documents_snapshot(&state, project).await;
    snapshot_to_messages(&project.id, channel_id, user_id, snapshot)
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
        "# 项目文档频道\n\n工作区：`{}`\n\n来源：`{}`\n\nRevision：`{}`\n\n这里固定汇总当前项目内的 AI 代理入口、说明文档、GitHub Copilot 指令、按需加载指令和 docs Markdown。AI 开发前也会优先读取这些项目文档。",
        snapshot.workspace_path, snapshot.source, snapshot.revision
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
    let source = if doc.source.is_empty() {
        String::new()
    } else {
        format!("\n\n来源：`{}`", doc.source)
    };
    let mut text = format!(
        "## {}\n\n路径：`{}`{}\n\n{}",
        doc.title, doc.path, source, doc.content
    );
    if doc.truncated {
        text.push_str("\n\n[文档较长，频道中已显示前半部分；AI 执行任务时仍可按需读取完整文件。]");
    }
    text
}
