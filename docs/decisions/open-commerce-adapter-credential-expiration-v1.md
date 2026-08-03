---
title: ADR：开放商业接入器凭据限时有效 V1
status: accepted
owner: backend
date: 2026-08-03
---

# ADR：开放商业接入器凭据限时有效 V1

## 背景

接入器机器 Token 已可单独轮换和撤销，但若管理员忘记操作，长期 Bearer Token 会持续有效。它虽然只有 `business_handoff.write` 权限，泄露后仍可能伪造接入器处理声明，因此有效期必须由服务端强制，而不能只依赖 PC 提醒。

## 决策

1. 签发或轮换机器凭据时必须明确提供 `expires_in_days`，允许范围为 1–366 天；PC 默认 90 天并提供 30、90、365 天选项。
2. 服务端保存绝对 `expires_at`。鉴权同时检查 Token 摘要、凭据活动状态、接入状态和数据库时间；任一条件不满足即失败关闭。
3. 列表返回 `expires_at` 和服务端派生的 `is_expired`。PC 同时显示到期日，并在剩余 14 天内标记“即将到期”。
4. 到期不删除或改写历史回执，也不自动续期。继续使用必须由项目编辑者明确轮换，产生新 Token 和新凭据版本。
5. 升级前已有凭据由迁移统一补 90 天期限，避免发布瞬间中断，同时消除永久有效状态。
6. HTTP 与 MCP 沿用原有明确确认要求；MCP Schema 同时限制整数范围，领域服务再次校验，不能只信任客户端。

## 边界

- 当前没有自动轮换、通知推送、mTLS、硬件密钥或外部秘密管理服务。
- 到期只限制一龙机器入口，不代表外部平台授权或商户 ERP 密钥已同步失效。
- 历史 `adapter_token_authenticated` 回执仍保留当时凭据 ID 和版本，不因当前凭据到期而失真。

## 实现证据

- 迁移：`server/src/open_commerce_adapter_expiration_migration.rs`
- 鉴权：`server/src/store/open_commerce_adapter_credentials.rs`
- 领域校验：`server/src/open_commerce_adapter_service.rs`
- HTTP 与 MCP：`server/src/open_commerce_adapter_api.rs`、`server/src/open_commerce_adapter_mcp.rs`
- PC：`pc-frontend/src/features/open-commerce/OpenCommerceAdapterCredentialManager.tsx`
- 测试：`server/src/open_commerce_adapter_tests.rs`、`scripts/test-open-commerce-pc-workspace.js`
