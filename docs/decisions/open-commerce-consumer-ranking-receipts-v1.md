---
title: 开放商业消费者排序凭证 V1
status: accepted
owner: open-commerce
reviewed_at: 2026-08-03
implementation_status: implementation_uncompiled
---

# 开放商业消费者排序凭证 V1

## 背景

透明排序器已经公开策略名称和解释，但一次具体发现响应仍缺少可下载的机器可读证据。消费者或其 AI 无法在离开当前页面后复核当时使用的策略、候选范围和返回顺序。V1 提供由消费者显式请求、客户端复核后下载的临时排序凭证，不新增平台跟踪数据库。

## 决定

1. 发现请求新增可选 `include_ranking_receipt`，默认 `false`；只有用户显式勾选才生成凭证。
2. 凭证负载记录生成时间、当前运营方公开目录候选范围、候选上限、实际候选数、符合条件数、返回数、排序策略版本、非付费标志和有序结果摘要。
3. 每条有序结果只包含公开商户 ID、能力键与版本、匹配分、访问级别、调用单价、币种、目录更新时间，以及商户声明的来源和新鲜度快照，不包含授权 Grant、消费者身份或原始请求。来源与新鲜度按 `open-commerce-public-data-provenance-v1` 解释，固定不代表外部平台核验。
4. 搜索词、能力条件、App 身份和低敏偏好只进入临时规范输入的 SHA-256 指纹；凭证明文不保存这些输入，服务端也不持久化凭证。
5. 服务端返回精确的 `canonical_payload_json` 及其 SHA-256。PC 收到后使用 Web Crypto 重新计算摘要，失败时阻断展示和下载；下载时再次复核。
6. 当前候选范围固定为本运营方公开目录、最多 100 个商户，并明确 `operator_exhaustive=false`；凭证不能被描述为全网穷举。
7. `signed_by_operator` 固定为 `false`。V1 只有完整性摘要，不提供运营方签名、时间戳机构或跨运营方信任链。

## 边界

- SHA-256 只能帮助发现下载负载是否变化，不证明是谁生成了凭证。
- 凭证不证明商户资料、价格、库存、更新时间、评分或履约状态真实。
- 凭证不证明排序公平、没有遗漏商户、没有目录层过滤或不存在运营方利益冲突。
- V1 不持久化、不可在平台内重新查询，不进入消费者 V5 可携带数据包。
- V1 不是第三方排序器签名、区块链存证、可信时间戳、零知识证明或监管审计报告。
- 当前代码未编译，未验证服务端序列化、浏览器摘要、下载文件或 UI，状态为 `implementation_uncompiled`。

## 实现入口

- `server/src/open_commerce_consumer_model.rs`
- `server/src/open_commerce_consumer.rs`
- `pc-frontend/src/features/open-commerce/openCommerceClientTypes.ts`
- `pc-frontend/src/features/open-commerce/consumerRankingReceipt.ts`
- `pc-frontend/src/features/open-commerce/ConsumerCommerceSandbox.tsx`
- `docs/open-commerce-consumer-ranking-receipts-v1-acceptance.md`
