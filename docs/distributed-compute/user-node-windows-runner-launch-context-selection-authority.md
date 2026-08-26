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
post-lease system-image fixed-point另见
[Recursive System-Image Closure authority](user-node-windows-runner-recursive-system-image-closure-authority.md)，逐波 policy/custody见
[Recursive System-Image Acquisition Custody authority](user-node-windows-runner-recursive-system-image-acquisition-custody-authority.md)，
policy signature verification与逐 A0/Ak currentness见
[Recursive Policy Currentness authority](user-node-windows-runner-recursive-policy-currentness-authority.md)，
Ak forward plan见
[Recursive Wave Resolution Plan authority](user-node-windows-runner-recursive-wave-resolution-plan-authority.md)。

## 1. 本批结论

本批写入一条仍不可生产的私有 typed chain：authenticated exact launch-context intent、authenticated pre-lease PE material、
preliminary unresolved resolution request plan、GrantReady wave-zero terminal/disposition plan、base acquisition、逐 producer wave
grants/route-specific owners/leases、最终统一 package+system lease custody、post-lease same-owner cross-binding，以及 QueryVerified
lineage到 process pre-create projection。preliminary plan仍只冻结
resolver 必须消费的精确请求；GrantReady 合同虽已写，真实 resolver/owner producer 仍为 `missing`。该 owner graph 修正
grant/lease 前错误持有 final resolution 的时间顺序，并消除 loader resolution digest 与 process required-launch-context digest 的
直接自引用。独立的 source-only recursive final-projection envelope现已冻结 wave-zero prefix、system-postlease suffix、
producer-bound parse receipts、frontier/fixpoint与 final slice摘要；authenticated recursive policy与逐 producer wave whole-owner
custody contract也已写。A0复用 GrantReady，Ak canonical request/resolution plan V1、exact-vector limit派生、
currentness-pending与 DispatchReady source shape现已补齐；signed envelope、typed verification evidence与逐 A0/Ak currentness
authorization合同也已写，但真实
signature verifier/currentness backend、parser/resolver、grant/candidate/lease backend、positive advancer与 sealer producer仍
`missing`。

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

递归 limits与 parser policy不再由 final sealer从上述 intent digest旁路推导。独立 recursive policy payload V1（由 signed
envelope V1验证并进入 authenticated binding V2）必须直接逐项
绑定本 intent digest、preliminary request-plan digest、parser policy、authenticated preloaded set、上述 route order与六项
limits；admission/manifest/machine/search lineage经前两项 authenticated digests传递。payload V1必须 exact复用 route order，不得收窄
或按 wave扩大；未来若允许显式收窄，必须升级 policy schema/domain。

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
architecture、expected process-machine context、parser policy 与 parser input receipt。parser policy只有与独立 authenticated
recursive policy逐项相等并通过其签名 lineage时才能称为 authenticated，不能由裸 digest自证。每个 parsed package image都必须按自己的
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
→ wave-zero exact terminals + per-step dispositions + external directory owners
→ GrantReadyWindowsRunnerResolutionPrerequisite
→ whole GrantReady borrowed validation + exact Control-ring/trusted-time currentness query
→ PolicyCurrentGrantReadyWindowsRunnerResolutionPrerequisite
→ base searched-name / launch-component grants + package leases + base route-specific system owners
→ same-handle full-package rehash/reparse under package leases
→ exact prelease-image/import-edge ↔ postlease-image/import-binding cross-binding receipt
→ producer wave `k` same-owner parse + canonical request/resolution plan V1
→ exact-vector validation + currentness-pending + point-of-use authorization + DispatchReady + grants/candidates/leases
→ [next frontier非空] target parse wave `k+1` owners + deterministic suffix；[empty] terminal `A_N` target=None
→ empty recursive frontier + final aggregate + exact final edge/name/system-owner reverse projection
→ final exact PE graph + launch path + loader resolution seal
→ consuming grant/lease generation query
→ close/reopen/final currentness
→ retained process pre-create launch-context projection
```

base name-grant failure持有 whole `PolicyCurrentGrantReadyWindowsRunnerResolutionPrerequisite`；base content-lease failure持有 outer
`PolicyCurrentPreFinalWindowsLoaderNamespaceGrantSet`，其 policy/authorization保持在 policy-free inner namespace之外，并保留已取得
package/system owner custody、active dispatch与 pending refs；recursive failure另按 producer wave保留 whole earlier graph、active
attempt、returned outcomes与 pending refs。纯 borrow failure持有 whole preliminary request owner。name-grant与 system-image
positive receipt虽已有 request/nonce/exact response bytes+digest/self-binding 的 typed custody，消费 exact attempt 形成正向 owner 的
transition仍以 `Infallible` 保持不可达。namespace query failure持有 post-lease whole prerequisite。任何 grants/leases 前路径都不
持有 `SealedWindowsLoaderResolutionAuthority`。final resolution只能在 base acquisition与全部 producer-wave
grants/owners/leases完成且 frontier为空后进入 `PostLeaseSealedWindowsRunnerLoadSetPreQueryPrerequisite`；same-owner
image/edge cross-binding、QueryVerified lineage 和 pre-create
projection已有 source shape，但 post-lease sealer/query/positive-consuming producer继续不可构造。pre-create projection只是 retained
typed lineage，不是 post-create `IsWow64Process2`/process-machine query receipt。

## 6. 无环摘要链

摘要权威固定为：

```text
launch_context_selector_digest
→ selected-context binding + preliminary_resolution_request_plan_digest
├→ grant_ready_resolution_plan_digest
└→ authenticated_recursive_resolution_policy_digest
authenticated_recursive_resolution_policy_digest + grant_ready_resolution_plan_digest
→ [A0 GrantReady reuse | Ak request/resolution plan V1]
→ recursive_wave_acquisition_receipt_v3_digests
→ receipt-set/acquisition-chain V1
→ recursive_resolution_closure_digest
→ windows_loader_resolution_profile.v3
→ windows_runner_required_launch_context.v3
→ process start material
```

final loader resolution profile V3 必须同时包含 selector digest、preliminary request-plan、grant-ready plan与 recursive closure
digest；closure内逐项绑定 authenticated recursive policy与 acquisition receipts。profile不得
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
recursive_system_import_closure_contract = source_written_uncompiled_unrun
recursive_system_import_acquisition_custody_contract = source_written_uncompiled_unrun
recursive_wave_request_resolution_plan_contract = source_written_uncompiled_unrun
authenticated_recursive_policy_producer = missing
recursive_policy_signature_verification_contract = source_written_uncompiled_unrun
recursive_policy_dispatch_currentness_contract = source_written_uncompiled_unrun
recursive_policy_signature_verifier_producer = missing
recursive_policy_dispatch_currentness_backend = missing
recursive_wave_plan_resolver_producer = missing
recursive_wave_acquisition_backend = missing
recursive_wave_positive_advancer = missing
recursive_system_import_closure_producer = missing
startup_import_resolution_producer = missing
post_create_live_process_machine_context_queryback = missing
```

四项 Ready gap逐字保持 `missing`：`node_local_authority_currentness`、`runtime_transition_authority`、
`host_runtime_authority`、`v15_authenticated_session`。loader 18 项 effect逐字保持 `none`：`runtime_phase`、
`runtime_generation`、`runtime_start`、`runtime_resume`、`runtime_store`、`health`、`readiness`、`node`、`provider`、
`route`、`offer`、`capacity`、`execution`、`attempt`、`lease`、`usage`、`settlement`、`money`。

## 8. 源码铺设与生产可达

源码铺设顺序允许在架构阶段继续冻结下一 typed contract；生产可达顺序不变：extraction-share 与 discovery 的真实 Windows
动态矩阵仍必须先留下非零通过证据，且 selector/policy/parser/recursive-plan resolver/grant/candidate/lease/positive-advancer/post-lease
sealer/query/reopen/currentness producers 全部闭合后
才可提升 loader predecessor。任何 source shape、digest 或人工 review 都不能升级 Runtime、Ready、Provider 或经济效果。
