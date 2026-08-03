---
title: 开放商业 Webhook 死信人工确认 V1
status: accepted
owner: backend
reviewed_at: 2026-08-03
implementation_status: implementation_uncompiled
---

# 开放商业 Webhook 死信人工确认 V1

## 背景

Webhook 投递达到自动重试上限后会进入死信。部分失败需要修复接收端后重新投递，另一些失败已经通过线下补偿、人工对账或业务放弃处理。若所有历史死信永久保持待处置状态，健康摘要会持续告警；若直接删除或覆盖失败记录，又会破坏审计证据。

## 决定

1. 只有 `dead` 状态的投递可以人工确认。确认不删除投递、不改变原事件、投递状态、失败码、尝试次数或业务调用结果。
2. 确认必须记录确认时间、项目操作人和 4 至 500 字符的处理原因。数据库投递记录是确认事实真源，项目审计作为补充记录。
3. 同一操作人以同一原因重复确认返回原结果；已由其他操作人确认或原因不同的请求拒绝覆盖，避免改写原处理证据。
4. 确认需要项目编辑权限和 App 所有权，但不要求 App、订阅或生产凭据当前仍然有效，使凭据撤销或资格失效后遗留的死信仍可收口。
5. 健康摘要分别统计待处理死信和已确认死信。只有待处理死信触发 `action_required`，已确认记录继续保留并可见。
6. 人工重试仍复用既有订阅验证、环境和生产资格规则；重试成功进入队列时原子清除确认字段，使新的投递轮次重新接受健康监控。
7. 确认只表达“当前不再重试这条通知”，不代表接收端成功、订单完成、支付成功、ERP 入库、赔付完成或争议已经解决。

## 边界

- V1 不提供批量确认、确认撤销、附件、审批流、外部工单同步、超时自动确认或自动责任认定。
- V1 不改变 Webhook 自动重试上限、订阅自动停用、历史补发和生产资格规则。
- 当前代码尚未编译、执行迁移、运行接口、验证并发确认或检查 PC 页面。

## 实现引用

- `server/src/open_commerce_webhook_dead_letter_migration.rs`
- `server/src/open_commerce_webhook_dead_letter_api.rs`
- `server/src/store/open_commerce_developer_webhook_dead_letters.rs`
- `server/src/store/open_commerce_developer_webhook_replays.rs`
- `server/src/store/open_commerce_developer_webhook_health.rs`
- `pc-frontend/src/features/open-commerce/DeveloperWebhookDeadLetterActions.tsx`
- `docs/open-commerce-webhook-dead-letter-acknowledgement-v1-acceptance.md`
