use std::fs;

use serde_json::{json, Value};

use super::{broker::LiveUiBroker, design_tools};

fn fixture_root(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "elon_design_workflow_{name}_{}",
        uuid::Uuid::new_v4().simple()
    ))
}

async fn fixture_session(
    name: &str,
) -> (
    std::path::PathBuf,
    std::sync::Arc<super::broker::LiveUiSession>,
) {
    let root = fixture_root(name);
    fs::create_dir_all(root.join("web")).unwrap();
    fs::write(
        root.join("web/package.json"),
        r#"{"scripts":{"dev":"vite"}}"#,
    )
    .unwrap();
    let session = LiveUiBroker::new()
        .create_session(
            format!("design-workflow-{name}"),
            "ui.design.workflow.test".to_string(),
            Some(root.display().to_string()),
            38917,
        )
        .await;
    (root, session)
}

async fn call(session: &super::broker::LiveUiSession, name: &str, arguments: Value) -> Value {
    design_tools::call(session, name, arguments)
        .await
        .unwrap_or_else(|error| panic!("{name} failed: {error:#}"))
}

#[tokio::test]
async fn capabilities_prove_v112_without_opening_pc_canvas() {
    let (root, session) = fixture_session("capabilities").await;

    let result = call(&session, "ui_get_design_capabilities", json!({})).await;

    assert_eq!(result["runtimeSchema"], "yilong-ui-live@1.12.0");
    assert_eq!(result["protocolRevision"], "1.12");
    assert_eq!(result["contentEmbedded"], false);
    assert!(result["capabilityIds"]
        .as_array()
        .unwrap()
        .iter()
        .any(|value| value == "DESIGN_INTENT_EXECUTION_LIFECYCLE"));
    assert!(result["capabilityIds"]
        .as_array()
        .unwrap()
        .iter()
        .any(|value| value == "REVIEWED_DETERMINISTIC_SOURCE_PATCHES"));
    assert!(result["capabilityIds"]
        .as_array()
        .unwrap()
        .iter()
        .any(|value| value == "VISUAL_SEMANTIC_REGRESSION_CONTRACTS"));
    assert!(result["capabilityIds"]
        .as_array()
        .unwrap()
        .iter()
        .any(|value| value == "NODE_LOCAL_REGRESSION_COMPARATOR"));
    assert_eq!(result["project"]["detectedPlatforms"][0], "web");
    fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn intent_plan_enforces_revision_receipts_and_replanning() {
    let (root, session) = fixture_session("intent-lifecycle").await;
    let task_id = "headless-design-contract";
    let opened = call(
        &session,
        "ui_open_design_target",
        json!({"platform":"web","route":"/settings"}),
    )
    .await;
    let design_session_id = opened["session"]["designSessionId"]
        .as_str()
        .unwrap()
        .to_string();
    let planned = call(
        &session,
        "ui_plan_design_intent",
        json!({
            "intent":"修改 Web 设置页标题",
            "taskId":task_id,
            "platform":"web",
            "route":"/settings",
            "designSessionId":design_session_id,
        }),
    )
    .await;
    assert_eq!(planned["plan"]["status"], "PLANNED");
    assert_eq!(planned["plan"]["needsClarification"], false);
    let plan_id = planned["plan"]["planId"].as_str().unwrap().to_string();

    let started = call(
        &session,
        "ui_start_design_intent_plan",
        json!({
            "planId":plan_id,
            "expectedRevision":1,
            "taskId":task_id,
            "designSessionId":design_session_id,
        }),
    )
    .await;
    assert_eq!(started["plan"]["status"], "RUNNING");
    assert_eq!(started["plan"]["revision"], 2);
    assert_eq!(started["taskBinding"]["binding"]["status"], "ACTIVE");

    let running = call(
        &session,
        "ui_record_design_intent_action",
        json!({
            "planId":plan_id,
            "expectedRevision":2,
            "actionOrder":1,
            "status":"RUNNING",
            "summary":"已恢复后台会话",
        }),
    )
    .await;
    assert_eq!(running["plan"]["actionReceipts"][0]["attempt"], 1);
    let succeeded = call(
        &session,
        "ui_record_design_intent_action",
        json!({
            "planId":plan_id,
            "expectedRevision":3,
            "actionOrder":1,
            "status":"SUCCEEDED",
            "evidenceRefs":[format!("design-session:{design_session_id}")],
        }),
    )
    .await;
    assert_eq!(succeeded["plan"]["revision"], 4);

    let stale = design_tools::call(
        &session,
        "ui_transition_design_intent_plan",
        json!({"planId":plan_id,"expectedRevision":3,"transition":"PAUSE"}),
    )
    .await
    .unwrap_err();
    assert!(stale
        .to_string()
        .contains("DESIGN_INTENT_REVISION_CONFLICT"));

    let paused = call(
        &session,
        "ui_transition_design_intent_plan",
        json!({"planId":plan_id,"expectedRevision":4,"transition":"PAUSE"}),
    )
    .await;
    assert_eq!(paused["plan"]["status"], "PAUSED");
    let resumed = call(
        &session,
        "ui_transition_design_intent_plan",
        json!({"planId":plan_id,"expectedRevision":5,"transition":"RESUME"}),
    )
    .await;
    assert_eq!(resumed["plan"]["status"], "RUNNING");
    let canceled = call(
        &session,
        "ui_transition_design_intent_plan",
        json!({
            "planId":plan_id,
            "expectedRevision":6,
            "transition":"CANCEL",
            "reason":"契约测试结束",
        }),
    )
    .await;
    assert_eq!(canceled["plan"]["status"], "CANCELED");
    assert_eq!(canceled["taskBinding"]["status"], "SETTLED");
    assert_eq!(
        canceled["taskBinding"]["result"]["binding"]["status"],
        "SETTLED"
    );

    let replanned = call(
        &session,
        "ui_replan_design_intent",
        json!({
            "planId":plan_id,
            "expectedRevision":7,
            "intent":"把 Web 设置页标题修改为账号设置",
        }),
    )
    .await;
    assert_eq!(replanned["previousPlan"]["status"], "SUPERSEDED");
    assert_eq!(replanned["plan"]["replannedFrom"], plan_id);
    assert_eq!(replanned["plan"]["status"], "PLANNED");
    fs::remove_dir_all(root).unwrap();
}
