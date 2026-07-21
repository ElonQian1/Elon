use std::fs;
use std::io::Write;

use serde_json::json;

use super::super::model::{FitRunPhase, FitTrial, FitTrialKind};
use super::super::store::FitRunStore;
use super::fixtures::{candidate, cleanup, run};
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
    assert_eq!(
        store
            .list_for_project(root.to_str().unwrap())
            .unwrap()
            .len(),
        1
    );
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
fn loading_legacy_run_reconciles_generic_failures_with_declared_thresholds() {
    let (root, mut run) = run(false);
    let store = FitRunStore::new();
    let mut accepted = candidate("accepted", 0.02);
    accepted.score.color_error = 0.031464;
    accepted.score.hard_failures = vec!["color".to_string()];
    run.baseline = Some(accepted.clone());
    run.current = Some(accepted.clone());
    run.best = Some(accepted);
    store.save(&run).unwrap();

    let loaded = store.load(root.to_str().unwrap(), &run.run_id).unwrap();
    assert!(loaded
        .baseline
        .as_ref()
        .unwrap()
        .score
        .hard_failures
        .is_empty());
    assert!(loaded
        .current
        .as_ref()
        .unwrap()
        .score
        .hard_failures
        .is_empty());
    assert!(loaded.best.as_ref().unwrap().score.hard_failures.is_empty());
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
