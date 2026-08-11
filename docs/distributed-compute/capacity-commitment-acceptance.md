---
title: Provider Capacity Commitment v225 验收证据
status: current
reviewed_at: 2026-08-12
owners: backend, ai-economy
implementation_status: implementation_partially_verified
---

# Provider Capacity Commitment v225 验收证据

## 1. 验收结论

v225 复用已有 Provider、Offer、Pool、Price Snapshot、平台参考价格绑定、Capacity Claim 和追加式容量账本，没有创建第二套容量、余额或价格权威。本轮补齐生产目标编译以及 Store/Service/进程内 HTTP 定向测试，状态提升为 `implementation_partially_verified`。

临时全新 SQLite 数据库可以执行当前全量迁移。Provider owner 可在 exact `capacity_future` Offer、未过期 Snapshot 和已批准应用的 reference binding 下原子创建容量承诺；同一事务写入 immutable commitment、Claim 与 ledger hold。取消或到期会通过唯一 terminal receipt 原子归还容量，磁盘数据库重开后仍可按 immutable revision/digest 读取终态。

该结论不代表真实 TCP、生产数据库升级、跨连接并发压力、后台到期任务、PC 页面、DeliveryAllocation、实际交付、资金预授权或结算已经完成。

## 2. 定向测试证据

2026-08-12 执行：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-rust.ps1 -Domain capacity-commitment -- test --manifest-path server\Cargo.toml --bin elon-server capacity_commitment -- --nocapture --test-threads=1
```

结果：3 项测试通过，rebase 到当时最新 `origin/main` 后的最终验证指纹为 `aaf0046ebf132ce6da3a963d5c612219b2d7d1659be0f82d97421aaf1dfe1b17`。覆盖：

- 未显式确认时创建失败，Commitment 表和容量余额均无副作用；
- 创建在同一事务内把 tokens/concurrency 从 available 移至 held，相同请求只返回历史重放；
- 非 owner 无法读取承诺，owner 取消后只形成一份 revision 2 `canceled` 回执并恢复全部容量；
- 关闭并重开同一 SQLite 文件后，Commitment、终态回执和 current status 仍可重建；
- 到期恢复只选择一次 due Commitment，形成 `expired` 回执并恢复全部容量，第二次扫描不重复处理；
- HTTP 入口拒绝未登录访问和非 owner 创建，个人列表按 owner 隔离，管理员到期入口拒绝普通用户；
- HTTP 测试使用真实用户、会话和 in-process Router，不以伪身份绕过认证与角色门卫。

测试夹具从空临时目录调用 `Store::open`，因此同时覆盖 v225 migration 进入当前全量迁移链；重开用例覆盖已落盘数据库的再次打开。它们不是已有生产数据库的升级演练。

## 3. 生产编译与全目标边界

同日使用项目 Rust 验证入口对 `elon-server` 生产二进制执行：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-rust.ps1 -Domain capacity-commitment -- check --manifest-path server\Cargo.toml --bin elon-server
```

`elon-server` 生产目标通过，rebase 后最终验证指纹为 `73b9bbb36958f4286624560a7e7af8d169f3358de3832903d1b0222a6be412da`。曾尝试扩大到 `--all-targets`，但被与 v225 无关的 `elon-pc-node` 测试模块 VFS 可见性错误阻断；该失败不改变容量承诺生产目标和定向测试结果，也不能被隐瞒为全工作区已通过。

## 4. 本轮修复

- 修复 due Commitment ID 查询中 `MappedRows` 临时值生命周期导致的生产编译错误；
- 测试余额断言复用 `Store::compute_capacity_bucket_balance`，不读取臆造的第二套余额表；
- 仅把 v223 Snapshot binding receipt 提升为 crate 内可见，继续复用同一价格绑定类型和审计路径。

## 5. 尚未验证或实现

- 真实 TCP 路由联调、跨连接取消/到期竞争、高并发压力和锁超时故障注入；
- 已有生产数据库的升级、异常断电恢复、长期磁盘耐久性和后台到期调度；
- PC/Android/MCP 的 Capacity Commitment 控制面和真实浏览器验收；
- DeliveryAllocation、执行派发、实际用量、违约替代容量和交付差额；
- 资金预授权、Provider 收益、保证资源、处罚、清算和 Sui 结算；
- `external_pool`、真实平台参考价格源和真实生产 Offer。

后续必须复用 v225 immutable Commitment、terminal receipt、既有 Claim 与 ledger，不得为 UI、派发或结算新增平行的容量承诺权威。
