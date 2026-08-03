---
title: 开放商业开发者签名 Webhook V1
status: accepted
owner: backend
reviewed_at: 2026-08-03
implementation_status: implementation_uncompiled
---

# 开放商业开发者签名 Webhook V1

> 后续修订：现有 Webhook 被明确限定为沙箱/旧调用通知，自动入队、历史补发、死信重试和工作器读取均不得交付生产调用。生产 Webhook 需要独立准入，当前边界以 `open-commerce-invocation-credential-provenance-v1.md` 为准。

## 背景

开发者 App 已能用测试 Token 按游标读取自己的终态调用事件，但持续轮询会增加接入复杂度和无效请求。开放商业需要主动通知能力，使外部 App 在调用成功或失败后及时继续订单、ERP 或用户通知流程，同时不能把平台变成可访问任意内网地址的代理，也不能把未签名响应当作可信业务事实。

## 决定

1. Webhook 复用 V134 终态序号和 Invocation 真源，不建立第二套订单、支付或结算状态机。轮询接口继续保留，作为恢复和对账入口。
2. 订阅绑定项目、App 记录、App ID 和 App 所有者。只有项目编辑者且同时是 App 所有者才能管理；App 停用时订阅和待投递记录同步停用。
3. 新订阅只接收创建后的事件，不回放历史。重新启用时从新的终态序号开始，避免停用期间积压突然发送。
4. 回调地址必须使用 HTTPS，不允许账号、查询参数或片段，且主机必须精确命中 `OPEN_COMMERCE_WEBHOOK_ALLOWED_HOSTS`。测试构建可使用本机回环地址。HTTP 客户端禁止重定向，连接与总请求时间有硬上限。
5. 服务端使用 `OPEN_COMMERCE_WEBHOOK_MASTER_SECRET` 派生每个订阅独立的 `whsec_` 密钥。原始主密钥和派生密钥均不入库；创建时只显示一次派生密钥，数据库只保存主密钥指纹。主密钥变化时旧订阅失败关闭，必须重新创建。
6. 每次请求使用 `HMAC-SHA256(secret, timestamp + "." + raw_body)`，通过 `x-yilong-webhook-id`、`x-yilong-webhook-timestamp` 和 `x-yilong-webhook-signature: v1=<hex>` 传递。接收端必须在解析 JSON 前验证原始字节并限制时间偏差。
7. 负载只含投递 ID、订阅 ID、App ID 和已有终态事件摘要，不推送结果正文、原始输入、Grant、用户 ID、项目 ID 或内部请求摘要。开发者仍用 App 自己的测试 Token 读取结果详情。
8. V145 用数据库触发器原子生成唯一投递记录。工作器通过 30 秒租约领取，网络错误、408、429 和 5xx 使用指数退避重试，最多 8 次；其他 4xx 进入死信，410 或签名主密钥变化立即停用订阅。连续 8 次失败自动停用，恢复必须由用户明确操作。
9. 每个 App 最多 5 个订阅；PC 门户显示订阅状态、连续失败和最近投递，不保存一次性签名密钥。
10. V1 是单运营方主动通知，不是跨运营方事件总线、支付回调、商户 ERP 写入证明或投递 SLA。域名白名单由运营方配置，不宣称自动完成域名所有权挑战。

## HTTP 契约

- `GET/POST /api/projects/:project_id/open-commerce/developer-apps/:app_record_id/webhooks`
- `POST .../webhooks/:subscription_id/disable`
- `POST .../webhooks/:subscription_id/enable`
- `GET .../webhooks/:subscription_id/deliveries`

## 实现引用

- `server/src/open_commerce_webhook_*.rs`
- `server/src/store/open_commerce_developer_webhooks.rs`
- `pc-frontend/src/features/open-commerce/DeveloperWebhookPanel.tsx`
- `sdk/open-commerce-connector/src/webhook-signature.js`
- `docs/open-commerce-developer-webhooks-v1-acceptance.md`
