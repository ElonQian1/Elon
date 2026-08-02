use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{
    open_commerce_consumer_preference_model::{
        UpsertConsumerPreferenceDisclosureRequest, UpsertConsumerPreferenceProfileRequest,
    },
    open_commerce_consumer_preference_service,
    open_commerce_service::OpenCommerceActor,
    store::Store,
};

const GET_PROFILE: &str = "open_commerce_get_consumer_preference_profile";
const UPSERT_PROFILE: &str = "open_commerce_upsert_consumer_preference_profile";
const DELETE_PROFILE: &str = "open_commerce_delete_consumer_preference_profile";
const GET_DISCLOSURE: &str = "open_commerce_get_consumer_preference_disclosure";
const LIST_CONSUMER_DISCLOSURES: &str = "open_commerce_list_consumer_preference_disclosures";
const UPSERT_DISCLOSURE: &str = "open_commerce_upsert_consumer_preference_disclosure";
const DELETE_DISCLOSURE: &str = "open_commerce_delete_consumer_preference_disclosure";
const LIST_MERCHANT_DISCLOSURES: &str = "open_commerce_list_merchant_preference_disclosures";

#[derive(Deserialize)]
struct RelationshipArguments {
    relationship_id: String,
}

#[derive(Deserialize)]
struct UpsertDisclosureArguments {
    relationship_id: String,
    shared_fields: Vec<String>,
}

#[derive(Deserialize)]
struct MerchantArguments {
    merchant_id: String,
}

pub(crate) fn definitions() -> Vec<Value> {
    vec![
        tool(
            GET_PROFILE,
            "读取当前用户在本项目保存的结构化消费者偏好档案；不会读取其他项目成员的数据。",
            json!({"type":"object","properties":{},"additionalProperties":false}),
            true,
            false,
        ),
        tool(
            UPSERT_PROFILE,
            "保存当前用户的结构化偏好档案。只接受类别、标签、城市、调用价格上限和公开能力偏好，不接受自由文本或敏感身份资料。",
            json!({
                "type":"object",
                "required":["preferences"],
                "properties":{"preferences":preference_schema(true)},
                "additionalProperties":false
            }),
            false,
            false,
        ),
        tool(
            DELETE_PROFILE,
            "删除当前用户的偏好档案及本项目中由该档案产生的关系级披露快照；不会声称已删除商户自行复制的数据。",
            json!({"type":"object","properties":{},"additionalProperties":false}),
            false,
            true,
        ),
        tool(
            LIST_CONSUMER_DISCLOSURES,
            "读取当前用户在本项目主动生成的关系级偏好披露列表，包括已经失效关系的本人审计视图。",
            json!({"type":"object","properties":{},"additionalProperties":false}),
            true,
            false,
        ),
        tool(
            GET_DISCLOSURE,
            "读取当前用户针对一条本人关系凭证保存的偏好披露快照和当前关系状态。",
            relationship_schema(),
            true,
            false,
        ),
        tool(
            UPSERT_DISCLOSURE,
            "从当前用户已保存的偏好档案中选择字段，向一条有效且含 preference.remember 的关系生成匿名披露快照。",
            json!({
                "type":"object",
                "required":["relationship_id","shared_fields"],
                "properties":{
                    "relationship_id":{"type":"string","minLength":1,"maxLength":120},
                    "shared_fields":{
                        "type":"array","minItems":1,"maxItems":4,"uniqueItems":true,
                        "items":{"type":"string","enum":["categories","tags","city","max_unit_price_micros"]}
                    }
                },
                "additionalProperties":false
            }),
            false,
            false,
        ),
        tool(
            DELETE_DISCLOSURE,
            "撤回当前用户针对一条本人关系凭证保存的偏好披露快照。重复撤回保持成功结果。",
            relationship_schema(),
            false,
            true,
        ),
        tool(
            LIST_MERCHANT_DISCLOSURES,
            "读取当前项目指定商户仍可访问的匿名偏好披露。只返回有效关系的明确共享字段，不返回消费者账号或项目。",
            json!({
                "type":"object",
                "required":["merchant_id"],
                "properties":{"merchant_id":{"type":"string","minLength":1,"maxLength":120}},
                "additionalProperties":false
            }),
            true,
            false,
        ),
    ]
}

pub(crate) fn call_if_handled(
    store: &Store,
    project_id: &str,
    user_id: &str,
    project_role: &str,
    app_id: &str,
    name: &str,
    arguments: Value,
) -> Result<Option<Value>> {
    if !handles(name) {
        return Ok(None);
    }
    let actor = OpenCommerceActor {
        user_id,
        app_id,
        project_role: Some(project_role),
    };
    let value = match name {
        GET_PROFILE => {
            ensure_empty(&arguments, name)?;
            serde_json::to_value(open_commerce_consumer_preference_service::get_profile(
                store, project_id, &actor,
            )?)?
        }
        UPSERT_PROFILE => {
            let request = decode(arguments, name)?;
            serde_json::to_value(open_commerce_consumer_preference_service::upsert_profile(
                store, project_id, &actor, request,
            )?)?
        }
        DELETE_PROFILE => {
            ensure_empty(&arguments, name)?;
            serde_json::to_value(open_commerce_consumer_preference_service::delete_profile(
                store, project_id, &actor,
            )?)?
        }
        GET_DISCLOSURE => {
            let input: RelationshipArguments = decode(arguments, name)?;
            serde_json::to_value(open_commerce_consumer_preference_service::get_disclosure(
                store,
                project_id,
                &input.relationship_id,
                &actor,
            )?)?
        }
        LIST_CONSUMER_DISCLOSURES => {
            ensure_empty(&arguments, name)?;
            serde_json::to_value(
                open_commerce_consumer_preference_service::list_consumer_disclosures(
                    store, project_id, &actor, 100,
                )?,
            )?
        }
        UPSERT_DISCLOSURE => {
            let input: UpsertDisclosureArguments = decode(arguments, name)?;
            serde_json::to_value(
                open_commerce_consumer_preference_service::upsert_disclosure(
                    store,
                    project_id,
                    &input.relationship_id,
                    &actor,
                    UpsertConsumerPreferenceDisclosureRequest {
                        shared_fields: input.shared_fields,
                    },
                )?,
            )?
        }
        DELETE_DISCLOSURE => {
            let input: RelationshipArguments = decode(arguments, name)?;
            serde_json::to_value(
                open_commerce_consumer_preference_service::delete_disclosure(
                    store,
                    project_id,
                    &input.relationship_id,
                    &actor,
                )?,
            )?
        }
        LIST_MERCHANT_DISCLOSURES => {
            let input: MerchantArguments = decode(arguments, name)?;
            serde_json::to_value(
                open_commerce_consumer_preference_service::list_merchant_disclosures(
                    store,
                    project_id,
                    &input.merchant_id,
                    &actor,
                    100,
                )?,
            )?
        }
        _ => unreachable!(),
    };
    Ok(Some(value))
}

fn handles(name: &str) -> bool {
    matches!(
        name,
        GET_PROFILE
            | UPSERT_PROFILE
            | DELETE_PROFILE
            | LIST_CONSUMER_DISCLOSURES
            | GET_DISCLOSURE
            | UPSERT_DISCLOSURE
            | DELETE_DISCLOSURE
            | LIST_MERCHANT_DISCLOSURES
    )
}

fn decode<T: serde::de::DeserializeOwned>(arguments: Value, name: &str) -> Result<T> {
    serde_json::from_value(arguments).with_context(|| format!("{name} 参数无效"))
}

fn ensure_empty(arguments: &Value, name: &str) -> Result<()> {
    let empty = arguments
        .as_object()
        .map(|value| value.is_empty())
        .unwrap_or(false);
    if !empty {
        anyhow::bail!("{name} 不接受参数");
    }
    Ok(())
}

fn tool(
    name: &str,
    description: &str,
    input_schema: Value,
    read_only: bool,
    idempotent: bool,
) -> Value {
    json!({
        "name":name,
        "description":description,
        "inputSchema":input_schema,
        "annotations":{
            "readOnlyHint":read_only,
            "destructiveHint":matches!(name, DELETE_PROFILE | DELETE_DISCLOSURE),
            "idempotentHint":idempotent,
            "openWorldHint":false
        }
    })
}

fn relationship_schema() -> Value {
    json!({
        "type":"object",
        "required":["relationship_id"],
        "properties":{"relationship_id":{"type":"string","minLength":1,"maxLength":120}},
        "additionalProperties":false
    })
}

fn preference_schema(include_prefer_public: bool) -> Value {
    let mut properties = serde_json::Map::from_iter([
        (
            "categories".to_string(),
            json!({
                "type":"array","maxItems":20,"items":{"type":"string","maxLength":80}
            }),
        ),
        (
            "tags".to_string(),
            json!({
                "type":"array","maxItems":40,"items":{"type":"string","maxLength":80}
            }),
        ),
        ("city".to_string(), json!({"type":"string","maxLength":120})),
        (
            "max_unit_price_micros".to_string(),
            json!({
            "type":"integer","minimum":0,"maximum":1_000_000_000_000_000_i64
            }),
        ),
    ]);
    if include_prefer_public {
        properties.insert("prefer_public".to_string(), json!({"type":"boolean"}));
    }
    json!({
        "type":"object",
        "properties":properties,
        "additionalProperties":false
    })
}
