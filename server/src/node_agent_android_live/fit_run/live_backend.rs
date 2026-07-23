use std::sync::Arc;
use std::time::Instant;

use crate::node_agent_android_inspector::adb_capture::capture_screen_png;
use anyhow::{anyhow, bail, Result};

use super::candidate::{from_build_value, from_diff, new_trial_id};
use super::live_artifacts::{build_verify_request, elapsed_ms, persist_frame, pixel_rect};
use super::live_values::{
    candidate_operation_value, inverse_operations, numeric_baseline_property_value,
    resolve_runtime_node, sha256_file,
};
use super::model::FitRunDocument;
use super::orchestrator::{
    FitBackendResult, FitRunBackend, FitRunBackendFuture, FitSourceVerifyResult,
};
use super::service::FitRunService;
use super::store::FitRunStore;
use super::workspace_revision::workspace_fingerprint;
use crate::node_agent_android_live::broker::LiveUiBroker;
use crate::node_agent_android_live::build_verify::build_and_verify;
use crate::node_agent_android_live::fit_learning::top_k_for_run;
use crate::node_agent_android_live::protocol::{
    LivePatchOperation, LivePatchTarget, LiveSessionView, LiveSourceProofView, LiveStylePatch,
    LiveUiNode,
};
use crate::node_agent_android_live::source_commit::{
    build_source_commit_plan_for_patches, commit_source_plan, SourceCommitRequest,
};
use crate::node_agent_android_live::ui_ir::load_or_build_ui_ir;
use crate::node_agent_android_live::visual_diff::compare_target_with_png_projected_masked;
use crate::node_agent_android_live::visual_solver::{solve_visual_style, VisualSolverRequest};

pub(crate) struct LiveFitRunBackend {
    broker: Arc<LiveUiBroker>,
}

impl LiveFitRunBackend {
    pub(crate) fn new(broker: Arc<LiveUiBroker>) -> Self {
        Self { broker }
    }

    async fn current_runtime_node(&self, run: &FitRunDocument) -> Result<LiveUiNode> {
        let ir = load_or_build_ui_ir(&self.broker, &run.session_id).await?;
        Ok(resolve_runtime_node(run, &ir.nodes)?.clone())
    }

    async fn current_runtime_node_id(&self, run: &FitRunDocument) -> Result<String> {
        Ok(self.current_runtime_node(run).await?.runtime_node_id)
    }

    async fn ensure_target_identity(&self, run: &FitRunDocument) -> Result<()> {
        let ir = load_or_build_ui_ir(&self.broker, &run.session_id).await?;
        let target = ir
            .target_design
            .ok_or_else(|| anyhow!("FitRun 尚未绑定目标设计图"))?;
        if target.id != run.pair.target_design_id || target.sha256 != run.pair.target_sha256 {
            bail!("目标设计图已变化，FitRun 必须重新校准");
        }
        if sha256_file(&target.path)? != run.pair.target_sha256 {
            bail!("目标设计图文件内容与绑定指纹不一致，FitRun 已停止");
        }
        Ok(())
    }

    async fn baseline(&self, run: FitRunDocument) -> Result<FitBackendResult> {
        let started = Instant::now();
        self.ensure_target_identity(&run).await?;
        let session = self.broker.session(&run.session_id).await?;
        let ir = load_or_build_ui_ir(&self.broker, &run.session_id).await?;
        let target = ir
            .target_design
            .ok_or_else(|| anyhow!("FitRun 尚未绑定目标设计图"))?;
        if target.sha256 != run.pair.target_sha256 {
            bail!("目标设计图已变化，FitRun 必须重新校准");
        }
        let png = capture_screen_png(&session.device_id).await?;
        let visual_mask = run.visual_mask.visual_mask();
        let diff = compare_target_with_png_projected_masked(
            &target.path,
            &png,
            Some(pixel_rect(run.pair.target_rect)),
            Some(pixel_rect(run.pair.current_rect)),
            Some(pixel_rect(run.pair.projected_target_rect)),
            &visual_mask,
        )?;
        let trial_id = new_trial_id("baseline");
        let screenshot_path = persist_frame(&run, &trial_id, &png)?;
        Ok(FitBackendResult {
            candidate: from_diff(&run, trial_id, diff, Vec::new(), Some(screenshot_path)),
            evaluations: 1,
            duration_ms: elapsed_ms(started),
        })
    }

    async fn local_solve(&self, run: FitRunDocument) -> Result<FitBackendResult> {
        let started = Instant::now();
        self.ensure_target_identity(&run).await?;
        let baseline_node = self.current_runtime_node(&run).await?;
        let runtime_node_id = baseline_node.runtime_node_id.clone();
        let remaining = run
            .budget
            .max_local_evaluations
            .saturating_sub(run.usage.local_evaluations)
            .clamp(1, 24) as usize;
        let learning_prior = match top_k_for_run(&run, 1) {
            Ok(matches) => matches.into_iter().next(),
            Err(error) => {
                tracing::warn!(run_id = %run.run_id, error = %error, "读取 UI 拟合先验失败，继续无先验求解");
                None
            }
        };
        let initial_property_deltas = learning_prior
            .as_ref()
            .map(|matched| prior_seed_deltas(&matched.prior, &baseline_node))
            .unwrap_or_default();
        let initial_step_dp = initial_property_deltas
            .values()
            .copied()
            .filter(|value| value.is_finite() && value.abs() >= 0.25)
            .map(f64::abs)
            .min_by(f64::total_cmp);
        // `target_rect` 是设计图 crop；接线阶段必须同时为 VisualSolverRequest
        // 增加 projectedCurrentRect，供几何 seed 使用，避免跨尺寸设计稿比例漂移。
        let result = solve_visual_style(
            &self.broker,
            &run.session_id,
            VisualSolverRequest {
                runtime_node_id,
                target_rect: pixel_rect(run.pair.target_rect),
                projected_current_rect: Some(pixel_rect(run.pair.projected_target_rect)),
                properties: run.properties.clone(),
                max_evaluations: Some(remaining),
                initial_step_dp,
                initial_property_deltas,
                visual_mask: run.visual_mask.visual_mask(),
            },
        )
        .await?;
        let session = self.broker.session(&run.session_id).await?;
        let png = capture_screen_png(&session.device_id).await?;
        let trial_id = new_trial_id("local");
        let screenshot_path = persist_frame(&run, &trial_id, &png)?;
        let operations = result
            .operations
            .iter()
            .map(|operation| {
                candidate_operation_value(
                    operation,
                    &baseline_node,
                    run.best
                        .as_ref()
                        .map(|candidate| candidate.operations.as_slice()),
                )
            })
            .collect::<serde_json::Result<Vec<_>>>()?;
        Ok(FitBackendResult {
            candidate: from_diff(
                &run,
                trial_id,
                result.final_diff,
                operations,
                Some(screenshot_path),
            ),
            evaluations: result.evaluations as u32,
            duration_ms: elapsed_ms(started),
        })
    }

    async fn build_evaluate(&self, run: FitRunDocument) -> Result<FitBackendResult> {
        let started = Instant::now();
        self.ensure_target_identity(&run).await?;
        let host_port = crate::node_agent_admin_open::admin_port_from_env();
        let result = build_and_verify(
            &self.broker,
            &run.session_id,
            build_verify_request(&run)?,
            host_port,
        )
        .await?;
        self.ensure_target_identity(&run).await?;
        let value = serde_json::to_value(&result)?;
        let mut candidate = from_build_value(&run, &value)?;
        candidate.source_revision = workspace_fingerprint(&run.project_root)?;
        Ok(FitBackendResult {
            candidate,
            evaluations: 1,
            duration_ms: elapsed_ms(started),
        })
    }

    async fn source_verify(&self, run: FitRunDocument) -> Result<FitSourceVerifyResult> {
        let started = Instant::now();
        self.ensure_target_identity(&run).await?;
        let session = self.broker.session(&run.session_id).await?;
        let source_revision = workspace_fingerprint(&run.project_root)?;
        let session_view = session.view().await;
        if let Some(candidate) =
            fresh_runtime_source_candidate(&run, &session_view, source_revision.as_deref())
        {
            return Ok(FitSourceVerifyResult {
                candidate,
                duration_ms: elapsed_ms(started),
            });
        }
        let already_source_backed = run.current.as_ref().is_some_and(|candidate| {
            candidate.source_parity_verified && candidate.score.passes(&run.thresholds)
        });
        if !already_source_backed {
            if let Some(patch) = self.best_patch(&run).await? {
                let plan =
                    build_source_commit_plan_for_patches(session.clone(), vec![patch]).await?;
                if plan.codex_count > 0 {
                    let mut candidate = run
                        .best
                        .clone()
                        .ok_or_else(|| anyhow!("FitRun 尚无最佳候选"))?;
                    candidate.trial_id = new_trial_id("source-deferred");
                    candidate.source_parity_verified = false;
                    candidate.source_parity_loss = None;
                    return Ok(FitSourceVerifyResult {
                        candidate,
                        duration_ms: elapsed_ms(started),
                    });
                }
                let revision = plan.source_revision.clone();
                commit_source_plan(
                    plan,
                    SourceCommitRequest {
                        source_revision: revision,
                    },
                )?;
            }
        }
        let host_port = crate::node_agent_admin_open::admin_port_from_env();
        let build = build_and_verify(
            &self.broker,
            &run.session_id,
            build_verify_request(&run)?,
            host_port,
        )
        .await?;
        self.ensure_target_identity(&run).await?;
        let value = serde_json::to_value(&build)?;
        let mut candidate = from_build_value(&run, &value)?;
        candidate.source_revision = workspace_fingerprint(&run.project_root)?;
        Ok(FitSourceVerifyResult {
            candidate,
            duration_ms: elapsed_ms(started),
        })
    }

    async fn apply_best(&self, run: FitRunDocument) -> Result<()> {
        self.ensure_target_identity(&run).await?;
        let runtime_node_id = self.current_runtime_node_id(&run).await?;
        let best = run.best.ok_or_else(|| anyhow!("FitRun 尚无最佳候选"))?;
        if best.operations.is_empty() {
            return Ok(());
        }
        let operations = best
            .operations
            .into_iter()
            .map(serde_json::from_value::<LivePatchOperation>)
            .collect::<serde_json::Result<Vec<_>>>()?;
        self.broker
            .apply_patch(
                &run.session_id,
                LiveStylePatch {
                    protocol_version: 1,
                    message_type: String::new(),
                    session_id: String::new(),
                    request_id: String::new(),
                    gesture_id: Some(format!("fit-run-restore:{}", run.run_id)),
                    sequence: 0,
                    base_tree_revision: None,
                    target: LivePatchTarget {
                        scope: "INSTANCE".to_string(),
                        runtime_node_id: Some(runtime_node_id),
                        definition_id: Some(run.pair.definition_id),
                        instance_key: run.pair.instance_key,
                    },
                    atomic: true,
                    ephemeral: true,
                    operations,
                },
            )
            .await?;
        Ok(())
    }

    async fn revert_best(&self, run: FitRunDocument) -> Result<()> {
        if run
            .current
            .as_ref()
            .is_some_and(|candidate| candidate.source_parity_verified)
        {
            return Ok(());
        }
        let runtime_node_id = self.current_runtime_node_id(&run).await?;
        let best = run.best.ok_or_else(|| anyhow!("FitRun 尚无最佳候选"))?;
        if best.operations.is_empty() {
            return Ok(());
        }
        let operations = inverse_operations(&best.operations)?;
        self.broker
            .apply_patch(
                &run.session_id,
                LiveStylePatch {
                    protocol_version: 1,
                    message_type: String::new(),
                    session_id: String::new(),
                    request_id: String::new(),
                    gesture_id: Some(format!("fit-run-cancel:{}", run.run_id)),
                    sequence: 0,
                    base_tree_revision: None,
                    target: LivePatchTarget {
                        scope: "INSTANCE".to_string(),
                        runtime_node_id: Some(runtime_node_id),
                        definition_id: Some(run.pair.definition_id),
                        instance_key: run.pair.instance_key,
                    },
                    atomic: true,
                    ephemeral: true,
                    operations,
                },
            )
            .await?;
        Ok(())
    }

    async fn best_patch(&self, run: &FitRunDocument) -> Result<Option<LiveStylePatch>> {
        let best = run
            .best
            .as_ref()
            .ok_or_else(|| anyhow!("FitRun 尚无最佳候选"))?;
        if best.operations.is_empty() {
            return Ok(None);
        }
        let runtime_node_id = self.current_runtime_node_id(run).await?;
        let operations = best
            .operations
            .iter()
            .cloned()
            .map(serde_json::from_value::<LivePatchOperation>)
            .collect::<serde_json::Result<Vec<_>>>()?;
        Ok(Some(LiveStylePatch {
            protocol_version: 1,
            message_type: String::new(),
            session_id: String::new(),
            request_id: String::new(),
            gesture_id: Some(format!("fit-run-commit:{}", run.run_id)),
            sequence: 0,
            base_tree_revision: None,
            target: LivePatchTarget {
                scope: "INSTANCE".to_string(),
                runtime_node_id: Some(runtime_node_id),
                definition_id: Some(run.pair.definition_id.clone()),
                instance_key: run.pair.instance_key.clone(),
            },
            atomic: true,
            ephemeral: true,
            operations,
        }))
    }
}

pub(super) fn fresh_runtime_source_candidate(
    run: &FitRunDocument,
    session: &LiveSessionView,
    origin_workspace_revision: Option<&str>,
) -> Option<super::model::FitCandidate> {
    let best = run.best.as_ref()?;
    let proof: &LiveSourceProofView = session.source_proof.as_ref()?;
    let origin_workspace_revision = origin_workspace_revision?;
    let runtime_build_matches = proof.runtime_build_id == session.runtime_build_id
        && session.runtime_build_id == run.runtime_build_id;
    let source_revision_matches = proof.origin_workspace_revision == origin_workspace_revision
        && run.source_revision.as_deref() == Some(origin_workspace_revision);
    if !session.connected
        || session.history_count != 0
        || session.redo_count != 0
        || !best.operations.is_empty()
        || !runtime_build_matches
        || !source_revision_matches
        || proof.generation_revision.trim().is_empty()
        || proof.source_parity_loss > run.thresholds.max_source_parity_loss
        || !best.score.passes(&run.thresholds)
    {
        return None;
    }
    let mut candidate = best.clone();
    candidate.trial_id = new_trial_id("fresh-runtime-source");
    candidate.runtime_build_id = session.runtime_build_id.clone();
    candidate.source_revision = Some(origin_workspace_revision.to_string());
    candidate.source_parity_loss = Some(proof.source_parity_loss);
    candidate.source_parity_verified = true;
    Some(candidate)
}

fn prior_seed_deltas(
    prior: &crate::node_agent_android_live::fit_learning::FitPrior,
    node: &LiveUiNode,
) -> std::collections::BTreeMap<String, f64> {
    let mut deltas = prior.median_deltas.clone();
    for (property, factor) in &prior.median_factors {
        let Some(current) = numeric_baseline_property_value(node, property) else {
            continue;
        };
        let scaled_delta = current * (factor - 1.0);
        if scaled_delta.is_finite() {
            deltas.insert(property.clone(), scaled_delta);
        }
    }
    deltas
}

impl FitRunBackend for LiveFitRunBackend {
    fn capture_baseline<'a>(
        &'a self,
        run: FitRunDocument,
    ) -> FitRunBackendFuture<'a, FitBackendResult> {
        Box::pin(self.baseline(run))
    }

    fn solve_local<'a>(&'a self, run: FitRunDocument) -> FitRunBackendFuture<'a, FitBackendResult> {
        Box::pin(self.local_solve(run))
    }

    fn evaluate_after_codex<'a>(
        &'a self,
        run: FitRunDocument,
    ) -> FitRunBackendFuture<'a, FitBackendResult> {
        Box::pin(self.build_evaluate(run))
    }

    fn verify_source<'a>(
        &'a self,
        run: FitRunDocument,
    ) -> FitRunBackendFuture<'a, FitSourceVerifyResult> {
        Box::pin(self.source_verify(run))
    }

    fn reapply_best<'a>(&'a self, run: FitRunDocument) -> FitRunBackendFuture<'a, ()> {
        Box::pin(self.apply_best(run))
    }

    fn revert_best<'a>(&'a self, run: FitRunDocument) -> FitRunBackendFuture<'a, ()> {
        Box::pin(self.revert_best(run))
    }
}

impl FitRunService {
    pub(crate) fn live(broker: Arc<LiveUiBroker>) -> Self {
        Self::new(
            FitRunStore::new(),
            Arc::new(LiveFitRunBackend::new(broker.clone())),
        )
        .with_live_broker(broker)
    }
}
