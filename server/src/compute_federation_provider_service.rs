use anyhow::{bail, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::{
    compute_federation::provider::{
        ComputeProvider, ComputeProviderCapabilities, ComputeProviderEvidenceProfile,
        COMPUTE_PROVIDER_SCHEMA, PROVIDER_KIND_EXTERNAL_POOL, PROVIDER_KIND_MANAGED_CLUSTER,
        PROVIDER_KIND_USER_NODE, PROVIDER_STATUS_REGISTERING,
    },
    store::{ComputeProviderRegistrationReceipt, Store},
};

const SELF_DECLARED_TRUST_TIER: &str = "self_declared";

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CreateMyComputeProviderRequest {
    pub provider_id: String,
    pub provider_kind: String,
    pub display_name: String,
    pub home_region: Option<String>,
    pub task_kinds: Vec<String>,
    pub accelerator_kinds: Vec<String>,
    pub regions: Vec<String>,
    pub allowed_data_classes: Vec<String>,
    pub supports_streaming: bool,
    pub supports_checkpointing: bool,
    pub declared_hardware_digest: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct MyComputeProviderView {
    pub provider_id: String,
    pub provider_kind: String,
    pub display_name: String,
    pub status: String,
    pub trust_tier: String,
    pub home_region: Option<String>,
    pub policy_revision: i64,
    pub capabilities: ComputeProviderCapabilities,
    pub evidence_profile: ComputeProviderEvidenceProfile,
    pub has_routing: bool,
    pub provider_digest: String,
    pub replayed: bool,
}

pub(crate) fn create_for_user(
    store: &Store,
    user_id: &str,
    request: CreateMyComputeProviderRequest,
) -> Result<MyComputeProviderView> {
    validate_create_request(&request)?;
    if request.provider_kind == PROVIDER_KIND_EXTERNAL_POOL {
        bail!("external_pool 必须由服务端适配器管理，不能通过本人接口直接创建");
    }
    if !matches!(
        request.provider_kind.as_str(),
        PROVIDER_KIND_USER_NODE | PROVIDER_KIND_MANAGED_CLUSTER
    ) {
        bail!("本人接口只支持 user_node 或 managed_cluster Provider");
    }
    let capabilities = requested_capabilities(&request);
    if let Some(mut existing) = store.compute_provider_if_exists(&request.provider_id)? {
        ensure_owned(&existing, user_id)?;
        ensure_create_replay_matches(&existing, &request, &capabilities)?;
        existing.replayed = true;
        return Ok(provider_view(existing));
    }

    let now = Utc::now().to_rfc3339();
    let receipt = store.register_compute_provider(&ComputeProvider {
        schema: COMPUTE_PROVIDER_SCHEMA.to_string(),
        provider_id: request.provider_id,
        provider_kind: request.provider_kind,
        owner_account_id: user_id.to_string(),
        settlement_account_id: Some(user_id.to_string()),
        display_name: request.display_name,
        status: PROVIDER_STATUS_REGISTERING.to_string(),
        trust_tier: SELF_DECLARED_TRUST_TIER.to_string(),
        home_region: request.home_region,
        policy_revision: 1,
        capabilities,
        endpoint: None,
        adapter: None,
        evidence_profile: ComputeProviderEvidenceProfile {
            declared_hardware_digest: request.declared_hardware_digest,
            observed_hardware_digest: None,
            verified_hardware_digest: None,
            last_observed_at: None,
            last_verified_at: None,
        },
        created_at: now.clone(),
        updated_at: now,
    })?;
    Ok(provider_view(receipt))
}

pub(crate) fn get_for_user(
    store: &Store,
    user_id: &str,
    provider_id: &str,
) -> Result<MyComputeProviderView> {
    let receipt = store.compute_provider(provider_id)?;
    ensure_owned(&receipt, user_id)?;
    Ok(provider_view(receipt))
}

pub(crate) fn list_for_user(
    store: &Store,
    user_id: &str,
    limit: usize,
) -> Result<Vec<MyComputeProviderView>> {
    store
        .list_compute_providers_for_owner(user_id, limit)?
        .into_iter()
        .map(|receipt| {
            ensure_owned(&receipt, user_id)?;
            Ok(provider_view(receipt))
        })
        .collect()
}

fn requested_capabilities(request: &CreateMyComputeProviderRequest) -> ComputeProviderCapabilities {
    ComputeProviderCapabilities {
        task_kinds: request.task_kinds.clone(),
        accelerator_kinds: request.accelerator_kinds.clone(),
        regions: request.regions.clone(),
        allowed_data_classes: request.allowed_data_classes.clone(),
        supports_streaming: request.supports_streaming,
        supports_checkpointing: request.supports_checkpointing,
    }
}

fn validate_create_request(request: &CreateMyComputeProviderRequest) -> Result<()> {
    validate_bounded("Provider ID", &request.provider_id, 160)?;
    validate_bounded("Provider 名称", &request.display_name, 160)?;
    validate_optional_bounded("Provider 所属区域", request.home_region.as_deref(), 80)?;
    validate_optional_bounded(
        "声明硬件摘要",
        request.declared_hardware_digest.as_deref(),
        256,
    )?;
    validate_values("任务类型", &request.task_kinds, 80, true)?;
    validate_values("加速器类型", &request.accelerator_kinds, 80, true)?;
    validate_values("服务区域", &request.regions, 80, false)?;
    validate_values("数据分类", &request.allowed_data_classes, 80, false)?;
    if request
        .allowed_data_classes
        .iter()
        .any(|value| !matches!(value.as_str(), "public" | "low_sensitivity" | "restricted"))
    {
        bail!("Provider 数据分类包含不受支持的值");
    }
    Ok(())
}

fn validate_values(label: &str, values: &[String], max_len: usize, required: bool) -> Result<()> {
    if required && values.is_empty() {
        bail!("Provider 至少需要一种{label}");
    }
    if values.len() > 64 {
        bail!("Provider {label}数量不能超过 64");
    }
    for value in values {
        validate_bounded(label, value, max_len)?;
    }
    Ok(())
}

fn validate_optional_bounded(label: &str, value: Option<&str>, max_len: usize) -> Result<()> {
    if let Some(value) = value {
        validate_bounded(label, value, max_len)?;
    }
    Ok(())
}

fn validate_bounded(label: &str, value: &str, max_len: usize) -> Result<()> {
    if value.trim().is_empty() || value != value.trim() {
        bail!("{label}不能为空或包含首尾空白");
    }
    if value.chars().count() > max_len {
        bail!("{label}长度不能超过 {max_len}");
    }
    Ok(())
}

fn ensure_owned(receipt: &ComputeProviderRegistrationReceipt, user_id: &str) -> Result<()> {
    if receipt.provider.owner_account_id != user_id {
        bail!("算力 Provider 不属于当前登录用户");
    }
    Ok(())
}

fn ensure_create_replay_matches(
    receipt: &ComputeProviderRegistrationReceipt,
    request: &CreateMyComputeProviderRequest,
    capabilities: &ComputeProviderCapabilities,
) -> Result<()> {
    let provider = &receipt.provider;
    if provider.provider_kind.as_str() != request.provider_kind.as_str()
        || provider.display_name.as_str() != request.display_name.as_str()
        || provider.home_region.as_ref() != request.home_region.as_ref()
        || &provider.capabilities != capabilities
        || provider.evidence_profile.declared_hardware_digest.as_ref()
            != request.declared_hardware_digest.as_ref()
    {
        bail!("算力 Provider ID 已绑定不同的供给声明");
    }
    Ok(())
}

fn provider_view(receipt: ComputeProviderRegistrationReceipt) -> MyComputeProviderView {
    let provider = receipt.provider;
    MyComputeProviderView {
        provider_id: provider.provider_id,
        provider_kind: provider.provider_kind,
        display_name: provider.display_name,
        status: provider.status,
        trust_tier: provider.trust_tier,
        home_region: provider.home_region,
        policy_revision: provider.policy_revision,
        capabilities: provider.capabilities,
        evidence_profile: provider.evidence_profile,
        has_routing: provider.endpoint.is_some() || provider.adapter.is_some(),
        provider_digest: receipt.provider_digest,
        replayed: receipt.replayed,
    }
}
