---
title: 开放商业调用凭据来源与环境隔离 V1
status: accepted
owner: backend
reviewed_at: 2026-08-03
implementation_status: implementation_uncompiled
---

# 开放商业调用凭据来源与环境隔离 V1

## 背景

开发者 App 同时拥有沙箱 Token 和限权生产凭据后，只按 App 记录调用无法回答“哪种环境、哪一条生产凭据发起了这次调用”。这会让幂等重放、商户运行时、终态结果和 Webhook 在沙箱与生产之间形成模糊边界。

## 决定

1. 每条调用记录保存 `credential_environment` 和非秘密 `credential_id`。环境只允许 `legacy`、`platform`、`sandbox` 和 `production`；生产调用必须绑定具体生产凭据 ID。
2. 平台 HTTP、MCP 和 PC 调用记为 `platform`；测试 Token 调用记为 `sandbox`；生产凭据调用记为 `production`。迁移前无法证明来源的记录记为 `legacy`。
3. 幂等键除用户、App、商户、能力和输入摘要外，还必须匹配凭据环境与凭据 ID。`legacy` 只兼容平台或沙箱重放，生产凭据不能继承无法证明来源的旧调用。
4. 沙箱凭据可以调用平台托管的脱敏资料和静态能力，但不得准备、确认或执行 `merchant_runtime` 能力。真实商户运行时只接受当前有效且能力范围获准的生产凭据。
5. 商户运行时信封携带凭据环境和凭据 ID，使商户节点能够把中央调用记录与自身审计对应起来；该字段不包含密钥或 Token 摘要。
6. 开发者终态事件游标绑定 App 与凭据环境。沙箱只读取沙箱及兼容旧记录，生产只读取当前 App 的生产记录；事件摘要返回环境和凭据 ID。
7. 现有签名 Webhook 定义为沙箱通知能力。数据库自动入队、历史补发、死信重试和工作器读取均拒绝生产调用；生产 Webhook 需要后续独立的环境绑定、回调准入和密钥治理方案。
8. 生产凭据轮换后，新凭据可以读取同一 App 的既有生产终态事件，但不能用旧幂等键冒充旧凭据重放调用。

## 边界

- `production` 只表示经过当前平台准入的调用凭据环境，不表示支付成功、真实清算、外部平台授权、链上提交或生产部署已经完成。
- V1 没有生产 Webhook、mTLS、硬件密钥、跨运营方事件总线、真实商户运行时回归或第三方合规证明。
- 沙箱不是完整的虚拟商户数据复制环境；它只允许不会触达真实商户运行时的现有平台托管能力。
- 当前代码尚未编译、迁移、运行接口或执行网络验证。

## 实现引用

- `server/src/open_commerce_invocation_provenance_migration.rs`
- `server/src/open_commerce_invocation_service.rs`
- `server/src/store/open_commerce_invocations.rs`
- `server/src/store/open_commerce_developer_events.rs`
- `server/src/store/open_commerce_developer_webhook_history.rs`
- `server/src/store/open_commerce_developer_webhook_replays.rs`
- `docs/open-commerce-invocation-credential-provenance-v1-acceptance.md`
