---
title: 开放商业能力与内部同步回执来源绑定 V1
status: accepted
owner: open-commerce
reviewed_at: 2026-08-03
implementation_status: implementation_uncompiled
---

# 开放商业能力与内部同步回执来源绑定 V1

## 背景

公开能力已经可以声明来源类型和有效时长，但此前不能说明某项能力引用了哪一次项目内部数据同步。数据接入控制面已经保存厂商标识、接入方式、数据域和幂等同步回执，因此可以建立一条受控关联，同时必须避免把商户或适配器提交的内部回执描述为美团、抖音、京东、淘宝闪购或其他外部平台的签名证明。

## 决定

1. V164 新增能力来源绑定表，每项能力最多保留一条当前绑定，重绑时递增修订并保留审计事件。
2. 能力、数据接入和同步回执必须属于同一项目；能力与数据接入还必须属于同一商户。已停用接入不能新建绑定。
3. 绑定的数据域必须已经登记在所选接入中。只有 `full` 或 `incremental` 且状态为 `succeeded` 或 `partial` 的回执可作为来源；健康检查和失败回执拒绝绑定。
4. 绑定固化当前能力版本。能力更新后旧记录继续保留，但 `publishable=false`，公开目录退回普通商户声明，直至编辑者重新确认绑定。
5. 有效绑定在公开目录中使用 `integration_sync_receipt` 来源类型，并公开厂商标识、接入方式、数据域、回执状态、完成时间和回执 SHA-256 摘要。
6. 公开来源的 `assertion_authority` 仍为 `merchant_project`，`externally_verified` 永远为 `false`。摘要只用于识别本项目保存的回执内容，不证明外部平台签发、真实回读或数据正确。
7. 有效绑定存在时，新鲜度以回执完成时间加商户声明的 `freshness_seconds` 派生；无有效绑定时继续以能力声明更新时间派生。
8. 商户 PC 工作台允许编辑者选择能力、合格回执和数据域并绑定，也可移除绑定；消费者结果只显示简短来源标签，详细提示明确“内部回执，未经外部平台验证”。
9. 本批不保存外部平台令牌或原始经营数据，不调用外部平台，不改变授权、调用、计量、结算、订单或支付语义。

## 失效与回退

以下任一条件成立时绑定不得进入公开目录：能力版本变化、接入停用、回执为健康检查，或回执状态不是成功/部分成功。项目总览仍返回失效记录及稳定阻断码，供商户重新绑定或移除。

## 信任边界

- `official_api` 只是商户登记的接入方式，不证明官方授权当前有效。
- `succeeded` 和 `partial` 是本项目内部回执状态，不是第三方鉴证结论。
- SHA-256 证明相同字节得到相同摘要，不证明数据来源身份、完整性或业务真实性。
- 新鲜度只表示距离所绑定回执完成时间是否仍在商户声明期限内，不保证库存、价格、营业状态或订单实时。
- 当前代码未编译，未执行 V164 迁移、HTTP、目录、排序凭证、兼容性或 PC 验证，状态为 `implementation_uncompiled`。

## 实现入口

- `server/src/open_commerce_capability_source_*.rs`
- `server/src/store/open_commerce_capability_sources.rs`
- `server/src/open_commerce_directory_model.rs`
- `pc-frontend/src/features/open-commerce/OpenCommerceIntegrationManager.tsx`
- `pc-frontend/src/features/open-commerce/ConsumerCommerceSandbox.tsx`
- `docs/open-commerce-capability-source-link-v1-acceptance.md`
