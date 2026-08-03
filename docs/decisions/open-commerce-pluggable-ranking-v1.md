---
title: 开放商业消费者可替换透明排序器 V1
status: accepted
owner: open-commerce
reviewed_at: 2026-08-03
implementation_status: implementation_uncompiled
---

# 开放商业消费者可替换透明排序器 V1

## 背景

开放商业网络不能把发现入口重新固化为单一平台控制的黑箱排序。消费者应当知道当前使用哪种排序规则、规则是否受付费位置影响，并能在有限且明确的策略中自行切换。V1 先在现有公开目录候选集内提供内置透明排序器，为后续第三方排序器契约和联邦治理建立响应结构，但不宣称已经解决全网公平排序。

## 决定

1. 消费者发现请求可选传入 `ranking_policy`；省略或传空值时继续使用 `transparent_preference_match.v1`，保持原有调用兼容。
2. V1 提供五种内置策略：偏好匹配、最低调用价、公开能力优先、最近更新和商户名称稳定排序。
3. 每次响应必须返回当前策略键、名称、解释、是否由用户显式选择、是否存在付费位置，以及全部可用策略描述。
4. 五种策略均固定声明 `paid_placement=false`。平台不得因商户付费改变这些排序结果；未来若引入赞助内容，必须使用独立字段和独立展示，不得伪装成自然排序。
5. 排序只使用商户主动发布目录中的公开字段、能力访问级别、调用单价、更新时间，以及消费者本次显式提供的低敏偏好。
6. 同一策略使用稳定的商户、能力和价格字段作为并列条件，避免同一候选集在无数据变化时随机漂移。
7. 未知策略键失败关闭，不静默回退为其他策略。
8. 排序器先在现有目录查询返回的候选集上运行：请求最多返回 50 条，服务端最多获取 100 个商户候选。本结果不是对全网全部商户的穷举排名。

## 策略语义

| 策略键 | 首要顺序 | 后续稳定条件 |
|---|---|---|
| `transparent_preference_match.v1` | 公开类别、城市、标签、访问方式和价格的匹配分 | 调用价、商户名、商户 ID、能力键 |
| `lowest_unit_price.v1` | 能力调用单价从低到高 | 匹配分及稳定标识 |
| `public_access_first.v1` | 无需额外授权的公开能力优先 | 匹配分、调用价及稳定标识 |
| `recently_updated.v1` | 目录能力最近更新时间优先 | 匹配分、调用价及稳定标识 |
| `merchant_name.v1` | 公开商户名称 | 匹配分、调用价及稳定标识 |

## 边界

- V1 不是第三方排序器 SDK、可下载算法市场、签名排序包或跨运营方排序协议。
- V1 不使用实时距离、地图路况、外部评价、销量、真实库存新鲜度或履约质量；缺少这些字段时不能把结果描述为客观最佳商户。
- `最近更新` 只表示目录能力记录的更新时间，不证明商户数据真实或最新。
- `最低调用价` 只比较公开能力调用单价，不代表商品总价、配送费或最终交易成本最低。
- 用户可选择规则不等于算法公平性已经得到证明；反作弊、虚假资料、利益冲突和联邦治理仍需独立实现。
- 当前代码未编译或运行，状态为 `implementation_uncompiled`。

## 实现入口

- `server/src/open_commerce_consumer_ranking.rs`
- `server/src/open_commerce_consumer_model.rs`
- `server/src/open_commerce_consumer.rs`
- `pc-frontend/src/features/open-commerce/openCommerceClientTypes.ts`
- `pc-frontend/src/features/open-commerce/ConsumerCommerceSandbox.tsx`
- `docs/open-commerce-pluggable-ranking-v1-acceptance.md`
- `docs/decisions/open-commerce-consumer-ranking-receipts-v1.md`
