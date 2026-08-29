---
title: 节点插件 handle-bound authority open attempt V1 验收草案
status: draft
reviewed_at: 2026-08-29
owners: node, platform
proposed_feature_id: compute-plugin-handle-bound-open-attempt-v1
registration_status: unregistered_feature_workflow_unavailable
design_status: draft_frozen
implementation_status: source_compiled_production_unwired
verification_status: targeted_local_source_and_registry_verified
---

# 节点插件 handle-bound authority open attempt V1 验收草案

## 1. 当前证据强度

唯一规范是
[`node-plugin-handle-bound-open-attempt-authority.md`](node-plugin-handle-bound-open-attempt-authority.md)。
本批只允许记载：

```text
proposed_feature=compute-plugin-handle-bound-open-attempt-v1
registry=unregistered
claim=none
design=draft_frozen
ownership_source=source_compiled_no_safe_producer
source_contract_guard=4/4
registry_lifecycle_regression=42/42
verification=targeted_local_source_and_registry_verified
compile/runtime=implementation_compiled/open_attempt_runtime_unrun
compiled_targets=1 test_cases_run=46 passed=46 failed=0
a2_barrier_attempt=formal_windows_dynamic_verified
a2_barrier_windows_dynamic=8/8
a2_registration_attempt=formal_windows_dynamic_reverified
a2_registration_windows_dynamic=8/8
a2b2_windows_dynamic=16/117
a2b2_remaining_without_dynamic_record=101
a2_evidence_commit=1d57b8d98a1fed70fe40ad6f1575f4b856226857
a2_barrier_validation_fingerprint=c300720aa17bae7850f0d0088827b918c5b5d66c2842ff6d5b548e80c7d185f4
a2_registration_validation_fingerprint=b970858c74a2226c69182b64afd4c86ffe66d2a3c6b5d1cd3cf26aa57f5698bb
a2_wide_validation_fingerprint=e6903ba19d0ff512bba4d1489857b179874a93666bd97f9fae815ef045347e99
a2_environment=Windows_10.0.26200_x86_64/fixed_NTFS/SQLite_3.45.0
a2_test_result=barrier_8_passed/registration_8_passed/0_failed/1695_filtered_each
a2_receipts=16_unique_exact_commit/child_exit_0/parent_cleanup_deleted
a2_wide_regression=121_passed/0_failed
migration/table/writer=none/none/none
vfs_registration/sqlite_open/connection/opened_authority=none/none/none/none
production_acceptance=deferred
```

`46/46` 由新 guard `4/4` 与本次 registry lifecycle `42/42` 构成；后者证明复用 owner 的既有行为，
不是 open-attempt typestate 的 production runtime。正式 A2 Barrier 与 RegistrationShutdown 证据绑定 exact
clean commit `1d57b8d98a1fed70fe40ad6f1575f4b856226857`：Windows 10.0.26200 x86_64、fixed NTFS、
SQLite 3.45.0，两个 family 各 `8 passed/0 failed/1695 filtered`；16 个 unique receipts 全部精确绑定该
commit，且均为 `child_exit=0`、`parent_cleanup=deleted`。缺编译期 Git SHA、旧 cache reuse 与 partial
failure 均是不计数历史。当前正式记录计为 Barrier `8/8`、Registration `8/8`、A2b2 `16/117`；其余
101 无动态 record，clean wide regression `121/121` 已通过，但 Map/Lock 与其余 family 未闭合，A2 仍
未完成。既有 managed-fs、A1 或 test-only VFS 的其他历史证据不能记为本草案通过数；功能工作流
不可用，注册表保持未修改。

## 2. Source review 清单

已运行的静态 guard 与配套只读全源审阅共同钉住：

1. 新逻辑只能位于 `sqlite_vfs_policy::registry` child，不能广泛 re-export generic owner/route internals；
2. owner seal 借用 exact specialized process owner，无生产 constructor，源码不调用 per-attempt leak；
3. registration 只消费现有 OpenIntent，失败对象保留并可返还 typed reason 与完整 intent；
4. `RegisteredPending` 不可 Clone/Serde/Send/Sync，Drop 只调用 exact `retire_pending`；
5. `begin_open` 顺序固定为 `main_logical_name_owned` 后 `begin_open_attempt`；
6. begin 失败不返 intent 或 retry capability；
7. `OpeningPreConnection` 不可 Clone/Serde/Send/Sync，Drop 使用
   `retain_terminal_custody(...FailureCustodyRetained...)`，不调用 pending retirement；
8. source 不含 VFS registration、SQLite open、`Connection`、sealed backend 或 Opened constructor；
9. 现有 `open()` 仍固定返回 `COMPUTE_PLUGIN_HANDLE_BOUND_SQLITE_VFS_UNAVAILABLE`；
10. connection handoff、activate、close observation 与 `retire_closed` 保持 future blocker；
11. authority/acceptance 同时声明 A2 动态门、零效果与未运行状态；
12. 私有模块与无安全 producer 共同阻断 Bootstrap/Host/Store/Ready/市场调用点，全源审阅未发现绕行 caller。

以上 12 项已由 4 个 source-contract 测试和独立源码审阅覆盖；它们仍不是生产运行结果。

## 3. 实证与未运行矩阵

| 轴 | passed | failed | unrun | 当前结论 |
|---|---:|---:|---:|---|
| Rust `elon-pc-node` test target 编译 | 1 | 0 | 0 | 实际通过 |
| source-contract guard | 4 | 0 | 0 | `4/4` 实际通过 |
| 既有 registry lifecycle 回归 | 42 | 0 | 0 | `42/42` 实际通过 |
| open-attempt typestate 行为 | 0 | 0 | 1 | 无安全 producer，未运行 |
| production VFS/SQLite/Connection | 0 | 0 | 1 | 明确未接线 |
| A2 Barrier WindowsDynamic | 8 | 0 | 0 | 正式 `8/8`，8 个 exact-commit records |
| A2 Registration WindowsDynamic | 8 | 0 | 0 | 正式 `8/8`，8 个 exact-commit records |
| A2b2 WindowsDynamic | 16 | 0 | 101 | 当前 `16/117`，剩余 101 无动态 record |
| migration/Store/runtime/network/device | 0 | 0 | 1 | 未运行 |
| Ready/Provider/market/economy | 0 | 0 | 1 | effects=none |

## 4. 负向验收

以下任一情况都拒绝本草案：

- 生产 `open()` 不再固定 unavailable；
- owner seal 有普通 constructor、从 caller nonce 构造，或 attempt 自行泄漏 owner；
- 注册失败不返完整 intent，或 begin 失败返可重试 route/intent；
- copied digest、logical name 或 route identity 被称为 durable authority；
- Opening Drop 释放 intent、调用 `retire_pending`，或被称为 Connection close 成功；
- 引入 `sqlite3_vfs_register`、`sqlite3_open_v2`、`rusqlite::Connection`、live `sqlite3_file` 或
  `OpenedComputePluginLocalAuthority::from_verified_backend`；
- 提升 test-only VFS，修改 migration/table/writer/API/Host/Ready/市场，或产生任何经济效果；
- 把已编译/guard/registry 回归或 Barrier/Registration `8/8` 外推为 open-attempt runtime、A2 `117/117` 或生产
  producer 已存在。

## 5. 晋级门

解除架构阶段禁令后，必须先完成 A2 全部 source inventory/terminal closure、Barrier/Registration 各 `8/8`、A2b2
`117/117` WindowsDynamic 与宽回归，再用唯一 production owner 验证正常注册/open/close、entropy/collision、
owner poison、logical-name/begin failure、SQLite error/extended code、callback/handle teardown、Connection close
不确定和 route retirement。只有这些证据与真实 opened-authority producer 同批闭合后，才可修改本页的
`production_acceptance=deferred`。
