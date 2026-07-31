---
title: "开放商业消费者与开发者沙盒 V1"
status: accepted
reviewed_at: 2026-07-31
---

# 开放商业消费者与开发者沙盒 V1

## 背景

开放商业 V1 已具备商户、能力、授权和调用主干，但缺少一个可验证的消费者发现路径与第三方应用接入闭环。直接把原型宣传为公共网络，会混淆“单项目沙盒”和“跨主体生产网络”，也会掩盖身份、滥用防护和真实结算仍未完成的事实。

## 决定

1. 先在现有登录项目中提供消费者发现与第三方应用沙盒。
2. 发现排序只使用公开商户资料、能力属性和消费者显式偏好，响应必须返回 `ranking_is_paid`。
3. `pc-web` 只代表公共网页身份，可调用公开能力，但不能申请受限能力授权。
4. 第三方测试 App 必须归属一个用户可编辑的项目；受限能力先申请、由商户项目批准后再调用。
5. 测试 Token 只在创建或轮换时显示一次，服务端只保存摘要，浏览器不得写入本地持久存储。
6. 只有状态为 `authorized` 的能力可以进入授权申请；`owner_only` 能力不能通过第三方申请绕过。
7. 所有调用继续复用开放商业领域服务、幂等键、计量和审计，不建立第二套调用规则。

## 当前边界

- 这是登录用户范围内的开发与验收沙盒，不是面向公众的全球商户发现网络。
- 没有付费排名、自动扣款、支付、订单履约或跨平台身份互认。
- 测试 Token 不是生产凭据，尚未提供应用发布审核、限额套餐和开发者结算。
- 发现结果只说明当前索引中可见的能力，不代表已接通美团、抖音、京东或淘宝闪购。

## 结果

该决定让消费者、商户和第三方开发者可以在不等待公共网络的情况下验证“发现 -> 授权 -> 调用 -> 审计”闭环，同时把凭据、权限和宣传边界保持为可审计事实。公共网络只有在身份互认、滥用防护、限流和跨项目发现经过单独决策后才能推进。

## 实现证据

- `server/src/open_commerce_client_api.rs`
- `server/src/open_commerce_consumer.rs`
- `server/src/open_commerce_client_service_tests.rs`
- `pc-frontend/src/features/open-commerce/ConsumerCommerceSandbox.tsx`
- `pc-frontend/src/features/open-commerce/DeveloperCommercePortal.tsx`
- `scripts/test-open-commerce-pc-workspace.js`
