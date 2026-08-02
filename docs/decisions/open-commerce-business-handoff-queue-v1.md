---
title: ADR：开放商业业务衔接待办队列 V1
status: accepted
owner: backend
date: 2026-08-03
---

# ADR：开放商业业务衔接待办队列 V1

## 背景

商户已经可以读取终态业务证据，并由项目编辑者记录 ERP/CRM 显式衔接回执。但仅靠人工浏览证据和回执，无法稳定识别哪些结果尚未处理、哪些处理失败需要重试。若再建立一张可手工修改的待办状态表，会与 Invocation 和衔接回执形成重复事实源。

## 决策

1. 待衔接队列不新增持久化状态表，由终态 Invocation、结果摘要和该 Invocation 的最新衔接回执实时派生。
2. 没有衔接回执的证据状态为 `pending`；最新回执为 `rejected` 时状态为 `retry_required`。
3. 最新回执为 `applied` 或 `ignored` 时，证据自动移出队列；后续若追加更新的 `rejected` 回执，则重新进入队列。
4. “最新”按 `completed_at`、`created_at`、`id` 降序确定，避免读取顺序不稳定。
5. 队列只包含具有结果摘要、可以绑定衔接回执的终态调用。`can_apply=true` 仍必须同时满足调用成功和标准业务回执有效。
6. HTTP、MCP 和 PC 使用同一领域服务；项目成员可读，写入仍走原有项目编辑者确认回执流程。
7. 队列只提示工作，不自动调用外部 ERP/CRM，不创建订单、客户、库存或财务记录，不移动资金。

## 实现证据

- 派生查询：`server/src/store/open_commerce_business_handoffs.rs`
- 领域服务：`server/src/open_commerce_business_handoff_service.rs`
- HTTP：`server/src/open_commerce_business_handoff_api.rs`
- MCP：`server/src/open_commerce_business_handoff_mcp.rs`
- PC 队列：`pc-frontend/src/features/open-commerce/MerchantBusinessHandoffQueue.tsx`
- 测试：`server/src/open_commerce_business_handoff_tests.rs`、`scripts/test-open-commerce-pc-workspace.js`

## 后续边界

生产接入器主动拉取、机器身份、签名回执、外部系统回读与自动重试仍需单独设计。未来自动化只能消费该队列并写入可验证回执，不能绕过现有确认和事实边界。
