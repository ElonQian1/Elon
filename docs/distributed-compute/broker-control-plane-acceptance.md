---
title: Broker Job、预留与未执行终态验收证据
status: current
reviewed_at: 2026-08-11
owners: backend, ai-economy
implementation_status: implementation_partially_verified
---

# Broker Job、预留与未执行终态验收证据

## 1. 验收结论

V5 已有 Job、候选发现、锁价、Reservation、Broker Reserve 与未执行 Release/Expire 的生产实现，本轮没有创建第二套 Broker。临时 SQLite 可执行当前全量迁移，既有 Store/Service 可让真实平台消费者账户创建 Job、发现唯一合格 Offer/Price Snapshot、完成锁价，并在一个事务内预授权人民币余额、持有全部 meter 容量、推进 Job/Reservation 及保存不可变回执。

状态提升为 `implementation_partially_verified`。该结论不代表 HTTP/MCP、并发、真实派发、运行中取消、实际用量或最终结算已经验证。

## 2. 服务端证据

2026-08-11 执行：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-rust.ps1 -- test --manifest-path server/Cargo.toml --bin elon-server compute_federation_broker_service --locked
```

结果：2 项测试通过，验证指纹为 `7278a2c411b50476da00ca005a7c8217ff74fd2d88123ae2ccf2d55f1de8c508`。覆盖：

- submitted Job 只从当前 active Provider、Offer 和未过期 Price Snapshot 中发现合格候选，再以 revision/digest CAS 进入 quoted；
- Reserve 在同一事务内冻结 10 分余额、持有 tokens/concurrency、推进 Job 和 Reservation，并允许相同请求精确重放；
- Release 在没有 Start 的前提下原子退款、归还全部容量、把 Job/Reservation 推进终态，并允许相同客户端请求精确重放；
- 余额不足时整笔事务回滚，余额、容量、quoted Job 和 Reservation 均不留下半成品；
- Provider owner 与消费者平台账户明确分离，计费预授权继续服从 `billing_reservations.user_id -> users.id` 外键。

定向验收发现并修复一项已有实现缺陷：Release/Expire 的 `occurred_at` 由服务端生成，原实现却把每次变化的时间和请求摘要作为重放相等条件，导致网络重试必然被拒绝。修复后仍严格核对 Reservation、消费者、幂等键、动作及源 revision/digest；仅服务端时间不参与客户端请求同一性判断。

## 3. PC 静态证据

同日，包含 `/compute-market` 的 PC 前端已通过严格 TypeScript 与 Vite 生产构建，并产出独立 `ComputeMarketPage` JS/CSS chunk。该证据只说明源码可静态生产构建，不证明真实接口、浏览器交互、权限行为、视觉验收或发布。

## 4. 尚未验证或实现

- HTTP/MCP 真实调用、Bearer/项目成员权限、跨用户隔离和并发幂等；
- 生产磁盘迁移、进程重启、真实 TCP 和浏览器操作；
- 真实价格源、批量报价、自动撮合和到期后台任务；
- sealed Plan、可信 Adapter、节点命令、ACK、Attempt 真实派发与重试；
- 已开始执行任务的安全取消、实际用量、验证、最终结算和 Provider 收益；
- 多币种、外部矿池、Sui 和真实资金清算。

后续必须复用当前 Job/Reservation Registry、Broker 事务和不可变回执，不得新增平行的预留或退款权威。
