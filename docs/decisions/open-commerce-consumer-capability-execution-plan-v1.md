# 消费者能力执行计划 V1

状态：已接受；代码已形成，尚未编译或测试。

## 决策

消费者 AI 在发现能力后，应先调用只读工具 `open_commerce_plan_consumer_capability`，用实际拟调用输入复核公开目录、能力契约、App 身份、封禁状态和授权状态，再决定下一步。

服务端返回以下稳定 `readiness`：

- `invoke_ready`：查询能力已具备调用条件。
- `action_confirmation_required`：动作能力必须依次准备确认、向用户展示并取得明确同意、确认、调用。
- `app_registration_required`：默认 MCP 身份不能调用需授权能力。
- `authorization_request_required`：注册 App 尚无 Grant 或待审批申请。
- `authorization_pending`：已有待商户决定的授权申请。

计划可返回当前有效 `grant_id` 或待审批 `authorization_request_id`，并提供有序下一步；参数不能覆盖 MCP 入口固定的 App 身份。

## 副作用边界

计划固定 `side_effects_created=false`。它不创建授权申请、Grant、动作确认、Invocation、计量、订单或结算记录，也不占用调用次数和预算。输入无效、能力未公开、App 不属于当前用户或已被封禁时失败关闭。

## 实现入口

- `server/src/open_commerce_consumer_execution_plan.rs`
- `server/src/open_commerce_consumer_discovery_mcp.rs`
- `docs/open-commerce-consumer-capability-execution-plan-v1-acceptance.md`
