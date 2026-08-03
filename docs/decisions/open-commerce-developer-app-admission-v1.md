---
title: 开放商业开发者 App 可撤销准入审查 V1
status: accepted
owner: backend
reviewed_at: 2026-08-03
implementation_status: implementation_uncompiled
---

# 开放商业开发者 App 可撤销准入审查 V1

## 背景

资料审核和主页域名控制证明只能说明“当前 App 资料完整”以及“申请人能够在验证时控制该域名”。它们不能表达申请主体的自我声明、平台风险分层、退回补充或紧急暂停，也不能直接复用为生产凭据状态。

## 决定

1. 每个开发者 App 可维护一条独立准入记录，包含当前资料修订、主体名称、注册地区、登记编号、申请人确认、审查状态、风险层级和审查说明。
2. 只有处于启用状态、当前资料修订已审核通过且当前域名证明有效的 App 才能提交准入申请。
3. App 所有者或项目管理员必须明确确认主体声明真实且有权提交；登记编号不写入审计详情。
4. 准入状态为 `submitted`、`changes_requested`、`approved` 或 `suspended`。平台管理员可批准或退回待审申请，也可单独暂停已批准记录；退回与暂停必须填写说明。
5. 批准时固定 `low`、`standard` 或 `enhanced` 风险层级。该层级只是人工治理标签，不是自动风控结论。
6. App 资料变化会原子失效待审或已批准记录；App 停用会原子暂停待审或已批准记录。重新启用不会恢复旧准入，申请人必须重新提交。
7. `approved` 只表示平台批准了当前资料修订对应的准入记录。V1 始终返回 `production_credential_issued=false` 和 `network_access_enabled=false`，不改变沙盒环境，不签发生产密钥，也不开放真实资金或交易。

## 边界

- 主体名称、地区和登记编号均为申请人声明，V1 未连接工商、税务或第三方身份核验机构。
- 域名证明不等于域名法律所有权，风险层级不等于信用评级。
- 未来生产凭据必须是独立、一次性展示、仅保存摘要、可轮换和可紧急撤销的能力，并同时检查 App 启用状态、当前资料修订、域名证明和准入状态。

## 实现引用

- `server/src/open_commerce_developer_admission_*.rs`
- `server/src/store/open_commerce_developer_app_admissions.rs`
- `pc-frontend/src/features/open-commerce/DeveloperAppAdmissionPanel.tsx`
- `pc-frontend/src/features/open-commerce/DeveloperAppAdmissionReviewPanel.tsx`
- `docs/open-commerce-developer-app-admission-v1-acceptance.md`
