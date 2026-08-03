---
title: 开放商业开发者 Webhook 有界历史补发 V1
status: accepted
owner: backend
reviewed_at: 2026-08-03
implementation_status: implementation_uncompiled
---

# 开放商业开发者 Webhook 有界历史补发 V1

## 背景

开发者从轮询切换到 Webhook，或接收端停机时间超过自动重试窗口时，可能需要补回已经存在于终态事件真源中的通知。补发必须有边界、可重复调用且不能被误解为重新执行商业动作。

## 决定

1. App 所有者可对已验证且启用的订阅，从指定终态序号之后显式补发最多 100 条历史事件。
2. 查询只读取该 App 所有者和 `app_id` 的终态事件，并继续执行订阅创建时固定的成功/失败事件筛选。
3. 历史通知沿用 `subscription_id + invocation_id` 的确定性投递 ID。已存在的投递使用 `INSERT OR IGNORE` 跳过，重复请求不会创建第二条通知。
4. V150 为投递记录来源 `live` 或 `history_replay`，并保存历史补发请求时间。结果返回处理到的序号、符合条件数、新入队数、已存在数和是否还有下一批。
5. 历史补发只恢复通知，不修改终态事件，不重做能力调用、订单、支付、退款、结算或 ERP 写入。接收端仍须按投递 ID 和调用 ID 幂等处理。

## 实现引用

- `server/src/open_commerce_webhook_history_migration.rs`
- `server/src/store/open_commerce_developer_webhook_history.rs`
- `server/src/open_commerce_webhook_api.rs`
- `pc-frontend/src/features/open-commerce/DeveloperWebhookPanel.tsx`
- `docs/open-commerce-developer-webhook-history-replay-v1-acceptance.md`
