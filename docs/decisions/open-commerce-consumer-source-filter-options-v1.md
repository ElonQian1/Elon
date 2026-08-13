---
title: 开放商业消费者来源筛选建议 V1
status: accepted
owner: open-commerce
reviewed_at: 2026-08-10
implementation_status: verified_rust_sqlite
---

# 开放商业消费者来源筛选建议 V1

## 背景

消费者 AI 和非技术用户不应依靠记忆手写厂商标识和数据域。发现接口可以从本次公开目录候选中汇总已登记值，但必须同时公开候选范围有限，不能把建议列表描述成全网目录或外部厂商认证结果。

## 决定

1. 消费者发现响应增加 `source_filter_options`，请求和旧筛选语义不变。
2. 选项只汇总当前运营方目录查询返回的最多 100 个商户候选中的有效内部回执来源。
3. 如果请求指定能力 Key，聚合只统计该能力；厂商、数据域、年龄等来源筛选不遮蔽其他可选值。
4. 厂商和数据域按规范化字符串去重并稳定排序，同时返回公开能力数量，不统计商户数量。
5. 响应固定 `scope=current_operator_candidate_window.v1`、`operator_exhaustive=false`。
6. PC 端使用 `datalist` 提供建议，但仍允许用户输入列表外值，最终由服务端规范化和筛选。
7. 本批不提供全局厂商注册表、别名映射、分页聚合或外部平台身份验证。

## 信任边界

- 建议值来自商户项目公开目录声明，不证明外部厂商授权、连接成功或数据真实。
- 能力数量只是本次候选窗口计数，不保证查询后一定产生匹配。
- 空列表不证明系统中不存在其他厂商或数据域。
- 已通过真实 SQLite 夹具验证按公开能力计数、稳定排序、能力 Key 缩小聚合范围、来源条件不遮蔽建议、失效绑定排除和私有字段不泄露；状态为 `verified_rust_sqlite`。
- 已执行携带来源条件的 MCP 与进程内 Axum HTTP 请求并读取筛选建议；尚未执行绑定真实 TCP 端口的请求或 PC 浏览器交互与视觉验证。

## 实现入口

- `server/src/open_commerce_consumer_source_options.rs`
- `server/src/open_commerce_consumer_model.rs`
- `pc-frontend/src/features/open-commerce/ConsumerSourceFilterFields.tsx`
- `docs/open-commerce-consumer-source-filter-options-v1-acceptance.md`
