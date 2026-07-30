//! Vendor-neutral MCP tool contracts for the V1 open-commerce network.

use serde_json::{json, Value};

pub(crate) fn definitions() -> Vec<Value> {
    vec![
        tool(
            "open_commerce_get_overview",
            "读取当前项目的商户节点、能力、授权、最近调用、计量与审计总览。先用它了解现状；不会修改数据。",
            json!({"type":"object","properties":{},"additionalProperties":false}),
            true,
            true,
        ),
        tool(
            "open_commerce_get_development_context",
            "生成供开发代理使用的商户能力、数据来源和同步健康度上下文。不会返回平台密钥、处理器配置或原始经营数据。",
            json!({"type":"object","properties":{},"additionalProperties":false}),
            true,
            true,
        ),
        tool(
            "open_commerce_search_merchants",
            "按名称、描述或能力键搜索已发布商户节点。只返回公开资料和公开能力契约，不返回处理器配置。",
            json!({
                "type":"object",
                "properties":{
                    "query":{"type":"string","maxLength":120},
                    "capability":{"type":"string","maxLength":80},
                    "limit":{"type":"integer","minimum":1,"maximum":100,"default":20}
                },
                "additionalProperties":false
            }),
            true,
            true,
        ),
        tool(
            "open_commerce_get_merchant",
            "读取一个已发布商户节点及其当前有效能力契约；不返回处理器私有配置。",
            json!({
                "type":"object",
                "required":["merchant_id"],
                "properties":{"merchant_id":{"type":"string","minLength":1,"maxLength":120}},
                "additionalProperties":false
            }),
            true,
            true,
        ),
        tool(
            "open_commerce_create_merchant",
            "在当前项目创建商户节点。需要项目编辑权限；默认由平台托管，但数据仍归当前项目控制。",
            json!({
                "type":"object",
                "required":["display_name"],
                "properties":{
                    "display_name":{"type":"string","minLength":1,"maxLength":120},
                    "slug":{"type":"string","minLength":3,"maxLength":64},
                    "description":{"type":"string","maxLength":2000},
                    "node_mode":{"type":"string","enum":["platform_hosted","self_hosted","third_party_hosted"],"default":"platform_hosted"},
                    "public_profile":{"type":"object","default":{}}
                },
                "additionalProperties":false
            }),
            false,
            false,
        ),
        tool(
            "open_commerce_publish_capability",
            "为当前项目的商户发布可被 AI 发现和调用的能力。V1 仅允许 merchant_profile 和 static_json 两种平台审核处理器，不允许任意外部 URL。",
            json!({
                "type":"object",
                "required":["merchant_id","capability_key","display_name","handler_type"],
                "properties":{
                    "merchant_id":{"type":"string","minLength":1,"maxLength":120},
                    "capability_key":{"type":"string","minLength":2,"maxLength":80},
                    "display_name":{"type":"string","minLength":1,"maxLength":120},
                    "description":{"type":"string","maxLength":2000},
                    "kind":{"type":"string","maxLength":80,"default":"information"},
                    "access_level":{"type":"string","enum":["public","authorized","owner_only"],"default":"public"},
                    "input_schema":{"type":"object","default":{}},
                    "output_schema":{"type":"object","default":{}},
                    "handler_type":{"type":"string","enum":["merchant_profile","static_json"]},
                    "handler_config":{"type":"object"},
                    "unit_price_micros":{"type":"integer","minimum":0,"default":0},
                    "currency":{"type":"string","minLength":3,"maxLength":8,"default":"CNY"},
                    "freshness_seconds":{"type":"integer","minimum":0,"default":0}
                },
                "additionalProperties":false
            }),
            false,
            false,
        ),
        tool(
            "open_commerce_create_grant",
            "授权指定 AI 应用调用某个商户的一个或多个能力。授权可设置用途和到期时间，并可随时撤销。",
            json!({
                "type":"object",
                "required":["merchant_id","grantee_app_id","scopes","purpose"],
                "properties":{
                    "merchant_id":{"type":"string","minLength":1,"maxLength":120},
                    "grantee_app_id":{"type":"string","minLength":2,"maxLength":80},
                    "scopes":{"type":"array","minItems":1,"maxItems":64,"items":{"type":"string","minLength":2,"maxLength":80}},
                    "purpose":{"type":"string","minLength":1,"maxLength":500},
                    "expires_at":{"type":"string","format":"date-time"}
                },
                "additionalProperties":false
            }),
            false,
            false,
        ),
        tool(
            "open_commerce_create_integration",
            "登记一个商户自有数据源的接入方式、授权范围和数据域。这里只登记连接事实，不接收也不保存访问令牌。",
            json!({
                "type":"object",
                "required":[
                    "merchant_id","integration_key","provider_key",
                    "display_name","connection_mode"
                ],
                "properties":{
                    "merchant_id":{"type":"string","minLength":1,"maxLength":120},
                    "integration_key":{"type":"string","minLength":3,"maxLength":96},
                    "provider_key":{"type":"string","minLength":2,"maxLength":64},
                    "display_name":{"type":"string","minLength":2,"maxLength":80},
                    "connection_mode":{
                        "type":"string",
                        "enum":["official_api","merchant_export","local_adapter","manual_import"]
                    },
                    "scopes":{
                        "type":"array","maxItems":32,
                        "items":{"type":"string","minLength":1,"maxLength":64}
                    },
                    "data_domains":{
                        "type":"array","maxItems":32,
                        "items":{"type":"string","minLength":1,"maxLength":64}
                    }
                },
                "additionalProperties":false
            }),
            false,
            false,
        ),
        tool(
            "open_commerce_set_integration_enabled",
            "停用或重新启用一个商户数据接入。重新启用只恢复为已配置状态，不能伪造连接成功。",
            json!({
                "type":"object",
                "required":["integration_id","enabled"],
                "properties":{
                    "integration_id":{"type":"string","minLength":1,"maxLength":120},
                    "enabled":{"type":"boolean"}
                },
                "additionalProperties":false
            }),
            false,
            true,
        ),
        tool(
            "open_commerce_record_sync_receipt",
            "由受信任适配器记录一次有界同步或健康检查结果。回执只含数量、摘要和错误码，不接收原始订单、客户或财务数据。",
            json!({
                "type":"object",
                "required":[
                    "integration_id","receipt_key","sync_kind","status",
                    "started_at","completed_at"
                ],
                "properties":{
                    "integration_id":{"type":"string","minLength":1,"maxLength":120},
                    "receipt_key":{"type":"string","minLength":3,"maxLength":128},
                    "sync_kind":{"type":"string","enum":["full","incremental","health_check"]},
                    "status":{"type":"string","enum":["succeeded","partial","failed"]},
                    "records_seen":{"type":"integer","minimum":0,"default":0},
                    "records_changed":{"type":"integer","minimum":0,"default":0},
                    "cursor_digest":{"type":"string","maxLength":128},
                    "error_code":{"type":"string","maxLength":96},
                    "started_at":{"type":"string","format":"date-time"},
                    "completed_at":{"type":"string","format":"date-time"}
                },
                "additionalProperties":false
            }),
            false,
            true,
        ),
        tool(
            "open_commerce_revoke_grant",
            "撤销当前项目的一项商业能力授权。重复撤销保持同一结果。",
            json!({
                "type":"object",
                "required":["grant_id"],
                "properties":{"grant_id":{"type":"string","minLength":1,"maxLength":120}},
                "additionalProperties":false
            }),
            false,
            true,
        ),
        tool(
            "open_commerce_invoke",
            "调用一个商户能力。调用方身份来自当前 MCP 入口，不能冒充其他应用；必须提供幂等键。返回结果、计量金额和 recorded_not_charged 状态，V1 不真实扣款。",
            json!({
                "type":"object",
                "required":["merchant_id","capability_key","idempotency_key"],
                "properties":{
                    "merchant_id":{"type":"string","minLength":1,"maxLength":120},
                    "capability_key":{"type":"string","minLength":2,"maxLength":80},
                    "grant_id":{"type":"string","maxLength":120},
                    "idempotency_key":{"type":"string","minLength":8,"maxLength":120},
                    "input":{"type":"object","default":{}}
                },
                "additionalProperties":false
            }),
            false,
            true,
        ),
        tool(
            "open_commerce_list_audit",
            "读取当前项目最近的商户、能力、授权和调用审计事件。审计只保存字段形状和摘要，不保存调用原始值。",
            json!({
                "type":"object",
                "properties":{"limit":{"type":"integer","minimum":1,"maximum":200,"default":50}},
                "additionalProperties":false
            }),
            true,
            true,
        ),
    ]
}

fn tool(
    name: &str,
    description: &str,
    input_schema: Value,
    read_only: bool,
    idempotent: bool,
) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": input_schema,
        "annotations": {
            "readOnlyHint": read_only,
            "destructiveHint": false,
            "idempotentHint": idempotent,
            "openWorldHint": name == "open_commerce_search_merchants"
                || name == "open_commerce_get_merchant"
                || name == "open_commerce_invoke"
        }
    })
}
