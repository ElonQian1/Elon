---
version_status: current
status: accepted
owner: open-commerce
updated: 2026-08-13
---

# 消费者订单闭环视图 V1

## 目标

让消费者本人及其 AI 通过一个只读接口，核对同一笔开放商业订单从终态 Invocation、商户标准订单回执到最新 ERP 衔接结果的连续证据。视图只聚合现有真源，不创建第二套订单状态机。

## 范围

- 只接受当前登录用户拥有的终态 Invocation ID。
- 只有商户结果包含有效 `open_commerce.merchant_business_receipt.v1` 且 `entity_type=order` 时生成订单闭环视图。
- 返回消费者可见的调用结果、商户订单声明、ERP 衔接状态和零资金计量边界。
- ERP 投影不得返回原始目标记录号、项目 ID、Integration ID、接入器凭据、租约密钥、Claim ID、内部用户 ID或请求哈希。
- 提供消费者 HTTP 与 MCP 只读入口，共用同一服务。

## 派生状态

- `merchant_confirmed_erp_pending`：有效商户订单回执存在，尚无 ERP 衔接回执。
- `erp_recorded`：最新衔接回执为 `applied`。
- `erp_retry_required`：最新衔接回执为 `rejected`。
- `erp_ignored`：最新衔接回执为 `ignored`。

派生状态只用于解释现有证据，不证明真实支付、配送、履约或退款。

## 验收标准

1. 消费者本人可按 Invocation ID 读取订单闭环，其他用户得到统一不存在。
2. 有效订单回执在无衔接、成功衔接、失败衔接和忽略时得到确定派生状态。
3. 非终态、无效回执和非订单回执失败关闭。
4. 响应不包含项目、Grant、请求哈希、Integration、接入器凭据、Claim、租约密钥或原始 ERP 记录号。
5. `funds_moved` 固定为 `false`，并区分订单金额与平台调用计量。
6. HTTP 与 MCP 共用同一服务，并通过定向 Rust/SQLite 验证。
