//! Shared portable schema for conversation-derived discussion graphs.

use serde::{Deserialize, Serialize};

pub(crate) const DISCUSSION_GRAPH_PATH: &str = ".elon/discussion-graph.json";
pub(crate) const DISCUSSION_SUGGESTIONS_PATH: &str = ".elon/discussion-graph-suggestions.json";

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct DiscussionGraph {
    #[serde(default = "version")]
    pub version: u8,
    #[serde(default)]
    pub sources: Vec<DiscussionSource>,
    #[serde(default)]
    pub nodes: Vec<DiscussionNode>,
    #[serde(default)]
    pub edges: Vec<DiscussionEdge>,
    #[serde(default)]
    pub evolution: DiscussionGraphEvolution,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct DiscussionGraphEvolution {
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub actor: String,
    #[serde(default)]
    pub changed_at: String,
    #[serde(default)]
    pub previous_revision: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct DiscussionSource {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub reference: String,
    #[serde(default)]
    pub imported_at: String,
    #[serde(default)]
    pub content_revision: String,
    #[serde(default)]
    pub source_format: String,
    #[serde(default)]
    pub message_count: usize,
    #[serde(default)]
    pub chunk_count: usize,
    #[serde(default)]
    pub processed_chunk_ids: Vec<String>,
    #[serde(default)]
    pub compilation_status: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct DiscussionNode {
    pub id: String,
    #[serde(default)]
    pub root_id: String,
    #[serde(default)]
    pub parent_id: String,
    #[serde(default)]
    pub kind: String,
    pub title: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub authority: String,
    #[serde(default)]
    pub section_id: String,
    #[serde(default)]
    pub order: i32,
    #[serde(default)]
    pub color: String,
    #[serde(default)]
    pub source_refs: Vec<String>,
    #[serde(default)]
    pub conversation_refs: Vec<String>,
    #[serde(default)]
    pub document_paths: Vec<String>,
    #[serde(default)]
    pub feature_node_ids: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct DiscussionEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    #[serde(default)]
    pub relation: String,
    #[serde(default)]
    pub label: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct DiscussionPromotion {
    pub id: String,
    pub node_id: String,
    pub path: String,
    pub title: String,
    pub content: String,
    #[serde(default)]
    pub document_type: String,
    #[serde(default)]
    pub section_id: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct DiscussionGraphProposal {
    #[serde(default = "version")]
    pub version: u8,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub change_kind: String,
    #[serde(default)]
    pub actor: String,
    #[serde(default)]
    pub graph: DiscussionGraph,
    #[serde(default)]
    pub promotions: Vec<DiscussionPromotion>,
    #[serde(default)]
    pub documents_read: usize,
    #[serde(default)]
    pub estimated_tokens_used: u64,
}

pub(crate) struct Versioned<T> {
    pub value: T,
    pub revision: Option<String>,
}

pub(crate) const fn version() -> u8 {
    1
}
