---
title: 开放商业消费者价格币种筛选 V1
status: accepted
owner: open-commerce
reviewed_at: 2026-08-03
implementation_status: implementation_uncompiled
---

# 开放商业消费者价格币种筛选 V1

## 背景

能力价格使用微单位整数保存，但不同币种的整数不能直接比较。原 PC 界面把价格上限解释为 CNY，服务端却未同时约束能力币种；开放目录扩展到多币种后，这会产生错误匹配。

## 决定

1. 消费者发现请求增加可选 `price_currency`，规范化为三位大写 ASCII 字母代码。
2. 非空非法代码失败关闭；未设置价格上限和币种时保持旧的不限价格行为。
3. 为兼容既有 PC 和偏好档案，存在 `max_unit_price_micros` 而省略币种时默认使用 `CNY`。
4. 显式传入币种但不传上限时，只按币种筛选。
5. 候选过滤必须先要求能力币种完全相等，再比较微单位整数，禁止跨币种数值比较。
6. 规范化后的条件通过 `price_filter` 回显，并进入匹配原因、请求指纹和排序凭证规范负载。
7. PC 默认显示 CNY，只有填写价格上限时发送币种，并显示服务端最终采用的条件。

## 信任边界

- 三位代码只进行结构校验，不证明其属于完整 ISO 4217 注册表。
- 能力价格和币种仍由商户项目声明，不证明结算汇率、税费或最终成交金额。
- 本批不提供汇率换算、多币种预算、价格锁定或支付结算。
- 当前代码未编译，未执行接口、兼容性、排序凭证、浏览器或 UI 验证，状态为 `implementation_uncompiled`。

## 实现入口

- `server/src/open_commerce_consumer_model.rs`
- `server/src/open_commerce_consumer.rs`
- `pc-frontend/src/features/open-commerce/ConsumerPriceFilterFields.tsx`
- `docs/open-commerce-consumer-price-currency-v1-acceptance.md`
