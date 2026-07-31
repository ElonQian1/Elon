use serde::{Deserialize, Serialize};

pub(crate) const RESOURCE_CLASSES: [&str; 4] =
    ["own_codex", "remote_node", "shared_codex", "platform_model"];

#[derive(Debug, Clone, Serialize)]
pub(crate) struct AiResourcePolicy {
    pub project_id: String,
    pub enabled_classes: Vec<String>,
    pub priority: Vec<String>,
    pub allow_fallback: bool,
    pub privacy_mode: String,
    pub max_estimated_unit_cost_micros: Option<i64>,
    pub updated_by_user_id: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct UpdateAiResourcePolicy {
    pub enabled_classes: Vec<String>,
    pub priority: Vec<String>,
    #[serde(default)]
    pub allow_fallback: bool,
    pub privacy_mode: String,
    #[serde(default)]
    pub max_estimated_unit_cost_micros: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct AiResourceSummary {
    pub resource_id: String,
    pub resource_class: String,
    pub label: String,
    pub provider: String,
    pub model: Option<String>,
    pub availability: String,
    pub execution_scope: String,
    pub cost_basis: String,
    pub quota_state: String,
    pub task_kinds: Vec<String>,
    pub estimated_unit_cost_micros: Option<i64>,
    pub evidence: Vec<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct AiResourceOverview {
    pub schema: &'static str,
    pub project_id: String,
    pub policy: AiResourcePolicy,
    pub resources: Vec<AiResourceSummary>,
    pub cautions: Vec<&'static str>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct AiRoutePreviewRequest {
    pub task_kind: String,
    #[serde(default)]
    pub preferred_model: Option<String>,
    #[serde(default)]
    pub require_local_execution: bool,
}

#[derive(Debug, Serialize)]
pub(crate) struct AiRoutePreview {
    pub schema: &'static str,
    pub project_id: String,
    pub task_kind: String,
    pub selected: Option<AiResourceSummary>,
    pub fallbacks: Vec<AiResourceSummary>,
    pub reasons: Vec<String>,
    pub execution_started: bool,
    pub quota_verified: bool,
}

pub(crate) fn default_policy(project_id: &str, user_id: &str) -> AiResourcePolicy {
    AiResourcePolicy {
        project_id: project_id.to_string(),
        enabled_classes: RESOURCE_CLASSES
            .iter()
            .map(|value| value.to_string())
            .collect(),
        priority: RESOURCE_CLASSES
            .iter()
            .map(|value| value.to_string())
            .collect(),
        allow_fallback: true,
        privacy_mode: "prefer_local".to_string(),
        max_estimated_unit_cost_micros: None,
        updated_by_user_id: user_id.to_string(),
        created_at: String::new(),
        updated_at: String::new(),
    }
}
