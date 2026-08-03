---
title: 开放商业开发者 Webhook 回调验证 V1
status: accepted
owner: backend
reviewed_at: 2026-08-03
implementation_status: implementation_uncompiled
---

# 开放商业开发者 Webhook 回调验证 V1

## 背景

Webhook V1 已限制为运营方白名单内的 HTTPS 主机，但白名单只证明运营方允许平台访问该主机，不能证明当前开发者控制具体回调处理器。若创建后立即激活，填错路径也会产生无效重试，且可能把终态事件发送给同一域名下错误的服务。

## 决定

1. V146 为订阅追加 `pending`、`failed`、`verified` 验证状态。新订阅初始停用且待验证；V145 已存在订阅按原行为迁移为已验证，避免升级后静默中断。
2. 开发者保存一次性签名密钥并部署接收端后，必须显式发起验证。平台向订阅的精确回调地址发送 5 分钟有效的随机 challenge。
3. 验证请求使用与正式事件相同的订阅 HMAC 密钥和三个 `x-yilong-webhook-*` 请求头。接收端应先验证原始请求体签名，再以 JSON 返回完全相同的 `challenge`。
4. 平台只接受 2xx、有效 JSON、完全相同的 challenge，响应上限 16 KiB；禁止重定向，连接和请求总时长有硬上限。网络、状态码、JSON、大小和回显错误使用稳定错误码记录。
5. 验证成功后订阅从当时最新终态序号开始激活，不回放创建到验证之间的历史。验证失败保持停用，可在修复接收端后重试。
6. 手动启用只能用于已经验证的订阅。App 停用、主密钥变化、连续投递失败等原有失败关闭规则不变。
7. challenge 回显证明请求到达并由精确地址处理，不等于工商身份、域名法律所有权、生产 App 审核、跨运营方互认或支付资质认证。

## 接收端响应

```json
{
  "challenge": "whch_..."
}
```

## 实现引用

- `server/src/open_commerce_webhook_verification.rs`
- `server/src/open_commerce_webhook_verification_migration.rs`
- `server/src/store/open_commerce_developer_webhooks.rs`
- `pc-frontend/src/features/open-commerce/DeveloperWebhookPanel.tsx`
- `docs/open-commerce-developer-webhook-verification-v1-acceptance.md`
