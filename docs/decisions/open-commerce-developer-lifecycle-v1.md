---
title: "开放商业开发者应用与授权申请生命周期 V1"
status: accepted
reviewed_at: 2026-08-01
---

# 开放商业开发者应用与授权申请生命周期 V1

## 背景

开发者沙盒已经支持应用注册、Token 轮换、授权申请和商户审批，但应用一旦创建就无法停用，申请方也看不到自己发出的完整申请记录，更不能撤回误发或不再需要的申请。数据库虽然预留了 `disabled` 和 `canceled`，产品与领域服务并未真正使用，导致凭据止损和双方状态闭环不完整。

## 决定

1. 开发者项目编辑者可以停用沙盒 App。停用时立即废弃当前测试 Token，并把该 App 的全部待处理授权申请改为 `canceled`。
2. 停用 App 不能发现受限能力、提交授权申请、使用登录身份调用，也不能通过测试 Token 调用。
3. 重新启用必须生成新的仅显示一次测试 Token；停用前 Token 永久不能恢复。
4. 已批准 Grant 不在 App 停用时暗中删除。App 停用期间身份验证会阻断调用；重新启用后，商户仍可单独撤销既有 Grant。
5. 商户批准授权申请前必须再次验证申请方 App 存在、仍启用且属于原申请用户，避免批准已经停用或被冒用的身份。
6. 开发者项目可以查看其所有 App 发出的授权申请，并撤回仍为 `pending` 的申请；商户收件箱与申请方列表读取同一条状态记录。
7. 停用、重新启用和申请撤回写入审计。涉及商户的撤回同时进入商户项目审计，保持双方可追溯。
8. 测试 Token 继续只在 PC 明确创建、轮换或重新启用时显示一次；本阶段不通过 MCP 返回新明文凭据。

## 非目标

- 不提供生产 App 审核、客户端密钥、公钥身份、OAuth、跨运营方身份联合或公开应用市场。
- 不实现调用限流、套餐、自动扣费、退款或链上结算。
- 不把“停用 App”解释为商户撤销 Grant；两种权力分别属于开发者和商户。

## 结果

沙盒应用具备“注册 -> 轮换 -> 停用止损 -> 新凭据重新启用”的身份生命周期，授权申请具备“发出 -> 查看 -> 撤回 / 商户决定”的双向状态闭环。该能力提高了试点可用性，但仍不是生产第三方应用身份系统。

## 实现证据

- `server/src/open_commerce_client_lifecycle_service.rs`
- `server/src/open_commerce_client_lifecycle_api.rs`
- `server/src/store/open_commerce_developer_apps.rs`
- `server/src/store/open_commerce_authorization_requests.rs`
- `pc-frontend/src/features/open-commerce/DeveloperCommercePortal.tsx`
- `pc-frontend/src/features/open-commerce/OutboundAuthorizationRequests.tsx`
- `server/src/open_commerce_client_service_tests.rs`
