---
title: ADR：业务证据到 ERP/CRM 的显式衔接回执 V1
status: accepted
owner: backend
date: 2026-08-03
---

# ADR：业务证据到 ERP/CRM 的显式衔接回执 V1

## 背景

商户业务证据已经能证明平台完成了一次能力调用，并保存商户运行时返回的结果摘要和可选标准业务回执。但“平台收到了商户结果”不等于“某个 ERP/CRM 接入器已经处理该结果”。如果平台直接根据调用成功自动创建订单、客户或财务记录，就会产生第二套业务账本，并可能把商户声明误写成支付或履约事实。

## 决策

1. 新增独立的业务衔接回执，显式记录某个已登记接入器对指定 Invocation 的处理结果：`applied`、`ignored` 或 `rejected`。
2. 回执必须同时绑定项目、商户、Invocation、接入器、业务证据结果 SHA-256、幂等键、当前用户和调用 App。
3. `applied` 只允许用于成功且带有效标准业务回执的调用，并要求提供外部 ERP/CRM 目标记录号；平台只保存该记录号的 SHA-256。
4. `ignored` 和 `rejected` 不允许提供目标记录号，必须提供有界结果代码。
5. 写入者必须是项目编辑者，并明确确认该回执来自真实 ERP/CRM 处理结果。V1 的权威级别固定为 `project_editor_asserted`。
6. 接入器必须属于同一项目和商户，停用的接入器不能产生新回执。
7. `integration_id + receipt_key` 构成幂等边界。同键同结果返回原回执，同键不同结果失败关闭。
8. 回执不修改 Invocation，不复制商户订单、库存、客户或财务表，也不自动改变接入健康状态。
9. 所有响应固定 `funds_moved=false`；回执不能被解释为支付、分账、履约或退款完成。

## 事实层级

| 层级 | 能证明 | 不能证明 |
|---|---|---|
| Invocation | 平台完成了一次能力调用 | 外部系统已经处理结果 |
| 标准业务回执 | 商户运行时声明业务引用和状态 | 平台独立核验订单、支付或履约 |
| 业务衔接回执 | 项目编辑者声明指定接入器已应用、忽略或拒绝证据 | 外部接入器具有独立机器身份或可验证签名 |
| 商户 ERP/CRM | 商户自己的经营事实 | 一龙自动拥有完整经营数据 |

## 后续边界

生产适配器仍需逐项实现官方授权、密钥保管、写入事务、回读校验和独立机器身份。未来可以新增适配器签名或受控服务身份，但不得把 V1 的人工确认记录静默升级成机器证明。

## 实现证据

- 迁移：`server/src/open_commerce_business_handoff_migration.rs`
- 领域服务：`server/src/open_commerce_business_handoff_service.rs`
- 存储与幂等：`server/src/store/open_commerce_business_handoffs.rs`
- HTTP：`server/src/open_commerce_business_handoff_api.rs`
- MCP：`server/src/open_commerce_business_handoff_mcp.rs`
- PC 工作台：`pc-frontend/src/features/open-commerce/MerchantBusinessHandoffPanel.tsx`
- 测试：`server/src/open_commerce_business_handoff_tests.rs`
