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
    open_commerce_model::{
        normalize_app_id, CreateCapabilityRequest, CreateGrantRequest, CreateMerchantRequest,
        InvokeCapabilityRequest,
    },
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
struct PublishCapabilityArguments {
    merchant_id: String,
    #[serde(flatten)]
    request: CreateCapabilityRequest,
}

#[derive(Debug, Deserialize)]
struct RevokeGrantArguments {
    grant_id: String,
}

#[derive(Debug, Deserialize)]
struct InvokeArguments {
    merchant_id: String,
    capability_key: String,
    #[serde(default)]
    grant_id: Option<String>,
    idempotency_key: String,
    #[serde(default = "empty_object")]
    input: Value,
}

#[derive(Debug, Deserialize)]
struct AuditArguments {
    #[serde(default = "default_audit_limit")]
    limit: usize,
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
        "tools/list" => Ok(json!({"tools": crate::open_commerce_mcp_tools::definitions()})),
        "tools/call" => call_tool(&state, &project_id, &caller, request.params),
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

fn call_tool(
    state: &AppState,
    project_id: &str,
    caller: &McpCaller,
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
    let actor = OpenCommerceActor {
        user_id: &caller.user_id,
        app_id: &caller.app_id,
        project_role: Some(&caller.project_role),
    };
    let value = match name {
        "open_commerce_get_overview" => {
            ensure_empty_object(&arguments, name)?;
            serde_json::to_value(open_commerce_service::overview(&state.store, project_id)?)?
        }
        "open_commerce_search_merchants" => {
            let input: SearchArguments = decode(arguments, name)?;
            json!({
                "schema":"open_commerce.discovery.v1",
                "merchants":open_commerce_service::discover_merchants(
                    &state.store,
                    input.query.as_deref(),
                    input.capability.as_deref(),
                    input.limit,
                )?
            })
        }
        "open_commerce_get_merchant" => {
            let input: MerchantArguments = decode(arguments, name)?;
            serde_json::to_value(open_commerce_service::discover_merchant(
                &state.store,
                &input.merchant_id,
            )?)?
        }
        "open_commerce_create_merchant" => {
            let input: CreateMerchantRequest = decode(arguments, name)?;
            serde_json::to_value(open_commerce_service::create_merchant(
                &state.store,
                project_id,
                &actor,
                input,
            )?)?
        }
        "open_commerce_publish_capability" => {
            let input: PublishCapabilityArguments = decode(arguments, name)?;
            serde_json::to_value(open_commerce_service::publish_capability(
                &state.store,
                project_id,
                &input.merchant_id,
                &actor,
                input.request,
            )?)?
        }
        "open_commerce_create_grant" => {
            let input: CreateGrantRequest = decode(arguments, name)?;
            serde_json::to_value(open_commerce_service::create_grant(
                &state.store,
                project_id,
                &actor,
                input,
            )?)?
        }
        "open_commerce_revoke_grant" => {
            let input: RevokeGrantArguments = decode(arguments, name)?;
            serde_json::to_value(open_commerce_service::revoke_grant(
                &state.store,
                project_id,
                &input.grant_id,
                &actor,
            )?)?
        }
        "open_commerce_invoke" => {
            let input: InvokeArguments = decode(arguments, name)?;
            let merchant = state.store.open_commerce_merchant(&input.merchant_id)?;
            let target_role = project_access(state, &caller.user_id, &merchant.project_id)
                .ok()
                .map(|access| access.role);
            open_commerce_service::invoke(
                &state.store,
                &OpenCommerceActor {
                    user_id: &caller.user_id,
                    app_id: &caller.app_id,
                    project_role: target_role.as_deref(),
                },
                InvokeCapabilityRequest {
                    merchant_id: input.merchant_id,
                    capability_key: input.capability_key,
                    requester_app_id: caller.app_id.clone(),
                    grant_id: input.grant_id,
                    idempotency_key: input.idempotency_key,
                    input: input.input,
                },
            )?
        }
        "open_commerce_list_audit" => {
            let input: AuditArguments = decode(arguments, name)?;
            json!({
                "schema":"open_commerce.audit.v1",
                "events":state.store.list_project_open_commerce_audit(project_id, input.limit)?
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
        "instructions":"先调用 open_commerce_get_overview 或 search_merchants。公开发现不暴露处理器配置；授权能力必须携带 grant_id；所有调用必须使用幂等键。V1 只记录计量，不真实扣款。写操作需要当前项目编辑权限，调用身份由 x-elon-app-id 固定，不能由工具参数冒充。"
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
