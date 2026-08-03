use std::collections::BTreeSet;

use anyhow::{bail, Result};
use chrono::{Duration, Utc};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::{project_auth::can_edit, store::Store};

use super::{
    sui_adapter_handoff_model::SuiAdapterHandoffBundle,
    sui_adapter_handoff_service,
    sui_preflight_model::{
        CreateSuiPreflightAdapterRequest, CreateSuiPreflightReport,
        RecordSuiPreflightReportRequest, SuiPreflightAdapter, SuiPreflightAdapterIssue,
        SuiPreflightAdapterList, SuiPreflightReport, SuiPreflightReportList,
        SUI_PREFLIGHT_ADAPTER_LIST_SCHEMA, SUI_PREFLIGHT_REPORT_LIST_SCHEMA,
    },
};

const RUNTIME_FLAG: &str = "ELON_SUI_OFFLINE_PREFLIGHT_ENABLED";
const BOUNDARY: [&str; 7] = [
    "预检适配器凭据明文只在签发或轮换时返回一次，服务端只保存 SHA-256",
    "机器端只能报告服务端可重新生成且摘要一致的链下交接包",
    "预检结果只有 passed 或 rejected，均不代表链上交易、终局或资金变动",
    "预检报告为追加式证据，同一幂等键不能覆盖为不同结果",
    "预检报告不会改写投影完整性、提交就绪状态或网络提交状态",
    "适配器只能处理档案中明确允许的目标网络和投影类型",
    "机器报告入口默认关闭，只有显式启用环境开关后才接受请求",
];

pub(super) fn runtime_enabled() -> bool {
    std::env::var(RUNTIME_FLAG)
        .ok()
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false)
}

pub(super) fn list_adapters(store: &Store, project_id: &str) -> Result<SuiPreflightAdapterList> {
    Ok(SuiPreflightAdapterList {
        schema: SUI_PREFLIGHT_ADAPTER_LIST_SCHEMA,
        project_id: project_id.trim().to_string(),
        runtime_enabled: runtime_enabled(),
        adapters: store.list_task_sui_preflight_adapters(project_id)?,
        boundary: BOUNDARY.to_vec(),
    })
}

pub(super) fn create_adapter(
    store: &Store,
    project_id: &str,
    actor_user_id: &str,
    actor_role: &str,
    request: &CreateSuiPreflightAdapterRequest,
) -> Result<SuiPreflightAdapterIssue> {
    require_editor(actor_role)?;
    let display_name = bounded_text(&request.display_name, "适配器名称", 2, 80)?;
    let allowed_networks = normalize_options(
        &request.allowed_networks,
        &["devnet", "testnet", "mainnet"],
        "目标网络",
    )?;
    let allowed_package_kinds = normalize_options(
        &request.allowed_package_kinds,
        &["standard", "correction"],
        "投影类型",
    )?;
    let expires_at = expiration_from_days(request.expires_in_days)?;
    store.create_task_sui_preflight_adapter(
        project_id,
        &display_name,
        &allowed_networks,
        &allowed_package_kinds,
        &expires_at,
        actor_user_id,
    )
}

pub(super) fn rotate_adapter(
    store: &Store,
    project_id: &str,
    adapter_id: &str,
    expires_in_days: i64,
    actor_role: &str,
) -> Result<SuiPreflightAdapterIssue> {
    require_editor(actor_role)?;
    let expires_at = expiration_from_days(expires_in_days)?;
    store.rotate_task_sui_preflight_adapter(project_id, adapter_id, &expires_at)
}

pub(super) fn disable_adapter(
    store: &Store,
    project_id: &str,
    adapter_id: &str,
    actor_role: &str,
) -> Result<SuiPreflightAdapter> {
    require_editor(actor_role)?;
    store.disable_task_sui_preflight_adapter(project_id, adapter_id)
}

pub(super) fn list_reports(store: &Store, project_id: &str) -> Result<SuiPreflightReportList> {
    Ok(SuiPreflightReportList {
        schema: SUI_PREFLIGHT_REPORT_LIST_SCHEMA,
        project_id: project_id.trim().to_string(),
        runtime_enabled: runtime_enabled(),
        reports: store.list_task_sui_preflight_reports(project_id, 200)?,
        boundary: BOUNDARY.to_vec(),
    })
}

pub(super) fn record_report(
    store: &Store,
    adapter: &SuiPreflightAdapter,
    request: &RecordSuiPreflightReportRequest,
) -> Result<SuiPreflightReport> {
    if !runtime_enabled() {
        bail!("Sui 离线预检机器报告入口未启用");
    }
    let package_kind = normalize_option(
        &request.package_kind,
        &["standard", "correction"],
        "投影类型",
    )?;
    if !adapter
        .allowed_package_kinds
        .iter()
        .any(|value| value == package_kind)
    {
        bail!("该 Sui 预检适配器无权处理此投影类型");
    }
    let projection_package_id = bounded_text(&request.projection_package_id, "投影包 ID", 8, 160)?;
    let handoff_digest = normalized_digest(&request.handoff_digest, "交接包摘要")?;
    let outcome = normalize_option(&request.outcome, &["passed", "rejected"], "预检结果")?;
    let summary = bounded_text(&request.summary, "预检摘要", 4, 500)?;
    let tool_version = bounded_text(&request.tool_version, "预检工具版本", 1, 100)?;
    let idempotency_key = bounded_text(&request.idempotency_key, "幂等键", 8, 128)?;
    let bundle = read_only_bundle(
        store,
        &adapter.project_id,
        package_kind,
        &projection_package_id,
    )?;
    if bundle.handoff_digest != handoff_digest {
        bail!("交接包摘要与服务端当前可重新生成内容不一致");
    }
    if !adapter
        .allowed_networks
        .iter()
        .any(|value| value == &bundle.payload.target_network)
    {
        bail!("该 Sui 预检适配器无权处理交接包的目标网络");
    }
    let report_digest = report_digest(ReportDigestPayload {
        project_id: &adapter.project_id,
        adapter_id: &adapter.id,
        credential_version: adapter.credential_version,
        package_kind,
        projection_package_id: &projection_package_id,
        target_network: &bundle.payload.target_network,
        handoff_digest: &handoff_digest,
        projection_digest: &bundle.payload.projection_digest,
        outcome,
        summary: &summary,
        tool_version: &tool_version,
        idempotency_key: &idempotency_key,
    })?;
    let report = store.record_task_sui_preflight_report(CreateSuiPreflightReport {
        project_id: &adapter.project_id,
        adapter_id: &adapter.id,
        credential_version: adapter.credential_version,
        package_kind,
        projection_package_id: &projection_package_id,
        target_network: &bundle.payload.target_network,
        handoff_digest: &handoff_digest,
        projection_digest: &bundle.payload.projection_digest,
        outcome,
        summary: &summary,
        tool_version: &tool_version,
        idempotency_key: &idempotency_key,
        report_digest: &report_digest,
    })?;
    store.touch_task_sui_preflight_adapter(&adapter.id)?;
    Ok(report)
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

fn normalize_options(values: &[String], allowed: &[&str], label: &str) -> Result<Vec<String>> {
    let normalized = values
        .iter()
        .map(|value| normalize_option(value, allowed, label))
        .collect::<Result<BTreeSet<_>>>()?;
    if normalized.is_empty() {
        bail!("{label}至少需要选择一项");
    }
    Ok(normalized.into_iter().map(str::to_string).collect())
}

fn normalize_option<'a>(value: &str, allowed: &[&'a str], label: &str) -> Result<&'a str> {
    let value = value.trim().to_ascii_lowercase();
    allowed
        .iter()
        .copied()
        .find(|allowed| *allowed == value)
        .ok_or_else(|| anyhow::anyhow!("不支持的{label}"))
}

fn bounded_text(value: &str, label: &str, min: usize, max: usize) -> Result<String> {
    let value = value.trim();
    let length = value.chars().count();
    if !(min..=max).contains(&length) {
        bail!("{label}长度必须在 {min} 至 {max} 个字符之间");
    }
    Ok(value.to_string())
}

fn normalized_digest(value: &str, label: &str) -> Result<String> {
    let value = value.trim().to_ascii_lowercase();
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        bail!("{label}必须是 64 位十六进制 SHA-256");
    }
    Ok(value)
}

fn expiration_from_days(expires_in_days: i64) -> Result<String> {
    if !(1..=366).contains(&expires_in_days) {
        bail!("Sui 预检适配器凭据有效期必须在 1 至 366 天之间");
    }
    Ok((Utc::now() + Duration::days(expires_in_days)).to_rfc3339())
}

fn require_editor(role: &str) -> Result<()> {
    if !can_edit(role) {
        bail!("只有项目编辑者有权限管理 Sui 预检适配器");
    }
    Ok(())
}

#[derive(Serialize)]
struct ReportDigestPayload<'a> {
    project_id: &'a str,
    adapter_id: &'a str,
    credential_version: i64,
    package_kind: &'a str,
    projection_package_id: &'a str,
    target_network: &'a str,
    handoff_digest: &'a str,
    projection_digest: &'a str,
    outcome: &'a str,
    summary: &'a str,
    tool_version: &'a str,
    idempotency_key: &'a str,
}

fn report_digest(payload: ReportDigestPayload<'_>) -> Result<String> {
    Ok(hex::encode(Sha256::digest(serde_json::to_vec(&payload)?)))
}
