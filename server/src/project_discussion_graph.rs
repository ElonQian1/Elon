//! Portable, Git-backed discussion graph compiled from long conversations.

use anyhow::{anyhow, bail, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::Path;

use crate::{
    project_discussion_graph_model::{
        DiscussionGraph, DiscussionGraphEvolution, DiscussionGraphProposal, DiscussionPromotion,
        Versioned, DISCUSSION_GRAPH_PATH, DISCUSSION_SUGGESTIONS_PATH,
    },
    project_discussion_graph_validation::{
        counts, merge_graph, normalize_graph, normalize_proposal, validate_promotions,
    },
    project_document_authorization::{authorize_document_apply, DocumentAutomationMode},
    project_document_files::{read_project_document_file, write_project_document_file},
    project_document_git_transaction::{commit_document_baseline, commit_document_result},
    project_document_vault::{current_version, is_managed_vault},
};

pub(crate) fn load_graph(workspace: &Path) -> Result<Versioned<DiscussionGraph>> {
    load_optional(workspace, DISCUSSION_GRAPH_PATH, normalize_graph)
}

pub(crate) fn load_proposal(
    workspace: &Path,
) -> Result<Versioned<Option<DiscussionGraphProposal>>> {
    let Some(file) = read_optional(workspace, DISCUSSION_SUGGESTIONS_PATH)? else {
        return Ok(Versioned {
            value: None,
            revision: None,
        });
    };
    let proposal = normalize_proposal(serde_json::from_str(&file.content)?)?;
    Ok(Versioned {
        value: Some(proposal),
        revision: Some(file.revision),
    })
}

pub(crate) fn save_proposal(
    workspace: &Path,
    proposal: DiscussionGraphProposal,
    authorization_mode: DocumentAutomationMode,
    expected_graph_revision: Option<&str>,
    expected_proposal_revision: Option<&str>,
) -> Result<Value> {
    let graph = load_graph(workspace)?;
    verify_revision("讨论图", graph.revision.as_deref(), expected_graph_revision)?;
    let previous = load_proposal(workspace)?;
    verify_revision(
        "讨论图建议",
        previous.revision.as_deref(),
        expected_proposal_revision,
    )?;
    let mut proposal = proposal;
    proposal.graph = merge_graph(graph.value.clone(), proposal.graph)?;
    let proposal = normalize_proposal(proposal)?;
    validate_promotions(workspace, &proposal)?;
    let content = pretty(&proposal)?;
    let saved = write_project_document_file(
        workspace,
        DISCUSSION_SUGGESTIONS_PATH,
        &content,
        expected_proposal_revision,
    )
    .map_err(|error| anyhow!(error.message))?;
    Ok(json!({
        "status": "ready",
        "graph_revision": graph.revision,
        "suggestions_revision": saved.revision,
        "counts": counts(&proposal.graph, proposal.promotions.len()),
        "authorization_mode": authorization_mode,
        "requires_user_review": authorization_mode == DocumentAutomationMode::ReviewAll,
        "apply_allowed": authorization_mode != DocumentAutomationMode::SuggestionsOnly,
    }))
}

pub(crate) fn apply_proposal(
    workspace: &Path,
    authorization_mode: DocumentAutomationMode,
    reviewed: bool,
    expected_graph_revision: Option<&str>,
    expected_proposal_revision: Option<&str>,
) -> Result<Value> {
    let authorization = authorize_document_apply(authorization_mode, reviewed)?;
    let current = load_graph(workspace)?;
    verify_revision(
        "讨论图",
        current.revision.as_deref(),
        expected_graph_revision,
    )?;
    let proposal_file = load_proposal(workspace)?;
    verify_revision(
        "讨论图建议",
        proposal_file.revision.as_deref(),
        expected_proposal_revision,
    )?;
    let mut proposal = proposal_file
        .value
        .ok_or_else(|| anyhow!("项目尚未生成讨论图建议"))?;
    if proposal.status == "applied" {
        return Ok(json!({
            "status": "applied",
            "already_applied": true,
            "graph_revision": current.revision,
            "suggestions_revision": proposal_file.revision,
            "counts": counts(&current.value, proposal.promotions.len()),
        }));
    }
    validate_promotions(workspace, &proposal)?;
    let previous_revision = current.revision.clone().unwrap_or_default();
    let mut merged = merge_graph(current.value, proposal.graph.clone())?;
    merged.evolution = DiscussionGraphEvolution {
        kind: proposal.change_kind.clone(),
        summary: proposal.summary.clone(),
        actor: proposal.actor.clone(),
        changed_at: chrono::Utc::now().to_rfc3339(),
        previous_revision,
    };
    let managed = is_managed_vault(workspace);
    let pre_commit = managed.then(|| current_version(workspace)).transpose()?;
    let baseline = if managed {
        pre_commit.clone()
    } else {
        Some(commit_document_baseline(workspace)?)
    };
    let promoted = write_promotions(workspace, &proposal.promotions)?;
    let graph_saved = write_project_document_file(
        workspace,
        DISCUSSION_GRAPH_PATH,
        &pretty(&merged)?,
        expected_graph_revision,
    )
    .map_err(|error| anyhow!(error.message))?;
    proposal.status = "applied".to_string();
    let proposal_saved = write_project_document_file(
        workspace,
        DISCUSSION_SUGGESTIONS_PATH,
        &pretty(&proposal)?,
        expected_proposal_revision,
    )
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
        "status": "applied",
        "already_applied": false,
        "graph_revision": graph_saved.revision,
        "suggestions_revision": proposal_saved.revision,
        "counts": counts(&merged, proposal.promotions.len()),
        "promoted_documents": promoted,
        "authorization_mode": authorization.mode,
        "auto_authorized": authorization.auto_authorized,
        "git_baseline_commit": baseline,
        "git_result_commit": result_commit,
        "git_document_transaction_complete": result_commit.is_some(),
        "discussion_version_required": true,
        "change_kind": proposal.change_kind,
    }))
}

fn write_promotions(workspace: &Path, promotions: &[DiscussionPromotion]) -> Result<Vec<String>> {
    let mut paths = Vec::new();
    for promotion in promotions {
        if read_project_document_file(workspace, &promotion.path).is_err() {
            write_project_document_file(workspace, &promotion.path, &promotion.content, None)
                .map_err(|error| anyhow!(error.message))?;
        }
        paths.push(promotion.path.clone());
    }
    Ok(paths)
}

fn load_optional<T>(
    workspace: &Path,
    path: &str,
    normalize: impl FnOnce(T) -> Result<T>,
) -> Result<Versioned<T>>
where
    T: Default + for<'de> Deserialize<'de>,
{
    let Some(file) = read_optional(workspace, path)? else {
        return Ok(Versioned {
            value: normalize(T::default())?,
            revision: None,
        });
    };
    Ok(Versioned {
        value: normalize(serde_json::from_str(&file.content)?)?,
        revision: Some(file.revision),
    })
}

fn read_optional(
    workspace: &Path,
    path: &str,
) -> Result<Option<crate::project_document_files::ProjectDocumentFile>> {
    match read_project_document_file(workspace, path) {
        Ok(file) => Ok(Some(file)),
        Err(error) if error.to_string().contains("不存在") => Ok(None),
        Err(error) => Err(error),
    }
}

fn verify_revision(label: &str, current: Option<&str>, expected: Option<&str>) -> Result<()> {
    let expected = expected.filter(|value| !value.trim().is_empty());
    match (current, expected) {
        (Some(current), Some(expected)) if current == expected => Ok(()),
        (None, None) => Ok(()),
        (Some(_), None) => bail!("{label}已存在，请读取最新 revision 后重试"),
        _ => bail!("{label}已被其他会话修改，请刷新后重试"),
    }
}

fn pretty(value: &impl Serialize) -> Result<String> {
    Ok(format!("{}\n", serde_json::to_string_pretty(value)?))
}
