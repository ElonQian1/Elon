use std::collections::BTreeMap;

use serde::Serialize;

const DEFAULT_CHUNK_LIMIT: usize = 12;
const MAX_CHUNK_LIMIT: usize = 50;

#[derive(Debug, Clone, Default)]
pub(crate) struct SymbolChunkSearch {
    pub(crate) trace_id: Option<String>,
    pub(crate) text: Option<String>,
    pub(crate) path: Option<String>,
    pub(crate) chunk_type: Option<String>,
    pub(crate) limit: usize,
}

impl SymbolChunkSearch {
    pub(crate) fn limit(&self) -> usize {
        if self.limit == 0 {
            DEFAULT_CHUNK_LIMIT
        } else {
            self.limit.min(MAX_CHUNK_LIMIT)
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SymbolChunkSearchResponse {
    pub(crate) db_path: String,
    pub(crate) query: SymbolChunkQueryEcho,
    pub(crate) metadata: BTreeMap<String, String>,
    pub(crate) chunks: Vec<SymbolChunkHit>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SymbolChunkQueryEcho {
    pub(crate) trace_id: Option<String>,
    pub(crate) q: Option<String>,
    pub(crate) path: Option<String>,
    pub(crate) chunk_type: Option<String>,
    pub(crate) limit: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SymbolChunkHit {
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
