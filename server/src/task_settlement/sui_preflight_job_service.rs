use anyhow::{bail, Result};

use crate::{project_auth::can_edit, store::Store};

use super::{
    sui_adapter_handoff_model::SuiAdapterHandoffBundle,
    sui_adapter_handoff_service,
    sui_preflight_job_model::{
        CancelSuiPreflightJobRequest, ClaimSuiPreflightJobRequest, CompleteSuiPreflightJobRequest,
        QueueSuiPreflightJobRequest, ReleaseSuiPreflightJobRequest, RenewSuiPreflightJobRequest,
        SuiPreflightJob, SuiPreflightJobComplete, SuiPreflightJobIssue, SuiPreflightJobList,
        SuiPreflightJobPoll, SuiPreflightJobRelease, SuiPreflightJobRenew,
        SUI_PREFLIGHT_JOB_COMPLETE_SCHEMA, SUI_PREFLIGHT_JOB_ISSUE_SCHEMA,
        SUI_PREFLIGHT_JOB_LIST_SCHEMA, SUI_PREFLIGHT_JOB_POLL_SCHEMA,
        SUI_PREFLIGHT_JOB_RELEASE_SCHEMA, SUI_PREFLIGHT_JOB_RENEW_SCHEMA,
    },
    sui_preflight_model::{RecordSuiPreflightReportRequest, SuiPreflightAdapter},
    sui_preflight_service,
};

const BOUNDARY: [&str; 7] = [
    "任务只能由项目编辑者从当前可导出的链下投影包显式入队",
    "机器租约为 60 至 900 秒，单次任务最长处理时间为 1 小时",
    "租约明文只在领取时返回，服务端只保存 SHA-256",
    "交接摘要漂移、争议阻断或投影不可导出时任务会被阻断",
    "预检报告与任务完成在同一数据库事务内追加",
    "领取、续租、释放或完成任务均不签名、不广播且不移动资金",
    "任务运行入口默认关闭，必须显式启用离线预检环境开关",
];

pub(super) fn list(store: &Store, project_id: &str) -> Result<SuiPreflightJobList> {
    Ok(SuiPreflightJobList {
        schema: SUI_PREFLIGHT_JOB_LIST_SCHEMA,
        project_id: project_id.trim().to_string(),
        jobs: store.list_task_sui_preflight_jobs(project_id, 200)?,
        boundary: BOUNDARY.to_vec(),
    })
}

pub(super) fn queue(
    store: &Store,
    project_id: &str,
    actor_user_id: &str,
    actor_role: &str,
    request: &QueueSuiPreflightJobRequest,
) -> Result<SuiPreflightJob> {
    require_editor(actor_role)?;
    if !request.confirmed_by_user {
        bail!("创建 Sui 离线预检任务前必须取得用户明确确认");
    }
    let package_kind = normalize_package_kind(&request.package_kind)?;
    let projection_id = bounded_text(&request.projection_package_id, "投影包 ID", 8, 160)?;
    let handoff = verified_bundle(store, project_id, package_kind, &projection_id)?;
    store.create_task_sui_preflight_job(
        project_id,
        package_kind,
        &projection_id,
        &handoff.payload.target_network,
        &handoff.handoff_digest,
        &handoff.payload.projection_digest,
        actor_user_id,
    )
}

pub(super) fn cancel(
    store: &Store,
    project_id: &str,
    job_id: &str,
    actor_role: &str,
    request: &CancelSuiPreflightJobRequest,
) -> Result<SuiPreflightJob> {
    require_editor(actor_role)?;
    if !request.confirmed_by_user {
        bail!("取消 Sui 离线预检任务前必须取得用户明确确认");
    }
    let reason = bounded_text(&request.reason, "取消原因", 4, 500)?;
    store.cancel_task_sui_preflight_job(project_id, job_id, &reason)
}

pub(super) fn claim_next(
    store: &Store,
    adapter: &SuiPreflightAdapter,
    request: &ClaimSuiPreflightJobRequest,
) -> Result<SuiPreflightJobPoll> {
    require_runtime()?;
    let lease_seconds = bounded_lease_seconds(request.lease_seconds)?;
    for job_id in store.list_task_sui_preflight_candidate_ids(&adapter.project_id, 50)? {
        let job = store.task_sui_preflight_job(&adapter.project_id, &job_id)?;
        if !adapter_allows(adapter, &job) {
            continue;
        }
        let handoff = match read_only_bundle(
            store,
            &job.project_id,
            &job.package_kind,
            &job.projection_package_id,
        ) {
            Ok(value) => value,
            Err(error) => {
                store.block_task_sui_preflight_job(
                    &job.project_id,
                    &job.id,
                    &format!("handoff_unavailable: {error:#}"),
                )?;
                continue;
            }
        };
        if !bundle_matches_job(&handoff, &job) {
            store.block_task_sui_preflight_job(&job.project_id, &job.id, "handoff_digest_drift")?;
            continue;
        }
        if let Some((leased, lease_token)) =
            store.try_claim_task_sui_preflight_job(adapter, &job.id, lease_seconds)?
        {
            store.touch_task_sui_preflight_adapter(&adapter.id)?;
            return Ok(SuiPreflightJobPoll {
                schema: SUI_PREFLIGHT_JOB_POLL_SCHEMA,
                claimed: true,
                issue: Some(SuiPreflightJobIssue {
                    schema: SUI_PREFLIGHT_JOB_ISSUE_SCHEMA,
                    job: leased,
                    lease_token,
                    lease_token_visible_once: true,
                    handoff,
                }),
                retry_after_seconds: 0,
                boundary: BOUNDARY.to_vec(),
            });
        }
    }
    Ok(SuiPreflightJobPoll {
        schema: SUI_PREFLIGHT_JOB_POLL_SCHEMA,
        claimed: false,
        issue: None,
        retry_after_seconds: 30,
        boundary: BOUNDARY.to_vec(),
    })
}

pub(super) fn renew(
    store: &Store,
    adapter: &SuiPreflightAdapter,
    job_id: &str,
    request: &RenewSuiPreflightJobRequest,
) -> Result<SuiPreflightJobRenew> {
    require_runtime()?;
    let lease_token = lease_token(&request.lease_token)?;
    let extend_seconds = bounded_lease_seconds(request.extend_seconds)?;
    let job = store.renew_task_sui_preflight_job(adapter, job_id, lease_token, extend_seconds)?;
    store.touch_task_sui_preflight_adapter(&adapter.id)?;
    Ok(SuiPreflightJobRenew {
        schema: SUI_PREFLIGHT_JOB_RENEW_SCHEMA,
        renewed: true,
        job,
        boundary: BOUNDARY.to_vec(),
    })
}

pub(super) fn release(
    store: &Store,
    adapter: &SuiPreflightAdapter,
    job_id: &str,
    request: &ReleaseSuiPreflightJobRequest,
) -> Result<SuiPreflightJobRelease> {
    require_runtime()?;
    let lease_token = lease_token(&request.lease_token)?;
    let reason = bounded_text(&request.reason, "释放原因", 4, 500)?;
    let job = store.release_task_sui_preflight_job(adapter, job_id, lease_token, &reason)?;
    store.touch_task_sui_preflight_adapter(&adapter.id)?;
    Ok(SuiPreflightJobRelease {
        schema: SUI_PREFLIGHT_JOB_RELEASE_SCHEMA,
        released: true,
        job,
        boundary: BOUNDARY.to_vec(),
    })
}

pub(super) fn complete(
    store: &Store,
    adapter: &SuiPreflightAdapter,
    job_id: &str,
    request: &CompleteSuiPreflightJobRequest,
) -> Result<SuiPreflightJobComplete> {
    require_runtime()?;
    let lease_token = lease_token(&request.lease_token)?;
    let job = store.task_sui_preflight_job(&adapter.project_id, job_id)?;
    let prepared = sui_preflight_service::prepare_report(
        store,
        adapter,
        &RecordSuiPreflightReportRequest {
            package_kind: job.package_kind.clone(),
            projection_package_id: job.projection_package_id.clone(),
            handoff_digest: job.handoff_digest.clone(),
            outcome: request.outcome.clone(),
            summary: request.summary.clone(),
            tool_version: request.tool_version.clone(),
            idempotency_key: request.idempotency_key.clone(),
        },
    )?;
    let (job, report) = store.complete_task_sui_preflight_job(
        adapter,
        job_id,
        lease_token,
        prepared.as_create(),
    )?;
    store.touch_task_sui_preflight_adapter(&adapter.id)?;
    Ok(SuiPreflightJobComplete {
        schema: SUI_PREFLIGHT_JOB_COMPLETE_SCHEMA,
        completed: true,
        job,
        report,
        boundary: BOUNDARY.to_vec(),
    })
}

fn verified_bundle(
    store: &Store,
    project_id: &str,
    package_kind: &str,
    projection_id: &str,
) -> Result<SuiAdapterHandoffBundle> {
    match package_kind {
        "standard" => sui_adapter_handoff_service::standard(store, project_id, projection_id),
        "correction" => sui_adapter_handoff_service::correction(store, project_id, projection_id),
        _ => bail!("未知 Sui 投影类型"),
    }
}

fn read_only_bundle(
    store: &Store,
    project_id: &str,
    package_kind: &str,
    projection_id: &str,
) -> Result<SuiAdapterHandoffBundle> {
    match package_kind {
        "standard" => {
            sui_adapter_handoff_service::standard_read_only(store, project_id, projection_id)
        }
        "correction" => {
            sui_adapter_handoff_service::correction_read_only(store, project_id, projection_id)
        }
        _ => bail!("未知 Sui 投影类型"),
    }
}

fn adapter_allows(adapter: &SuiPreflightAdapter, job: &SuiPreflightJob) -> bool {
    adapter
        .allowed_package_kinds
        .iter()
        .any(|value| value == &job.package_kind)
        && adapter
            .allowed_networks
            .iter()
            .any(|value| value == &job.target_network)
}

fn bundle_matches_job(bundle: &SuiAdapterHandoffBundle, job: &SuiPreflightJob) -> bool {
    bundle.handoff_digest == job.handoff_digest
        && bundle.payload.projection_digest == job.projection_digest
        && bundle.payload.target_network == job.target_network
        && bundle.payload.package_kind == job.package_kind
        && bundle.payload.projection_package_id == job.projection_package_id
}

fn require_runtime() -> Result<()> {
    if !sui_preflight_service::runtime_enabled() {
        bail!("Sui 离线预检机器任务入口未启用");
    }
    Ok(())
}

fn require_editor(role: &str) -> Result<()> {
    if !can_edit(role) {
        bail!("只有项目编辑者有权限管理 Sui 预检任务");
    }
    Ok(())
}

fn normalize_package_kind(value: &str) -> Result<&'static str> {
    match value.trim().to_ascii_lowercase().as_str() {
        "standard" => Ok("standard"),
        "correction" => Ok("correction"),
        _ => bail!("不支持的 Sui 投影类型"),
    }
}

fn bounded_lease_seconds(value: i64) -> Result<i64> {
    if !(60..=900).contains(&value) {
        bail!("Sui 预检任务租约必须在 60 至 900 秒之间");
    }
    Ok(value)
}

fn lease_token(value: &str) -> Result<&str> {
    let value = value.trim();
    if !value.starts_with("sui_preflight_lease_") || value.len() < 58 {
        bail!("Sui 预检任务租约凭据无效");
    }
    Ok(value)
}

fn bounded_text(value: &str, label: &str, min: usize, max: usize) -> Result<String> {
    let value = value.trim();
    let length = value.chars().count();
    if !(min..=max).contains(&length) {
        bail!("{label}长度必须在 {min} 至 {max} 个字符之间");
    }
    Ok(value.to_string())
}
