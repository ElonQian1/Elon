//! Consumer action confirmation MCP tools with recoverable, actor-bound status reads.

use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{
    open_commerce_action_confirmation_model::{
        ACTION_CANCELLATION_PHRASE, ACTION_CONFIRMATION_PHRASE,
    },
    open_commerce_action_confirmation_service,
    open_commerce_model::InvokeCapabilityRequest,
    open_commerce_service::{self, OpenCommerceActor},
    store::Store,
};

const GET_MY_ACTION_CONFIRMATION: &str = "open_commerce_get_my_action_confirmation";
const CANCEL_MY_ACTION_CONFIRMATION: &str = "open_commerce_cancel_my_action_confirmation";
const PREPARE_ACTION_CONFIRMATION: &str = "open_commerce_prepare_action_confirmation";
const CONFIRM_ACTION_CONFIRMATION: &str = "open_commerce_confirm_action_confirmation";
const INVOKE: &str = "open_commerce_invoke";

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
struct ConfirmationArguments {
    confirmation_id: String,
}

#[derive(Debug, Deserialize)]
struct ConfirmActionArguments {
    confirmation_id: String,
    confirmation_phrase: String,
}

#[derive(Debug, Deserialize)]
struct CancelActionArguments {
    confirmation_id: String,
    confirmation_phrase: String,
}

pub(crate) fn definitions() -> Vec<Value> {
    vec![
        tool(
            GET_MY_ACTION_CONFIRMATION,
            "重新读取当前用户与当前 App 持有的一份动作确认，返回商户、能力、输入形状、状态、失效时间和下一步；不返回原始输入值。",
            json!({
                "type":"object",
                "required":["confirmation_id"],
                "properties":{
                    "confirmation_id":{"type":"string","minLength":1,"maxLength":120}
                },
                "additionalProperties":false
            }),
            true,
            false,
            true,
            false,
        ),
        tool(
            CANCEL_MY_ACTION_CONFIRMATION,
            "仅在当前用户明确要求停止后，取消当前用户与当前 App 持有且尚未创建 Invocation 的动作确认。已消费或自然过期确认不能伪装为取消。",
            json!({
                "type":"object",
                "required":["confirmation_id","confirmation_phrase"],
                "properties":{
                    "confirmation_id":{"type":"string","minLength":1,"maxLength":120},
                    "confirmation_phrase":{"const":"CANCEL_ACTION"}
                },
                "additionalProperties":false
            }),
            false,
            true,
            true,
            false,
        ),
        tool(
            PREPARE_ACTION_CONFIRMATION,
            "为动作类商户能力准备一份服务端短时确认。确认绑定当前用户、App、商户、能力、授权、幂等键和输入摘要；只准备，不执行。查询能力不需要调用此工具。",
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
            false,
            true,
            false,
        ),
        tool(
            CONFIRM_ACTION_CONFIRMATION,
            "仅在当前用户已明确同意执行准备结果所指向的经营操作后确认。确认不会执行能力；随后 open_commerce_invoke 仍须携带 confirmation_id。",
            json!({
                "type":"object",
                "required":["confirmation_id","confirmation_phrase"],
                "properties":{
                    "confirmation_id":{"type":"string","minLength":1,"maxLength":120},
                    "confirmation_phrase":{"const":"CONFIRM_ACTION"}
                },
                "additionalProperties":false
            }),
            false,
            true,
            true,
            false,
        ),
        tool(
            INVOKE,
            "调用一个商户能力。调用方身份来自当前 MCP 入口，不能冒充其他应用；必须提供幂等键。动作能力还必须携带已经明确确认、与同一输入绑定的一次性 confirmation_id。返回结果、计量金额和 recorded_not_charged 状态，V1 不真实扣款。",
            json!({
                "type":"object",
                "required":["merchant_id","capability_key","idempotency_key"],
                "properties":{
                    "merchant_id":{"type":"string","minLength":1,"maxLength":120},
                    "capability_key":{"type":"string","minLength":2,"maxLength":80},
                    "grant_id":{"type":"string","maxLength":120},
                    "idempotency_key":{"type":"string","minLength":8,"maxLength":120},
                    "action_confirmation_id":{"type":"string","maxLength":120},
                    "input":{"type":"object","default":{}}
                },
                "additionalProperties":false
            }),
            false,
            false,
            true,
            true,
        ),
    ]
}

pub(crate) async fn call_if_handled(
    store: &Store,
    user_id: &str,
    project_role: &str,
    app_id: &str,
    name: &str,
    arguments: Value,
) -> Result<Option<Value>> {
    match name {
        GET_MY_ACTION_CONFIRMATION => {
            let input: ConfirmationArguments = decode(arguments, name)?;
            let confirmation = store.open_commerce_action_confirmation_for_actor(
                &input.confirmation_id,
                user_id,
                app_id,
            )?;
            let expired = matches!(confirmation.status.as_str(), "pending" | "confirmed")
                && DateTime::parse_from_rfc3339(&confirmation.expires_at)
                    .context("动作确认失效时间无效")?
                    .with_timezone(&Utc)
                    <= Utc::now();
            let effective_status = match (
                confirmation.status.as_str(),
                expired,
                confirmation.canceled_at.is_some(),
            ) {
                (_, _, true) => "canceled",
                ("pending" | "confirmed", true, false) => "expired",
                ("pending", false, false) => "pending",
                ("confirmed", false, false) => "confirmed",
                ("consumed", _, false) => "consumed",
                ("expired", _, false) => "expired",
                _ => "invalid",
            };
            let next_step = match effective_status {
                "pending" => "obtain_explicit_user_confirmation",
                "confirmed" => "invoke_with_confirmation",
                "consumed" => "read_invocation_receipt",
                "canceled" => "stop",
                "expired" => "prepare_new_confirmation",
                _ => "stop",
            };
            Ok(Some(json!({
                "schema":"open_commerce.consumer_action_confirmation.v1",
                "confirmation_id":confirmation.id,
                "merchant_id":confirmation.merchant_id,
                "capability_key":confirmation.capability_key,
                "requester_app_id":confirmation.requester_app_id,
                "grant_id":confirmation.grant_id,
                "idempotency_key":confirmation.idempotency_key,
                "request_shape":confirmation.request_shape,
                "contains_raw_input_values":false,
                "status":effective_status,
                "stored_status":confirmation.status,
                "expires_at":confirmation.expires_at,
                "created_at":confirmation.created_at,
                "confirmed_at":confirmation.confirmed_at,
                "consumed_at":confirmation.consumed_at,
                "canceled_at":confirmation.canceled_at,
                "invocation_id":confirmation.invocation_id,
                "next_step":next_step
            })))
        }
        CANCEL_MY_ACTION_CONFIRMATION => {
            let input: CancelActionArguments = decode(arguments, name)?;
            if input.confirmation_phrase != ACTION_CANCELLATION_PHRASE {
                bail!("动作取消短语无效");
            }
            let confirmation = open_commerce_action_confirmation_service::cancel(
                store,
                &OpenCommerceActor {
                    user_id,
                    app_id,
                    project_role: Some(project_role),
                },
                &input.confirmation_id,
                &input.confirmation_phrase,
            )?;
            Ok(Some(json!({
                "schema":"open_commerce.consumer_action_confirmation_cancellation.v1",
                "confirmation_id":confirmation.id,
                "merchant_id":confirmation.merchant_id,
                "capability_key":confirmation.capability_key,
                "requester_app_id":confirmation.requester_app_id,
                "status":"canceled",
                "canceled_at":confirmation.canceled_at,
                "invocation_created":false,
                "next_step":"stop"
            })))
        }
        PREPARE_ACTION_CONFIRMATION => {
            let input: InvokeArguments = decode(arguments, name)?;
            let merchant = store.open_commerce_merchant(&input.merchant_id)?;
            let target_role = store
                .get_project_access(user_id, &merchant.project_id)
                .ok()
                .map(|access| access.role);
            Ok(Some(serde_json::to_value(
                open_commerce_action_confirmation_service::prepare(
                    store,
                    &OpenCommerceActor {
                        user_id,
                        app_id,
                        project_role: target_role.as_deref(),
                    },
                    invocation_request(input, app_id),
                )?,
            )?))
        }
        CONFIRM_ACTION_CONFIRMATION => {
            let input: ConfirmActionArguments = decode(arguments, name)?;
            if input.confirmation_phrase != ACTION_CONFIRMATION_PHRASE {
                bail!("动作确认短语无效");
            }
            Ok(Some(serde_json::to_value(
                open_commerce_action_confirmation_service::confirm(
                    store,
                    &OpenCommerceActor {
                        user_id,
                        app_id,
                        project_role: Some(project_role),
                    },
                    &input.confirmation_id,
                    &input.confirmation_phrase,
                )?,
            )?))
        }
        INVOKE => {
            let input: InvokeArguments = decode(arguments, name)?;
            let action_confirmation_id = input.action_confirmation_id.clone();
            let merchant = store.open_commerce_merchant(&input.merchant_id)?;
            let target_role = store
                .get_project_access(user_id, &merchant.project_id)
                .ok()
                .map(|access| access.role);
            Ok(Some(
                open_commerce_service::invoke_with_action_confirmation(
                    store,
                    &OpenCommerceActor {
                        user_id,
                        app_id,
                        project_role: target_role.as_deref(),
                    },
                    invocation_request(input, app_id),
                    action_confirmation_id.as_deref(),
                )
                .await?,
            ))
        }
        _ => Ok(None),
    }
}

fn invocation_request(input: InvokeArguments, app_id: &str) -> InvokeCapabilityRequest {
    InvokeCapabilityRequest {
        merchant_id: input.merchant_id,
        capability_key: input.capability_key,
        requester_app_id: app_id.to_string(),
        grant_id: input.grant_id,
        idempotency_key: input.idempotency_key,
        input: input.input,
    }
}

fn decode<T>(arguments: Value, name: &str) -> Result<T>
where
    T: serde::de::DeserializeOwned,
{
    serde_json::from_value(arguments).with_context(|| format!("{name} 参数无效"))
}

fn empty_object() -> Value {
    json!({})
}

fn tool(
    name: &str,
    description: &str,
    input_schema: Value,
    read_only: bool,
    destructive: bool,
    idempotent: bool,
    open_world: bool,
) -> Value {
    json!({
        "name":name,
        "description":description,
        "inputSchema":input_schema,
        "annotations":{
            "readOnlyHint":read_only,
            "destructiveHint":destructive,
            "idempotentHint":idempotent,
            "openWorldHint":open_world
        }
    })
}
