use super::*;
use serde_json::json;

#[test]
fn rendered_tool_block_marks_success_as_required_for_facts() {
    let block = prompt_executed_tools_block(Some(&json!({
        "schema": "external_app.executed_tools.v1",
        "app_id": "fb2",
        "status": "ready",
        "executed_at": "2026-06-21T00:00:00Z",
        "results": [{
            "tool_name": "search_matches",
            "status": "ready",
            "success": true,
            "grounding": {"status": "grounded"}
        }]
    })));

    assert!(block.contains("<executed_external_app_tools"));
    assert!(block.contains("success=true"));
    assert!(block.contains("grounding.status=grounded"));
    assert!(block.contains("不能编造"));
    assert!(block.contains("来源 ID"));
    assert!(block.contains("历史观点记忆"));
    assert!(block.contains("组合简报"));
    assert!(block.contains("match_focused_brief.data.user_orders"));
    assert!(block.contains("不能否定已有本人订单事实"));
    assert!(block.contains("历史赛后复盘"));
}

#[test]
fn rendered_tool_block_summarizes_user_orders_before_truncation() {
    let block = prompt_executed_tools_block(Some(&json!({
        "schema": "external_app.executed_tools.v1",
        "app_id": "fb2",
        "status": "partial",
        "executed_at": "2026-06-21T00:00:00Z",
        "results": [{
            "tool_name": "match_analysis_brief",
            "status": "ready",
            "success": true,
            "visibility": "match_focused_brief",
            "data": {
                "matches": [{"id": "match-1", "odds": "x".repeat(20_000)}],
                "user_orders": [{
                    "order_id": "order-1",
                    "status": "pending",
                    "total_amount": 54,
                    "bet_slips": [{
                        "home_team": "主队",
                        "away_team": "客队",
                        "selection": "主胜",
                        "odds": 1.96
                    }]
                }]
            },
            "source_ids": ["match-1", "order-1"],
            "grounding": {"status": "grounded"}
        }]
    })));

    let summary_index = block.find("<tool_fact_summary>").unwrap();
    let truncated_index = block.find("[external app tool results truncated]").unwrap();
    assert!(summary_index < truncated_index);
    assert!(block.contains("current_user_order_count=1"));
    assert!(block.contains("order_id=order-1"));
    assert!(block.contains("first_slip=主队 vs 客队 主胜 odds=1.96"));
    assert!(block.contains("可用于“我的票”分析"));
}

#[test]
fn rendered_tool_block_surfaces_skipped_readiness_gap_before_body() {
    let block = prompt_executed_tools_block(Some(&json!({
        "schema": "external_app.executed_tools.v1",
        "app_id": "fb2",
        "status": "skipped",
        "executed_at": "2026-06-22T00:00:00Z",
        "results": [{
            "tool_name": "match_analysis_brief",
            "status": "skipped",
            "reason": "fb2_readiness_blocked"
        }]
    })));

    let summary_index = block.find("<tool_gap_summary>").unwrap();
    let body_index = block
        .find("\"schema\": \"external_app.executed_tools.v1\"")
        .unwrap();
    assert!(summary_index < body_index);
    assert!(block.contains("status=skipped"));
    assert!(block.contains("fb2_readiness_blocked"));
    assert!(block.contains("这只是数据缺口"));
    assert!(block.contains("不能编造成比赛、赔率、订单或群友观点事实"));
}
