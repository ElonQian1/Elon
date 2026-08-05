//! Bounded, body-free projections of feature workflow metadata.

use serde_json::{json, Value};
use std::{
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    project_document_native_context::validate_evidence_current,
    project_feature_registry::{
        ProjectFeature, ProjectFeaturePriority, ProjectFeatureRegistry, ProjectFeatureStatus,
    },
    project_feature_registry_store::{ensure_requirement_current, load_registry},
};

const MAX_CONTEXT_CANDIDATES: usize = 12;
const MAX_CONTEXT_FEATURES: usize = 3;

pub(crate) fn context_projection(workspace: &Path, query: &str, task_paths: &[String]) -> Value {
    let loaded = match load_registry(workspace) {
        Ok(value) => value,
        Err(error) => {
            return json!({"schema":"elon.project_feature_context.v1","status":"registry_invalid","selected":[],"selected_count":0,"error":error.to_string(),"source_bodies_returned":0})
        }
    };
    let broad = broad_feature_query(query);
    let mut scored = loaded
        .registry
        .features
        .iter()
        .filter(|feature| feature.status.is_context_visible())
        .filter_map(|feature| {
            let score = feature_score(feature, query, task_paths, broad);
            (score > 0).then_some((score, feature))
        })
        .collect::<Vec<_>>();
    scored.sort_by(|left, right| {
        right
            .0
            .cmp(&left.0)
            .then_with(|| left.1.priority.cmp(&right.1.priority))
            .then_with(|| right.1.updated_at_ms.cmp(&left.1.updated_at_ms))
    });
    let candidate_count = scored.len();
    let mut selected = Vec::new();
    let mut invalidated = Vec::new();
    let mut invalidated_count = 0usize;
    let mut evaluated_count = 0usize;
    for (score, feature) in scored.into_iter().take(MAX_CONTEXT_CANDIDATES) {
        evaluated_count += 1;
        if ensure_requirement_current(workspace, feature).is_err() {
            invalidated_count += 1;
            if invalidated.len() < MAX_CONTEXT_FEATURES {
                invalidated.push(json!({
                    "id":feature.id,
                    "requirement_path":feature.requirement.path,
                    "reason":"requirement_drifted",
                    "action":"Open current requirement/source and update the registry explicitly."
                }));
            }
            continue;
        }
        let blockers = dependency_blockers(&loaded.registry, feature);
        let claim_expired = feature
            .claim
            .as_ref()
            .is_some_and(|claim| claim.expires_at_ms <= now_millis());
        selected.push(json!({
            "id":feature.id,"title":feature.title,"summary":feature.summary.chars().take(280).collect::<String>(),
            "status":feature.status,"priority":feature.priority,"score":score,
            "requirement_path":feature.requirement.path,"requirement_hash":feature.requirement.content_hash,
            "knowledge_node_id":feature.knowledge_node_id,"task_paths":feature.task_paths.iter().take(4).collect::<Vec<_>>(),
            "acceptance_criteria":feature.acceptance_criteria.iter().take(3).collect::<Vec<_>>(),
            "dependency_blockers":blockers,"claimable":(feature.status == ProjectFeatureStatus::Ready || claim_expired) && blockers.is_empty()
        }));
        if selected.len() == MAX_CONTEXT_FEATURES {
            break;
        }
    }
    json!({
        "schema":"elon.project_feature_context.v1","status":"ok","registry_revision":loaded.revision,
        "candidate_count":candidate_count,"evaluated_count":evaluated_count,"evaluation_limit":MAX_CONTEXT_CANDIDATES,
        "selected_count":selected.len(),"selected":selected,
        "invalidated_count":invalidated_count,"invalidated":invalidated,"source_bodies_returned":0,
        "authority":"workflow_navigation_only","instruction":"Open the hash-bound requirement and current source/tests before editing; registry status never overrides implementation truth. Use the separate feature workflow or full governance profile for lifecycle writes; never edit registry JSON ad hoc."
    })
}

pub(crate) fn feature_snapshot(
    workspace: &Path,
    registry: &ProjectFeatureRegistry,
    feature: &ProjectFeature,
    validate_implementation: bool,
) -> Value {
    let requirement_current = if matches!(
        feature.status,
        ProjectFeatureStatus::Draft
            | ProjectFeatureStatus::Proposed
            | ProjectFeatureStatus::Retired
    ) {
        validate_evidence_current(workspace, &feature.requirement).is_ok()
    } else {
        ensure_requirement_current(workspace, feature).is_ok()
    };
    let evidence_checked = validate_implementation || feature.implementation_evidence.is_empty();
    let evidence_current = evidence_checked.then(|| {
        feature
            .implementation_evidence
            .iter()
            .all(|evidence| validate_evidence_current(workspace, evidence).is_ok())
    });
    let claim_expired = feature
        .claim
        .as_ref()
        .is_some_and(|claim| claim.expires_at_ms <= now_millis());
    let blockers = dependency_blockers(registry, feature);
    let drift_status = if !requirement_current {
        "requirement_drifted"
    } else if evidence_current == Some(false) {
        "implementation_evidence_drifted"
    } else if evidence_current.is_none() {
        "implementation_evidence_not_checked"
    } else {
        "current"
    };
    json!({
        "id":feature.id,"title":feature.title,"summary":feature.summary,"status":feature.status,"priority":feature.priority,
        "requirement_path":feature.requirement.path,"requirement_hash":feature.requirement.content_hash,"requirement_current":requirement_current,
        "knowledge_node_id":feature.knowledge_node_id,"owner":feature.owner,"tags":feature.tags,"task_paths":feature.task_paths,
        "acceptance_criteria_count":feature.acceptance_criteria.len(),"implementation_evidence_count":feature.implementation_evidence.len(),
        "implementation_evidence_current":evidence_current,"implementation_evidence_checked":evidence_checked,"dependency_blockers":blockers,
        "claim":feature.claim,"claim_expired":claim_expired,"claimable":(feature.status == ProjectFeatureStatus::Ready || claim_expired) && requirement_current && blockers.is_empty(),
        "drift_status":drift_status,"created_at_ms":feature.created_at_ms,"updated_at_ms":feature.updated_at_ms,
    })
}

pub(crate) fn dependency_blockers(
    registry: &ProjectFeatureRegistry,
    feature: &ProjectFeature,
) -> Vec<String> {
    feature
        .dependencies
        .iter()
        .filter_map(|id| {
            registry
                .features
                .iter()
                .find(|item| item.id.eq_ignore_ascii_case(id))
                .filter(|item| !item.status.is_implementation_complete())
                .map(|_| id.clone())
        })
        .collect()
}

pub(crate) fn dependency_snapshot(registry: &ProjectFeatureRegistry, id: &str) -> Value {
    registry.features.iter().find(|feature| feature.id == id)
        .map(|feature| json!({"id":feature.id,"title":feature.title,"status":feature.status,"complete":feature.status.is_implementation_complete()}))
        .unwrap_or_else(|| json!({"id":id,"status":"missing","complete":false}))
}

pub(crate) fn feature_search_text(feature: &ProjectFeature) -> String {
    format!(
        "{} {} {} {} {} {}",
        feature.id,
        feature.title,
        feature.summary,
        feature.requirement.path,
        feature.tags.join(" "),
        feature.task_paths.join(" ")
    )
    .to_ascii_lowercase()
}

fn feature_score(feature: &ProjectFeature, query: &str, task_paths: &[String], broad: bool) -> i64 {
    let terms = query_terms(query);
    let text = feature_search_text(feature);
    let mut score = if broad { 10 } else { 0 };
    for term in terms {
        if text.contains(&term) {
            score += if feature.title.to_ascii_lowercase().contains(&term) {
                12
            } else {
                5
            };
        }
    }
    for task_path in task_paths {
        if feature
            .task_paths
            .iter()
            .any(|path| path.starts_with(task_path) || task_path.starts_with(path))
        {
            score += 20;
        }
    }
    if score == 0 {
        return 0;
    }
    score
        + match feature.priority {
            ProjectFeaturePriority::P0 => 4,
            ProjectFeaturePriority::P1 => 3,
            ProjectFeaturePriority::P2 => 2,
            ProjectFeaturePriority::P3 => 1,
        }
}

fn query_terms(query: &str) -> Vec<String> {
    const STOP_WORDS: &[&str] = &[
        "feature",
        "requirement",
        "pending",
        "实现",
        "功能",
        "需求",
        "模块",
        "开发",
        "项目",
        "新的",
        "一个",
    ];
    let lower = query.to_ascii_lowercase();
    let mut terms = Vec::new();
    for token in lower.split(|ch: char| !(ch.is_alphanumeric() || ch == '_' || ch == '-')) {
        let chars = token.chars().collect::<Vec<_>>();
        if chars.len() < 2 {
            continue;
        }
        if chars
            .iter()
            .any(|ch| ('\u{4e00}'..='\u{9fff}').contains(ch))
        {
            for window in chars.windows(2) {
                let term = window.iter().collect::<String>();
                if !STOP_WORDS.contains(&term.as_str()) && !terms.contains(&term) {
                    terms.push(term);
                }
                if terms.len() == 16 {
                    return terms;
                }
            }
        } else if !STOP_WORDS.contains(&token) && !terms.iter().any(|term| term == token) {
            terms.push(token.to_string());
        }
    }
    terms.into_iter().take(16).collect()
}

fn broad_feature_query(query: &str) -> bool {
    let lower = query.to_ascii_lowercase();
    let trimmed = lower.trim();
    matches!(trimmed, "feature" | "requirement" | "需求" | "模块")
        || [
            "next",
            "pending",
            "待办",
            "下一步",
            "下一个",
            "接下来",
            "做什么",
            "新功能",
            "功能需求",
            "待开发",
            "未完成",
            "没有完成",
            "继续完善",
            "还有什么",
            "开发什么",
        ]
        .iter()
        .any(|term| lower.contains(term))
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or_default()
}
