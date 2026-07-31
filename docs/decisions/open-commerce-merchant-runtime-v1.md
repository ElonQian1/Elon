---
title: 开放商业受控商户运行时 V1
status: accepted
owner: backend
reviewed_at: 2026-07-31
---

# 开放商业受控商户运行时 V1

## 背景

开放商业 V1 原有 `merchant_profile` 与 `static_json` 处理器可以验证发现、授权、幂等、计量和审计，但不能把调用转交给商户自己的 ERP。直接允许能力配置任意 URL 和明文密钥，会引入服务端请求伪造、密钥泄漏、身份冒充和不可审计写入，因此不能作为生产路线。

`cofficethinking` 是首个真实付费商户项目。它已经拥有商品、订单和经营数据，适合作为参考商户节点，但其 APK 是管理客户端，不能被误当成始终在线的服务端。

## 决定

增加平台审核的 `merchant_runtime` 处理器，并把远端地址和凭据从能力契约中分离为项目级 `RuntimeBinding`：

1. 能力只声明 `handler_type=merchant_runtime`，不接受 URL 或密钥配置。
2. 运行绑定保存 HTTPS 基础地址、服务端环境变量引用、预期 Manifest 摘要、超时和验证状态。
3. 生产地址必须命中 `OPEN_COMMERCE_RUNTIME_ALLOWED_HOSTS`；只有测试允许回环 HTTP。
4. 平台以 HMAC-SHA256 对原始请求体和时间戳签名，商户运行时验证密钥标识、签名和重放窗口。
5. 签名健康检查核对商户 ID 与 Manifest SHA-256，成功后绑定才进入 `active`。
6. 运行失败将绑定降级；地址、密钥引用和处理器配置不进入公开发现或 AI 开发上下文。

## 交易边界

- 商户后端是商品、价格、库存和订单的事实来源。
- 消费者只提交商品 ID 与数量，报价由商户服务端按整数最小货币单位计算。
- `order.commit` 必须具备有效 Grant、未过期报价、显式用户确认和幂等键。
- 商品订单金额与平台能力调用费分离；调用继续生成 `recorded_not_charged` 结算回执，不移动真实资金。
- 平台保留 Invocation、Meter、Audit；商户保留报价、订单、库存和本地调用回执。

## 参考实现

首个参考节点位于独立仓库 `D:\rust\active-projects\cofficethinking`。它实现：

- `GET /commerce/v1/manifest`
- `POST /commerce/v1/invoke`
- `merchant.profile.read`
- `catalog.search`
- `product.detail.read`
- `order.quote.create`
- `order.commit`
- `order.status.read`

该决定不表示已完成公共跨项目发现、第三方生产 App 审核、支付扣款或大型平台生产适配器。

## 实现引用

- `server/src/open_commerce_runtime_*.rs`
- `server/src/store/open_commerce_runtime_bindings.rs`
- `pc-frontend/src/features/open-commerce/OpenCommerceRuntimeManager.tsx`
- `contracts/open-commerce/merchant-runtime-v1.json`
- `scripts/test-open-commerce-merchant-runtime-contract.ps1`
