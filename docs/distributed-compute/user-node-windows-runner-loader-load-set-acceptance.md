---
title: UserNode Windows Runner Loader Load-Set Authority V1 验收草案
status: draft
reviewed_at: 2026-08-26
owners: node, compute, windows
proposed_feature_id: compute-user-node-windows-runner-loader-load-set-v1
registration_status: unregistered_feature_workflow_unavailable
design_status: draft_frozen
implementation_status: source_written_uncompiled
verification_status: source_review_only
---

# UserNode Windows Runner Loader Load-Set Authority V1 验收草案

权威合同见 [Loader Load-Set authority](user-node-windows-runner-loader-load-set-authority.md)。

## 1. 本批证据等级

本批只有 `source_written/source_review_only` 证据。架构铺设阶段不编译、不运行 Rust test、不执行 migration/SQLite，也不进行
Windows、网络、设备或真实 Runner 验证：

- implementation: `source_written/source_review_only/implementation_uncompiled`；
- runtime: `implementation_unrun`；
- code acceptance: `passed=0/failed=0`；
- persistence: `migration/table/writer=none/none/none`；
- dynamic Windows evidence: `0`；
- successful transition/backend/producer: `none/none/none`。

新增 source-contract guard 也未执行。格式化、文本、diff、体积和模块化检查仅属于交付卫生，不提高运行成熟度；
`failed=0` 只表示没有执行失败项，不表示通过。

## 2. 文件责任

| Owner | 文件 | 责任 |
|---|---|---|
| private facade | `runtime_loader_load_set.rs` | 路由 model/failure/policy/resolution/transition/validation；声明无 producer 与成功 transition |
| launch-path discovery | `node_agent_managed_fs/{loader_launch_path_discovery,windows_loader_launch_path_discovery}.rs`、`runtime_loader_load_set/launch_path_discovery.rs` | typed retained-owner handle-chain candidate observations；不选择 CWD、不授予 grant |
| exact-context/preliminary | `runtime_loader_load_set/launch_path_discovery/{exact_context_plan,prelease_pe_material}{,/digest,/closure}.rs` | uninhabited selector/parser、package-only pre-lease PE与 unresolved request plan；不是 GrantReady/final authority |
| GrantReady + lineage | `runtime_loader_load_set/resolution/grant_ready{,/validation,/search_projection,/final_projection}.rs`、`exact_context_plan/lineage.rs` | private exact terminal/disposition/movable-owner、pre/post cross-binding、QueryVerified lineage与 pre-create projection source shapes；无 producer |
| resolution owners | `runtime_loader_load_set/resolution.rs` | uninhabited PE graph/launch path/resolution、name grants、package/system FileId leases 与 pre/post-query authority |
| successor model | `runtime_loader_load_set/model.rs` | 自有 exact root-lock 的单一 successor、全 authority residue、持有 lease/reopen receipt 的 indexed package files |
| failure custody | `runtime_loader_load_set/failure.rs` | name-grant、content-lease、borrow-only、namespace-query 与 indexed post-barrier 五类 custody |
| frozen policy | `runtime_loader_load_set/policy.rs` | package-file candidate recipe、retained-directory 边界、transition order、四项 missing gap 与全部 zero effects |
| consuming graph | `runtime_loader_load_set/transition.rs` | by-value destructure 与无实现 file+lease owner-graph indexer；无 query/reopen/success backend |
| cardinality/ordinal review | `runtime_loader_load_set/validation.rs` | exact vector/ordinal/path/FileId/lease/reopen-receipt 绑定；不截断平行 vectors |
| sealed-binding review | `runtime_loader_load_set/digest.rs`、`namespace_validation.rs`、`system_resolution_validation.rs`、`launch_path_validation.rs` | aggregate digests、response bytes/digest、exact query generations、system dependency 与 handle-derived launch-path bindings；都不是 producer |
| managed loader shapes | `node_agent_managed_fs/loader.rs`、`loader/{name_grant_positive,system_image_custody}.rs` | namespace session/grants、明确的 name/system positive response bytes+digest、FileId lease custody、anchor-consuming reopen receipt 与 no-Drop quarantine；consuming producers缺失 |
| nested seams | `work_admission_contract/capability.rs`、`candidate_promotion_contract/capability.rs`、`candidate_extraction/zip/types.rs` | 不丢 receipt/time/barrier/evidence/handle 的 purpose-specific consuming parts |
| process successor bridge | `runtime_process_custody/model.rs`、`namespace_query.rs`、`windows.rs` | process preparation 只消费 loader-locked successor；pre-create currentness backend 形状已定义但无实现 |
| source review | `runtime_loader_load_set_source_contract_tests.rs` | 未运行 guard，固定 owner graph、uninhabited prerequisites、failure split、zero-effect 负边界 |

## 3. 静态源码审阅目标

源码应满足：

1. `LoaderLockedWorkAdmittedPluginSlot<'root>` 是 `authority + image` 的单一 successor，不含 original admission + detached
   image 双重所有权；
2. package files 只由 image 按 exact ordinal 持有，Runner 是其中一个 ordinal；原 package-root/plan-directory
   handles retained/wrapped，只有 package files close/reopen；
3. 每个 successful package file 持有 exact FileId/content-digest lease 和 anchor-consuming reopen receipt；每个普通
   filesystem system image 持有 servicing-generation-bound exact FileId/immutable-section lease 与 parent-relative
   file/open/section receipts。package+system leases 合成统一 linear lease-set，穿越全部 failure/success/process custody；
4. authority residue 保留 work/promotion receipts、trusted-time/barriers、health/staging receipts、recovery key、
   plan/evidence/artifacts、staging、seal、completion time 与 exact 自有 `ComputePluginRootLockLease`；该 lease 随全部
   failure/success/process custody，外部 root borrow 结束或 graph leaked/unconfirmed 都不得解锁；
5. purpose-specific seams 按值移动全部 authority，exact 验证 file+lease vector/cardinality/ordinal/path/FileId，不借用
   cleanup projection、截断式 zip、clone 或 scalar reconstruction；
6. source-written launch-path discovery 只从 retained Runner/package-root/全部 plan-directory handles 生成候选观测，
   success/failure 均返回原 admission owner；它不含 selected CWD 或 grant。`SealedWindowsPeImportGraphAuthority`、
   `SealedWindowsLoaderLaunchPathAuthority` 与整个 resolution authority 继续含 `Infallible`，exact PE graph/launch-path
   producer 不存在；PE validation 必须先以 Runner basename 与
   authenticated process preloaded/bootstrap module set 预种 cache，逐项绑定 expected process-machine context/cache key/
   immutable section/evidence，再形成 Runner-rooted package-image closure claim与 typed external leaves；normal/delay base
   imports必须绑定 symbol name/ordinal、descriptor/thunk ordinal，forwarder作为 separate source-edge hop绑定 source export/
   target symbol/逐跳 evidence/cycle-depth receipt并执行已冻结 canonical rule。pre-lease validator不证明 true transitive system
   closure；post-lease sealer仍须闭合 system imports/cache collisions。pre-lease parser material不得预测
   lease generation，final sealer 必须在 lease 下 same-handle rehash/reparse 后加入真实 generation，禁止跨代 splice；
7. resolution material 覆盖 normal/delay/forwarder、package/system modules、KnownDLL/API-set/SxS/system images、
   searched names 及 exact token/AppContainer、architecture/WOW64、environment/search policy、cwd/flags launch context；
   KnownDLL named section必须映射到 exact immutable image section；当前 API-set contract只允许一步映射到 exact
   non-recursive authenticated preloaded、KnownDLL、普通 filesystem search 或 SxS terminal，nested API-set fail-closed；SxS必须绑定 activation-context receipt、
   exact search directory、FileId 与 immutable section；普通 system image 必须保留 servicing-generation-bound immutable
   lease 及 parent-relative file/open/section receipts；external System/Windows/SxS search directory 必须保留 full
   handle-derived ancestor chain、parent share/grant contract 与 namespace-alias currentness receipt，并纳入
   `startup_import_resolution_profile_digest`；
8. sealed KnownDLL/API-set/SxS 字段只是 echoes，无 live OS resolution-currentness backend；pre-resume currentness
   与 runtime `LoadLibrary` enforcement 均为 resume blocker；
9. exact searched-name/launch-path grants合同绑定 session、parent/name、disposition和 generation；terminal/
   `MustRemainAbsent` shape已写，`ShadowedByEarlierName`仅占位且当前 validator显式拒绝。present disposition digest直接绑定 package/system exact FileId，system还绑定 immutable section与 servicing
   generation；实际 package+system lease owners 先为 post-lease final sealer 提供 generation，随后 consuming
   query/aggregate 再确认同一 set；统一 FileId lease-set
   拒绝 writable open/write/delete/writable-section mapping；两类 backend 均无 producer；
10. name-grant `AuthenticatedRejected`、content-lease `DefinitiveRejected` 和 namespace-query `DefinitiveRejected`
    都必须持有与 exact owner/session/attempt/request/nonce 匹配的 authenticated-negative receipt；错绑或缺失
    必须归类 `OutcomeUncertain`；initial/final namespace query 的 positive-but-invalid returned receipt 也必须进入
    `OutcomeUncertain` custody 并按值保留；grant/lease/query 同时返回 positive+negative 时一律不得 definitive，
    positive receipt 必须进入相应 outcome-uncertain custody；凡 typed custody声称 authenticated response都保留 exact bytes+
    digest并在 classification前重算。当前明确的 positive shapes仅为 name-grant与system-image；consuming producers保持 uninhabited；
11. 只有所有 grant/lease dispatch 前的纯 borrow failure 返回 `BorrowOnlyNotTransitioned`；其余 pre-barrier
    failures 都保留已取得 owners、active attempt、negative 与 pending ordinals，无 retry extractor；
12. 首个 file close 后只返回 indexed `PostBarrierOutcomeUncertain`，保留 anchors/files/replacements/
    final-query attempt/optional exact negative/positive-but-invalid returned receipt/directories/schedule/Runner ordinal 与
    每项 path；即使 final-query negative authenticated，外层也不降格为可重试 failure；
13. reopen 必须消费 exact anchor + replacement handle 产生 `ManagedLoaderFileReopenReceipt`；close ambiguity
    以 `ManuallyDrop<PinnedManagedFile>` 禁止 ordinary `File` Drop；post-close reopen attempt 只能由 anchor 构造，
    replacement 只能作为 backend outcome 返回，failure 保留 anchor-only attempt custody；同时 positive+negative 时不得
    definitive，positive reopen receipt/owner 及其 response bytes/digest 留在 outcome-uncertain custody；
14. managed loader owners 不暴露 raw handle/path constructor、clone 或 Serde 逃逸；
15. package files 与 purpose-specific extraction-directory probe 都有 candidate Windows access/share recipe，均只
    `FILE_OPEN`；目录 probe 保留原 DELETE owner、要求 share-read/write/delete 且不替代 namespace/content authority；
16. transition 权威顺序要求 borrow-only launch-path discovery/pre-lease PE plan、authenticated exact CWD/unresolved request、
    exact terminal/disposition/external-owner grant-ready resolution、name/launch grant acquisition、统一 package+system FileId
    lease acquisition、lease 下 same-handle rehash/reparse 与
    final PE/launch/resolution seal、consuming generation query、exact indexing、
    retain directories、Runner-last file reopen、anchor receipt、identity/hash/path 与 final query；
17. `existing_extraction_directory_access_share_compatibility=source_seam_written_windows_dynamic_unverified`；typed
    retained-owner seam 已接入 extraction→loader owner graph，但没有 Windows 动态证据；
    `launch_path_handle_chain_discovery=source_written_windows_dynamic_unverified`；exact-context/pre-lease/unresolved request/
    GrantReady/post-lease lineage contracts为 `source_written_uncompiled_unrun`，但 selector/parser/GrantReady resolver、external-directory owner与 positive advancer producer以及
    `launch_path_component_grant_backend` 仍 `missing`；
18. success digest 不代替 resolution/namespace/lease authority；persistent grants 的 explicit authorized release/recovery 无
    producer，不得以 Drop/session disconnect 释放；
19. process custody 只接受 loader-locked successor；loader slice 内没有 `CreateProcessAsUserW`、`ResumeThread`、
    Store/Ready/market mutation；
20. 四项 authority gap 均为 `missing`，loader exact 18 项 policy effects 均为 `none`。

以上都只是未运行 Rust guard 与人工 source review 的目标，不能登记为 passed。

## 4. 明确未验收矩阵

| 轴 | passed | failed | unrun | 当前结论 |
|---|---:|---:|---:|---|
| Rust compile / Windows link | 0 | 0 | 1 | 未编译，type/borrow/Win32 constants 未由 compiler 证明 |
| source-contract Rust test | 0 | 0 | 1 | guard 已写但未运行 |
| launch-path retained-handle candidate discovery | 0 | 0 | 1 | source seam 已写；Windows access/FileId/type/reparse/canonical-chain/failure custody 未运行 |
| exact selector/pre-lease PE/preliminary unresolved request | 0 | 0 | 1 | typed source已写；selector/parser producer missing，未编译/未运行 |
| GrantReady terminal/disposition/external-owner contract | 0 | 0 | 1 | plan/validator/movable-owner source written；resolver/producer missing，Shadow rejected |
| exact PE graph / launch-path authority | 0 | 0 | 1 | authenticated parser 与 parent-chain grant/share authorities 均以 `Infallible` uninhabited |
| exact startup/import resolution producer | 0 | 0 | 1 | resolution 不可构造，无 imports/system identities 或 launch-context proof |
| searched-name / launch-path grants | 0 | 0 | 1 | 无 acquisition backend；terminal/absence未证明，Shadow path显式不可接受 |
| unified package+system FileId immutable-content leases | 0 | 0 | 1 | 无 backend；system servicing generation 与 writable open/disposition/section-map denial 未证明 |
| consuming namespace/content-generation query | 0 | 0 | 1 | 无 backend；exact attempt/receipt/session/current generation 未证明 |
| successful loader transition | 0 | 0 | 1 | 无 transition function、caller 或 success producer |
| successor-owned root-lock lease | 0 | 0 | 1 | ownership shape 已写；无 success/recovery producer，未验证 parked/leaked lifetime |
| anchor-consuming parent-relative reopen | 0 | 0 | 1 | anchor/reopen receipt/close quarantine 只有 source shape，无 backend |
| package-file candidate access/share recipe | 0 | 0 | 1 | 未在 Windows 验证兼容性或拒绝效果；不由目录 probe 推导 content lease |
| existing extraction directory access/share compatibility | 0 | 0 | 1 | retained DELETE owner + narrow share-delete probe 源码 seam 已写；Windows access/share、identity/path 与 failure custody 未运行 |
| full-package rehash / replacement rehash | 0 | 0 | 1 | 顺序已冻结，未实现/未执行 |
| FileId/reparse/hardlink/delete-pending/handle path | 0 | 0 | 1 | 未实现 query-back/fault injection |
| five failure-custody families | 0 | 0 | 1 | name-grant/content-lease/borrow/namespace/post-barrier shapes 存在；无运行 recovery |
| authenticated-negative ownership | 0 | 0 | 1 | 三种 definitive/authenticated rejection 均未运行 exact-owner binding matrix |
| authenticated response bytes/digest | 0 | 0 | 1 | name-grant/system-image positive shapes已写；consuming producers uninhabited，未运行重算矩阵 |
| post-create live machine/WOW64 query-back | 0 | 0 | 1 | pre-create expected projection only；`IsWow64Process2`/equivalent receipt/backend missing |
| live OS KnownDLL/API-set/SxS currentness | 0 | 0 | 1 | sealed/pre-create fields 只为 echoes；live backend 不存在 |
| explicit namespace release / recovery | 0 | 0 | 1 | persistent grants 无 authorized release/crash-recovery owner，resume blocked |
| pre-resume loader currentness | 0 | 0 | 1 | pre-create path-open 观测不封存 eventual imports；resume gate 不存在 |
| dynamic module-load enforcement | 0 | 0 | 1 | startup/import authority 不覆盖 runtime `LoadLibrary`，resume blocker 未实现 |
| startup/import searched-name namespace authority | 0 | 0 | 1 | directory share/oplock/digest 不能构造 kernel authority |
| process custody reachability | 0 | 0 | 1 | loader successor、launch-security 与 pre-create currentness backend producer 均不可达 |
| runtime Store / recovery | 0 | 0 | 1 | migration/table/writer 均为 none |
| health / Ready / v15 verifier | 0 | 0 | 1 | 不存在 |
| Provider / route / market / money | 0 | 0 | 1 | effect=none |

## 5. Authority gap 与 effect 核对

以下四项必须逐项保持 `missing`：

| Authority | 状态 |
|---|---|
| `node_local_authority_currentness` | `missing` |
| `runtime_transition_authority` | `missing` |
| `host_runtime_authority` | `missing` |
| `v15_authenticated_session` | `missing` |

以下全部必须逐项保持 `none`：

exact 18 项：`runtime_phase`、`runtime_generation`、`runtime_start`、`runtime_resume`、`runtime_store`、`health`、
`readiness`、`node`、`provider`、`route`、`offer`、`capacity`、`execution`、`attempt`、`lease`、`usage`、
`settlement`、`money`。

任一 source review 若发现这些值漂移，均应阻止本批收尾；当前核对仍不得计为动态或 test passed。

## 6. 未来动态故障矩阵

解除架构阶段禁令后，至少验证：

- launch-path Runner/package-root/全部 plan-directory retained chains 的 granted access、volume/FileId/type/reparse、
  single-component/Volume-GUID canonical relation 与每阶段 failure admission custody；
- authenticated PE normal/delay base-import symbol name/ordinal、descriptor/thunk ordinal与 separate forwarder
  source-export/target-symbol/逐跳 evidence/cycle-depth、canonical merge rule、package external-leaf coverage、expected architecture/WOW64、
  exact launch-path selection/grants、
  Runner-basename/preloaded/bootstrap authenticated cache seed、process-machine/cache-key/immutable-section/evidence
  drift、未来独立的 true transitive system closure、已冻结 canonical merged edge order、resolved-module cache-key collision closure、
  pre-lease package parser FileId/sealed-digest/policy 与 post-lease same-handle reparse/真实 lease-generation composite、跨代 splice、
  KnownDLL named-section→immutable-image-section mapping、当前 API-set non-recursive host与未来独立 nested typed DAG、SxS
  activation-context/search-directory/FileId/section binding、filesystem parent-relative retained file/open/section receipts、
  servicing-generation-bound system immutable lease、external search-directory handle-derived ancestor/parent-share-grant/
  alias-currentness chain 及 exact launch-context drift；
- searched-name terminal/absence与 launch-path grant漏项、Shadow当前拒绝边界、case/canonicalization/8.3/ADS/hardlink/reparse/mount、
  session disconnect/generation drift；present disposition 的 package/system FileId、system section/servicing-generation drift；
  统一 package+system lease-set 的 writable-open/disposition/section-map denial、partial acquisition 与 generation drift；
- name-grant/content-lease/namespace definitive rejection 必须持有 exact authenticated-negative owner；wrong session/
  attempt/request/nonce/FileId/digest 必须变为 outcome-uncertain，不得伪造 definitive；initial/final positive-but-invalid
  query receipt 同样必须进入 outcome-uncertain custody 且不得丢弃；grant/lease/query 同时 positive+negative 时同样不得
  definitive，positive receipt 必须保留；positive/negative response bytes/digest mutation、truncation 与重算 mismatch；
- 纯 pre-dispatch borrow failure 才返回 `BorrowOnlyNotTransitioned`；其余 pre-barrier failures 保留 acquired
  grants/leases、attempt/negative 和 pending ordinals；首个 close 后只返回 indexed
  `PostBarrierOutcomeUncertain`；
- close definitive/uncertain result、handle-value reuse 与 destructor fault 必须证明 close-uncertain source 进入 `ManuallyDrop`
  quarantine，不发生 ordinary `File` Drop；
- same path/different FileId、rename/swap/delete-pending、share conflict、size/link/reparse/content drift；anchor 必须被
  reopen attempt 独占消费，failure 必须保留 anchor-only attempt custody；receipt 必须证明 close/open/identity/hash/path
  和 content-lease continuity；reopen 同时 positive+negative 时 positive receipt/owner 也必须留在 uncertain custody；
- package/root/plan vectors 长度不等、ordinal/path 错配、截断式 zip；package files 多 ordinal、Runner
  first/middle/last 输入但实际 close 始终 Runner last，以及所有 indexed partial custody；
- extraction directory 原 DELETE access + share-read/write owner 与 parent-relative narrow share-delete probe 的
  共存、同 FileId/volume/non-reparse/canonical-path、retained-parent create-new、失败保管，以及普通目录 open 未被放宽；
- rejected replacement handle 不泄漏、不丢失，retry/admission extractor 不存在；
- Node crash/restart、fence owner disconnect、explicit-authorized release/recovery、orphan lease/grant/handle scan；
- 外部 root borrow 结束、unconfirmed/leaked graph 与 recovery scan 期间 exact `ComputePluginRootLockLease` 仍由 successor 保管；
- pre-create sealed KnownDLL/API-set/SxS echoes 与 live OS observation 差异，path open 后/resume 前的
  name/content/currentness drift，以及 runtime `LoadLibrary` denial；
- 后续 process bridge 形成后，loader/launch/pre-resume-currentness/IPC/enforcement/Store 任一 blocker 失败都不能
  create/resume child。

## 7. 负向验收

以下任一声明均为失败：

- 声称本批已生成 `LoaderLockedWorkAdmittedPluginSlot` 或调用了 Windows loader/process backend；
- 把 source-written candidate/exact-context/pre-lease/request/GrantReady/post-lease lineage source称为可达 resolver/producer、runtime selected CWD、component grant、exact launch-path authority 或
  Windows dynamic proof；
- 声称 exact PE graph、launch-path、resolution、name-grant、content-lease/query/reopen/release producer 可构造；
- 把完整 package file set、hash 或 extraction plan 称为 startup/import resolution authority；
- 忽略 Runner-basename/preloaded/bootstrap cache seed 与其 process-machine/cache-key/immutable-section/evidence binding、
  未来 PE true transitive system closure/canonical merged edge order/cache-key collision closure、KnownDLL
  named-section immutable-image mapping、当前 API-set non-recursive terminal与未来 nested typed DAG、SxS activation-context/search-directory/FileId/
  section binding、external search-directory ancestor/parent-share-grant/alias-currentness receipt，或 filesystem system
  servicing-generation lease 与 parent-relative retained file/open/section receipts；
- 把 directory handle/share mode、oplock 或 digest 称为 searched-name namespace authority；
- 声称 share-delete probe 替换了原 package-root/plan-directory owner、授权 loader namespace/content authority，或普通
  managed-directory open 已被全局放宽；
- 声称既有 extraction directory producer 的 access/share compatibility 已验证、admission→loader predecessor 已可达；
- 把 `startup_import_resolution_profile_digest` 或 `startup_import_namespace_authority_digest` 称为对应 authority；
- 把 sealed/pre-create KnownDLL/API-set/SxS fields 称为 live OS currentness，或把 share-read/hash 称为 FileId
  immutable-content lease；
- 把 source type、`Infallible` prerequisite 或 consuming seam 称为 producer 已实现；
- 用 FileId、digest、lease generation 或 policy 任一单字段替代 package PE parser canonical composite，或接受跨代
  parser splice；
- 用 path、digest、receipt scalar、caller-opened `File` 或短 borrow 替代 owner custody；
- 在缺少 exact authenticated-negative owner 时声称 name-grant/content-lease/namespace definitive rejection，或将其
  降格为 `BorrowOnlyNotTransitioned`，或在 grant/lease/reopen/query 同时 positive+negative 时丢弃 positive 并声称
  definitive，或丢弃 authenticated response bytes/digest、声称其 producer 可达；
- post-barrier failure 返回 old admission/retry permit，丢弃 indexed custody/replacement/final-query attempt，或 reopen
  不以 anchor-only attempt 消费/失败不保留该 custody/不保留 receipt+lease continuity，或丢弃 initial/final
  positive-but-invalid returned receipt；
- 对 close-outcome-uncertain source 执行 ordinary `File` Drop，或声称 startup/import set 已管控 runtime `LoadLibrary`；
- 声称 pre-create currentness 已封存 suspended child 开始运行时才解析的 eventual imports，或忽略精确 launch
  context binding；
- 以 Drop/session disconnect 冒充 persistent name grants/content leases 的 explicit authorized release 或 crash recovery；
- 让 loader successor 只借用 root lock，或在外部 root borrow 结束、unconfirmed/leaked graph 尚存时释放
  `ComputePluginRootLockLease`；
- 声称四项 authority gap 任一已关闭；
- 声称 runtime phase/generation/start/resume/Store、health、Ready、Provider、route、Offer、Capacity、Execution、Attempt、
  Lease、usage、settlement 或 money 已产生效果；
- 声称编译、Rust test、migration、SQLite、Windows、网络、设备、真实 Runner 或生产验收已经完成。
