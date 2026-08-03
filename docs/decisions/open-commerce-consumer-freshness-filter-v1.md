---
title: 开放商业消费者声明期筛选 V1
status: accepted
owner: open-commerce
reviewed_at: 2026-08-03
implementation_status: implementation_uncompiled
---

# 开放商业消费者声明期筛选 V1

## 背景

公开能力已经暴露商户声明来源与 `current/stale/unknown` 新鲜度，但仅展示状态不能满足“只让 AI 使用仍在声明有效期内数据”的消费者选择。平台也不应默认隐藏未知或过期声明，以免把未核验时间状态变成暗中排序权。

## 决定

1. 消费者发现请求增加 `require_current_declaration`，默认 `false`，保持旧客户端和旧查询结果不变。
2. 用户显式开启后，只保留 `freshness.status=current` 的能力；`stale` 和 `unknown` 均被排除。
3. 筛选发生在每个商户的能力候选选择之前，不修改目录发布状态、能力状态、匹配分或排序器规则。
4. 响应以 `freshness_requirement=current_declaration` 回显用户选择，并在每条结果原因中说明该能力符合声明有效期要求。
5. 排序凭证输入指纹和规范负载都记录筛选条件，防止开启与关闭筛选的结果被描述为同一次发现。
6. V1 只有用户主动筛选，不自动下架、降权、告警或阻断调用，也不改变授权、计量和结算。

## 信任边界

- `current` 仍是商户项目基于能力声明更新时间作出的承诺，不是外部平台实时回读或第三方核验。
- 开启筛选不保证库存充足、价格正确、商户营业、运行时在线或交易成功。
- 关闭筛选不代表消费者认可过期数据，只表示目录不会替用户作隐藏决定。
- V1 不提供按分钟、来源、外部回执或行业 SLA 的高级过滤表达式。
- 当前代码未编译、未执行接口、排序凭证、兼容性或 PC 验证，状态为 `implementation_uncompiled`。

## 实现入口

- `server/src/open_commerce_consumer_model.rs`
- `server/src/open_commerce_consumer.rs`
- `pc-frontend/src/features/open-commerce/openCommerceClientTypes.ts`
- `pc-frontend/src/features/open-commerce/ConsumerCommerceSandbox.tsx`
- `docs/open-commerce-consumer-freshness-filter-v1-acceptance.md`
