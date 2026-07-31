//! One service layer shared by HTTP, MCP and the PC workbench.

use anyhow::{anyhow, bail, Result};
use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::{
    open_commerce_integration_model::{
        CreateIntegrationRequest, OpenCommerceIntegration, OpenCommerceSyncReceipt,
        RecordSyncReceiptRequest, INTEGRATION_STATUS_CONNECTED, INTEGRATION_STATUS_DEGRADED,
    },
    open_commerce_model::{
        normalize_app_id, normalize_idempotency_key, CreateCapabilityRequest, CreateGrantRequest,
        CreateMerchantRequest, InvokeCapabilityRequest, OpenCommerceCapability,
        OpenCommerceInvocation, OpenCommerceMerchant, OpenCommerceMerchantDetail,
        OpenCommerceOverview, OpenCommerceTotals, UpdateCapabilityRequest, UpdateMerchantRequest,
        ACCESS_AUTHORIZED, ACCESS_OWNER_ONLY, CAPABILITY_STATUS_ACTIVE, HANDLER_MERCHANT_PROFILE,
        HANDLER_MERCHANT_RUNTIME, HANDLER_STATIC_JSON, MERCHANT_STATUS_ACTIVE,
        OPEN_COMMERCE_SCHEMA,
    },
    open_commerce_runtime_model::{MerchantRuntimeEnvelope, RUNTIME_STATUS_ACTIVE},
    project_auth::can_edit,
    store::{OpenCommerceInvocationStart, RecordOpenCommerceSyncReceipt, Store},
};

pub(crate) struct OpenCommerceActor<'a> {
    pub user_id: &'a str,
    pub app_id: &'a str,
    pub project_role: Option<&'a str>,
}

pub(crate) fn overview(store: &Store, project_id: &str) -> Result<OpenCommerceOverview> {
    let merchants = store.list_project_open_commerce_merchants(project_id)?;
    let grants = store.list_project_open_commerce_grants(project_id, 100)?;
    let recent_invocations = store.list_project_open_commerce_invocations(project_id, 100)?;
    let integrations = store.list_project_open_commerce_integrations(project_id)?;
    let runtime_bindings = store.list_project_open_commerce_runtime_bindings(project_id)?;
    let recent_sync_receipts = store.list_project_open_commerce_sync_receipts(project_id, 100)?;
    let recent_audit_events = store.list_project_open_commerce_audit(project_id, 100)?;
    let active_merchants = merchants
        .iter()
        .filter(|entry| entry.merchant.status == MERCHANT_STATUS_ACTIVE)
        .count();
    let capabilities = merchants.iter().map(|entry| entry.capabilities.len()).sum();
    let active_capabilities = merchants
        .iter()
        .flat_map(|entry| &entry.capabilities)
        .filter(|capability| capability.status == CAPABILITY_STATUS_ACTIVE)
        .count();
    let active_grants = grants.iter().filter(|grant| grant_is_active(grant)).count();
    let metered_amount_micros = recent_invocations
        .iter()
        .map(|invocation| invocation.amount_micros)
        .sum();
    let connected_integrations = integrations
        .iter()
        .filter(|integration| integration.status == INTEGRATION_STATUS_CONNECTED)
        .count();
    let degraded_integrations = integrations
        .iter()
        .filter(|integration| integration.status == INTEGRATION_STATUS_DEGRADED)
        .count();
    let active_runtime_bindings = runtime_bindings
        .iter()
        .filter(|binding| binding.status == RUNTIME_STATUS_ACTIVE)
        .count();
    Ok(OpenCommerceOverview {
        schema: OPEN_COMMERCE_SCHEMA,
        project_id: project_id.to_string(),
        totals: OpenCommerceTotals {
            merchants: merchants.len(),
            active_merchants,
            capabilities,
            active_capabilities,
            active_grants,
            invocations: recent_invocations.len(),
            integrations: integrations.len(),
            connected_integrations,
            degraded_integrations,
            active_runtime_bindings,
            sync_receipts: recent_sync_receipts.len(),
            metered_amount_micros,
        },
        merchants,
        grants,
        recent_invocations,
        integrations,
        runtime_bindings,
        recent_sync_receipts,
        recent_audit_events,
    })
}

pub(crate) fn development_context(store: &Store, project_id: &str) -> Result<Value> {
    let merchants = store.list_project_open_commerce_merchants(project_id)?;
    let integrations = store.list_project_open_commerce_integrations(project_id)?;
    let runtime_bindings = store.list_project_open_commerce_runtime_bindings(project_id)?;
    let receipts = store.list_project_open_commerce_sync_receipts(project_id, 50)?;
    let merchant_contexts = merchants
        .into_iter()
        .map(|mut detail| {
            for capability in &mut detail.capabilities {
                capability.handler_config = None;
            }
            let merchant_integrations = integrations
                .iter()
                .filter(|integration| integration.merchant_id == detail.merchant.id)
                .map(|integration| {
                    json!({
                        "id": integration.id,
                        "integration_key": integration.integration_key,
                        "provider_key": integration.provider_key,
                        "display_name": integration.display_name,
                        "connection_mode": integration.connection_mode,
                        "status": integration.status,
                        "scopes": integration.scopes,
                        "data_domains": integration.data_domains,
                        "last_verified_at": integration.last_verified_at,
                        "last_sync_at": integration.last_sync_at
                    })
                })
                .collect::<Vec<_>>();
            let runtime = runtime_bindings
                .iter()
                .find(|binding| binding.merchant_id == detail.merchant.id)
                .map(|binding| {
                    json!({
                        "status":binding.status,
                        "manifest_sha256":binding.manifest_sha256,
                        "last_verified_at":binding.last_verified_at,
                        "last_error_code":binding.last_error_code
                    })
                });
            json!({
                "merchant": {
                    "id": detail.merchant.id,
                    "display_name": detail.merchant.display_name,
                    "status": detail.merchant.status,
                    "node_mode": detail.merchant.node_mode
                },
                "capabilities": detail.capabilities,
                "integrations": merchant_integrations,
                "runtime": runtime
            })
        })
        .collect::<Vec<_>>();
    Ok(json!({
        "schema": "open_commerce.development_context.v1",
        "project_id": project_id,
        "generated_at": Utc::now().to_rfc3339(),
        "summary": {
            "merchants": merchant_contexts.len(),
            "integrations": integrations.len(),
            "connected_integrations": integrations.iter()
                .filter(|entry| entry.status == INTEGRATION_STATUS_CONNECTED).count(),
            "degraded_integrations": integrations.iter()
                .filter(|entry| entry.status == INTEGRATION_STATUS_DEGRADED).count(),
            "recent_sync_receipts": receipts.len(),
            "active_runtime_bindings": runtime_bindings.iter()
                .filter(|entry| entry.status == RUNTIME_STATUS_ACTIVE).count()
        },
        "merchants": merchant_contexts,
        "recent_sync_evidence": receipts.into_iter().map(|receipt| json!({
            "integration_id": receipt.integration_id,
            "sync_kind": receipt.sync_kind,
            "status": receipt.status,
            "records_seen": receipt.records_seen,
            "records_changed": receipt.records_changed,
            "error_code": receipt.error_code,
            "started_at": receipt.started_at,
            "completed_at": receipt.completed_at
        })).collect::<Vec<_>>(),
        "implementation_constraints": [
            "接入状态和同步回执只证明连接健康度，不代表已取得未授权的平台数据。",
            "不得把 scopes、data_domains 或字段结构误当成原始经营数据。",
            "不得在源码、Matter、Assignment 或文档中写入访问令牌和平台密钥。",
            "新增适配器必须复用 Merchant、Capability、Grant、Invocation 和 Audit 主干。",
            "任何自动发布、下单、财务或库存写操作都必须经过最小权限与人工确认策略。",
            "merchant_runtime 只有签名健康验证通过后才可调用；开发上下文不得返回运行地址或密钥引用。"
        ]
    }))
}

pub(crate) fn create_integration(
    store: &Store,
    project_id: &str,
    actor: &OpenCommerceActor<'_>,
    request: CreateIntegrationRequest,
) -> Result<OpenCommerceIntegration> {
    require_editor(actor.project_role)?;
    let integration = store.create_open_commerce_integration(project_id, actor.user_id, request)?;
    store.record_open_commerce_audit(
        project_id,
        actor.user_id,
        Some(actor.app_id),
        "integration.created",
        "integration",
        &integration.id,
        &json!({
            "merchant_id": integration.merchant_id,
            "integration_key": integration.integration_key,
            "provider_key": integration.provider_key,
            "connection_mode": integration.connection_mode,
            "data_domains": integration.data_domains
        }),
    )?;
    Ok(integration)
}

pub(crate) fn set_integration_enabled(
    store: &Store,
    project_id: &str,
    integration_id: &str,
    actor: &OpenCommerceActor<'_>,
    enabled: bool,
) -> Result<OpenCommerceIntegration> {
    require_editor(actor.project_role)?;
    let integration =
        store.set_open_commerce_integration_enabled(project_id, integration_id, enabled)?;
    store.record_open_commerce_audit(
        project_id,
        actor.user_id,
        Some(actor.app_id),
        "integration.enabled_changed",
        "integration",
        &integration.id,
        &json!({"enabled": enabled, "status": integration.status}),
    )?;
    Ok(integration)
}

pub(crate) fn record_sync_receipt(
    store: &Store,
    project_id: &str,
    actor: &OpenCommerceActor<'_>,
    request: RecordSyncReceiptRequest,
) -> Result<OpenCommerceSyncReceipt> {
    require_editor(actor.project_role)?;
    let receipt = store.record_open_commerce_sync_receipt(RecordOpenCommerceSyncReceipt {
        project_id,
        actor_user_id: actor.user_id,
        actor_app_id: actor.app_id,
        request,
    })?;
    store.record_open_commerce_audit(
        project_id,
        actor.user_id,
        Some(actor.app_id),
        "integration.sync_recorded",
        "sync_receipt",
        &receipt.id,
        &json!({
            "integration_id": receipt.integration_id,
            "receipt_key": receipt.receipt_key,
            "sync_kind": receipt.sync_kind,
            "status": receipt.status,
            "records_seen": receipt.records_seen,
            "records_changed": receipt.records_changed,
            "error_code": receipt.error_code
        }),
    )?;
    Ok(receipt)
}

pub(crate) fn discover_merchants(
    store: &Store,
    query: Option<&str>,
    capability_key: Option<&str>,
    limit: usize,
) -> Result<Vec<OpenCommerceMerchantDetail>> {
    store.search_open_commerce_merchants(query, capability_key, limit)
}

pub(crate) fn discover_merchant(
    store: &Store,
    merchant_id: &str,
) -> Result<OpenCommerceMerchantDetail> {
    let mut detail = store.open_commerce_merchant_detail(merchant_id)?;
    if detail.merchant.status != MERCHANT_STATUS_ACTIVE {
        bail!("商户节点未发布");
    }
    detail
        .capabilities
        .retain(|capability| capability.status == CAPABILITY_STATUS_ACTIVE);
    for capability in &mut detail.capabilities {
        capability.handler_config = None;
    }
    Ok(detail)
}

pub(crate) fn create_merchant(
    store: &Store,
    project_id: &str,
    actor: &OpenCommerceActor<'_>,
    request: CreateMerchantRequest,
) -> Result<OpenCommerceMerchant> {
    require_editor(actor.project_role)?;
    let merchant = store.create_open_commerce_merchant(project_id, actor.user_id, request)?;
    store.record_open_commerce_audit(
        project_id,
        actor.user_id,
        Some(actor.app_id),
        "merchant.created",
        "merchant",
        &merchant.id,
        &json!({"slug": merchant.slug, "status": merchant.status}),
    )?;
    Ok(merchant)
}

pub(crate) fn update_merchant(
    store: &Store,
    project_id: &str,
    merchant_id: &str,
    actor: &OpenCommerceActor<'_>,
    request: UpdateMerchantRequest,
) -> Result<OpenCommerceMerchant> {
    require_editor(actor.project_role)?;
    let merchant = store.update_open_commerce_merchant(project_id, merchant_id, request)?;
    store.record_open_commerce_audit(
        project_id,
        actor.user_id,
        Some(actor.app_id),
        "merchant.updated",
        "merchant",
        &merchant.id,
        &json!({"status": merchant.status, "node_mode": merchant.node_mode}),
    )?;
    Ok(merchant)
}

pub(crate) fn publish_capability(
    store: &Store,
    project_id: &str,
    merchant_id: &str,
    actor: &OpenCommerceActor<'_>,
    request: CreateCapabilityRequest,
) -> Result<OpenCommerceCapability> {
    require_editor(actor.project_role)?;
    let capability = store.create_open_commerce_capability(project_id, merchant_id, request)?;
    store.record_open_commerce_audit(
        project_id,
        actor.user_id,
        Some(actor.app_id),
        "capability.published",
        "capability",
        &capability.id,
        &json!({
            "merchant_id": merchant_id,
            "capability_key": capability.capability_key,
            "access_level": capability.access_level,
            "handler_type": capability.handler_type
        }),
    )?;
    Ok(capability)
}

pub(crate) fn update_capability(
    store: &Store,
    project_id: &str,
    capability_id: &str,
    actor: &OpenCommerceActor<'_>,
    request: UpdateCapabilityRequest,
) -> Result<OpenCommerceCapability> {
    require_editor(actor.project_role)?;
    let capability = store.update_open_commerce_capability(project_id, capability_id, request)?;
    store.record_open_commerce_audit(
        project_id,
        actor.user_id,
        Some(actor.app_id),
        "capability.updated",
        "capability",
        &capability.id,
        &json!({
            "capability_key": capability.capability_key,
            "access_level": capability.access_level,
            "status": capability.status,
            "version": capability.version
        }),
    )?;
    Ok(capability)
}

pub(crate) fn create_grant(
    store: &Store,
    project_id: &str,
    actor: &OpenCommerceActor<'_>,
    request: CreateGrantRequest,
) -> Result<crate::open_commerce_model::OpenCommerceGrant> {
    require_editor(actor.project_role)?;
    let grant = store.create_open_commerce_grant(project_id, actor.user_id, request)?;
    store.record_open_commerce_audit(
        project_id,
        actor.user_id,
        Some(actor.app_id),
        "grant.created",
        "grant",
        &grant.id,
        &json!({
            "merchant_id": grant.merchant_id,
            "grantee_app_id": grant.grantee_app_id,
            "scopes": grant.scopes,
            "expires_at": grant.expires_at
        }),
    )?;
    Ok(grant)
}

pub(crate) fn revoke_grant(
    store: &Store,
    project_id: &str,
    grant_id: &str,
    actor: &OpenCommerceActor<'_>,
) -> Result<crate::open_commerce_model::OpenCommerceGrant> {
    require_editor(actor.project_role)?;
    let grant = store.revoke_open_commerce_grant(project_id, grant_id)?;
    store.record_open_commerce_audit(
        project_id,
        actor.user_id,
        Some(actor.app_id),
        "grant.revoked",
        "grant",
        &grant.id,
        &json!({
            "merchant_id": grant.merchant_id,
            "grantee_app_id": grant.grantee_app_id,
            "revoked_at": grant.revoked_at
        }),
    )?;
    Ok(grant)
}

pub(crate) async fn invoke(
    store: &Store,
    actor: &OpenCommerceActor<'_>,
    request: InvokeCapabilityRequest,
) -> Result<Value> {
    let requester_app_id = normalize_app_id(&request.requester_app_id)?;
    if requester_app_id != normalize_app_id(actor.app_id)? {
        bail!("requester_app_id 与当前调用入口不一致");
    }
    let idempotency_key = normalize_idempotency_key(&request.idempotency_key)?;
    let input = crate::open_commerce_model::validate_json_object(&request.input, "调用输入")?;
    let merchant = store.open_commerce_merchant(&request.merchant_id)?;
    let capability =
        store.open_commerce_capability_by_key(&merchant.id, &request.capability_key)?;
    if merchant.status != MERCHANT_STATUS_ACTIVE {
        bail!("商户节点当前不可用");
    }
    if capability.status != CAPABILITY_STATUS_ACTIVE {
        bail!("商业能力当前不可用");
    }
    let grant_id = authorize_invocation(
        store,
        actor,
        &merchant,
        &capability,
        request.grant_id.as_deref(),
    )?;
    let request_hash = request_digest(
        &merchant.id,
        &capability.capability_key,
        &requester_app_id,
        &input,
    )?;
    let request_shape = request_shape(&input)?;
    let claim = store.start_open_commerce_invocation(OpenCommerceInvocationStart {
        project_id: &merchant.project_id,
        merchant_id: &merchant.id,
        capability_id: &capability.id,
        capability_key: &capability.capability_key,
        requester_user_id: actor.user_id,
        requester_app_id: &requester_app_id,
        grant_id: grant_id.as_deref(),
        idempotency_key: &idempotency_key,
        request_hash: &request_hash,
        request_shape: &request_shape,
        unit_price_micros: capability.unit_price_micros,
        currency: &capability.currency,
    })?;
    if !claim.created {
        return invocation_response(&claim.invocation, true);
    }

    let result = match execute_handler(
        store,
        actor,
        &merchant,
        &capability,
        grant_id.as_deref(),
        &idempotency_key,
        &claim.invocation.id,
        &input,
    )
    .await
    {
        Ok(result) => result,
        Err(error) => {
            let failed = store
                .finish_open_commerce_invocation_failure(&claim.invocation.id, "handler_failed")?;
            store.record_open_commerce_audit(
                &merchant.project_id,
                actor.user_id,
                Some(&requester_app_id),
                "invocation.failed",
                "invocation",
                &failed.id,
                &json!({
                    "merchant_id": merchant.id,
                    "capability_key": capability.capability_key,
                    "error_code": "handler_failed"
                }),
            )?;
            return Err(error);
        }
    };
    let invocation =
        store.finish_open_commerce_invocation_success(&claim.invocation.id, &result)?;
    store.record_open_commerce_audit(
        &merchant.project_id,
        actor.user_id,
        Some(&requester_app_id),
        "invocation.succeeded",
        "invocation",
        &invocation.id,
        &json!({
            "merchant_id": merchant.id,
            "capability_key": capability.capability_key,
            "grant_id": grant_id,
            "amount_micros": invocation.amount_micros,
            "settlement_status": invocation.settlement_status
        }),
    )?;
    if let Err(error) = crate::task_settlement::capture_commerce_invocation(
        store,
        &invocation,
        &merchant.owner_user_id,
    ) {
        tracing::warn!(
            project_id = merchant.project_id,
            invocation_id = invocation.id,
            error = %error,
            "failed to capture optional open-commerce shadow usage"
        );
    }
    invocation_response(&invocation, false)
}

async fn execute_handler(
    store: &Store,
    actor: &OpenCommerceActor<'_>,
    merchant: &OpenCommerceMerchant,
    capability: &OpenCommerceCapability,
    grant_id: Option<&str>,
    idempotency_key: &str,
    invocation_id: &str,
    input: &Value,
) -> Result<Value> {
    if capability.handler_type != HANDLER_MERCHANT_RUNTIME {
        return execute_first_party_handler(merchant, capability, input);
    }
    let binding = store.active_open_commerce_runtime_binding(&merchant.id)?;
    let envelope = MerchantRuntimeEnvelope {
        schema: "merchant_runtime.invoke.v1",
        invocation_id: invocation_id.to_string(),
        merchant_id: merchant.id.clone(),
        capability_key: capability.capability_key.clone(),
        requester_user_id: actor.user_id.to_string(),
        requester_app_id: actor.app_id.to_string(),
        grant_id: grant_id.map(str::to_string),
        idempotency_key: idempotency_key.to_string(),
        issued_at_unix: Utc::now().timestamp(),
        input: input.clone(),
    };
    match crate::open_commerce_runtime_client::invoke_runtime(&binding, &envelope).await {
        Ok(result) => Ok(result),
        Err(error) => {
            if error.degrades_binding() {
                let _ = store
                    .mark_open_commerce_runtime_degraded(&merchant.id, "runtime_invocation_failed");
            }
            Err(error.into())
        }
    }
}

fn authorize_invocation(
    store: &Store,
    actor: &OpenCommerceActor<'_>,
    merchant: &OpenCommerceMerchant,
    capability: &OpenCommerceCapability,
    grant_id: Option<&str>,
) -> Result<Option<String>> {
    match capability.access_level.as_str() {
        ACCESS_AUTHORIZED => {
            let grant_id = grant_id.ok_or_else(|| anyhow!("该能力需要有效授权"))?;
            let grant = store.active_open_commerce_grant(
                grant_id,
                &merchant.id,
                actor.app_id,
                &capability.capability_key,
            )?;
            Ok(Some(grant.id))
        }
        ACCESS_OWNER_ONLY => {
            require_editor(actor.project_role)?;
            Ok(None)
        }
        "public" => Ok(None),
        _ => bail!("商业能力访问级别无效"),
    }
}

fn execute_first_party_handler(
    merchant: &OpenCommerceMerchant,
    capability: &OpenCommerceCapability,
    _input: &Value,
) -> Result<Value> {
    match capability.handler_type.as_str() {
        HANDLER_MERCHANT_PROFILE => Ok(json!({
            "merchant_id": merchant.id,
            "slug": merchant.slug,
            "display_name": merchant.display_name,
            "description": merchant.description,
            "public_profile": merchant.public_profile,
            "updated_at": merchant.updated_at
        })),
        HANDLER_STATIC_JSON => capability
            .handler_config
            .as_ref()
            .map(|config| config.get("response").unwrap_or(config).clone())
            .ok_or_else(|| anyhow!("静态处理器缺少配置")),
        _ => bail!("未知或未审核的处理器"),
    }
}

fn invocation_response(invocation: &OpenCommerceInvocation, replayed: bool) -> Result<Value> {
    Ok(json!({
        "schema": "open_commerce.invocation.v1",
        "invocation_id": invocation.id,
        "status": invocation.status,
        "replayed": replayed,
        "result": invocation.result,
        "error_code": invocation.error_code,
        "metering": {
            "units": invocation.units,
            "unit_price_micros": invocation.unit_price_micros,
            "amount_micros": invocation.amount_micros,
            "currency": invocation.currency,
            "settlement_status": invocation.settlement_status
        },
        "settlement_receipt": {
            "schema": "open_commerce.settlement_receipt.v1",
            "receipt_id": invocation.id,
            "billable_units": invocation.units,
            "amount_micros": invocation.amount_micros,
            "currency": invocation.currency,
            "status": invocation.settlement_status,
            "funds_moved": false
        }
    }))
}

fn request_digest(
    merchant_id: &str,
    capability_key: &str,
    requester_app_id: &str,
    input: &Value,
) -> Result<String> {
    let bytes = serde_json::to_vec(&json!({
        "merchant_id": merchant_id,
        "capability_key": capability_key,
        "requester_app_id": requester_app_id,
        "input": input
    }))?;
    Ok(hex::encode(Sha256::digest(bytes)))
}

fn request_shape(input: &Value) -> Result<Value> {
    let fields = input
        .as_object()
        .ok_or_else(|| anyhow!("调用输入必须是 JSON object"))?
        .keys()
        .cloned()
        .collect::<Vec<_>>();
    Ok(json!({
        "input_fields": fields,
        "input_bytes": serde_json::to_vec(input)?.len(),
        "contains_raw_values": false
    }))
}

fn require_editor(role: Option<&str>) -> Result<()> {
    if role.is_some_and(can_edit) {
        Ok(())
    } else {
        bail!("当前调用方没有项目编辑权限")
    }
}

fn grant_is_active(grant: &crate::open_commerce_model::OpenCommerceGrant) -> bool {
    if grant.revoked_at.is_some() {
        return false;
    }
    grant.expires_at.as_deref().is_none_or(|expires_at| {
        DateTime::parse_from_rfc3339(expires_at)
            .map(|value| value.with_timezone(&Utc) > Utc::now())
            .unwrap_or(false)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::open_commerce_model::OpenCommerceCapability;

    #[test]
    fn request_shape_never_contains_values() {
        let shape = request_shape(&json!({"phone": "secret", "count": 2})).unwrap();
        assert_eq!(shape["contains_raw_values"], false);
        assert_eq!(shape["input_fields"], json!(["count", "phone"]));
        assert!(!shape.to_string().contains("secret"));
    }

    #[test]
    fn static_handler_returns_only_configured_response() {
        let merchant = sample_merchant();
        let capability = OpenCommerceCapability {
            id: "cap_1".into(),
            merchant_id: merchant.id.clone(),
            capability_key: "booking.preview".into(),
            display_name: "预约预览".into(),
            description: String::new(),
            kind: "query".into(),
            access_level: "public".into(),
            input_schema: json!({}),
            output_schema: json!({}),
            handler_type: HANDLER_STATIC_JSON.into(),
            handler_config: Some(json!({"response": {"available": true}})),
            unit_price_micros: 0,
            currency: "CNY".into(),
            freshness_seconds: 0,
            status: "active".into(),
            version: 1,
            created_at: String::new(),
            updated_at: String::new(),
        };
        let result = execute_first_party_handler(&merchant, &capability, &json!({})).unwrap();
        assert_eq!(result, json!({"available": true}));
    }

    fn sample_merchant() -> OpenCommerceMerchant {
        OpenCommerceMerchant {
            id: "merchant_1".into(),
            project_id: "project_1".into(),
            owner_user_id: "user_1".into(),
            slug: "demo-store".into(),
            display_name: "演示商户".into(),
            description: String::new(),
            status: "active".into(),
            node_mode: "platform_hosted".into(),
            public_profile: json!({}),
            created_at: String::new(),
            updated_at: String::new(),
        }
    }
}
