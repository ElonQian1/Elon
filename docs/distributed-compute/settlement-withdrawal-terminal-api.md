---
title: 分布式算力 Provider 提款唯一终态
status: current
reviewed_at: 2026-08-05
owners: ai-economy, backend
---

# 分布式算力 Provider 提款唯一终态

## 1. 当前实现

v201、追加式 Store、独立 Service 与 Provider/管理员 HTTP 路由已经写入代码，但尚未编译、执行迁移或运行接口验证，状态固定为 `implementation_uncompiled`。

每份 v200 Withdrawal Request 最多绑定一份不可变 Terminal Receipt。终态只能是：

- `cancelled`：Provider 所有者主动取消，冻结额全额返回 available；
- `rejected`：平台管理员拒绝，冻结额全额返回 available；
- `external_paid_attested`：平台管理员声明外部付款已经完成，并登记证据引用与摘要；内部余额保持不变。

终态不会改写 v200 申请。不同动作、不同操作人或不同证据不能覆盖已经存在的终态。

## 2. HTTP 路由

| 方法 | 路径 | 调用者 | 作用 |
|---|---|---|---|
| POST | `/api/me/compute/providers/:provider_id/settlement-withdrawals/:withdrawal_id/cancellation` | Provider 所有者 | 取消本人提款申请并返还内部余额 |
| GET | `/api/me/compute/providers/:provider_id/settlement-withdrawals/:withdrawal_id/terminal` | Provider 所有者 | 读取并重审计本人提款终态 |
| GET | `/api/admin/compute/settlement-withdrawals/:withdrawal_id` | 平台 `admin/owner` | 管理侧读取并重审计 v200 申请 |
| POST | `/api/admin/compute/settlement-withdrawals/:withdrawal_id/terminal` | 平台 `admin/owner` | 拒绝申请或登记外部已付款声明 |
| GET | `/api/admin/compute/settlement-withdrawals/:withdrawal_id/terminal` | 平台 `admin/owner` | 管理侧读取并重审计唯一终态 |

写请求必须精确绑定 v200 Withdrawal Event Digest、Request Posting ID 和 Posting Digest，并提供稳定幂等键。Provider 所有者不能登记管理员终态，管理员不能使用所有者取消动作。

## 3. 取消与拒绝

`cancelled` 和 `rejected` 在一个 `BEGIN IMMEDIATE` 事务中完成：

- Provider withdrawn 保留区借记申请全额；
- Provider available 贷记同额；
- Terminal Posting 与两条不可变账本腿写入；
- 唯一 Terminal Receipt、请求 JSON、回执 JSON 和摘要写入。

账户更新比较 revision、旧 available 和旧 withdrawn，防止并发覆盖。返还额必须严格等于原申请额，不支持部分取消或部分拒绝。

## 4. 外部已付款声明

`external_paid_attested` 只允许平台管理员在确认外部付款已经完成后登记。请求必须提供：

- 证据类型：`bank_receipt`、`payment_provider_receipt`、`sui_transaction_digest` 或 `other_receipt`；
- 外部证据引用；
- 64 位十六进制证据摘要；
- 原因代码和稳定幂等键。

该动作保存 Terminal Posting，但没有资金账本腿，也不修改账户 revision、available 或 withdrawn。回执固定声明：

- `fund_effect=provider_withdrawn_balance_retained`；
- `external_transfer_effect=external_payment_attested_not_executed_or_verified`。

因此，该终态只证明某位平台管理员提交了一份可审计声明，不证明本接口发起了付款，也不证明银行、支付机构或 Sui 网络已经被平台自动核验。证据引用不得包含银行卡密码、私钥、助记词或生产支付凭据。

## 5. 读取审计

每次读取都会重新核对：

- 请求 JSON、回执 JSON、数据库列、幂等作用域和事件摘要；
- v200 Withdrawal Request Receipt、Request Posting 及其摘要；
- 动作、角色、原因、外部证据和资金效果是否一致；
- Terminal Posting 摘要；
- 取消/拒绝必须恰好有两条返还账本腿，付款声明必须没有资金腿；
- 当前 available/withdrawn 是否可由 v198 Release、v200 Request 和 v201 Terminal 的不可变账本重建。

## 6. 尚未实现

- Cargo 编译、v201 迁移执行、HTTP 真实调用、并发与故障注入验证；
- 银行、支付机构、钱包或 Sui 网络的真实付款适配器；
- 外部回执自动拉取、签名核验、链上确认数和对账文件验证；
- 部分付款、手续费、税务、KYC、风控、多币种和批量提款；
- 平台 available 收益提款，以及已付款后的冲正或追索。

因此，v201 不能被描述为生产提现通道已经上线。

## 7. 代码入口

- `server/src/store/compute_settlement_withdrawal_terminals.rs`
- `server/src/store/compute_settlement_withdrawal_terminals/`
- `server/src/compute_settlement_withdrawal_terminal_migration.rs`
- `server/src/compute_federation_settlement_withdrawal_terminal_service.rs`
- `server/src/compute_federation_settlement_withdrawal_terminal_api.rs`

上游提款申请见 `docs/distributed-compute/settlement-withdrawal-request-api.md`。
