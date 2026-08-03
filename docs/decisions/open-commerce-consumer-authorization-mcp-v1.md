# 消费者 AI 单能力授权申请 MCP V1

状态：已接受；代码已形成，尚未编译或测试。

## 决策

新增 MCP 工具 `open_commerce_request_consumer_authorization`，用于消费者 AI 在执行计划返回 `authorization_request_required` 或 `grant_refresh_required` 后，向商户提交一项最小能力授权或续额申请。

调用必须同时满足：

- MCP 使用显式 `x-elon-app-id`，且该 App 已注册并属于当前用户；默认 `mcp-client` 身份失败关闭。
- 一次只申请一个已发布的 `authorized` 能力。
- 用户已明确同意，并提交固定确认短语 `REQUEST_AUTHORIZATION`。
- 用途为 3 至 200 个字符；任一现有 Grant 仍能覆盖下一次调用时拒绝重复申请。
- 现有 Grant 全部因次数、金额或币种不能覆盖下一次调用时允许申请新授权，但不修改旧 Grant。
- 相同待审批范围和用途复用原请求，不创建第二条待审批记录。

工具写入现有授权申请和审计真源，不建立 MCP 专属授权账本。

## 权力边界

提交成功只表示 `pending`。商户仍独立决定批准或拒绝，以及新 Grant 的有效期、总调用次数和总预算；消费者 AI 不能自批或直接扩权。所谓续额是重新申请和重新审批，不是自动修改旧 Grant。工具不调用能力、不创建订单、不扣款。

## 实现入口

- `server/src/open_commerce_consumer_discovery_mcp.rs`
- `server/src/open_commerce_consumer_execution_plan.rs`
- `server/src/open_commerce_grant_readiness.rs`
- `server/src/store/open_commerce_grants.rs`
- `server/src/store/open_commerce_authorization_requests.rs`
- `docs/open-commerce-consumer-authorization-mcp-v1-acceptance.md`
