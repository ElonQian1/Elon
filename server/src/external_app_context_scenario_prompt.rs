//! server/src/external_app_context_scenario_prompt.rs
//! Runtime prompt guidance for fb2 domain scenarios.

use std::collections::BTreeSet;

use serde_json::{json, Value};

use crate::external_app_context_projection::fb2_domain_scenario_matrix;

pub(crate) fn prompt_domain_scenario_guidance(
    context: Option<&Value>,
    execution: Option<&Value>,
) -> String {
    if !is_fb2_context(context, execution) {
        return String::new();
    }

    let topic_hint = topic_hint(execution);
    let tool_names = tool_names(execution);
    let selection = fb2_domain_scenario_selection(context, topic_hint.as_deref(), &tool_names);
    let scenario_lines = selection
        .get("selected_scenarios")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(scenario_prompt_line)
        .collect::<Vec<_>>();
    if scenario_lines.is_empty() {
        return String::new();
    }

    let planned_tools = if tool_names.is_empty() {
        "[]".to_string()
    } else {
        format!("[{}]", tool_names.join(", "))
    };
    let topic_hint = topic_hint.unwrap_or_else(|| "unknown".to_string());
    let context_audit_id = context
        .and_then(|value| value.get("context_audit_id"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("unknown");

    format!(
        "<fb2_domain_scenario_guidance schema=\"fb2.domain_scenario_prompt.v1\" context_audit_id=\"{context_audit_id}\">\n\
         topic_hint={topic_hint}\n\
         planned_tools={planned_tools}\n\
         {}\n\
         <scenario_rules>\n\
         - 回答必须按场景把「数据事实」「用户订单」「平台汇总」「群友观点」「AI推断」「风险边界」分开，缺哪个来源就说明缺失。\n\
         - 只引用 Context Pack citation_sources 或 executed tool source_ids 中真实出现的 ID，不能发明 match_id、order_id、ticket_id、message_id、opinion_memory_id 或 platform_order_summary。\n\
         - 比赛事实、用户订单、平台匿名摘要、群友观点和 AI 推断不能互相冒充；涉及投注或预测时必须说明不保证命中。\n\
         </scenario_rules>\n\
         </fb2_domain_scenario_guidance>",
        scenario_lines.join("\n")
    )
}

pub(crate) fn fb2_domain_scenario_selection(
    context: Option<&Value>,
    topic_hint: Option<&str>,
    tool_names: &[&str],
) -> Value {
    let mut scenario_ids = BTreeSet::new();
    infer_scenarios_from_tools(&mut scenario_ids, tool_names);
    infer_scenarios_from_topic(&mut scenario_ids, topic_hint);
    infer_scenarios_from_context(&mut scenario_ids, context);

    if scenario_ids.is_empty() {
        scenario_ids.insert("source_reference_audit");
    }

    let scenario_matrix = fb2_domain_scenario_matrix();
    let selected_scenarios = scenario_ids
        .iter()
        .filter_map(|id| scenario_metadata(id, &scenario_matrix))
        .collect::<Vec<_>>();

    json!({
        "schema": "fb2.domain_scenario_selection.v1",
        "selected_count": selected_scenarios.len(),
        "selected_scenarios": selected_scenarios
    })
}

fn is_fb2_context(context: Option<&Value>, execution: Option<&Value>) -> bool {
    context.is_some_and(|value| {
        value.get("app_id").and_then(Value::as_str) == Some("fb2")
            || value
                .get("answer_policy")
                .and_then(|policy| policy.get("schema"))
                .and_then(Value::as_str)
                == Some("fb2.answer_policy.v1")
            || value
                .get("context_pack")
                .and_then(Value::as_str)
                .is_some_and(|pack| pack.contains("<fb2_context_pack"))
    }) || execution.is_some_and(|value| value.get("app_id").and_then(Value::as_str) == Some("fb2"))
}

fn topic_hint(execution: Option<&Value>) -> Option<String> {
    execution
        .and_then(|value| value.get("plan"))
        .and_then(|plan| plan.get("topic_hint"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.chars().take(240).collect())
}

fn tool_names(execution: Option<&Value>) -> Vec<&str> {
    let Some(execution) = execution else {
        return Vec::new();
    };

    let mut names = BTreeSet::new();
    if let Some(planned) = execution
        .get("plan")
        .and_then(|plan| plan.get("planned_tools"))
        .and_then(Value::as_array)
    {
        for tool in planned {
            if let Some(name) = tool.get("name").and_then(Value::as_str) {
                names.insert(name);
            }
        }
    }
    if let Some(results) = execution.get("results").and_then(Value::as_array) {
        for result in results {
            if let Some(name) = result.get("tool_name").and_then(Value::as_str) {
                names.insert(name);
            }
        }
    }
    names.into_iter().collect()
}

fn infer_scenarios_from_tools(scenarios: &mut BTreeSet<&'static str>, tool_names: &[&str]) {
    // 工具名是运行时最可靠的信号：它已经经过 planner 和权限裁剪。
    for name in tool_names {
        match *name {
            "match_analysis_brief" | "search_matches" | "get_match_detail" => {
                scenarios.insert("today_matches_analysis");
            }
            "search_user_orders" | "get_order_detail" => {
                scenarios.insert("my_ticket_analysis");
            }
            "platform_orders" => {
                scenarios.insert("platform_order_risk");
            }
            "group_opinion_summary" | "search_group_opinions" | "opinion_memories" => {
                scenarios.insert("group_opinion_summary");
            }
            "opinion_result_review_summary" | "opinion_result_reviews" => {
                scenarios.insert("selected_message_review");
            }
            "get_context_audit" | "context_audit_summary" | "context_feedback_summary" => {
                scenarios.insert("source_reference_audit");
            }
            _ => {}
        }
    }
}

fn infer_scenarios_from_topic(scenarios: &mut BTreeSet<&'static str>, topic_hint: Option<&str>) {
    let Some(topic) = topic_hint else {
        return;
    };

    // topic_hint 负责补齐无工具结果或工具被跳过时的用户意图。
    if contains_any(
        topic,
        &[
            "今天", "比赛", "赛事", "场次", "赔率", "预测", "推荐", "竞彩", "北单",
        ],
    ) {
        scenarios.insert("today_matches_analysis");
    }
    if contains_any(
        topic,
        &["我的票", "我的单", "订单", "票据", "方案", "下单", "串关"],
    ) {
        scenarios.insert("my_ticket_analysis");
    }
    if contains_any(
        topic,
        &["平台", "全站", "匿名汇总", "订单风险", "投注集中", "赔付"],
    ) {
        scenarios.insert("platform_order_risk");
    }
    if contains_any(
        topic,
        &[
            "群友", "大家", "群里", "观点", "讨论", "分歧", "采纳", "记忆",
        ],
    ) {
        scenarios.insert("group_opinion_summary");
    }
    if contains_any(
        topic,
        &[
            "这条消息",
            "这句",
            "这段",
            "对吗",
            "对不对",
            "靠谱吗",
            "合理吗",
        ],
    ) {
        scenarios.insert("selected_message_review");
    }
    if contains_any(
        topic,
        &["依据", "来源", "引用", "source", "context_audit", "审计"],
    ) {
        scenarios.insert("source_reference_audit");
    }
}

fn infer_scenarios_from_context(scenarios: &mut BTreeSet<&'static str>, context: Option<&Value>) {
    let Some(context) = context else {
        return;
    };
    // Context Pack 已有的数据也会影响回答形态，尤其是本人订单已在 pack 中时。
    if array_field_has_items(context, "matches") {
        scenarios.insert("today_matches_analysis");
    }
    if array_field_has_items(context, "user_orders") {
        scenarios.insert("my_ticket_analysis");
    }
    if context
        .get("platform_order_summary")
        .is_some_and(|value| !value.is_null())
    {
        scenarios.insert("platform_order_risk");
    }
    if array_field_has_items(context, "group_messages")
        || array_field_has_items(context, "opinion_memories")
    {
        scenarios.insert("group_opinion_summary");
    }
}

fn array_field_has_items(context: &Value, field: &str) -> bool {
    context
        .get(field)
        .and_then(Value::as_array)
        .is_some_and(|items| !items.is_empty())
}

fn contains_any(text: &str, needles: &[&str]) -> bool {
    let lower = text.to_ascii_lowercase();
    needles.iter().any(|needle| {
        let needle_lower = needle.to_ascii_lowercase();
        lower.contains(&needle_lower) || text.contains(*needle)
    })
}

fn scenario_metadata(id: &str, matrix: &Value) -> Option<Value> {
    let scenario = matrix.as_array()?.iter().find(|item| item["id"] == id)?;
    let guidance = scenario_guidance(id)?;
    Some(json!({
        "id": id,
        "permission_scope": scenario["permission_scope"],
        "primary_tools": scenario["primary_tools"],
        "required_citations": scenario["required_citations"],
        "forbidden_outputs": scenario["forbidden_outputs"],
        "guidance": guidance
    }))
}

fn scenario_prompt_line(scenario: &Value) -> Option<String> {
    let id = scenario.get("id").and_then(Value::as_str)?;
    Some(format!(
        "- scenario={id}：{} required_citations={} forbidden_outputs={}",
        scenario.get("guidance").and_then(Value::as_str)?,
        compact_json_array(&scenario["required_citations"]),
        compact_json_array(&scenario["forbidden_outputs"])
    ))
}

fn scenario_guidance(id: &str) -> Option<&'static str> {
    let guidance = match id {
        "today_matches_analysis" => "使用 match_analysis_brief/search_matches 和 match/odds 来源回答“今天比赛怎么看”；不得编造赔率或承诺命中。",
        "my_ticket_analysis" => "只使用 current_user_only 的 user_order/ticket 来源分析“我的票”；缺订单就说明当前上下文没有本人票据。",
        "platform_order_risk" => "只使用 anonymous_aggregate_only 的 platform_order_summary；可以讲平台集中度和风险，不得暴露单个用户订单、身份或下注明细。",
        "group_opinion_summary" => "把 group_message/opinion_memory 标成群友观点；采纳观点时说明只是群友观点输入，不是比赛事实。",
        "selected_message_review" => "围绕 selected_message_id 复核这条消息；只能判断已由比赛/群观点/复盘来源支持的部分，遇到稳赢、包赢、重注、梭哈等说法必须指出风险。",
        "source_reference_audit" => "回答“依据了哪些来源”时只列当前 Context Pack、工具结果和 feedback 中真实存在的来源 ID；没有的来源明确说没有，不得补造。",
        _ => return None,
    };
    Some(guidance)
}

fn compact_json_array(value: &Value) -> String {
    value
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join("/")
        })
        .filter(|text| !text.is_empty())
        .unwrap_or_else(|| "none".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn emits_ticket_match_and_group_guidance_from_plan() {
        let context = json!({
            "app_id": "fb2",
            "context_audit_id": "audit-1",
            "answer_policy": {"schema": "fb2.answer_policy.v1"}
        });
        let execution = json!({
            "app_id": "fb2",
            "plan": {
                "topic_hint": "今天比赛怎么看，帮我分析我的票和群友观点",
                "planned_tools": [
                    {"name": "match_analysis_brief"},
                    {"name": "search_user_orders"},
                    {"name": "opinion_memories"}
                ]
            },
            "results": []
        });

        let block = prompt_domain_scenario_guidance(Some(&context), Some(&execution));

        assert!(block.contains("fb2.domain_scenario_prompt.v1"));
        assert!(block.contains("scenario=today_matches_analysis"));
        assert!(block.contains("scenario=my_ticket_analysis"));
        assert!(block.contains("scenario=group_opinion_summary"));
        assert!(block.contains("current_user_only"));
        assert!(block.contains("order_id/ticket_id/match_id"));
    }

    #[test]
    fn returns_machine_readable_selection_for_planner_metadata() {
        let context = json!({
            "app_id": "fb2",
            "user_orders": [{"order_id": "order-1"}],
            "matches": [{"match_id": "match-1"}]
        });
        let selection = fb2_domain_scenario_selection(
            Some(&context),
            Some("帮我分析我的票"),
            &["match_analysis_brief"],
        );

        assert_eq!(
            selection["schema"].as_str(),
            Some("fb2.domain_scenario_selection.v1")
        );
        let selected = selection["selected_scenarios"].as_array().unwrap();
        assert!(selected
            .iter()
            .any(|scenario| scenario["id"] == "my_ticket_analysis"));
        let ticket = selected
            .iter()
            .find(|scenario| scenario["id"] == "my_ticket_analysis")
            .unwrap();
        assert_eq!(
            ticket["permission_scope"].as_str(),
            Some("current_user_only")
        );
        assert!(ticket["required_citations"]
            .as_array()
            .unwrap()
            .contains(&json!("order_id")));
    }

    #[test]
    fn emits_platform_guidance_without_user_detail_leak() {
        let execution = json!({
            "app_id": "fb2",
            "plan": {
                "topic_hint": "平台今天订单风险怎么样",
                "planned_tools": [{"name": "platform_orders"}]
            },
            "results": []
        });

        let block = prompt_domain_scenario_guidance(None, Some(&execution));

        assert!(block.contains("scenario=platform_order_risk"));
        assert!(block.contains("anonymous_aggregate_only"));
        assert!(block.contains("不得暴露单个用户订单"));
    }

    #[test]
    fn ignores_non_fb2_context() {
        let context = json!({"app_id": "other"});

        assert!(prompt_domain_scenario_guidance(Some(&context), None).is_empty());
    }
}
