use std::collections::BTreeMap;

use serde::Serialize;

pub(crate) const DEFAULT_LIMIT: usize = 20;
pub(crate) const MAX_LIMIT: usize = 100;
pub(crate) const MAX_EDGE_LIMIT: usize = 300;

#[derive(Debug, Clone, Default)]
pub(crate) struct SymbolIndexSearch {
    pub(crate) trace_id: Option<String>,
    pub(crate) text: Option<String>,
    pub(crate) kind: Option<String>,
    pub(crate) path: Option<String>,
    pub(crate) edge_kind: Option<String>,
    pub(crate) include_edges: bool,
    pub(crate) limit: usize,
}

impl SymbolIndexSearch {
    pub(crate) fn limit(&self) -> usize {
        if self.limit == 0 {
            DEFAULT_LIMIT
        } else {
            self.limit.min(MAX_LIMIT)
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SymbolIndexSearchResponse {
    pub(crate) db_path: String,
    pub(crate) query: SymbolIndexQueryEcho,
    pub(crate) metadata: BTreeMap<String, String>,
    pub(crate) symbols: Vec<SymbolHit>,
    pub(crate) edges: Vec<SymbolEdgeHit>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SymbolIndexQueryEcho {
    pub(crate) trace_id: Option<String>,
    pub(crate) q: Option<String>,
    pub(crate) kind: Option<String>,
    pub(crate) path: Option<String>,
    pub(crate) edge_kind: Option<String>,
    pub(crate) include_edges: bool,
    pub(crate) limit: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SymbolHit {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) qualified_name: String,
    pub(crate) kind: String,
    pub(crate) language: String,
    pub(crate) file_path: String,
    pub(crate) start_line: usize,
    pub(crate) end_line: usize,
    pub(crate) signature: String,
    pub(crate) visibility: String,
    pub(crate) parent_symbol_id: Option<String>,
    pub(crate) module_path: String,
    pub(crate) doc_summary: Option<String>,
    pub(crate) role: String,
    pub(crate) importance_score: Option<f64>,
    pub(crate) source_providers: Vec<String>,
    pub(crate) score: f64,
    pub(crate) matched_terms: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SymbolEdgeHit {
    pub(crate) id: String,
    pub(crate) source: String,
    pub(crate) kind: String,
    pub(crate) from_symbol_id: Option<String>,
    pub(crate) from_path: String,
    pub(crate) line: usize,
    pub(crate) to_symbol_id: Option<String>,
    pub(crate) to_symbol_name: Option<String>,
    pub(crate) to_path: Option<String>,
    pub(crate) confidence: f64,
    pub(crate) reason: String,
}
