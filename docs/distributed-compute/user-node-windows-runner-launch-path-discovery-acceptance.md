---
title: UserNode Windows Runner Launch Path Discovery V1 验收草案
status: draft
reviewed_at: 2026-08-26
owners: node, compute, windows
proposed_feature_id: compute-user-node-windows-runner-launch-path-discovery-v1
registration_status: unregistered_feature_workflow_unavailable
design_status: draft_frozen
implementation_status: source_written_uncompiled
verification_status: source_review_only
---

# UserNode Windows Runner Launch Path Discovery V1 验收草案

权威合同见 [Launch Path Discovery authority](user-node-windows-runner-launch-path-discovery-authority.md)。

## 1. 本批证据等级

- implementation: `source_written/source_review_only/implementation_uncompiled`；
- runtime: `implementation_unrun`；
- Windows dynamic: `missing`；
- code acceptance: `passed=0/failed=0`；
- persistence: `migration/table/writer=none/none/none`。

没有运行编译、Rust test/source-contract test、Windows fixture、migration、SQLite、网络、设备或真实 Runner 验证。
`failed=0` 不表示通过。

## 2. 静态责任面

| 文件/模块 | 静态审阅责任 |
|---|---|
| `node_agent_managed_fs/loader_launch_path_discovery.rs` | opaque receipt/set、typed owner-only API、aggregate binding |
| `node_agent_managed_fs/windows_loader_launch_path_discovery.rs` | Windows granted-access 与 handle-path observation |
| `node_agent_managed_fs/{windows,unsupported}.rs` | Windows route 与 non-Windows fail-closed |
| `fetch_file/staging.rs`、`candidate_extraction/zip/types.rs` | package root/files/directories purpose-specific borrow view |
| `runtime_loader_load_set/launch_path_discovery.rs` | Runner ordinal binding、success/failure admission custody |
| `runtime_loader_load_set/policy.rs` | discovery/selection/grant blockers 与两阶段顺序 |

不得把 exact resolution、Win32 process create、persistence 或 Ready projection混入这些文件。

## 3. 静态源码审阅目标

### 3.1 typed 输入与 custody

- managed-fs 入口参数只包含 `PinnedManagedFile`、`PinnedManagedExtractionLoaderDirectory` 与
  `PinnedManagedDirectory` slice；
- runtime 入口按值接收并在成功/失败两侧返回同一 `DurableWorkAdmittedPluginSlot`；
- archive borrow view 同时保留 plan/evidence/package-root/directories/files，且不返回 path/raw handle；
- receipt、candidate set 与 success/failure fields private，non-`Clone`、non-`Copy`、non-Serde；
- 无 detached scalar constructor、caller-opened `File`、`AsRawHandle`/`BorrowedHandle` escape 或 reopen-by-path。

### 3.2 Windows handle observations

- application、package root 与每个 plan directory 都遍历 retained handle chain；
- 每个 object 重新查询 volume/FileId/type/reparse 与 discovery-minimum granted access；目录只要求
  attributes+traverse+sync，file 再要求 read-data，不冒充 DELETE/write exact recipe；
- parent-child canonical path 都来自 handles，差值只能是一个 normal component；
- canonical path 继续满足 stable Volume-GUID form，不接受 drive-letter/path alias；
- directory/file type精确、reparse一律拒绝、root volume/identity 与 ordinal/count 全匹配；
- share mode不能从 handle 回查；源码只绑定 directory delete-share-denied/file share-none 的 broad static class，不区分
  share-R/share-RW 或证明 exact opener/coexistence。

### 3.3 runtime binding

- Runner 由 authenticated work-admission profile 在 extraction plan 中唯一定位；
- plan/evidence/retained file 的 ordinal、relative path、digest、size、executable、FileId digest 全部一致；
- candidate binding 覆盖 admission source/receipt、plan/evidence、Runner 与 managed set；
- package root 与全部 plan-directory candidates 都存在，但没有一个被标记为 selected CWD；
- local error 原样返回 admission owner，不生成 retry、grant、loader、process 或 Ready authority。

### 3.4 blocker 与时序

- `launch_path_handle_chain_discovery=source_written_windows_dynamic_unverified`；
- `launch_path_exact_context_selection=missing`；
- `launch_path_component_grant_backend=missing`；
- exact `SealedWindowsLoaderLaunchPathAuthority` 仍含 `Infallible`；
- transition order 固定为 discovery/pre-lease material → authenticated selection/preliminary plan → grants → leases →
  same-handle rehash/reparse → final seal → query/reopen。

以上均只计为 source review，不增加动态 passed 数。

## 4. 明确未验收矩阵

| 门禁 | 当前结果 | 说明 |
|---|---|---|
| Rust compile/check | `not_run` | Windows/non-Windows cfg 均未编译 |
| Rust unit/source-contract test | `not_run` | 未运行 visibility、custody 或 source shape tests |
| retained chain observation | `missing` | 未在真实 Windows 观测每个 ancestor/final handle |
| granted-access query | `missing` | 未证明 discovery-minimum mask；exact opener recipe 不在 receipt 内 |
| volume/FileId/type/reparse | `missing` | 未覆盖 replace、different-volume、junction/symlink |
| Volume-GUID canonical chain | `missing` | 未覆盖 alias、case、rename race 与 multi-component 反例 |
| application/package/directory candidates | `missing` | 未对真实 extraction archive 执行 |
| failure admission custody | `missing` | 未逐阶段故障注入 |
| exact CWD selection/grants | `out_of_scope_missing` | 当前无 authenticated selector/backend |
| PE/resolution/lease/reopen/process | `out_of_scope_missing` | 最终 loader success 仍不可达 |

动态总计保持 `passed=0/failed=0`。

## 5. Ready gaps 与 effects

四项 gap 必须逐字保持 `missing`：

```text
node_local_authority_currentness
runtime_transition_authority
host_runtime_authority
v15_authenticated_session
```

loader 18 项 effects 必须全部为 `none`：

```text
runtime_phase, runtime_generation, runtime_start, runtime_resume, runtime_store,
health, readiness, node, provider, route, offer, capacity, execution, attempt,
lease, usage, settlement, money
```

## 6. 未来动态矩阵

后续专项至少覆盖：

1. Runner file、package root、零/一/多层 plan directory 正例；
2. missing/duplicate Runner、plan/evidence/file ordinal/count/digest/size/FileId 漂移；
3. granted access不足、file/directory type反转、reparse、different volume；
4. canonical drive alias、case/rename race、非 Volume-GUID、multi-component parent-child；
5. application/package-root/directory chain root前缀不一致；
6. 每个 query/identity/path/binding failure 都返回完整 admission owner；
7. non-Windows backend始终 fail closed；
8. discovery receipt不能被 exact selection/grant/resolution API 当作 authority。

只有留下环境、命令、断言和非零通过数后才能提升 discovery dynamic 状态；该提升仍不自动关闭 exact selection、
grant、PE/resolution、process、Runtime 或 Ready blocker。

## 7. 负向验收

本批明确不证明 Windows 编译/运行通过，不证明 extraction share兼容、exact CWD、component grant、PE graph、
startup/import resolution、FileId lease、reopen/currentness、process launch/resume、Host runtime、Ready、Provider、route、
Offer、Attempt、Lease、Usage、Settlement 或 Money 效果存在。
