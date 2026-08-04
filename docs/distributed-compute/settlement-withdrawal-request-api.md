---
title: 分布式算力 Provider 提现申请与内部冻结
status: current
reviewed_at: 2026-08-05
owners: ai-economy, backend
---

# 分布式算力 Provider 提现申请与内部冻结

## 1. 当前实现

v200、追加式 Store、独立 Service 与 Provider 本人 HTTP 路由已经写入代码，但尚未编译、执行迁移或运行接口验证，状态固定为 `implementation_uncompiled`。

Provider 所有者可把本人结算账户中的一笔 CNY `available` 余额原子转入 `withdrawn` 保留区，并取得不可变 Withdrawal Request Receipt、Posting 和两条账本腿。该动作只创建提款申请和内部资金冻结，不执行银行付款、数字钱包转账、Sui 交易或任何外部清算。

## 2. HTTP 路由

| 方法 | 路径 | 作用 |
|---|---|---|
| POST | `/api/me/compute/providers/:provider_id/settlement-withdrawals` | Provider 所有者创建提款申请并冻结内部余额 |
| GET | `/api/me/compute/providers/:provider_id/settlement-withdrawals` | Provider 所有者分页读取并重审计本人申请 |
| GET | `/api/me/compute/providers/:provider_id/settlement-withdrawals/:withdrawal_id` | Provider 所有者读取并重审计一份申请 |

写请求使用整数 `amount_micros`、稳定幂等键和一个外部目标引用。目标类型只允许：

- `bank_account_vault_ref`；
- `digital_wallet_vault_ref`；
- `sui_address_ref`；
- `other_vault_ref`。

`destination_ref` 是外部金库或公开地址的引用，不是银行卡密码、私钥、助记词或其它秘密。调用者必须显式确认该接口只冻结内部余额，且目标引用不含秘密。

## 3. 所有权与并发门卫

服务端从当前 Provider 注册回执派生结算账户，不接受调用者伪造账户归属。一次申请必须同时匹配：

1. 当前登录用户是 Provider 的 `owner_account_id`；
2. Provider ID、策略版本和 Provider Digest 与事务内当前版本一致；
3. 结算账户等于 Provider 的 `settlement_account_id`，未配置时回退到所有者账户；
4. 币种固定为 CNY，金额为大于零的整数微单位；
5. available 余额足以覆盖申请金额；
6. 相同幂等键只能重放完全相同的请求。

账户更新同时比较 revision、旧 available 和旧 withdrawn。并发请求只有一个能够成功修改指定旧状态，其余请求失败关闭或按原请求重放。

## 4. 原子账本

单一 `BEGIN IMMEDIATE` 事务完成：

- Provider available 借记；
- Provider withdrawn 保留区贷记；
- Withdrawal Request Posting 与两条不可变账本腿写入；
- Provider 版本引用、请求 JSON、回执 JSON、请求摘要和事件摘要写入。

任一步失败时全部回滚。读取时重新核对历史 Provider 版本、所有权、Posting 摘要、两条账本腿、历史余额快照，以及当前 available/withdrawn 是否可由 v198 Release、v200 Withdrawal Request 与可选 v201 Terminal 的不可变账本重建。

## 5. 状态语义

`available` 表示已度过当前结算挑战门卫、可进入提款流程的内部余额。`withdrawn` 在 v200 中表示已经从 available 隔离、等待独立终态处理的提款保留额，不单独证明钱已离开平台。

v200 回执固定声明：

- `fund_effect=provider_available_moved_to_withdrawn_reserve`；
- `external_transfer_effect=not_executed`。

v201 使用独立追加式记录表达取消、拒绝或“管理员登记外部付款证据”，不改写 v200 申请，也不把管理员声明伪装成链上或银行系统的自动证明。边界见 `docs/distributed-compute/settlement-withdrawal-terminal-api.md`。

## 6. PC 本人入口

`pc-frontend/src/features/compute-settlement/` 已写入 `/my-compute-settlement` 页面源码。登录用户可切换本人 Provider、查看经账本重建的余额和提款终态，并在 available 大于零时打开提款申请对话框。表单把人民币输入转换为整数微单位，要求目标引用非空，并强制确认“只冻结内部余额”和“引用不含秘密”；成功后重新读取账户与队列。

本人可对仍为 pending 的申请执行显式取消。页面使用 v200 Event Digest 与 Request Posting ID/Digest 构造 v201 取消请求，服务端仍负责最终所有权、唯一终态和余额审计。该页面尚未构建、运行、视觉验收或发布。

## 7. 尚未实现

- Cargo 编译、v200 迁移执行、HTTP 真实调用、并发与故障注入验证；
- 自动银行打款、支付机构清算、钱包签名或 Sui 链上提交；
- 外部付款证据自动拉取、签名验证或链上确认；
- 提现风控、KYC、多币种、手续费、税务、生产密钥和对账文件。

因此，v200 不能被描述为“真实提现已经完成”。

## 8. 代码入口

- `server/src/store/compute_settlement_withdrawal_requests.rs`
- `server/src/store/compute_settlement_withdrawal_requests/`
- `server/src/compute_settlement_withdrawal_request_migration.rs`
- `server/src/compute_federation_settlement_withdrawal_request_service.rs`
- `server/src/compute_federation_settlement_withdrawal_request_api.rs`
- `pc-frontend/src/features/compute-settlement/MyComputeSettlementPage.tsx`
- `pc-frontend/src/features/compute-settlement/WithdrawalRequestDialog.tsx`

上游 pending 到 available 释放见 `docs/distributed-compute/attempt-settlement-release-api.md`，下游唯一终态见 `docs/distributed-compute/settlement-withdrawal-terminal-api.md`。
