---
title: 开放商业消费者来源厂商与数据域筛选 V1
status: accepted
owner: open-commerce
reviewed_at: 2026-08-03
implementation_status: implementation_uncompiled
---

# 开放商业消费者来源厂商与数据域筛选 V1

## 背景

内部同步回执来源已经公开厂商标识和单个数据域。消费者 AI 除了要求“必须有内部回执”，还可能需要明确查找某个来源厂商或数据域，但过滤必须作用于商户已发布的结构化元数据，不能根据名称模糊猜测或暗示厂商官方背书。

## 决定

1. 消费者发现请求增加可选 `source_provider_key` 和 `source_data_domain`，两者均省略时保持旧行为。
2. 厂商标识复用接入控制面的规范，数据域复用单项标识规范；空白按未设置处理，非法长度或字符失败关闭。
3. 过滤使用规范化后的精确匹配。普通商户声明没有厂商和数据域字段，因此填写任一条件后自然只返回有效内部回执来源。
4. 响应以 `source_filter` 回显规范化条件；填写任一条件时，`source_requirement` 同时派生为 `internal_sync_receipt`。
5. 匹配原因显示命中的厂商标识和数据域。请求指纹与排序凭证规范负载固化完整 `source_filter`。
6. 厂商、数据域、内部回执要求和声明期要求可组合，过滤发生在能力候选选择之前，不改变排序规则和得分。
7. 本批不提供模糊匹配、多值 OR、排除表达式或外部厂商目录，也不验证真实平台连接。

## 信任边界

- 厂商标识和数据域由商户项目登记，不证明厂商授权、接口可用或数据真实。
- 精确命中只说明目录字段相等，不证明查询覆盖全部商户或全部来源。
- 回执摘要、状态和时间仍固定 `externally_verified=false`。
- 当前代码未编译，未执行规范化、组合筛选、排序凭证、兼容性或 PC 验证，状态为 `implementation_uncompiled`。

## 实现入口

- `server/src/open_commerce_consumer_model.rs`
- `server/src/open_commerce_consumer.rs`
- `pc-frontend/src/features/open-commerce/ConsumerCommerceSandbox.tsx`
- `docs/open-commerce-consumer-source-filters-v1-acceptance.md`
