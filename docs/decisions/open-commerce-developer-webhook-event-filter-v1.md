---
title: 开放商业开发者 Webhook 终态事件筛选 V1
status: accepted
owner: backend
reviewed_at: 2026-08-03
implementation_status: implementation_uncompiled
---

# 开放商业开发者 Webhook 终态事件筛选 V1

## 背景

开发者 App 不一定需要接收所有终态事件。只处理异常告警的接收端如果同时接收成功事件，会产生无效网络、签名校验、存储和重试成本。

## 决定

1. V149 为每个订阅增加“调用成功”和“调用失败”两个布尔筛选项，至少选择一种。既有订阅迁移后默认两项都开启，保持原行为。
2. 筛选在终态事件触发器创建投递之前执行。未订阅的事件不生成投递记录，不进入租约、自动重试或死信流程。
3. V1 的事件筛选在订阅创建时固定。需要修改筛选时创建并验证新订阅，再停用旧订阅，避免原地扩展范围时产生不明确的历史回放边界。
4. 筛选只控制通知事件，不改变开发者 App 通过终态结果接口读取自身调用结果的权限。
5. 本能力不是内容级规则引擎、动态路由、批量历史回放或第三方消息总线。

## 实现引用

- `server/src/open_commerce_webhook_event_filter_migration.rs`
- `server/src/store/open_commerce_developer_webhooks.rs`
- `server/src/store/open_commerce_developer_webhook_rows.rs`
- `pc-frontend/src/features/open-commerce/DeveloperWebhookPanel.tsx`
- `docs/open-commerce-developer-webhook-event-filter-v1-acceptance.md`
