use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
};

use serde::Serialize;

use super::{
    symbol_index_chunks::SymbolChunkHit, symbol_index_impact_types::SymbolImpactResponse,
    symbol_index_query_types::SymbolHit, symbol_index_rank_profile::HybridRankProfile,
    symbol_index_retrieval_plan::RetrievalPlan, symbol_index_vector_types::SymbolVectorHit,
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RankedContextItem {
    pub(crate) rank: usize,
    pub(crate) source: String,
    pub(crate) sources: Vec<String>,
    pub(crate) id: String,
    pub(crate) label: String,
    pub(crate) file_path: String,
    pub(crate) symbol_id: Option<String>,
    pub(crate) start_line: Option<usize>,
    pub(crate) end_line: Option<usize>,
    pub(crate) score: f64,
    pub(crate) token_count: usize,
    pub(crate) matched_terms: Vec<String>,
    pub(crate) reasons: Vec<String>,
    pub(crate) is_test_context: bool,
    pub(crate) decision: RerankDecision,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RerankDecision {
    MustInclude,
    Include,
    Summarize,
    Drop,
}

impl RerankDecision {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            RerankDecision::MustInclude => "must_include",
            RerankDecision::Include => "include",
            RerankDecision::Summarize => "summarize",
            RerankDecision::Drop => "drop",
        }
    }
}

#[derive(Debug, Clone)]
struct ContextDraft {
    source: &'static str,
    sources: BTreeSet<String>,
    id: String,
    label: String,
    file_path: String,
    symbol_id: Option<String>,
    start_line: Option<usize>,
    end_line: Option<usize>,
    score: f64,
    token_count: usize,
    matched_terms: Vec<String>,
    reasons: Vec<String>,
    is_test_context: bool,
}

pub(crate) fn rank_hybrid_context_with_profile(
    symbols: &[SymbolHit],
    text_chunks: &[SymbolChunkHit],
    vector_chunks: &[SymbolVectorHit],
    impact: Option<&SymbolImpactResponse>,
    profile: &HybridRankProfile,
) -> Vec<RankedContextItem> {
    let mut drafts = Vec::new();
    for (index, symbol) in symbols.iter().enumerate() {
        drafts.push(symbol_context(symbol, index, profile));
    }
    for (index, chunk) in text_chunks.iter().enumerate() {
        drafts.push(text_chunk_context(chunk, index, profile));
    }
    for (index, chunk) in vector_chunks.iter().enumerate() {
        drafts.push(vector_chunk_context(chunk, index, profile));
    }
    if let Some(impact) = impact {
        push_impact_context(&mut drafts, impact, profile);
    }
    rank_drafts(drafts)
}

pub(crate) fn rank_hybrid_context_with_plan(
    symbols: &[SymbolHit],
    text_chunks: &[SymbolChunkHit],
    vector_chunks: &[SymbolVectorHit],
    impact: Option<&SymbolImpactResponse>,
    profile: &HybridRankProfile,
    plan: &RetrievalPlan,
) -> Vec<RankedContextItem> {
    let mut ranked =
        rank_hybrid_context_with_profile(symbols, text_chunks, vector_chunks, impact, profile);
    apply_plan_ranking(&mut ranked, plan);
    rerank_items(ranked, plan)
}


#[path = "symbol_index_ranker_impl.rs"]
mod impl_funcs;
use self::impl_funcs::*;
