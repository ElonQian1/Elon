---
title: 开放商业开发者 App 限权生产凭据 V1
status: accepted
owner: backend
reviewed_at: 2026-08-03
implementation_status: implementation_uncompiled
---

# 开放商业开发者 App 限权生产凭据 V1

## 背景

测试 Token 只适合沙盒开发。资料审核、主页域名控制证明和主体准入分别回答“App 声明了什么”“当前是否控制主页域名”和“平台是否批准当前主体声明”，任何一项都不应自动变成长期、全权限的生产密钥。

## 决定

1. 生产凭据使用独立表和 `oc_live_` 前缀，不修改沙盒 App 的测试 Token，也不把明文写入 App、准入或审计记录。
2. 功能由 `OPEN_COMMERCE_PRODUCTION_CREDENTIALS_ENABLED` 显式启用，默认关闭。关闭时不能签发，现有生产凭据也不能完成鉴权；撤销入口始终可用。
3. 只有启用中的 App、当前已批准资料、当前已验证主页域名和当前已批准准入记录同时成立时，平台管理员才能签发。主体声明仍不是外部工商核验。
4. 每次签发生成一个只显示一次的随机密钥，数据库只保存 SHA-256、末尾提示和审计元数据。同一 App 同时最多一条活动生产凭据；再次签发会原子撤销旧凭据。
5. 凭据范围必须是当前已审核 `requested_scopes` 的非空子集。生产调用、动作准备和动作确认均按能力 Key 失败关闭；调用账本、终态事件和幂等重放同时按凭据环境隔离。
6. 有效期由人工风险层级限制：`low` 最长 366 天、`standard` 最长 180 天、`enhanced` 最长 90 天。到期凭据鉴权失败。
7. 修改资料、重新发起域名验证、停用 App 或暂停准入会在同一事务中撤销活动生产凭据。重新审核、重新验证或重新启用不会恢复旧密钥。
8. App 所有者或项目管理员可查看凭据元数据并紧急撤销；平台管理员负责签发或轮换。PC 只在当前内存中显示一次完整密钥，不写入浏览器持久存储。

## 边界

- 本能力只建立可撤销的应用鉴权层，不代表商户已经授权具体能力；受限能力仍需独立 Grant、预算、期限和动作确认。
- `production` 表示凭据环境，不表示真实支付、清算、链上资产、外部平台授权、工商认证或生产部署已经完成。
- V1 没有硬件密钥、mTLS、IP 约束、自动风险评分、密钥托管、跨运营方信任根和完整生产运维验证。
- 当前代码按快速开发策略尚未编译、迁移、运行接口或验证 PC 交互。

## 实现引用

- `server/src/open_commerce_developer_credential_*.rs`
- `server/src/store/open_commerce_developer_credentials.rs`
- `server/src/open_commerce_client_api.rs`
- `server/src/open_commerce_action_confirmation_api.rs`
- `pc-frontend/src/features/open-commerce/DeveloperProductionCredentialPanel.tsx`
- `pc-frontend/src/features/open-commerce/DeveloperAppAdmissionReviewPanel.tsx`
- `docs/open-commerce-developer-production-credentials-v1-acceptance.md`
- `docs/decisions/open-commerce-invocation-credential-provenance-v1.md`
