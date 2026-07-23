use std::sync::Arc;

use chrono::{Duration, Utc};

use super::super::live_artifacts::build_verify_request;
use super::super::model::{
    AttachStateReplayRequest, FitRunAuditOutcome, FitRunDocument, FitRunPhase, FitStateReplay,
};
use super::super::store::FitRunStore;
use super::super::FitRunService;
use super::fixtures::{cleanup, context, request, FakeBackend};
use crate::node_agent_android_live::broker::LiveUiBroker;
use crate::node_agent_android_live::protocol::{LiveGeometry, LiveRect, LiveUiNode};

#[tokio::test]
async fn legacy_manifest_without_scenario_migrates_atomically_and_drives_build_replay() {
    let harness = Harness::new(true).await;
    let manifest = harness
        .root
        .join(".elon/ui-tuner/fit-runs")
        .join(&harness.run.run_id)
        .join("manifest.json");
    let mut legacy: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&manifest).unwrap()).unwrap();
    legacy
        .pointer_mut("/environment")
        .and_then(serde_json::Value::as_object_mut)
        .unwrap()
        .remove("stateReplay");
    legacy.as_object_mut().unwrap().remove("auditEvents");
    std::fs::write(&manifest, serde_json::to_vec_pretty(&legacy).unwrap()).unwrap();
    let result = harness
        .service
        .attach_state_replay(
            harness.context.clone(),
            &harness.run.run_id,
            harness.attach_request(replay(Duration::minutes(10))),
        )
        .await
        .unwrap();

    assert!(!result.idempotent);
    let loaded = FitRunStore::new()
        .load(harness.root.to_str().unwrap(), &harness.run.run_id)
        .unwrap();
    assert_eq!(loaded.environment.scenario.as_deref(), Some("CHAT_PAGE"));
    assert_eq!(
        loaded.audit_events.last().unwrap().outcome,
        FitRunAuditOutcome::Attached
    );
    let verification = build_verify_request(&loaded).unwrap();
    assert_eq!(verification.state_replay.unwrap().scenario_id, "CHAT_PAGE");
    harness.cleanup();
}

#[tokio::test]
async fn identical_attachment_is_idempotent_and_audited() {
    let harness = Harness::new(true).await;
    let state_replay = replay(Duration::minutes(10));
    harness
        .service
        .attach_state_replay(
            harness.context.clone(),
            &harness.run.run_id,
            harness.attach_request(state_replay.clone()),
        )
        .await
        .unwrap();
    let repeated = harness
        .service
        .attach_state_replay(
            harness.context.clone(),
            &harness.run.run_id,
            harness.attach_request(state_replay),
        )
        .await
        .unwrap();

    assert!(repeated.idempotent);
    assert_eq!(
        repeated.run.audit_events.last().unwrap().outcome,
        FitRunAuditOutcome::Idempotent
    );
    harness.cleanup();
}

#[tokio::test]
async fn conflicting_attachment_is_rejected_without_overwrite_and_leaves_audit() {
    let harness = Harness::new(true).await;
    let original = replay(Duration::minutes(10));
    harness
        .service
        .attach_state_replay(
            harness.context.clone(),
            &harness.run.run_id,
            harness.attach_request(original.clone()),
        )
        .await
        .unwrap();
    let mut conflicting = replay(Duration::minutes(20));
    conflicting.steps[0].name = "different-explicit-step".into();
    let error = harness
        .service
        .attach_state_replay(
            harness.context.clone(),
            &harness.run.run_id,
            harness.attach_request(conflicting),
        )
        .await
        .unwrap_err()
        .to_string();

    assert!(error.contains("FIT_STATE_REPLAY_CONFLICT"));
    let loaded = FitRunStore::new()
        .load(harness.root.to_str().unwrap(), &harness.run.run_id)
        .unwrap();
    assert_eq!(loaded.environment.state_replay.as_ref(), Some(&original));
    assert_eq!(
        loaded.audit_events.last().unwrap().outcome,
        FitRunAuditOutcome::RejectedConflict
    );
    assert!(loaded
        .audit_events
        .last()
        .unwrap()
        .previous_replay_sha256
        .is_some());
    harness.cleanup();
}

#[tokio::test]
async fn expired_attachment_is_rejected_and_audited() {
    let harness = Harness::new(true).await;
    let captured_at = Utc::now() - Duration::minutes(20);
    let mut expired = replay(Duration::minutes(10));
    expired.captured_at = captured_at.to_rfc3339();
    expired.expires_at = (captured_at + Duration::minutes(10)).to_rfc3339();
    let error = harness
        .service
        .attach_state_replay(
            harness.context.clone(),
            &harness.run.run_id,
            harness.attach_request(expired),
        )
        .await
        .unwrap_err()
        .to_string();

    assert!(error.contains("FIT_STATE_REPLAY_EXPIRED"));
    let loaded = FitRunStore::new()
        .load(harness.root.to_str().unwrap(), &harness.run.run_id)
        .unwrap();
    assert!(loaded.environment.state_replay.is_none());
    assert_eq!(
        loaded.audit_events.last().unwrap().outcome,
        FitRunAuditOutcome::RejectedInvalid
    );
    harness.cleanup();
}

#[tokio::test]
async fn missing_current_target_is_rejected_and_audited() {
    let harness = Harness::new(false).await;
    let error = harness
        .service
        .attach_state_replay(
            harness.context.clone(),
            &harness.run.run_id,
            harness.attach_request(replay(Duration::minutes(10))),
        )
        .await
        .unwrap_err()
        .to_string();

    assert!(error.contains("FIT_STATE_REPLAY_TARGET_MISSING"));
    let loaded = FitRunStore::new()
        .load(harness.root.to_str().unwrap(), &harness.run.run_id)
        .unwrap();
    assert_eq!(
        loaded.audit_events.last().unwrap().outcome,
        FitRunAuditOutcome::RejectedTargetMissing
    );
    harness.cleanup();
}

#[tokio::test]
async fn terminal_run_and_cross_project_request_are_rejected() {
    let mut harness = Harness::new(true).await;
    harness.run.phase = FitRunPhase::Accepted;
    FitRunStore::new().save(&harness.run).unwrap();
    let terminal_error = harness
        .service
        .attach_state_replay(
            harness.context.clone(),
            &harness.run.run_id,
            harness.attach_request(replay(Duration::minutes(10))),
        )
        .await
        .unwrap_err()
        .to_string();
    assert!(terminal_error.contains("FIT_STATE_REPLAY_RUN_IMMUTABLE"));

    let other_root = std::env::temp_dir().join(format!(
        "fit-replay-other-project-{}",
        uuid::Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(&other_root).unwrap();
    let mut cross_project = harness.attach_request(replay(Duration::minutes(10)));
    cross_project.project_root = other_root.display().to_string();
    let project_error = harness
        .service
        .attach_state_replay(harness.context.clone(), &harness.run.run_id, cross_project)
        .await
        .unwrap_err()
        .to_string();
    assert!(project_error.contains("FIT_STATE_REPLAY_PROJECT_MISMATCH"));
    let _ = std::fs::remove_dir_all(other_root);
    harness.cleanup();
}

#[tokio::test]
async fn attachment_rejects_a_different_live_session_owner() {
    let harness = Harness::new(true).await;
    let mut foreign_context = harness.context.clone();
    foreign_context.session_id = "live_foreign".into();
    let error = harness
        .service
        .attach_state_replay(
            foreign_context,
            &harness.run.run_id,
            harness.attach_request(replay(Duration::minutes(10))),
        )
        .await
        .unwrap_err()
        .to_string();

    assert!(error.contains("FIT_STATE_REPLAY_SESSION_MISMATCH"));
    let loaded = FitRunStore::new()
        .load(harness.root.to_str().unwrap(), &harness.run.run_id)
        .unwrap();
    assert!(loaded.environment.state_replay.is_none());
    harness.cleanup();
}

struct Harness {
    root: std::path::PathBuf,
    run: FitRunDocument,
    context: super::super::model::FitSessionContext,
    service: FitRunService,
}

impl Harness {
    async fn new(include_target: bool) -> Self {
        let root = std::env::temp_dir().join(format!(
            "fit-replay-attachment-{}",
            uuid::Uuid::new_v4().simple()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let broker = Arc::new(LiveUiBroker::new());
        let session = broker
            .create_session(
                "device-1".into(),
                "com.example.test".into(),
                Some(root.display().to_string()),
                38917,
            )
            .await;
        let mut context = context(root.to_str().unwrap());
        context.session_id = session.id.clone();
        let run = FitRunDocument::new(context.clone(), request(false)).unwrap();
        let nodes = include_target
            .then(|| vec![target_node(&run)])
            .unwrap_or_default();
        session
            .set_runtime_state_for_test(nodes, Some("build-1".into()))
            .await;
        FitRunStore::new().save(&run).unwrap();
        let backend = Arc::new(FakeBackend::new(Vec::new(), Vec::new()));
        let service =
            FitRunService::new(FitRunStore::new(), backend).with_live_broker(broker.clone());
        Self {
            root,
            run,
            context,
            service,
        }
    }

    fn attach_request(&self, state_replay: FitStateReplay) -> AttachStateReplayRequest {
        AttachStateReplayRequest {
            command_id: format!("attach_{}", uuid::Uuid::new_v4().simple()),
            project_root: self.root.display().to_string(),
            scenario: "CHAT_PAGE".into(),
            state_replay,
            target_runtime_node_id: self.run.pair.runtime_node_id.clone(),
            target_definition_id: self.run.pair.definition_id.clone(),
            target_instance_key: self.run.pair.instance_key.clone(),
        }
    }

    fn cleanup(self) {
        cleanup(self.root);
    }
}

fn replay(valid_for: Duration) -> FitStateReplay {
    let captured_at = Utc::now();
    serde_json::from_value(serde_json::json!({
        "scenarioId":"CHAT_PAGE",
        "capturedAt":captured_at.to_rfc3339(),
        "expiresAt":(captured_at + valid_for).to_rfc3339(),
        "steps":[
            {
                "name":"open-chat-tab",
                "action":{
                    "type":"ACTIVATE_NODE",
                    "definitionId":"home.navigation.chat",
                    "occurrence":0
                }
            },
            {"name":"wait-chat-content","action":{"type":"WAIT","durationMs":500}}
        ]
    }))
    .unwrap()
}

fn target_node(run: &FitRunDocument) -> LiveUiNode {
    LiveUiNode {
        runtime_node_id: run.pair.runtime_node_id.clone(),
        definition_id: run.pair.definition_id.clone(),
        instance_key: run.pair.instance_key.clone(),
        parent_runtime_node_id: None,
        screen_id: "chat".into(),
        kind: "button".into(),
        text: None,
        resource_id: None,
        class_name: "Button".into(),
        source: None,
        geometry: LiveGeometry {
            bounds_in_display_px: LiveRect {
                left: 0,
                top: 0,
                right: 100,
                bottom: 40,
                width: 100,
                height: 40,
            },
            density: 1.0,
            font_scale: 1.0,
            rotation: 0,
            visible: true,
        },
        properties: Default::default(),
        capabilities: Default::default(),
    }
}
