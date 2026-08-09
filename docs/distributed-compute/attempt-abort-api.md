---
title: 分布式算力 staging Attempt 无用量安全中止控制面
status: current
reviewed_at: 2026-08-09
owners: backend, node, ai-economy
implementation_status: implementation_uncompiled
---

# 分布式算力 staging Attempt 无用量安全中止控制面

## 1. 当前状态

v187 Store kernel、历史读取、HTTP 路由与 PC `/compute-execution` 中止控制面已经写入代码，但尚未编译、执行迁移或运行接口/页面验证。v213 明确 cancel response 不等于 no-start 后，Provider-owner 人工 Abort POST 已固定失败 `COMPUTE_ATTEMPT_ABORT_GATEWAY_NOT_READY`；GET 和历史回执仍可读取。下文原子中止描述的是保留的 dormant kernel，不是当前可调用写能力。

它用于修复“Broker Finish 已因 Claim 进入 active 而拒绝，但外部执行器实际上尚未开工”这一狭窄状态。它不是运行中取消、超时回收、实际用量结算或节点命令通道。

## 2. HTTP 接口

| 方法 | 路径 | 权限 | 作用 |
|---|---|---|---|
| POST | `/api/me/compute/providers/:provider_id/attempt-leases/:lease_id/abort` | Provider 所有者 | 固定失败，等待 authenticated no-start proof 与 service-actor compensation kernel |
| GET | `/api/me/compute/attempt-leases/:lease_id/abort` | Provider 所有者或 Job 消费者 | 读取并重新审计不可变中止回执 |

旧请求 shape 仍保留兼容解析，但 `confirm_no_execution_started` 和外部中止引用不再获得退款、容量归还或状态推进权。

`executor_abort_ref` 只是外部凭据引用。当前平台不读取证明正文、不验证执行器签名，也不发送 `Cancel` 命令；确认字段表示 Provider 所有者主动声明“执行从未开始”，不是平台验证结果。

PC `/compute-execution` 的旧控件条件不是 no-start 证明；后端固定拒绝该写请求。未来只有绑定 exact command/route/fence 的 final authenticated proof 才能进入独立 service-actor 补偿路径。

## 3. 允许条件

一次中止必须同时满足：

- Lease 仍是 v185 激活产生的 revision 1 `staging` 快照；
- Lease 从未记录 `last_heartbeat_at`，且 revision/digest/fencing 与请求精确一致；
- Job 仍是激活回执绑定的当前 `running` 版本；
- Reservation 仍是激活回执绑定的当前 `active` 版本；
- Capacity Claim 仍是激活回执绑定的当前 `active` 版本；
- 原 v175 平台余额预授权仍与 v185 激活回执金额一致；
- 调用者仍是 Provider 当前所有者；
- 服务端中止时间仍早于 Job deadline。

出现任何心跳、Lease 已进入 `running`、任一精确版本漂移、预算已结算或历史因果链不一致时，整条路径失败关闭。

## 4. 原子效果

一次成功中止在同一个 `BEGIN IMMEDIATE` 事务中完成：

1. 重新审计 v175 Broker 预留、v185 激活回执和 v186 Lease 当前状态；
2. 按原预授权 ID 将 `platform_balance_cny` 预算全额标记为 `released_no_usage`；
3. 追加 `attempt_returned` 容量账本事务，把该 Claim 自有的全部 meter 从 `active` 归还 `available`；
4. 把 Capacity Claim 从 `active` 推进为 `released`；
5. 把 Job 从 `running` 推进为 `canceled`；
6. 把 Reservation 从 `active` 推进为 `released`，绑定新的 Job 与 Claim 精确版本；
7. 把 Lease 从 `staging` 推进为 `terminal`，保存原因码；
8. 保存不可更新、不可删除的 v187 中止回执。

容量归还时间同时作为 Claim、Job 与 Reservation 的终态时间。任一步失败，退款、容量、四类状态和回执全部回滚，避免出现只退款或只归还容量的半完成状态。

## 5. 幂等与审计

幂等范围固定到 Provider；相同幂等键或同一 Lease 只能对应同一规范请求。v187 回执绑定：

- source/terminal Lease revision、摘要和终态 JSON；
- running/canceled Job 历史版本；
- active/released Reservation 历史版本；
- active/released Capacity Claim 历史版本；
- 原 `attempt_activated` 与新 `attempt_returned` 容量事务因果链；
- 原平台余额预授权 ID、全额退款金额和终态；
- 外部中止引用、操作者、服务端时间、请求摘要和事件摘要。

读取或幂等重放会重新计算 Lease 与事件摘要，复核历史版本、预算退款及容量账本字段。重复请求只返回原回执，不会再次退款或归还容量。

## 6. 响应效果边界

响应固定声明：

- `execution_effect: "external_abort_assertion_only"`；
- `capacity_effect: "returned_to_available"`；
- `reservation_effect: "released"`；
- `money_effect: "preauthorization_refunded"`。

这里的资金效果仅指现有平台人民币预授权全额退回消费者余额，不代表真实支付通道退款、Provider 收益结算、多币种清算或 Sui 链上资金移动。

## 7. 尚未实现

- Cargo 编译、v187 迁移执行、HTTP 真实调用、并发和故障注入验证；
- PC 构建、接口联调、视觉验收和发布；
- 外部中止证明签名校验、可信节点身份、服务器 outbox 和真实 `Cancel` 送达；
- 已出现心跳或 `running` Lease 的安全取消、检查点、实际用量和部分结算；
- Lease 超时扫描、自动回收、重试 Attempt 和 fencing generation 单调递增；
- declared、observed、verified 用量分层及 Provider 收益；
- 项目 MCP 写入口、外部矿池、多币种和 Sui 链上资产。

## 8. 实现入口

- `server/src/store/compute_attempt_aborts.rs`
- `server/src/store/compute_attempt_aborts/`
- `server/src/store/compute_capacity_claim_return.rs`
- `server/src/compute_attempt_abort_migration.rs`
- `server/src/compute_federation_attempt_service.rs`
- `server/src/compute_federation_attempt_api.rs`
