---
title: UserNode Windows Runner Launch Path Discovery V1 权威草案
status: draft
reviewed_at: 2026-08-26
owners: node, compute, windows
proposed_feature_id: compute-user-node-windows-runner-launch-path-discovery-v1
registration_status: unregistered_feature_workflow_unavailable
design_status: draft_frozen
implementation_status: source_written_uncompiled
verification_status: source_review_only
---

# UserNode Windows Runner Launch Path Discovery V1 权威草案

对应验收见 [Launch Path Discovery acceptance](user-node-windows-runner-launch-path-discovery-acceptance.md)。上游目录
owner 合同见 [Extraction Directory Share Custody authority](user-node-windows-runner-extraction-directory-share-custody-authority.md)，
最终 loader 合同仍由 [Loader Load-Set authority](user-node-windows-runner-loader-load-set-authority.md) 负责。
候选之后的 typed source 合同见
[Launch Context Selection authority](user-node-windows-runner-launch-context-selection-authority.md)。

## 1. 本批结论

本批只建立 Windows Runner 启动路径的 **borrow-only 候选发现层**：从原 `DurableWorkAdmittedPluginSlot` 内已经保留的
Runner file、package root 与 extraction plan directory handles 观测逐组件 handle chain，并在成功和失败两侧都保留
完整 admission owner。它不选择 exact working directory，不取得 namespace grant，不构造
`SealedWindowsLoaderLaunchPathAuthority`，也不推进 process create/resume、Runtime 或 Ready。

证据严格为 `source_written/source_review_only/implementation_uncompiled/implementation_unrun`、
`passed=0/failed=0`。Windows 动态验证仍未运行；源码形状不能解释为句柄查询、access mask 或 Volume-GUID 路径已在
真实系统通过。

## 2. 查重与唯一责任

本批复用：

- extraction 的 retained DELETE package-root owner 与 share-delete probe typed custody；
- extraction plan/evidence 的确定性 file/directory ordinal；
- `PinnedManagedFile`、`PinnedManagedDirectory` 的完整 ancestor handles、FileId 与 root identity；
- work-admission 中已认证的 Runner relative path、digest、size 与 receipt pair。

新增职责只分两层：

| 层 | 唯一职责 |
|---|---|
| `node_agent_managed_fs::loader_launch_path_discovery` | 只接受 typed retained owners，生成不透明 handle-chain observations |
| `runtime_loader_load_set::launch_path_discovery` | 把 authenticated Runner ordinal 与 managed observations 绑定，并原样返回 admission owner |

不得另建 Store、Service、API、migration、表、writer、平行 loader success type、raw-handle helper 或 path/digest
constructor。最终 authority 的既有 `Infallible` 继续保留。

## 3. 输入与线性 custody

managed-fs 入口只能接受：

1. extraction plan/evidence 对应的 exact retained Runner `PinnedManagedFile`；
2. 同一 archive 内的 `PinnedManagedExtractionLoaderDirectory` package root；
3. plan ordinal 顺序的全部 `PinnedManagedDirectory` working-directory candidates。

外部调用方不得传入 `Path`、path string、FileId/digest scalar、`File`、raw handle 或自行 reopen 的对象。runtime 层必须
先核对 plan/evidence/file 数量、Runner ordinal/path/digest/size/executable 与 retained FileId digest，再调用 managed-fs。
发现成功返回 `LaunchPathDiscoveredWork { admitted, candidates }`；本地失败返回
`LaunchPathDiscoveryFailure { error, admitted }`。两侧均不得从 scalars 重建 owner 或产生 retry authority。

## 4. handle-chain observation

每个 application/package-root/plan-directory receipt 必须从仍存活的 handle chain 推导并逐项检查：

```text
root volume = every component volume = final object volume
every directory: is_directory=true, is_reparse_point=false
application final object: is_directory=false, is_reparse_point=false
parent(handle canonical) + exactly one normal component = child(handle canonical)
canonical path is handle-derived stable Volume-GUID form
queried access contains discovery minimum: attributes+sync and directory traverse or file read-data
```

FileId 必须与 volume 一起比较；basename、relative path、canonical string 或 digest 不能替代 object identity。component
ordinal 必须连续，plan directory ordinal 必须与 extraction plan 原顺序一致。任一空 chain、root mismatch、type/reparse、
multi-component、canonical alias、access insufficiency 或数量漂移都 fail closed。

Windows share mode不能从既有句柄可靠回查。本批只把异质目录 owners 归入“delete-share denied”静态类别，把 file
owner 归入“share-none”静态类别，并与 queried discovery-minimum access 一起摘要；它不区分 initial/prefix 的 share-R、
managed/extraction 的 share-RW，也不证明 extraction owner 的 `DELETE|FILE_WRITE_DATA` exact opener recipe。该 broad
class 不是动态 coexistence、exact grant 或 opener receipt，不能提升 extraction share matrix。

## 5. 候选集不是 exact launch authority

候选集固定包含：

- authenticated Runner application observation；
- package-root directory observation；
- extraction plan 中全部 directory observations；
- admission source/receipt、plan/evidence、Runner ordinal/FileId 与 managed observation 的聚合绑定。

候选集不含“选中的 CWD”。当前 work-admission 只有 Runner path，没有 authenticated working-directory selector；因此不得
默认选择 package root、Runner parent 或第一个 plan directory。未来 launch-context authority 必须从候选集中选择一个
exact CWD，再取得逐组件 namespace grants。discovery receipt 既不是 grant，也不是
`SealedWindowsLoaderLaunchPathAuthority` 的替代品。

类型字段与构造器保持 private，类型不得实现 `Clone`、`Copy`、Serialize 或 Deserialize，不得暴露 component full path、
`File`、raw handle 或可脱离 owner graph 使用的 scalar constructor。Debug 只显示 redacted/count 状态。

## 6. loader 两阶段顺序

现有 loader 文档曾把 final PE/resolution authority 与 grant/lease 前置发现混在同一阶段，形成循环：final package parser
material 绑定真实 content-lease generation，但 lease 又依赖 preliminary resolution。权威顺序修正为：

```text
retained admitted owners
→ borrow-only launch-path discovery + pre-lease authenticated PE material
→ authenticated launch-context selection + preliminary unresolved request plan
→ GrantReady contract → missing exact terminal/disposition/external-directory resolver/producer
→ searched-name/launch-component grants
→ all package + deduplicated resolved-filesystem-system FileId content leases
→ lease 下 same-handle rehash/re-parse
→ sealed exact PE graph + launch-path + startup/import resolution
→ consuming generation query
→ close/reopen/final currentness
```

本专题只负责第一行中的 launch-path discovery seam。下一专题已写入 uninhabited PE material、selection、unresolved request、
GrantReady private plan/typed movable owners与 post-lease same-owner lineage source shapes；PE source shape只冻结 package-image
base import/separate forwarder-hop、cycle/depth与 canonical merge。真实 PE parser、authenticated selector、exact terminal/
disposition resolver、external directory owner、grant/lease positive advancer、seal、query 或 reopen producer仍不存在；它也不证明
system-image recursive closure。request skeleton不能被解释为 GrantReady authority，GrantReady contract不能被解释为 producer。

## 7. blocker、Ready 与零效果

本批细分 blocker：

```text
launch_path_handle_chain_discovery = source_written_windows_dynamic_unverified
launch_context_selection_contract = source_written_uncompiled_unrun
authenticated_launch_context_source_producer = missing
prelease_authenticated_pe_material = source_written_uncompiled_unrun
authenticated_prelease_pe_parser_producer = missing
preliminary_resolution_request_plan = source_written_uncompiled_unrun
grant_ready_resolution_contract = source_written_uncompiled_unrun
grant_ready_resolution_producer = missing
external_search_directory_authority_producer = missing
launch_path_component_grant_backend = missing
```

上游 `existing_extraction_directory_access_share_compatibility` 仍是
`source_seam_written_windows_dynamic_unverified`；exact PE、startup/import resolution、FileId lease、name fence、live-OS、
reopen/recovery 与 dynamic-load enforcement 仍为 `missing`。四项 Ready gap 保持 `missing`：
`node_local_authority_currentness`、`runtime_transition_authority`、`host_runtime_authority`、
`v15_authenticated_session`。

loader 18 项 effect 全为 `none`：`runtime_phase`、`runtime_generation`、`runtime_start`、`runtime_resume`、
`runtime_store`、`health`、`readiness`、`node`、`provider`、`route`、`offer`、`capacity`、`execution`、`attempt`、
`lease`、`usage`、`settlement`、`money`。本批 `migration/table/writer=none/none/none`。

## 8. 后续门槛

源码铺设允许继续冻结后续 typed contracts；生产可达仍按以下门槛：

1. 完成 extraction directory share custody 与本发现层的 Windows 动态矩阵并留下 `passed>0` 证据；
2. 实现 authenticated selector/parser producers，并动态验收 exact CWD 与 PE material；
3. 补齐 exact terminal/disposition、external directory owners及 resolved-system canonical dedupe，形成 grant-ready owner；
4. 实现 grants/leases，再在 leases 下重哈希/重解析并封印最终 graph/resolution；
5. 完成 query/reopen/recovery 后，才进入 launch-security、pre-create currentness、process create 与 pre-resume。

任何后续步骤都不能从本批 receipt 或 binding digest 推导“启动路径已授权”或“loader load-set 已锁定”。
