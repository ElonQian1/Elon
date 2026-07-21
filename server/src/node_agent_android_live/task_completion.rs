use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};
use serde_json::{json, Value};

use super::broker::LiveUiBroker;
use super::fit_run::{
    workspace_fingerprint, FitRunDocument, FitRunPhase, FitRunService, FitSessionContext,
};
use super::ui_ir::{load_or_build_ui_ir, TargetDesignRef};

pub(crate) async fn verify(
    broker: &LiveUiBroker,
    fit_runs: &FitRunService,
    session_id: &str,
    arguments: &Value,
) -> Result<Value> {
    let session = broker.session(session_id).await?;
    let task = super::design_bootstrap::design_task(&session, arguments)?;
    let task_id = task_id(&task)?;
    let capability_arguments = json!({"taskId": task_id});
    let capabilities =
        super::capability_gap::check_capabilities(&session, &capability_arguments).await?;
    let delegated_gap = super::capability_gap::delegated_gap(&session, &task_id)?;
    let source_revision = session
        .project_root
        .as_deref()
        .map(workspace_fingerprint)
        .transpose()?
        .flatten();
    let platform_evolution = delegated_gap
        .as_ref()
        .map_or(Value::Null, |gap| gap.platform_view());
    let nonblocking_delegation = delegated_gap.as_ref().is_some_and(|gap| {
        gap.is_nonblocking()
            && gap.covers_capability_result(&capabilities)
            && gap.evidence_matches_source(source_revision.as_deref())
    });
    if delegated_gap.is_some() && !nonblocking_delegation {
        let gap = delegated_gap.as_ref().expect("checked above");
        return Ok(completion_result(
            &task_id,
            "BLOCKED",
            false,
            false,
            vec![json!({
                "gate": "CAPABILITIES",
                "status": "DELIVERY_BLOCKED_BY_PLATFORM_EVOLUTION",
                "gapId": gap.gap_id(),
                "capabilityStatus": capabilities["status"],
                "missing": capabilities["missing"],
                "preparationRequired": capabilities["preparationRequired"],
            })],
            vec![json!("CREATE_CODEX_WORKTREE_EVOLUTION_THREAD")],
            capabilities,
            Value::Null,
            platform_evolution,
        ));
    }
    if capabilities["status"] != "READY" && !nonblocking_delegation {
        return Ok(completion_result(
            &task_id,
            "BLOCKED",
            false,
            false,
            vec![json!({
                "gate": "CAPABILITIES",
                "status": capabilities["status"],
                "missing": capabilities["missing"],
                "preparationRequired": capabilities["preparationRequired"],
            })],
            vec![capabilities["next"].clone()],
            capabilities,
            Value::Null,
            Value::Null,
        ));
    }

    let effective = capabilities["effectiveCapabilities"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let target_required = contains_capability(&effective, "TARGET_DESIGN_BINDING");
    let cross_platform_required = contains_capability(&effective, "CROSS_PLATFORM_STYLE_WRITEBACK");
    let ir = load_or_build_ui_ir(broker, session_id).await?;
    let mut gates = Vec::new();
    let mut next = Vec::new();
    let mut visual_acceptance = Value::Null;

    gates.push(json!({
        "gate":"CAPABILITIES",
        "status": if nonblocking_delegation {
            "DEFERRED_FOR_BUSINESS_DELIVERY"
        } else {
            "PASSED"
        }
    }));
    if target_required {
        let Some(target) = ir.target_design.as_ref() else {
            gates.push(json!({"gate":"TARGET_DESIGN", "status":"MISSING"}));
            next.push(json!("ui_bind_target_design"));
            return Ok(completion_result(
                &task_id,
                "BLOCKED",
                false,
                false,
                gates,
                next,
                capabilities,
                visual_acceptance,
                platform_evolution,
            ));
        };
        gates.push(json!({"gate":"TARGET_DESIGN", "status":"PASSED", "targetId":target.id}));
        let context = fit_session_context(&session).await?;
        let runs = fit_runs.list_runs(&context)?;
        if let Some(run) =
            delegated_fit_run_for_target(&runs, &task_id, target, delegated_gap.as_ref())
        {
            gates.push(json!({
                "gate":"FIT_RUN",
                "status":"PASSED_FOR_BUSINESS_DELIVERY",
                "runId":run.run_id,
                "phase":run.phase,
                "sourceVerified":run.source_verified(),
            }));
            visual_acceptance = fit_run_acceptance(run);
        } else if delegated_gap.is_some() {
            gates.push(json!({
                "gate":"FIT_RUN",
                "status":"MISSING_DELEGATED_BUSINESS_RUN",
                "requiredRunId":delegated_gap.as_ref().and_then(|gap| gap.fit_run_id()),
            }));
            next.push(json!("RESTORE_DELEGATED_FIT_RUN_PROOF"));
        } else if let Some(run) = accepted_fit_run(&runs, &task_id, target) {
            gates.push(json!({
                "gate":"FIT_RUN",
                "status":"PASSED",
                "runId":run.run_id,
                "phase":run.phase,
                "sourceVerified":run.source_verified(),
            }));
            visual_acceptance = fit_run_acceptance(run);
        } else {
            gates.push(json!({
                "gate":"FIT_RUN",
                "status":"MISSING_ACCEPTED_RUN",
                "observedRuns": runs
                    .iter()
                    .filter(|run| fit_run_matches(run, &task_id, target))
                    .map(|run| json!({"runId":run.run_id, "phase":run.phase}))
                    .collect::<Vec<_>>(),
            }));
            next.push(json!("ui_start_fit_run"));
        }
    } else if let Some(gap) = delegated_gap.as_ref() {
        let context = fit_session_context(&session).await?;
        let runs = fit_runs.list_runs(&context)?;
        if let Some(run) = delegated_fit_run(&runs, &task_id, Some(gap)) {
            gates.push(json!({
                "gate":"FIT_RUN",
                "status":"PASSED_FOR_BUSINESS_DELIVERY",
                "runId":run.run_id,
                "phase":run.phase,
                "sourceVerified":run.source_verified(),
            }));
            visual_acceptance = fit_run_acceptance(run);
        } else {
            gates.push(json!({
                "gate":"FIT_RUN",
                "status":"MISSING_DELEGATED_BUSINESS_RUN",
                "requiredRunId":gap.fit_run_id(),
            }));
            next.push(json!("RESTORE_DELEGATED_FIT_RUN_PROOF"));
        }
    } else {
        gates.push(json!({
            "gate":"FIT_RUN",
            "status":"NOT_REQUIRED_WITHOUT_CLEAN_TARGET",
        }));
    }

    if cross_platform_required {
        let task_directory = task["taskDirectory"]
            .as_str()
            .ok_or_else(|| anyhow!("设计任务缺少 taskDirectory"))?;
        let evidence_path = Path::new(task_directory).join("cross-platform-verification.json");
        match cross_platform_evidence(&evidence_path, &task_id, source_revision.as_deref()) {
            Ok(evidence) => gates.push(json!({
                "gate":"CROSS_PLATFORM_VISUAL_PARITY",
                "status":"PASSED",
                "evidence":evidence,
            })),
            Err(error) => {
                gates.push(json!({
                    "gate":"CROSS_PLATFORM_VISUAL_PARITY",
                    "status":"MISSING_OR_FAILED",
                    "evidencePath":evidence_path,
                    "reason":format!("{error:#}"),
                }));
                next.push(json!("ui_write_cross_platform_verification"));
            }
        }
    } else if super::capability_requirements::task_is_launcher_only(Some(&task)) {
        gates.push(json!({
            "gate":"CROSS_PLATFORM_VISUAL_PARITY",
            "status":"NOT_APPLICABLE",
            "reason":"LAUNCHER_ONLY_NO_WEB_SURFACE",
        }));
    } else {
        gates.push(json!({
            "gate":"CROSS_PLATFORM_VISUAL_PARITY",
            "status":"NOT_REQUIRED",
        }));
    }

    let completion_ready = gates.iter().all(strict_gate_passed);
    let business_delivery_ready = gates.iter().all(business_gate_passed);
    Ok(completion_result(
        &task_id,
        if completion_ready {
            "VERIFIED"
        } else if business_delivery_ready && delegated_gap.is_some() {
            "BUSINESS_VERIFIED_EVOLUTION_DEFERRED"
        } else {
            "BLOCKED"
        },
        completion_ready,
        business_delivery_ready,
        gates,
        if business_delivery_ready && delegated_gap.is_some() {
            let mut next = next;
            next.push(json!("CREATE_CODEX_WORKTREE_EVOLUTION_THREAD"));
            next
        } else {
            next
        },
        capabilities,
        visual_acceptance,
        platform_evolution,
    ))
}

fn completion_result(
    task_id: &str,
    status: &str,
    completion_ready: bool,
    business_delivery_ready: bool,
    gates: Vec<Value>,
    next: Vec<Value>,
    capabilities: Value,
    visual_acceptance: Value,
    platform_evolution: Value,
) -> Value {
    json!({
        "taskId": task_id,
        "status": status,
        "completionReady": completion_ready,
        "businessDeliveryReady": business_delivery_ready,
        "platformEvolutionPending": !platform_evolution.is_null(),
        "platformEvolution": platform_evolution,
        "gates": gates,
        "visualAcceptance": visual_acceptance,
        "capabilities": capabilities,
        "next": next,
        "reportContract": {
            "requiredUiFields": [
                "FIT_RUN_STATUS",
                "FINAL_VISUAL_LOSS",
                "VISUAL_ACCEPTANCE_THRESHOLD",
                "CROSS_PLATFORM_VISUAL_PARITY",
                "BUSINESS_DELIVERY_READY",
                "PLATFORM_EVOLUTION_PENDING",
                "EVOLUTION_THREAD"
            ],
            "requiredWorkflowFields": [
                "BUSINESS_STATUS",
                "LOCAL_MAIN_STATUS",
                "TASK_WORKTREE_STATUS",
                "MAIN_UNTRACKED_STATUS",
                "FINALIZABLE"
            ],
            "completionClaimAllowed": completion_ready,
            "businessDeliveryClaimAllowed": business_delivery_ready,
        }
    })
}

fn strict_gate_passed(gate: &Value) -> bool {
    matches!(
        gate.get("status").and_then(Value::as_str),
        Some("PASSED" | "NOT_APPLICABLE" | "NOT_REQUIRED" | "NOT_REQUIRED_WITHOUT_CLEAN_TARGET")
    )
}

fn business_gate_passed(gate: &Value) -> bool {
    strict_gate_passed(gate)
        || matches!(
            gate.get("status").and_then(Value::as_str),
            Some("DEFERRED_FOR_BUSINESS_DELIVERY" | "PASSED_FOR_BUSINESS_DELIVERY")
        )
}

fn task_id(task: &Value) -> Result<String> {
    task.pointer("/task/task/taskId")
        .or_else(|| task.pointer("/task/task/task_id"))
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .ok_or_else(|| anyhow!("设计任务缺少 taskId"))
}

fn contains_capability(capabilities: &[Value], name: &str) -> bool {
    capabilities
        .iter()
        .any(|value| value.as_str() == Some(name))
}

async fn fit_session_context(session: &super::broker::LiveUiSession) -> Result<FitSessionContext> {
    let view = session.view().await;
    let project_root = session
        .project_root
        .clone()
        .ok_or_else(|| anyhow!("FitRun 需要本机项目目录"))?;
    Ok(FitSessionContext {
        session_id: session.id.clone(),
        source_revision: workspace_fingerprint(&project_root)?,
        project_root,
        package_name: session.package_name.clone(),
        device_id: session.device_id.clone(),
        runtime_build_id: view.runtime_build_id,
        tree_revision: view.tree_revision,
    })
}

fn accepted_fit_run<'a>(
    runs: &'a [FitRunDocument],
    task_id: &str,
    target: &TargetDesignRef,
) -> Option<&'a FitRunDocument> {
    runs.iter().find(|run| {
        fit_run_matches(run, task_id, target)
            && run.phase == FitRunPhase::Accepted
            && run.source_verified()
    })
}

fn delegated_fit_run_for_target<'a>(
    runs: &'a [FitRunDocument],
    task_id: &str,
    target: &TargetDesignRef,
    delegated_gap: Option<&super::capability_gap::DelegatedCapabilityGap>,
) -> Option<&'a FitRunDocument> {
    delegated_fit_run(runs, task_id, delegated_gap)
        .filter(|run| fit_run_matches(run, task_id, target))
}

fn delegated_fit_run<'a>(
    runs: &'a [FitRunDocument],
    task_id: &str,
    delegated_gap: Option<&super::capability_gap::DelegatedCapabilityGap>,
) -> Option<&'a FitRunDocument> {
    let gap = delegated_gap.filter(|gap| gap.is_nonblocking())?;
    let run_id = gap.fit_run_id()?;
    runs.iter()
        .find(|run| run.run_id == run_id && gap.accepts_fit_run(run, task_id))
}

fn fit_run_matches(run: &FitRunDocument, task_id: &str, target: &TargetDesignRef) -> bool {
    run.task_id.as_deref() == Some(task_id)
        || (run.pair.target_design_id == target.id && run.pair.target_sha256 == target.sha256)
}

fn fit_run_acceptance(run: &FitRunDocument) -> Value {
    let candidate = run.current.as_ref().or(run.best.as_ref());
    json!({
        "runId": run.run_id,
        "phase": run.phase,
        "overallLoss": candidate.map(|value| value.score.overall_loss),
        "maxOverallLoss": run.thresholds.max_overall_loss,
        "geometryError": candidate.map(|value| value.score.geometry_error),
        "maxGeometryError": run.thresholds.max_geometry_error,
        "colorError": candidate.map(|value| value.score.color_error),
        "maxColorError": run.thresholds.max_color_error,
        "edgeError": candidate.map(|value| value.score.edge_error),
        "maxEdgeError": run.thresholds.max_edge_error,
        "sourceParityVerified": candidate.is_some_and(|value| value.source_parity_verified),
        "sourceParityLoss": candidate.and_then(|value| value.source_parity_loss),
        "maxSourceParityLoss": run.thresholds.max_source_parity_loss,
    })
}

pub(crate) fn cross_platform_evidence(
    path: &Path,
    task_id: &str,
    expected_source_revision: Option<&str>,
) -> Result<Value> {
    let evidence: Value = serde_json::from_slice(&fs::read(path)?)?;
    let schema_version = evidence["schemaVersion"].as_u64();
    let evidence_task_id = evidence["taskId"].as_str();
    let visual_loss = evidence["visualLoss"].as_f64();
    let max_visual_loss = evidence["maxVisualLoss"].as_f64();
    let artifacts_present = ["androidArtifact", "webArtifact"]
        .iter()
        .all(|field| artifact_file_exists(path, evidence[*field].as_str()));
    let source_revision_matches = expected_source_revision.is_none_or(|expected| {
        evidence["sourceRevision"]
            .as_str()
            .is_some_and(|actual| actual == expected)
    });
    if schema_version != Some(1)
        || evidence_task_id != Some(task_id)
        || !artifacts_present
        || !source_revision_matches
        || visual_loss.is_none()
        || max_visual_loss.is_none()
        || visual_loss
            .zip(max_visual_loss)
            .is_none_or(|(loss, limit)| {
                !loss.is_finite() || !limit.is_finite() || limit < 0.0 || loss > limit
            })
        || evidence["sourceWritebackVerified"].as_bool() != Some(true)
        || evidence["patchFreeBuildVerified"].as_bool() != Some(true)
    {
        return Err(anyhow!(
            "跨端验收工件必须包含同一 taskId、当前 sourceRevision、真实存在的 Android/Web 截图、通过阈值的 visualLoss 以及源码写回/无补丁构建证据"
        ));
    }
    Ok(evidence)
}

fn artifact_file_exists(evidence_path: &Path, value: Option<&str>) -> bool {
    let Some(value) = value.filter(|value| !value.trim().is_empty()) else {
        return false;
    };
    let Some(parent) = evidence_path.parent() else {
        return false;
    };
    let Ok(parent) = parent.canonicalize() else {
        return false;
    };
    let artifact = PathBuf::from(value);
    let resolved = if artifact.is_absolute() {
        artifact
    } else {
        parent.join(artifact)
    };
    resolved
        .canonicalize()
        .is_ok_and(|resolved| resolved.starts_with(&parent) && resolved.is_file())
}

#[cfg(test)]
mod tests {
    use super::{business_gate_passed, cross_platform_evidence, strict_gate_passed};
    use serde_json::json;
    use std::fs;

    #[test]
    fn cross_platform_evidence_requires_visual_threshold_and_both_artifacts() {
        let root = std::env::temp_dir().join(format!(
            "elon_cross_platform_evidence_{}",
            uuid::Uuid::new_v4().simple()
        ));
        fs::create_dir_all(&root).unwrap();
        let path = root.join("cross-platform-verification.json");
        fs::write(root.join("android.png"), "android").unwrap();
        fs::write(root.join("web.png"), "web").unwrap();
        fs::write(
            &path,
            serde_json::to_vec(&json!({
                "schemaVersion":1,
                "taskId":"task-1",
                "androidArtifact":"android.png",
                "webArtifact":"web.png",
                "sourceRevision":"source-1",
                "visualLoss":0.02,
                "maxVisualLoss":0.035,
                "sourceWritebackVerified":true,
                "patchFreeBuildVerified":true
            }))
            .unwrap(),
        )
        .unwrap();

        assert!(cross_platform_evidence(&path, "task-1", Some("source-1")).is_ok());
        assert!(cross_platform_evidence(&path, "task-2", Some("source-1")).is_err());
        assert!(cross_platform_evidence(&path, "task-1", Some("source-2")).is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn deferred_platform_evolution_allows_business_but_not_platform_completion() {
        for status in [
            "DEFERRED_FOR_BUSINESS_DELIVERY",
            "PASSED_FOR_BUSINESS_DELIVERY",
        ] {
            let gate = json!({"status":status});
            assert!(!strict_gate_passed(&gate));
            assert!(business_gate_passed(&gate));
        }
        assert!(!business_gate_passed(
            &json!({"status":"MISSING_ACCEPTED_RUN"})
        ));
    }

    #[test]
    fn not_applicable_gate_is_complete() {
        let gate = json!({"status":"NOT_APPLICABLE"});
        assert!(strict_gate_passed(&gate));
        assert!(business_gate_passed(&gate));
    }
}
