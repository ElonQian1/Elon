//! Contract normalization for external app business context.

use serde_json::{json, Value};

use crate::{
    external_app_context_answer_policy::default_answer_policy,
    external_app_context_quality::context_quality, external_app_usage_policy::default_usage_policy,
};

pub(crate) fn fb2_pack_context(app_id: &str, external_group_id: &str, data: Value) -> Value {
    let mut context = json!({
        "app_id": app_id,
        "group": external_group_id,
        "status": "ready",
        "source": "fb2:/api/main-project/context/pack",
        "generated_at": data.get("generated_at"),
        "context_audit_id": data.get("context_audit_id"),
        "context_pack_version": data.get("context_pack_version").or_else(|| data.get("version")),
        "context_pack": data.get("context_pack"),
        "matches": data.get("matches"),
        "user_orders": data.get("user_orders"),
        "group_messages": data.get("group_messages"),
        "platform_order_summary": data.get("platform_order_summary"),
        "citation_sources": data.get("citation_sources"),
        "metrics": data.get("metrics"),
        "tool_contract": data.get("tool_contract"),
        "usage_policy": data.get("usage_policy").cloned().unwrap_or_else(default_usage_policy),
        "answer_policy": data.get("answer_policy").cloned().unwrap_or_else(default_answer_policy)
    });
    context["context_quality"] = context_quality(&context, true);
    context
}

pub(crate) fn fb2_match_context(app_id: &str, external_group_id: &str, data: Value) -> Value {
    let matches = data["matches"]
        .as_array()
        .map(|matches| matches.iter().map(slim_match).collect::<Vec<_>>())
        .unwrap_or_default();
    let mut context = json!({
        "app_id": app_id,
        "group": external_group_id,
        "status": "ready",
        "source": "fb2:/api/main-project/context/today-matches",
        "generated_at": data.get("generated_at"),
        "count": matches.len(),
        "matches": matches,
        "usage_policy": default_usage_policy(),
        "answer_policy": default_answer_policy()
    });
    context["context_quality"] = context_quality(&context, false);
    context
}

fn slim_match(raw: &Value) -> Value {
    json!({
        "id": raw.get("id"),
        "lottery_type": raw.get("lottery_type"),
        "league": raw.get("league"),
        "home_team": raw.get("home_team"),
        "away_team": raw.get("away_team"),
        "match_time": raw.get("match_time"),
        "status": raw.get("status"),
        "odds": raw.get("odds"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn match_context_slims_match_payload_and_marks_schema() {
        let context = fb2_match_context(
            "fb2",
            "official",
            json!({
                "generated_at": "2026-06-20T16:00:00+08:00",
                "matches": [{
                    "id": "m1",
                    "league": "J1",
                    "home_team": "A",
                    "away_team": "B",
                    "internal_field": "hidden"
                }]
            }),
        );

        assert_eq!(context["context_quality"]["schema"], "fb2.today_matches.v1");
        assert!(context["matches"][0].get("internal_field").is_none());
        assert_eq!(context["count"], 1);
        assert_eq!(context["answer_policy"]["schema"], "fb2.answer_policy.v1");
    }

    #[test]
    fn pack_context_promotes_budget_status_to_quality_warning() {
        let context = fb2_pack_context(
            "fb2",
            "official",
            json!({
                "generated_at": "2026-06-20T16:00:00+08:00",
                "context_pack_version": "fb2-chat-pack-v1",
                "context_pack": "<fb2_context_pack>large</fb2_context_pack>",
                "matches": [{"id": "m1"}],
                "tool_contract": {"tools": [{"name": "get_match_detail"}]},
                "metrics": {"budget_status": "too_large"}
            }),
        );

        let warnings = context["context_quality"]["warnings"].as_array().unwrap();
        assert!(warnings.contains(&json!("fb2_budget_too_large")));
        assert_eq!(context["answer_policy"]["schema"], "fb2.answer_policy.v1");
    }
}
