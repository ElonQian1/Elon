# 消费者本人授权状态 MCP V1

状态：已接受；代码已形成，尚未编译或测试。

## 决策

新增只读工具 `open_commerce_list_my_consumer_authorization_requests`，让消费者 AI 查看当前用户通过当前项目内本人 App 发出的授权申请。

查询同时绑定 `requester_user_id` 和当前项目中由该用户拥有的 App，可按 `pending`、`approved`、`rejected`、`canceled` 精确筛选，最多返回 100 条。响应保留商户 ID、App ID、能力范围、用途、状态、商户说明，以及批准后 Grant 的 ID、期限、总次数和总预算。

## 隔离边界

响应不包含其他项目成员的申请、商户项目 ID、申请人内部用户 ID 或审核人内部用户 ID。工具不返回 App Token 或生产凭据。

列表只读，不重提、批准、拒绝或调用能力；`approved` 仍需结合 Grant 当前有效性和执行计划复核，不能仅凭历史状态假定仍可调用。

## 本人申请撤回

项目编辑者可在当前用户明确同意并提交固定短语 `CANCEL_AUTHORIZATION_REQUEST` 后，调用 `open_commerce_cancel_my_consumer_authorization_request` 撤回当前项目中由本人 App 发出的 pending 请求。服务端先核对请求的 `requester_user_id`，再复用现有项目侧取消和消费者/商户双侧审计。

非 pending 请求保持原状态；该工具尤其不会撤销已批准 Grant。需要停止已有 Grant 时仍走商户授权撤销流程。

## 实现入口

- `server/src/open_commerce_consumer_authorization_mcp.rs`
- `server/src/store/open_commerce_authorization_requests.rs`
- `docs/open-commerce-consumer-authorization-status-mcp-v1-acceptance.md`
