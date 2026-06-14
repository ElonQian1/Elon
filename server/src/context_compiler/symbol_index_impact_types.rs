use std::collections::BTreeMap;

use serde::Serialize;

use super::symbol_index_query_types::{SymbolEdgeHit, SymbolHit, MAX_EDGE_LIMIT};

const DEFAULT_IMPACT_LIMIT: usize = 120;
const DEFAULT_IMPACT_DEPTH: usize = 1;
const MAX_IMPACT_DEPTH: usize = 3;

#[derive(Debug, Clone, Default)]
pub(crate) struct SymbolImpactQuery {
    pub(crate) trace_id: Option<String>,
    pub(crate) symbol_id: Option<String>,
    pub(crate) path: Option<String>,
    pub(crate) edge_kind: Option<String>,
    pub(crate) depth: usize,
    pub(crate) limit: usize,
}

impl SymbolImpactQuery {
    pub(crate) fn depth(&self) -> usize {
        if self.depth == 0 {
            DEFAULT_IMPACT_DEPTH
        } else {
            self.depth.min(MAX_IMPACT_DEPTH)
        }
    }

    pub(crate) fn limit(&self) -> usize {
        if self.limit == 0 {
            DEFAULT_IMPACT_LIMIT
        } else {
            self.limit.min(MAX_EDGE_LIMIT)
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SymbolImpactResponse {
    pub(crate) db_path: String,
    pub(crate) query: SymbolImpactQueryEcho,
    pub(crate) metadata: BTreeMap<String, String>,
    pub(crate) seed_symbols: Vec<SymbolHit>,
    pub(crate) impacted_symbols: Vec<SymbolHit>,
    pub(crate) edges: Vec<SymbolEdgeHit>,
    pub(crate) impacted_files: Vec<ImpactFile>,
    pub(crate) test_hints: Vec<ImpactTestHint>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SymbolImpactQueryEcho {
    pub(crate) trace_id: Option<String>,
    pub(crate) symbol_id: Option<String>,
    pub(crate) path: Option<String>,
    pub(crate) edge_kind: Option<String>,
    pub(crate) depth: usize,
    pub(crate) limit: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ImpactFile {
    pub(crate) path: String,
    pub(crate) seed: bool,
    pub(crate) symbol_count: usize,
    pub(crate) edge_count: usize,
    pub(crate) test_hint_count: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ImpactTestHint {
    pub(crate) symbol_id: String,
    pub(crate) symbol_name: String,
    pub(crate) path: String,
    pub(crate) line: usize,
    pub(crate) reason: String,
    pub(crate) edge_kind: Option<String>,
    pub(crate) target_symbol_id: Option<String>,
}
