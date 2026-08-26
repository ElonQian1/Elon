---
title: UserNode Windows Runner Recursive System-Image Closure V1 权威草案
status: draft
reviewed_at: 2026-08-26
owners: node, compute, windows
proposed_feature_id: compute-user-node-windows-runner-recursive-system-image-closure-v1
registration_status: unregistered_feature_workflow_unavailable
design_status: draft_frozen
implementation_status: source_written_uncompiled
verification_status: source_review_only
---

# UserNode Windows Runner Recursive System-Image Closure V1 权威草案

对应验收见 [Recursive System-Image Closure acceptance](user-node-windows-runner-recursive-system-image-closure-acceptance.md)。
上游 package-only 请求与 GrantReady prefix 见
[Launch Context Selection authority](user-node-windows-runner-launch-context-selection-authority.md)，最终 load-set owner 边界见
[Loader Load-Set authority](user-node-windows-runner-loader-load-set-authority.md)。

## 1. 本批结论

本批冻结并写入的是 **source-only final projection envelope**，不是可达的递归解析器、逐波 acquisition workflow 或运行时
Windows 证明。它解除旧 final validator 把 package-only preliminary/GrantReady 数量强行等同全部 final edge/name/system-image
数量的结构矛盾：wave 0 保持不可变 base prefix；真实 system image 第一次由 post-lease resolution 到达后，其 imports/forwarders
只能进入 stage-explicit recursive suffix，直到 empty frontier。

状态严格为 `source_written/source_review_only/implementation_uncompiled/implementation_unrun`、
`passed=0/failed=0`、Windows dynamic=`0`、`migration/table/writer=none/none/none`。本批没有运行编译、Cargo/Rust test、
source-contract test、migration、SQLite、网络、设备、Win32 fixture 或真实 Runner。`failed=0` 不表示通过。

## 2. Wave-zero prefix 与 final suffix

final module edge provenance 必须二选一：

- `BasePrelease`：逐项绑定 preliminary request ordinal、pre/post edge cross-binding ordinal 与 authenticated package-only
  locator；它只能占据 GrantReady 计划的连续 final prefix。
- `SystemPostLease`：逐项绑定 wave ordinal、source parsed-image ordinal、parse-receipt ordinal 与 post-lease locator；它只能占据
  recursive wave 的连续 suffix，不能冒充 preliminary observation。

GrantReady 只证明 wave-zero prefix。final validator 先重证该 prefix，再由
`SealedWindowsRecursiveResolutionClosure` 反向覆盖其余 parsed image、module request、searched name 与 filesystem system-image
owner。base count + 每 wave count 必须与 final 四张集合逐项闭合；任何 gap、重叠、越界或 future-owner 引用都失败关闭。

## 3. Parse receipt 与 exact producer owner

每个 recursive parsed image 必须有唯一 `WindowsPostLeaseSystemImageParseReceipt`，至少绑定：

- receipt/wave/parsed-image ordinal；
- `producer_module_request_ordinal`，即上一波第一次产生该 target 的最小 final module-request ordinal；
- exact target node 与四类 source owner之一；
- source-owner binding digest、immutable byte/section material identity、preliminary parser-policy digest、import-table digest；
- normal/delay/forwarder counts、same-owner parse receipt digest 与整份 receipt digest。

四类 owner 只有：

1. `PackageContentLease`：exact package file、FileId、sealed content、lease generation 与 immutable policy；
2. `AuthenticatedPreloadedModule`：exact preloaded ordinal、cache key、component/section identity 与 preload evidence；
3. `KnownDllSection`：exact authority record、Object Manager section、immutable image section、mapping receipt 与 namespace generation；
4. `ResolvedFilesystemSystemImage`：exact resolution request、candidate/session/request/nonce/response/receipt、parent-relative
   file/open/mapping custody、servicing generation、lease generation 与 immutable policy。

owner 不能只在整个 final graph 中找到“某个同 node 记录”。receipt 的 owner 必须与
`producer_module_request_ordinal` 指向的 package/system final binding、terminal route 和 target node 同时匹配；frontier 又要求该
producer 属于上一波 request range。不同 parser receipt 也不能绕过 immutable byte/section identity 去重复解析同一 material。

## 4. Frontier、wave 与 deterministic merge

wave ordinal 从 1 连续递增。每 wave 持有严格递增且只能使用一次的 source parse-receipt ordinals，以及连续的
module-request、searched-name、new filesystem-system-image request ranges。下一 frontier 必须从当前 wave 实际 final targets 反推：

- target 已在更早 wave 解析时不重复加入；
- target 第一次在当前 wave 到达时，其 receipt 必须恰好属于下一 wave；
- receipt 不得故意延迟到更晚 wave；
- 同一 range 内多次到达同一 target 时只接受最小 final module-request ordinal作为 producer；
- 最后一个 wave 的 next frontier 必须为空；有 recursive target却没有 wave，或有 receipt却没有实际 frontier producer，都失败关闭。

一 wave 内 final edges 按 `source_parse_receipt_ordinals` 顺序合并；同一 parsed importer 内再按
`importer_graph_edge_ordinal` 连续排序。normal imports、delay imports、forwarders 分域排序，direct import 用
descriptor/thunk ordinal，forwarder 用 root-import/hop ordinal，因此不能通过任意交错获得另一份 final graph。

## 5. Forwarder root、逐跳连续性与界限

post-lease direct import locator 的 `source_import_edge_ordinal` 等于该 edge 的 final global/module-request ordinal。
forwarder locator 中同名字段则引用 **root normal/delay final edge**，不是 importer-local ordinal；root 可以来自
`BasePrelease`、更早 recursive wave，或同一 wave 中按 canonical merge 排在该 forwarder之前的 direct edge。hop ordinal 是
该 root 的 base-plus-postlease 完整链上的零基序号；forwarder或未来 edge均不能成为 root。

validator 从 root target node + imported symbol 开始，逐跳要求：当前 importer/source-export 等于 root 或上一 hop 的
target node + target symbol；hop ordinal 无重复、无 gap；证据 digest 与 name-XOR-ordinal symbol 均合法；重复的
`(module node, symbol)` 构成 cycle并失败关闭。`max_forwarder_hop_count` 按最大 hop depth而不是 forwarder edge 总数执行。

## 6. 每波反向摘要与 final profile

每个 wave 的以下摘要都从 final sealed slice重算，不能接受孤立的 SHA-shaped 自报字段：

- exact package/system edge set，含 binding ordinal、stage locator、importer、symbol、cache key、完整 system
  `resolution_origin` 与 filesystem owner ref；
- exact searched-name disposition set；
- exact new filesystem system-image custody set。

parse receipt、wave 与 closure 自身再使用固定 JCS/SHA-256 material重算。private digest schema 本批更新为：PE import table V2、
PE final edge set V2、pre/post receipt/import-edge cross-binding V2、parsed-image set V2、loader resolution profile V3。
profile V3 必须纳入 recursive closure digest；process pre-create只能消费该 final profile，不能把 digest 当成 live OS observation。

recursive parser policy逐 receipt绑定既有 authenticated preliminary parser policy；limit source绑定 selected launch-context intent，
max wave/image/module/name/system-owner/forwarder-depth 数值封入 closure digest。未来仍需真实 policy producer证明这些 limit 值确实由
签名配置选择，不能由 final sealer任意决定。

## 7. 明确未冻结的逐波 custody

当前结构没有逐 wave 的 request → searched-name/launch-component grant → candidate → lease → same-handle parse 的可构造 typed
advancer，也没有 per-wave authenticated negative、partial acquisition、outcome-uncertain parking、release/recovery backend。
因此本合同只能称为 final projection envelope，不能称为完整 recursive acquisition authority。

以下 producer全部保持 `missing`：authenticated selector、prelease/recursive parser、GrantReady resolver、external-directory owners、
name/system positive-consuming transition、recursive advancer、post-lease sealer/query、reopen/currentness、live-OS、post-create
machine query、create/resume/recovery、pre-resume与 dynamic-load enforcement。`Infallible` producer使当前 closure不可构造。

当前 API-set只表达一步 contract → exact non-recursive preloaded/KnownDLL/filesystem/SxS host；recursive suffix可包含这种一步
terminal，但 host enum不能表达另一个 API-set contract，nested API-set DAG仍 fail-closed。`ShadowedByEarlierName` positive path仍
拒绝。runtime `LoadLibrary` 不在本合同内。

## 8. 零效果与 Ready 边界

四项 Ready gap逐字保持 `missing`：`node_local_authority_currentness`、`runtime_transition_authority`、
`host_runtime_authority`、`v15_authenticated_session`。

loader exact 18 effects逐字保持 `none`：`runtime_phase`、`runtime_generation`、`runtime_start`、`runtime_resume`、
`runtime_store`、`health`、`readiness`、`node`、`provider`、`route`、`offer`、`capacity`、`execution`、`attempt`、
`lease`、`usage`、`settlement`、`money`。本批不生成 Runtime、Ready、Provider、route、Offer、Attempt、Lease、
usage、settlement 或 money effect。

## 9. 后续顺序

1. 把 recursive limits加入 authenticated launch policy，并实现真实 retained-handle recursive parser；
2. 冻结并实现每 wave 的 grant/candidate/lease/negative/outcome-uncertain typed custody与 advancer；
3. 实现 final sealer/query、namespace/currentness/reopen 与 generation-drift fault matrix；
4. 单独设计 nested API-set DAG与 Shadow positive authority；
5. 再做 live OS、post-create machine、pre-resume/dynamic-load enforcement和受控 resume；
6. 只有动态验收完成后，才可讨论 Host runtime/Ready/v15与市场接线。
