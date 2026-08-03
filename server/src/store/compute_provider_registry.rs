use std::collections::BTreeSet;

use anyhow::{anyhow, bail, Context, Result};
use chrono::DateTime;
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::compute_federation::provider::{
    ComputeProvider, ComputeProviderAdapterRef, ComputeProviderCapabilities,
    ComputeProviderEndpointRef, ComputeProviderEvidenceProfile, COMPUTE_PROVIDER_SCHEMA,
    PROVIDER_KIND_EXTERNAL_POOL, PROVIDER_KIND_MANAGED_CLUSTER, PROVIDER_KIND_USER_NODE,
    PROVIDER_STATUS_ACTIVE, PROVIDER_STATUS_DISABLED, PROVIDER_STATUS_DRAINING,
    PROVIDER_STATUS_QUARANTINED, PROVIDER_STATUS_REGISTERING,
};

use super::{now, Store};

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ComputeProviderRegistrationReceipt {
    pub provider: ComputeProvider,
    pub provider_digest: String,
    pub replayed: bool,
}

#[derive(Debug)]
struct CurrentProviderProjection {
    provider_id: String,
    provider_kind: String,
    owner_account_id: String,
    settlement_account_id: Option<String>,
    display_name: String,
    status: String,
    trust_tier: String,
    home_region: Option<String>,
    current_policy_revision: i64,
    current_provider_digest: String,
    created_at: String,
    updated_at: String,
}

impl Store {
    pub(crate) fn register_compute_provider(
        &self,
        provider: &ComputeProvider,
    ) -> Result<ComputeProviderRegistrationReceipt> {
        validate_provider(provider)?;
        let provider_json = serde_json::to_string(provider)?;
        let provider_digest = sha256_hex(provider_json.as_bytes());
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let current = current_projection_on(&tx, provider.provider_id.trim())?;

        if let Some(current) = current {
            let current_version = provider_version_on(
                &tx,
                provider.provider_id.trim(),
                current.current_policy_revision,
            )?
            .ok_or_else(|| anyhow!("算力提供者当前历史版本缺失，拒绝继续写入"))?;
            audited_provider(&current, &current_version)?;
            if provider.policy_revision <= current.current_policy_revision {
                let stored = provider_version_on(
                    &tx,
                    provider.provider_id.trim(),
                    provider.policy_revision,
                )?
                .ok_or_else(|| anyhow!("算力提供者历史版本缺失，拒绝覆盖"))?;
                ensure_replay_matches(provider, &provider_json, &provider_digest, &stored)?;
                if provider.policy_revision == current.current_policy_revision
                    && (current.current_provider_digest != provider_digest
                        || current.status != provider.status)
                {
                    bail!("算力提供者当前投影与历史版本不一致");
                }
                tx.commit()?;
                return Ok(ComputeProviderRegistrationReceipt {
                    provider: provider.clone(),
                    provider_digest,
                    replayed: true,
                });
            }

            ensure_stable_identity(provider, &current)?;
            if provider.policy_revision != current.current_policy_revision + 1 {
                bail!(
                    "算力提供者策略版本必须连续递增，当前版本为 {}",
                    current.current_policy_revision
                );
            }
            if !status_transition_allowed(&current.status, &provider.status) {
                bail!(
                    "算力提供者状态不允许从 {} 变更为 {}",
                    current.status,
                    provider.status
                );
            }

            insert_provider_version(&tx, provider, &provider_digest, &provider_json)?;
            let updated = tx.execute(
                "UPDATE compute_providers
                    SET settlement_account_id=?1, display_name=?2, status=?3,
                        trust_tier=?4, home_region=?5, current_policy_revision=?6,
                        current_provider_digest=?7, updated_at=?8
                  WHERE provider_id=?9 AND current_policy_revision=?10
                    AND current_provider_digest=?11",
                params![
                    clean_optional(provider.settlement_account_id.as_deref()),
                    provider.display_name.trim(),
                    provider.status.trim(),
                    provider.trust_tier.trim(),
                    clean_optional(provider.home_region.as_deref()),
                    provider.policy_revision,
                    provider_digest,
                    provider.updated_at.trim(),
                    provider.provider_id.trim(),
                    current.current_policy_revision,
                    current.current_provider_digest,
                ],
            )?;
            if updated != 1 {
                bail!("算力提供者当前投影已变化，请基于最新版本重试");
            }
        } else {
            if provider.policy_revision != 1 || provider.status != PROVIDER_STATUS_REGISTERING {
                bail!("新算力提供者必须以 registering 状态和策略版本 1 创建");
            }
            tx.execute(
                "INSERT INTO compute_providers (
                    provider_id, provider_kind, owner_account_id,
                    settlement_account_id, display_name, status, trust_tier,
                    home_region, current_policy_revision, current_provider_digest,
                    created_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                params![
                    provider.provider_id.trim(),
                    provider.provider_kind.trim(),
                    provider.owner_account_id.trim(),
                    clean_optional(provider.settlement_account_id.as_deref()),
                    provider.display_name.trim(),
                    provider.status.trim(),
                    provider.trust_tier.trim(),
                    clean_optional(provider.home_region.as_deref()),
                    provider.policy_revision,
                    provider_digest,
                    provider.created_at.trim(),
                    provider.updated_at.trim(),
                ],
            )?;
            insert_provider_version(&tx, provider, &provider_digest, &provider_json)?;
        }

        tx.commit()?;
        Ok(ComputeProviderRegistrationReceipt {
            provider: provider.clone(),
            provider_digest,
            replayed: false,
        })
    }

    pub(crate) fn compute_provider(
        &self,
        provider_id: &str,
    ) -> Result<ComputeProviderRegistrationReceipt> {
        if provider_id.trim().is_empty() {
            bail!("算力提供者 ID 不能为空");
        }
        let conn = self.conn()?;
        current_registered_provider_on(&conn, provider_id.trim())?
            .ok_or_else(|| anyhow!("算力提供者不存在"))
    }
}

#[derive(Debug)]
struct StoredProviderVersion {
    provider_digest: String,
    provider_json: String,
}

pub(super) fn current_registered_provider_on(
    conn: &Connection,
    provider_id: &str,
) -> Result<Option<ComputeProviderRegistrationReceipt>> {
    let Some((projection, stored)) = current_provider_version_on(conn, provider_id)? else {
        return Ok(None);
    };
    let provider = audited_provider(&projection, &stored)?;
    Ok(Some(ComputeProviderRegistrationReceipt {
        provider,
        provider_digest: stored.provider_digest,
        replayed: false,
    }))
}

fn current_projection_on(
    conn: &Connection,
    provider_id: &str,
) -> Result<Option<CurrentProviderProjection>> {
    conn.query_row(
        "SELECT provider_id, provider_kind, owner_account_id, settlement_account_id, display_name,
                status, trust_tier, home_region, current_policy_revision,
                current_provider_digest, created_at, updated_at
           FROM compute_providers WHERE provider_id=?1",
        params![provider_id],
        |row| {
            Ok(CurrentProviderProjection {
                provider_id: row.get(0)?,
                provider_kind: row.get(1)?,
                owner_account_id: row.get(2)?,
                settlement_account_id: row.get(3)?,
                display_name: row.get(4)?,
                status: row.get(5)?,
                trust_tier: row.get(6)?,
                home_region: row.get(7)?,
                current_policy_revision: row.get(8)?,
                current_provider_digest: row.get(9)?,
                created_at: row.get(10)?,
                updated_at: row.get(11)?,
            })
        },
    )
    .optional()
    .map_err(Into::into)
}

fn current_provider_version_on(
    conn: &Connection,
    provider_id: &str,
) -> Result<Option<(CurrentProviderProjection, StoredProviderVersion)>> {
    conn.query_row(
        "SELECT p.provider_id, p.provider_kind, p.owner_account_id, p.settlement_account_id,
                p.display_name, p.status, p.trust_tier, p.home_region,
                p.current_policy_revision, p.current_provider_digest,
                p.created_at, p.updated_at, v.provider_digest, v.provider_json
           FROM compute_providers p
           JOIN compute_provider_versions v
             ON v.provider_id=p.provider_id
            AND v.policy_revision=p.current_policy_revision
          WHERE p.provider_id=?1",
        params![provider_id],
        |row| {
            Ok((
                CurrentProviderProjection {
                    provider_id: row.get(0)?,
                    provider_kind: row.get(1)?,
                    owner_account_id: row.get(2)?,
                    settlement_account_id: row.get(3)?,
                    display_name: row.get(4)?,
                    status: row.get(5)?,
                    trust_tier: row.get(6)?,
                    home_region: row.get(7)?,
                    current_policy_revision: row.get(8)?,
                    current_provider_digest: row.get(9)?,
                    created_at: row.get(10)?,
                    updated_at: row.get(11)?,
                },
                StoredProviderVersion {
                    provider_digest: row.get(12)?,
                    provider_json: row.get(13)?,
                },
            ))
        },
    )
    .optional()
    .map_err(Into::into)
}

fn audited_provider(
    projection: &CurrentProviderProjection,
    stored: &StoredProviderVersion,
) -> Result<ComputeProvider> {
    let provider: ComputeProvider =
        serde_json::from_str(&stored.provider_json).context("算力提供者版本 JSON 无效")?;
    validate_provider(&provider)?;
    let recomputed_digest = sha256_hex(stored.provider_json.as_bytes());
    if recomputed_digest != stored.provider_digest
        || recomputed_digest != projection.current_provider_digest
        || provider.provider_id != projection.provider_id
        || provider.provider_kind != projection.provider_kind
        || provider.owner_account_id != projection.owner_account_id
        || provider.settlement_account_id != projection.settlement_account_id
        || provider.display_name != projection.display_name
        || provider.status != projection.status
        || provider.trust_tier != projection.trust_tier
        || provider.home_region != projection.home_region
        || provider.policy_revision != projection.current_policy_revision
        || provider.created_at != projection.created_at
        || provider.updated_at != projection.updated_at
    {
        bail!("算力提供者当前投影或版本摘要审计失败");
    }
    Ok(provider)
}

fn provider_version_on(
    conn: &Connection,
    provider_id: &str,
    policy_revision: i64,
) -> Result<Option<StoredProviderVersion>> {
    conn.query_row(
        "SELECT provider_digest, provider_json
           FROM compute_provider_versions
          WHERE provider_id=?1 AND policy_revision=?2",
        params![provider_id, policy_revision],
        |row| {
            Ok(StoredProviderVersion {
                provider_digest: row.get(0)?,
                provider_json: row.get(1)?,
            })
        },
    )
    .optional()
    .map_err(Into::into)
}

fn insert_provider_version(
    conn: &Connection,
    provider: &ComputeProvider,
    provider_digest: &str,
    provider_json: &str,
) -> Result<()> {
    conn.execute(
        "INSERT INTO compute_provider_versions (
            provider_id, policy_revision, provider_digest, provider_json, created_at
         ) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            provider.provider_id.trim(),
            provider.policy_revision,
            provider_digest,
            provider_json,
            now(),
        ],
    )?;
    Ok(())
}

fn ensure_replay_matches(
    provider: &ComputeProvider,
    provider_json: &str,
    provider_digest: &str,
    stored: &StoredProviderVersion,
) -> Result<()> {
    if stored.provider_digest != provider_digest || stored.provider_json != provider_json {
        bail!(
            "算力提供者 {} 的策略版本 {} 已绑定不同合同",
            provider.provider_id,
            provider.policy_revision
        );
    }
    Ok(())
}

fn ensure_stable_identity(
    provider: &ComputeProvider,
    current: &CurrentProviderProjection,
) -> Result<()> {
    if provider.provider_kind != current.provider_kind
        || provider.owner_account_id != current.owner_account_id
        || provider.created_at != current.created_at
    {
        bail!("算力提供者类型、所有者和创建时间不能原地改变");
    }
    Ok(())
}

fn status_transition_allowed(current: &str, next: &str) -> bool {
    if current == PROVIDER_STATUS_DISABLED {
        return false;
    }
    current == next
        || matches!(
            (current, next),
            (PROVIDER_STATUS_REGISTERING, PROVIDER_STATUS_ACTIVE)
                | (PROVIDER_STATUS_REGISTERING, PROVIDER_STATUS_QUARANTINED)
                | (PROVIDER_STATUS_REGISTERING, PROVIDER_STATUS_DISABLED)
                | (PROVIDER_STATUS_ACTIVE, PROVIDER_STATUS_DRAINING)
                | (PROVIDER_STATUS_ACTIVE, PROVIDER_STATUS_QUARANTINED)
                | (PROVIDER_STATUS_ACTIVE, PROVIDER_STATUS_DISABLED)
                | (PROVIDER_STATUS_DRAINING, PROVIDER_STATUS_ACTIVE)
                | (PROVIDER_STATUS_DRAINING, PROVIDER_STATUS_QUARANTINED)
                | (PROVIDER_STATUS_DRAINING, PROVIDER_STATUS_DISABLED)
                | (PROVIDER_STATUS_QUARANTINED, PROVIDER_STATUS_ACTIVE)
                | (PROVIDER_STATUS_QUARANTINED, PROVIDER_STATUS_DRAINING)
                | (PROVIDER_STATUS_QUARANTINED, PROVIDER_STATUS_DISABLED)
        )
}

fn validate_provider(provider: &ComputeProvider) -> Result<()> {
    if provider.schema != COMPUTE_PROVIDER_SCHEMA {
        bail!("算力提供者 schema 不受支持");
    }
    validate_exact_value("算力提供者 ID", &provider.provider_id)?;
    validate_exact_value("算力提供者所有者", &provider.owner_account_id)?;
    validate_exact_value("算力提供者名称", &provider.display_name)?;
    validate_exact_value("算力提供者信任等级", &provider.trust_tier)?;
    validate_optional_value(
        "算力提供者结算账户",
        provider.settlement_account_id.as_deref(),
    )?;
    validate_optional_value("算力提供者所属区域", provider.home_region.as_deref())?;
    if !matches!(
        provider.provider_kind.as_str(),
        PROVIDER_KIND_USER_NODE | PROVIDER_KIND_MANAGED_CLUSTER | PROVIDER_KIND_EXTERNAL_POOL
    ) {
        bail!("算力提供者类型不受支持");
    }
    if !matches!(
        provider.status.as_str(),
        PROVIDER_STATUS_REGISTERING
            | PROVIDER_STATUS_ACTIVE
            | PROVIDER_STATUS_DRAINING
            | PROVIDER_STATUS_DISABLED
            | PROVIDER_STATUS_QUARANTINED
    ) {
        bail!("算力提供者状态不受支持");
    }
    if provider.policy_revision <= 0 {
        bail!("算力提供者策略版本必须为正整数");
    }
    validate_capabilities(&provider.capabilities, provider.status.as_str())?;
    if let Some(endpoint) = &provider.endpoint {
        validate_endpoint(endpoint)?;
    }
    if let Some(adapter) = &provider.adapter {
        validate_adapter(adapter)?;
    }
    if provider.status == PROVIDER_STATUS_ACTIVE
        && provider.endpoint.is_none()
        && provider.adapter.is_none()
    {
        bail!("active 算力提供者必须配置可路由端点或适配器");
    }
    if provider.provider_kind == PROVIDER_KIND_EXTERNAL_POOL && provider.adapter.is_none() {
        bail!("external_pool 算力提供者必须配置服务端适配器");
    }
    validate_evidence_profile(&provider.evidence_profile)?;
    let created_at = parse_utc("算力提供者创建时间", &provider.created_at)?;
    let updated_at = parse_utc("算力提供者更新时间", &provider.updated_at)?;
    if created_at > updated_at {
        bail!("算力提供者更新时间不能早于创建时间");
    }
    Ok(())
}

fn validate_capabilities(capabilities: &ComputeProviderCapabilities, status: &str) -> Result<()> {
    validate_string_set("任务类型", &capabilities.task_kinds, true)?;
    validate_string_set("加速器类型", &capabilities.accelerator_kinds, true)?;
    validate_string_set(
        "服务区域",
        &capabilities.regions,
        status == PROVIDER_STATUS_ACTIVE,
    )?;
    validate_string_set("允许的数据分类", &capabilities.allowed_data_classes, false)?;
    Ok(())
}

fn validate_endpoint(endpoint: &ComputeProviderEndpointRef) -> Result<()> {
    validate_exact_value("算力提供者端点 ID", &endpoint.endpoint_id)?;
    validate_exact_value("算力提供者端点传输协议", &endpoint.transport)?;
    validate_optional_value("算力提供者地址提示", endpoint.address_hint.as_deref())?;
    validate_optional_value("算力提供者网关 ID", endpoint.gateway_id.as_deref())?;
    validate_optional_value("算力提供者凭据引用", endpoint.credential_ref.as_deref())?;
    Ok(())
}

fn validate_adapter(adapter: &ComputeProviderAdapterRef) -> Result<()> {
    validate_exact_value("算力适配器 ID", &adapter.adapter_id)?;
    validate_exact_value("算力适配器版本", &adapter.adapter_version)?;
    validate_exact_value("算力适配器配置摘要", &adapter.config_digest)?;
    if adapter.config_revision <= 0 {
        bail!("算力适配器配置版本必须为正整数");
    }
    Ok(())
}

fn validate_evidence_profile(profile: &ComputeProviderEvidenceProfile) -> Result<()> {
    validate_optional_value("声明硬件摘要", profile.declared_hardware_digest.as_deref())?;
    validate_optional_value("观测硬件摘要", profile.observed_hardware_digest.as_deref())?;
    validate_optional_value("验证硬件摘要", profile.verified_hardware_digest.as_deref())?;
    if let Some(value) = profile.last_observed_at.as_deref() {
        parse_utc("硬件最后观测时间", value)?;
    }
    if let Some(value) = profile.last_verified_at.as_deref() {
        parse_utc("硬件最后验证时间", value)?;
    }
    Ok(())
}

fn validate_string_set(label: &str, values: &[String], required: bool) -> Result<()> {
    if required && values.is_empty() {
        bail!("算力提供者至少需要一种{label}");
    }
    let mut unique = BTreeSet::new();
    for value in values {
        validate_exact_value(label, value)?;
        if !unique.insert(value.as_str()) {
            bail!("算力提供者{label}不能重复");
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

fn parse_utc(label: &str, value: &str) -> Result<DateTime<chrono::FixedOffset>> {
    let parsed =
        DateTime::parse_from_rfc3339(value).with_context(|| format!("{label}不是 RFC3339"))?;
    if parsed.offset().local_minus_utc() != 0 {
        bail!("{label}必须使用 UTC 时区");
    }
    Ok(parsed)
}

fn clean_optional(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn sha256_hex(value: &[u8]) -> String {
    hex::encode(Sha256::digest(value))
}
