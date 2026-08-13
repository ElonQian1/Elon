---
title: 开放商业消费者候选范围摘要 V1
status: accepted
owner: open-commerce
reviewed_at: 2026-08-10
implementation_status: verified_rust_sqlite
---

# 开放商业消费者候选范围摘要 V1

## 背景

排序凭证已经记录候选上限，但普通发现响应只返回匹配结果。消费者 AI 如果看不到目录窗口大小和非穷尽声明，可能把“本次没有结果”错误解释为“全网不存在符合条件的商户”。

## 决定

1. 每次消费者发现响应增加 `candidate_scope`，不要求用户额外生成排序凭证。
2. 范围固定为 `current_operator_public_directory.v1`，并固定 `operator_exhaustive=false`。
3. 当前候选窗口固定为 100，响应公开 `candidate_cap`、`directory_candidate_count`、`eligible_match_count` 和 `returned_match_count`；请求 `limit` 只控制最终返回 1 至 50 条，不改变参与排序的候选集合。
4. `results_truncated` 只在合格匹配数大于实际返回数时为真。
5. 目录候选数等于候选上限不能用来推断全网总量，也不代表当前运营方目录已全部扫描。
6. PC 使用无框摘要行显示计数、截断状态和“非全网穷尽”，不在结果容器中嵌套卡片。
7. 本批不提供全局游标分页、跨运营方聚合或全网基数估计；固定窗口也不代表前 100 个候选具有全网代表性。

## 信任边界

- 候选范围只描述当前运营方本次查询窗口，不证明目录完整或排序公平。
- 合格数是本次代码过滤结果，不证明商户数据真实、授权有效或能力可成功调用。
- 空结果不能作为不存在其他商户、来源或能力的证明。
- 已通过真实 SQLite 夹具验证空结果、单结果、截断、固定 100 候选窗口、同时间戳目录并列稳定性、请求上限及普通响应与排序凭证计数一致；状态为 `verified_rust_sqlite`。
- 进程内 Axum HTTP 已验证候选范围投影；真实 TCP、PC 浏览器、窄屏布局、跨运营方聚合和目录完整性仍未验证。

## 实现入口

- `server/src/open_commerce_consumer_model.rs`
- `server/src/open_commerce_consumer.rs`
- `server/src/open_commerce_consumer_ranking_tests.rs`
- `pc-frontend/src/features/open-commerce/ConsumerCandidateScopeSummary.tsx`
- `docs/open-commerce-consumer-candidate-scope-v1-acceptance.md`
