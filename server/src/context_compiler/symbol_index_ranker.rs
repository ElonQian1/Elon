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
}

#[derive(Debug, Clone)]
struct ContextDraft {
    source: &'static str,
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
    rerank_items(ranked)
}

fn symbol_context(symbol: &SymbolHit, index: usize, profile: &HybridRankProfile) -> ContextDraft {
    let importance = symbol.importance_score.unwrap_or_default();
    let is_test_context = looks_like_test(&symbol.file_path) || symbol.name.contains("test");
    ContextDraft {
        source: "symbol",
        id: format!("symbol:{}", symbol.id),
        label: symbol.qualified_name.clone(),
        file_path: symbol.file_path.clone(),
        symbol_id: Some(symbol.id.clone()),
        start_line: Some(symbol.start_line),
        end_line: Some(symbol.end_line),
        score: profile.source_weight("symbol")
            + symbol.score
            + importance
            + profile.test_bonus(is_test_context)
            - index as f64,
        token_count: estimate_token_count(&symbol.signature),
        matched_terms: symbol.matched_terms.clone(),
        reasons: compact_reasons([
            Some("symbol_search".to_string()),
            Some(profile.reason("symbol")),
            (!symbol.matched_terms.is_empty())
                .then(|| format!("matched={}", symbol.matched_terms.join(","))),
            (importance > 0.0).then(|| format!("importance={importance:.2}")),
        ]),
        is_test_context,
    }
}

fn text_chunk_context(
    chunk: &SymbolChunkHit,
    index: usize,
    profile: &HybridRankProfile,
) -> ContextDraft {
    let is_test_context = chunk.chunk_type == "test" || looks_like_test(&chunk.file_path);
    ContextDraft {
        source: "full_text",
        id: format!("chunk:{}", chunk.id),
        label: chunk
            .qualified_name
            .clone()
            .unwrap_or_else(|| chunk.id.clone()),
        file_path: chunk.file_path.clone(),
        symbol_id: chunk.symbol_id.clone(),
        start_line: chunk.start_line,
        end_line: chunk.end_line,
        score: profile.source_weight("full_text")
            + chunk.score
            + profile.test_bonus(is_test_context)
            + profile.chunk_type_bonus(&chunk.chunk_type)
            - index as f64,
        token_count: chunk.token_count,
        matched_terms: chunk.matched_terms.clone(),
        reasons: compact_reasons([
            Some("fts_bm25".to_string()),
            Some(profile.reason("full_text")),
            Some(format!("chunk_type={}", chunk.chunk_type)),
            (!chunk.matched_terms.is_empty())
                .then(|| format!("matched={}", chunk.matched_terms.join(","))),
        ]),
        is_test_context,
    }
}

fn vector_chunk_context(
    chunk: &SymbolVectorHit,
    index: usize,
    profile: &HybridRankProfile,
) -> ContextDraft {
    let is_test_context = chunk.chunk_type == "test" || looks_like_test(&chunk.file_path);
    ContextDraft {
        source: "vector",
        id: format!("vector:{}", chunk.id),
        label: chunk
            .qualified_name
            .clone()
            .unwrap_or_else(|| chunk.id.clone()),
        file_path: chunk.file_path.clone(),
        symbol_id: chunk.symbol_id.clone(),
        start_line: chunk.start_line,
        end_line: chunk.end_line,
        score: profile.source_weight("vector")
            + (chunk.score * 100.0)
            + profile.test_bonus(is_test_context)
            + profile.chunk_type_bonus(&chunk.chunk_type)
            - index as f64,
        token_count: chunk.token_count,
        matched_terms: chunk.matched_terms.clone(),
        reasons: compact_reasons([
            Some("vector_similarity".to_string()),
            Some(profile.reason("vector")),
            Some(format!("chunk_type={}", chunk.chunk_type)),
            Some(format!("similarity={:.4}", chunk.score)),
        ]),
        is_test_context,
    }
}

fn push_impact_context(
    drafts: &mut Vec<ContextDraft>,
    impact: &SymbolImpactResponse,
    profile: &HybridRankProfile,
) {
    for (index, symbol) in impact.impacted_symbols.iter().enumerate() {
        let mut candidate = symbol_context(symbol, index, profile);
        candidate.source = "graph_symbol";
        candidate.id = format!("graph-symbol:{}", symbol.id);
        candidate.score = profile.source_weight("graph_symbol")
            + symbol.importance_score.unwrap_or_default()
            + profile.test_bonus(candidate.is_test_context)
            - index as f64;
        candidate.reasons = compact_reasons([
            Some("graph_expansion".to_string()),
            Some("impacted_symbol".to_string()),
            Some(profile.reason("graph_symbol")),
            symbol
                .importance_score
                .map(|score| format!("importance={score:.2}")),
        ]);
        drafts.push(candidate);
    }
    for (index, file) in impact.impacted_files.iter().enumerate() {
        let is_test_context = file.test_hint_count > 0 || looks_like_test(&file.path);
        drafts.push(ContextDraft {
            source: "graph_file",
            id: format!("graph-file:{}", file.path),
            label: file.path.clone(),
            file_path: file.path.clone(),
            symbol_id: None,
            start_line: None,
            end_line: None,
            score: profile.source_weight("graph_file")
                + (file.test_hint_count as f64 * 8.0)
                + (file.edge_count as f64 * 2.0)
                + (file.symbol_count as f64)
                + profile.test_bonus(is_test_context)
                - index as f64,
            token_count: 0,
            matched_terms: Vec::new(),
            reasons: compact_reasons([
                Some("graph_expansion".to_string()),
                Some(profile.reason("graph_file")),
                Some(format!("symbols={}", file.symbol_count)),
                Some(format!("edges={}", file.edge_count)),
                (file.test_hint_count > 0).then(|| format!("test_hints={}", file.test_hint_count)),
                file.seed.then(|| "seed_file".to_string()),
            ]),
            is_test_context,
        });
    }
    for (index, hint) in impact.test_hints.iter().enumerate() {
        drafts.push(ContextDraft {
            source: "graph_test",
            id: format!("graph-test:{}", hint.symbol_id),
            label: hint.symbol_name.clone(),
            file_path: hint.path.clone(),
            symbol_id: Some(hint.symbol_id.clone()),
            start_line: Some(hint.line),
            end_line: None,
            score: profile.source_weight("graph_test") + profile.test_bonus(true) - index as f64,
            token_count: 0,
            matched_terms: Vec::new(),
            reasons: compact_reasons([
                Some("test_hint".to_string()),
                Some(profile.reason("graph_test")),
                hint.edge_kind
                    .as_deref()
                    .map(|kind| format!("edge_kind={kind}")),
                Some(hint.reason.clone()),
            ]),
            is_test_context: true,
        });
    }
}

fn apply_plan_ranking(items: &mut [RankedContextItem], plan: &RetrievalPlan) {
    for item in items {
        let bonus = plan.ranking_bonus(&item.source);
        if bonus > 0.0 {
            item.score += bonus;
            item.reasons.push(format!(
                "retrieval_plan={} source_weight={:.2} bonus={bonus:.1}",
                plan.intent.as_str(),
                plan.source_weight(&item.source)
            ));
        }
        if item.is_test_context && !plan.pack_policy.include_tests {
            item.score -= 120.0;
            item.reasons
                .push("retrieval_plan_deprioritized_tests".to_string());
        }
        if item.source == "full_text" && !plan.pack_policy.include_code_snippets {
            item.score -= 80.0;
            item.reasons
                .push("retrieval_plan_prefers_summary_over_snippet".to_string());
        }
    }
}

fn rerank_items(mut items: Vec<RankedContextItem>) -> Vec<RankedContextItem> {
    items.sort_by(compare_ranked_candidates);
    for (index, item) in items.iter_mut().enumerate() {
        item.rank = index + 1;
    }
    items
}

fn compare_ranked_candidates(left: &RankedContextItem, right: &RankedContextItem) -> Ordering {
    right
        .score
        .partial_cmp(&left.score)
        .unwrap_or(Ordering::Equal)
        .then_with(|| left.file_path.cmp(&right.file_path))
        .then_with(|| left.start_line.cmp(&right.start_line))
        .then_with(|| left.label.cmp(&right.label))
}

fn rank_drafts(drafts: Vec<ContextDraft>) -> Vec<RankedContextItem> {
    let mut best = BTreeMap::<String, ContextDraft>::new();
    for draft in drafts {
        let key = candidate_key(&draft);
        let replace = best
            .get(&key)
            .map(|existing| draft.score > existing.score)
            .unwrap_or(true);
        if replace {
            best.insert(key, draft);
        }
    }
    let mut drafts = best.into_values().collect::<Vec<_>>();
    drafts.sort_by(compare_candidates);
    drafts
        .into_iter()
        .enumerate()
        .map(|(index, draft)| ranked_item(index + 1, draft))
        .collect()
}

fn ranked_item(rank: usize, draft: ContextDraft) -> RankedContextItem {
    RankedContextItem {
        rank,
        source: draft.source.to_string(),
        id: draft.id,
        label: draft.label,
        file_path: draft.file_path,
        symbol_id: draft.symbol_id,
        start_line: draft.start_line,
        end_line: draft.end_line,
        score: draft.score,
        token_count: draft.token_count,
        matched_terms: draft.matched_terms,
        reasons: draft.reasons,
        is_test_context: draft.is_test_context,
    }
}

fn compare_candidates(left: &ContextDraft, right: &ContextDraft) -> Ordering {
    right
        .score
        .partial_cmp(&left.score)
        .unwrap_or(Ordering::Equal)
        .then_with(|| left.file_path.cmp(&right.file_path))
        .then_with(|| left.start_line.cmp(&right.start_line))
        .then_with(|| left.label.cmp(&right.label))
}

fn candidate_key(candidate: &ContextDraft) -> String {
    if let Some(symbol_id) = candidate.symbol_id.as_deref() {
        format!("{}:{symbol_id}", candidate.source)
    } else {
        format!(
            "{}:{}:{}",
            candidate.source, candidate.file_path, candidate.label
        )
    }
}

fn compact_reasons(reasons: impl IntoIterator<Item = Option<String>>) -> Vec<String> {
    reasons
        .into_iter()
        .flatten()
        .map(|reason| reason.trim().to_string())
        .filter(|reason| !reason.is_empty())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn looks_like_test(path: &str) -> bool {
    let path = path.to_ascii_lowercase();
    path.contains("/tests/")
        || path.ends_with("_test.rs")
        || path.ends_with("_tests.rs")
        || path.contains("tests.rs")
}

fn estimate_token_count(value: &str) -> usize {
    value
        .split_whitespace()
        .map(|part| (part.len() / 4).max(1))
        .sum()
}
