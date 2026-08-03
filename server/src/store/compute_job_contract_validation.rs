use std::collections::{BTreeMap, BTreeSet};

use anyhow::{bail, Context, Result};
use chrono::{DateTime, FixedOffset};
use sha2::{Digest, Sha256};

use crate::compute_federation::{
    execution::{
        ComputeJob, ComputeProviderScope, JOB_STATUS_CANCELED, JOB_STATUS_FAILED,
        JOB_STATUS_QUOTED, JOB_STATUS_RESERVED, JOB_STATUS_RUNNING, JOB_STATUS_SETTLED,
        JOB_STATUS_SUBMITTED, JOB_STATUS_VERIFICATION_PENDING,
    },
    market::ComputePriceSnapshot,
    offer::ComputeOffer,
    provider::{
        ComputeProvider, PROVIDER_KIND_EXTERNAL_POOL, PROVIDER_KIND_MANAGED_CLUSTER,
        PROVIDER_KIND_USER_NODE,
    },
    workload::ComputeWorkloadSpec,
};

mod workload;

use self::workload::validate_workload;

pub(super) fn validate_job_contract(
    job: &ComputeJob,
    offer: Option<&ComputeOffer>,
    snapshot: Option<&ComputePriceSnapshot>,
    provider: Option<&ComputeProvider>,
) -> Result<String> {
    validate_job_identity(job)?;
    validate_provider_scope(&job.provider_scope)?;
    validate_workload(&job.workload)?;
    validate_job_times(job)?;
    validate_selection_shape(job, offer, snapshot, provider)?;
    if let (Some(offer), Some(snapshot), Some(provider)) = (offer, snapshot, provider) {
        validate_selected_contract(job, offer, snapshot, provider)?;
    }
    compute_job_digest(job)
}

pub(super) fn compute_job_digest(job: &ComputeJob) -> Result<String> {
    let encoded = serde_json::to_vec(job)?;
    Ok(hex::encode(Sha256::digest(encoded)))
}

fn validate_job_identity(job: &ComputeJob) -> Result<()> {
    if job.schema != crate::compute_federation::execution::COMPUTE_JOB_SCHEMA {
        bail!("算力 Job schema 不受支持");
    }
    for (label, value) in [
        ("Job ID", job.job_id.as_str()),
        ("消费者账户 ID", job.consumer_account_id.as_str()),
        ("Job 幂等键", job.idempotency_key.as_str()),
        ("Job 币种", job.currency.as_str()),
    ] {
        validate_exact_value(label, value)?;
    }
    validate_optional_value("项目 ID", job.project_id.as_deref())?;
    validate_optional_value("商户 ID", job.merchant_id.as_deref())?;
    if job.max_consumer_charge_micros < 0 {
        bail!("Job 最大消费者金额不能为负数");
    }
    if !matches!(
        job.status.as_str(),
        JOB_STATUS_SUBMITTED
            | JOB_STATUS_QUOTED
            | JOB_STATUS_RESERVED
            | JOB_STATUS_RUNNING
            | JOB_STATUS_VERIFICATION_PENDING
            | JOB_STATUS_SETTLED
            | JOB_STATUS_FAILED
            | JOB_STATUS_CANCELED
    ) {
        bail!("算力 Job 状态不受支持");
    }
    Ok(())
}

fn validate_provider_scope(scope: &ComputeProviderScope) -> Result<()> {
    validate_unique_values("允许 Provider ID", &scope.allowed_provider_ids, false)?;
    validate_unique_values("允许 Provider 类型", &scope.allowed_provider_kinds, false)?;
    validate_unique_values("排除 Provider ID", &scope.excluded_provider_ids, false)?;
    validate_unique_values("要求区域", &scope.required_regions, false)?;
    validate_exact_value("要求信任等级", &scope.required_trust_tier)?;
    if scope.allowed_provider_kinds.iter().any(|value| {
        !matches!(
            value.as_str(),
            PROVIDER_KIND_USER_NODE | PROVIDER_KIND_MANAGED_CLUSTER | PROVIDER_KIND_EXTERNAL_POOL
        )
    }) {
        bail!("允许 Provider 类型包含不受支持的值");
    }
    let allowed = scope
        .allowed_provider_ids
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if scope
        .excluded_provider_ids
        .iter()
        .any(|provider_id| allowed.contains(provider_id.as_str()))
    {
        bail!("同一个 Provider 不能同时出现在允许和排除列表");
    }
    Ok(())
}

fn validate_job_times(job: &ComputeJob) -> Result<()> {
    let submitted_at = parse_utc("Job 提交时间", &job.submitted_at)?;
    let updated_at = parse_utc("Job 更新时间", &job.updated_at)?;
    let deadline_at = parse_utc("Workload 截止时间", &job.workload.deadline_at)?;
    if submitted_at > updated_at || updated_at >= deadline_at {
        bail!("算力 Job 必须满足 submitted_at <= updated_at < deadline_at");
    }
    Ok(())
}

fn validate_selection_shape(
    job: &ComputeJob,
    offer: Option<&ComputeOffer>,
    snapshot: Option<&ComputePriceSnapshot>,
    provider: Option<&ComputeProvider>,
) -> Result<()> {
    let job_has_selection = job.selected_offer.is_some() || job.price_snapshot_id.is_some();
    if job.selected_offer.is_some() != job.price_snapshot_id.is_some() {
        bail!("算力 Job 必须同时绑定 Offer 与 Price Snapshot");
    }
    let supplied_selection = offer.is_some() || snapshot.is_some() || provider.is_some();
    if supplied_selection && !(offer.is_some() && snapshot.is_some() && provider.is_some()) {
        bail!("算力 Job 校验必须同时提供 Offer、Price Snapshot 与 Provider");
    }
    if job_has_selection != supplied_selection {
        bail!("算力 Job 选择字段与校验上下文不一致");
    }
    let requires_selection = matches!(
        job.status.as_str(),
        JOB_STATUS_QUOTED
            | JOB_STATUS_RESERVED
            | JOB_STATUS_RUNNING
            | JOB_STATUS_VERIFICATION_PENDING
            | JOB_STATUS_SETTLED
    );
    if requires_selection && !job_has_selection {
        bail!("quoted 及后续 Job 状态必须绑定 Offer 与 Price Snapshot");
    }
    if job.status == JOB_STATUS_SUBMITTED && job_has_selection {
        bail!("submitted Job 不能提前绑定 Offer 或 Price Snapshot");
    }
    Ok(())
}

fn validate_selected_contract(
    job: &ComputeJob,
    offer: &ComputeOffer,
    snapshot: &ComputePriceSnapshot,
    provider: &ComputeProvider,
) -> Result<()> {
    let selected = job
        .selected_offer
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("算力 Job 缺少 Offer 绑定"))?;
    if selected.provider_id != offer.provider_id
        || selected.offer_id != offer.offer_id
        || selected.offer_version != offer.offer_version
        || selected.offer_digest != offer.offer_digest
        || job.price_snapshot_id.as_deref() != Some(snapshot.snapshot_id.as_str())
        || snapshot.offer_id != offer.offer_id
        || snapshot.offer_version != offer.offer_version
        || snapshot.offer_digest != offer.offer_digest
        || offer.provider_id != provider.provider_id
        || snapshot.provider_id != provider.provider_id
    {
        bail!("算力 Job、Offer、Price Snapshot 与 Provider 绑定不一致");
    }
    validate_scope_match(job, offer, provider)?;
    validate_workload_match(job, offer, snapshot, provider)?;
    if job.currency != snapshot.currency
        || job.max_consumer_charge_micros < snapshot.consumer_max_amount_micros
    {
        bail!("算力 Job 币种或最大消费者预算不能覆盖 Price Snapshot");
    }
    Ok(())
}

fn validate_scope_match(
    job: &ComputeJob,
    offer: &ComputeOffer,
    provider: &ComputeProvider,
) -> Result<()> {
    let scope = &job.provider_scope;
    if scope
        .excluded_provider_ids
        .iter()
        .any(|value| value == &provider.provider_id)
        || (!scope.allowed_provider_ids.is_empty()
            && !scope
                .allowed_provider_ids
                .iter()
                .any(|value| value == &provider.provider_id))
        || (!scope.allowed_provider_kinds.is_empty()
            && !scope
                .allowed_provider_kinds
                .iter()
                .any(|value| value == &provider.provider_kind))
        || (!scope.required_regions.is_empty()
            && !scope
                .required_regions
                .iter()
                .any(|value| value == &offer.sku.region_or_data_zone))
        || scope.required_trust_tier != provider.trust_tier
    {
        bail!("所选 Provider 不满足 Job 的 Provider 范围");
    }
    Ok(())
}

fn validate_workload_match(
    job: &ComputeJob,
    offer: &ComputeOffer,
    snapshot: &ComputePriceSnapshot,
    provider: &ComputeProvider,
) -> Result<()> {
    let workload = &job.workload;
    if workload.task_kind != offer.sku.task_kind
        || workload
            .model
            .as_ref()
            .is_some_and(|model| offer.model.as_ref() != Some(model))
        || workload
            .runtime
            .as_ref()
            .is_some_and(|runtime| runtime != &offer.runtime)
        || !workload
            .resources
            .accelerator_kinds
            .iter()
            .any(|kind| kind == &offer.resource_profile.accelerator_kind)
        || workload.resources.min_accelerator_count > offer.resource_profile.accelerator_count
        || workload.resources.min_vram_bytes > offer.resource_profile.vram_bytes
        || workload.resources.min_ram_bytes > offer.resource_profile.ram_bytes
        || workload.resources.max_runtime_seconds
            > offer.execution_limits.max_attempt_runtime_seconds
    {
        bail!("所选 Offer 不能满足 Job 的模型、运行时或资源要求");
    }
    if workload.output.streaming && !provider.capabilities.supports_streaming {
        bail!("所选 Provider 不支持 Job 要求的流式输出");
    }
    if workload.checkpoint_policy.mode != "disabled"
        && !provider.capabilities.supports_checkpointing
    {
        bail!("所选 Provider 不支持 Job 要求的检查点");
    }
    if !offer.authorization.allowed_data_classes.is_empty()
        && !offer
            .authorization
            .allowed_data_classes
            .iter()
            .any(|value| value == &workload.data_class)
    {
        bail!("所选 Offer 未授权 Job 的数据等级");
    }
    if !provider
        .capabilities
        .allowed_data_classes
        .iter()
        .any(|value| value == &workload.data_class)
    {
        bail!("所选 Provider 不支持 Job 的数据等级");
    }
    validate_offer_audience(job, offer)?;
    validate_usage_match(workload, snapshot)?;
    let deadline = parse_utc("Workload 截止时间", &workload.deadline_at)?;
    let window_start = parse_utc(
        "Price Snapshot 窗口开始时间",
        &snapshot.delivery_window.starts_at_utc,
    )?;
    let window_end = parse_utc(
        "Price Snapshot 窗口结束时间",
        &snapshot.delivery_window.ends_at_utc,
    )?;
    if deadline <= window_start || deadline > window_end {
        bail!("Job 截止时间必须位于 Price Snapshot 交付窗口内");
    }
    Ok(())
}

fn validate_offer_audience(job: &ComputeJob, offer: &ComputeOffer) -> Result<()> {
    if offer.authorization.public {
        return Ok(());
    }
    let account_allowed = offer
        .authorization
        .allowed_account_ids
        .iter()
        .any(|value| value == &job.consumer_account_id);
    let project_allowed = job.project_id.as_ref().is_some_and(|project_id| {
        offer
            .authorization
            .allowed_project_ids
            .iter()
            .any(|value| value == project_id)
    });
    if !account_allowed && !project_allowed {
        bail!("消费者账户或项目不在所选 Offer 授权范围内");
    }
    Ok(())
}

fn validate_usage_match(
    workload: &ComputeWorkloadSpec,
    snapshot: &ComputePriceSnapshot,
) -> Result<()> {
    let limits = workload
        .usage_limits
        .iter()
        .map(|limit| (limit.meter.as_str(), limit.max_quantity))
        .collect::<BTreeMap<_, _>>();
    if limits.len() != snapshot.components.len() {
        bail!("Job 使用量上限必须覆盖 Price Snapshot 的全部 meter");
    }
    for component in &snapshot.components {
        let Some(max_quantity) = limits.get(component.meter.as_str()) else {
            bail!("Job 缺少 Price Snapshot meter 的使用量上限");
        };
        if component.unit_size <= 0
            || component.max_units <= 0
            || *max_quantity > component.max_units
            || *max_quantity % component.unit_size != 0
        {
            bail!("Job 使用量上限超过 Price Snapshot 或不符合计价粒度");
        }
    }
    Ok(())
}

fn validate_unique_values(label: &str, values: &[String], required: bool) -> Result<()> {
    if required && values.is_empty() {
        bail!("{label}不能为空");
    }
    let mut unique = BTreeSet::new();
    for value in values {
        validate_exact_value(label, value)?;
        if !unique.insert(value.as_str()) {
            bail!("{label}不能重复");
        }
    }
    Ok(())
}

fn validate_exact_value(label: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        bail!("{label}不能为空");
    }
    if value != value.trim() {
        bail!("{label}不能包含首尾空白");
    }
    Ok(())
}

fn validate_optional_value(label: &str, value: Option<&str>) -> Result<()> {
    if let Some(value) = value {
        validate_exact_value(label, value)?;
    }
    Ok(())
}

fn parse_utc(label: &str, value: &str) -> Result<DateTime<FixedOffset>> {
    let parsed =
        DateTime::parse_from_rfc3339(value).with_context(|| format!("{label}不是 RFC3339"))?;
    if parsed.offset().local_minus_utc() != 0 {
        bail!("{label}必须使用 UTC 时区");
    }
    Ok(parsed)
}
