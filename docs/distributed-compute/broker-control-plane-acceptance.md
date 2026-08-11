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

状态提升为 `implementation_partially_verified`。Store/Service、进程内 HTTP/MCP、两连接竞争与临时磁盘重开已有定向证据；该结论不代表真实 TCP、高并发压力、生产数据库升级、异常断电恢复、真实派发、运行中取消、实际用量或最终结算已经验证。

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

同日执行：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-rust.ps1 -- test --manifest-path server/Cargo.toml --bin elon-server compute_federation_mcp::broker_interface_tests --locked
```

结果：1 项进程内 HTTP/MCP 组合测试通过，验证指纹为 `040e26569635b6b3d58e18bf4102d31891645a121447945a1d57ab37a89b803d`。覆盖：

- 聚合 MCP 可发现 Broker 工具，并保留 read-only、destructive 和显式确认 schema；
- 未确认 Reserve/Release 失败关闭，不改变容量；确认后可完成预留、释放及终态重放；
- MCP 拒绝跨项目和跨消费者读取 Job，未知工具不被误处理；
- HTTP 项目写入口拒绝未登录和非项目成员，个人读取入口拒绝其他消费者读取 Job；
- 测试使用真实用户、会话、项目和项目成员记录，不以伪身份绕过外键或项目门卫。

同日再执行：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-rust.ps1 -- test --manifest-path server/Cargo.toml --bin elon-server compute_federation_broker_service::concurrency_tests --locked
```

结果：2 项独立 SQLite 连接并发测试通过，验证指纹为 `47ed77b76b57343fb1813e75c472cdcf358c3ac2270799df751d846e0a1736fa`。覆盖：

- 相同 Reserve 请求同时越过同步屏障后，只形成一份预算、Claim、Job/Reservation 版本和不可变回执；两个调用分别返回首次提交与重放；
- 两个不同 Reservation 同时竞争同一 quoted Job 时只允许一个成功，另一请求在精确版本门卫失败；
- 两种竞争结束后均只扣 10 分、只持有一次 tokens/concurrency，并且只存在一份 active Reservation。

同日最后执行：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-rust.ps1 -- test --manifest-path server/Cargo.toml --bin elon-server compute_federation_broker_service::restart_tests --locked
```

结果：1 项临时磁盘数据库重开测试通过，验证指纹为 `955152e1016e18b4e72bf22788ab2b3aae6598024a8ef5ab56907b9be99df25c`。覆盖：

- Reserve 后关闭原 Store，再从同一 SQLite 文件重开，预算、容量、reserved Job、active Reservation 及不可变摘要均可继续读取；
- 第一次重开后，相同 Reserve 请求只返回历史重放，不重复冻结余额或持有容量；
- 重开后的 Store 可继续原子 Release，并恢复全部余额和容量；
- 再次关闭并重开后，canceled Job、released Reservation 与不可变终态摘要仍一致，相同 Release 请求只返回历史重放。

## 3. PC 静态证据

同日，包含 `/compute-market` 的 PC 前端已通过严格 TypeScript 与 Vite 生产构建，并产出独立 `ComputeMarketPage` JS/CSS chunk。该证据只说明源码可静态生产构建，不证明真实接口、浏览器交互、权限行为、视觉验收或发布。

## 4. 尚未验证或实现

- HTTP/MCP 真实 TCP、高并发压力、锁超时故障注入和完整路由联调；
- 生产数据库升级、异常断电/进程崩溃恢复、长期磁盘耐久性和浏览器操作；
- 真实价格源、批量报价、自动撮合和到期后台任务；
- sealed Plan、可信 Adapter、节点命令、ACK、Attempt 真实派发与重试；
- 已开始执行任务的安全取消、实际用量、验证、最终结算和 Provider 收益；
- 多币种、外部矿池、Sui 和真实资金清算。

后续必须复用当前 Job/Reservation Registry、Broker 事务和不可变回执，不得新增平行的预留或退款权威。
