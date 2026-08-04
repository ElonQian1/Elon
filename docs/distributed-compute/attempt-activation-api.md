---
title: 分布式算力 Attempt 已接受激活控制面
status: current
reviewed_at: 2026-08-05
owners: backend, node, ai-economy
implementation_status: implementation_uncompiled
---

# 分布式算力 Attempt 已接受激活控制面

## 1. 当前状态

v185、Store、Service、HTTP 路由与 PC `/compute-execution` 工作区已经写入代码，但尚未编译、执行迁移或运行接口/页面验证，状态固定为 `implementation_uncompiled`。本控制面登记“外部执行器已经接受任务”这一调用方声明，并把既有 Broker Reservation 推进到首个 Attempt；它不发送节点命令，也不验证外部执行器签名，不能描述为通用节点派发已经完成。v186 会在同一激活事务中初始化 Lease 当前状态；从未心跳的 staging Lease 无用量中止边界见 `attempt-abort-api.md`。

## 2. HTTP 接口

| 方法 | 路径 | 权限 | 作用 |
|---|---|---|---|
| GET | `/api/me/compute/providers/:provider_id/attempt-activations?limit=...` | Provider 所有者 | 列出当前仍可登记首次 Attempt 的本人 Reservation 候选 |
| POST | `/api/me/compute/providers/:provider_id/attempt-activations` | Provider 所有者 | 登记首个已被执行器接受的 Attempt |
| GET | `/api/me/compute/attempt-leases/:lease_id/activation` | Provider 所有者或 Job 消费者 | 读取并重新审计不可变激活回执 |

写请求提供稳定 `lease_id`、`reservation_id`、`executor_id`、可选 `shard_id`、执行器接受证明引用、Lease 凭据引用与脱敏提示、Job/Reservation/Claim 的精确 revision/digest、Lease 到期时间、hard deadline 和幂等键。当前首次入口要求 `attempt_no=1`、`fencing_generation=1`，并要求 `confirm_executor_accepted=true`。

`executor_acceptance_ref` 只是外部证明的引用，不是证明正文或平台验证结果。`lease_credential_ref` 同样只保存凭据引用；接口不接收、返回或持久化 Lease 密钥正文。

候选读取只返回当前 Provider 自有、Reservation 仍为 `active` 且未过期、Job 仍为 `reserved`、并且尚不存在 Attempt 激活回执的记录。每条候选在返回前重新审计当前注册表；读取不冻结资金、不改变容量或状态，也不代表外部执行器已经接受任务。

PC `/compute-execution` 允许 Provider 所有者选择本人 Provider、读取候选并显式填写外部执行器接受引用、Lease 凭据引用、期限和精确版本后提交激活。页面不采集凭据正文，不发送 Start 命令，也不验证执行器、节点在线状态或证明真实性。

## 3. 原子效果

一次成功写入在同一个 `BEGIN IMMEDIATE` 事务中完成：

1. 精确核对当前 active Reservation、reserved Job、held Capacity Claim 和 v175 Broker 回执；
2. 确认平台人民币预授权仍为 `reserved`，金额和消费者均未变化；
3. 确认 Provider 属于调用者，Provider 与 Offer 处于 `active` 或 `draining`；
4. 追加 `attempt_activated` 容量账本事务，把该 Claim 自有的全部 meter 从 `held` 转入 `active`；
5. 把 Job 从 `reserved` 推进为 `running`；
6. 保持 Reservation 为 `active`，同时绑定新的 Job 和 Claim 精确版本；
7. 保存 status 为 `staging` 的不可变 Attempt Lease 及 v185 激活回执。

任一步失败，容量、Job、Reservation 和回执全部回滚。Offer 或 Provider 进入 `draining` 后仍可履行已经存在的 Reservation；终态 Offer、disabled/quarantined Provider、过期 Reservation、失效预算或旧 revision/digest 一律失败关闭。

## 4. 幂等与审计

幂等范围固定到 Provider，相同幂等键只能重放同一规范请求。v185 回执不可更新或删除，并绑定：

- source/running Job 历史版本；
- source/active Reservation 历史版本；
- source/active Capacity Claim 历史版本；
- `attempt_activated` 账本事务、Lease ID 和 fencing generation；
- 原平台余额预授权 ID 与冻结金额；
- 执行器接受引用、操作者和服务端激活时间。

读取时重新计算 Lease 摘要，复核历史版本、容量账本因果字段和交易摘要。旧 fencing generation 不能通过当前首次入口伪装成新 Attempt。

## 5. 明确无效果

响应固定返回：

- `execution_effect: "none"`：事务不发送 WebSocket、Sidecar、插件或外部矿池命令；
- `money_effect: "preauthorization_unchanged"`：不新增扣款、不确认 Provider 收益、不结算实际用量；
- Lease `status: "staging"`：只表示已登记外部接受声明，不证明任务已经产生结果。

因此本接口不是支付成功、节点在线、硬件 verified、任务执行成功或链上结算证明。

## 6. 尚未实现

- Cargo 编译、v185 迁移执行、HTTP 真实调用和并发验证；
- PC 构建、接口联调、视觉验收和发布；
- 执行器接受证明的签名校验、可信节点身份绑定和服务器 outbox；
- Start Attempt 真实派发、送达确认、心跳与 `running` Attempt 事件；
- 自动超时取消、已出现心跳后的 active 容量处理或实际消耗；v187 只覆盖从未心跳的 staging 无用量中止；
- 多次 Attempt、分片重试和 fencing generation 单调递增；
- verification pending、实际用量、消费者扣款和 Provider 收益结算；
- 项目 MCP 写入口、外部矿池、多币种和 Sui 链上资产。
