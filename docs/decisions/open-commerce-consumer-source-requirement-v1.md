---
title: 开放商业消费者内部回执来源筛选 V1
status: accepted
owner: open-commerce
reviewed_at: 2026-08-03
implementation_status: implementation_uncompiled
---

# 开放商业消费者内部回执来源筛选 V1

## 背景

公开目录既包含普通商户声明能力，也可以包含显式关联项目内部同步回执的能力。消费者 AI 可能希望只使用带内部同步证据的能力，但平台不能默认隐藏普通声明，也不能把内部回执误称为外部平台验证。

## 决定

1. 消费者发现请求增加 `require_internal_sync_receipt`，默认 `false`，旧请求和旧结果保持不变。
2. 用户显式开启后，只保留 `source.kind=integration_sync_receipt` 的能力。因能力版本变化、接入停用或回执失效而回退普通声明的能力自动被排除。
3. 响应使用 `source_requirement=internal_sync_receipt` 回显选择；默认值为 `any_merchant_source`。
4. 每条保留结果增加“已关联商户项目内部业务同步回执”的匹配原因。
5. 请求指纹和排序凭证规范负载同时固化来源要求，切换开关必须得到不同输入摘要。
6. 来源筛选与声明期筛选独立且可组合：同时开启时，能力既要有有效内部回执绑定，也要处于商户声明有效期内。
7. 本批不自动降权、下架、阻断调用或改变排序器，不新增外部平台校验和真实交易语义。

## 信任边界

- 开启筛选只证明目录当前存在符合项目规则的内部回执关联。
- 内部回执由商户项目或受控适配器提交，固定 `externally_verified=false`。
- 筛选不证明外部平台授权、数据真实性、实时库存、价格、营业状态、支付或履约。
- 当前代码未编译，未执行请求兼容、筛选组合、排序凭证或 PC 验证，状态为 `implementation_uncompiled`。

## 实现入口

- `server/src/open_commerce_consumer_model.rs`
- `server/src/open_commerce_consumer.rs`
- `pc-frontend/src/features/open-commerce/ConsumerCommerceSandbox.tsx`
- `docs/open-commerce-consumer-source-requirement-v1-acceptance.md`
