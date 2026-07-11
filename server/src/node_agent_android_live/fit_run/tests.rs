use std::collections::VecDeque;
use std::fs;
use std::io::Write;
use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Result};
use serde_json::json;

use super::model::{
    CreateFitRunRequest, FitBudget, FitCandidate, FitCommand, FitEnvironment, FitRect,
    FitRunDocument, FitRunPhase, FitScore, FitSessionContext, FitTargetPair, FitThresholds,
    FitTrial, FitTrialKind,
};
use super::orchestrator::{
    FitBackendResult, FitRunBackend, FitRunBackendFuture, FitSourceVerifyResult,
};
use super::store::FitRunStore;
use super::FitRunService;
use crate::node_agent_android_live::fit_learning::{record_and_promote, FitUserDecision};

#[test]
fn state_machine_rejects_illegal_transition() {
    let (_root, mut run) = run(false);
    assert!(run.transition(FitRunPhase::Accepted).is_err());
    run.transition(FitRunPhase::Baselining).unwrap();
    run.transition(FitRunPhase::LocalSolving).unwrap();
    assert_eq!(run.phase, FitRunPhase::LocalSolving);
}

#[test]
fn worse_candidate_never_replaces_best() {
    let (_root, mut run) = run(false);
    assert!(run.consider_candidate(candidate("best", 0.20)));
    assert!(!run.consider_candidate(candidate("worse", 0.30)));
    assert_eq!(run.best.as_ref().unwrap().trial_id, "best");
    assert_eq!(run.current.as_ref().unwrap().trial_id, "worse");
}

#[test]
fn store_falls_back_to_atomic_manifest_backup() {
    let (root, mut run) = run(false);
    let store = FitRunStore::new();
    store.save(&run).unwrap();
    run.transition(FitRunPhase::Baselining).unwrap();
    store.save(&run).unwrap();
    let manifest = root
        .join(".elon/ui-tuner/fit-runs")
        .join(&run.run_id)
        .join("manifest.json");
    fs::write(&manifest, b"not-json").unwrap();
    let loaded = store.load(root.to_str().unwrap(), &run.run_id).unwrap();
    assert_eq!(loaded.phase, FitRunPhase::Created);
    cleanup(root);
}

#[test]
fn trial_journal_reconciles_manifest_after_interrupted_checkpoint() {
    let (root, mut run) = run(false);
    let store = FitRunStore::new();
    store.save(&run).unwrap();
    run.transition(FitRunPhase::Baselining).unwrap();
    let trial = FitTrial {
        sequence: run.next_sequence(),
        trial_id: "recovery-1".to_string(),
        kind: FitTrialKind::Baseline,
        created_at: chrono::Utc::now().to_rfc3339(),
        duration_ms: 1,
        evaluations: 1,
        candidate: None,
        accepted_as_best: false,
        error: None,
        checkpoint: run.checkpoint(),
    };
    store.append_trial(&run, &trial).unwrap();
    assert_eq!(store.read_trials(&run).unwrap().len(), 1);
    assert_eq!(store.list_for_project(root.to_str().unwrap()).unwrap().len(), 1);
    let loaded = store.load(root.to_str().unwrap(), &run.run_id).unwrap();
    assert_eq!(loaded.phase, FitRunPhase::Baselining);
    assert_eq!(loaded.last_sequence, 1);
    cleanup(root);
}

#[test]
fn truncated_trial_tail_is_repaired_before_next_append() {
    let (root, mut run) = run(false);
    let store = FitRunStore::new();
    store.save(&run).unwrap();
    run.transition(FitRunPhase::Baselining).unwrap();
    let first = FitTrial {
        sequence: run.next_sequence(),
        trial_id: "recovery-1".to_string(),
        kind: FitTrialKind::Baseline,
        created_at: chrono::Utc::now().to_rfc3339(),
        duration_ms: 1,
        evaluations: 1,
        candidate: None,
        accepted_as_best: false,
        error: None,
        checkpoint: run.checkpoint(),
    };
    store.append_trial(&run, &first).unwrap();
    let journal = root
        .join(".elon/ui-tuner/fit-runs")
        .join(&run.run_id)
        .join("trials.jsonl");
    let mut file = fs::OpenOptions::new().append(true).open(&journal).unwrap();
    file.write_all(br#"{"partial"#).unwrap();
    file.sync_all().unwrap();
    drop(file);

    assert_eq!(store.read_trials(&run).unwrap().len(), 1);
    let second = FitTrial {
        sequence: run.next_sequence(),
        trial_id: "recovery-2".to_string(),
        ..first.clone()
    };
    store.append_trial(&run, &second).unwrap();
    let recovered = store.read_trials(&run).unwrap();
    assert_eq!(recovered.len(), 2);
    assert_eq!(recovered[1].trial_id, "recovery-2");
    cleanup(root);
}

#[test]
fn corrupt_run_is_reported_instead_of_silently_disappearing_from_list() {
    let (root, mut run) = run(false);
    let store = FitRunStore::new();
    store.save(&run).unwrap();
    run.transition(FitRunPhase::Baselining).unwrap();
    store.save(&run).unwrap();
    let dir = root.join(".elon/ui-tuner/fit-runs").join(&run.run_id);
    fs::write(dir.join("manifest.json"), b"bad-main").unwrap();
    fs::write(dir.join("manifest.json.bak"), b"bad-backup").unwrap();
    let error = store.list_for_project(root.to_str().unwrap()).unwrap_err();
    assert!(format!("{error:#}").contains("不能在列表中静默跳过"));
    cleanup(root);
}

#[test]
fn persisted_solver_operation_keeps_baseline_and_promotes_non_zero_delta() {
    let (root, mut run) = run(false);
    run.environment.density = Some(2.0);
    run.properties = vec!["height".to_string()];
    let store = FitRunStore::new();
    store.save(&run).unwrap();
    let mut solved = candidate("local", 0.01);
    solved.operations = vec![json!({
        "property": "height",
        "value": { "type": "dp", "value": 27.0 }
    })];
    let trial = FitTrial {
        sequence: run.next_sequence(),
        trial_id: solved.trial_id.clone(),
        kind: FitTrialKind::LiveApply,
        created_at: chrono::Utc::now().to_rfc3339(),
        duration_ms: 1,
        evaluations: 1,
        candidate: Some(solved.clone()),
        accepted_as_best: true,
        error: None,
        checkpoint: run.checkpoint(),
    };
    store.append_trial(&run, &trial).unwrap();
    let persisted = store.read_trials(&run).unwrap();
    assert_eq!(
        persisted[0].candidate.as_ref().unwrap().operations[0]
            .pointer("/beforeValue/value")
            .and_then(serde_json::Value::as_f64),
        Some(20.0)
    );

    solved.source_parity_verified = true;
    solved.source_parity_loss = Some(0.01);
    run.baseline = Some(candidate("baseline", 0.20));
    run.current = Some(solved.clone());
    run.best = Some(solved);
    run.phase = FitRunPhase::Accepted;
    let learning = record_and_promote(
        &run,
        &persisted,
        FitUserDecision::Accepted,
        Some("真实 solver operation".to_string()),
    )
    .unwrap();
    assert_eq!(
        learning.promotion.document.priors[0]
            .median_deltas
            .get("height"),
        Some(&7.0)
    );
    cleanup(root);
}

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
            context,
            &created.run_id,
            FitCommand::AcceptBest {
                command_id: "accept-best".to_string(),
            },
        )
        .await
        .unwrap();
    assert_eq!(accepted.run.phase, FitRunPhase::Accepted);
    assert!(root.join(".elon/ui-standards/fit-cases.v1.json").is_file());
    cleanup(root);
}

struct FakeBackend {
    baseline: Mutex<VecDeque<FitBackendResult>>,
    local: Mutex<VecDeque<FitBackendResult>>,
    verify: Mutex<Option<FitSourceVerifyResult>>,
}

impl FakeBackend {
    fn new(baseline: Vec<FitBackendResult>, local: Vec<FitBackendResult>) -> Self {
        Self {
            baseline: Mutex::new(baseline.into()),
            local: Mutex::new(local.into()),
            verify: Mutex::new(None),
        }
    }

    fn with_verify(self, result: FitSourceVerifyResult) -> Self {
        *self.verify.lock().unwrap() = Some(result);
        self
    }

    fn pop(queue: &Mutex<VecDeque<FitBackendResult>>) -> Result<FitBackendResult> {
        queue
            .lock()
            .unwrap()
            .pop_front()
            .ok_or_else(|| anyhow!("fake backend queue exhausted"))
    }
}

impl FitRunBackend for FakeBackend {
    fn capture_baseline<'a>(
        &'a self,
        _run: FitRunDocument,
    ) -> FitRunBackendFuture<'a, FitBackendResult> {
        let result = Self::pop(&self.baseline);
        Box::pin(async move { result })
    }

    fn solve_local<'a>(
        &'a self,
        _run: FitRunDocument,
    ) -> FitRunBackendFuture<'a, FitBackendResult> {
        let result = Self::pop(&self.local);
        Box::pin(async move { result })
    }

    fn evaluate_after_codex<'a>(
        &'a self,
        _run: FitRunDocument,
    ) -> FitRunBackendFuture<'a, FitBackendResult> {
        Box::pin(async { Err(anyhow!("not expected")) })
    }

    fn verify_source<'a>(
        &'a self,
        _run: FitRunDocument,
    ) -> FitRunBackendFuture<'a, FitSourceVerifyResult> {
        let result = self
            .verify
            .lock()
            .unwrap()
            .take()
            .ok_or_else(|| anyhow!("not expected"));
        Box::pin(async move { result })
    }

    fn reapply_best<'a>(&'a self, _run: FitRunDocument) -> FitRunBackendFuture<'a, ()> {
        Box::pin(async { Ok(()) })
    }

    fn revert_best<'a>(&'a self, _run: FitRunDocument) -> FitRunBackendFuture<'a, ()> {
        Box::pin(async { Ok(()) })
    }
}

fn result(id: &str, loss: f64) -> FitBackendResult {
    FitBackendResult {
        candidate: candidate(id, loss),
        evaluations: 1,
        duration_ms: 1,
    }
}

fn candidate(id: &str, loss: f64) -> FitCandidate {
    FitCandidate {
        trial_id: id.to_string(),
        score: FitScore {
            scorer_version: "test".to_string(),
            overall_loss: loss,
            geometry_error: loss,
            color_error: loss,
            edge_error: loss,
            alpha_error: 0.0,
            shape_error: None,
            typography_error: None,
            hard_failures: Vec::new(),
        },
        operations: Vec::new(),
        screenshot_path: None,
        diff_artifact_path: None,
        runtime_build_id: Some("build-1".to_string()),
        source_revision: Some("source-1".to_string()),
        commit_id: None,
        source_parity_loss: None,
        source_parity_verified: false,
    }
}

fn run(auto_start: bool) -> (std::path::PathBuf, FitRunDocument) {
    let root = std::env::temp_dir().join(format!("fit-run-test-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&root).unwrap();
    let context = context(root.to_str().unwrap());
    let run = FitRunDocument::new(context, request(auto_start)).unwrap();
    (root, run)
}

fn context(project_root: &str) -> FitSessionContext {
    FitSessionContext {
        session_id: "live_test".to_string(),
        project_root: project_root.to_string(),
        package_name: "com.example.test".to_string(),
        device_id: "device-1".to_string(),
        runtime_build_id: Some("build-1".to_string()),
        tree_revision: 1,
        source_revision: Some("source-1".to_string()),
    }
}

fn request(auto_start: bool) -> CreateFitRunRequest {
    CreateFitRunRequest {
        pair: FitTargetPair {
            target_design_id: "target-1".to_string(),
            target_sha256: "abc123".to_string(),
            target_rect: rect(0, 0, 100, 40),
            runtime_node_id: "node-1".to_string(),
            definition_id: "screen.button".to_string(),
            component_kind: Some("button".to_string()),
            parent_layout_kind: Some("column".to_string()),
            instance_key: None,
            current_rect: rect(10, 10, 110, 50),
            projected_target_rect: rect(10, 10, 110, 50),
            calibration_id: Some("cal-1".to_string()),
            confidence: Some(1.0),
        },
        environment: FitEnvironment::default(),
        properties: vec!["width".to_string(), "height".to_string()],
        budget: FitBudget::default(),
        thresholds: FitThresholds::default(),
        auto_start,
    }
}

fn rect(left: i32, top: i32, right: i32, bottom: i32) -> FitRect {
    FitRect {
        left,
        top,
        right,
        bottom,
    }
}

fn cleanup(path: std::path::PathBuf) {
    let _ = fs::remove_dir_all(path);
}
