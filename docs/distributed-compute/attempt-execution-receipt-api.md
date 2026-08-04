---
title: 分布式算力 Attempt Execution Receipt
status: current
reviewed_at: 2026-08-05
owners: ai-economy, backend, pc
implementation_status: implementation_uncompiled
---

# 分布式算力 Attempt Execution Receipt

## 1. 当前实现

v193、追加式 Store、独立 Service、HTTP 路由、管理员待签发队列与 PC `/compute-receipts` 已经写入代码，但尚未编译、执行迁移或运行接口/页面验证，状态固定为 `implementation_uncompiled`。平台 `admin/owner` 可发现尚无 v193 的 `accepted` v192 Verification，并基于精确决定签发第一份不可变 Execution Receipt。

Execution Receipt 固化执行事实，不负责修改业务状态或移动资金。v193 不推进 Lease、Job、Reservation、Capacity Claim，不消费容量，不扣除预授权，也不释放 Provider 收益。

## 2. HTTP 路由

| 方法 | 路径 | 调用者 | 作用 |
|---|---|---|---|
| GET | `/api/admin/compute/attempt-verifications/pending-execution-receipt` | 平台 `admin/owner` | 读取尚无 v193 的 accepted Verification 与终态候选 |
| POST | `/api/admin/compute/attempt-leases/:lease_id/execution-receipt` | 平台 `admin/owner` | 基于 accepted Verification 签发回执 |
| GET | `/api/me/compute/attempt-leases/:lease_id/execution-receipt` | Job 消费者或 Provider 所有者 | 读取并重新审计 Execution Receipt |

POST 必须提供精确 Verification 决定 ID 和事件摘要、稳定幂等键，并显式设置 `confirm_execution_receipt_only=true`。

待签发队列排除 `rejected/disputed` 及已有回执的决定。Store 在同一连接内重新读取并审计 Attempt 激活、Job/Reservation 历史版本、最终用量和 v189-v192 完整证据链；队列只读且可能随并发签发过期，因此 POST 仍会在事务内重新检查 accepted 状态、精确摘要、唯一性和幂等键。

PC `/compute-receipts` 仅向 `admin/owner` 显示导航，展示 accepted Verification、终态结果、输出工件引用和 verified/compensable meter。管理员必须明确确认后才能签发；页面不读取或验证工件正文，也不会把已有 Verification 自动升级为 Execution Receipt。

## 3. 回执内容

v193 在一个事务中重新读取并审计 Attempt 激活、精确 Job/Reservation 历史版本和 v188-v192 全部证据，然后生成已有领域合同 `compute_federation.execution_receipt.v1`：

- Job、Reservation、Lease、Attempt 编号与 fencing generation；
- Provider、Executor、Offer 历史版本和摘要；
- Workload 固定的 runner、plugin、model、tokenizer 与输入工件摘要；
- Provider 终态、输出摘要和结果工件；
- declared、observed、verified、compensable 四类用量；
- Provider 候选、消费者审核、平台观测三项证明；
- v192 policy、reason codes、决定摘要和时间；
- 激活、完成、签发时间和完整回执摘要。

没有固定 runtime/runner 摘要的 Workload 不能签发。`rejected/disputed` Verification 也不能签发。

## 4. 失败关闭与不可变审计

同一 Verification 决定和 Lease 只允许一份回执；同一签发者幂等键不能绑定不同请求。签发前会重新核对：

- v188-v192 是否绑定同一候选、用量快照和业务因果链；
- Attempt 激活中的 executor、attempt number、fencing、Job 和 Reservation；
- Job/Reservation 精确历史 revision/digest、Offer 与 Capacity Claim；
- 完成时间是否晚于或等于激活时间；
- Verification 是否为 `accepted`。

数据库触发器禁止更新和删除。每次读取都会重建完整 Execution Receipt 并重新计算摘要；任何源证据、JSON 内容、幂等字段或时间字段不一致都会失败关闭。

## 5. 回执效果

- `execution_effect: execution_receipt_recorded`；
- Lease、Job、Capacity、Reservation 均为 `unchanged`；
- `money_effect: preauthorization_unchanged`。

因此，Execution Receipt 是后续状态推进和结算的输入，不是“已经完成扣款”的证明。

## 6. 尚未实现

- Cargo 编译、v193 迁移执行、HTTP 真实调用、并发与故障注入验证；
- PC 构建、接口联调、视觉验收和发布；
- NodeAgent 到云端的签名事件传输、真实工件取回与恶意内容扫描；
- 自动签发、多独立回执、挑战和争议裁决；
- v193 本身不推进 Lease/Job，也不消费 Capacity Claim 与 Reservation；后续 v194 已写入独立可信终态事务，但仍为 `implementation_uncompiled`；
- Settlement Receipt、消费者扣款、Provider 收益、退款和纠正回执。

后续状态与容量效果见 `docs/distributed-compute/attempt-finalization-api.md`。

## 7. 代码入口

- `server/src/store/compute_attempt_execution_receipts.rs`
- `server/src/store/compute_attempt_execution_receipts/`
- `server/src/compute_attempt_execution_receipt_migration.rs`
- `server/src/compute_federation_attempt_receipt_service.rs`
- `server/src/compute_federation_attempt_api.rs`
- `pc-frontend/src/features/compute-attempt/executionReceiptContracts.ts`
- `pc-frontend/src/features/compute-receipts/`
