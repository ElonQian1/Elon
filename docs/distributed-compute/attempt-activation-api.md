---
title: 分布式算力 Attempt 已接受激活控制面
status: current
reviewed_at: 2026-08-09
owners: backend, node, ai-economy
implementation_status: implementation_uncompiled
---

# 分布式算力 Attempt 已接受激活控制面

## 1. 当前状态

v185 的 Store 与状态推进内核已经写入代码，但尚未编译、执行迁移或运行接口/页面验证。v211 又把该内核收进 Attempt Execution Gateway：新 activation 必须与 exact provisional Adapter ACK 和 application receipt 在同一事务提交；原来依赖调用者声明“外部执行器已经接受任务”的写入口现固定返回 `COMPUTE_ATTEMPT_EXECUTION_GATEWAY_NOT_READY`。当前没有可构造 execution plan、生产 Adapter 或真实节点派发，详见 `attempt-execution-gateway-v1.md`。

## 2. HTTP 接口

| 方法 | 路径 | 权限 | 作用 |
|---|---|---|---|
| GET | `/api/me/compute/providers/:provider_id/attempt-activations?limit=...` | Provider 所有者 | 列出当前仍可登记首次 Attempt 的本人 Reservation 候选 |
| POST | `/api/me/compute/providers/:provider_id/attempt-activations` | Provider 所有者 | 旧写入口已失败关闭，不再接受人工确认绕过 Gateway |
| GET | `/api/me/compute/attempt-leases/:lease_id/activation` | Provider 所有者或 Job 消费者 | 读取并重新审计不可变激活回执 |

旧请求形状仍包含 `lease_id`、`reservation_id`、`executor_id`、执行器接受引用、Lease 凭据引用与精确版本，但 `confirm_executor_accepted=true` 不再形成授权。未来写入只能由 sealed Gateway capability 调用事务内 v185 kernel。

`executor_acceptance_ref` 只是外部证明的引用，不是证明正文或平台验证结果。`lease_credential_ref` 同样只保存凭据引用；接口不接收、返回或持久化 Lease 密钥正文。

候选读取只返回当前 Provider 自有、Reservation 仍为 `active` 且未过期、Job 仍为 `reserved`、并且尚不存在 Attempt 激活回执的记录。每条候选在返回前重新审计当前注册表；读取不冻结资金、不改变容量或状态，也不代表外部执行器已经接受任务。

PC `/compute-execution` 的旧提交源码尚未改成 Gateway workflow；即使页面被构建，提交也只会收到失败关闭错误。页面不得据此显示节点已接受或 Attempt 已启动。

## 3. 原子效果

未来一次 Gateway accepted 成功写入在同一个 `BEGIN IMMEDIATE` 事务中完成：

1. 先精确核对 immutable command、Adapter binding、当前 active Reservation、reserved Job、held Capacity Claim 和 v175 Broker 回执；
2. 确认平台人民币预授权仍为 `reserved`，金额和消费者均未变化；
3. 确认 Provider 属于调用者，Provider 与 Offer 处于 `active` 或 `draining`；
4. 追加 `attempt_activated` 容量账本事务，把该 Claim 自有的全部 meter 从 `held` 转入 `active`；
5. 把 Job 从 `reserved` 推进为 `running`；
6. 保持 Reservation 为 `active`，同时绑定新的 Job 和 Claim 精确版本；
7. 保存 status 为 `staging` 的不可变 Attempt Lease、v185 激活回执和 Gateway application receipt。

v211 会先插入同时指向 v185 与 deterministic application 的 deferred ACK，再调用唯一事务内 kernel；v185 反向 trigger 又要求 exact accepted ACK。任一步失败，ACK、容量、Job、Reservation、Lease 和 application 全部回滚。Offer 进入 `draining` 后仍按 Reservation 保存的历史 Offer/Provider 版本履约；Provider 的当前身份、状态与 route facts 则在创建和 accepted ACK 时重验。终态或其他漂移事实一律 quarantine/失败关闭。

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

因此 v185 回执不是支付成功、节点在线、硬件 verified、远端已收到 commit、任务执行成功或链上结算证明。Gateway accepted 也只是一份 provisional 接受证据。

## 6. 尚未实现

- Cargo 编译、v185 迁移执行、HTTP 真实调用和并发验证；
- PC 构建、接口联调、视觉验收和发布；
- execution plan producer、Adapter 身份/凭据、provisional prepare/commit 和服务器 outbox；
- Start Attempt 真实派发、送达确认、崩溃恢复、心跳与 `running` Attempt 事件；
- 自动超时取消、已出现心跳后的 active 容量处理或实际消耗；v187 只覆盖从未心跳的 staging 无用量中止；
- 多次 Attempt、分片重试和 fencing generation 单调递增；
- verification pending、实际用量、消费者扣款和 Provider 收益结算；
- 项目 MCP 写入口、外部矿池、多币种和 Sui 链上资产。
