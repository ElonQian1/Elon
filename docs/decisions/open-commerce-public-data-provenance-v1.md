---
title: 开放商业公开数据来源与新鲜度声明 V1
status: accepted
owner: open-commerce
reviewed_at: 2026-08-03
implementation_status: implementation_uncompiled
---

# 开放商业公开数据来源与新鲜度声明 V1

## 背景

公开目录原本返回能力更新时间和 `freshness_seconds`，但消费者无法区分该时间代表什么，也无法判断声明是否已经过期。连接器同步回执与公开能力之间尚无可验证关联，因此系统不能把商户自报数据描述为美团、京东、抖音、淘宝闪购或其他外部平台已经核验的数据。

## 决定

1. 每项公开能力增加机器可读的 `source`。V1 根据能力处理器区分商户公开资料、商户静态数据、商户运行时和兜底商户声明，声明权威固定为 `merchant_project`。
2. `source.externally_verified` 固定为 `false`，`integration_receipt_id` 固定为空。没有建立能力与接入同步回执的摘要绑定前，不得展示为外部平台来源证明。
3. 每项公开能力增加 `freshness`，根据能力声明更新时间与商户填写的有效秒数，在每次目录读取时派生 `current`、`stale` 或 `unknown`。
4. 有效秒数为零、时间无法解析或有效期无法安全计算时，状态必须为 `unknown`，不得猜测为当前有效。
5. 新鲜度计算依据固定为 `capability_declaration_updated_at`。它只表示商户对当前能力声明的时间承诺，不代表库存、价格、营业状态或外部订单已经实时回读。
6. 商户 PC 可在发布能力时填写有效秒数，并明确显示零代表不声明；消费者 PC 显示来源类型、商户声明身份及新鲜度状态。
7. 消费者显式生成排序凭证时，每条有序结果同时固化来源声明和新鲜度快照，使下载文件与当次页面展示保持一致。
8. V1 不新增数据库表，不改变调用授权、计量、结算、业务回执或连接器同步语义。
9. 消费者是否排除 `stale/unknown` 由 `open-commerce-consumer-freshness-filter-v1` 单独决定；来源声明层本身不自动隐藏能力。

## 信任边界

- `current` 只表示当前时间仍处于商户声明有效期内，不证明业务数据真实或实时。
- `stale` 只表示声明有效期已经经过，不自动下架能力，也不阻止消费者在知情后调用。
- `unknown` 不等于数据错误，只表示商户没有给出可计算的时间承诺。
- `merchant_runtime` 只描述能力处理器类型，不证明运行时在线、外部平台授权有效或调用必然成功。
- 排序凭证中的来源和新鲜度仍是未签名目录快照，不证明运营方身份、商户身份或外部平台背书。
- 将来若接入同步回执，必须另建能力、数据域、回执摘要和观察时间之间的显式关系，不能复用 V1 字段伪造证明。
- 当前代码未编译、未执行接口、时间边界、浏览器或兼容性验证，状态为 `implementation_uncompiled`。

## 实现入口

- `server/src/open_commerce_directory_model.rs`
- `server/src/open_commerce_consumer.rs`
- `pc-frontend/src/features/open-commerce/openCommerceClientTypes.ts`
- `pc-frontend/src/features/open-commerce/OpenCommerceMerchantEditor.tsx`
- `pc-frontend/src/features/open-commerce/ConsumerCommerceSandbox.tsx`
- `docs/open-commerce-public-data-provenance-v1-acceptance.md`
