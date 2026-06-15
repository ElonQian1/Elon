use std::{
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
    symbol_index_rank_profile::infer_rank_profile,
    symbol_index_ranker::{rank_hybrid_context_with_plan, RankedContextItem},
    symbol_index_retrieval_plan::build_retrieval_plan,
    symbol_index_vector::{search_latest_symbol_vectors, SymbolVectorSearchQuery},
};

pub(crate) use super::symbol_index_eval_types::SymbolRetrievalEvalQuery as RetrievalEvalQuery;

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
    let vector_model = clean_filter(query.vector_model.as_deref());
    let retrieval_plan = build_retrieval_plan(&text, vector_model.is_some());
    let planned_symbol_limit = query.planned_symbol_limit(&retrieval_plan);
    let planned_chunk_limit = query.planned_chunk_limit(&retrieval_plan);
    let planned_vector_limit = query.planned_vector_limit(&retrieval_plan);
    let planned_depth = query.planned_depth(&retrieval_plan);

    let symbol_response = search_latest_symbol_index(
        data_dir,
        &SymbolIndexSearch {
            trace_id: query.trace_id.clone(),
            text: Some(text.clone()),
            include_edges: false,
            limit: planned_symbol_limit,
            ..Default::default()
        },
    )?;
    let chunk_hits = search_latest_symbol_chunks(
        data_dir,
        &SymbolChunkSearch {
            trace_id: query.trace_id.clone(),
            text: Some(text.clone()),
            limit: planned_chunk_limit,
            ..Default::default()
        },
    )
    .map(|response| response.chunks)
    .unwrap_or_default();
    let vector_hits = if retrieval_plan.retrievers.vector {
        let Some(vector_model) = vector_model.as_deref() else {
            unreachable!("vector retriever requires a requested vector model");
        };
        search_latest_symbol_vectors(
            data_dir,
            &SymbolVectorSearchQuery {
                trace_id: query.trace_id.clone(),
                text: Some(text.clone()),
                model: Some(vector_model.to_string()),
                limit: planned_vector_limit,
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
                depth: planned_depth,
                limit: query.impact_limit(),
                ..Default::default()
            },
        )
        .ok()
    });

    let requirements = clean_requirements(&query.must_include);
    let ranking_profile = infer_rank_profile(&text);
    let ranked_context = rank_hybrid_context_with_plan(
        &symbol_response.symbols,
        &chunk_hits,
        &vector_hits,
        impact.as_ref(),
        &ranking_profile,
        &retrieval_plan,
    );
    let candidates = ranked_context
        .into_iter()
        .map(|item| candidate_from_ranked(item, &requirements))
        .collect::<Vec<_>>();
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
            symbol_limit: planned_symbol_limit,
            chunk_limit: planned_chunk_limit,
            vector_model: query.vector_model.clone(),
            vector_limit: planned_vector_limit,
            depth: impact
                .as_ref()
                .map(|impact| impact.query.depth)
                .unwrap_or(planned_depth),
            impact_limit: query.impact_limit(),
        },
        metadata: symbol_response.metadata,
        retrieval_plan,
        ranking_profile,
        metrics,
        candidates,
        missing_requirements,
    })
}

fn candidate_from_ranked(
    item: RankedContextItem,
    requirements: &[String],
) -> SymbolRetrievalEvalCandidate {
    let matched_requirements = requirements
        .iter()
        .filter(|requirement| candidate_matches(&item, requirement))
        .cloned()
        .collect::<Vec<_>>();
    SymbolRetrievalEvalCandidate {
        rank: item.rank,
        source: item.source,
        id: item.id,
        label: item.label,
        file_path: item.file_path,
        symbol_id: item.symbol_id,
        start_line: item.start_line,
        end_line: item.end_line,
        score: item.score,
        token_count: item.token_count,
        matched_terms: item.matched_terms,
        reasons: item.reasons,
        matched_requirements,
        is_test_context: item.is_test_context,
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

fn candidate_matches(candidate: &RankedContextItem, requirement: &str) -> bool {
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

fn normalize_match_text(value: &str) -> String {
    value.replace('\\', "/").to_ascii_lowercase()
}

fn ratio(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}
