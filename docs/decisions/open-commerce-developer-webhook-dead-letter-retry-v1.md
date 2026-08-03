---
title: 开放商业开发者 Webhook 死信人工重试 V1
status: accepted
owner: backend
reviewed_at: 2026-08-03
implementation_status: implementation_uncompiled
---

# 开放商业开发者 Webhook 死信人工重试 V1

## 背景

Webhook 自动重试达到上限后会进入死信。接收端修复故障后，开发者需要恢复这条通知，但不能创建第二条业务事件、重复订单或绕过订阅验证状态。

## 决定

1. V148 为投递增加独立的 `manual_retry_count` 和 `last_manual_retry_at`，保留人工重试轮次，不用累计的自动尝试次数冒充完整历史。
2. 只有开发者 App 所有者所在项目的编辑者可以显式重试，且 App 与订阅都必须启用，订阅端点已经验证。
3. 只允许状态为 `dead` 的同一投递重新进入 `pending`。已成功投递、自动重试中或已被工作器领取的记录拒绝操作。
4. 人工重试不创建新投递 ID，不创建新终态事件，也不执行订单、支付、退款或 ERP 写入。接收端必须继续按投递 ID 和调用 ID 幂等处理。
5. 重新排队会清除上一轮的临时响应与错误、重置本轮自动尝试次数，并增加人工重试轮次。事务条件防止两个操作员同时重试同一死信。

## 实现引用

- `server/src/open_commerce_webhook_replay_migration.rs`
- `server/src/store/open_commerce_developer_webhook_replays.rs`
- `server/src/open_commerce_webhook_api.rs`
- `pc-frontend/src/features/open-commerce/DeveloperWebhookPanel.tsx`
- `docs/open-commerce-developer-webhook-dead-letter-retry-v1-acceptance.md`
