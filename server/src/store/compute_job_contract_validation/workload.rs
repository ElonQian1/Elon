use std::collections::BTreeSet;

use anyhow::{bail, Result};

use crate::compute_federation::workload::{
    ComputeArtifactRef, ComputeCheckpointPolicy, ComputeModelRef, ComputeOutputContract,
    ComputeResourceRequirements, ComputeRetryPolicy, ComputeRuntimeRef, ComputeShardSpec,
    ComputeVerificationPolicy, ComputeWorkloadSpec, COMPUTE_WORKLOAD_SCHEMA,
    DATA_CLASS_LOW_SENSITIVITY, DATA_CLASS_PUBLIC, DATA_CLASS_RESTRICTED, TASK_KIND_EMBEDDING,
    TASK_KIND_EVALUATION_SHARD, TASK_KIND_GPU_BATCH, TASK_KIND_IMAGE_GENERATION,
    TASK_KIND_LLM_CHAT, TASK_KIND_RERANK, TASK_KIND_VIDEO_GENERATION,
};

use super::{parse_utc, validate_exact_value, validate_optional_value, validate_unique_values};

pub(super) fn validate_workload(workload: &ComputeWorkloadSpec) -> Result<()> {
    if workload.schema != COMPUTE_WORKLOAD_SCHEMA {
        bail!("算力 Workload schema 不受支持");
    }
    if !matches!(
        workload.task_kind.as_str(),
        TASK_KIND_LLM_CHAT
            | TASK_KIND_EMBEDDING
            | TASK_KIND_RERANK
            | TASK_KIND_IMAGE_GENERATION
            | TASK_KIND_VIDEO_GENERATION
            | TASK_KIND_EVALUATION_SHARD
            | TASK_KIND_GPU_BATCH
    ) {
        bail!("算力 Workload 任务类型不受支持");
    }
    validate_artifacts(&workload.input_artifacts)?;
    if let Some(model) = &workload.model {
        validate_model(model)?;
    }
    if let Some(runtime) = &workload.runtime {
        validate_runtime(runtime)?;
    }
    validate_resources(&workload.resources)?;
    validate_output(&workload.output)?;
    validate_usage_limits(workload)?;
    if !matches!(
        workload.data_class.as_str(),
        DATA_CLASS_PUBLIC | DATA_CLASS_LOW_SENSITIVITY | DATA_CLASS_RESTRICTED
    ) {
        bail!("算力 Workload 数据等级不受支持");
    }
    if let Some(shard) = &workload.shard {
        validate_shard(shard)?;
    }
    validate_retry_policy(&workload.retry_policy)?;
    validate_checkpoint_policy(&workload.checkpoint_policy)?;
    validate_verification_policy(&workload.verification_policy)?;
    parse_utc("Workload 截止时间", &workload.deadline_at)?;
    Ok(())
}

fn validate_model(model: &ComputeModelRef) -> Result<()> {
    for (label, value) in [
        ("模型 ID", model.model_id.as_str()),
        ("模型家族", model.model_family.as_str()),
        ("模型摘要", model.model_digest.as_str()),
    ] {
        validate_exact_value(label, value)?;
    }
    validate_optional_value("模型 Tokenizer 摘要", model.tokenizer_digest.as_deref())?;
    validate_unique_values("模型 Adapter 摘要", &model.adapter_digests, false)?;
    Ok(())
}

fn validate_runtime(runtime: &ComputeRuntimeRef) -> Result<()> {
    for (label, value) in [
        ("运行时家族", runtime.runtime_family.as_str()),
        ("运行时版本", runtime.runtime_version.as_str()),
        ("运行时精度", runtime.precision.as_str()),
        ("Runner 摘要", runtime.runner_digest.as_str()),
    ] {
        validate_exact_value(label, value)?;
    }
    let plugin_parts = [
        runtime.plugin_id.as_deref(),
        runtime.plugin_version.as_deref(),
        runtime.plugin_digest.as_deref(),
    ];
    if plugin_parts.iter().any(|value| value.is_some())
        && plugin_parts.iter().any(|value| value.is_none())
    {
        bail!("运行时插件 ID、版本和摘要必须同时提供");
    }
    for (label, value) in [
        ("运行时插件 ID", runtime.plugin_id.as_deref()),
        ("运行时插件版本", runtime.plugin_version.as_deref()),
        ("运行时插件摘要", runtime.plugin_digest.as_deref()),
    ] {
        validate_optional_value(label, value)?;
    }
    Ok(())
}

fn validate_artifacts(artifacts: &[ComputeArtifactRef]) -> Result<()> {
    let mut artifact_ids = BTreeSet::new();
    for artifact in artifacts {
        for (label, value) in [
            ("工件 ID", artifact.artifact_id.as_str()),
            ("工件摘要算法", artifact.digest_algorithm.as_str()),
            ("工件摘要", artifact.digest.as_str()),
            ("工件媒体类型", artifact.media_type.as_str()),
            ("工件位置引用", artifact.location_ref.as_str()),
        ] {
            validate_exact_value(label, value)?;
        }
        validate_optional_value("工件加密档案", artifact.encryption_profile.as_deref())?;
        if artifact.size_bytes < 0 || !artifact_ids.insert(artifact.artifact_id.as_str()) {
            bail!("算力 Workload 工件大小无效或 ID 重复");
        }
    }
    Ok(())
}

fn validate_resources(resources: &ComputeResourceRequirements) -> Result<()> {
    validate_unique_values("加速器类型", &resources.accelerator_kinds, true)?;
    if resources.min_accelerator_count <= 0
        || resources.min_vram_bytes < 0
        || resources.min_ram_bytes <= 0
        || resources.min_disk_bytes < 0
        || resources.max_runtime_seconds <= 0
    {
        bail!("算力 Workload 资源下限或最长运行时间无效");
    }
    Ok(())
}

fn validate_output(output: &ComputeOutputContract) -> Result<()> {
    validate_exact_value("输出媒体类型", &output.media_type)?;
    if output.max_output_bytes <= 0 {
        bail!("算力 Workload 最大输出字节数必须为正整数");
    }
    Ok(())
}

fn validate_usage_limits(workload: &ComputeWorkloadSpec) -> Result<()> {
    if workload.usage_limits.is_empty() {
        bail!("算力 Workload 至少需要一个使用量上限");
    }
    let mut meters = BTreeSet::new();
    for limit in &workload.usage_limits {
        validate_exact_value("使用量 meter", &limit.meter)?;
        if limit.max_quantity <= 0 || !meters.insert(limit.meter.as_str()) {
            bail!("算力 Workload 使用量上限无效或 meter 重复");
        }
    }
    Ok(())
}

fn validate_shard(shard: &ComputeShardSpec) -> Result<()> {
    validate_exact_value("分片 ID", &shard.shard_id)?;
    validate_exact_value("分片合并策略", &shard.merge_strategy)?;
    if shard.shard_count <= 0 || shard.shard_index < 0 || shard.shard_index >= shard.shard_count {
        bail!("算力 Workload 分片序号或总数无效");
    }
    Ok(())
}

fn validate_retry_policy(policy: &ComputeRetryPolicy) -> Result<()> {
    if policy.max_attempts <= 0
        || policy.initial_backoff_ms < 0
        || policy.max_backoff_ms < policy.initial_backoff_ms
    {
        bail!("算力 Workload 重试次数或退避时间无效");
    }
    validate_unique_values("可重试错误码", &policy.retryable_error_codes, false)?;
    Ok(())
}

fn validate_checkpoint_policy(policy: &ComputeCheckpointPolicy) -> Result<()> {
    match policy.mode.as_str() {
        "disabled" => {
            if policy.interval_seconds.is_some()
                || policy.max_checkpoints != 0
                || policy.checkpoint_media_type.is_some()
            {
                bail!("disabled 检查点策略不能携带间隔、数量或媒体类型");
            }
        }
        "periodic" => {
            if !policy.interval_seconds.is_some_and(|value| value > 0)
                || policy.max_checkpoints <= 0
            {
                bail!("periodic 检查点策略需要正数间隔和数量");
            }
            validate_required_optional_value(
                "检查点媒体类型",
                policy.checkpoint_media_type.as_deref(),
            )?;
        }
        "on_signal" => {
            if policy.interval_seconds.is_some() || policy.max_checkpoints <= 0 {
                bail!("on_signal 检查点策略不能设置间隔且数量必须为正整数");
            }
            validate_required_optional_value(
                "检查点媒体类型",
                policy.checkpoint_media_type.as_deref(),
            )?;
        }
        _ => bail!("算力 Workload 检查点模式不受支持"),
    }
    Ok(())
}

fn validate_verification_policy(policy: &ComputeVerificationPolicy) -> Result<()> {
    validate_exact_value("验证等级", &policy.verification_tier)?;
    validate_optional_value("挑战档案 ID", policy.challenge_profile_id.as_deref())?;
    if policy.minimum_independent_receipts <= 0
        || !(0..=10_000).contains(&policy.duplicate_sample_rate_basis_points)
    {
        bail!("算力 Workload 独立回执数或副本抽样基点无效");
    }
    Ok(())
}

fn validate_required_optional_value(label: &str, value: Option<&str>) -> Result<()> {
    let value = value.ok_or_else(|| anyhow::anyhow!("{label}不能为空"))?;
    validate_exact_value(label, value)
}
