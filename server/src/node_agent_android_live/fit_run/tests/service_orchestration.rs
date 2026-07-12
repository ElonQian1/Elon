use std::sync::atomic::Ordering;
use std::sync::Arc;

use serde_json::json;

use super::super::model::{FitCommand, FitRunPhase};
use super::super::orchestrator::FitSourceVerifyResult;
use super::super::store::FitRunStore;
use super::super::FitRunService;
use super::fixtures::{candidate, cleanup, context, request, result, run, FakeBackend};

#[tokio::test]
async fn local_first_plateau_creates_codex_handoff_and_commands_are_idempotent() {
    let (root, run) = run(false);
    let context = context(root.to_str().unwrap());
    let mut request = request(false);
    request.thresholds.plateau_window = 1;
    request.budget.max_no_improvement_trials = 2;
    let backend = Arc::new(FakeBackend::new(
        vec![result("baseline", 0.50)],
        vec![result("local", 0.50)],
    ));
    let service = FitRunService::new(FitRunStore::new(), backend);
    let created = service.create_run(context.clone(), request).await.unwrap();
    let command = FitCommand::Start {
        command_id: "start-1".to_string(),
    };
    let first = service
        .command(context.clone(), &created.run_id, command)
        .await
        .unwrap();
    assert_eq!(first.run.phase, FitRunPhase::AwaitingCodex);
    assert!(first
        .run
        .handoff
        .as_ref()
        .and_then(|value| value.artifact_path.as_ref())
        .is_some_and(|path| std::path::Path::new(path).is_file()));
    let repeated = service
        .command(
            context,
            &created.run_id,
            FitCommand::Start {
                command_id: "start-1".to_string(),
            },
        )
        .await
        .unwrap();
    assert!(repeated.idempotent);
    assert_eq!(repeated.run.phase, FitRunPhase::AwaitingCodex);
    drop(run);
    cleanup(root);
}

#[tokio::test]
async fn accepted_requires_target_score_and_source_parity() {
    let (root, _) = run(false);
    let context = context(root.to_str().unwrap());
    let mut verified = candidate("source-verified", 0.01);
    verified.source_parity_loss = Some(0.01);
    verified.source_parity_verified = true;
    let backend = Arc::new(
        FakeBackend::new(vec![result("baseline", 0.01)], Vec::new()).with_verify(
            FitSourceVerifyResult {
                candidate: verified,
                duration_ms: 1,
            },
        ),
    );
    let service = FitRunService::new(FitRunStore::new(), backend);
    let created = service
        .create_run(context.clone(), request(false))
        .await
        .unwrap();
    let ready = service
        .command(
            context.clone(),
            &created.run_id,
            FitCommand::Start {
                command_id: "start-ready".to_string(),
            },
        )
        .await
        .unwrap();
    assert_eq!(ready.run.phase, FitRunPhase::CandidateReady);
    let accepted = service
        .command(
            context.clone(),
            &created.run_id,
            FitCommand::AcceptBest {
                command_id: "accept-best".to_string(),
            },
        )
        .await
        .unwrap();
    assert_eq!(accepted.run.phase, FitRunPhase::Accepted);
    assert!(root.join(".elon/ui-standards/fit-cases.v1.json").is_file());
    assert!(service
        .command(
            context.clone(),
            &created.run_id,
            FitCommand::RebindSession {
                command_id: "rebind-terminal".to_string(),
                new_session_id: context.session_id,
                new_runtime_node_id: None,
                new_current_rect: None,
            },
        )
        .await
        .is_err());
    cleanup(root);
}

#[tokio::test]
async fn interrupted_accept_command_can_resume_source_verification() {
    let (root, _) = run(false);
    let context = context(root.to_str().unwrap());
    let mut verified = candidate("source-verified-after-resume", 0.01);
    verified.source_parity_loss = Some(0.01);
    verified.source_parity_verified = true;
    let backend = Arc::new(
        FakeBackend::new(vec![result("baseline", 0.01)], Vec::new()).with_verify(
            FitSourceVerifyResult {
                candidate: verified,
                duration_ms: 1,
            },
        ),
    );
    let store = FitRunStore::new();
    let service = FitRunService::new(store.clone(), backend);
    let created = service
        .create_run(context.clone(), request(false))
        .await
        .unwrap();
    let ready = service
        .command(
            context.clone(),
            &created.run_id,
            FitCommand::Start {
                command_id: "start-interrupted-accept".to_string(),
            },
        )
        .await
        .unwrap();
    assert_eq!(ready.run.phase, FitRunPhase::CandidateReady);

    let mut stranded = ready.run;
    stranded.transition(FitRunPhase::SourceVerifying).unwrap();
    stranded.record_command("accept-interrupted".to_string());
    store.save(&stranded).unwrap();

    let mut resumed_context = context;
    resumed_context.source_revision = Some("source-written-by-interrupted-accept".to_string());

    let recovered = service
        .command(
            resumed_context,
            &stranded.run_id,
            FitCommand::AcceptBest {
                command_id: "accept-interrupted".to_string(),
            },
        )
        .await
        .unwrap();
    assert!(recovered.idempotent);
    assert_eq!(recovered.run.phase, FitRunPhase::Accepted);
    cleanup(root);
}

#[tokio::test]
async fn commands_require_original_session_until_explicit_rebind() {
    let (root, _) = run(false);
    let original = context(root.to_str().unwrap());
    let mut replacement = original.clone();
    replacement.session_id = "live_replacement".to_string();
    replacement.device_id = "device-2".to_string();
    let backend = Arc::new(FakeBackend::new(Vec::new(), Vec::new()));
    let service = FitRunService::new(FitRunStore::new(), backend);
    let created = service.create_run(original, request(false)).await.unwrap();
    assert!(service
        .command(
            replacement.clone(),
            &created.run_id,
            FitCommand::Start {
                command_id: "wrong-session".to_string(),
            },
        )
        .await
        .is_err());
    let rebound = service
        .command(
            replacement.clone(),
            &created.run_id,
            FitCommand::RebindSession {
                command_id: "explicit-rebind".to_string(),
                new_session_id: replacement.session_id.clone(),
                new_runtime_node_id: None,
                new_current_rect: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(rebound.run.session_id, replacement.session_id);
    assert_eq!(rebound.run.device_id, replacement.device_id);
    cleanup(root);
}

#[tokio::test]
async fn cancel_reverts_the_fit_run_before_entering_terminal_state() {
    let (root, mut run) = run(false);
    let context = context(root.to_str().unwrap());
    let mut best = candidate("best", 0.20);
    best.operations = vec![json!({
        "property": "height",
        "value": {"type": "dp", "value": 56.0},
        "beforeValue": {"type": "dp", "value": 48.0}
    })];
    run.best = Some(best);
    FitRunStore::new().save(&run).unwrap();
    let backend = Arc::new(FakeBackend::new(Vec::new(), Vec::new()));
    let service = FitRunService::new(FitRunStore::new(), backend.clone());
    let cancelled = service
        .command(
            context,
            &run.run_id,
            FitCommand::Cancel {
                command_id: "cancel-run".to_string(),
            },
        )
        .await
        .unwrap();
    assert_eq!(cancelled.run.phase, FitRunPhase::Cancelled);
    assert_eq!(backend.revert_calls.load(Ordering::SeqCst), 1);
    cleanup(root);
}
