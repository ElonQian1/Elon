---
title: UserNode Windows Runner Extraction Directory Share Custody V1 验收草案
status: draft
reviewed_at: 2026-08-26
owners: node, compute, windows
proposed_feature_id: compute-user-node-windows-runner-extraction-directory-share-custody-v1
registration_status: unregistered_feature_workflow_unavailable
design_status: draft_frozen
implementation_status: source_written_uncompiled
verification_status: source_review_only
---

# UserNode Windows Runner Extraction Directory Share Custody V1 验收草案

权威合同见 [Extraction Directory Share Custody authority](user-node-windows-runner-extraction-directory-share-custody-authority.md)。

## 1. 本批证据等级

本批只允许记录 source shape 与静态审阅目标：

- implementation: `source_written/source_review_only/implementation_uncompiled`；
- runtime: `implementation_unrun`；
- Windows dynamic: `missing`；
- code acceptance: `passed=0/failed=0`；
- persistence: `migration/table/writer=none/none/none`。

没有运行编译、Rust test、Windows fixture、migration、SQLite、网络、设备或真实 Runner 验证；本文件不是验收通过
声明。

## 2. 文件责任

静态审阅面保持模块化：

| 文件/模块 | 唯一责任 |
|---|---|
| `node_agent_managed_fs/extraction_loader_directory.rs` | typed owner、handle-derived receipt、identity/path 校验与完整 failure custody |
| `node_agent_managed_fs/windows_extraction_loader_directory.rs` | exact Windows access/share probe 与 granted-access 查询 |
| `node_agent_managed_fs.rs`、`windows.rs`、`unsupported.rs` | 私有模块路由；Windows 实现与非 Windows fail-closed |
| `node_agent_compute_plugin_host/fetch_file/staging.rs` | staging root/descendant retained-directory-relative ownership |
| `candidate_extraction/zip/{extract,types}.rs` | package directories/files/seal 的线性持有与 loader handoff |
| `runtime_loader_load_set/{transition,model,failure}.rs` | receipt by-value 进入既有 loader success/failure graph |

不得把 Windows FFI、staging mutation、loader resolution 或 persistence 混入 receipt model 文件。

## 3. 静态源码审阅目标

### 3.1 access/share profile

- staging create owner 请求 `DELETE`，share 恰为 `FILE_SHARE_READ | FILE_SHARE_WRITE`；
- probe desired 恰为 `FILE_READ_ATTRIBUTES | FILE_TRAVERSE | SYNCHRONIZE`；
- probe share 恰为 `FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE`；
- probe 使用 parent handle + single normal component、`FILE_OPEN`、`FILE_DIRECTORY_FILE |
  FILE_OPEN_REPARSE_POINT | FILE_SYNCHRONOUS_IO_NONALERT`；
- granted access 复核 owner 含 DELETE、probe 含全部 narrow rights，且 probe 不含 DELETE/`FILE_WRITE_DATA`。

### 3.2 identity 与 canonical path

- owner、probe、managed root 的 volume identity 相等，owner/probe FileId 相等；
- 两个句柄都指向 directory 且不是 reparse point；
- owner canonical path 等于 retained binding path，probe canonical path 等于 owner；
- 所有比较都来自 live handle，不接受 path/digest scalar substitute。

### 3.3 线性 custody

- 原 DELETE owner 在 probe 前后及 loader handoff 中始终存在，probe 不替换它；
- receipt private、non-`Clone`、non-`Copy`、non-Serde，且无 raw handle/path/FileId digest constructor；
- package-root receipt 随其 typed wrapper by-value 进入现有 loader owner graph；plan directories 保留各自
  `PinnedManagedDirectory` owner，不复制长期 receipt；
- 无 detached receipt、第二个 loader success type 或 ordinary early drop。

### 3.4 descendant 与 failure

- directory/file/seal 从 retained package-root 或 plan 中已经保留的 exact parent owner 单 component
  parent-relative create-new；existing descendant reopen 与 AlreadyExists fallback 均不存在；
- plan child 的 retained owner 保持 DELETE desired + share R|W，并以 share R|W|DELETE 的 narrow
  probe 校验同 volume/FileId、目录、非 reparse 与 handle canonical path；probe 校验后关闭，最终 owner chain 保留；
- `relative_root` 不再用于从 managed root 重走 full path；
- before-probe failure 保留 owner；after-probe failure 保留 owner+probe；child failure 保留 child graph；
- purpose-specific failure 保留受影响的 owner/probe，调用方仍持有 staging root；本批不声称既有
  extraction-wide error 已新增全量 sibling/file parking。

以上只是审阅清单，不计入动态 `passed`。

## 4. 明确未验收矩阵

| 门禁 | 当前结果 | 说明 |
|---|---|---|
| Rust compile/check | `not_run` | 未编译 Windows/非 Windows cfg |
| Rust unit/source contract test | `not_run` | 未运行 receipt、visibility、trait 或 owner-shape 测试 |
| Windows owner/probe coexistence | `missing` | 未证明 DELETE owner 与 share-delete probe 可同时存活 |
| granted-access query | `missing` | 未在目标 Windows/NTFS/ReFS 上观察 access mask |
| same-volume/FileId | `missing` | 未注入 replace/rename/volume mismatch |
| directory/non-reparse | `missing` | 未覆盖 junction、symlink、mount/reparse cases |
| handle canonical path | `missing` | 未覆盖 alias、case、rename race 或 volume-GUID path |
| retained-directory-relative descendants | `missing` | 未证明 directory/file/seal 不再 root-relative reopen |
| failure owner graph | `missing` | 未逐阶段注入 open/query/identity/path/child failure |
| cleanup/loader handoff | `missing` | 未证明 receipt 线性进入既有 loader graph |
| real Runner/process resume | `out_of_scope` | loader 及 process producers 仍不可达 |

动态总计保持 `passed=0/failed=0`，不是 `failed=0` 即通过。

## 5. Ready gaps 与 loader effects

四项 gap 必须逐字保持：

```text
node_local_authority_currentness = missing
runtime_transition_authority = missing
host_runtime_authority = missing
v15_authenticated_session = missing
```

loader 18 项 effect 必须全部为 `none`：

| effects | 状态 |
|---|---|
| `runtime_phase`, `runtime_generation`, `runtime_start`, `runtime_resume`, `runtime_store` | `none` |
| `health`, `readiness`, `node`, `provider`, `route`, `offer`, `capacity` | `none` |
| `execution`, `attempt`, `lease`, `usage`, `settlement`, `money` | `none` |

任一 effect 非 `none`、任一 Ready gap 被本批标成 present，均为越权。

## 6. 未来 Windows 动态矩阵

后续专项至少应覆盖：

1. 新目录 owner 的 DELETE + share R|W 与 narrow probe share R|W|DELETE 成功 coexist；
2. probe 请求、granted access、disposition、options 任一漂移均 fail closed；
3. 同 volume/FileId 正例，以及 replace/rename/不同 volume 反例；
4. regular file、reparse directory、junction/symlink 反例；
5. owner/probe canonical-path 相等正例与 alias/race 反例；
6. nested directory 按 plan parent index 复用已保留 parent owner，regular file 与 seal 也只从 exact retained parent
   create-new；existing reopen/AlreadyExists fallback 不存在，child 临时 probe 校验后关闭而 owner chain 保留；
7. probe 前、probe 后、identity、path、child 与 handoff 每个故障点的完整 owner graph；
8. cleanup consuming seam 与 loader by-value handoff 均无 raw reconstruction；
9. 非 Windows backend 始终 fail closed。

只有留下命令、环境、断言和非零通过数后，才能更新 dynamic 状态；仍不得外推到 loader、process、Runtime 或
Ready 验收。

## 7. 负向验收

本批明确不证明：

- Windows 编译或运行通过；
- extraction/loader 动态兼容性已经关闭；
- exact PE/import resolution、FileId content lease、searched-name fence 或 dynamic-load enforcement 可用；
- `LoaderLockedWorkAdmittedPluginSlot`、process launch/resume、Host Runtime 或 Ready 可达；
- migration、Store、Service、API、节点命令、Offer、Attempt、Lease、Usage、Settlement 或 Money 效果存在。
