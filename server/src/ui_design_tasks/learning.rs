use anyhow::Result;
use serde_json::Value;

use crate::store::{
    Store, UiLearnedRoute, UiRouteLearningEntry, UiRouteLearningSource,
};

const TASK_MARKER_BEGIN: &str = "<elon-ui-design-task version=\"1\">";
const TASK_MARKER_END: &str = "</elon-ui-design-task>";

pub(crate) fn finalize_ui_route_learning(
    store: &Store,
    project_id: &str,
    user_id: &str,
    prompt: &str,
    codex_jsonl: &str,
    exit_ok: bool,
) -> Result<Option<UiRouteLearningEntry>> {
    let Some((origin, phrase)) = learning_metadata(prompt) else {
        return Ok(None);
    };
    if !matches!(
        origin.as_str(),
        "ambiguous_local"
            | "codex_rescue"
            | "local_confirmed"
            | "active_library"
            | "active_cluster"
            | "cluster_conflict"
    ) {
        return Ok(None);
    }
    let observation = observe_codex_route(codex_jsonl);
    let Some(learned_route) = observation.learned_route else {
        return Ok(None);
    };
    if !matches!(origin.as_str(), "ambiguous_local" | "codex_rescue")
        && learned_route == UiLearnedRoute::Ui
    {
        return Ok(None);
    }
    let candidate = store.record_ui_route_candidate(
        project_id,
        Some(user_id),
        &phrase,
        learned_route,
        observation.confidence,
        observation.reason.as_deref().unwrap_or("ui_confirm_route"),
    )?;
    if !exit_ok {
        return Ok(Some(candidate));
    }
    let verified = match learned_route {
        UiLearnedRoute::Ui => observation.ui_execution_evidence,
        UiLearnedRoute::NonUi => observation.non_ui_source_change,
    };
    if !verified {
        return Ok(Some(candidate));
    }
    let evidence = match learned_route {
        UiLearnedRoute::Ui => "Codex route confirmation plus successful UI Runtime/FitRun tool",
        UiLearnedRoute::NonUi => "Codex route confirmation plus successful non-UI source change",
    };
    store
        .confirm_ui_route_learning(
            project_id,
            Some(user_id),
            &phrase,
            learned_route,
            UiRouteLearningSource::ExecutionVerified,
            evidence,
        )
        .map(Some)
}

fn learning_metadata(prompt: &str) -> Option<(String, String)> {
    let (_, rest) = prompt.rsplit_once(TASK_MARKER_BEGIN)?;
    let (json, _) = rest.split_once(TASK_MARKER_END)?;
    let envelope: Value = serde_json::from_str(json.trim()).ok()?;
    let origin = envelope
        .pointer("/task/route_learning_origin")
        .or_else(|| envelope.pointer("/task/routeLearningOrigin"))?
        .as_str()?
        .trim()
        .to_string();
    let phrase = envelope
        .pointer("/task/route_learning_phrase")
        .or_else(|| envelope.pointer("/task/routeLearningPhrase"))?
        .as_str()?
        .trim()
        .chars()
        .take(2_000)
        .collect::<String>();
    (!phrase.is_empty()).then_some((origin, phrase))
}

#[derive(Default)]
struct RouteObservation {
    learned_route: Option<UiLearnedRoute>,
    reason: Option<String>,
    confidence: f64,
    ui_execution_evidence: bool,
    non_ui_source_change: bool,
}

fn observe_codex_route(jsonl: &str) -> RouteObservation {
    let mut observation = RouteObservation::default();
    for line in jsonl.lines() {
        let Ok(event) = serde_json::from_str::<Value>(line.trim()) else {
            continue;
        };
        let event_type = event.get("type").and_then(Value::as_str).unwrap_or_default();
        let Some(item) = event.get("item") else {
            continue;
        };
        let item_type = item.get("type").and_then(Value::as_str).unwrap_or_default();
        let tool_name = item
            .get("tool")
            .or_else(|| item.get("name"))
            .or_else(|| item.get("tool_name"))
            .and_then(Value::as_str)
            .unwrap_or_default();
        if tool_name.ends_with("ui_confirm_route") {
            if let Some(arguments) = tool_arguments(item) {
                observation.learned_route = match arguments.get("route").and_then(Value::as_str) {
                    Some("UI_DESIGN") => Some(UiLearnedRoute::Ui),
                    Some("NON_UI") => Some(UiLearnedRoute::NonUi),
                    _ => observation.learned_route,
                };
                observation.reason = arguments
                    .get("reason")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(|value| value.chars().take(500).collect());
                observation.confidence = arguments
                    .get("confidence")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.5)
                    .clamp(0.0, 1.0);
            }
        }
        if event_type == "item.completed" && item_type == "file_change" {
            observation.non_ui_source_change = item_succeeded(item);
        }
        if event_type == "item.completed"
            && item_succeeded(item)
            && is_ui_execution_evidence(tool_name)
        {
            observation.ui_execution_evidence = true;
        }
    }
    observation
}

fn tool_arguments(item: &Value) -> Option<Value> {
    let value = item
        .get("arguments")
        .or_else(|| item.get("args"))
        .or_else(|| item.get("input"))?;
    if let Some(text) = value.as_str() {
        serde_json::from_str(text).ok()
    } else {
        Some(value.clone())
    }
}

fn item_succeeded(item: &Value) -> bool {
    !matches!(
        item.get("status").and_then(Value::as_str),
        Some("failed" | "error" | "cancelled")
    )
}

fn is_ui_execution_evidence(tool: &str) -> bool {
    [
        "ui_apply_live_patch",
        "ui_prepare_debug_runtime",
        "ui_create_compose_screen_scaffold",
        "ui_bind_target_design",
        "ui_start_fit_run",
        "ui_run_visual_solver",
        "ui_commit_bound_styles",
        "ui_build_and_verify",
    ]
    .iter()
    .any(|name| tool.ends_with(name))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> Store {
        let path = std::env::temp_dir().join(format!(
            "elon-ui-route-learning-{}.db",
            uuid::Uuid::new_v4().simple()
        ));
        Store::open(&path).unwrap()
    }

    #[test]
    fn observes_ui_confirmation_and_runtime_evidence() {
        let output = concat!(
            r#"{"type":"item.started","item":{"type":"mcp_tool_call","name":"yilong_ui_live.ui_confirm_route","arguments":{"route":"UI_DESIGN","reason":"视觉层级调整","confidence":0.91}}}"#,
            "\n",
            r#"{"type":"item.completed","item":{"type":"mcp_tool_call","name":"yilong_ui_live.ui_apply_live_patch","status":"completed"}}"#
        );
        let observed = observe_codex_route(output);
        assert_eq!(observed.learned_route, Some(UiLearnedRoute::Ui));
        assert!(observed.ui_execution_evidence);
        assert_eq!(observed.reason.as_deref(), Some("视觉层级调整"));
    }

    #[test]
    fn model_confirmation_without_execution_is_not_verified() {
        let output = r#"{"type":"item.started","item":{"type":"mcp_tool_call","name":"ui_confirm_route","arguments":{"route":"NON_UI","reason":"点击逻辑"}}}"#;
        let observed = observe_codex_route(output);
        assert_eq!(observed.learned_route, Some(UiLearnedRoute::NonUi));
        assert!(!observed.non_ui_source_change);
    }

    #[test]
    fn codex_rescue_activates_only_after_successful_ui_execution_evidence() {
        let store = store();
        let prompt = super::super::dispatch::promote_codex_ui_route("让操作区更有呼吸感")
            .unwrap();
        let confirmation_only = r#"{"type":"item.started","item":{"type":"mcp_tool_call","name":"ui_confirm_route","arguments":{"route":"UI_DESIGN","reason":"视觉间距任务","confidence":0.9}}}"#;
        let candidate = finalize_ui_route_learning(
            &store,
            "project-1",
            "user-1",
            &prompt,
            confirmation_only,
            true,
        )
        .unwrap()
        .unwrap();
        assert_eq!(candidate.status, "candidate");
        assert!(store
            .lookup_ui_route_learning("project-1", "让操作区更有呼吸感")
            .unwrap()
            .is_none());

        let verified = concat!(
            r#"{"type":"item.started","item":{"type":"mcp_tool_call","name":"ui_confirm_route","arguments":{"route":"UI_DESIGN","reason":"视觉间距任务","confidence":0.9}}}"#,
            "\n",
            r#"{"type":"item.completed","item":{"type":"mcp_tool_call","name":"ui_apply_live_patch","status":"completed"}}"#
        );
        let active = finalize_ui_route_learning(
            &store,
            "project-1",
            "user-1",
            &prompt,
            verified,
            true,
        )
        .unwrap()
        .unwrap();
        assert_eq!(active.status, "active");
        assert_eq!(active.source, "execution_verified");
        assert_eq!(
            store
                .lookup_ui_route_learning("project-1", "让操作区更有呼吸感")
                .unwrap()
                .unwrap()
                .learned_route,
            UiLearnedRoute::Ui
        );
    }
}
