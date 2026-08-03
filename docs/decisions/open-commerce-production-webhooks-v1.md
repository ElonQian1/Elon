---
title: 开放商业环境绑定生产 Webhook V1
status: accepted
owner: backend
reviewed_at: 2026-08-03
implementation_status: implementation_uncompiled
---

# 开放商业环境绑定生产 Webhook V1

## 背景

沙箱 Webhook 可以帮助开发者验证接入流程，但不能承载生产调用结果。直接让既有订阅同时接收沙箱和生产事件，会造成凭据来源混淆、历史补发越界，并使失效的 App 资料、域名证明或准入状态继续获得生产通知。

## 决定

1. V156 为每个 Webhook 订阅固定 `sandbox` 或 `production` 环境；已有订阅迁移为 `sandbox`，环境创建后不可修改，需要切换时创建新订阅。
2. 生产 Webhook 由 `OPEN_COMMERCE_PRODUCTION_WEBHOOKS_ENABLED` 单独控制且默认关闭，同时依赖生产凭据开关。创建、验证、启用、轮换签名密钥、重试死信和历史补发均复核当前生产资格。
3. 当前生产资格同时要求：活动且未过期的唯一生产凭据、启用中的 App、当前修订资料已批准、当前修订主页域名已验证、当前修订准入已批准。只满足其中一部分时失败关闭。
4. 终态事件触发器按环境原子入队：沙箱订阅只接收 `legacy` 和 `sandbox` 调用，生产订阅只接收 `production` 调用。历史补发、死信重试、工作器结果读取采用相同边界。
5. 工作器领取任务前清理已失去生产资格的订阅，并把尚未完成的投递转为死信。生产凭据撤销或轮换会在同一事务中停用生产订阅、终止在途投递；新凭据签发后需要用户明确重新启用订阅。
6. 工作器投递前再次检查两个生产开关。关闭任一开关都会停止生产投递并自动停用订阅，不影响沙箱通知。
7. challenge 和正式事件负载升级为 V2，明确携带订阅环境。每个订阅继续使用独立版本化 HMAC 密钥、精确主机白名单、公网地址固定、禁代理和禁重定向机制。
8. 生产事件只携带开发者 App 自己的终态调用摘要，不包含原始输入、结果正文、用户或项目内部标识。结果详情继续通过对应生产凭据按环境读取。

## 边界

- `production` 只表示本平台当前准入条件下的生产凭据事件，不代表支付、退款、履约、外部平台回调、ERP 写入或链上结算已经发生。
- 当前仍是单运营方 Webhook，不是跨运营方事件总线；没有 mTLS、硬件密钥、投递 SLA、独立生产签名根或外部组织身份持续核验。
- 生产凭据到期的清理依赖工作器领取周期；到期后控制面操作会立即失败，待投递记录在下一次领取时失败关闭。
- 当前代码尚未编译、执行 V156 迁移、运行接口、发送真实 HTTPS 请求或验证 PC 交互。

## 实现引用

- `server/src/open_commerce_production_webhook.rs`
- `server/src/open_commerce_production_webhook_migration.rs`
- `server/src/open_commerce_webhook_*.rs`
- `server/src/store/open_commerce_production_webhooks.rs`
- `server/src/store/open_commerce_developer_webhook*.rs`
- `server/src/store/open_commerce_developer_credentials.rs`
- `pc-frontend/src/features/open-commerce/DeveloperWebhookPanel.tsx`
- `docs/open-commerce-production-webhooks-v1-acceptance.md`
- `docs/decisions/open-commerce-webhook-operational-health-v1.md`
