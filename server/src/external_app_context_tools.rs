//! Prompt-facing projection for external app tool contracts.

use serde_json::{json, Value};

const MAX_TOOL_ENTRIES: usize = 8;
const MAX_TOOL_JSON_CHARS: usize = 4_000;

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

fn no_tools_block(reason: &str) -> String {
    format!(
        "<available_external_app_tools status=\"unavailable\" reason=\"{reason}\">\n\
         当前外部项目没有声明可用工具。信息不足时只能说明缺口，不能假装查询了明细。\n\
         </available_external_app_tools>"
    )
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
    }
}
