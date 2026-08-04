---
title: 分布式算力 Attempt 平台终态观测证据
status: current
reviewed_at: 2026-08-04
owners: ai-economy, backend
---

# 分布式算力 Attempt 平台终态观测证据

## 1. 当前实现

v191、追加式 Store、Service 与 HTTP 路由已经写入代码，但尚未编译、执行迁移或运行接口验证，状态固定为 `implementation_uncompiled`。本控制面允许平台 `admin/owner` 为精确 v189 Provider 终态候选登记第一份平台观测证据，并保存与最终 v188 Provider 声明不同的 meter。

平台观测值仍是 Verification 的输入，不是 `verified_usage`。即使来源标记为 `server_metering`，v191 也不会生成 Execution Receipt、推进可信终态、消费容量、扣除预授权或释放 Provider 收益。

## 2. HTTP 路由

| 方法 | 路径 | 调用者 | 作用 |
|---|---|---|---|
| POST | `/api/admin/compute/attempt-leases/:lease_id/terminal-candidate/platform-observation` | 平台 `admin/owner` | 登记第一份平台终态观测证据 |
| GET | `/api/me/compute/attempt-leases/:lease_id/terminal-candidate/platform-observation` | Job 消费者或 Provider 所有者 | 读取并重新审计平台观测回执 |

写请求必须提供精确候选 ID/事件摘要、观测来源、观测系统引用、观测 outcome、完整累计 meter、至少一个外部证据引用、稳定幂等键，并显式设置 `confirm_platform_observation_only=true`。

## 3. 观测来源与结果

`observation_source` 只允许：

- `control_plane`：平台控制面可直接观测的时间、状态或计量事实；
- `transport_gateway`：平台传输层、网关或流量计量产生的事实；
- `server_metering`：受管服务端计量组件产生的事实。

`observed_outcome` 只允许 `succeeded`、`failed`、`canceled` 或 `indeterminate`。它可以与 Provider outcome 不同；v191 只保存差异，不自动判定任何一方错误。

## 4. 用量与差异

平台累计观测必须精确覆盖最终 v188 快照的完整 meter 集合，每项数量为非负整数。服务端排序 meter，并按平台观测来源、观测系统引用、服务端时间和数量生成独立 reading 摘要及整体摘要。

Store 会比较 Provider 声明和平台观测，将数量不同的 meter 保存到 `variance_meters`。差异存在不等于 Provider 欺诈；没有差异也不等于结果或用量已经验证通过。后续 Verification 仍需考虑策略、签名、来源可信度、消费者证据、重复执行和挑战结果。

## 5. 失败关闭与不可变回执

首次写入在同一 `BEGIN IMMEDIATE` 事务中执行。Store 会重新审计 v189 候选和候选绑定的最终 v188 快照，并检查候选、Lease、Job、Reservation、Claim、fencing、快照序号及摘要完全一致。同一候选和 Lease 只允许第一份平台观测；相同管理员幂等键不能绑定不同请求。

每份回执固定平台观测 ID、候选 ID/事件摘要、完整业务因果链、最终 Provider 用量摘要、Provider/平台 outcome、观测来源、累计观测值、差异 meter、证据引用、请求摘要、事件摘要、操作者与服务端时间。数据库触发器禁止更新和删除，读取时重新计算并对账。

回执效果固定为：

- `evidence_status: "unverified_platform_observation"`；
- `observation_effect: "platform_evidence_recorded"`；
- `verification_effect: "none"`；
- Lease、Job、Capacity、Reservation 均为 `unchanged`；
- `money_effect: "preauthorization_unchanged"`。

## 6. 尚未实现

- Cargo 编译、v191 迁移执行、HTTP 真实调用、并发与故障注入验证；
- 控制面、网关和 server metering 组件自动写入及签名验证；
- 多份独立平台观测、跨来源仲裁和可信时间证明；
- 自动 Verification、独立验证器、多策略治理与可信终态；v192 已提供管理员触发的首版保守决定，见 `docs/distributed-compute/attempt-verification-api.md`；
- 消费者争议裁决、挑战任务、重复执行和异常检测；
- Execution Receipt 自动签发、容量消费、扣款、Provider 收益和结算纠正；v193 已提供管理员签发入口，见 `docs/distributed-compute/attempt-execution-receipt-api.md`。

## 7. 代码入口

- `server/src/store/compute_attempt_platform_observations.rs`
- `server/src/store/compute_attempt_platform_observations/`
- `server/src/compute_attempt_platform_observation_migration.rs`
- `server/src/compute_federation_attempt_service.rs`
- `server/src/compute_federation_attempt_api.rs`
