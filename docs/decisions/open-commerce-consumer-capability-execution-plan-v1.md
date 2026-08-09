# 消费者能力执行计划 V1

状态：已接受；代码已实现并完成 Rust、SQLite 假数据与 MCP 路由验收。

## 决策

消费者 AI 在发现能力后，应先调用只读工具 `open_commerce_plan_consumer_capability`，用实际拟调用输入复核公开目录、能力契约、App 身份、封禁状态和授权状态，再决定下一步。

服务端返回以下稳定 `readiness`：

- `invoke_ready`：查询能力已具备调用条件。
- `action_confirmation_required`：动作能力必须依次准备确认、向用户展示并取得明确同意、确认、调用。
- `app_registration_required`：默认 MCP 身份不能调用需授权能力。
- `authorization_request_required`：注册 App 尚无 Grant 或待审批申请。
- `authorization_pending`：已有待商户决定的授权申请。
- `grant_refresh_required`：存在未撤销、未过期的 Grant，但没有任何一张能覆盖下一次调用次数、单位价格或币种要求，需要申请新的期限或预算。

计划可返回当前选中的 `grant_id`、`grant_budget_status` 或待审批 `authorization_request_id`，并提供有序下一步；参数不能覆盖 MCP 入口固定的 App 身份。预算状态固定为 `available`、`invocation_budget_exhausted`、`amount_budget_exhausted` 或 `budget_currency_mismatch`。

同一能力存在多张有效 Grant 时，计划按创建时间从新到旧检查，但优先选择任意一张能覆盖下一次调用的 Grant；不能仅因最新 Grant 耗尽而忽略仍可用的旧 Grant。全部不可用且已有续额申请时返回 `authorization_pending`，避免重复申请。

## 副作用边界

计划固定 `side_effects_created=false`。它不创建授权申请、Grant、动作确认、Invocation、计量、订单或结算记录，也不占用调用次数和预算。输入无效、能力未公开、App 不属于当前用户或已被封禁时失败关闭。

历史上曾批准的授权申请不代表当前仍可调用。规划只接受未撤销、未过期且能覆盖下一次调用的当前 Grant；对应 Grant 已撤销或过期时，必须重新申请授权。商户封禁 App 后，即使数据库中仍存在历史批准记录，规划也会在 Grant 选择前失败关闭。

## 验证结论

定向测试已覆盖真实 SQLite Store 与 MCP `tools/call` 路由，包括公开/授权、query/action、默认/注册 App、输入 Schema、未发布商户、非本人/停用/封禁 App、待审批与历史批准申请、撤销/过期 Grant、次数/金额/币种预算、多 Grant 选择、零价调用和无副作用快照。测试不代表真实外部平台适配器、真实支付或生产部署已经验收。

## 实现入口

- `server/src/open_commerce_consumer_execution_plan.rs`
- `server/src/open_commerce_grant_readiness.rs`
- `server/src/store/open_commerce_grants.rs`
- `server/src/open_commerce_consumer_discovery_mcp.rs`
- `docs/open-commerce-consumer-capability-execution-plan-v1-acceptance.md`
