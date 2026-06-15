use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

use anyhow::{bail, Result};

use super::{
    symbol_index_chunks::{search_latest_symbol_chunks, SymbolChunkSearch},
    symbol_index_eval_types::{
        SymbolRetrievalEvalCandidate, SymbolRetrievalEvalMetrics, SymbolRetrievalEvalQuery,
        SymbolRetrievalEvalQueryEcho, SymbolRetrievalEvalResponse,
    },
    symbol_index_impact_query::load_latest_symbol_impact,
    symbol_index_impact_types::SymbolImpactQuery,
    symbol_index_query::{search_latest_symbol_index, SymbolIndexSearch},
    symbol_index_query_types::SymbolHit,
    symbol_index_vector::{search_latest_symbol_vectors, SymbolVectorSearchQuery},
};

pub(crate) use super::symbol_index_eval_types::SymbolRetrievalEvalQuery as RetrievalEvalQuery;

#[derive(Debug, Clone)]
struct CandidateDraft {
    source: &'static str,
    id: String,
    label: String,
    file_path: String,
    symbol_id: Option<String>,
    start_line: Option<usize>,
    end_line: Option<usize>,
    score: f64,
    token_count: usize,
    is_test_context: bool,
}

pub(crate) fn evaluate_latest_symbol_retrieval(
    data_dir: &Path,
    query: &SymbolRetrievalEvalQuery,
) -> Result<SymbolRetrievalEvalResponse> {
    let text = query
        .text
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow::anyhow!("q 不能为空"))?
        .to_string();

    let symbol_response = search_latest_symbol_index(
        data_dir,
        &SymbolIndexSearch {
            trace_id: query.trace_id.clone(),
            text: Some(text.clone()),
            include_edges: false,
            limit: query.symbol_limit(),
            ..Default::default()
        },
    )?;
    let chunk_hits = search_latest_symbol_chunks(
        data_dir,
        &SymbolChunkSearch {
            trace_id: query.trace_id.clone(),
            text: Some(text.clone()),
            limit: query.chunk_limit(),
            ..Default::default()
        },
    )
    .map(|response| response.chunks)
    .unwrap_or_default();
    let vector_hits = if let Some(vector_model) = clean_filter(query.vector_model.as_deref()) {
        search_latest_symbol_vectors(
            data_dir,
            &SymbolVectorSearchQuery {
                trace_id: query.trace_id.clone(),
                text: Some(text.clone()),
                model: Some(vector_model),
                limit: query.vector_limit(),
                ..Default::default()
            },
        )
        .map(|response| response.chunks)
        .unwrap_or_default()
    } else {
        Vec::new()
    };

    if symbol_response.symbols.is_empty() && chunk_hits.is_empty() && vector_hits.is_empty() {
        bail!("没有找到可评测的符号或全文 chunk");
    }

    let impact = symbol_response.symbols.first().and_then(|seed| {
        load_latest_symbol_impact(
            data_dir,
            &SymbolImpactQuery {
                trace_id: query.trace_id.clone(),
                symbol_id: Some(seed.id.clone()),
                depth: query.depth,
                limit: query.impact_limit(),
                ..Default::default()
            },
        )
        .ok()
    });

    let requirements = clean_requirements(&query.must_include);
    let drafts = collect_candidates(
        &symbol_response.symbols,
        &chunk_hits,
        &vector_hits,
        impact.as_ref(),
    );
    let candidates = rank_candidates(drafts, &requirements);
    let top_k = candidates.iter().take(query.k()).collect::<Vec<_>>();
    let missing_requirements = missing_requirements(&requirements, &top_k);
    let metrics = build_metrics(&requirements, &candidates, query.k());

    Ok(SymbolRetrievalEvalResponse {
        db_path: symbol_response.db_path,
        query: SymbolRetrievalEvalQueryEcho {
            trace_id: query.trace_id.clone(),
            q: text,
            must_include: requirements,
            k: query.k(),
            symbol_limit: query.symbol_limit(),
            chunk_limit: query.chunk_limit(),
            vector_model: query.vector_model.clone(),
            vector_limit: query.vector_limit(),
            depth: impact
                .as_ref()
                .map(|impact| impact.query.depth)
                .unwrap_or(query.depth),
            impact_limit: query.impact_limit(),
        },
        metadata: symbol_response.metadata,
        metrics,
        candidates,
        missing_requirements,
    })
}

fn collect_candidates(
    symbols: &[SymbolHit],
    chunks: &[super::symbol_index_chunks::SymbolChunkHit],
    vector_hits: &[super::symbol_index_vector_types::SymbolVectorHit],
    impact: Option<&super::symbol_index_impact_types::SymbolImpactResponse>,
) -> Vec<CandidateDraft> {
    let mut candidates = Vec::new();
    for (index, symbol) in symbols.iter().enumerate() {
        candidates.push(symbol_candidate(symbol, index));
    }
    for (index, chunk) in chunks.iter().enumerate() {
        candidates.push(CandidateDraft {
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
            score: 800.0 + chunk.score - index as f64,
            token_count: chunk.token_count,
            is_test_context: chunk.chunk_type == "test" || looks_like_test(&chunk.file_path),
        });
    }
    for (index, chunk) in vector_hits.iter().enumerate() {
        candidates.push(CandidateDraft {
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
            score: 760.0 + (chunk.score * 100.0) - index as f64,
            token_count: chunk.token_count,
            is_test_context: chunk.chunk_type == "test" || looks_like_test(&chunk.file_path),
        });
    }
    if let Some(impact) = impact {
        push_impact_candidates(&mut candidates, impact);
    }
    candidates
}

fn push_impact_candidates(
    candidates: &mut Vec<CandidateDraft>,
    impact: &super::symbol_index_impact_types::SymbolImpactResponse,
) {
    for (index, symbol) in impact.impacted_symbols.iter().enumerate() {
        let mut candidate = symbol_candidate(symbol, index);
        candidate.source = "graph_symbol";
        candidate.id = format!("graph-symbol:{}", symbol.id);
        candidate.score = 650.0 + symbol.importance_score.unwrap_or_default() - index as f64;
        candidates.push(candidate);
    }
    for (index, file) in impact.impacted_files.iter().enumerate() {
        candidates.push(CandidateDraft {
            source: "graph_file",
            id: format!("graph-file:{}", file.path),
            label: file.path.clone(),
            file_path: file.path.clone(),
            symbol_id: None,
            start_line: None,
            end_line: None,
            score: 620.0
                + (file.test_hint_count as f64 * 8.0)
                + (file.edge_count as f64 * 2.0)
                + (file.symbol_count as f64)
                - index as f64,
            token_count: 0,
            is_test_context: file.test_hint_count > 0 || looks_like_test(&file.path),
        });
    }
    for (index, hint) in impact.test_hints.iter().enumerate() {
        candidates.push(CandidateDraft {
            source: "graph_test",
            id: format!("graph-test:{}", hint.symbol_id),
            label: hint.symbol_name.clone(),
            file_path: hint.path.clone(),
            symbol_id: Some(hint.symbol_id.clone()),
            start_line: Some(hint.line),
            end_line: None,
            score: 700.0 - index as f64,
            token_count: 0,
            is_test_context: true,
        });
    }
}

fn symbol_candidate(symbol: &SymbolHit, index: usize) -> CandidateDraft {
    CandidateDraft {
        source: "symbol",
        id: format!("symbol:{}", symbol.id),
        label: symbol.qualified_name.clone(),
        file_path: symbol.file_path.clone(),
        symbol_id: Some(symbol.id.clone()),
        start_line: Some(symbol.start_line),
        end_line: Some(symbol.end_line),
        score: 1000.0 + symbol.score + symbol.importance_score.unwrap_or_default() - index as f64,
        token_count: estimate_token_count(&symbol.signature),
        is_test_context: looks_like_test(&symbol.file_path) || symbol.name.contains("test"),
    }
}

fn rank_candidates(
    drafts: Vec<CandidateDraft>,
    requirements: &[String],
) -> Vec<SymbolRetrievalEvalCandidate> {
    let mut best = BTreeMap::<String, CandidateDraft>::new();
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
        .map(|(index, draft)| candidate_from_draft(index + 1, draft, requirements))
        .collect()
}

fn candidate_from_draft(
    rank: usize,
    draft: CandidateDraft,
    requirements: &[String],
) -> SymbolRetrievalEvalCandidate {
    let matched_requirements = requirements
        .iter()
        .filter(|requirement| candidate_matches(&draft, requirement))
        .cloned()
        .collect::<Vec<_>>();
    SymbolRetrievalEvalCandidate {
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
        matched_requirements,
        is_test_context: draft.is_test_context,
    }
}

fn build_metrics(
    requirements: &[String],
    candidates: &[SymbolRetrievalEvalCandidate],
    k: usize,
) -> SymbolRetrievalEvalMetrics {
    let top_k = candidates.iter().take(k).collect::<Vec<_>>();
    let hit_ranks = requirement_hit_ranks(candidates, Some(k));
    let top_k_token_count = top_k
        .iter()
        .map(|candidate| candidate.token_count)
        .sum::<usize>();
    let top_k_len = top_k.len();
    SymbolRetrievalEvalMetrics {
        requirement_count: requirements.len(),
        hit_count_at_k: hit_ranks.len(),
        recall_at_k: ratio(hit_ranks.len(), requirements.len()),
        mean_reciprocal_rank: mean_reciprocal_rank(requirements, candidates),
        first_relevant_rank: candidates
            .iter()
            .find(|candidate| !candidate.matched_requirements.is_empty())
            .map(|candidate| candidate.rank),
        top_k_candidate_count: top_k_len,
        symbol_candidate_count: candidates
            .iter()
            .filter(|candidate| candidate.source == "symbol")
            .count(),
        chunk_candidate_count: candidates
            .iter()
            .filter(|candidate| candidate.source == "full_text")
            .count(),
        vector_candidate_count: candidates
            .iter()
            .filter(|candidate| candidate.source == "vector")
            .count(),
        graph_candidate_count: candidates
            .iter()
            .filter(|candidate| candidate.source.starts_with("graph_"))
            .count(),
        test_candidate_count_at_k: top_k
            .iter()
            .filter(|candidate| candidate.is_test_context)
            .count(),
        total_token_count_at_k: top_k_token_count,
        average_token_count_at_k: ratio(top_k_token_count, top_k_len),
        has_test_context_at_k: top_k.iter().any(|candidate| candidate.is_test_context),
    }
}

fn requirement_hit_ranks(
    candidates: &[SymbolRetrievalEvalCandidate],
    k: Option<usize>,
) -> BTreeMap<String, usize> {
    let mut hits = BTreeMap::new();
    for candidate in candidates {
        if k.is_some_and(|limit| candidate.rank > limit) {
            break;
        }
        for requirement in &candidate.matched_requirements {
            hits.entry(requirement.clone()).or_insert(candidate.rank);
        }
    }
    hits
}

fn mean_reciprocal_rank(
    requirements: &[String],
    candidates: &[SymbolRetrievalEvalCandidate],
) -> f64 {
    if requirements.is_empty() {
        return 0.0;
    }
    let ranks = requirement_hit_ranks(candidates, None);
    let total = requirements
        .iter()
        .map(|requirement| {
            ranks
                .get(requirement)
                .map(|rank| 1.0 / *rank as f64)
                .unwrap_or(0.0)
        })
        .sum::<f64>();
    total / requirements.len() as f64
}

fn missing_requirements(
    requirements: &[String],
    top_k: &[&SymbolRetrievalEvalCandidate],
) -> Vec<String> {
    let matched = top_k
        .iter()
        .flat_map(|candidate| candidate.matched_requirements.iter().cloned())
        .collect::<BTreeSet<_>>();
    requirements
        .iter()
        .filter(|requirement| !matched.contains(*requirement))
        .cloned()
        .collect()
}

fn compare_candidates(left: &CandidateDraft, right: &CandidateDraft) -> Ordering {
    right
        .score
        .partial_cmp(&left.score)
        .unwrap_or(Ordering::Equal)
        .then_with(|| left.file_path.cmp(&right.file_path))
        .then_with(|| left.start_line.cmp(&right.start_line))
        .then_with(|| left.label.cmp(&right.label))
}

fn candidate_key(candidate: &CandidateDraft) -> String {
    if let Some(symbol_id) = candidate.symbol_id.as_deref() {
        format!("{}:{symbol_id}", candidate.source)
    } else {
        format!(
            "{}:{}:{}",
            candidate.source, candidate.file_path, candidate.label
        )
    }
}

fn candidate_matches(candidate: &CandidateDraft, requirement: &str) -> bool {
    let requirement = normalize_match_text(requirement);
    if requirement.is_empty() {
        return false;
    }
    let haystack = normalize_match_text(&format!(
        "{} {} {} {}",
        candidate.id,
        candidate.label,
        candidate.file_path,
        candidate.symbol_id.as_deref().unwrap_or_default()
    ));
    haystack.contains(&requirement) || requirement.contains(&haystack)
}

fn clean_requirements(requirements: &[String]) -> Vec<String> {
    requirements
        .iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn clean_filter(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn looks_like_test(path: &str) -> bool {
    let path = path.to_ascii_lowercase();
    path.contains("/tests/")
        || path.ends_with("_test.rs")
        || path.ends_with("_tests.rs")
        || path.contains("tests.rs")
}

fn normalize_match_text(value: &str) -> String {
    value.replace('\\', "/").to_ascii_lowercase()
}

fn estimate_token_count(value: &str) -> usize {
    value
        .split_whitespace()
        .map(|part| (part.len() / 4).max(1))
        .sum()
}

fn ratio(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}
