use std::collections::BTreeMap;

use serde::Serialize;

pub(crate) const LOCAL_HASH_VECTOR_MODEL: &str = "local-hash-v1";
pub(crate) const LOCAL_HASH_VECTOR_DIM: usize = 256;

const DEFAULT_VECTOR_LIMIT: usize = 12;
const MAX_VECTOR_LIMIT: usize = 50;
const DEFAULT_BACKFILL_LIMIT: usize = 5_000;
const MAX_BACKFILL_LIMIT: usize = 20_000;

#[derive(Debug, Clone, Default)]
pub(crate) struct SymbolVectorBackfillQuery {
    pub(crate) trace_id: Option<String>,
    pub(crate) model: Option<String>,
    pub(crate) limit: usize,
    pub(crate) force: bool,
}

impl SymbolVectorBackfillQuery {
    pub(crate) fn model(&self) -> String {
        self.model
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(LOCAL_HASH_VECTOR_MODEL)
            .to_string()
    }

    pub(crate) fn limit(&self) -> usize {
        if self.limit == 0 {
            DEFAULT_BACKFILL_LIMIT
        } else {
            self.limit.min(MAX_BACKFILL_LIMIT)
        }
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct SymbolVectorSearch {
    pub(crate) trace_id: Option<String>,
    pub(crate) text: Option<String>,
    pub(crate) model: Option<String>,
    pub(crate) path: Option<String>,
    pub(crate) chunk_type: Option<String>,
    pub(crate) limit: usize,
}

impl SymbolVectorSearch {
    pub(crate) fn model(&self) -> String {
        self.model
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(LOCAL_HASH_VECTOR_MODEL)
            .to_string()
    }

    pub(crate) fn limit(&self) -> usize {
        if self.limit == 0 {
            DEFAULT_VECTOR_LIMIT
        } else {
            self.limit.min(MAX_VECTOR_LIMIT)
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SymbolVectorBackfillResponse {
    pub(crate) db_path: String,
    pub(crate) query: SymbolVectorBackfillQueryEcho,
    pub(crate) metadata: BTreeMap<String, String>,
    pub(crate) model: String,
    pub(crate) dim: usize,
    pub(crate) scanned_count: usize,
    pub(crate) upserted_count: usize,
    pub(crate) skipped_count: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SymbolVectorBackfillQueryEcho {
    pub(crate) trace_id: Option<String>,
    pub(crate) model: String,
    pub(crate) limit: usize,
    pub(crate) force: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SymbolVectorSearchResponse {
    pub(crate) db_path: String,
    pub(crate) query: SymbolVectorSearchQueryEcho,
    pub(crate) metadata: BTreeMap<String, String>,
    pub(crate) model: String,
    pub(crate) dim: usize,
    pub(crate) chunks: Vec<SymbolVectorHit>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SymbolVectorSearchQueryEcho {
    pub(crate) trace_id: Option<String>,
    pub(crate) q: String,
    pub(crate) model: String,
    pub(crate) path: Option<String>,
    pub(crate) chunk_type: Option<String>,
    pub(crate) limit: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SymbolVectorHit {
    pub(crate) id: String,
    pub(crate) chunk_type: String,
    pub(crate) file_path: String,
    pub(crate) symbol_id: Option<String>,
    pub(crate) qualified_name: Option<String>,
    pub(crate) kind: Option<String>,
    pub(crate) start_line: Option<usize>,
    pub(crate) end_line: Option<usize>,
    pub(crate) content: String,
    pub(crate) summary: Option<String>,
    pub(crate) hash: String,
    pub(crate) token_count: usize,
    pub(crate) score: f64,
    pub(crate) matched_terms: Vec<String>,
}
