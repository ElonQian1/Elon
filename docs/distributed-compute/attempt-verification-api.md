---
title: 分布式算力 Attempt Verification 决定
status: current
reviewed_at: 2026-08-05
owners: ai-economy, backend, pc
implementation_status: implementation_uncompiled
---

# 分布式算力 Attempt Verification 决定

## 1. 当前实现

v192、追加式 Store、Service、HTTP 路由、管理员待验证证据链队列与 PC `/compute-verification` 已经写入代码，但尚未编译、执行迁移或运行接口/页面验证，状态固定为 `implementation_uncompiled`。平台 `admin/owner` 可发现已经具备 v189 Provider 候选、v190 消费者审核和 v191 平台观测且尚无 v192 的证据链，并把三者绑定为第一份不可变 Verification 决定。

v192 只记录 `accepted/rejected/disputed` 及其确定性 `verified_usage`、`compensable_usage`。它本身不会生成 Execution Receipt，不会推进 Lease、Job、Reservation 或 Claim，不会消费容量，也不会扣除预授权或释放 Provider 收益；v193 可另行基于 accepted 决定签发回执。

## 2. HTTP 路由

| 方法 | 路径 | 调用者 | 作用 |
|---|---|---|---|
| GET | `/api/admin/compute/attempt-terminal-candidates/pending-verification` | 平台 `admin/owner` | 读取尚无 v192 决定的完整 v189-v191 证据链 |
| POST | `/api/admin/compute/attempt-leases/:lease_id/verification-decision` | 平台 `admin/owner` | 写入第一份 Verification 决定 |
| GET | `/api/me/compute/attempt-leases/:lease_id/verification-decision` | Job 消费者或 Provider 所有者 | 读取并重新审计 Verification 回执 |

POST 必须提供 v189-v191 三份证据的精确 ID 和事件摘要、policy ID/version、决定、至少一个 reason code、外部决定引用、稳定幂等键，并显式设置 `confirm_no_state_or_settlement_effect=true`。

待验证队列只选择消费者审核和平台观测均已存在、且尚无 Verification 决定的候选。Store 在同一连接内重新读取候选、最终 Provider 用量、消费者审核、平台观测和 Reservation 历史版本，并复用 v192 证据绑定审计；队列只读且可能随并发写入过期，因此 POST 仍会在 `BEGIN IMMEDIATE` 事务中重新验证全部 ID、摘要、meter 和唯一性条件。

PC `/compute-verification` 仅向 `admin/owner` 显示导航，展示三方 outcome、Provider/平台 meter、差异与保守计量预览。前端会在消费者未接受、双方 outcome 不一致或平台结果为 `indeterminate` 时禁用 `accepted`，但这只是操作防错，服务端 policy 才是最终权威。

## 3. 保守策略

v192 只支持 `conservative_min_v1@1`：

- `accepted` 仅在消费者审核为 `accepted`、Provider outcome 与平台 observed outcome 一致且平台结果不是 `indeterminate` 时允许；
- 每个 meter 的 `verified_usage` 取 Provider 累计声明值与平台累计观测值的较小值；
- `compensable_usage` 再取 `verified_usage` 与原 Reservation 预留数量的较小值，超额部分不进入首版补偿量；
- `rejected` 或 `disputed` 保存完整 meter 集合，但 verified/compensable 数量均为零；
- reason codes 排序、去重并写入摘要，policy、输入证据和结果用量全部进入事件摘要。

这是一项保守的人工平台策略，不代表平台观测来源已经可信，也不替代签名验证、重复执行、挑战任务或争议裁决。

## 4. 失败关闭与不可变回执

首次写入在同一 `BEGIN IMMEDIATE` 事务中重新读取并审计 v188-v191 证据和精确 Reservation 历史版本。以下任一条件不满足都会拒绝写入：

- 三份证据未绑定同一 Lease、候选、最终用量快照和业务因果链；
- 请求中的任一证据 ID 或事件摘要不匹配；
- Reservation revision/digest 或 Capacity Claim 绑定与候选不一致；
- meter 集合不完整，或 policy/version 不受支持；
- 同一候选、Lease 或幂等键已经绑定不同决定。

数据库触发器禁止更新和删除。每次读取都会重新计算 reason codes、verified/compensable readings、请求摘要和事件摘要，并再次审计全部上游证据。

## 5. 回执效果

- `decision: accepted/rejected/disputed`；
- `verification_effect: verified_usage_recorded/rejection_recorded/dispute_recorded`；
- `execution_receipt_effect: none`；
- Lease、Job、Capacity、Reservation 均为 `unchanged`；
- `money_effect: preauthorization_unchanged`。

因此，`accepted` 表示 v192 policy 已形成验证计量事实，不表示任务生命周期已经终态，也不是付款授权。

## 6. 尚未实现

- Cargo 编译、v192 迁移执行、HTTP 真实调用、并发与故障注入验证；
- PC 构建、接口联调、视觉验收和发布；
- 平台观测来源签名、可信时钟、自动采集、独立验证器和多策略版本治理；
- 多份观测、重复执行、挑战任务、异常检测和争议裁决；
- Execution Receipt 自动签发、Lease/Job 终态、Capacity Claim 消费和 Reservation 消费；v193 已提供管理员签发入口，见 `docs/distributed-compute/attempt-execution-receipt-api.md`；
- 消费者扣款、Provider 收益、退款、Settlement Receipt 和纠正回执。

## 7. 代码入口

- `server/src/store/compute_attempt_verifications.rs`
- `server/src/store/compute_attempt_verifications/`
- `server/src/compute_attempt_verification_migration.rs`
- `server/src/compute_federation_attempt_service.rs`
- `server/src/compute_federation_attempt_api.rs`
- `pc-frontend/src/features/compute-attempt/verificationContracts.ts`
- `pc-frontend/src/features/compute-verification/`
