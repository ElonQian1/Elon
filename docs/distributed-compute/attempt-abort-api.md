---
title: 分布式算力 staging Attempt 安全中止控制面
status: current
reviewed_at: 2026-08-04
owners: backend, node, ai-economy
implementation_status: implementation_uncompiled
---

# 分布式算力 staging Attempt 安全中止控制面

## 1. 当前状态

v187、Store、Service 与 HTTP 路由已经写入代码，但尚未编译、执行迁移或运行接口验证，状态固定为 `implementation_uncompiled`。它只处理“外部执行器已接受、平台已激活，但尚无任何心跳或用量”的首个 staging Attempt，避免这类失败永久占用消费者预授权与 active Capacity Claim。

本接口不发送真实 Abort/Cancel 命令。`executor_abort_ref` 和 `confirm_no_execution_started=true` 只是 Provider 所有者对外部执行器已中止且从未开始执行的声明；平台当前不读取证明正文，也不验证执行器签名。

## 2. HTTP 接口

| 方法 | 路径 | 权限 | 作用 |
|---|---|---|---|
| POST | `/api/me/compute/providers/:provider_id/attempt-leases/:lease_id/abort` | Provider 所有者 | 基于精确版本栅栏登记 staging 无用量中止 |
| GET | `/api/me/compute/attempt-leases/:lease_id/abort` | Provider 所有者或 Job 消费者 | 读取并重新审计不可变中止回执 |

写请求必须提供 Lease、Job、Reservation、Capacity Claim 的当前 revision/digest、`fencing_generation`、外部中止引用、稳定原因码、幂等键，并显式确认没有开始执行。消费者可以读取与自己 Job 关联的结果，但当前不能通过该接口发起中止。

## 3. 失败关闭边界

一次中止只允许同时满足：

- 调用者仍是当前 Provider 所有者；
- Lease 精确等于 v185 激活回执和当前 revision 1 投影，状态仍为 `staging`，且从未记录 `last_heartbeat_at`；
- Job 仍是激活回执绑定的当前 `running` 精确版本；
- Reservation 仍是当前 `active` 精确版本并绑定同一 Job；
- Capacity Claim 仍是激活回执绑定的当前 `active` 精确版本；
- Broker 预授权、消费者、Offer、Claim、Reservation、Attempt 和 fencing 因果链仍完全一致；
- 服务端中止时间仍早于 Job 不可变截止时间。

任何 revision、摘要、状态、所有权、时间或因果绑定变化都整笔拒绝。已续租为 `running`、出现心跳、超过截止时间或进入其他终态的 Attempt 不能伪装成“无用量中止”。

## 4. 原子效果

一次成功中止在同一个 `BEGIN IMMEDIATE` 事务中完成：

1. 消费者预算预授权以 `released_no_usage` 全额退款；
2. Capacity Claim 从 `active` 转为 `released`，容量账本把对应 meter 从 active 归还 available；
3. Job 追加下一版本并进入 `canceled`；
4. Reservation 追加下一版本并进入 `released`，同时绑定归还后的 Claim 与终态 Job；
5. Lease 追加下一 revision 并进入 `terminal`，保存原因码；
6. 保存不可更新、不可删除的 v187 中止回执，固定全部来源/终态 revision、摘要、账本事务、操作者、请求摘要和事件摘要。

任一步失败，退款、容量、Job、Reservation、Lease 和回执全部回滚。该路径不产生实际用量扣费，也不确认 Provider 收益。

## 5. 幂等与审计

幂等范围固定到 Provider；相同幂等键只能重放同一规范请求。每个 Lease 最多形成一份中止回执。读取时重新核对激活回执、来源与终态版本链、预算退款、容量双分录、Lease 终态和事件摘要，不能用当前投影替换历史回执。

响应效果固定为：

- `execution_effect: "external_abort_assertion_only"`；
- `capacity_effect: "returned_to_available"`；
- `reservation_effect: "released"`；
- `money_effect: "preauthorization_refunded"`。

## 6. 尚未实现

- Cargo 编译、v187 迁移执行、HTTP 真实调用、并发与失败注入验证；
- 外部中止证明签名、可信执行器身份、outbox 和真实 Abort/Cancel 命令送达；
- 已出现心跳或实际用量后的运行中取消、部分收费、赔付、重试和下一 Attempt 创建；
- 消费者中止写入口、项目 MCP 写入口、自动超时扫描和运维恢复；
- declared/observed/verified 用量分层、Provider 收益确认与最终结算。
