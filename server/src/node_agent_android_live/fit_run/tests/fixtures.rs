use std::collections::VecDeque;
use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use anyhow::{anyhow, Result};

use super::super::model::{
    CreateFitRunRequest, FitBudget, FitCandidate, FitEnvironment, FitRect, FitRunDocument,
    FitScore, FitSessionContext, FitTargetPair, FitThresholds,
};
use super::super::orchestrator::{
    FitBackendResult, FitRunBackend, FitRunBackendFuture, FitSourceVerifyResult,
};

pub(super) struct FakeBackend {
    baseline: Mutex<VecDeque<FitBackendResult>>,
    local: Mutex<VecDeque<FitBackendResult>>,
    verify: Mutex<Option<FitSourceVerifyResult>>,
    pub(super) revert_calls: AtomicUsize,
}

impl FakeBackend {
    pub(super) fn new(baseline: Vec<FitBackendResult>, local: Vec<FitBackendResult>) -> Self {
        Self {
            baseline: Mutex::new(baseline.into()),
            local: Mutex::new(local.into()),
            verify: Mutex::new(None),
            revert_calls: AtomicUsize::new(0),
        }
    }

    pub(super) fn with_verify(self, result: FitSourceVerifyResult) -> Self {
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
        self.revert_calls.fetch_add(1, Ordering::SeqCst);
        Box::pin(async { Ok(()) })
    }
}

pub(super) fn result(id: &str, loss: f64) -> FitBackendResult {
    FitBackendResult {
        candidate: candidate(id, loss),
        evaluations: 1,
        duration_ms: 1,
    }
}

pub(super) fn candidate(id: &str, loss: f64) -> FitCandidate {
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

pub(super) fn run(auto_start: bool) -> (std::path::PathBuf, FitRunDocument) {
    let root = std::env::temp_dir().join(format!("fit-run-test-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&root).unwrap();
    let context = context(root.to_str().unwrap());
    let run = FitRunDocument::new(context, request(auto_start)).unwrap();
    (root, run)
}

pub(super) fn context(project_root: &str) -> FitSessionContext {
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

pub(super) fn request(auto_start: bool) -> CreateFitRunRequest {
    CreateFitRunRequest {
        task_id: Some("task-test".to_string()),
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
        visual_mask: Default::default(),
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

pub(super) fn cleanup(path: std::path::PathBuf) {
    let _ = fs::remove_dir_all(path);
}
