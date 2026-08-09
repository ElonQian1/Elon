# 消费者 AI 单能力授权申请 MCP V1 验收

状态：`verified_local`

## 已实现并验证

- 默认 MCP 身份、非本人 App、错误确认短语和仍能覆盖下一次调用的已有 Grant 失败关闭。
- 单能力范围进入既有授权申请服务和审计账本。
- 相同待审批范围及用途保持既有幂等语义。
- 执行计划把申请工具标为需要用户明确确认的下一步。
- 现有 Grant 全部预算不足或币种不匹配时允许重新申请，不改写旧 Grant。
- 成功响应复用消费者授权状态安全投影，不返回商户项目、申请人、决定人、Token 或其他内部身份。
- 相同范围和用途的顺序重放复用一条 pending 申请，并只形成一次 `authorization.requested` 审计。

## 已通过的定向回归

- `definition_and_identity_confirmation_guards_fail_closed_without_writes`
- `first_request_routes_through_mcp_replays_once_and_returns_safe_projection`
- `public_missing_unpublished_disabled_and_blocked_targets_fail_closed`
- `any_available_grant_blocks_duplicate_requests_including_free_calls`
- `exhausted_or_currency_mismatched_grants_allow_pending_refresh_without_mutation`
- `a_different_pending_request_cannot_be_silently_rewritten`

授权申请验证通过，指纹：`ea884f1aeab544d9c08927a60708bbc7639793ccaed3005a7b541ed8de727837`。执行计划回归同时通过，指纹：`6aad093dd31dee62a2a619f3521c4726bbf27255aae62f8b0207ed2e6ed9e67a`。

## 尚未宣称

- 未完成并发首次申请的多连接竞争测试或存储级唯一性证明。
- 未连接真实外部平台、订单履约、支付、链上资产或生产部署。
- 本验收证明本地授权申请合同、最小响应和顺序幂等边界，不代表商户已经批准或扩充 Grant。
