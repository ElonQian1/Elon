use serde::Serialize;

use super::{
    symbol_index_query_features::{analyze_query_features, QueryFeatures},
    symbol_index_rank_profile::{infer_rank_profile, HybridRankProfile},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum QueryIntent {
    Locate,
    Explain,
    DebugError,
    ModifyBehavior,
    Refactor,
    AddFeature,
    Test,
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RetrieverSwitches {
    pub(crate) symbol: bool,
    pub(crate) full_text: bool,
    pub(crate) graph: bool,
    pub(crate) repo_map: bool,
    pub(crate) vector: bool,
    pub(crate) recent_files: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RetrievalWeights {
    pub(crate) symbol: f64,
    pub(crate) full_text: f64,
    pub(crate) graph: f64,
    pub(crate) repo_map: f64,
    pub(crate) vector: f64,
    pub(crate) recent_files: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GraphPolicy {
    pub(crate) include_callers: bool,
    pub(crate) include_callees: bool,
    pub(crate) include_tests: bool,
    pub(crate) include_types: bool,
    pub(crate) include_implementations: bool,
    pub(crate) include_references: bool,
    pub(crate) include_error_mappers: bool,
    pub(crate) max_depth: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PackPolicy {
    pub(crate) include_repo_map_slice: bool,
    pub(crate) include_signatures: bool,
    pub(crate) include_code_snippets: bool,
    pub(crate) include_tests: bool,
    pub(crate) include_error_mapping: bool,
    pub(crate) prefer_exact_snippets: bool,
    pub(crate) prefer_summaries_for_large_files: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RetrievalPlan {
    pub(crate) intent: QueryIntent,
    pub(crate) features: QueryFeatures,
    pub(crate) retrievers: RetrieverSwitches,
    pub(crate) weights: RetrievalWeights,
    pub(crate) graph_policy: GraphPolicy,
    pub(crate) pack_policy: PackPolicy,
    pub(crate) ranking_profile: HybridRankProfile,
    pub(crate) reasons: Vec<String>,
}

pub(crate) fn build_retrieval_plan(query: &str, vector_requested: bool) -> RetrievalPlan {
    let features = analyze_query_features(query);
    let intent = detect_intent(&features);
    let mut plan = plan_for_intent(intent);
    plan.features = features;
    plan.ranking_profile = infer_rank_profile(query);
    if vector_requested {
        plan.retrievers.vector = true;
        let vector_weight = requested_vector_weight(intent);
        plan.weights.vector = plan.weights.vector.max(vector_weight);
        plan.reasons.push("vector_enabled_by_request".to_string());
        plan.reasons
            .push(format!("vector_weight={vector_weight:.2}"));
    } else {
        plan.reasons.push("vector_disabled_no_model".to_string());
    }
    plan
}

pub(crate) fn render_retrieval_plan(plan: &RetrievalPlan) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "<retrieval_plan intent=\"{}\">\n",
        plan.intent.as_str()
    ));
    out.push_str(&format!(
        "- retrievers: symbol={} full_text={} graph={} repo_map={} vector={} recent_files={}\n",
        on_off(plan.retrievers.symbol),
        on_off(plan.retrievers.full_text),
        on_off(plan.retrievers.graph),
        on_off(plan.retrievers.repo_map),
        on_off(plan.retrievers.vector),
        on_off(plan.retrievers.recent_files)
    ));
    out.push_str(&format!(
        "- weights: symbol={:.2} full_text={:.2} graph={:.2} repo_map={:.2} vector={:.2} recent_files={:.2}\n",
        plan.weights.symbol,
        plan.weights.full_text,
        plan.weights.graph,
        plan.weights.repo_map,
        plan.weights.vector,
        plan.weights.recent_files
    ));
    out.push_str(&format!(
        "- graph_policy: callers={} callees={} tests={} types={} implementations={} references={} error_mappers={} max_depth={}\n",
        on_off(plan.graph_policy.include_callers),
        on_off(plan.graph_policy.include_callees),
        on_off(plan.graph_policy.include_tests),
        on_off(plan.graph_policy.include_types),
        on_off(plan.graph_policy.include_implementations),
        on_off(plan.graph_policy.include_references),
        on_off(plan.graph_policy.include_error_mappers),
        plan.graph_policy.max_depth
    ));
    out.push_str(&format!(
        "- pack_policy: repo_map={} signatures={} snippets={} tests={} error_mapping={} exact_snippets={} large_file_summaries={}\n",
        on_off(plan.pack_policy.include_repo_map_slice),
        on_off(plan.pack_policy.include_signatures),
        on_off(plan.pack_policy.include_code_snippets),
        on_off(plan.pack_policy.include_tests),
        on_off(plan.pack_policy.include_error_mapping),
        on_off(plan.pack_policy.prefer_exact_snippets),
        on_off(plan.pack_policy.prefer_summaries_for_large_files)
    ));
    if !plan.features.status_codes.is_empty() {
        out.push_str(&format!(
            "- status_codes: {}\n",
            plan.features
                .status_codes
                .iter()
                .map(|code| code.to_string())
                .collect::<Vec<_>>()
                .join(",")
        ));
    }
    if !plan.features.error_like_terms.is_empty() {
        out.push_str(&format!(
            "- error_terms: {}\n",
            plan.features.error_like_terms.join(",")
        ));
    }
    if !plan.features.symbol_like_terms.is_empty() {
        out.push_str(&format!(
            "- symbol_like_terms: {}\n",
            plan.features.symbol_like_terms.join(",")
        ));
    }
    if !plan.reasons.is_empty() {
        out.push_str(&format!("- reasons: {}\n", plan.reasons.join("; ")));
    }
    out.push_str("</retrieval_plan>\n");
    out
}

impl QueryIntent {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            QueryIntent::Locate => "locate",
            QueryIntent::Explain => "explain",
            QueryIntent::DebugError => "debug_error",
            QueryIntent::ModifyBehavior => "modify_behavior",
            QueryIntent::Refactor => "refactor",
            QueryIntent::AddFeature => "add_feature",
            QueryIntent::Test => "test",
            QueryIntent::Unknown => "unknown",
        }
    }
}

impl RetrievalPlan {
    pub(crate) fn source_weight(&self, source: &str) -> f64 {
        match source {
            "symbol" => self.weights.symbol,
            "full_text" | "text" | "chunk" => self.weights.full_text,
            "vector" => self.weights.vector,
            "graph" | "graph_symbol" | "graph_file" | "graph_test" => self.weights.graph,
            "repo_map" => self.weights.repo_map,
            "recent_files" | "recent" => self.weights.recent_files,
            _ => 0.0,
        }
    }

    pub(crate) fn planned_limit(
        &self,
        requested: usize,
        default_limit: usize,
        max_limit: usize,
        source: &str,
    ) -> usize {
        if requested > 0 {
            return requested.min(max_limit).max(1);
        }

        let weight = self.source_weight(source);
        let limit = if weight >= 0.35 {
            default_limit + ((default_limit + 1) / 2)
        } else if weight <= 0.15 {
            default_limit.saturating_sub((default_limit + 3) / 4)
        } else {
            default_limit
        };
        limit.min(max_limit).max(1)
    }

    pub(crate) fn planned_graph_depth(&self, requested: usize) -> usize {
        if requested > 0 {
            requested
        } else {
            self.graph_policy.max_depth.max(1)
        }
    }

    pub(crate) fn ranking_bonus(&self, source: &str) -> f64 {
        self.source_weight(source) * 250.0
    }
}

fn detect_intent(features: &QueryFeatures) -> QueryIntent {
    if features.mentions_refactor {
        QueryIntent::Refactor
    } else if features.mentions_error || !features.status_codes.is_empty() {
        QueryIntent::DebugError
    } else if features.mentions_test {
        QueryIntent::Test
    } else if features.mentions_add_feature {
        QueryIntent::AddFeature
    } else if features.mentions_modify {
        QueryIntent::ModifyBehavior
    } else if features.mentions_explain {
        QueryIntent::Explain
    } else if features.mentions_locate
        || !features.symbol_like_terms.is_empty()
        || !features.file_like_terms.is_empty()
    {
        QueryIntent::Locate
    } else {
        QueryIntent::Unknown
    }
}

fn requested_vector_weight(intent: QueryIntent) -> f64 {
    match intent {
        QueryIntent::Explain | QueryIntent::AddFeature => 0.20,
        QueryIntent::Unknown => 0.15,
        QueryIntent::Locate
        | QueryIntent::DebugError
        | QueryIntent::ModifyBehavior
        | QueryIntent::Refactor
        | QueryIntent::Test => 0.10,
    }
}

fn plan_for_intent(intent: QueryIntent) -> RetrievalPlan {
    let (weights, graph_policy, pack_policy, reasons) = match intent {
        QueryIntent::Locate => (
            weights(0.40, 0.25, 0.15, 0.15, 0.00, 0.05),
            graph_policy(true, true, false, true, false, false, false, 1),
            pack_policy(true, true, true, false, false, true, true),
            vec!["intent=locate_prefers_exact_symbols_and_paths".to_string()],
        ),
        QueryIntent::Explain => (
            weights(0.20, 0.20, 0.30, 0.25, 0.00, 0.05),
            graph_policy(true, true, true, true, false, false, false, 1),
            pack_policy(true, true, false, true, false, false, true),
            vec!["intent=explain_prefers_repo_map_and_graph".to_string()],
        ),
        QueryIntent::DebugError => (
            weights(0.25, 0.35, 0.30, 0.05, 0.00, 0.05),
            graph_policy(true, true, true, true, false, true, true, 1),
            pack_policy(true, true, true, true, true, true, true),
            vec!["intent=debug_error_prefers_fts_errors_graph_tests".to_string()],
        ),
        QueryIntent::ModifyBehavior => (
            weights(0.35, 0.20, 0.30, 0.05, 0.00, 0.10),
            graph_policy(true, true, true, true, false, true, true, 1),
            pack_policy(true, true, true, true, true, true, true),
            vec!["intent=modify_behavior_prefers_target_graph_tests".to_string()],
        ),
        QueryIntent::Refactor => (
            weights(0.25, 0.15, 0.45, 0.05, 0.00, 0.10),
            graph_policy(true, true, true, true, true, true, false, 2),
            pack_policy(true, true, true, true, false, true, true),
            vec!["intent=refactor_prefers_references_implementations_api".to_string()],
        ),
        QueryIntent::AddFeature => (
            weights(0.20, 0.20, 0.25, 0.25, 0.00, 0.10),
            graph_policy(true, true, true, true, true, false, true, 1),
            pack_policy(true, true, true, true, true, false, true),
            vec!["intent=add_feature_prefers_similar_patterns_and_project_shape".to_string()],
        ),
        QueryIntent::Test => (
            weights(0.25, 0.25, 0.30, 0.05, 0.00, 0.15),
            graph_policy(true, true, true, true, false, true, false, 1),
            pack_policy(true, true, true, true, false, true, true),
            vec!["intent=test_prefers_test_hints_and_assertions".to_string()],
        ),
        QueryIntent::Unknown => (
            weights(0.30, 0.25, 0.25, 0.10, 0.00, 0.10),
            graph_policy(true, true, true, true, false, false, false, 1),
            pack_policy(true, true, true, true, false, true, true),
            vec!["intent=unknown_uses_balanced_plan".to_string()],
        ),
    };

    RetrievalPlan {
        intent,
        features: QueryFeatures::empty(),
        retrievers: RetrieverSwitches {
            symbol: true,
            full_text: true,
            graph: true,
            repo_map: true,
            vector: false,
            recent_files: true,
        },
        weights,
        graph_policy,
        pack_policy,
        ranking_profile: infer_rank_profile(""),
        reasons,
    }
}

fn weights(
    symbol: f64,
    full_text: f64,
    graph: f64,
    repo_map: f64,
    vector: f64,
    recent_files: f64,
) -> RetrievalWeights {
    RetrievalWeights {
        symbol,
        full_text,
        graph,
        repo_map,
        vector,
        recent_files,
    }
}

#[allow(clippy::too_many_arguments)]
fn graph_policy(
    include_callers: bool,
    include_callees: bool,
    include_tests: bool,
    include_types: bool,
    include_implementations: bool,
    include_references: bool,
    include_error_mappers: bool,
    max_depth: usize,
) -> GraphPolicy {
    GraphPolicy {
        include_callers,
        include_callees,
        include_tests,
        include_types,
        include_implementations,
        include_references,
        include_error_mappers,
        max_depth,
    }
}

fn pack_policy(
    include_repo_map_slice: bool,
    include_signatures: bool,
    include_code_snippets: bool,
    include_tests: bool,
    include_error_mapping: bool,
    prefer_exact_snippets: bool,
    prefer_summaries_for_large_files: bool,
) -> PackPolicy {
    PackPolicy {
        include_repo_map_slice,
        include_signatures,
        include_code_snippets,
        include_tests,
        include_error_mapping,
        prefer_exact_snippets,
        prefer_summaries_for_large_files,
    }
}

fn on_off(value: bool) -> &'static str {
    if value {
        "on"
    } else {
        "off"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_error_plan_prefers_full_text_graph_and_error_context() {
        let plan = build_retrieval_plan("登录失败为什么返回 500？", false);

        assert_eq!(plan.intent, QueryIntent::DebugError);
        assert!(plan.features.status_codes.contains(&500));
        assert!(plan.graph_policy.include_error_mappers);
        assert!(plan.pack_policy.include_tests);
        assert!(plan.weights.full_text > plan.weights.symbol);
        assert!(!plan.retrievers.vector);
    }

    #[test]
    fn refactor_plan_uses_deeper_references_policy() {
        let plan = build_retrieval_plan("重构 AuthService::login callers", false);

        assert_eq!(plan.intent, QueryIntent::Refactor);
        assert!(plan
            .features
            .symbol_like_terms
            .contains(&"AuthService::login".to_string()));
        assert!(plan.graph_policy.include_references);
        assert!(plan.graph_policy.include_implementations);
        assert_eq!(plan.graph_policy.max_depth, 2);
    }

    #[test]
    fn plan_defaults_drive_depth_and_limits_but_respect_explicit_values() {
        let plan = build_retrieval_plan("重构 AuthService::login callers", false);

        assert_eq!(plan.planned_graph_depth(0), 2);
        assert_eq!(plan.planned_graph_depth(1), 1);
        assert_eq!(plan.planned_limit(0, 8, 20, "graph"), 12);
        assert_eq!(plan.planned_limit(5, 8, 20, "graph"), 5);
    }

    #[test]
    fn render_plan_exposes_selected_strategy() {
        let plan = build_retrieval_plan("新增 refresh token", true);
        let rendered = render_retrieval_plan(&plan);

        assert_eq!(plan.intent, QueryIntent::AddFeature);
        assert!(plan.weights.vector > 0.0);
        assert!(rendered.contains("<retrieval_plan intent=\"add_feature\">"));
        assert!(rendered.contains("vector=on"));
        assert!(rendered.contains("pack_policy"));
    }
}
