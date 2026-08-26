---
title: UserNode Windows Runner Launch Context Selection V1 权威草案
status: draft
reviewed_at: 2026-08-26
owners: node, compute, windows
proposed_feature_id: compute-user-node-windows-runner-launch-context-selection-v1
registration_status: unregistered_feature_workflow_unavailable
design_status: draft_frozen
implementation_status: source_written_uncompiled
verification_status: source_review_only
---

# UserNode Windows Runner Launch Context Selection V1 权威草案

对应验收见 [Launch Context Selection acceptance](user-node-windows-runner-launch-context-selection-acceptance.md)。上游候选
只来自 [Launch Path Discovery authority](user-node-windows-runner-launch-path-discovery-authority.md)，下游最终封印仍由
[Loader Load-Set authority](user-node-windows-runner-loader-load-set-authority.md) 负责。

## 1. 本批结论

本批写入一条仍不可生产的私有 typed chain：authenticated exact launch-context intent、authenticated pre-lease PE material、
preliminary unresolved resolution request plan、GrantReady exact terminal/disposition plan、grants/统一 package+system lease
custody、post-lease same-owner cross-binding，以及 QueryVerified lineage 到 process pre-create projection。preliminary plan 仍只冻结
resolver 必须消费的精确请求；GrantReady 合同虽已写，真实 resolver/owner producer 仍为 `missing`。该 owner graph 修正
grant/lease 前错误持有 final resolution 的时间顺序，并消除 loader resolution digest 与 process required-launch-context digest 的
直接自引用。

证据严格为 `source_written/source_review_only/implementation_uncompiled/implementation_unrun`、
`passed=0/failed=0`、Windows dynamic=`0`、`migration/table/writer=none/none/none`。Manifest V1、InstallPlan V1 与
work-admission profile 均无 authenticated CWD selector；因此 selector、retained-handle PE parser、GrantReady resolver、
grant/lease positive-consuming transition、post-lease sealer、query 与 live-process machine query-back 的真实 producer 都保持
`missing`，新类型不能解释为 Runner 已可启动。

## 2. Authenticated exact selector

`AuthenticatedWindowsRunnerLaunchContextIntent` 是版本化、Control-signed 或同等 sealed typed source 的目标形状。它必须
绑定同一 plugin/release admission source+receipt、manifest/envelope、grant、target、Runner path、entrypoint argv、control
key/generation、signature receipt，以及以下完整 projection：

- working directory 只能是 `PackageRoot` 或 `PlanDirectory { directory_ordinal, relative_path }`；
- target architecture、native/WOW64 process-machine expectation；
- 显式空 Unicode environment、禁止继承 handles、精确 process-creation flags；
- required restricted-token/AppContainer policy expectation；它不是 token handle 或 launch-security authority；
- 有序 DLL search roles 与有序 `preloaded/api_set/known_dll/side_by_side/filesystem` routes，禁止 ambient PATH。

不得默认 package root、Runner parent、第一个 plan directory 或当前进程 CWD。Plan-directory 选择必须同时核对 plan ordinal、
relative path、candidate ordinal、FileId identity、component-set 与 observation receipt。最终 selected binding 分开保留：

```text
application file identity
application directory identity
package root identity
selected working-directory identity
```

Runner 可以位于 `bin/runner.exe`；application directory、package root 与 CWD 是三个不同概念，同一物理目录承担两个 search
role 时也不能丢失 role/step ordinal。

## 3. Authenticated pre-lease PE material

`AuthenticatedWindowsPreLeasePeMaterial` 只绑定 retained package-file handles 上的 admission、plan/evidence、Runner、target
architecture、expected process-machine context、parser policy 与 parser input receipt。每个 parsed package image 都必须按自己的
package-file ordinal 与 plan/evidence/retained file 的 relative path、sealed digest、size、FileId 做 exact cross-binding。parsed-image
ordinal 与 package-file ordinal 是不同坐标；Runner root通过 package-file ordinal 查找后再绑定其 parsed-image ordinal，不得按
vector index 猜测。

material 必须覆盖：

- 每个 normal/delay descriptor+thunk 的 base import request、imported symbol name 或 ordinal；forwarder 不冒充第三种 import
  table edge，而是附加到 exact source edge 的独立 source-export hop request；
- canonical lowercase DLL basename/module-cache key；
- forwarder source export、target DLL/symbol、连续 hop ordinal、跨 hop module continuity 与逐跳 evidence；
- normal descriptor/thunk → delay descriptor/thunk → 按 source-edge/hop 排序的 forwarder hop 的 canonical merge ordinal/rule；
- retained package-image 范围内 Runner-rooted reachable closure、cycle receipt、maximum depth、reachable-set digest；
- authenticated preloaded/bootstrap module set、retained package importer/forwarder-source closure 与 module-cache collision closure。

它不包含、预测或伪造 name/component grant、FileId content lease、lease generation、KnownDLL section、API-set/SxS live-OS
authority、final PE graph、launch-path authority或 resolution authority。真实 parser producer仍以 private `Infallible` 缺口保持
不可达。它不覆盖 ordinary system image 自身 imports、真正系统递归闭包或递归 API-set DAG；API-set host 当前只允许
non-recursive terminal，nested redirection 必须 fail-closed。

## 4. Preliminary unresolved resolution request plan

`PreliminaryWindowsRunnerResolutionRequestPlan` 由完整 discovery owner、authenticated intent 与 pre-lease material 做纯
borrowed validation 后形成。它是 request skeleton，不声称已知道任何 name 的 terminal resolution 或 present/absent/shadow
disposition。当前固定：

1. exact selected application/application-directory/CWD binding；
2. 有序 search-directory role/target bindings；retained candidate带同一目录 identity+observation receipt，system/Windows/SxS
   只标记 `ExternalTypedOwnerRequired`；
3. 每条 import edge 一份 normalized module request，并引用完整 ordered search-step ordinals；其状态逐字为
   `exact_terminal_and_step_dispositions_required_before_grant`；
4. application 与 CWD 的 parent identity、normalized component、expected object identity requests；
5. extraction plan中全部 package file ordinals 的 content-lease request refs；在 exact terminal resolution 与 canonical dedupe 前，
   不生成 system-image lease request；
6. route、parser、preloaded set、selected context、candidate set与 admission/plan/evidence 的域分隔摘要。

私有 `GrantReadyWindowsRunnerResolutionPlan` 合同必须消费整个 `PreliminaryResolutionRequestsPlannedWork`，补齐 exact terminal、
每步 disposition、external directory typed owners、KnownDLL section-image mapping + namespace generation、OS-build/schema-bound
non-recursive API-set host，以及 ordinary filesystem system images 的 canonical dedupe。每个 leaf digest、最终 search directory/
sequence/searched-name projection和 exact typed edge locator 都必须重算；`ShadowedByEarlierName` producer 当前 fail-closed。
`GrantReadyWindowsRunnerResolutionPrerequisite` 的真实 producer仍由 private `Infallible` 阻断。API-set、KnownDLL、SxS、filesystem
与 preloaded 在 preliminary request skeleton 中都不是 live OS 事实、grant、immutable section 或 content lease。planning success
整体持有 discovery、intent、PE material 与 request plan；failure也保留前三个 owner。不得提供成功 `into_parts`、只取回
admission 的 extractor、scalar retry permit、Clone/Copy/Serde、path/raw-handle/File escape。

## 5. 修正后的线性时间顺序

```text
LaunchPathDiscoveredWork
→ authenticated exact selection + pre-lease PE material
→ PreliminaryResolutionRequestsPlannedWork
→ exact terminals + per-step dispositions + external directory owners
→ GrantReadyWindowsRunnerResolutionPrerequisite
→ searched-name / launch-component grants
→ all package + deduplicated resolved-filesystem-system immutable-content leases
→ same-handle full-package rehash/reparse under grants+leases
→ exact prelease-image/import-edge ↔ postlease-image/import-binding cross-binding receipt
→ final exact PE graph + launch path + loader resolution seal
→ consuming grant/lease generation query
→ close/reopen/final currentness
→ retained process pre-create launch-context projection
```

name-grant failure持有 whole grant-ready owner；content-lease failure持有 pre-final namespace grants、统一 package/system acquired
lease custody、active dispatch与 pending refs；纯 borrow failure持有 whole preliminary request owner。name-grant 与 system-image
positive receipt虽已有 request/nonce/exact response bytes+digest/self-binding 的 typed custody，消费 exact attempt 形成正向 owner 的
transition仍以 `Infallible` 保持不可达。namespace query failure持有 post-lease whole prerequisite。任何 grants/leases 前路径都不
持有 `SealedWindowsLoaderResolutionAuthority`。final resolution只能在全部 grants 与 package/system leases 完成后进入
`PostLeaseSealedWindowsRunnerLoadSetPreQueryPrerequisite`；same-owner image/edge cross-binding、QueryVerified lineage 和 pre-create
projection已有 source shape，但 post-lease sealer/query/positive-consuming producer继续不可构造。pre-create projection只是 retained
typed lineage，不是 post-create `IsWow64Process2`/process-machine query receipt。

## 6. 无环摘要链

摘要权威固定为：

```text
launch_context_selector_digest
→ selected-context binding + preliminary_resolution_request_plan_digest
→ grant_ready_resolution_plan_digest
→ windows_loader_resolution_profile.v2
→ windows_runner_required_launch_context.v2
→ process start material
```

final loader resolution profile V2 必须同时包含 selector digest、preliminary request-plan digest与 grant-ready plan digest；不得
包含未来 required process context digest。required process launch context V3 则同时包含 selector digest与 final startup/import
resolution profile digest。其 expected digest存入不参与 resolution-profile hash 的 launch-security bridge，并在 process policy
重算后做 equality check。由此 final resolution先完成、process context后完成，不要求 SHA-256 fixed point，也不丢失 authenticated
expected-value比较。未来 final sealer必须从同一线性 owner核对摘要，不能接受调用方 scalar 或跨 generation splice。

## 7. Blocker、Ready 与零效果

```text
existing_extraction_directory_access_share_compatibility = source_seam_written_windows_dynamic_unverified
launch_path_handle_chain_discovery = source_written_windows_dynamic_unverified
launch_context_selection_contract = source_written_uncompiled_unrun
authenticated_launch_context_source_producer = missing
prelease_authenticated_pe_material = source_written_uncompiled_unrun
authenticated_prelease_pe_parser_producer = missing
preliminary_resolution_request_plan = source_written_uncompiled_unrun
grant_ready_resolution_contract = source_written_uncompiled_unrun
grant_ready_resolution_producer = missing
external_search_directory_authority = missing
launch_path_component_grant_backend = missing
searched_name_grant_acquisition_backend = missing
searched_name_grant_positive_consuming_transition = missing
fileid_immutable_content_lease_backend = missing
system_image_positive_consuming_transition = missing
postlease_exact_pe_import_graph_sealer = missing
postlease_same_owner_lineage_contract = source_written_uncompiled_unrun
startup_import_resolution_producer = missing
post_create_live_process_machine_context_queryback = missing
```

四项 Ready gap逐字保持 `missing`：`node_local_authority_currentness`、`runtime_transition_authority`、
`host_runtime_authority`、`v15_authenticated_session`。loader 18 项 effect逐字保持 `none`：`runtime_phase`、
`runtime_generation`、`runtime_start`、`runtime_resume`、`runtime_store`、`health`、`readiness`、`node`、`provider`、
`route`、`offer`、`capacity`、`execution`、`attempt`、`lease`、`usage`、`settlement`、`money`。

## 8. 源码铺设与生产可达

源码铺设顺序允许在架构阶段继续冻结下一 typed contract；生产可达顺序不变：extraction-share 与 discovery 的真实 Windows
动态矩阵仍必须先留下非零通过证据，且 selector/parser/grant/lease/post-lease sealer/query/reopen/currentness producers 全部闭合后
才可提升 loader predecessor。任何 source shape、digest 或人工 review 都不能升级 Runtime、Ready、Provider 或经济效果。
