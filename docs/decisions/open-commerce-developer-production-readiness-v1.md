---
title: 开放商业开发者 App 生产就绪总览 V1
status: accepted
owner: backend
reviewed_at: 2026-08-03
implementation_status: implementation_uncompiled
---

# 开放商业开发者 App 生产就绪总览 V1

## 背景

开发者 App 的启停、资料审核、当前修订域名证明、网络准入、生产凭据和生产 Webhook 已由不同模块管理。开发者需要逐个面板判断生产调用或主动通知为什么尚不可用，容易把某个局部通过状态误认为完整生产资格。

## 决定

1. 新增 App 级只读生产就绪摘要，从既有真源查询时派生，不新增就绪表、审批状态或缓存状态。
2. 就绪步骤固定为 App、资料审核、当前域名证明、网络准入、生产凭据开关、当前生产凭据、生产 Webhook 开关和活动生产订阅。
3. 每个未完成步骤返回稳定阻断码，并按依赖顺序给出 `next_action_code`。PC 把稳定码转换为可理解的操作提示，但不在总览内复制审核、签发或订阅动作。
4. `production_invocation_ready` 要求 App、资料、域名、准入、生产凭据开关和当前生产凭据全部有效。
5. `production_webhook_ready` 在生产调用就绪基础上，继续要求生产 Webhook 开关和至少一个活动生产订阅。
6. 摘要复用当前生产凭据资格和 Webhook 健康查询，不自行放宽到期、修订绑定、环境隔离或生产开关规则。
7. 接口沿用项目编辑权限及 App 所有者或项目管理员边界；读取总览不会签发凭据、启用开关、修改订阅或执行调用。

## 边界

- 摘要是查询时快照，不是外部组织身份、真实平台授权、资金能力、SLA、送达结果或生产部署证明。
- V1 不提供一键修复、自动提交审核、自动签发凭据、自动创建 Webhook、外部告警或跨运营方互认。
- 当前代码尚未编译、运行接口、验证权限组合、检查状态竞争或验证 PC 页面。

## 实现引用

- `server/src/open_commerce_developer_readiness_model.rs`
- `server/src/open_commerce_developer_readiness_service.rs`
- `server/src/open_commerce_developer_readiness_api.rs`
- `pc-frontend/src/features/open-commerce/DeveloperProductionReadinessPanel.tsx`
- `docs/open-commerce-developer-production-readiness-v1-acceptance.md`
