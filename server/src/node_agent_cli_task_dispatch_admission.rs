use std::{path::PathBuf, time::Duration};

use elon_pc_dev_runtime::NodeDataPaths;
use tokio::sync::{watch, OwnedMutexGuard};
use tracing::{warn, Instrument};

use crate::{
    node_agent_active_task_registry::CliPromptRegistration,
    node_agent_build_runtime::{BuildRunRequest, RegisteredCliBuildRun},
    node_agent_cli_runner::PreparedCliPromptCwd,
    node_agent_supervision_worktree_lease::ResumeAdmissionGuard,
    NodeRuntime,
};

const BUILD_ADMISSION_TIMEOUT: Duration = Duration::from_secs(45);
const SUPERVISED_REGISTRATION_TIMEOUT: Duration = Duration::from_secs(8);

pub(super) enum AdmissionOutcome {
    Ready {
        deadline_cancel_tx: watch::Sender<bool>,
        build_run_guard: Option<RegisteredCliBuildRun>,
    },
    Duplicate,
    Failed {
        message: String,
        active_handle_registered: bool,
    },
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn register_active_and_prepare_build(
    runtime: &NodeRuntime,
    req_id: &str,
    cli_name: &str,
    prepared_cwd: &PreparedCliPromptCwd,
    runtime_permission: Option<String>,
    cancel_tx: watch::Sender<bool>,
    effective_requires_cloud_control: bool,
    supervised_registration: bool,
    true_workspace_resume: bool,
    resume_admission: Option<ResumeAdmissionGuard>,
    transition: OwnedMutexGuard<()>,
    data_paths: Option<NodeDataPaths>,
    supervision_protocol: Option<&str>,
) -> AdmissionOutcome {
    let registration_stage = if true_workspace_resume {
        "resume_admission"
    } else {
        "supervised_registration"
    };
    let _ = runtime
        .task_journal
        .record_dispatch_stage(req_id, registration_stage);
    let journal = runtime.task_journal.clone();
    let lease_task_id = req_id.to_string();
    let lease_protocol = supervision_protocol.map(str::to_string);
    let lease_workspace = prepared_cwd.conversation_workspace.clone();
    let lease = tokio::time::timeout(
        SUPERVISED_REGISTRATION_TIMEOUT,
        tokio::task::spawn_blocking(move || {
            crate::node_agent_cli_supervision_lease::acquire_for_task(
                &journal,
                &lease_task_id,
                lease_protocol.as_deref(),
                lease_workspace.as_ref(),
            )
        }),
    )
    .await;
    match lease {
        Ok(Ok(Ok(()))) => {}
        Ok(Ok(Err(error))) => {
            return AdmissionOutcome::Failed {
                message: format!(
                    "SUPERVISED_REGISTRATION_FAILED: failed to persist supervision worktree lease: {error}"
                ),
                active_handle_registered: false,
            }
        }
        Ok(Err(error)) => {
            return AdmissionOutcome::Failed {
                message: format!(
                    "SUPERVISED_REGISTRATION_JOIN_FAILED: supervision lease task ended unexpectedly: {error}"
                ),
                active_handle_registered: false,
            }
        }
        Err(_) => {
            let message =
                "SUPERVISED_REGISTRATION_TIMEOUT: supervision worktree lease exceeded 8 seconds";
            let _ = runtime.task_journal.record_dispatch_failure(
                req_id,
                registration_stage,
                "supervised_registration_timeout",
                message,
            );
            return AdmissionOutcome::Failed {
                message: message.to_string(),
                active_handle_registered: false,
            };
        }
    }
    let deadline_cancel_tx = cancel_tx.clone();
    let registration = crate::node_agent_cli_task_registration::register(
        runtime,
        req_id,
        cli_name,
        prepared_cwd.cwd.clone(),
        runtime_permission,
        cancel_tx,
        effective_requires_cloud_control,
        supervised_registration,
        true_workspace_resume,
        resume_admission.as_ref(),
    )
    .await
    .unwrap_or_else(|error| {
        warn!(%error, %req_id, "supervised CLI owner admission failed closed");
        CliPromptRegistration::WorkspaceBusy
    });
    match registration {
        CliPromptRegistration::Inserted => {}
        CliPromptRegistration::DuplicateReq => return AdmissionOutcome::Duplicate,
        CliPromptRegistration::WorkspaceBusy => {
            return AdmissionOutcome::Failed {
                message: "父任务隔离 worktree 已被其他活跃任务占用，已拒绝续跑。".to_string(),
                active_handle_registered: false,
            }
        }
    }
    drop(resume_admission);
    let _ = runtime
        .task_journal
        .record_dispatch_stage(req_id, "active_handle");

    let build_request = prepared_cwd
        .project_context
        .as_ref()
        .filter(|context| !crate::cli_prompt_read_only(context.runtime_permission.as_deref()))
        .filter(|_| prepared_cwd.data_policy.uses_managed_workspace())
        .and_then(|context| {
            data_paths.clone().map(|paths| {
                (
                    paths,
                    req_id.to_string(),
                    context.project_id.clone(),
                    prepared_cwd.cwd.as_deref().map(PathBuf::from),
                )
            })
        });
    let build_run_guard = if let Some((paths, task_id, project_id, build_cwd)) = build_request {
        let _ = runtime
            .task_journal
            .record_dispatch_stage(req_id, "build_admission_cache_telemetry");
        let span = tracing::info_span!("node_build_admission", %task_id, %project_id);
        let prepared = tokio::time::timeout(
            BUILD_ADMISSION_TIMEOUT,
            tokio::task::spawn_blocking(move || {
                let _transition = transition;
                crate::node_agent_build_runtime::register_cli_run(
                    &paths,
                    BuildRunRequest {
                        task_id: &task_id,
                        project_id: &project_id,
                        cwd: build_cwd.as_deref(),
                    },
                )
            })
            .instrument(span),
        )
        .await;
        match prepared {
            Ok(Ok(Ok(run))) => Some(run),
            Ok(Ok(Err(error))) => {
                return AdmissionOutcome::Failed {
                    message: format!("BUILD_ADMISSION_FAILED: 一龙推荐构建环境准备失败: {error:#}"),
                    active_handle_registered: true,
                }
            }
            Ok(Err(error)) => {
                return AdmissionOutcome::Failed {
                    message: format!("BUILD_ADMISSION_JOIN_FAILED: 构建准入任务异常结束: {error}"),
                    active_handle_registered: true,
                }
            }
            Err(_) => {
                let message = "BUILD_ADMISSION_TIMEOUT: 构建准入与缓存遥测超过 45 秒；任务已明确失败，后台扫描不会阻塞 Tokio worker";
                let _ = runtime.task_journal.record_dispatch_failure(
                    req_id,
                    "build_admission_cache_telemetry",
                    "build_admission_timeout",
                    message,
                );
                return AdmissionOutcome::Failed {
                    message: message.to_string(),
                    active_handle_registered: true,
                };
            }
        }
    } else {
        drop(transition);
        if prepared_cwd.data_policy.uses_managed_workspace() && data_paths.is_none() {
            warn!("推荐数据根暂不可用，继续继承原项目构建环境");
        }
        None
    };
    AdmissionOutcome::Ready {
        deadline_cancel_tx,
        build_run_guard,
    }
}
