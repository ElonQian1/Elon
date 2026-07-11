use std::fs::{self, File};
use std::io::Read;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use crate::node_agent_android_inspector::adb_capture::capture_screen_png;
use anyhow::{anyhow, bail, Context, Result};
use sha2::{Digest, Sha256};

use super::candidate::{from_build_value, from_diff, new_trial_id};
use super::model::{FitRect, FitRunDocument};
use super::orchestrator::{
    FitBackendResult, FitRunBackend, FitRunBackendFuture, FitSourceVerifyResult,
};
use super::service::FitRunService;
use super::store::FitRunStore;
use super::workspace_revision::workspace_fingerprint;
use crate::node_agent_android_live::broker::LiveUiBroker;
use crate::node_agent_android_live::build_verify::{build_and_verify, BuildVerifyRequest};
use crate::node_agent_android_live::fit_learning::top_k_for_run;
use crate::node_agent_android_live::protocol::{
    LivePatchOperation, LivePatchTarget, LivePropertyValue, LiveStylePatch, LiveUiNode,
};
use crate::node_agent_android_live::source_commit::{
    build_source_commit_plan_for_patches, commit_source_plan, SourceCommitRequest,
};
use crate::node_agent_android_live::ui_ir::load_or_build_ui_ir;
use crate::node_agent_android_live::visual_diff::{compare_target_with_png_projected, PixelRect};
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
        let diff = compare_target_with_png_projected(
            &target.path,
            &png,
            Some(pixel_rect(run.pair.target_rect)),
            Some(pixel_rect(run.pair.current_rect)),
            Some(pixel_rect(run.pair.projected_target_rect)),
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
            .map(|matched| matched.prior.median_deltas.clone())
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
            .map(|operation| candidate_operation_value(operation, &baseline_node))
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
            build_verify_request(&run),
            host_port,
        )
        .await?;
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
            build_verify_request(&run),
            host_port,
        )
        .await?;
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
        Self::new(FitRunStore::new(), Arc::new(LiveFitRunBackend::new(broker)))
    }
}

fn persist_frame(run: &FitRunDocument, trial_id: &str, png: &[u8]) -> Result<String> {
    let root = PathBuf::from(&run.project_root)
        .canonicalize()
        .context("FitRun 项目目录不存在")?;
    let dir = root
        .join(".elon")
        .join("ui-tuner")
        .join("fit-runs")
        .join(&run.run_id)
        .join("frames");
    fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{trial_id}.png"));
    fs::write(&path, png)?;
    Ok(path.display().to_string())
}

fn pixel_rect(value: FitRect) -> PixelRect {
    PixelRect {
        left: value.left,
        top: value.top,
        right: value.right,
        bottom: value.bottom,
    }
}

fn build_verify_request(run: &FitRunDocument) -> BuildVerifyRequest {
    BuildVerifyRequest {
        preview: None,
        debug_application_id_suffix: None,
        target_rect: Some(pixel_rect(run.pair.target_rect)),
        current_rect: Some(pixel_rect(run.pair.current_rect)),
        projected_current_rect: Some(pixel_rect(run.pair.projected_target_rect)),
        target_definition_id: Some(run.pair.definition_id.clone()),
        target_instance_key: run.pair.instance_key.clone(),
    }
}

fn elapsed_ms(started: Instant) -> u64 {
    started.elapsed().as_millis().min(u64::MAX as u128) as u64
}

fn resolve_runtime_node<'a>(
    run: &FitRunDocument,
    nodes: &'a [crate::node_agent_android_live::protocol::LiveUiNode],
) -> Result<&'a crate::node_agent_android_live::protocol::LiveUiNode> {
    if let Some(node) = nodes
        .iter()
        .find(|node| node.runtime_node_id == run.pair.runtime_node_id)
    {
        if node.definition_id == run.pair.definition_id
            && node.instance_key == run.pair.instance_key
        {
            return Ok(node);
        }
        bail!("runtimeNodeId 已指向不同稳定节点，必须重新绑定");
    }
    let matches = nodes
        .iter()
        .filter(|node| {
            node.definition_id == run.pair.definition_id
                && run
                    .pair
                    .instance_key
                    .as_ref()
                    .is_none_or(|key| node.instance_key.as_ref() == Some(key))
        })
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [node] => Ok(*node),
        [] => bail!("FitRun 目标节点在当前 Runtime 树中不存在"),
        _ => bail!("稳定 Node ID 对应多个运行实例；必须提供 instanceKey 后重新绑定"),
    }
}

fn inverse_operations(values: &[serde_json::Value]) -> Result<Vec<LivePatchOperation>> {
    values
        .iter()
        .map(|operation| {
            let property = operation
                .get("property")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| anyhow!("FitRun candidate operation 缺少 property"))?;
            let before = operation
                .get("beforeValue")
                .cloned()
                .ok_or_else(|| anyhow!("取消 FitRun 需要 operation.beforeValue: {property}"))?;
            Ok(LivePatchOperation {
                property: property.to_string(),
                value: serde_json::from_value(before)?,
            })
        })
        .collect()
}

fn candidate_operation_value(
    operation: &LivePatchOperation,
    node: &LiveUiNode,
) -> serde_json::Result<serde_json::Value> {
    let mut value = serde_json::to_value(operation)?;
    if let Some(before) = baseline_property_value(node, &operation.property) {
        if let Some(object) = value.as_object_mut() {
            object.insert("beforeValue".to_string(), serde_json::to_value(before)?);
        }
    }
    Ok(value)
}

fn baseline_property_value(node: &LiveUiNode, property: &str) -> Option<LivePropertyValue> {
    if let Some(value) = node.properties.get(property).and_then(|snapshot| {
        snapshot
            .effective
            .clone()
            .or_else(|| snapshot.measured.clone())
    }) {
        return Some(value);
    }
    let density = node.geometry.density.max(0.01) as f64;
    let (value_type, value) = match property {
        "width" => (
            "dp",
            node.geometry.bounds_in_display_px.width as f64 / density,
        ),
        "height" => (
            "dp",
            node.geometry.bounds_in_display_px.height as f64 / density,
        ),
        "translationX" | "translationY" => ("dp", 0.0),
        "opacity" => ("float", 1.0),
        _ => return None,
    };
    Some(LivePropertyValue {
        value_type: value_type.to_string(),
        value: serde_json::json!((value * 1000.0).round() / 1000.0),
    })
}

fn sha256_file(path: &str) -> Result<String> {
    let mut file = File::open(path).with_context(|| format!("目标设计图不存在: {path}"))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex::encode(hasher.finalize()))
}
