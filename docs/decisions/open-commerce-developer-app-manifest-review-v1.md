---
title: 开放商业开发者 App 资料清单与审核 V1
status: accepted
owner: backend
reviewed_at: 2026-08-03
implementation_status: implementation_uncompiled
---

# 开放商业开发者 App 资料清单与审核 V1

## 背景

沙盒 App 只有名称、App ID 和测试凭据，不能表达开发者主页、隐私政策、服务条款、支持渠道及拟申请的商业能力。开放商业网络进入生产准入前，需要先形成可版本化、可审计且不会被误解为生产授权的资料审核控制面。

## 决定

1. 每个开发者 App 保存 HTTPS 主页、隐私政策、服务条款、支持邮箱和最多 32 项标准化能力申请。
2. 资料采用 `draft`、`submitted`、`changes_requested`、`approved` 四态；每次编辑递增修订号、清除旧审核结论并回到草稿。
3. App 所有者或项目管理员可保存和提交资料。普通项目编辑者不能代替其他 App 所有者维护资料。
4. 平台管理员只能审核仍处于 `submitted` 的同一修订；要求修改必须填写审核说明，所有状态变化进入开放商业审计事件。
5. `approved` 只表示平台完成当前资料修订的人工审核。它不改变 `sandbox` 环境，不签发生产凭据，不开放真实扣款或交易，也不代表工商身份、域名法律所有权或外部平台背书。
6. 生产准入必须在后续独立能力中增加身份核验、域名控制证明、风险策略、生产密钥和可撤销准入状态，不能复用资料审核字段冒充完成。

## 实现引用

- `server/src/open_commerce_developer_manifest_api.rs`
- `server/src/open_commerce_developer_manifest_service.rs`
- `server/src/store/open_commerce_developer_app_manifests.rs`
- `pc-frontend/src/features/open-commerce/DeveloperAppManifestPanel.tsx`
- `docs/open-commerce-developer-app-manifest-review-v1-acceptance.md`
