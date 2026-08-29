---
title: 节点插件 handle-bound authority open attempt V1 权威草案
status: draft
reviewed_at: 2026-08-30
owners: node, platform
proposed_feature_id: compute-plugin-handle-bound-open-attempt-v1
registration_status: unregistered_feature_workflow_unavailable
design_status: draft_frozen
implementation_status: source_compiled_production_unwired
verification_status: targeted_local_source_and_registry_verified
---

# 节点插件 handle-bound authority open attempt V1 权威草案

## 1. 唯一结论

本未登记草案只冻结未来生产 handle-bound SQLite open 的 ownership/error topology。源码允许一个
当前没有安全 constructor 或生产 producer 的 process-registration owner seal 消费现有
`ComputePluginHandleBoundAuthorityOpenIntent`，先形成 `RegisteredPending`，再单向进入
`OpeningPreConnection`。它不注册 VFS、不调用 SQLite open、不产生 `Connection`，也不构造
`OpenedComputePluginLocalAuthority`。

当前生产 `ComputePluginHandleBoundAuthorityOpenIntent::open()` 必须继续固定返回
`COMPUTE_PLUGIN_HANDLE_BOUND_SQLITE_VFS_UNAVAILABLE`。本草案不关闭 handle-bound open producer
缺口，也不把 A2 的 test-only VFS 或历史测试证据提升为生产能力。

## 2. 必须复用的唯一 owner

| 事实或 custody | 唯一 owner | 本草案行为 |
|---|---|---|
| pinned root、controller、instance lock、sealed SQLite namespace | `ComputePluginHandleBoundAuthorityOpenIntent` | 只能被 generic registry 原子消费；失败前不得拆成标量。 |
| one-shot token、session ID、route epoch、policy 与 callback/file lifecycle | `ManagedSqliteRegistryProcessOwner` | 复用既有 register/state/retirement；不复制第二套 route 状态机。 |
| opaque main logical name | registry-owned `SealedHandleBoundSqlitePolicy` | 只在 exact route 上读取；不接受 caller 字符串、路径或 URI。 |
| installation、root 与 authority instance | 原 open intent | 只形成 private descriptor 绑定；复制出的 digest 不是 authority。 |
| future SQLite connection/backend | `OpenedComputePluginLocalAuthority` 私有模块 | 本批不存在 handoff 或 constructor。 |

`ComputePluginHandleBoundOpenAttemptProcess` 只借用唯一、进程寿命的 specialized process
owner，作为注册的 sealed gate；attempt 注册后由底层 process owner 而非该 gate 保活 custody。它没有生产 constructor，也不得在每次 attempt 时调用 `leak_with_system_nonce_source()`
制造新的 owner。没有该 seal，注册入口不可调用。

## 3. 两态线性拓扑

### 3.1 `RegisteredPending`

`register(owner_seal, intent)` 必须先让既有 registry 重验 intent currentness，再原子生成 opaque
route。注册失败必须返还完整原 intent；调用方不能只得到错误码而遗失 root/controller/namespace
custody。

注册成功值必须同时保存 exact process owner、route 与从原 intent 派生的 private identity descriptor。
它不可 Clone、Copy、Serde、Send 或 Sync。隐式放弃时只允许消费 exact route 调用
`retire_pending`；成功后 registry 才能自然释放 intent。owner poison、身份错配或终态错误沿既有
process-lifetime retain/quarantine 语义失败关闭，不能从标量重建 route。

### 3.2 `OpeningPreConnection`

`begin_open` 固定先从 exact route 取得 opaque main logical name，再执行既有
`begin_open_attempt`。任一步失败都消费 pending capability，并由其 Drop 尝试 pending retirement；
不得返还可重试 intent 或 route。

只有两步都成功才进入 `OpeningPreConnection`。本批故意不给它成功消费者：没有 VFS registration
proof、live `sqlite3_file` graph、SQLite return/extended-code proof、authorizer/PRAGMA install proof、
`Connection` ownership acceptance 或 sealed backend。因而任何该值的 Drop 都必须以
`FailureCustodyRetained` 隔离 exact route，并让原 intent 留在 process-lifetime registry custody；
不得误用 `retire_pending`。

## 4. 失败与恢复边界

| 边界 | 允许结果 | 禁止结果 |
|---|---|---|
| registration 前/中失败 | typed reason + 完整原 intent | 丢失 intent、只返 digest、自动重建 owner |
| 已注册但 logical-name/begin 失败 | pending capability 被消费并尝试 exact pending retirement | 返还可重试 intent、换 route 或换 nonce |
| 已进入 Opening 后放弃 | exact route `FailureCustodyRetained` quarantine | pending retirement、普通 Drop 释放 intent |
| owner mutex poison/route identity 不确定 | process-lifetime retain；等待未来显式 recovery 合同 | 假定 route 已移除或 Connection 已关闭 |

本批没有 connection-close observation、close proof、`retire_closed` handoff 或 crash recovery。文档中的
状态名只约束未来 owner 顺序，不是 durable receipt、Store row 或 wire ABI。

## 5. A2 与生产启用门

[`node-plugin-planning-snapshot-authority.md`](node-plugin-planning-snapshot-authority.md) 与
[`node-plugin-vfs-fault-authority.md`](node-plugin-vfs-fault-authority.md) 的顺序保持不变。A2 当前为
Barrier 与 Registration 各 `WindowsDynamic=8/8`、RegistryLifecycle `WindowsDynamic=16/16`、Unmap
`WindowsDynamic=49/49`、A2b2 `WindowsDynamic=81/117`；剩余 36 项全部是 JointClose，clean wide
regression `205/205` 已通过，但 Map/Lock pending/open frontiers 与 JointClose 仍未闭合。A2 仍未完成，
测试 VFS 不得作为本草案 owner seal 的 producer。

未来只有在 A2 完整动态验收后，才能同批补齐：唯一 production process owner、VFS 注册/注销所有权、
ABI→registry live route、main/journal/WAL/SHM handle graph、connection success/failure custody、显式 close
observation 与 route retirement。单独提供 owner seal constructor 或打开现有 `open()` 都属于越级。

## 6. 零效果

```text
migration/table/view/trigger/writer = none/none/none/none/none
service/api/http/mcp/pc/wire = none/none/none/none/none/none
vfs_registration/sqlite_open/connection/opened_authority = none/none/none/none
plan_apply/runtime/ready/provider/route/offer/capacity = none
job/attempt/lease/receipt/usage/settlement/money = none
```

本批不修改 schema、Store、Bootstrap caller、Host、协议或市场状态；`route=none` 指联邦任务路由，
不否认本草案内部持有的不可序列化 SQLite registry route identity。

## 7. 当前证据与禁线

状态严格为 `unregistered/draft_frozen/source_written/source_compiled/production_unwired`。实际
`elon-pc-node` 测试目标已编译，新 source-contract `4/4`、复用的 registry lifecycle 回归 `45/45`
通过，均为 `failed=0`。这些证据只证明源码边界和既有 registry 行为；由于 process seal 仍没有安全
producer，open-attempt 两态没有行为运行证据，生产 VFS/SQLite/Connection 也仍未运行。A2 Barrier、
RegistrationShutdown 与 RegistryLifecycle 已在各自 exact clean evidence commit 上正式验证；Unmap 又在
exact clean commit `da62f95b09287b79bc1f4c23780b95993cdd85a0`、Windows 10.0.26200 x86_64、fixed NTFS、
SQLite 3.45.0 上正式 `49/49`。四个 family 分别 `8/8`、`8/8`、`16/16`、`49/49`，正式 records 均为
`child_exit=0`、`parent_cleanup=deleted`。A2b2 为 `81/117`；剩余 36 项全部是 JointClose，
clean wide regression `205/205` 已通过，但 Map/Lock 与 JointClose 未闭合，不能据此宣称 A2 完成。
缺编译期 Git SHA、旧 cache reuse 与 partial failure 均保留为不计数历史。当前工具目录没有
`project_feature_workflow`，所以
proposed feature 未登记、未 claim；禁止手改 `.elon/project-features.json`。

禁止：

- 从路径、裸 `Connection`、caller nonce 或 test VFS 构造 attempt；
- 每次 attempt 泄漏新的 process owner；
- 在本批调用 `sqlite3_vfs_register`、`sqlite3_open_v2` 或 `from_verified_backend`；
- 把 copied descriptor、opaque logical name 或 route handle 序列化成 authority/receipt；
- 把 Opening Drop 写成正常成功、Connection close 或 opened authority；
- 用 source-contract/registry 回归或 Registration 的 8 个 record 宣称 open-attempt runtime、A2 全量
  WindowsDynamic 或生产验收通过。

## 8. 后续顺序

1. 先完成 A2 StaticContract、WindowsDynamic 与宽回归；
2. 建立唯一 production registration/VFS owner，并动态验证注册、open、ABI file graph 与失败回收；
3. 让 verified connection/backend 在 exact route custody 下进入 opened authority，并完成显式 close/retire；
4. 才让 Planning A1 与 Ready local-currentness 获得真实 producer；
5. 随后独立推进 runtime transition、Host runtime 与 v15 authenticated session。
