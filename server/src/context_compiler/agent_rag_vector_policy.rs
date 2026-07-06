use serde::Serialize;

use super::{
    symbol_index_retrieval_plan::{build_retrieval_plan, QueryIntent},
    symbol_index_vector_types::LOCAL_HASH_VECTOR_MODEL,
};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AgentVectorPolicy {
    pub(crate) enabled: bool,
    pub(crate) explicit: bool,
    pub(crate) intent: QueryIntent,
    pub(crate) model: Option<String>,
    pub(crate) reason: &'static str,
}

pub(crate) fn choose_agent_vector_policy(
    query: &str,
    explicit_use_vector: Option<bool>,
    requested_model: Option<String>,
    configured_model: Option<String>,
) -> AgentVectorPolicy {
    let intent = build_retrieval_plan(query, false).intent;
    match explicit_use_vector {
        Some(false) => AgentVectorPolicy {
            enabled: false,
            explicit: true,
            intent,
            model: None,
            reason: "disabled_by_tool_argument",
        },
        Some(true) => AgentVectorPolicy {
            enabled: true,
            explicit: true,
            intent,
            model: Some(default_model(requested_model, configured_model)),
            reason: "enabled_by_tool_argument",
        },
        None => {
            let enabled = auto_enable_vector(intent);
            AgentVectorPolicy {
                enabled,
                explicit: false,
                intent,
                model: enabled.then(|| default_model(requested_model, configured_model)),
                reason: auto_reason(intent, enabled),
            }
        }
    }
}

fn default_model(requested_model: Option<String>, configured_model: Option<String>) -> String {
    requested_model
        .or(configured_model)
        .unwrap_or_else(|| LOCAL_HASH_VECTOR_MODEL.to_string())
}

fn auto_enable_vector(intent: QueryIntent) -> bool {
    matches!(
        intent,
        QueryIntent::Explain | QueryIntent::AddFeature | QueryIntent::Unknown
    )
}

fn auto_reason(intent: QueryIntent, enabled: bool) -> &'static str {
    match (intent, enabled) {
        (QueryIntent::Explain, true) => "auto_enabled_for_explain_semantic_recall",
        (QueryIntent::AddFeature, true) => "auto_enabled_for_add_feature_pattern_recall",
        (QueryIntent::Unknown, true) => "auto_enabled_for_unknown_query_recall",
        (QueryIntent::Locate, false) => "auto_disabled_for_locate_exact_symbols",
        (QueryIntent::DebugError, false) => "auto_disabled_for_debug_error_precision",
        (QueryIntent::ModifyBehavior, false) => "auto_disabled_for_modify_behavior_precision",
        (QueryIntent::Refactor, false) => "auto_disabled_for_refactor_graph_priority",
        (QueryIntent::Test, false) => "auto_disabled_for_test_context_precision",
        _ => "auto_vector_policy",
    }
}


#[cfg(test)]
#[path = "agent_rag_vector_policy_tests.rs"]
mod tests;
