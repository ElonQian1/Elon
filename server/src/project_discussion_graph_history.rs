//! Semantic, low-token history for the portable discussion graph.

use anyhow::{anyhow, bail, Context, Result};
use serde::Serialize;
use serde_json::{json, Value};
use std::{collections::HashMap, path::Path, process::Output};

use crate::{
    project_discussion_graph::load_graph,
    project_discussion_graph_model::{
        DiscussionEdge, DiscussionGraph, DiscussionNode, DISCUSSION_GRAPH_PATH,
    },
    project_discussion_graph_validation::{counts, normalize_graph},
    project_document_files::content_revision,
};

#[path = "project_discussion_graph_history_diff.rs"]
mod diff;

use diff::{incident_edges, node_changed_fields, semantic_diff};

#[derive(Debug, Clone, Serialize)]
pub(crate) struct HistoricalDiscussionGraph {
    pub commit: String,
    pub created_at: String,
    pub summary: String,
    pub graph_revision: String,
    pub graph: DiscussionGraph,
}

#[derive(Debug, Clone)]
struct CommitMeta {
    commit: String,
    created_at: String,
    summary: String,
}

pub(crate) fn list_discussion_versions(workspace: &Path, limit: usize) -> Result<Value> {
    let commits = history_commits(workspace, limit.clamp(1, 100))?;
    let mut versions = Vec::with_capacity(commits.len());
    for meta in commits {
        let graph = graph_at_resolved(workspace, &meta.commit)?;
        let previous = first_parent(workspace, &meta.commit)?
            .map(|parent| graph_at_resolved(workspace, &parent))
            .transpose()?
            .unwrap_or_default();
        let changes = semantic_diff(&previous, &graph);
        versions.push(json!({
            "commit": meta.commit,
            "created_at": non_empty(&graph.evolution.changed_at, &meta.created_at),
            "summary": non_empty(&graph.evolution.summary, &meta.summary),
            "change_kind": &graph.evolution.kind,
            "actor": &graph.evolution.actor,
            "previous_revision": &graph.evolution.previous_revision,
            "graph_revision": graph_revision(&graph)?,
            "counts": counts(&graph, 0),
            "changes": changes["counts"],
        }));
    }
    Ok(json!({
        "versions": versions,
        "budget": metadata_budget(),
    }))
}

pub(crate) fn load_discussion_graph_version(
    workspace: &Path,
    commit: &str,
) -> Result<HistoricalDiscussionGraph> {
    let commit = verified_commit(workspace, commit, false)?;
    let meta = commit_meta(workspace, &commit)?;
    let graph = graph_at_resolved(workspace, &commit)?;
    Ok(HistoricalDiscussionGraph {
        commit,
        created_at: non_empty(&graph.evolution.changed_at, &meta.created_at).to_string(),
        summary: non_empty(&graph.evolution.summary, &meta.summary).to_string(),
        graph_revision: graph_revision(&graph)?,
        graph,
    })
}

pub(crate) fn compare_discussion_versions(
    workspace: &Path,
    base_commit: &str,
    target_commit: Option<&str>,
) -> Result<Value> {
    let base_commit = verified_commit(workspace, base_commit, false)?;
    let target_commit = verified_commit(
        workspace,
        target_commit
            .filter(|value| !value.trim().is_empty())
            .unwrap_or("HEAD"),
        true,
    )?;
    let base = graph_at_resolved(workspace, &base_commit)?;
    let target = graph_at_resolved(workspace, &target_commit)?;
    let mut result = semantic_diff(&base, &target);
    result["base_commit"] = json!(base_commit);
    result["target_commit"] = json!(target_commit);
    result["base_graph_revision"] = json!(graph_revision(&base)?);
    result["target_graph_revision"] = json!(graph_revision(&target)?);
    result["base_counts"] = counts(&base, 0);
    result["target_counts"] = counts(&target, 0);
    result["budget"] = metadata_budget();
    Ok(result)
}

pub(crate) fn trace_discussion_node(
    workspace: &Path,
    node_id: &str,
    limit: usize,
) -> Result<Value> {
    let node_id = node_id.trim().to_ascii_lowercase();
    if node_id.is_empty() || node_id.len() > 100 {
        bail!("node_id 无效");
    }
    let requested = limit.clamp(1, 100);
    let mut commits = history_commits(workspace, requested.saturating_add(1))?;
    let truncated = commits.len() > requested;
    commits.truncate(requested);
    commits.reverse();
    let mut previous_node: Option<DiscussionNode> = None;
    let mut previous_edges = HashMap::<String, DiscussionEdge>::new();
    let mut events = Vec::new();
    for meta in commits {
        let graph = graph_at_resolved(workspace, &meta.commit)?;
        let node = graph.nodes.iter().find(|node| node.id == node_id).cloned();
        let edges = incident_edges(&graph, &node_id);
        push_node_event(
            &mut events,
            &meta,
            previous_node.as_ref(),
            node.as_ref(),
            &previous_edges,
            &edges,
            truncated,
        );
        previous_node = node;
        previous_edges = edges;
    }
    let working = load_graph(workspace)?;
    let working_node = working
        .value
        .nodes
        .iter()
        .find(|node| node.id == node_id)
        .cloned();
    let working_edges = incident_edges(&working.value, &node_id);
    if working_node != previous_node || working_edges != previous_edges {
        let meta = CommitMeta {
            commit: "WORKING_COPY".to_string(),
            created_at: String::new(),
            summary: "尚未进入 Git 提交的当前图谱".to_string(),
        };
        push_node_event(
            &mut events,
            &meta,
            previous_node.as_ref(),
            working_node.as_ref(),
            &previous_edges,
            &working_edges,
            false,
        );
    }
    if events.is_empty() && working_node.is_none() {
        bail!("讨论节点不存在：{node_id}");
    }
    Ok(json!({
        "node_id": node_id,
        "current_node": working_node,
        "events": events,
        "truncated_history": truncated,
        "budget": metadata_budget(),
    }))
}

fn push_node_event(
    events: &mut Vec<Value>,
    meta: &CommitMeta,
    previous: Option<&DiscussionNode>,
    current: Option<&DiscussionNode>,
    previous_edges: &HashMap<String, DiscussionEdge>,
    current_edges: &HashMap<String, DiscussionEdge>,
    history_truncated: bool,
) {
    let edge_added = current_edges
        .keys()
        .filter(|id| !previous_edges.contains_key(*id))
        .cloned()
        .collect::<Vec<_>>();
    let edge_removed = previous_edges
        .keys()
        .filter(|id| !current_edges.contains_key(*id))
        .cloned()
        .collect::<Vec<_>>();
    let (kind, fields) = match (previous, current) {
        (None, Some(_)) => (
            if history_truncated && events.is_empty() {
                "observed"
            } else {
                "created"
            },
            Vec::new(),
        ),
        (Some(_), None) => ("removed", Vec::new()),
        (Some(before), Some(after)) if before != after => {
            ("updated", node_changed_fields(before, after))
        }
        _ if !edge_added.is_empty() || !edge_removed.is_empty() => {
            ("relations_changed", Vec::new())
        }
        _ => return,
    };
    events.push(json!({
        "commit": meta.commit,
        "created_at": meta.created_at,
        "summary": meta.summary,
        "event": kind,
        "changed_fields": fields,
        "from_status": previous.map(|node| node.status.as_str()),
        "to_status": current.map(|node| node.status.as_str()),
        "from_parent_id": previous.map(|node| node.parent_id.as_str()),
        "to_parent_id": current.map(|node| node.parent_id.as_str()),
        "edges_added": edge_added,
        "edges_removed": edge_removed,
    }));
}

fn history_commits(workspace: &Path, limit: usize) -> Result<Vec<CommitMeta>> {
    ensure_git(workspace)?;
    let output = git(
        workspace,
        &[
            "log",
            &format!("-n{}", limit.clamp(1, 101)),
            "--format=%H%x1f%cI%x1f%s%x1e",
            "--",
            DISCUSSION_GRAPH_PATH,
        ],
    )?;
    Ok(String::from_utf8(output.stdout)?
        .split('\u{1e}')
        .filter_map(|record| {
            let fields = record.trim().split('\u{1f}').collect::<Vec<_>>();
            (fields.len() == 3).then(|| CommitMeta {
                commit: fields[0].to_string(),
                created_at: fields[1].to_string(),
                summary: fields[2].to_string(),
            })
        })
        .collect())
}

fn graph_at_resolved(workspace: &Path, commit: &str) -> Result<DiscussionGraph> {
    let object = format!("{commit}:{DISCUSSION_GRAPH_PATH}");
    if !git_status(workspace, &["cat-file", "-e", &object])? {
        return normalize_graph(DiscussionGraph::default());
    }
    let output = git(workspace, &["show", &object])?;
    normalize_graph(serde_json::from_slice(&output.stdout)?)
}

fn graph_revision(graph: &DiscussionGraph) -> Result<String> {
    Ok(content_revision(&serde_json::to_string(graph)?))
}

fn first_parent(workspace: &Path, commit: &str) -> Result<Option<String>> {
    let output = git(workspace, &["rev-list", "--parents", "-n", "1", commit])?;
    Ok(String::from_utf8(output.stdout)?
        .split_whitespace()
        .nth(1)
        .map(str::to_string))
}

fn commit_meta(workspace: &Path, commit: &str) -> Result<CommitMeta> {
    let output = git(
        workspace,
        &["show", "-s", "--format=%H%x1f%cI%x1f%s", commit],
    )?;
    let text = String::from_utf8(output.stdout)?;
    let fields = text.trim().split('\u{1f}').collect::<Vec<_>>();
    if fields.len() != 3 {
        bail!("无法读取讨论图版本信息");
    }
    Ok(CommitMeta {
        commit: fields[0].to_string(),
        created_at: fields[1].to_string(),
        summary: fields[2].to_string(),
    })
}

fn verified_commit(workspace: &Path, value: &str, allow_head: bool) -> Result<String> {
    ensure_git(workspace)?;
    let value = value.trim();
    if value != "HEAD"
        && (value.len() < 7 || value.len() > 64 || !value.chars().all(|ch| ch.is_ascii_hexdigit()))
    {
        bail!("讨论图版本提交格式无效");
    }
    if value == "HEAD" && !allow_head {
        bail!("请提供明确的历史提交");
    }
    let output = git(
        workspace,
        &["rev-parse", "--verify", &format!("{value}^{{commit}}")],
    )?;
    let commit = String::from_utf8(output.stdout)?.trim().to_string();
    if !git_status(workspace, &["merge-base", "--is-ancestor", &commit, "HEAD"])? {
        bail!("只能读取当前项目历史中的讨论图版本");
    }
    Ok(commit)
}

fn ensure_git(workspace: &Path) -> Result<()> {
    if !git_status(workspace, &["rev-parse", "--is-inside-work-tree"])? {
        bail!("项目不是 Git 工作区，无法读取讨论图历史");
    }
    Ok(())
}

fn git(workspace: &Path, args: &[&str]) -> Result<Output> {
    let output = crate::git_command_error::git_command()
        .current_dir(workspace)
        .args(args)
        .output()
        .context("无法启动 Git 讨论图版本操作")?;
    if !output.status.success() {
        return Err(anyhow!(
            "git {} 失败：{}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(output)
}

fn git_status(workspace: &Path, args: &[&str]) -> Result<bool> {
    Ok(crate::git_command_error::git_command()
        .current_dir(workspace)
        .args(args)
        .output()
        .context("无法检查 Git 讨论图版本")?
        .status
        .success())
}

fn metadata_budget() -> Value {
    json!({
        "classification_model_tokens": 0,
        "chat_bodies_read": 0,
        "document_bodies_read": 0,
        "metadata_only": true,
    })
}

fn non_empty<'a>(preferred: &'a str, fallback: &'a str) -> &'a str {
    if preferred.trim().is_empty() {
        fallback
    } else {
        preferred
    }
}

#[cfg(test)]
#[path = "project_discussion_graph_history_tests.rs"]
mod tests;
