//! Registers imported sources before AI compilation so interrupted work remains resumable.

use anyhow::{anyhow, Result};
use chrono::Utc;
use std::path::Path;

use crate::{
    project_discussion_graph::load_graph,
    project_discussion_graph_model::{
        DiscussionGraph, DiscussionGraphEvolution, DiscussionSource, DISCUSSION_GRAPH_PATH,
    },
    project_discussion_graph_validation::normalize_graph,
    project_discussion_source_chunks::source_chunk_count,
    project_discussion_source_normalizer::NormalizedConversation,
    project_document_files::write_project_document_file,
};

pub(crate) struct PendingSourcePlan {
    graph: DiscussionGraph,
    pub(crate) expected_graph_revision: Option<String>,
    pub(crate) changed: bool,
    pub(crate) chunk_count: usize,
}

pub(crate) fn plan_pending_source(
    workspace: &Path,
    title: &str,
    path: &str,
    source: &NormalizedConversation,
) -> Result<PendingSourcePlan> {
    let loaded = load_graph(workspace)?;
    let chunk_count = source_chunk_count(&source.body)?;
    let candidate = DiscussionSource {
        id: source.source_id.clone(),
        title: title.to_string(),
        kind: "imported_conversation".to_string(),
        reference: path.to_string(),
        imported_at: Utc::now().to_rfc3339(),
        content_revision: source.content_revision.clone(),
        source_format: source.format.clone(),
        message_count: source.message_count,
        chunk_count,
        processed_chunk_ids: Vec::new(),
        compilation_status: "pending".to_string(),
    };
    let (mut graph, changed) = merge_pending_source(loaded.value, candidate);
    if changed {
        graph.evolution = DiscussionGraphEvolution {
            kind: "import".to_string(),
            summary: format!("已登记聊天来源“{title}”，等待增量编译"),
            actor: "project-docs-import".to_string(),
            changed_at: Utc::now().to_rfc3339(),
            previous_revision: loaded.revision.clone().unwrap_or_default(),
        };
        graph = normalize_graph(graph)?;
    }
    Ok(PendingSourcePlan {
        graph,
        expected_graph_revision: loaded.revision,
        changed,
        chunk_count,
    })
}

pub(crate) fn apply_pending_source_plan(
    workspace: &Path,
    plan: PendingSourcePlan,
) -> Result<Option<String>> {
    if !plan.changed {
        return Ok(plan.expected_graph_revision);
    }
    let content = format!("{}\n", serde_json::to_string_pretty(&plan.graph)?);
    let saved = write_project_document_file(
        workspace,
        DISCUSSION_GRAPH_PATH,
        &content,
        plan.expected_graph_revision.as_deref(),
    )
    .map_err(|error| anyhow!(error.message))?;
    Ok(Some(saved.revision))
}

fn merge_pending_source(
    mut graph: DiscussionGraph,
    candidate: DiscussionSource,
) -> (DiscussionGraph, bool) {
    let existing = graph
        .sources
        .iter()
        .position(|source| source.id == candidate.id);
    let changed = match existing {
        Some(index) => {
            let current = &graph.sources[index];
            let merged = DiscussionSource {
                imported_at: if current.imported_at.is_empty() {
                    candidate.imported_at
                } else {
                    current.imported_at.clone()
                },
                processed_chunk_ids: current.processed_chunk_ids.clone(),
                compilation_status: current.compilation_status.clone(),
                ..candidate
            };
            if *current == merged {
                false
            } else {
                graph.sources[index] = merged;
                true
            }
        }
        None => {
            graph.sources.push(candidate);
            true
        }
    };
    (graph, changed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeated_registration_preserves_compilation_progress() {
        let candidate = DiscussionSource {
            id: "conversation-one".into(),
            title: "讨论".into(),
            processed_chunk_ids: vec!["chunk-0001-1234567890".into()],
            compilation_status: "partial".into(),
            ..Default::default()
        };
        let graph = DiscussionGraph {
            sources: vec![candidate.clone()],
            ..Default::default()
        };
        let incoming = DiscussionSource {
            processed_chunk_ids: Vec::new(),
            compilation_status: "pending".into(),
            ..candidate
        };
        let (merged, changed) = merge_pending_source(graph, incoming);
        assert!(!changed);
        assert_eq!(merged.sources[0].compilation_status, "partial");
        assert_eq!(merged.sources[0].processed_chunk_ids.len(), 1);
    }
}
