---
title: 节点插件 Planning Snapshot 本机投影权威
status: current
reviewed_at: 2026-08-21
owners: node, server
---

# 节点插件 Planning Snapshot 本机投影权威

## 1. 权威范围与当前结论

本文是节点插件 Planning Snapshot 的本机一致性投影、生产启用顺序和协议换代边界的主权威。SQLite 文件与 VFS 生命周期仍由 [`node-plugin-manifest-catalog-authority.md`](node-plugin-manifest-catalog-authority.md) 维护；本机 schema、Store 和恢复语义仍由 [`node-plugin-local-authority.md`](node-plugin-local-authority.md) 维护；endpoint 会话 currentness 仍由 [`node-endpoint-session-authority.md`](node-endpoint-session-authority.md) 维护。

当前 A1 sealed projector 源码已随完整测试目标编译，但没有专项运行，producer 也仍不可达。A2 总合同已经冻结；A2b2/A2c 所在完整测试目标已可编译，相关 targeted fault matrix 已通过 5 项，但 A2b2 的 117 项逐 case `WindowsDynamic` 仍为 0，状态只能是 `implementation_not_dynamically_accepted`；精确边界见 [`A2 authority`](node-plugin-vfs-fault-authority.md) 与 [`acceptance`](node-plugin-vfs-fault-acceptance.md)。生产 `OpenedComputePluginLocalAuthority` 仍不能成功打开，生产 VFS、process owner/fence、root/currentness、trusted-time、rollback 与 node-profile provider 均未接线，因此不存在可生产的 snapshot custody。所有现有节点继续诚实报告 `context_ready=false`、`snapshot_ready=false`，并保持 Runtime stopped。

## 2. A1 的唯一入口

A1 源码把 projector 限定为 `OpenedComputePluginLocalAuthority` 的 crate-private 消费入口。它必须满足：

1. 输入是不可 `Clone`、不可序列化、字段私有的 sealed projection intent；调用方不能传裸路径、`rusqlite::Connection`、revision、digest、时间戳或布尔 ready 标志。
2. projector 只能借用 already-opened、handle-bound 的同一 SQLite connection；不能自行建目录、迁移、切 WAL、按路径重开或调用 legacy `connect/with_deferred/with_immediate` facade。
3. intent 必须不可拆地保活 exact installation、controller、root lock、process fence、Bootstrap generation、policy observation 及未来协议请求绑定。任一 currentness witness 失效都只返回 typed blocked outcome。
4. trusted-time observation、rollback witness、catalog/keyring 与 node-profile witness 必须来自各自的生产 sealed provider。projector 不接受调用方拼出的标量替身，也不在事务中联网取证。
5. A1 输出只是不可复制的本机 coherent projection custody，不是 wire DTO、签名计划、PlanApply capability、work admission 或 Ready capability；本批没有构造入口和 Host 调用点。

## 3. 同一只读事务规则

projector 必须在 already-opened connection 的一个只读 SQLite snapshot 内读取并验证全部数据库事实。禁止用多次连接、事务外缓存或“先读摘要、稍后补正文”拼接结果。

| 事实组 | 同一 snapshot 内的必要闭合 |
|---|---|
| authority head | exact schema/user version、state/inventory/authority revisions、authority epoch、可信时间高水位与 inventory JCS |
| sharing policy | current policy、authorization、revocation/companion receipt、prepared work 终态及 request/receipt digest |
| catalog/keyring | current catalog head、完整 binding receipt、Publisher/Control keyring revision 与指纹 |
| inventory/profile | 全量 inventory 结构、active/candidate 槽、资源与运行时承诺；外部 node-profile witness 必须绑定同一 generation |
| installed/promotion | stable-sorted installed records、active release、install/promotion receipt、generation/fence 及 retained identity commitment |
| work admission | 只有 exact v8 current head 与 active release/authority 全字段匹配时才为 `Some`；新晋升或失配必须诚实为 `None`，不得伪造 generation 1 |
| rollback | 由同一数据库事实重算本机 checkpoint，并与已验外部锚逐字段比较；缺失、落后、分叉或过期全部阻断 |

事务开始前、首个 authority read 前以及最终投影封存前，外层 custody 都必须重验 controller/root/process/Bootstrap 与请求绑定仍 current，并在整个 read view 生命周期内持续持有这些线性租约。SQLite read view 提供数据库内一致性；它不能替代数据库外 currentness、可信时间或 rollback 单调锚。

投影须对 installed records 使用唯一稳定顺序，对所有 JSON 先作结构校验再按既定 JCS 规则重算摘要；缺表、额外或漂移 schema、NULL 形状异常、重复身份、receipt 缺口、摘要不符、代次变化和上限超出都失败关闭。任何失败都销毁未完成 projection，不得返回部分 snapshot 或降级成“未知即空”。

## 4. 输出与失败语义

A1 只允许两类内部结果：

- `Projected`：线性持有完整 validated facts、规范摘要及所有必要 currentness custody；没有公开字段构造器，不能跨进程、落盘或经 serde 传播。
- `Blocked`：保存稳定 blocker code 与最小脱敏诊断；不携带可重用的部分 authority、ready bit 或继续执行能力。

`Projected` 也不等于 `snapshot_ready=true`。只有未来独立协议版本的 producer 在消费 exact request/session witness、再次验证外部 generation 并封存完整 wire hash 后，才有资格生成 ready observation。Signer、Control 私钥、计划发布、PlanApply、下载、Sidecar、Runtime、Ready、route、outbox、Lease 和派发都在本合同之外。

## 5. 严格依赖顺序

后续实现必须依次推进，不得跨级接线：

1. A1：先落 sealed、handle-bound、单只读事务 projector 形状；保持生产 open 和所有协议 producer 不可达。
2. A2 先按 [`authority`](node-plugin-vfs-fault-authority.md) 与 [`acceptance`](node-plugin-vfs-fault-acceptance.md) 完成测试 VFS 的 SHM map/lock/unmap、联合 close 确定性平台故障矩阵和同 namespace 多 Connection custody；当前测试目标虽可编译且有 5 项 targeted 通过，仍须 117/117 逐 case Windows 动态证据和宽范围回归通过才可进入第 3 步，且这些证据也不能自动提升为生产入口。
3. 建立生产 process owner、VFS 注册/注销所有权、live `sqlite3_file`/route、持续 authorizer/PRAGMA 门卫、handle-bound open/close 和 exact root/process fence 生命周期。
4. 接入生产 root/keyring、trusted-time、rollback 与 node-profile provider；用真实 opened authority 证明 A1 全字段同快照投影及全部 typed blocker。
5. 另建协议 v15 producer、会话账本和节点接收链；完成版本隔离、兼容失败关闭及动态验收后，才讨论 `snapshot_ready=true`。
6. 在 v15 ready 已成立后，才依次接 signed reauthorization/work-admission enforcement、Sidecar health、Runtime、Ready V2、route/outbox/Lease 与真实派发。

任何阶段缺少前一阶段的动态证据或生产 custody，都必须保留后续入口不可构造。

## 6. v14 永久阻断，v15 独立换代

v14 `planning_snapshot_bootstrap_only` 是已冻结的 blocked-only compatibility profile。它的 capability、mode、六消息 sequence、前序摘要、校验器、Node/Store 语义和 v219 provenance 都不得扩展成 ready 分支；即使未来本机 projector 可达，v14 仍只能返回 `snapshot_ready=false`。

未来 ready 路线必须使用独立 v15 profile 与 capability，并拥有独立 protocol threshold、消息/摘要域、sequence 终态、验证器、节点状态和追加式 ledger 版本。禁止把 v14 session、ACK、endpoint witness 或 ledger row 原地升级为 v15 权威；凭据 epoch、session、Bootstrap generation 或请求绑定变化时，旧链必须终态撤销并重新开始。

## 7. 明确禁线

- 禁止让 legacy path facade、默认 SQLite VFS、`cfg(test)` VFS 或测试 constructor 参与 planning。
- 禁止以 canonical path、open 后 FileId 复核或裸 namespace 替代 handle-bound main/journal/WAL/SHM custody。
- 禁止跨事务拼接 policy、catalog、inventory、promotion、work-admission 或 rollback 事实。
- 禁止把 endpoint session、内存布尔值、缓存摘要或远端 checkpoint 当作 root/process/本机 rollback 权威。
- 禁止为缺失 work-admission、receipt、keyring、可信时间或 rollback 填默认值；`None` 只能表达经过验证的确实不存在或不匹配。
- 禁止让 A1 projector 签名、联网、迁移 schema、修复数据、写 Store、下载或启动任何进程。
- 禁止从 v14 输出、旧 ACK 或测试 snapshot 构造 v15 ready custody。
- 禁止以文档冻结、静态审阅或单元夹具宣称生产 producer、Runtime、Ready 或派发已可达。

## 8. A1 验收边界

A1 完成只意味着源码具备不可旁路的 sealed API、同一只读事务枚举/重验、稳定投影和 typed blocked 分类，并由静态审阅确认没有 legacy/path/test 入口。只要生产 `open()` 仍固定 unavailable，A1 就必须保持无生产调用方；其验收记录必须逐项列出“随完整目标编译、未专项运行、无 producer”等实际证据，不能预支 A2-A6 的完成状态。
