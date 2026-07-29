//! Direct, provider-neutral import of long conversations as low-authority sources.

use anyhow::{anyhow, bail, Result};
use chrono::Utc;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::Path;

use crate::{
    project_discussion_source_normalizer::{normalize_conversation, NormalizedConversation},
    project_document_authorization::{authorize_document_apply, DocumentAutomationMode},
    project_document_files::{read_project_document_file, write_project_document_file},
    project_document_git_transaction::{commit_document_baseline, commit_document_result},
    project_document_vault::{current_version, is_managed_vault},
};

const SOURCE_DIRECTORY: &str = "docs/inbox/conversations";
const MAX_SOURCE_BYTES: usize = 2 * 1024 * 1024;

pub(crate) fn import_conversation_source(
    workspace: &Path,
    title: &str,
    body: &str,
    source_reference: &str,
    suggested_filename: &str,
    authorization_mode: DocumentAutomationMode,
    reviewed: bool,
) -> Result<Value> {
    let authorization = authorize_document_apply(authorization_mode, reviewed)?;
    let title = single_line(title, 160);
    let source_reference = single_line(source_reference, 1_000);
    let body = body.trim_start_matches('\u{feff}').trim();
    if title.is_empty() {
        bail!("导入聊天必须提供标题");
    }
    if body.is_empty() {
        bail!("导入聊天正文不能为空");
    }
    if body.len() > MAX_SOURCE_BYTES {
        bail!("导入聊天正文超过 2 MiB");
    }

    let normalized = normalize_conversation(body)?;
    let content = source_markdown(&title, &normalized, &source_reference)?;
    let target = choose_target_path(workspace, &content, suggested_filename)?;
    if let Ok(existing) = read_project_document_file(workspace, &target.path) {
        if existing.content == content {
            return Ok(json!({
                "status": "imported",
                "already_imported": true,
                "path": target.path,
                "revision": existing.revision,
                "source_bytes": body.len(),
                "source_id": normalized.source_id,
                "source_revision": normalized.content_revision,
                "source_format": normalized.format,
                "message_count": normalized.message_count,
                "authorization_mode": authorization.mode,
                "auto_authorized": authorization.auto_authorized,
                "git_document_transaction_complete": true,
                "budget": metadata_budget(),
            }));
        }
    }

    let managed = is_managed_vault(workspace);
    let pre_commit = managed.then(|| current_version(workspace)).transpose()?;
    let baseline = if managed {
        pre_commit
    } else {
        Some(commit_document_baseline(workspace)?)
    };
    let saved = write_project_document_file(workspace, &target.path, &content, None)
        .map_err(|error| anyhow!(error.message))?;
    let result_commit = if managed {
        Some(current_version(workspace)?)
    } else {
        baseline
            .as_deref()
            .map(|commit| commit_document_result(workspace, commit))
            .transpose()?
    };
    Ok(json!({
        "status": "imported",
        "already_imported": false,
        "path": target.path,
        "revision": saved.revision,
        "source_bytes": body.len(),
        "source_id": normalized.source_id,
        "source_revision": normalized.content_revision,
        "source_format": normalized.format,
        "message_count": normalized.message_count,
        "authorization_mode": authorization.mode,
        "auto_authorized": authorization.auto_authorized,
        "git_baseline_commit": baseline,
        "git_result_commit": result_commit,
        "git_document_transaction_complete": result_commit.is_some(),
        "next_action": "调用 project_discussions_get_graph，再按路径 plan_context/read，并保存增量讨论图 proposal。",
        "budget": metadata_budget(),
    }))
}

struct TargetPath {
    path: String,
}

fn choose_target_path(
    workspace: &Path,
    content: &str,
    suggested_filename: &str,
) -> Result<TargetPath> {
    let digest = format!("{:x}", Sha256::digest(content.as_bytes()));
    let normalized_filename = suggested_filename.trim().replace('\\', "/");
    let suggested = normalized_filename
        .rsplit('/')
        .next()
        .unwrap_or_default()
        .trim_end_matches(".md")
        .trim_end_matches(".markdown")
        .trim_end_matches(".mdx");
    let slug = ascii_slug(suggested);
    let slug = if slug.is_empty() {
        format!("conversation-{}", &digest[..12])
    } else {
        slug
    };
    let date = Utc::now().format("%Y-%m-%d");
    let base = format!("{date}-{slug}");
    for suffix in 1..=1_000 {
        let name = if suffix == 1 {
            base.clone()
        } else {
            format!("{base}-{suffix}")
        };
        let path = format!("{SOURCE_DIRECTORY}/{name}.md");
        match read_project_document_file(workspace, &path) {
            Ok(existing) if existing.content == content => return Ok(TargetPath { path }),
            Ok(_) => {}
            Err(_) => return Ok(TargetPath { path }),
        }
    }
    bail!("无法为导入聊天分配不冲突的文件名")
}

fn source_markdown(
    title: &str,
    source: &NormalizedConversation,
    source_reference: &str,
) -> Result<String> {
    let title_yaml = serde_json::to_string(title)?;
    let source_yaml = serde_json::to_string(source_reference)?;
    let source_id_yaml = serde_json::to_string(&source.source_id)?;
    let source_revision_yaml = serde_json::to_string(&source.content_revision)?;
    let source_format_yaml = serde_json::to_string(&source.format)?;
    let reviewed_at = Utc::now().format("%Y-%m-%d");
    let heading = if source
        .body
        .lines()
        .any(|line| line.trim_start().starts_with("# "))
    {
        String::new()
    } else {
        format!("# {title}\n\n")
    };
    Ok(format!(
        "---\n\
title: {title_yaml}\n\
role: discussion\n\
lifecycle: source_material\n\
authority: none\n\
default_retrieval: false\n\
owner: user\n\
reviewed_at: {reviewed_at}\n\
source_type: imported_conversation\n\
source_reference: {source_yaml}\n\
source_id: {source_id_yaml}\n\
source_revision: {source_revision_yaml}\n\
source_format: {source_format_yaml}\n\
source_message_count: {}\n\
---\n\n\
> 本文是导入的原始聊天来源，不是当前项目事实；稳定结论必须经讨论图审查后晋升。\n\n\
{heading}{}\n",
        source.message_count, source.body
    ))
}

fn ascii_slug(value: &str) -> String {
    let mut output = String::new();
    let mut separator = false;
    for character in value.to_ascii_lowercase().chars() {
        if character.is_ascii_alphanumeric() {
            output.push(character);
            separator = false;
        } else if !output.is_empty() && !separator {
            output.push('-');
            separator = true;
        }
    }
    output.trim_matches('-').chars().take(80).collect()
}

fn single_line(value: &str, limit: usize) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(limit)
        .collect()
}

fn metadata_budget() -> Value {
    json!({
        "classification_model_tokens": 0,
        "chat_bodies_read": 0,
        "document_bodies_read": 0,
        "metadata_only": false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_markdown_uses_fixed_non_authoritative_facets() {
        let source = normalize_conversation("用户：为什么？").unwrap();
        let markdown = source_markdown("产品讨论", &source, "chat://one").unwrap();
        assert!(markdown.contains("role: discussion"));
        assert!(markdown.contains("lifecycle: source_material"));
        assert!(markdown.contains("authority: none"));
        assert!(markdown.contains("default_retrieval: false"));
        assert!(markdown.contains("source_id:"));
        assert!(markdown.contains("source_revision:"));
        assert!(markdown.contains("# 产品讨论"));
    }

    #[test]
    fn filename_slug_never_keeps_a_path_or_extension() {
        assert_eq!(ascii_slug("../My Chat.md"), "my-chat-md");
        assert_eq!(ascii_slug("商户数据"), "");
    }
}
