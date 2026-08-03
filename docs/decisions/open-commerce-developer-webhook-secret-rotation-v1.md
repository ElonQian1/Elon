---
title: 开放商业开发者 Webhook 签名密钥轮换 V1
status: accepted
owner: backend
reviewed_at: 2026-08-03
implementation_status: implementation_uncompiled
---

# 开放商业开发者 Webhook 签名密钥轮换 V1

## 背景

Webhook 密钥只在创建时显示一次，但接收端泄露、人员变更和日常安全治理都要求能够废弃旧密钥。直接创建新订阅会改变订阅身份，也容易遗漏旧订阅下的待投递事件。

## 决定

1. V147 为每个订阅增加从 1 开始的 `signing_secret_version`。版本 1 保留原有派生消息，避免升级后改变已经展示的密钥；后续版本把版本号纳入 HMAC 派生消息。
2. 只有开发者 App 所有者所在项目的编辑者可以显式轮换。轮换先在内存中计算下一版本的一次性密钥，再以当前版本为并发条件完成数据库更新。
3. 轮换原子停用订阅、把验证状态重置为待验证，并将旧密钥下尚未完成的投递标为死信。接收端保存新密钥并重新完成 challenge 后，才从新的终态序号继续投递。
4. 轮换不会修改已成功投递记录，也不会自动重放旧事件。一次性密钥返回后，即使附加审计写入失败也不能隐藏响应；服务端记录告警，避免订阅永久丢失新密钥。
5. 本能力不是自动密钥托管、跨运营方证书互认或零停机双密钥窗口。需要零停机切换时应并行创建并验证新订阅，再停用旧订阅。

## 实现引用

- `server/src/open_commerce_webhook_lifecycle_migration.rs`
- `server/src/store/open_commerce_developer_webhook_secret.rs`
- `server/src/open_commerce_webhook_security.rs`
- `server/src/open_commerce_webhook_api.rs`
- `pc-frontend/src/features/open-commerce/DeveloperWebhookPanel.tsx`
- `docs/open-commerce-developer-webhook-secret-rotation-v1-acceptance.md`
