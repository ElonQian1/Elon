# 消费者能力执行计划 V1 验收

状态：`verified_local`

## 已实现并验证

- 只读复核公开目录、拟调用输入、App 所有权、封禁和授权状态。
- 公开/受限与 query/action 组合映射为六种稳定 readiness。
- 动作能力返回准备、用户明确确认、确认和调用四个步骤。
- 现有 Grant 和待审批申请只读回显，不自动推进状态。
- 计划按下一次调用的次数、单位价格和币种检查全部有效 Grant，并返回选中 Grant 与稳定预算原因。
- 历史批准申请对应的 Grant 已撤销或过期时不会被误判为可调用；被商户封禁的 App 在 Grant 选择前失败关闭。
- 重复直接规划和真实 MCP 路由规划均不增加授权、Grant、动作确认、Invocation、预算预留或审计记录，也不修改 Grant 用量和更新时间。

## 已通过的定向回归

- `public_and_default_identity_and_action_steps_are_stable`
- `authorization_lifecycle_requires_a_current_active_grant`
- `blocked_or_unowned_apps_fail_closed_before_grant_selection`
- `grant_selection_checks_all_active_grants_and_reports_stable_budget_reasons`
- `invalid_input_disabled_app_unknown_capability_and_unpublished_merchant_fail_closed`
- `repeated_query_and_action_planning_create_no_side_effects_or_budget_changes`

Rust 验证通过，指纹：`3335d4229ac0bd652c57e8e14047316790f7d7cea026839cc4bd5d82c1806e58`。

## 尚未宣称

- 未连接美团、抖音、京东或淘宝闪购等真实生产适配器。
- 未发生真实支付、订单履约、链上提交或生产部署。
- 本验收证明本地执行计划合同和持久化边界，不证明外部商业系统已可用。
