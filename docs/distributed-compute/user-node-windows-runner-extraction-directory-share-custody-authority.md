---
title: UserNode Windows Runner Extraction Directory Share Custody V1 权威草案
status: draft
reviewed_at: 2026-08-26
owners: node, compute, windows
proposed_feature_id: compute-user-node-windows-runner-extraction-directory-share-custody-v1
registration_status: unregistered_feature_workflow_unavailable
design_status: draft_frozen
implementation_status: source_written_uncompiled
verification_status: source_review_only
---

# UserNode Windows Runner Extraction Directory Share Custody V1 权威草案

对应验收见 [Extraction Directory Share Custody acceptance](user-node-windows-runner-extraction-directory-share-custody-acceptance.md)。

## 1. 本批结论

本批只修正 Windows extraction staging 目录 owner 与 loader 前置探针之间的 access/share 矛盾，并把证明随原
owner 线性送入既有 loader owner graph。它不创建第二套 loader graph，不替代
[`Loader Load-Set authority`](user-node-windows-runner-loader-load-set-authority.md)，也不推进 process resume、
Runtime 或 Ready。

当前证据严格为 `source_written/source_review_only/implementation_uncompiled/implementation_unrun`，
`passed=0/failed=0`。Windows 动态兼容性仍是 `missing`；本文的常量与类型形状不能解释为已在 Windows
上打开成功、已编译或已验收。

## 2. 查重与责任边界

本批复用：

- `PinnedManagedDirectory` 的 parent-relative native open、FileId、reparse 与 canonical-path 检查；
- `PreparedComputePluginCandidateStaging`、`ExtractedComputePluginCandidateArchive` 的既有 extraction custody；
- `LoaderTransitionAuthorityCustody`、`LoaderLockedWorkAdmittedPluginSlot` 与现有失败 custody。

新增边界只应位于 `node_agent_managed_fs::extraction_loader_directory` 及其 Windows platform adapter，并由
staging/extraction/loader transition 线性消费。禁止另建平行 Store、Service、API、migration、表、writer、
raw-handle helper 或 path/digest receipt。

## 3. Windows access/share 安全等式

### 3.1 原 staging create owner

每个新 staging 目录的原始 owner 保持既有可写、可删除目录 profile：

```text
create.desired_access =
  FILE_READ_ATTRIBUTES | FILE_TRAVERSE | FILE_WRITE_DATA | SYNCHRONIZE | DELETE
create.share_access = FILE_SHARE_READ | FILE_SHARE_WRITE
create.disposition = FILE_CREATE
create.options =
  FILE_DIRECTORY_FILE | FILE_OPEN_REPARSE_POINT | FILE_SYNCHRONOUS_IO_NONALERT
```

这里 `DELETE ∈ desired_access` 且 `FILE_SHARE_DELETE ∉ share_access` 是有意的：原 owner 继续阻止目录被其他
句柄 rename/delete。不得为了让探针成功而降低原 owner 的 DELETE 权限或提前关闭它。

### 3.2 purpose-specific compatibility probe

探针必须通过已持有的 parent handle 与单个 normal relative component 调用 `NtCreateFile`：

```text
OBJECT_ATTRIBUTES.RootDirectory = retained_parent_handle
OBJECT_ATTRIBUTES.ObjectName = single_normal_relative_component
probe.desired_access = FILE_READ_ATTRIBUTES | FILE_TRAVERSE | SYNCHRONIZE
probe.share_access = FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE
probe.disposition = FILE_OPEN
probe.options =
  FILE_DIRECTORY_FILE | FILE_OPEN_REPARSE_POINT | FILE_SYNCHRONOUS_IO_NONALERT
```

探针不是新的目录 authority，不得取得 DELETE 或 `FILE_WRITE_DATA`。成功后还必须从句柄查询 granted access：

```text
(retained_owner.granted_access & DELETE) = DELETE
(probe.granted_access & probe.desired_access) = probe.desired_access
(probe.granted_access & (DELETE | FILE_WRITE_DATA)) = 0
```

只比较请求常量、只按 absolute path reopen、或允许系统默默扩大 probe 权限，均不构成 receipt。

## 4. 同一目录与非 reparse 证明

receipt 只能由同时存活的原 owner 与 probe 句柄派生，并满足：

```text
owner.volume_serial = probe.volume_serial = managed_root.volume_serial
owner.file_id = probe.file_id
owner.is_directory = probe.is_directory = true
owner.is_reparse_point = probe.is_reparse_point = false
canonical(owner_handle) = expected_retained_directory_path
canonical(probe_handle) = canonical(owner_handle)
```

FileId 比较必须包含同 volume 约束；path、digest、文件名或 relative string 均不能替代 FileId。canonical path
必须来自句柄，且继续满足 managed-fs 的稳定 volume-GUID path 规则。任何 identity、类型、reparse 或 path
不等都 fail closed。

## 5. 线性 owner 与 receipt

`PinnedManagedExtractionLoaderDirectory` 的成功 custody 同时拥有：

1. 原 `PinnedManagedDirectory`，包括其完整 ancestor handle chain 与 DELETE-capable final handle；
2. share-delete probe handle；
3. `ManagedExtractionLoaderDirectoryShareReceipt`。

原 owner 从 probe 前、probe 中到 loader handoff 全程存活，probe 永不替换它。receipt 字段与构造器保持 private，
类型不得实现 `Clone`、`Copy`、Serialize 或 Deserialize；也不得提供 raw handle、path、FileId digest 或其他 scalar
constructor。调用方只能 by-value 移动带 receipt 的 owner。

package-root receipt 必须随 `PreparedComputePluginCandidateStaging`、extracted archive 与 admission owner 逐层线性
进入以 `LoaderTransitionAuthorityCustody` 为 authority residue 的既有 loader transition graph。package root 与 plan
directories 仍各自只有一个 owner；不得把 receipt 拷贝进 detached evidence，也不得新增平行成功结果。

## 6. staging descendant 必须 retained-directory-relative

完成 custody 转换后，以下操作必须从 `PinnedManagedExtractionLoaderDirectory` 或 extraction plan 中已经保留的
parent directory owner 开始，以单个 component parent-relative 执行：

- create-new 计划内子目录；extraction 按规范 parent index 选择已保留的 root/plan-directory owner，不重开
  existing descendant。每个 retained child owner 都保持 DELETE desired + share R|W，再以
  share R|W|DELETE 的 narrow probe 校验同 volume/FileId、目录、非 reparse 与 handle canonical path；probe
  在校验后关闭，最终 `PinnedManagedDirectory` owner chain 保留；
- 创建计划内 regular file；
- 在 retained package root 创建 staging seal。

长期 typed share receipt 只属于 package root；plan child 不伪造平行 receipt。`relative_root` 只保留为
binding/evidence 字段，不再允许把它拼回 managed root 后重新 traversal。禁止 existing-descendant reopen、
AlreadyExists fallback、absolute path、root-relative full staging path、`..`、多 component native ObjectName、reparse
follow 或从 scalar path 重建 owner。任一 descendant failure 必须返回仍保有受影响 owner graph 的 typed failure。

## 7. 失败保管

失败必须保留能够解释当时状态的完整 owner graph：

- probe 打开前失败：保留原 directory owner 与完整 ancestor chain；
- probe 已打开后查询、identity、reparse 或 path 校验失败：同时保留原 owner 与 probe；
- child create 后 compatibility seal 失败：保留新 child 的原 owner、可能存在的 probe，并让 parent owner 继续由调用方持有；
- purpose-specific descendant open/create/seal 失败：调用方继续持有 staging root；若新 child/probe 已打开，错误值
  继续持有该 child/probe。既有 extraction-wide error 对先前 sibling/file 的恢复形状不在本批重写范围内，本批不把
  局部 seam 外推成“整个解包失败图已 parked”。

失败类型不得提供 raw-handle/path extractor 或允许用 digest retry 重建 authority。cleanup 只能通过明确 consuming seam
取得仍然 owned 的原目录；不确定结果继续 quarantine/park，不可声称安全关闭。

## 8. Loader、Ready 与零效果边界

本批只关闭 `existing_extraction_directory_access_share_compatibility` 的静态合同形状；Windows 动态证明尚缺，
loader 的 exact PE/import resolution、searched-name grants、FileId leases、launch-path authority、dynamic module-load
enforcement 与 parent-relative file reopen producers 仍缺。因此 `LoaderLockedWorkAdmittedPluginSlot` 的真实成功链、
process launch/resume 与 Ready 均不可由本批推出。

四项 Ready gap 均保持：

| gap | 状态 |
|---|---|
| `node_local_authority_currentness` | `missing` |
| `runtime_transition_authority` | `missing` |
| `host_runtime_authority` | `missing` |
| `v15_authenticated_session` | `missing` |

loader 18 项 effect 全为 `none`：

```text
runtime_phase, runtime_generation, runtime_start, runtime_resume, runtime_store,
health, readiness, node, provider, route, offer, capacity, execution, attempt,
lease, usage, settlement, money
```

本批 `migration/table/writer=none/none/none`，无 Store、Service、HTTP/MCP、公开 API、网络、节点命令、Offer、
Attempt、Lease、Usage、Settlement 或 Money 写效果。

## 9. 后续门槛

只有在 Windows 上动态覆盖 owner/probe coexistence、granted-access、same-volume FileId、reparse、handle canonical
path、descendant-relative mutation 与每个 failure owner graph，并留下 `passed>0` 的可复验证据后，才能把该
兼容性从 `missing` 提升。该提升仍不自动填补四项 Ready gap，也不使 loader 或进程成功链可达。
