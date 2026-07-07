use super::*;

pub(super) fn symbol_context(symbol: &SymbolHit, index: usize, profile: &HybridRankProfile) -> ContextDraft {
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
        sources: source_set("symbol"),
    }
}

pub(super) fn text_chunk_context(
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
        sources: source_set("full_text"),
    }
}

pub(super) fn vector_chunk_context(
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
        sources: source_set("vector"),
    }
}

pub(super) fn push_impact_context(
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
        candidate.sources = source_set("graph_symbol");
        drafts.push(candidate);
    }
    for (index, file) in impact.impacted_files.iter().enumerate() {
        let is_test_context = file.test_hint_count > 0 || looks_like_test(&file.path);
        drafts.push(ContextDraft {
            source: "graph_file",
            sources: source_set("graph_file"),
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
            sources: source_set("graph_test"),
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

pub(super) fn apply_plan_ranking(items: &mut [RankedContextItem], plan: &RetrievalPlan) {
    for item in items {
        let bonus = item
            .sources
            .iter()
            .map(|source| plan.ranking_bonus(source))
            .fold(plan.ranking_bonus(&item.source), f64::max);
        if bonus > 0.0 {
            item.score += bonus;
            item.reasons.push(format!(
                "retrieval_plan={} sources={} bonus={bonus:.1}",
                plan.intent.as_str(),
                item.sources.join("+")
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

pub(super) fn rerank_items(mut items: Vec<RankedContextItem>, plan: &RetrievalPlan) -> Vec<RankedContextItem> {
    items.sort_by(compare_ranked_candidates);
    apply_rerank_diversity(&mut items);
    items.sort_by(compare_ranked_candidates);
    for (index, item) in items.iter_mut().enumerate() {
        item.rank = index + 1;
        item.decision = decide_rerank_item(item, plan);
        item.reasons
            .push(format!("rerank_decision={}", item.decision.as_str()));
    }
    items
}

pub(super) fn apply_rerank_diversity(items: &mut [RankedContextItem]) {
    let mut file_counts = BTreeMap::<String, usize>::new();
    for item in items {
        let count = file_counts.entry(item.file_path.clone()).or_default();
        *count += 1;
        if *count > 3 && item.token_count > 0 {
            item.score -= 90.0 + ((*count - 3) as f64 * 20.0);
            item.reasons.push(format!(
                "rerank_diversity_file_cap file={} ordinal={}",
                item.file_path, count
            ));
        }
    }
}

pub(super) fn decide_rerank_item(item: &RankedContextItem, plan: &RetrievalPlan) -> RerankDecision {
    if item.is_test_context && !plan.pack_policy.include_tests {
        return RerankDecision::Drop;
    }
    if item.rank <= 3
        && (item.score >= 120.0
            || item_has_source(item, "symbol")
            || item_has_source(item, "graph_symbol"))
    {
        return RerankDecision::MustInclude;
    }
    if item.rank <= 8 {
        return RerankDecision::Include;
    }
    if item.rank <= 20 && item.score > 0.0 {
        return RerankDecision::Summarize;
    }
    RerankDecision::Drop
}

pub(super) fn item_has_source(item: &RankedContextItem, expected: &str) -> bool {
    item.source == expected || item.sources.iter().any(|source| source == expected)
}

pub(super) fn compare_ranked_candidates(left: &RankedContextItem, right: &RankedContextItem) -> Ordering {
    right
        .score
        .partial_cmp(&left.score)
        .unwrap_or(Ordering::Equal)
        .then_with(|| left.file_path.cmp(&right.file_path))
        .then_with(|| left.start_line.cmp(&right.start_line))
        .then_with(|| left.label.cmp(&right.label))
}

pub(super) fn rank_drafts(drafts: Vec<ContextDraft>) -> Vec<RankedContextItem> {
    let mut best = BTreeMap::<String, ContextDraft>::new();
    for draft in drafts {
        let key = candidate_key(&draft);
        if let Some(existing) = best.get_mut(&key) {
            merge_context_draft(existing, draft);
        } else {
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

pub(super) fn ranked_item(rank: usize, draft: ContextDraft) -> RankedContextItem {
    RankedContextItem {
        rank,
        source: draft.source.to_string(),
        sources: draft.sources.into_iter().collect(),
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
        decision: RerankDecision::Include,
    }
}

pub(super) fn compare_candidates(left: &ContextDraft, right: &ContextDraft) -> Ordering {
    right
        .score
        .partial_cmp(&left.score)
        .unwrap_or(Ordering::Equal)
        .then_with(|| left.file_path.cmp(&right.file_path))
        .then_with(|| left.start_line.cmp(&right.start_line))
        .then_with(|| left.label.cmp(&right.label))
}

pub(super) fn candidate_key(candidate: &ContextDraft) -> String {
    if let Some(symbol_id) = candidate.symbol_id.as_deref() {
        format!("symbol:{symbol_id}")
    } else if let Some(start_line) = candidate.start_line {
        format!(
            "range:{}:{}:{}",
            candidate.file_path,
            start_line,
            candidate.end_line.unwrap_or(start_line)
        )
    } else {
        format!("file-label:{}:{}", candidate.file_path, candidate.label)
    }
}

pub(super) fn merge_context_draft(existing: &mut ContextDraft, incoming: ContextDraft) {
    let mut sources = existing.sources.clone();
    sources.extend(incoming.sources.iter().cloned());
    let source_reason = if sources.len() > 1 {
        Some(format!(
            "merged_sources={}",
            sources.iter().cloned().collect::<Vec<_>>().join("+")
        ))
    } else {
        None
    };
    let reasons = merge_strings(
        existing
            .reasons
            .iter()
            .cloned()
            .chain(incoming.reasons.iter().cloned())
            .chain(source_reason),
    );
    let matched_terms = merge_strings(
        existing
            .matched_terms
            .iter()
            .cloned()
            .chain(incoming.matched_terms.iter().cloned()),
    );

    if incoming.score > existing.score {
        let fallback_token_count = existing.token_count;
        *existing = incoming;
        existing.sources = sources;
        existing.reasons = reasons;
        existing.matched_terms = matched_terms;
        if existing.token_count == 0 {
            existing.token_count = fallback_token_count;
        }
    } else {
        existing.sources = sources;
        existing.reasons = reasons;
        existing.matched_terms = matched_terms;
        if existing.token_count == 0 && incoming.token_count > 0 {
            existing.token_count = incoming.token_count;
        }
    }
}

pub(super) fn compact_reasons(reasons: impl IntoIterator<Item = Option<String>>) -> Vec<String> {
    reasons
        .into_iter()
        .flatten()
        .map(|reason| reason.trim().to_string())
        .filter(|reason| !reason.is_empty())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

pub(super) fn merge_strings(values: impl IntoIterator<Item = String>) -> Vec<String> {
    values
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

pub(super) fn source_set(source: &str) -> BTreeSet<String> {
    BTreeSet::from([source.to_string()])
}

pub(super) fn looks_like_test(path: &str) -> bool {
    let path = path.to_ascii_lowercase();
    path.contains("/tests/")
        || path.ends_with("_test.rs")
        || path.ends_with("_tests.rs")
        || path.contains("tests.rs")
}

pub(super) fn estimate_token_count(value: &str) -> usize {
    value
        .split_whitespace()
        .map(|part| (part.len() / 4).max(1))
        .sum()
}
