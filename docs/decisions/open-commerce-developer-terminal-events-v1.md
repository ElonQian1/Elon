---
title: 开放商业开发者终态调用事件流 V1
status: accepted
owner: backend
reviewed_at: 2026-08-03
---

# 开放商业开发者终态调用事件流 V1

## 背景

开发者 App 已能通过测试 Token 同步调用商户能力，但网络超时、客户端重启或响应丢失后，只能依赖原幂等请求重放，缺少一个可以持续追读本 App 调用结果的入口。直接开放项目审计会泄露其他 App、原始请求摘要和内部授权标识；直接实现任意 Webhook 又会引入回调地址认证、SSRF、签名密钥、重试和死信治理，不能在这些边界未完成时冒充生产通知。

## 决定

1. V1 提供测试 Token 认证的终态调用事件列表和单条详情。App 身份完全由 Token 派生，不接受查询参数或请求头另行指定 App。
2. 事件只来自已有 Invocation 从 `started` 进入 `succeeded` 或 `failed` 的终态转换，不创建第二套调用、订单或结算状态机。
3. V134 追加只增不改的终态序号表。数据库触发器在终态更新或终态插入时原子登记一次，已有终态调用按完成时间回填；唯一约束阻止重复事件。
4. 列表按全局终态序号升序读取，但查询同时绑定 Token 所属用户和 App。游标包含版本、App ID 和最后序号，跨 App、非正数或不可解析游标失败关闭。
5. 列表默认 20 条、最多 100 条，只返回状态、外部商户与能力标识、调用方自己的幂等键、计量和完成时间；不返回结果正文。即使当前没有新事件，也返回原检查点，客户端可继续轮询。
6. 单条详情只有同一 App 和所有者可读，找不到与不属于当前 App 使用相同语义。详情可返回该 App 原本已经获得的商户结果。
7. 事件不公开原始输入、请求形状、请求哈希、Grant ID、能力内部 ID、项目 ID 或用户 ID。资金状态继续固定显示平台记录，不把 `recorded_not_charged` 解释为真实扣款。
8. V1 是可恢复的拉取事件流，不是外部 Webhook、消息推送、跨运营方事件总线或支付回调。未来主动推送必须复用同一终态序号并另行完成目标验证、签名、重试、速率限制和 SSRF 安全评审。

## HTTP 契约

- `GET /api/open-commerce/developer/events?cursor=&limit=`：读取当前 App 后续终态摘要。
- `GET /api/open-commerce/developer/events/:invocation_id`：读取当前 App 的单条终态结果。
- 两个接口都使用 `Authorization: Bearer <test-token>`；停用、轮换或重新启用 App 后，旧 Token 不能读取事件。

## 实现引用

- `server/src/open_commerce_developer_event_api.rs`
- `server/src/open_commerce_developer_event_service.rs`
- `server/src/store/open_commerce_developer_events.rs`
- `server/src/open_commerce_developer_event_migration.rs`
- `server/src/open_commerce_developer_event_tests.rs`
