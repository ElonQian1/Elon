use std::collections::BTreeMap;

use serde::Serialize;

use super::symbol_index_product::{
    EmbeddingModelCostSummary, EmbeddingQueueSummary, ProjectIndexProductStatus,
    RetrievalEvalSetSummary,
};

const DEFAULT_EMBEDDING_STATUS_LIMIT: usize = 20;
const MAX_EMBEDDING_STATUS_LIMIT: usize = 100;

#[derive(Debug, Clone, Default)]
pub(crate) struct SymbolEmbeddingStatusQuery {
    pub(crate) trace_id: Option<String>,
    pub(crate) model: Option<String>,
    pub(crate) limit: usize,
}

impl SymbolEmbeddingStatusQuery {
    pub(crate) fn limit(&self) -> usize {
        if self.limit == 0 {
            DEFAULT_EMBEDDING_STATUS_LIMIT
        } else {
            self.limit.min(MAX_EMBEDDING_STATUS_LIMIT)
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SymbolEmbeddingStatusResponse {
    pub(crate) db_path: String,
    pub(crate) query: SymbolEmbeddingStatusQueryEcho,
    pub(crate) metadata: BTreeMap<String, String>,
    pub(crate) project_status: ProjectIndexProductStatus,
    pub(crate) totals: SymbolEmbeddingTotals,
    pub(crate) models: Vec<SymbolEmbeddingModelSummary>,
    pub(crate) queue: EmbeddingQueueSummary,
    pub(crate) costs: Vec<EmbeddingModelCostSummary>,
    pub(crate) eval_set: RetrievalEvalSetSummary,
    pub(crate) missing_chunks: Vec<SymbolEmbeddingMissingChunk>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SymbolEmbeddingStatusQueryEcho {
    pub(crate) trace_id: Option<String>,
    pub(crate) model: Option<String>,
    pub(crate) limit: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SymbolEmbeddingTotals {
    pub(crate) embeddings_table_available: bool,
    pub(crate) chunk_count: usize,
    pub(crate) embedded_count: usize,
    pub(crate) missing_count: usize,
    pub(crate) stale_count: usize,
    pub(crate) coverage: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SymbolEmbeddingModelSummary {
    pub(crate) model: String,
    pub(crate) embedding_count: usize,
    pub(crate) min_dim: usize,
    pub(crate) max_dim: usize,
    pub(crate) first_created_at: Option<i64>,
    pub(crate) last_created_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SymbolEmbeddingMissingChunk {
    pub(crate) id: String,
    pub(crate) chunk_type: String,
    pub(crate) file_path: String,
    pub(crate) symbol_id: Option<String>,
    pub(crate) qualified_name: Option<String>,
    pub(crate) kind: Option<String>,
    pub(crate) start_line: Option<usize>,
    pub(crate) end_line: Option<usize>,
    pub(crate) hash: String,
    pub(crate) token_count: usize,
    pub(crate) has_embedding: bool,
}
