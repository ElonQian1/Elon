---
title: UserNode Windows Runner Loader Load-Set Authority V1 权威草案
status: draft
reviewed_at: 2026-08-26
owners: node, compute, windows
proposed_feature_id: compute-user-node-windows-runner-loader-load-set-v1
registration_status: unregistered_feature_workflow_unavailable
design_status: draft_frozen
implementation_status: source_written_uncompiled
verification_status: source_review_only
---

# UserNode Windows Runner Loader Load-Set Authority V1 权威草案

对应验收见 [Loader Load-Set acceptance](user-node-windows-runner-loader-load-set-acceptance.md)。
exact selector/pre-lease/preliminary seam见
[Launch Context Selection authority](user-node-windows-runner-launch-context-selection-authority.md)。
post-lease递归边界见
[Recursive System-Image Closure authority](user-node-windows-runner-recursive-system-image-closure-authority.md)，逐波来源与失败保管见
[Recursive System-Image Acquisition Custody authority](user-node-windows-runner-recursive-system-image-acquisition-custody-authority.md)，
Ak forward plan见
[Recursive Wave Resolution Plan authority](user-node-windows-runner-recursive-wave-resolution-plan-authority.md)。

## 1. 本批结论

本批只冻结并写入 share-none admitted package → Windows loader load-set 的 **线性 owner graph、全量 consuming
seam、五类 failure-custody contract 与不可伪造 prerequisite**。它修正了旧进程监管草案中
`DurableWorkAdmittedPluginSlot + detached image` 的不可能所有权形状：Runner 本来就是 admission 内完整 package
file set 的一个 ordinal，成功结果只能是替换原 admission 的单一 successor owner。

当前 exact launch-context/pre-lease/preliminary unresolved request、GrantReady private plan/movable owners、post-lease
same-owner lineage 与 process pre-create projection typed seam 已写，但 authenticated selector/parser、GrantReady
terminal/disposition resolver与 external-directory owner producer仍缺；exact PE graph、
launch-path grants/authority、startup/import resolution、searched-name grants/query、FileId content leases、package-file
reopen/recovery 也都无 producer；其中 PE/launch-path/resolution authority
以 `Infallible` 保持 uninhabited。既有 extraction directory access/share compatibility 与 Runner/package-root/全部
plan-directory retained handle-chain discovery 已形成 purpose-specific source seam，但 Windows 动态矩阵仍为
`unverified`。discovery receipt 不是 exact CWD、grant 或 launch-path authority。因此
`LoaderLockedWorkAdmittedPluginSlot` 没有成功 producer，进程 backend 仍不可达。
system-image recursive imports 的 **final projection envelope** 已写入 stage-explicit base/suffix、producer-bound owner、
frontier/fixpoint、deterministic merge与 final slice摘要；authenticated recursive policy及逐 producer wave acquisition custody
source contract也已写；A0复用 GrantReady，Ak canonical request/resolution plan V1、exact-vector currentness-pending→DispatchReady与 acquisition
receipt/output V3与逐 A0/Ak currentness source shape也已写。但真实 policy signature verifier/currentness backend、recursive parser/resolver、grant/candidate/lease backend、positive advancer、
sealer/query仍缺，因此不是可达的 acquisition authority或 runtime closure。
本批只定义指定 launch context 下的 startup/import resolution material；pre-create 观测最多保护 path-based
process open，普通 imports 多在 suspended primary thread 后续运行时解析。因此 pre-resume loader-currentness 与
运行期 `LoadLibrary` enforcement 都仍是 resume blocker，不能声称已封存 eventual imports、Host runtime 或 Runner start。

状态固定为：

- design: `draft_frozen`；
- implementation: `source_written/source_review_only/implementation_uncompiled`；
- runtime: `implementation_unrun`；
- code acceptance: `passed=0/failed=0`；
- persistence: `migration/table/writer=none/none/none`；
- feature registration: `unregistered_feature_workflow_unavailable`。

## 2. 修正后的 successor owner graph

冻结的完整图为：

`DurableWorkAdmittedPluginSlot<'root>`
→ borrow-only receipt/evidence preflight
→ 从 retained handles 发现 Runner/package-root/全部 plan-directory launch-path candidates，并预租约解析 PE material
→ authenticated launch-context 选择 exact CWD 并形成 preliminary unresolved request plan
→ 补齐 wave-zero exact terminal、每步 disposition、external directory owners与 resolved-system canonical dedupe，形成 grant-ready owner
→ borrowed whole-owner validation + exact Control-ring/trusted-time currentness query，形成 policy-current GrantReady owner
→ 取得 base searched/launch-path grants、全部 package leases及 base requests命中的 route-specific system owners
→ 在 package leases 下做 same-handle full-package rehash/re-parse与 pre/post cross-binding
→ 对 producer wave `k`：same-owner parse本波 source images → canonical outgoing requests/terminal/dispositions → same-session grants
  → route-specific owner/candidate/lease acquisition；next frontier非空时形成 target parse wave `k+1`，空时 terminal target=None
→ empty frontier后形成 final aggregate并封印 exact PE graph、launch path、recursive closure与 startup/import resolution，形成
  `PostLeaseSealedWindowsRunnerLoadSetPreQueryPrerequisite`
→ consuming query同时验证完整 base+recursive name-grant与 content-lease generation set
→ purpose-specific by-value destructure
→ validate 并包装原 package-root/plan-directory handles，不关闭或重开目录
→ 只对 package files 进入 close/reopen barrier；每个 package-file lease 线性移入 identity anchor，system-image leases
留在 resolution owner；reopen 消费 anchor 产生 `ManagedLoaderFileReopenReceipt`，再做 identity/hash/handle-path 与 final fence query
→ `LoaderLockedWorkAdmittedPluginSlot<'root>`。

successor 只由两部分组成：

1. `LoaderTransitionAuthorityCustody<'root>`：保留 work-admission/promotion receipts、两层 trusted-time observation 与
   revalidation barrier、health/staging receipts、staging recovery key、extraction plan/evidence、verified raw artifacts、
   staging root、exact 自有 `ComputePluginRootLockLease`、share-none staging seal及其 evidence、completion time；
2. `SealedComputePluginRunnerImage`：按 extraction-plan ordinal 持有完整 package loader-file custody；每个 file 继续持有
   exact FileId content lease 与 anchor-consuming reopen receipt；resolution authority 同时保留每个普通 filesystem
   system image 的 servicing-generation-bound immutable content lease、parent-relative file/open/section receipts。package 与
   system leases 合成统一 lease-set，并与 Runner ordinal、原 package-root/plan-directory wrappers、working-directory
   location 及 resolution/namespace authority 共属一个 owner graph；
   `startup_import_resolution_profile_digest` 和 `startup_import_namespace_authority_digest` 只能从该 prerequisite 派生。

它不是“原 admission 加一份 image”。package files 从旧 owner 移入 image，其余 authority 移入 successor authority；
两者始终同属一个 `LoaderLockedWorkAdmittedPluginSlot<'root>`。后续进程监管只能按值消费这个 successor 和另行产生的
launch security，不能再同时接收原 admission、caller path、digest 或 caller-opened handle。
successor 按值拥有 exact root 自有 `ComputePluginRootLockLease`；该 lease 随全部 failure/success/process custody，外部
root borrow 生命周期结束不会释放它，unconfirmed/leaked graph 也不得因此解锁。

## 3. Purpose-specific consuming seams

现有短 consuming API 会丢失 trusted-time/barrier，cleanup projection 也会把运行 authority 改写成清理 parts。为此源码只
增加 loader transition 专用 seam：

- `DurableWorkAdmittedPluginSlot::into_loader_transition_parts` 保留 revalidated admission 与 receipt pair；
- `RevalidatedInstalledWorkAdmission::into_loader_transition_parts` 保留 installed slot、trusted time 与 revalidated time；
- `RevalidatedCandidatePromotion::into_loader_transition_parts` 保留 health publication、trusted time 与 revalidated time；
- `ExtractedComputePluginCandidateArchive::into_loader_transition_parts` 保留原目录/文件 vectors、plan、evidence、verified
  artifacts、staging、seal、seal evidence 与 completion time；后续 validation 必须先核对 exact 长度，再按 ordinal/path
  绑定，不能以截断式 zip 冒充完整 package；
- `PreparedComputePluginCandidateStaging::into_loader_transition_parts` 保留 staging root borrow、relative root/run digest 与
  原 package-root directory handle；
- 私有 `destructure_query_verified_owners` 同时消费 admission 与 query-verified prerequisite，一次性移动
  中间各层 receipt/recovery material；无实现的 `WindowsRunnerLoaderOwnerGraphIndexer` 还必须把每个
  package file 与其 exact ordinal/path/FileId content lease 合并为单个 element，不允许平行 vector
  截断、clone 或丢弃。

这些 seam 不是通用 getter，不返回 raw handle、path、receipt scalar 或 retry permit；源码也不借用 cleanup seam 重建
authority。当前函数名表达调用顺序约束，但没有 caller，也没有实施 preflight/fence/backend；它不能单独证明这些步骤已经
发生。

## 4. 不可构造的 resolution/name/content authority

关闭任何 share-none package file 前，必须按 borrow-only discovery/pre-lease material → authenticated exact CWD
selection/preliminary unresolved request plan → wave-zero exact terminal/disposition/external owner grant-ready resolution →
whole GrantReady borrowed validation + exact Control-ring/trusted-time currentness query → PolicyCurrent GrantReady →
base name/launch-component grants + package leases + base route-specific system owners → package same-handle rehash/re-parse →
producer wave parse/canonical request-resolution plan → exact-vector validation → currentness-pending → point-of-use authorization →
DispatchReady → grant/candidate/lease acquisition → empty frontier → final aggregate/final resolution seal →
consuming generation query-back 的权威顺序完成。当前源码已删除 grants/leases 前持有 final resolution 的旧 prerequisite。
`PreliminaryResolutionRequestsPlannedWork` 仅持有未解析 request skeleton；future
`GrantReadyWindowsRunnerResolutionPrerequisite` 合同必须消费 whole request owner并绑定 exact terminal、每步 disposition、
external owners与 resolved-system request set；其 source shape已写，真实 producer仍由 `Infallible` 阻断，且
`ShadowedByEarlierName` variant当前在 GrantReady/final projection validator中显式拒绝。name-grant failure持有
`PolicyCurrentGrantReadyWindowsRunnerResolutionPrerequisite`；content-lease failure持有 outer
`PolicyCurrentPreFinalWindowsLoaderNamespaceGrantSet`，policy/authorization保持在 policy-free inner namespace之外，并保留统一
package/system acquired lease、active dispatch与 pending refs；borrow-only failure
持有 whole request owner；namespace-query failure持有 post-lease whole prerequisite。只有 base与全部 producer-wave
acquisition完整、same-owner parse闭合且 frontier为空后，final sealer才允许形成
`PostLeaseSealedWindowsRunnerLoadSetPreQueryPrerequisite`，其 producer仍不可构造。不得预测 lease generation，
或让 discovery/request receipt冒充 grant-ready/final authority。

### 4.1 Exact startup/import resolution authority

未来 resolution producer 至少必须绑定：

- exact work-admission source、extraction plan、Runner 与 working directory；
- exact launch token/AppContainer profile、target architecture/WOW64 mode、显式空 environment 及 loader search
  policy、cwd 与 process creation flags；
- normal imports、delay imports 与实际被引用的 forwarders；每条边必须绑定 import symbol name/ordinal、descriptor/thunk
  ordinal；forwarder 还必须绑定 source export、target DLL/symbol、逐跳 evidence 与 cycle/depth receipt；
- 在校验任何 PE edge 前，以 Runner basename 和 authenticated process preloaded/bootstrap module set 预种 module-cache
  closure；每项预种绑定 expected process-machine context、cache key、immutable section 与 authenticated evidence；
- pre-lease只计算 Runner-rooted package-image/importer/forwarder-source closure，并把非 package target变成 typed external
  leaf；normal/delay base imports与 separate forwarder hops形成 immutable wave-zero prefix。独立 post-lease contract已冻结
  system-image parsed receipt、earliest producer owner、连续 ranges、frontier/fixpoint、deterministic edge merge、global-root
  forwarder chain与 final edge/name/system-owner反向摘要。独立 source contract还冻结 signed policy、nonempty producer→target /
  terminal None、A0 GrantReady复用、Ak canonical request/resolution plan V1、exact-vector limit派生、currentness-pending→DispatchReady、
  whole-owner partial custody及 V3 acquisition receipt/output→projection cross-binding；其真实 policy/parser/resolver/backend/
  positive advancer与 sealer producer仍不可构造；
- package module name → exact package-file ordinal/digest/FileId；
- pre-lease package PE material 绑定 FileId + sealed digest + parser policy，但不得包含或预测 content-lease generation；
  final sealer 必须在 leases 下从同一 handles 重哈希/重解析，并把真实 lease generation 加入 canonical composite。
  parser evidence/edges 禁止跨代 splice；
- final resolution profile V3 必须绑定 `launch_context_selector_digest`、
  `preliminary_resolution_request_plan_digest`、`grant_ready_resolution_plan_digest` 与
  `recursive_resolution_closure_digest`，不得包含未来
  `required_launch_context_digest`；process required-launch-context V3 才同时绑定 selector 与 final resolution profile；其
  expected digest由 resolution外 launch-security bridge持有并比较，摘要链不得要求 fixed point；
- allowed system dependency → exact resolved component identity；普通 filesystem system image 还必须持有
  servicing-generation-bound immutable content lease、parent-relative retained file custody、authenticated open receipt 与
  immutable image-section receipt；
- package、working 与 plan directory 的 exact ordinal、handle identity 与 policy source；external
  System/Windows/SxS search directory 还必须保留 full handle-derived ancestor chain、每层 parent share/grant contract
  与 namespace-alias currentness receipt，并把整组证据纳入 `startup_import_resolution_profile_digest`；
- OS-build-bound KnownDLL/Object Manager section generation，以及每个 KnownDLL named section → exact immutable image
  section 的映射；
- 当前 V1 API-set contract只允许一步映射到 exact non-recursive authenticated preloaded、KnownDLL、ordinary filesystem
  search 或 SxS terminal；nested API-set必须 fail-closed，未来递归解析需独立 typed DAG contract；
- 每个 SxS binding 必须同时保留 activation-context receipt、exact search directory、resolved FileId 与 immutable
  image-section receipt；
- 启动/import resolution 中每个 search directory 的 terminal或 `MustRemainAbsent` name；Shadow variant当前仅占位且 validator
  显式拒绝，不能声称 shadow grant path已冻结或可消费。

当前 exact `SealedWindowsPeImportGraphAuthority` 和 `SealedWindowsLoaderLaunchPathAuthority` 分别以
authenticated-parser 与 parent-chain grant/share `Infallible` 字段保持 uninhabited；整个
`SealedWindowsLoaderResolutionAuthority` 也无 producer。source-written handle-chain candidate discovery 只证明 typed
owner 上的候选观测形状；新 exact-context/pre-lease/unresolved-request/GrantReady/post-lease lineage types已有 source shape，
  recursive final envelope与逐波 custody contract也已有 source shape，但 selector/policy signature verifier/currentness backend、prelease-or-recursive
  parser/resolver/acquisition backend/positive advancer/sealer producer仍 `missing`。完整 package file set只证明
archive coverage，上述 source shapes都 **不证明** runtime selected CWD、component grants、preloaded/bootstrap cache
seed、exact PE graph、launch path 或 startup/import resolution。

### 4.2 Whole searched-name namespace authority

未来 namespace producer必须在同一 kernel session/generation内逐 producer wave取得 exact grants，并在 empty frontier后的 final
aggregate/seal原子覆盖 startup/import policy的完整 base+recursive searched-name和 base launch-path component set，包括预期存在
文件与预期缺失 name；不得把“原子覆盖”解释为递归前预取尚未知的 names。Shadow variant的 typed evidence/producer仍
`missing`。每个 grant必须与 exact session、parent/name、
disposition 和 generation 绑定。present searched-name disposition digest 必须直接绑定 resolved package/system exact
FileId；system target 还必须绑定 immutable section identity 与 servicing generation。实际 package+system lease owners
逐波把 generation 交给 final aggregate/sealer，随后 consuming query必须再次确认同一 generation set。普通 directory
handle/share mode、digest 或 oplock 不能代替。

当前 searched-name grant acquisition/fence/query backend 均不存在。name-grant positive shape虽保留 exact
request/nonce/response bytes+digest/self-binding，消费 exact attempt形成正向 owner的 transition仍以 `Infallible` 阻断。成功
owner只能暴露从该 authority 派生的 `startup_import_namespace_authority_digest`，该 digest 本身不是 namespace authority。

### 4.3 Unified FileId immutable-content lease-set 与 live-OS currentness

name grants 后、final same-handle hash 前，每个 package file 必须取得 exact FileId/content-digest lease；每个普通
filesystem system image 必须取得 exact FileId/immutable-section、servicing-generation-bound content lease。两类 lease
合成统一 linear lease-set，在 startup/import mapping、全部 failure custody、loader success 与后续 process custody 全期
持有；lease 必须拒绝 writable open、write/delete disposition 与 writable section mapping。当前这些 lease 与
acquisition/negative custody 只有 source shape，backend 不存在。ordinary system-image positive custody虽已定义 unique owner与
exact response material，消费同一 attempt形成该 owner的 transition仍以 `Infallible` 阻断。
recursive envelope当前只从 final custody slice重算每波 new owner set；相邻 source contract现已补充 producer wave parse→outgoing
request→grant→route-specific candidate/lease→next frontier custody及 authenticated-negative/partial-acquisition quarantine，但真实
backend、positive advancer与 release/recovery producer均不存在。

resolution 中的 KnownDLL/API-set/SxS/immutable system-image字段与 machine/WOW64只是 sealed expected material。pre-create
projection/query也只回显这些字段，不是 live OS KnownDLL/Object Manager、API-set schema/host 或 SxS activation-context
观测。`live_windows_resolution_currentness_backend=missing`，post-create `IsWow64Process2`/equivalent machine query-back
receipt也 `missing`，两者仍是 resume blocker。

## 5. Barrier 与失败 custody

不可逆 file barrier 是第一个 admission-owned share-none package-file handle 被关闭的时刻；但 name-grant、
content-lease 与 namespace query dispatch 在此之前已可销毁 retry authority。五类 failure custody 固定为：

1. `NameGrantAcquisitionUnusable`：只有 authenticated-negative receipt 与 exact namespace session、request digest
   及 nonce 匹配时才是 `AuthenticatedRejected`；否则必须是 `OutcomeUncertain`。custody 保留
   完整 preliminary owner、session、已取得 grants、active attempt、negative receipt 与 pending ordinals；若 backend
   同时返回 positive 与 negative，绝不能标为 definitive，positive receipt 必须留在 outcome-uncertain custody；
2. `ContentLeaseAcquisitionUnusable`：只有 FileId/content-digest authenticated-negative receipt 与 exact
   acquisition attempt 匹配时才是 `DefinitiveRejected`；否则为 `OutcomeUncertain`。custody 保留
   完整 preliminary owner、namespace grants、已取得统一 package+system leases、active attempt/negative 及 pending
   ordinals；同时返回 positive 与 negative 时不得 definitive，positive receipt 必须保留；
3. `BorrowOnlyNotTransitioned`：只用于任何 grant/lease dispatch 之前的纯 borrow-only validation，保留
   exact whole preliminary owner；
4. `NamespaceQueryUnusable`：`DefinitiveRejected` 仅在 authenticated-negative 属于 exact retained
   session/request/nonce 时成立；缺失或错绑的 negative 必须降为 `OutcomeUncertain`。admission、
   unqueried prerequisite、query attempt 和 optional negative 全部 quarantine；initial query 返回 positive-but-invalid
   receipt 时也必须归入 `OutcomeUncertain` 并保留该 returned receipt；initial query 同时返回 positive 与 negative
   时也不得 definitive，positive receipt 进入相同 custody；
5. `PostBarrierOutcomeUncertain`：query 成功并跨过首个 close 后的任何 failure 都只返回 indexed
   anchors/pending/transitioned files、close-uncertain source、rejected replacement、final-query attempt、retained
   directories、transition schedule 与 Runner ordinal，不返 admission/retry extractor。final-fence query 只有 exact
   session/request/nonce authenticated negative 且没有同时 positive receipt 时，才能把内部 class 标为 definitive；
   positive-but-invalid 或与 negative 同时返回的 positive receipt 必须保留在 outcome-uncertain final-query custody，
   外层仍保持 post-barrier quarantine。

package-file content lease 必须从 unified indexed lease set 线性移入对应 `ManagedLoaderFileIdentityAnchor`；system-image
lease 保留在 resolution owner 中。parent-relative reopen
只有在消费该 anchor 与 replacement handle 后才能产生 `ManagedLoaderFileReopenReceipt`；receipt 绑定
source/replacement volume/FileId/type/reparse/link/size、parent-relative binding、close/open receipts、access profile、hash/path
与 content-lease continuity，并与 lease 一起进入 `PinnedManagedLoaderFile`。post-close reopen dispatch 只能从该
anchor 构造 attempt custody；replacement handle 是 backend outcome，不是 attempt input。不得再接收 detached
path/FileId/lease scalar，失败也必须保留 anchor-only attempt custody。
reopen backend 同时返回 positive 与 negative 时也不得 definitive；positive reopen receipt/owner 必须留在该
outcome-uncertain anchor custody。
close ambiguity 仍使用
`ManuallyDrop<PinnedManagedFile>` quarantine，禁止 ordinary `File` destructor。

只有全部 package+system leases、package files、retained directory bindings、reopen receipts、identity/hash/path 与 final fence query 成功，
才能形成单一 successor。源码当前只写入 owner/failure shapes；grant/lease/query/reopen/release/recovery
backend 均不存在，五类 contract 不是已通过 fault injection 的运行事实。
凡 typed custody声称 authenticated response，都必须按值保留 exact response bytes与 response digest，并在
classification前重算。当前明确写入的是 name-grant positive与 resolved-system-image positive shape；其 consuming
transition/backend仍 uninhabited，不能外推为所有 lease/query/reopen positive path均已有合同或真实回执。

## 6. Candidate package-file reopen 与 retained-directory 边界

候选顺序固定为：borrow-only receipt/evidence preflight → retained handle-chain launch-path discovery + pre-lease PE material →
authenticated exact CWD/preliminary unresolved request → wave-zero exact terminal/disposition/external-owner grant-ready resolution →
base grants + package leases + base route-specific system owners → package same-handle rehash/re-parse → producer-wave recursive
parse/outgoing request/grant/candidate/lease advancement → empty frontier → final aggregate + PE/launch/resolution seal → consuming
name-grant/content-lease-generation query → exact file+lease owner-graph indexing → retain/wrap 原 package-root 与 plan-directory handles → 只对 package
files parent-relative close/reopen（Runner 最后）→ anchor-consuming reopen receipt → replacement identity/hash/handle-path →
ordered final name/content query。

package files 与 extraction-directory compatibility probe 的候选 access/share shape 为：

| Object | Desired access | Share access | Create options |
|---|---|---|---|
| package executable / DLL | `FILE_GENERIC_READ | FILE_GENERIC_EXECUTE` | `FILE_SHARE_READ` | `FILE_NON_DIRECTORY_FILE | FILE_OPEN_REPARSE_POINT | FILE_SYNCHRONOUS_IO_NONALERT` |
| package read-only asset | `FILE_GENERIC_READ` | `FILE_SHARE_READ` | `FILE_NON_DIRECTORY_FILE | FILE_OPEN_REPARSE_POINT | FILE_SYNCHRONOUS_IO_NONALERT` |
| extraction package-root / descendant directory probe | `FILE_READ_ATTRIBUTES | FILE_TRAVERSE | SYNCHRONIZE` | `FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE` | `FILE_DIRECTORY_FILE | FILE_OPEN_REPARSE_POINT | FILE_SYNCHRONOUS_IO_NONALERT` |

两类 create disposition 都固定候选为 `FILE_OPEN`。新增目录 recipe 只证明原 `DELETE` owner 与 parent-relative
share-delete probe 可同时保留，并以同 volume/FileId、非 reparse directory 和 handle-derived canonical path 绑定；它不
把 probe 替换成 loader authority，也不放宽普通 managed-directory open。staging descendant 必须从这份 retained
package-root 或 plan 中已保留的 exact parent owner 单 component create-new，不重开 existing descendant；清理与 loader
分支再线性消费同一 typed receipt。外部 System/Windows/SxS search
directory 仍属于 future resolution producer 的独立 custody。当前 blocker 精确为
`source_seam_written_windows_dynamic_unverified`，不得声称 admission→loader predecessor 已动态可达。

package-file recipe 也只是未编译/未运行的 source candidate。`FILE_SHARE_READ` 不会阻止 writable section
mapping，不能替代 exact FileId content lease；reopen 后的 hash/name 观测也不能替代 lease continuity、
startup/import resolution、searched-name authority 或 dynamic module-load enforcement。

## 7. 四项 authority gap 与零效果

本批保持以下四项为 `missing`：

- `node_local_authority_currentness`；
- `runtime_transition_authority`；
- `host_runtime_authority`；
- `v15_authenticated_session`。

源码把以下 exact 18 项 effect 固定为 `none`：`runtime_phase`、`runtime_generation`、`runtime_start`、
`runtime_resume`、`runtime_store`、`health`、`readiness`、`node`、`provider`、`route`、`offer`、`capacity`、
`execution`、`attempt`、`lease`、`usage`、`settlement`、`money`。

本批不修改 lifecycle transition、local-authority schema、migration、writer、Ready builder、v14/v15、HTTP/MCP/Wire 或
控制 WebSocket，也没有 Provider/market/money effect。

`PRE_RESUME_LOADER_CURRENTNESS=missing_resume_blocker`：pre-create 查询不封存 suspended child 开始运行时才
发生的 eventual import resolution。`DYNAMIC_MODULE_LOAD_AUTHORITY=missing_resume_blocker`：sealed set 不覆盖运行期
`LoadLibrary`/等价模块加载。`LIVE_WINDOWS_RESOLUTION_CURRENTNESS=missing_resume_blocker` 说明 sealed
KnownDLL/API-set/SxS material 不是 live OS 观测。name grants 还需未实现的 explicit authorized release 与
crash/recovery owner；不得以 Drop 或 session disconnect 当作 release。这些都必须阻止 resume。

## 8. 后续顺序

源码铺设顺序允许继续冻结下一合同；生产可达顺序固定为：

1. 动态验证 source-written extraction-share 与 retained handle-chain discovery；验证前不得假定 predecessor 可达；
2. 实现并动态验收 authenticated exact-context selector、pre-lease PE parser 与 preliminary request producer；
3. 实现 exact terminal/disposition、external-directory owner与 resolved-system dedupe resolver，形成 grant-ready owner；
4. 实现 base searched-name/launch-path grant acquisition、package lease与 base route-specific system-owner backends，对三种
   authenticated-negative 精确绑定 owner/attempt/session/request/nonce；
5. 接入 authenticated recursive-policy signature verifier/currentness backend、recursive retained-handle parser与 per-wave resolver，使 canonical
   plans完成 currentness-pending与 point-of-use authorization后推进为真实 DispatchReady owner，再按已冻结合同接入每 producer wave grant/candidate/lease/negative/
   outcome-uncertain backend与 positive advancer；
6. 在 leases 下 same-handle 重哈希/重解析，封印 exact PE graph、launch path、recursive fixpoint与 startup/import/system resolution；
7. 实现 consuming namespace/content-generation query、anchor-consuming reopen receipt、final query 与全五类 recovery；
8. 实现 persistent-grant explicit authorized release/crash recovery 与 live Windows KnownDLL/API-set/SxS currentness；
9. 执行 Windows share/TOCTOU/content-mapping/rename/swap/reparse/hardlink/delete-pending/startup-import matrix；
10. 后续另行实现 launch security、pre-create/pre-resume loader-currentness、dynamic `LoadLibrary` enforcement、IPC/Store、
   controlled resume、health/Ready/v15。

任何后续步骤都不能从本批 source types 或 digest 字段推导“loader load-set 已锁定”。
