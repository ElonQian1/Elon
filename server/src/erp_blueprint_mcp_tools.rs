use serde_json::{json, Value};

pub(crate) fn definitions() -> Vec<Value> {
    vec![
        tool(
            "erp_get_overview",
            "读取当前项目关联的 ERP 蓝图、版本、商户实例、能力目录、提案与升级状态。不会返回商户原始经营数据、密钥或私有源码。",
            json!({"type":"object","properties":{},"additionalProperties":false}),
            true,
            false,
            true,
        ),
        tool(
            "erp_search_capabilities",
            "在官方 ERP 能力目录中检索可直接复用的能力。开发前应先调用，避免重复造轮子。",
            json!({
                "type":"object",
                "required":["query"],
                "properties":{
                    "query":{"type":"string","minLength":1,"maxLength":500},
                    "limit":{"type":"integer","minimum":1,"maximum":100,"default":20}
                },
                "additionalProperties":false
            }),
            true,
            false,
            true,
        ),
        tool(
            "erp_resolve_requirement",
            "将商户需求判定为已有能力、能力组合、私有扩展或通用候选。此工具只给出计划，不修改公共内核。",
            json!({
                "type":"object",
                "required":["requirement"],
                "properties":{
                    "instance_id":{"type":"string","maxLength":120},
                    "requirement":{"type":"string","minLength":4,"maxLength":500},
                    "expected_scope":{"type":"string","enum":["merchant_specific","potential_common"]}
                },
                "additionalProperties":false
            }),
            true,
            false,
            true,
        ),
        tool(
            "erp_submit_feature_signal",
            "在商户明确授权后提交脱敏通用需求信号。同一实例同一 need_key 只计一次；禁止原始经营数据、密钥、个人信息和源码。",
            json!({
                "type":"object",
                "required":["schema","instance_id","requirement_summary","industry","merchant_authorized","classification"],
                "properties":{
                    "schema":{"const":"yilong.erp.feature_signal.v1"},
                    "instance_id":{"type":"string","minLength":1,"maxLength":120},
                    "requirement_summary":{"type":"string","minLength":8,"maxLength":500},
                    "need_key":{"type":"string","maxLength":80},
                    "industry":{"type":"string","minLength":1,"maxLength":80},
                    "requested_outcome":{"type":"string","maxLength":300},
                    "merchant_authorized":{"const":true},
                    "classification":{"type":"string","enum":["sanitized_aggregate","public_requirement"]},
                    "evidence":{
                        "type":"object",
                        "properties":{
                            "occurrence_count":{"type":"integer","minimum":1,"maximum":100000},
                            "affected_workflow":{"type":"string","maxLength":120},
                            "estimated_time_saved_minutes":{"type":"integer","minimum":0,"maximum":100000}
                        },
                        "additionalProperties":false
                    }
                },
                "additionalProperties":false
            }),
            false,
            false,
            true,
        ),
        tool(
            "erp_update_instance_configuration",
            "在商户确认后更新当前实例的主题、启用模块、插件和私有扩展元数据。只登记边界与版本，不上传扩展源码、密钥或经营数据。",
            json!({
                "type":"object",
                "required":["instance_id","expected_revision","merchant_confirmed","theme_key","enabled_modules","plugins","private_extensions"],
                "properties":{
                    "instance_id":{"type":"string","minLength":1,"maxLength":120},
                    "expected_revision":{"type":"integer","minimum":1},
                    "merchant_confirmed":{"const":true},
                    "theme_key":{"type":"string","minLength":2,"maxLength":80},
                    "enabled_modules":{"type":"array","items":{"type":"string","minLength":2,"maxLength":80},"uniqueItems":true},
                    "plugins":{"type":"array","items":{"$ref":"#/definitions/extension"}},
                    "private_extensions":{"type":"array","items":{"$ref":"#/definitions/extension"}}
                },
                "definitions":{
                    "extension":{
                        "type":"object",
                        "required":["extension_key","version","extension_point"],
                        "properties":{
                            "extension_key":{"type":"string","minLength":2,"maxLength":80},
                            "version":{"type":"string","pattern":"^[0-9]+\\.[0-9]+\\.[0-9]+$"},
                            "extension_point":{"type":"string","minLength":2,"maxLength":80},
                            "requires_modules":{"type":"array","items":{"type":"string"},"uniqueItems":true}
                        },
                        "additionalProperties":false
                    }
                },
                "additionalProperties":false
            }),
            false,
            true,
            false,
        ),
        tool(
            "erp_prepare_upgrade_check",
            "只准备目标版本兼容检查并保存升级计划，不执行 Git、迁移、部署、采用或回滚。",
            json!({
                "type":"object",
                "required":["instance_id","target_version"],
                "properties":{
                    "instance_id":{"type":"string","minLength":1,"maxLength":120},
                    "target_version":{"type":"string","minLength":5,"maxLength":40}
                },
                "additionalProperties":false
            }),
            false,
            false,
            false,
        ),
    ]
}

fn tool(
    name: &str,
    description: &str,
    input_schema: Value,
    read_only: bool,
    destructive: bool,
    idempotent: bool,
) -> Value {
    json!({
        "name":name,
        "description":description,
        "inputSchema":input_schema,
        "annotations":{
            "readOnlyHint":read_only,
            "destructiveHint":destructive,
            "idempotentHint":idempotent,
            "openWorldHint":false
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instance_configuration_is_the_only_destructive_agent_tool() {
        let tools = definitions();
        let destructive = tools
            .iter()
            .filter(|tool| tool["annotations"]["destructiveHint"] == true)
            .map(|tool| tool["name"].as_str().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(destructive, vec!["erp_update_instance_configuration"]);
    }
}
