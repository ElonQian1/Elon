---
title: Provider Capacity Commitment v225 验收证据
status: current
reviewed_at: 2026-08-12
owners: backend, ai-economy
implementation_status: implementation_partially_verified
---

# Provider Capacity Commitment v225 验收证据

## 1. 验收结论

v225 复用已有 Provider、Offer、Pool、Price Snapshot、平台参考价格绑定、Capacity Claim 和追加式容量账本，没有创建第二套 Broker、容量、余额或价格权威。本轮先补齐生产目标编译以及 Store/Service/进程内 HTTP 定向测试，随后在现有 `/compute-supply` Offer 工作区接入 owner 查询、创建与取消控制面，状态保持 `implementation_partially_verified`。

临时全新 SQLite 数据库可以执行当前全量迁移。Provider owner 可在 exact `capacity_future` Offer、未过期 Snapshot 和已批准应用的 reference binding 下原子创建容量承诺；同一事务写入 immutable commitment、Claim 与 ledger hold。取消或到期会通过唯一 terminal receipt 原子归还容量，磁盘数据库重开后仍可按 immutable revision/digest 读取终态。

该结论不代表真实 TCP、生产数据库升级、跨连接并发压力、后台到期任务、PC 浏览器操作、DeliveryAllocation、实际交付、资金预授权或结算已经完成。

## 2. 定向测试证据

2026-08-12 执行：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-rust.ps1 -Domain capacity-commitment -- test --manifest-path server\Cargo.toml --bin elon-server capacity_commitment -- --nocapture --test-threads=1
```

结果：3 项测试通过，rebase 到最新 `origin/main` 后新增 owner source 查询的最终验证指纹为 `8f9ceaeab1d32c26386a23d11e545afcf489e16a2be276645e21cb52b5d7d630`。覆盖：

- 未显式确认时创建失败，Commitment 表和容量余额均无副作用；
- 创建在同一事务内把 tokens/concurrency 从 available 移至 held，相同请求只返回历史重放；
- 非 owner 无法读取承诺，owner 取消后只形成一份 revision 2 `canceled` 回执并恢复全部容量；
- 关闭并重开同一 SQLite 文件后，Commitment、终态回执和 current status 仍可重建；
- 到期恢复只选择一次 due Commitment，形成 `expired` 回执并恢复全部容量，第二次扫描不重复处理；
- HTTP 入口拒绝未登录访问和非 owner 创建，个人列表按 owner 隔离，管理员到期入口拒绝普通用户；
- owner source 查询先复用 Price Snapshot 所有权门卫，再返回 exact v223 reference binding；未登录和其他 owner 均不能取得绑定；
- HTTP 测试使用真实用户、会话和 in-process Router，不以伪身份绕过认证与角色门卫。

测试夹具从空临时目录调用 `Store::open`，因此同时覆盖 v225 migration 进入当前全量迁移链；重开用例覆盖已落盘数据库的再次打开。它们不是已有生产数据库的升级演练。

## 3. 生产编译与全目标边界

同日使用项目 Rust 验证入口对 `elon-server` 生产二进制执行：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-rust.ps1 -Domain capacity-commitment -- check --manifest-path server\Cargo.toml --bin elon-server
```

`elon-server` 生产目标通过；rebase 后接入 owner source 查询的最终验证指纹为 `cede2dcc1bd3e1422c62d5f5c49b85416c50b7c71e5c6557ea45933bc5e741e3`。曾尝试扩大到 `--all-targets`，但被与 v225 无关的 `elon-pc-node` 测试模块 VFS 可见性错误阻断；该失败不改变容量承诺生产目标和定向测试结果，也不能被隐瞒为全工作区已通过。

## 4. 本轮修复

- 修复 due Commitment ID 查询中 `MappedRows` 临时值生命周期导致的生产编译错误；
- 测试余额断言复用 `Store::compute_capacity_bucket_balance`，不读取臆造的第二套余额表；
- 仅把 v223 Snapshot binding receipt 提升为 crate 内可见，继续复用同一价格绑定类型和审计路径。
- owner source 查询复用 v171 Snapshot owner 门卫和 v223 binding 读取，不复制 reference curve 审计逻辑；
- PC 控制面嵌入既有 Offer 详情，按 exact Provider/Offer/Pool/Snapshot/binding 派生完整 meter 数量，并对锁定和取消分别要求显式确认。

PC 侧 `test:compute-capacity-commitment`、严格 TypeScript、lint、Vite 生产构建和 bundle budget 已通过。该证据不包含真实 API、浏览器交互、视觉或窄屏验收。

## 5. 尚未验证或实现

- 真实 TCP 路由联调、跨连接取消/到期竞争、高并发压力和锁超时故障注入；
- 已有生产数据库的升级、异常断电恢复、长期磁盘耐久性和后台到期调度；
- Android/MCP 控制面，以及 PC 真实 API、浏览器、视觉和窄屏验收；
- DeliveryAllocation、执行派发、实际用量、违约替代容量和交付差额；
- 资金预授权、Provider 收益、保证资源、处罚、清算和 Sui 结算；
- `external_pool`、真实平台参考价格源和真实生产 Offer。

后续必须复用 v225 immutable Commitment、terminal receipt、既有 Claim 与 ledger，不得为 UI、派发或结算新增平行的容量承诺权威。

## 6. v238 CapacityInstrument 接入不继承旧证据

v238 已在 CapacityCommitment Create 前增加 exact active Instrument、immutable Offer publication adoption 与共同合约单位门，并在 SQLite direct-insert trigger 复核相同边界。所有 meter 的 claim quantity 必须分别是 Instrument `quantity_units` 的正整数倍，且 multiplier 完全相同；仅同 meter、window 或 `instrument_id` 相等不足以创建 Commitment。

该接入当前为 `source_written/implementation_uncompiled/implementation_unrun`、`passed=0`。本页第 2–4 节的 v225 编译、临时 SQLite、HTTP、重开和 PC 证据发生在 v238 之前，没有执行 v238 migration 或新门卫，不得据此声称 v238 后的 Commitment 已验证。后续至少须重新覆盖正常共同倍数、缺/多 meter、不同 multiplier、registered/retired Instrument、stale adoption/publication、raw SQL 旁路，以及退休后既有 Cancel/Expire 仍能归还容量。v238 不生成订单、真实价格、可信计量、Provider 收益或结算，权威见 [`capacity-instrument-authority.md`](capacity-instrument-authority.md)。
