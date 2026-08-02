//! Streamable HTTP MCP adapter for project-scoped open-commerce operations.

use anyhow::{anyhow, Context, Result};
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;

use crate::{
    open_commerce_action_confirmation_model::ACTION_CONFIRMATION_PHRASE,
    open_commerce_action_confirmation_service,
    open_commerce_app_block_model::BlockOpenCommerceAppRequest,
    open_commerce_app_block_service,
    open_commerce_data_request_model::{
        CreateConsumerDataErasureRequest, DecideConsumerDataRequest,
    },
    open_commerce_data_request_service, open_commerce_directory_service,
    open_commerce_integration_model::{CreateIntegrationRequest, RecordSyncReceiptRequest},
    open_commerce_model::{
        normalize_app_id, CreateCapabilityRequest, CreateGrantRequest, CreateMerchantRequest,
        InvokeCapabilityRequest,
    },
    open_commerce_portability_model::CreateConsumerPortabilityExportRequest,
    open_commerce_portability_service,
    open_commerce_rate_limit_model::{
        SetOpenCommerceRateLimitEnabledRequest, UpsertOpenCommerceRateLimitRequest,
    },
    open_commerce_rate_limit_service,
    open_commerce_relationship_model::{
        CreateConsumerRelationshipRequest, RenewConsumerRelationshipRequest,
    },
    open_commerce_relationship_service,
    open_commerce_runtime_model::UpsertRuntimeBindingRequest,
    open_commerce_runtime_service,
    open_commerce_service::{self, OpenCommerceActor},
    project_auth::{auth_from_headers, json_error, project_access},
    types::AppState,
};

const MCP_PROTOCOL_VERSION: &str = "2025-03-26";
const DEFAULT_MCP_APP_ID: &str = "mcp-client";

#[derive(Debug, Deserialize)]
struct McpRequest {
    #[allow(dead_code)]
    jsonrpc: Option<String>,
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Value,
}

#[derive(Debug, Deserialize)]
struct SearchArguments {
    #[serde(default)]
    query: Option<String>,
    #[serde(default)]
    capability: Option<String>,
    #[serde(default = "default_search_limit")]
    limit: usize,
}

#[derive(Debug, Deserialize)]
struct MerchantArguments {
    merchant_id: String,
}

#[derive(Debug, Deserialize)]
struct DirectoryPublicationArguments {
    merchant_id: String,
    published: bool,
}

#[derive(Debug, Deserialize)]
struct PublishCapabilityArguments {
    merchant_id: String,
    #[serde(flatten)]
    request: CreateCapabilityRequest,
}

#[derive(Debug, Deserialize)]
struct RuntimeBindingArguments {
    merchant_id: String,
    #[serde(flatten)]
    request: UpsertRuntimeBindingRequest,
}

#[derive(Debug, Deserialize)]
struct RevokeGrantArguments {
    grant_id: String,
}

#[derive(Debug, Deserialize)]
struct CreateConsumerRelationshipArguments {
    merchant_id: String,
    scopes: Vec<String>,
    purpose: String,
    expires_at: String,
}

#[derive(Debug, Deserialize)]
struct RevokeConsumerRelationshipArguments {
    relationship_id: String,
}

#[derive(Debug, Deserialize)]
struct RenewConsumerRelationshipArguments {
    relationship_id: String,
    expires_at: String,
}

#[derive(Debug, Deserialize)]
struct ConsumerDataRequestArguments {
    request_id: String,
}

#[derive(Debug, Deserialize)]
struct CreateConsumerPortabilityExportArguments {
    idempotency_key: String,
}

#[derive(Debug, Deserialize)]
struct ConsumerPortabilityExportArguments {
    export_id: String,
}

#[derive(Debug, Deserialize)]
struct CreateConsumerDataErasureArguments {
    relationship_id: String,
}

#[derive(Debug, Deserialize)]
struct DecideConsumerDataRequestArguments {
    merchant_id: String,
    request_id: String,
    action: String,
    #[serde(default)]
    note: String,
}

#[derive(Debug, Deserialize)]
struct InvokeArguments {
    merchant_id: String,
    capability_key: String,
    #[serde(default)]
    grant_id: Option<String>,
    idempotency_key: String,
    #[serde(default)]
    action_confirmation_id: Option<String>,
    #[serde(default = "empty_object")]
    input: Value,
}

#[derive(Debug, Deserialize)]
struct ConfirmActionArguments {
    confirmation_id: String,
    confirmation_phrase: String,
}

#[derive(Debug, Deserialize)]
struct AuditArguments {
    #[serde(default = "default_audit_limit")]
    limit: usize,
}

#[derive(Debug, Deserialize)]
struct SetIntegrationEnabledArguments {
    integration_id: String,
    enabled: bool,
}

#[derive(Debug, Deserialize)]
struct SetRateLimitEnabledArguments {
    policy_id: String,
    enabled: bool,
}

#[derive(Debug, Deserialize)]
struct UnblockAppArguments {
    block_id: String,
}

struct McpCaller {
    user_id: String,
    project_role: String,
    app_id: String,
}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route(
        "/api/projects/:project_id/open-commerce/mcp",
        post(mcp_handler),
    )
}

async fn mcp_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(request): Json<McpRequest>,
) -> Response {
    let caller = match authenticate(&state, &headers, &project_id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let id = request.id.clone().unwrap_or(Value::Null);
    let result = match request.method.as_str() {
        "initialize" => Ok(initialize_response()),
        "notifications/initialized" => return StatusCode::ACCEPTED.into_response(),
        "tools/list" => {
            let mut tools = crate::open_commerce_mcp_tools::definitions();
            tools.extend(crate::open_commerce_consumer_preference_mcp::definitions());
            tools.extend(crate::open_commerce_consumer_receipt_mcp::definitions());
            tools.extend(crate::open_commerce_merchant_evidence_mcp::definitions());
            tools.extend(crate::open_commerce_business_handoff_mcp::definitions());
            tools.extend(crate::erp_blueprint_mcp_tools::definitions());
            Ok(json!({"tools": tools}))
        }
        "tools/call" => {
            call_tool(
                &state.store,
                &project_id,
                &caller.user_id,
                &caller.project_role,
                &caller.app_id,
                request.params,
            )
            .await
        }
        "ping" => Ok(json!({})),
        _ => Err(anyhow!("不支持 MCP method: {}", request.method)),
    };
    Json(match result {
        Ok(value) => json!({"jsonrpc":"2.0","id":id,"result":value}),
        Err(error) => json!({
            "jsonrpc":"2.0",
            "id":id,
            "error":{"code":-32000,"message":format!("{error:#}")}
        }),
    })
    .into_response()
}

pub(crate) async fn call_tool(
    store: &crate::store::Store,
    project_id: &str,
    user_id: &str,
    project_role: &str,
    app_id: &str,
    params: Value,
) -> Result<Value> {
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("tools/call 缺少 name"))?;
    let arguments = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));
    if crate::erp_blueprint_mcp::handles(name) {
        let value = crate::erp_blueprint_mcp::call_tool(
            store,
            project_id,
            user_id,
            project_role,
            name,
            arguments,
        )?;
        return tool_response(value);
    }
    if let Some(value) = crate::open_commerce_consumer_preference_mcp::call_if_handled(
        store,
        project_id,
        user_id,
        project_role,
        app_id,
        name,
        arguments.clone(),
    )? {
        return tool_response(value);
    }
    if let Some(value) = crate::open_commerce_consumer_receipt_mcp::call_if_handled(
        store,
        user_id,
        name,
        arguments.clone(),
    )? {
        return tool_response(value);
    }
    if let Some(value) = crate::open_commerce_merchant_evidence_mcp::call_if_handled(
        store,
        project_id,
        name,
        arguments.clone(),
    )? {
        return tool_response(value);
    }
    if let Some(value) = crate::open_commerce_business_handoff_mcp::call_if_handled(
        store,
        project_id,
        user_id,
        project_role,
        app_id,
        name,
        arguments.clone(),
    )? {
        return tool_response(value);
    }
    let actor = OpenCommerceActor {
        user_id,
        app_id,
        project_role: Some(project_role),
    };
    let value = match name {
        "open_commerce_get_overview" => {
            ensure_empty_object(&arguments, name)?;
            serde_json::to_value(open_commerce_service::overview(store, project_id)?)?
        }
        "open_commerce_get_development_context" => {
            ensure_empty_object(&arguments, name)?;
            open_commerce_service::development_context(store, project_id)?
        }
        "open_commerce_search_merchants" => {
            let input: SearchArguments = decode(arguments, name)?;
            json!({
                "schema":"open_commerce.discovery.v1",
                "merchants":open_commerce_directory_service::discover_merchants(
                    store,
                    input.query.as_deref(),
                    input.capability.as_deref(),
                    input.limit,
                )?
            })
        }
        "open_commerce_get_merchant" => {
            let input: MerchantArguments = decode(arguments, name)?;
            serde_json::to_value(open_commerce_directory_service::discover_merchant(
                store,
                &input.merchant_id,
            )?)?
        }
        "open_commerce_list_consumer_relationships" => {
            ensure_empty_object(&arguments, name)?;
            serde_json::to_value(
                open_commerce_relationship_service::list_consumer_relationships(
                    store, project_id, &actor, 100,
                )?,
            )?
        }
        "open_commerce_list_merchant_relationships" => {
            let input: MerchantArguments = decode(arguments, name)?;
            serde_json::to_value(
                open_commerce_relationship_service::list_merchant_relationships(
                    store,
                    project_id,
                    &input.merchant_id,
                    &actor,
                    100,
                )?,
            )?
        }
        "open_commerce_list_consumer_data_requests" => {
            ensure_empty_object(&arguments, name)?;
            serde_json::to_value(open_commerce_data_request_service::list_consumer_requests(
                store, project_id, &actor, 100,
            )?)?
        }
        "open_commerce_list_consumer_portability_exports" => {
            ensure_empty_object(&arguments, name)?;
            serde_json::to_value(open_commerce_portability_service::list_exports(
                store, project_id, &actor, 100,
            )?)?
        }
        "open_commerce_get_consumer_portability_export" => {
            let input: ConsumerPortabilityExportArguments = decode(arguments, name)?;
            serde_json::to_value(open_commerce_portability_service::get_export(
                store,
                project_id,
                &input.export_id,
                &actor,
            )?)?
        }
        "open_commerce_list_merchant_data_requests" => {
            let input: MerchantArguments = decode(arguments, name)?;
            serde_json::to_value(open_commerce_data_request_service::list_merchant_requests(
                store,
                project_id,
                &input.merchant_id,
                &actor,
                100,
            )?)?
        }
        "open_commerce_create_merchant" => {
            let input: CreateMerchantRequest = decode(arguments, name)?;
            serde_json::to_value(open_commerce_service::create_merchant(
                store, project_id, &actor, input,
            )?)?
        }
        "open_commerce_publish_capability" => {
            let input: PublishCapabilityArguments = decode(arguments, name)?;
            serde_json::to_value(open_commerce_service::publish_capability(
                store,
                project_id,
                &input.merchant_id,
                &actor,
                input.request,
            )?)?
        }
        "open_commerce_set_directory_publication" => {
            let input: DirectoryPublicationArguments = decode(arguments, name)?;
            serde_json::to_value(open_commerce_directory_service::set_publication(
                store,
                project_id,
                &input.merchant_id,
                &actor,
                input.published,
            )?)?
        }
        "open_commerce_create_consumer_relationship" => {
            let input: CreateConsumerRelationshipArguments = decode(arguments, name)?;
            serde_json::to_value(open_commerce_relationship_service::create_relationship(
                store,
                project_id,
                &actor,
                CreateConsumerRelationshipRequest {
                    merchant_id: input.merchant_id,
                    source_app_id: app_id.to_string(),
                    scopes: input.scopes,
                    purpose: input.purpose,
                    expires_at: input.expires_at,
                },
            )?)?
        }
        "open_commerce_revoke_consumer_relationship" => {
            let input: RevokeConsumerRelationshipArguments = decode(arguments, name)?;
            serde_json::to_value(open_commerce_relationship_service::revoke_relationship(
                store,
                project_id,
                &input.relationship_id,
                &actor,
            )?)?
        }
        "open_commerce_renew_consumer_relationship" => {
            let input: RenewConsumerRelationshipArguments = decode(arguments, name)?;
            serde_json::to_value(open_commerce_relationship_service::renew_relationship(
                store,
                project_id,
                &input.relationship_id,
                &actor,
                RenewConsumerRelationshipRequest {
                    source_app_id: app_id.to_string(),
                    expires_at: input.expires_at,
                },
            )?)?
        }
        "open_commerce_create_consumer_data_erasure_request" => {
            let input: CreateConsumerDataErasureArguments = decode(arguments, name)?;
            serde_json::to_value(open_commerce_data_request_service::create_erasure_request(
                store,
                project_id,
                &actor,
                CreateConsumerDataErasureRequest {
                    relationship_id: input.relationship_id,
                },
            )?)?
        }
        "open_commerce_create_consumer_portability_export" => {
            let input: CreateConsumerPortabilityExportArguments = decode(arguments, name)?;
            serde_json::to_value(open_commerce_portability_service::create_export(
                store,
                project_id,
                &actor,
                CreateConsumerPortabilityExportRequest {
                    idempotency_key: input.idempotency_key,
                },
            )?)?
        }
        "open_commerce_withdraw_consumer_data_request" => {
            let input: ConsumerDataRequestArguments = decode(arguments, name)?;
            serde_json::to_value(open_commerce_data_request_service::withdraw_request(
                store,
                project_id,
                &input.request_id,
                &actor,
            )?)?
        }
        "open_commerce_decide_consumer_data_request" => {
            let input: DecideConsumerDataRequestArguments = decode(arguments, name)?;
            serde_json::to_value(open_commerce_data_request_service::decide_request(
                store,
                project_id,
                &input.merchant_id,
                &input.request_id,
                &actor,
                DecideConsumerDataRequest {
                    action: input.action,
                    note: input.note,
                },
            )?)?
        }
        "open_commerce_upsert_runtime" => {
            let input: RuntimeBindingArguments = decode(arguments, name)?;
            serde_json::to_value(open_commerce_runtime_service::upsert_binding(
                store,
                project_id,
                &input.merchant_id,
                &actor,
                input.request,
            )?)?
        }
        "open_commerce_verify_runtime" => {
            let input: MerchantArguments = decode(arguments, name)?;
            serde_json::to_value(
                open_commerce_runtime_service::verify_binding(
                    store,
                    project_id,
                    &input.merchant_id,
                    &actor,
                )
                .await?,
            )?
        }
        "open_commerce_create_grant" => {
            let input: CreateGrantRequest = decode(arguments, name)?;
            serde_json::to_value(open_commerce_service::create_grant(
                store, project_id, &actor, input,
            )?)?
        }
        "open_commerce_upsert_rate_limit" => {
            let input: UpsertOpenCommerceRateLimitRequest = decode(arguments, name)?;
            serde_json::to_value(open_commerce_rate_limit_service::upsert_policy(
                store,
                project_id,
                user_id,
                app_id,
                project_role,
                input,
            )?)?
        }
        "open_commerce_set_rate_limit_enabled" => {
            let input: SetRateLimitEnabledArguments = decode(arguments, name)?;
            serde_json::to_value(open_commerce_rate_limit_service::set_policy_enabled(
                store,
                project_id,
                &input.policy_id,
                user_id,
                app_id,
                project_role,
                SetOpenCommerceRateLimitEnabledRequest {
                    enabled: input.enabled,
                },
            )?)?
        }
        "open_commerce_list_app_blocks" => serde_json::to_value(
            open_commerce_app_block_service::list_blocks(store, project_id)?,
        )?,
        "open_commerce_block_app" => {
            let input: BlockOpenCommerceAppRequest = decode(arguments, name)?;
            serde_json::to_value(open_commerce_app_block_service::block_app(
                store,
                project_id,
                user_id,
                app_id,
                project_role,
                input,
            )?)?
        }
        "open_commerce_unblock_app" => {
            let input: UnblockAppArguments = decode(arguments, name)?;
            serde_json::to_value(open_commerce_app_block_service::unblock_app(
                store,
                project_id,
                &input.block_id,
                user_id,
                app_id,
                project_role,
            )?)?
        }
        "open_commerce_create_integration" => {
            let input: CreateIntegrationRequest = decode(arguments, name)?;
            serde_json::to_value(open_commerce_service::create_integration(
                store, project_id, &actor, input,
            )?)?
        }
        "open_commerce_set_integration_enabled" => {
            let input: SetIntegrationEnabledArguments = decode(arguments, name)?;
            serde_json::to_value(open_commerce_service::set_integration_enabled(
                store,
                project_id,
                &input.integration_id,
                &actor,
                input.enabled,
            )?)?
        }
        "open_commerce_record_sync_receipt" => {
            let input: RecordSyncReceiptRequest = decode(arguments, name)?;
            serde_json::to_value(open_commerce_service::record_sync_receipt(
                store, project_id, &actor, input,
            )?)?
        }
        "open_commerce_revoke_grant" => {
            let input: RevokeGrantArguments = decode(arguments, name)?;
            serde_json::to_value(open_commerce_service::revoke_grant(
                store,
                project_id,
                &input.grant_id,
                &actor,
            )?)?
        }
        "open_commerce_prepare_action_confirmation" => {
            let input: InvokeArguments = decode(arguments, name)?;
            let merchant = store.open_commerce_merchant(&input.merchant_id)?;
            let target_role = store
                .get_project_access(user_id, &merchant.project_id)
                .ok()
                .map(|access| access.role);
            serde_json::to_value(open_commerce_action_confirmation_service::prepare(
                store,
                &OpenCommerceActor {
                    user_id,
                    app_id,
                    project_role: target_role.as_deref(),
                },
                InvokeCapabilityRequest {
                    merchant_id: input.merchant_id,
                    capability_key: input.capability_key,
                    requester_app_id: app_id.to_string(),
                    grant_id: input.grant_id,
                    idempotency_key: input.idempotency_key,
                    input: input.input,
                },
            )?)?
        }
        "open_commerce_confirm_action_confirmation" => {
            let input: ConfirmActionArguments = decode(arguments, name)?;
            if input.confirmation_phrase != ACTION_CONFIRMATION_PHRASE {
                return Err(anyhow!("动作确认短语无效"));
            }
            serde_json::to_value(open_commerce_action_confirmation_service::confirm(
                store,
                &OpenCommerceActor {
                    user_id,
                    app_id,
                    project_role: Some(project_role),
                },
                &input.confirmation_id,
                &input.confirmation_phrase,
            )?)?
        }
        "open_commerce_invoke" => {
            let input: InvokeArguments = decode(arguments, name)?;
            let action_confirmation_id = input.action_confirmation_id.clone();
            let merchant = store.open_commerce_merchant(&input.merchant_id)?;
            let target_role = store
                .get_project_access(user_id, &merchant.project_id)
                .ok()
                .map(|access| access.role);
            open_commerce_service::invoke_with_action_confirmation(
                store,
                &OpenCommerceActor {
                    user_id,
                    app_id,
                    project_role: target_role.as_deref(),
                },
                InvokeCapabilityRequest {
                    merchant_id: input.merchant_id,
                    capability_key: input.capability_key,
                    requester_app_id: app_id.to_string(),
                    grant_id: input.grant_id,
                    idempotency_key: input.idempotency_key,
                    input: input.input,
                },
                action_confirmation_id.as_deref(),
            )
            .await?
        }
        "open_commerce_list_audit" => {
            let input: AuditArguments = decode(arguments, name)?;
            json!({
                "schema":"open_commerce.audit.v1",
                "events":store.list_project_open_commerce_audit(project_id, input.limit)?
            })
        }
        _ => return Err(anyhow!("未知开放商业 MCP 工具：{name}")),
    };
    tool_response(value)
}

fn authenticate(
    state: &AppState,
    headers: &HeaderMap,
    project_id: &str,
) -> Result<McpCaller, Response> {
    let user = auth_from_headers(state, headers)
        .map_err(|error| json_error(StatusCode::UNAUTHORIZED, error))?;
    let access = project_access(state, &user.id, project_id)
        .map_err(|error| json_error(StatusCode::FORBIDDEN, error))?;
    let raw_app_id = headers
        .get("x-elon-app-id")
        .and_then(|value| value.to_str().ok())
        .unwrap_or(DEFAULT_MCP_APP_ID);
    let app_id =
        normalize_app_id(raw_app_id).map_err(|error| json_error(StatusCode::BAD_REQUEST, error))?;
    Ok(McpCaller {
        user_id: user.id,
        project_role: access.role,
        app_id,
    })
}

fn initialize_response() -> Value {
    json!({
        "protocolVersion":MCP_PROTOCOL_VERSION,
        "capabilities":{"tools":{"listChanged":false}},
        "serverInfo":{"name":"yilong-open-commerce","version":"1.0.0"},
        "instructions":"开放商业任务先调用 open_commerce_get_overview；商户只有显式发布目录后才会进入跨项目发现。ERP 开发先调用 erp_get_overview、erp_search_capabilities 和 erp_resolve_requirement，避免重复造轮子。ERP 工具不允许接受提案、创建 Matter、合并、发布、采用或回滚。数据接入记录不包含令牌，公开发现只返回脱敏目录契约；授权能力必须携带 grant_id 并使用已注册应用身份；所有调用和同步回执必须使用幂等键。商户可手动封禁已注册 App，封禁会撤销现有授权且解除后不会恢复。当前只记录计量，不真实扣款。写操作需要当前项目编辑权限，调用身份由 x-elon-app-id 固定，不能由工具参数冒充。"
    })
}

fn tool_response(value: Value) -> Result<Value> {
    Ok(json!({
        "content":[{"type":"text","text":serde_json::to_string_pretty(&value)?}],
        "structuredContent":value,
        "isError":false
    }))
}

fn decode<T: for<'de> Deserialize<'de>>(arguments: Value, name: &str) -> Result<T> {
    serde_json::from_value(arguments).with_context(|| format!("{name} 参数无效"))
}

fn ensure_empty_object(arguments: &Value, name: &str) -> Result<()> {
    if arguments.as_object().is_some_and(|value| value.is_empty()) {
        Ok(())
    } else {
        Err(anyhow!("{name} 不接受参数"))
    }
}

fn empty_object() -> Value {
    json!({})
}

fn default_search_limit() -> usize {
    20
}

fn default_audit_limit() -> usize {
    50
}

#[cfg(test)]
mod tests {
    use super::initialize_response;

    #[test]
    fn initialize_declares_v1_safety_contract() {
        let value = initialize_response();
        assert_eq!(value["protocolVersion"], "2025-03-26");
        assert!(value["instructions"]
            .as_str()
            .is_some_and(|text| text.contains("不真实扣款")));
    }
}
