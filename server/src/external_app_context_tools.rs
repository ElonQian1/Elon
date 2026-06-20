//! Prompt-facing projection for external app tool contracts.

use serde_json::{json, Value};

const MAX_TOOL_ENTRIES: usize = 8;
const MAX_TOOL_JSON_CHARS: usize = 4_000;
const RECOMMENDED_FB2_TOOLS: &[&str] = &[
    "search_matches",
    "get_match_detail",
    "search_user_orders",
    "get_order_detail",
    "search_group_opinions",
];

pub(crate) fn public_tool_contract_guidance(app_id: &str) -> Option<Value> {
    match app_id {
        "fb2" => Some(json!({
            "app_id": "fb2",
            "schema": "fb2.tools.v1",
            "execution_status": "declared_only",
            "recommended_tools": recommended_fb2_tool_contract(),
            "required_context_fields": [
                "context_pack",
                "context_pack_version",
                "generated_at",
                "matches",
                "user_orders",
                "group_messages",
                "tool_contract",
                "usage_policy"
            ],
            "recommended_env": [
                "ELON_EXTERNAL_APP_FB2_BASE_URL",
                "ELON_EXTERNAL_APP_FB2_CONTEXT_TOKEN",
                "ELON_EXTERNAL_APP_FB2_CONTEXT_PACK_ENABLED",
                "ELON_EXTERNAL_APP_CONTEXT_MAX_CHARS"
            ],
            "notes": [
                "主项目当前只把工具作为规划和追问依据，不会自动执行 fb2 工具。",
                "用户订单工具必须限制为 current_user_only。",
                "群友观点必须返回 message_id，比赛和订单必须返回 source id。"
            ]
        })),
        _ => None,
    }
}

pub(crate) fn prompt_tool_contract_block(context: &Value) -> String {
    let Some(contract) = context.get("tool_contract") else {
        return no_tools_block("missing_tool_contract");
    };
    if contract.is_null() {
        return no_tools_block("missing_tool_contract");
    }

    let projected = project_tool_contract(contract);
    if projected["tools"]
        .as_array()
        .map(|tools| tools.is_empty())
        .unwrap_or(true)
    {
        return no_tools_block("empty_tool_contract");
    }

    format!(
        "<available_external_app_tools status=\"declared_only\">\n\
         {}\n\
         <tool_rules>\n\
         - 这些工具当前只作为后续追问/检索计划的契约提示，不能在回答中假装已经调用。\n\
         - 如果当前 context_pack 信息不足，可以说明需要调用哪个工具补充，例如 get_match_detail 或 search_user_orders。\n\
         - 不能编造工具返回结果；未调用工具时只能基于现有上下文回答。\n\
         - 工具调用必须遵守用户权限，用户订单只能查询当前用户自己的数据。\n\
         </tool_rules>\n\
         </available_external_app_tools>",
        serde_json::to_string_pretty(&projected).unwrap_or_else(|_| "{}".to_string())
    )
}

pub(crate) fn tool_contract_quality_warning(context: &Value) -> Option<&'static str> {
    let contract = context.get("tool_contract")?;
    if contract.is_null() {
        return Some("missing_tool_contract");
    }
    let tools = extract_tools(contract);
    if tools.is_empty() {
        Some("empty_tool_contract")
    } else {
        None
    }
}

pub(crate) fn tool_contract_readiness(context: &Value) -> Value {
    let Some(contract) = context.get("tool_contract") else {
        return readiness("missing", Vec::new(), RECOMMENDED_FB2_TOOLS.to_vec());
    };
    if contract.is_null() {
        return readiness("missing", Vec::new(), RECOMMENDED_FB2_TOOLS.to_vec());
    }

    let names = tool_names(contract);
    if names.is_empty() {
        return readiness("empty", names, RECOMMENDED_FB2_TOOLS.to_vec());
    }
    let missing = RECOMMENDED_FB2_TOOLS
        .iter()
        .copied()
        .filter(|name| !names.iter().any(|existing| existing == name))
        .collect::<Vec<_>>();
    let status = if missing.is_empty() {
        "ready"
    } else {
        "partial"
    };
    readiness(status, names, missing)
}

fn no_tools_block(reason: &str) -> String {
    format!(
        "<available_external_app_tools status=\"unavailable\" reason=\"{reason}\">\n\
         当前外部项目没有声明可用工具。信息不足时只能说明缺口，不能假装查询了明细。\n\
         </available_external_app_tools>"
    )
}

fn readiness(status: &str, names: Vec<String>, missing: Vec<&str>) -> Value {
    json!({
        "status": status,
        "declared_tools": names,
        "declared_count": names.len(),
        "recommended_tools": RECOMMENDED_FB2_TOOLS,
        "missing_recommended_tools": missing,
        "execution_status": "declared_only"
    })
}

fn recommended_fb2_tool_contract() -> Value {
    json!([
        {
            "name": "search_matches",
            "description": "按日期、联赛、球队、彩种搜索比赛。",
            "permission": "group_context",
            "when_to_use": "用户询问今天、某联赛或某球队有哪些比赛可分析时"
        },
        {
            "name": "get_match_detail",
            "description": "按 match id 查询比赛、赔率、伤停、更新时间和数据源。",
            "permission": "group_context",
            "when_to_use": "用户追问某一场比赛细节或 context_pack 被截断时"
        },
        {
            "name": "search_user_orders",
            "description": "查询当前登录用户自己的票据和订单摘要。",
            "permission": "current_user_only",
            "when_to_use": "用户要求分析自己的票或订单风险时"
        },
        {
            "name": "get_order_detail",
            "description": "按订单或票据 ID 查询当前用户可见的明细。",
            "permission": "current_user_only",
            "when_to_use": "用户追问某张票的组合、赔率或风险拆解时"
        },
        {
            "name": "search_group_opinions",
            "description": "按比赛或关键词检索群友观点，并返回 message_id。",
            "permission": "group_context",
            "when_to_use": "用户要求总结群友观点、分歧或采纳建议时"
        }
    ])
}

fn project_tool_contract(contract: &Value) -> Value {
    let mut tools = extract_tools(contract);
    let truncated = tools.len() > MAX_TOOL_ENTRIES;
    tools.truncate(MAX_TOOL_ENTRIES);

    let mut projected = json!({
        "schema": contract.get("schema").or_else(|| contract.get("version")),
        "tools": tools,
        "truncated": truncated
    });

    let json_chars = serde_json::to_string(&projected)
        .map(|text| text.chars().count())
        .unwrap_or(0);
    if json_chars > MAX_TOOL_JSON_CHARS {
        projected["tools"] = Value::Array(Vec::new());
        projected["truncated"] = Value::Bool(true);
        projected["warning"] = json!("tool_contract_too_large");
    }
    projected
}

fn tool_names(contract: &Value) -> Vec<String> {
    extract_tools(contract)
        .into_iter()
        .filter_map(|tool| {
            tool.get("name")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .map(ToOwned::to_owned)
        })
        .collect()
}

fn extract_tools(contract: &Value) -> Vec<Value> {
    contract
        .get("tools")
        .or_else(|| contract.get("functions"))
        .and_then(Value::as_array)
        .map(|tools| tools.iter().map(project_tool).collect())
        .unwrap_or_default()
}

fn project_tool(tool: &Value) -> Value {
    json!({
        "name": tool.get("name"),
        "description": tool.get("description"),
        "input_schema": tool.get("input_schema").or_else(|| tool.get("parameters")),
        "permission": tool.get("permission").or_else(|| tool.get("scope")),
        "when_to_use": tool.get("when_to_use")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_declared_tools_without_claiming_execution() {
        let block = prompt_tool_contract_block(&json!({
            "tool_contract": {
                "schema": "fb2.tools.v1",
                "tools": [{
                    "name": "get_match_detail",
                    "description": "Load one match",
                    "input_schema": {"type": "object"}
                }]
            }
        }));

        assert!(block.contains("get_match_detail"));
        assert!(block.contains("不能在回答中假装已经调用"));
    }

    #[test]
    fn reports_missing_tool_contract() {
        let block = prompt_tool_contract_block(&json!({}));
        assert!(block.contains("status=\"unavailable\""));
        assert_eq!(
            tool_contract_quality_warning(&json!({"tool_contract": null})),
            Some("missing_tool_contract")
        );
        assert_eq!(
            tool_contract_readiness(&json!({}))["status"].as_str(),
            Some("missing")
        );
    }

    #[test]
    fn reports_partial_tool_readiness() {
        let readiness = tool_contract_readiness(&json!({
            "tool_contract": {
                "tools": [
                    {"name": "get_match_detail"},
                    {"name": "search_user_orders"}
                ]
            }
        }));

        assert_eq!(readiness["status"], "partial");
        assert_eq!(readiness["declared_count"], 2);
        assert!(readiness["missing_recommended_tools"]
            .as_array()
            .unwrap()
            .contains(&json!("search_group_opinions")));
    }

    #[test]
    fn exposes_public_fb2_guidance() {
        let guidance = public_tool_contract_guidance("fb2").unwrap();
        assert_eq!(guidance["schema"], "fb2.tools.v1");
        assert!(guidance["recommended_tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["name"] == "search_group_opinions"));
        assert!(public_tool_contract_guidance("unknown").is_none());
    }
}
