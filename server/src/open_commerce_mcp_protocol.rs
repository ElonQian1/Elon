//! MCP initialization metadata kept separate from request routing.

use serde_json::{json, Value};

const MCP_PROTOCOL_VERSION: &str = "2025-03-26";

pub(crate) fn initialize_response() -> Value {
    json!({
        "protocolVersion":MCP_PROTOCOL_VERSION,
        "capabilities":{"tools":{"listChanged":false}},
        "serverInfo":{"name":"yilong-open-commerce","version":"1.0.0"},
        "instructions":"开放商业任务先调用 open_commerce_get_overview；消费者 AI 可用 open_commerce_list_my_consumer_apps 查看本人可选 App，但工具不返回任何 Token。使用 open_commerce_discover_for_consumer 获取透明排序、候选范围和授权状态，选择能力后先调用 open_commerce_plan_consumer_capability 校验输入并读取下一步。两者都不会自动调用或下单。需授权能力只有在用户明确同意并使用本人已注册 App 时，才能调用 open_commerce_request_consumer_authorization 提交单能力申请；商户仍独立决定。可用 open_commerce_list_my_consumer_authorization_requests 跟踪本人申请，用 open_commerce_list_my_active_grants 查看未撤销未过期 Grant 及剩余额度；实际调用前仍须执行计划。撤回 pending 申请必须由项目编辑者明确确认后调用 open_commerce_cancel_my_consumer_authorization_request，且不会撤销已批准 Grant。动作确认可在用户同意前用 open_commerce_get_my_action_confirmation 重新核对状态和输入形状，但服务端不返回原始输入值。商户只有显式发布目录后才会进入跨项目发现。ERP 开发先调用 erp_get_overview、erp_search_capabilities 和 erp_resolve_requirement，避免重复造轮子。ERP 工具不允许接受提案、创建 Matter、合并、发布、采用或回滚。数据接入记录不包含令牌，公开发现只返回脱敏目录契约；授权能力必须携带 grant_id 并使用已注册应用身份；所有调用和同步回执必须使用幂等键。商户可手动封禁已注册 App，封禁会撤销现有授权且解除后不会恢复。当前只记录计量，不真实扣款。写操作需要当前项目编辑权限，调用身份由 x-elon-app-id 固定，不能由工具参数冒充。"
    })
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
