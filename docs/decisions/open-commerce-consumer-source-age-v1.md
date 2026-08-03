---
title: 开放商业消费者内部回执年龄筛选 V1
status: accepted
owner: open-commerce
reviewed_at: 2026-08-03
implementation_status: implementation_uncompiled
---

# 开放商业消费者内部回执年龄筛选 V1

## 背景

商户声明期表达商户愿意为能力数据承担多长时间的新鲜度声明，但消费者 AI 还需要独立限制内部同步回执距本次查询的最长时间。该限制必须基于目录已经公开的内部回执完成时间，不能被解释为外部平台实时性证明。

## 决定

1. 消费者发现请求增加可选 `max_source_age_seconds`，省略时保持旧行为。
2. 允许范围为 1 秒至 31,536,000 秒（365 天）；越界值失败关闭。
3. 填写该字段会派生 `internal_sync_receipt` 来源要求，普通商户声明不会命中。
4. 年龄以单次发现开始时刻减去 RFC 3339 内部回执完成时间计算；完成时间缺失、格式错误或晚于发现时刻时失败关闭。
5. 最大年龄通过 `source_filter.max_age_seconds` 回显，并进入匹配原因、请求指纹和排序凭证规范负载。
6. 该条件可与厂商、数据域、声明期、能力键、价格和排序器组合，不改变排序得分。
7. PC 端以分钟输入，服务端仍以秒作为稳定接口单位。

## 信任边界

- 内部回执由商户项目生成，不证明美团等外部平台签发、授权有效或数据真实。
- 年龄命中只证明登记完成时间满足本次查询阈值，不证明数据覆盖完整或库存仍然可售。
- 该能力不是支付、下单或外部平台实时回读证明。
- 当前代码未编译，未执行时间边界、组合筛选、排序凭证、兼容性或 PC 验证，状态为 `implementation_uncompiled`。

## 实现入口

- `server/src/open_commerce_consumer_model.rs`
- `server/src/open_commerce_consumer.rs`
- `pc-frontend/src/features/open-commerce/ConsumerCommerceSandbox.tsx`
- `docs/open-commerce-consumer-source-age-v1-acceptance.md`
