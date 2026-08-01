---
title: 开放商业 Grant 限时授权 V1
status: accepted
date: 2026-08-02
owners: backend, product
---

# 开放商业 Grant 限时授权 V1

## 背景

开放商业 Grant 的底层数据模型和调用校验已经支持 `expires_at`，但商户 PC 直接授权和第三方 App 申请审批没有提供期限选择，审批路径还固定写入永不过期。商户即使设置了调用次数和金额，也无法通过日常界面限制信任关系持续多久。

V1 把既有期限能力贯通到商户工作流，不新增第二套授权模型。

## 决定

1. 商户直接创建 Grant 和批准第三方 App 申请时，都可以设置 `expires_at`、总调用次数和总计量金额。
2. PC 提供 7 天、30 天、90 天、1 年和长期有效五个选项；新表单默认 30 天。长期授权必须由商户显式选择。
3. HTTP API 保持向后兼容：未提供 `expires_at` 仍表示长期有效；服务端只接受晚于当前服务器时间的 RFC 3339 时间。
4. 授权审批必须把期限原样传入统一 Grant 创建领域服务，不能在 API 层静默丢弃。
5. 批准后的授权申请返回实际 Grant 的期限、次数上限、金额上限和币种，使商户与申请方看到同一组批准条件。
6. 到期不删除或改写 Grant 和审批历史。发现不再把它视为有效授权，调用必须失败关闭，PC 不再把它列入可调用 Grant。
7. V1 不自动续期。需要继续合作时，商户应创建新 Grant 或由 App 重新申请，形成新的审计边界。
8. 批准审计记录期限和预算边界，但不记录 Token、调用正文或经营数据。

## 非目标

V1 不包含到期前通知、自动续期、批量续期、宽限期、生产开发者身份审核、跨运营方授权互认、真实扣款和链上授权对象。

## 实现入口

- 审批模型：`server/src/open_commerce_developer_model.rs`
- 审批到 Grant 映射：`server/src/open_commerce_authorization_decision.rs`
- 审批 API：`server/src/open_commerce_client_api.rs`
- 申请与 Grant 条件回读：`server/src/store/open_commerce_authorization_requests.rs`
- PC 期限规则：`pc-frontend/src/features/open-commerce/openCommerceGrantExpiry.ts`
- 商户与开发者界面：`pc-frontend/src/features/open-commerce/OpenCommerceMerchantEditor.tsx`、`pc-frontend/src/features/open-commerce/DeveloperCommercePortal.tsx`
- 验收：`docs/open-commerce-grant-expiration-v1-acceptance.md`
