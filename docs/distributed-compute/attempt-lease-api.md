---
title: 分布式算力 Attempt Lease 状态与续租控制面
status: current
reviewed_at: 2026-08-05
owners: backend, node, ai-economy
implementation_status: implementation_uncompiled
---

# 分布式算力 Attempt Lease 状态与续租控制面

## 1. 当前状态

v186、Store、Service、HTTP 路由与 PC `/compute-execution` Lease 控制面已经写入代码，但尚未编译、执行迁移或运行接口/页面验证，状态固定为 `implementation_uncompiled`。本控制面在 v185 激活回执之外维护可版本化的 Lease 当前状态，并允许 Provider 所有者依据外部执行器心跳声明延长软期限。

`executor_heartbeat_ref` 只是调用方提交的外部证据引用。当前平台不读取证明正文、不验证执行器签名，也不发送 `RenewLease` 节点命令，因此续租成功不能证明节点真实在线或任务真实运行。

## 2. HTTP 接口

| 方法 | 路径 | 权限 | 作用 |
|---|---|---|---|
| POST | `/api/me/compute/providers/:provider_id/attempt-leases/:lease_id/renewals` | Provider 所有者 | 基于当前精确状态登记一次外部心跳声明并续租 |
| GET | `/api/me/compute/attempt-leases/:lease_id/state` | Provider 所有者或 Job 消费者 | 读取并重新审计 Lease 当前状态 |

写请求必须提供当前 `expected_lease_revision`、`expected_lease_digest`、`expected_fencing_generation`、外部心跳引用、新软期限、幂等键，并显式设置 `confirm_executor_alive=true`。确认字段只表示操作者主动声明，不替代可信执行器签名。

PC `/compute-execution` 可按稳定 Lease ID 读取激活回执和当前状态，并在界面中带入当前 revision、digest 与 fencing generation 发起续租。页面只登记外部心跳引用和新的软期限，不发送 `RenewLease` 命令，也不把续租成功展示为实际执行或平台验证。

## 3. 状态与时间边界

v186 为每个 v185 激活回执建立 Lease 当前状态投影；历史激活在迁移时回填为 revision 1，新激活则在原激活事务中同步初始化。

一次续租只允许：

- 当前状态为 `staging` 或 `running`；
- revision、Lease 摘要和 `fencing_generation` 与请求精确一致；
- 服务端续租时间早于当前软期限和不可变硬期限；
- 新软期限晚于当前软期限，且不超过 `hard_deadline_at`；
- Provider 仍为调用者所有，并处于 `active` 或 `draining`。

续租后状态固定为 `running`，revision 增加 1，`last_heartbeat_at` 使用服务端时间。已经过期的 Lease 不可复活；重试必须创建新的 Attempt，并递增 `attempt_no` 和 `fencing_generation`。

## 4. 原子效果

一次成功续租在同一个 `BEGIN IMMEDIATE` 事务中完成：

1. 核对 Provider 所有权、Lease 当前投影及精确版本栅栏；
2. 核对软期限、硬期限和 fencing 代次；
3. 以 CAS 更新 Lease 当前投影；
4. 保存不可更新、不可删除的续租回执；
5. 记录请求摘要、事件摘要、外部心跳引用、操作者和服务端时间。

并发修改导致 CAS 不匹配时失败关闭，调用方必须重新读取当前状态。任一步失败，状态投影和续租回执一并回滚。

## 5. 幂等与审计

幂等范围固定到 Provider，相同幂等键只能重放同一规范请求。续租回执形成 previous revision/digest 到 target revision/digest 的连续版本链，并保存目标 Lease JSON。读取时重新计算 Lease 摘要，核对投影列、心跳时间、状态、期限和事件摘要。

旧续租回放返回该次续租形成的历史目标状态，不伪装成后来更新过的当前状态；当前状态应通过独立 GET 接口读取。

## 6. 明确无效果

响应固定声明：

- `execution_effect: "external_liveness_assertion_only"`；
- `capacity_effect: "unchanged"`；
- `reservation_effect: "unchanged"`；
- `money_effect: "preauthorization_unchanged"`。

因此本接口不会发送 Start/Renew/Cancel 命令，不会归还或消耗 active Capacity Claim，不会改变 Job/Reservation，不会新增扣款、确认 Provider 收益或完成结算。

## 7. 尚未实现

- Cargo 编译、v186 迁移执行、HTTP 真实调用、并发和时钟边界验证；
- PC 构建、接口联调、视觉验收和发布；
- 外部心跳证明签名校验、可信节点身份、服务器 outbox 和真实 `RenewLease` 送达；
- 自动超时扫描、已出现心跳后的运行中取消确认与部分收费；staging 无心跳安全中止已由 v187 覆盖，v189 只保存 Provider 终态候选但不更新本 Lease，分别见 `docs/distributed-compute/attempt-abort-api.md`、`docs/distributed-compute/attempt-terminal-candidate-api.md`；
- observed、verified 用量及可消耗 meter 结算仍未实现；v188 已写入 running Lease 的累计 `provider_declared` 快照，但不更新本 Lease 状态，见 `docs/distributed-compute/attempt-usage-api.md`；
- 多次 Attempt、分片重试和 fencing generation 单调递增；
- 项目 MCP 写入口、外部矿池、多币种和 Sui 链上资产。
