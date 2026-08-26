---
title: UserNode Windows Runner 进程监管前置 V1 验收草案
status: draft
reviewed_at: 2026-08-26
owners: node, compute, windows
proposed_feature_id: compute-user-node-windows-runner-process-custody-v1
registration_status: unregistered_feature_workflow_unavailable
design_status: draft_frozen
implementation_status: source_written_uncompiled
verification_status: source_review_only
---

# UserNode Windows Runner 进程监管前置 V1 验收草案

## 1. 本批证据等级

本批只有 `source_written/source_review_only` 证据。用户要求架构铺设阶段不编译、不运行、不执行 migration 或真实验证，因此新增 Rust
source-contract guard 也未执行：

- implementation: `source_written/source_review_only/implementation_uncompiled`；
- runtime: `implementation_unrun`；
- code acceptance: `passed=0/failed=0`；
- persistence: `migration/table/writer=none/none/none`；
- dynamic Windows evidence: `0`。

格式化、文本、diff、体积与模块化检查只属于交付卫生，不提高运行成熟度。

## 2. 文件责任

| Owner | 文件 | 责任 |
|---|---|---|
| private facade | `runtime_process_custody.rs` | 路由 sealed model、policy、encoding、launch security、namespace query、Job 与 Windows backend；声明无 resume/Store/Ready |
| linear model | `runtime_process_custody/model.rs` | 自有 root-lock 的 loader successor、launch security、pre-create evidence 与完整 process/loader/lease/namespace owner graph |
| signed policy | `runtime_process_custody/policy.rs` | 精确来源 binding、creation prerequisites、resume blockers 与零效果 |
| Windows encoding | `runtime_process_custody/encoding.rs` | absolute UTF-16 path、argv quoting、显式空 environment |
| launch security | `runtime_process_custody/launch_security.rs` | uninhabited restricted/AppContainer/object-isolation authority、exact private desktop owners 与 post-currentness live validation |
| loader currentness | `runtime_process_custody/namespace_query.rs` | exact attempt/response bytes+digest/content-lease currentness、authenticated quarantine、unavailable release policy 与无实现 backend trait |
| Job/startup owner | `runtime_process_custody/windows_job.rs` | Job set/query-back、aligned Job-list RAII，以及 borrow-bind Job+launch-security 后设置 `lpDesktop` 的 startup info |
| Windows backend | `runtime_process_custody/windows.rs`、`windows_rollback.rs` | atomic-Job create、membership/identity query、四项 termination confirmation 与 unconfirmed whole-graph parking |
| source review | `runtime_process_custody_source_contract_tests.rs` | 未运行 guard，固定 owner、调用顺序、负边界和证据状态 |

## 3. 静态源码审阅目标

源码应满足：

1. loader load-set、launch security、pre-create/live-OS/pre-resume currentness、release/recovery 与 dynamic-load
   enforcement producers 均不存在；prepare/backend 只有私有定义，process 不可达；
2. load-set 以单一 `LoaderLockedWorkAdmittedPluginSlot<'root>` 持有 authority residue、indexed package files、
   retained/wrapped package-root/plan-directory handles、Runner ordinal 与 cwd；每个 package file 持有线性 FileId
   content lease 和 anchor-consuming reopen receipt，每个普通 filesystem system image 持有 servicing-generation-bound
   FileId/immutable-section lease 与 parent-relative file/open/section receipts；package+system 统一 lease-set 随全部
   failure/success/process custody，且只有 package files close/reopen；successor 还按值自有 exact
   `ComputePluginRootLockLease`，外部 root borrow 结束或 graph unconfirmed/leaked 均不得解锁；
3. name-grant、content-lease、纯 pre-dispatch borrow、namespace-query 与 indexed post-barrier 五类 loader failure
   custody 不暴露 raw handle/clone/retry authority；三种 definitive/authenticated rejection 都要求 exact
   authenticated-negative ownership；initial/final namespace query 的 positive-but-invalid returned receipt 同样进入
   outcome-uncertain custody 且不得丢弃；grant/lease/reopen/initial-or-final query 同时 positive+negative 时也不得
   definitive，positive receipt/owner必须保留；凡 typed custody声称 authenticated response均保留 exact bytes+digest并在
   classification前重算。当前明确的 positive shapes仅为 name-grant与system-image，其 consuming producer uninhabited；
4. `existing_extraction_directory_access_share_compatibility=source_seam_written_windows_dynamic_unverified`；
   retained DELETE owner + narrow share-delete probe 源码 seam 已线性接入；
   `launch_path_handle_chain_discovery=source_written_windows_dynamic_unverified`；exact-context/prelease/request/GrantReady/
   post-lease lineage contracts为 `source_written_uncompiled_unrun`，但真实 selector/parser/resolver/advancer/component-grant
   producers仍 `missing`，不得认为 loader predecessor已动态可达；
5. 两阶段顺序固定为 discovery/pre-lease material → authenticated selection/preliminary plan → GrantReady exact resolution → grants → leases →
   same-handle rehash/reparse → final seal → query/reopen。authenticated exact PE graph 与 launch-path parent-chain
   authorities 以 `Infallible` 保持 uninhabited；sealed
   startup/import resolution 必须先以 Runner basename 与 authenticated process preloaded/bootstrap set 预种
   expected process-machine/cache-key/immutable-section/evidence-bound cache，再形成 Runner-rooted package-image closure claim与
   typed external leaves；true transitive system closure/system-image own imports仍是 freeze blocker。normal/delay base imports绑定
   symbol name/ordinal、descriptor/thunk ordinal，separate forwarder hops绑定 source-export/target-symbol/逐跳 evidence/cycle-depth、
   已冻结 canonical merge rule 与 resolved-module cache-key collision closure；pre-lease parser material 不得预测 lease
   generation，post-lease final sealer 必须在同一 handles 重哈希/重解析后加入真实 generation，禁止跨代 parser splice；
   KnownDLL named-section→immutable-image-section mapping；当前 API-set只允许一步映射到 exact non-recursive authenticated
   preloaded、KnownDLL、普通 filesystem search或 SxS terminal，nested API-set DAG fail-closed；SxS绑定
   activation-context receipt、exact search directory、FileId 与 immutable section；present searched-name disposition
   直接绑定 package FileId，或 system FileId + immutable section + servicing generation，实际 lease owners 先为 post-lease
   final sealer 提供 generations，随后 consuming query/aggregate 再确认同一 set；external System/Windows/SxS search directory 保留 full handle-derived ancestor chain、
   parent share/grant contract 与 namespace-alias currentness receipt，并纳入
   `startup_import_resolution_profile_digest`；同时绑定 exact
   token/AppContainer、expected architecture/WOW64、empty-environment/search policy、cwd和 creation flags；这些 machine字段
   不是 post-create live query-back receipt；
6. preparation 与 process custody 按值拥有 successor 与 launch security，不再同时拥有 original admission 和
   detached image；
7. launch token 必须 query-back primary 且 restricted/AppContainer profile 不漂移，token handle 不可继承；
8. process/thread SD 基线必须 aligned/self-relative 且 empty-DACL 形状精确；但 empty DACL 不证明
   same-account sibling-safe，owner SID、mandatory label、service-SID/account isolation、effective ACL 与
   `WRITE_DAC` 拒绝仍需 producer/query-back；
9. launch-security owner graph 必须按值保留 exact private window-station 与 desktop owners 及 validated qualified
   desktop name，并持有 authenticated HDESK↔HWINSTA parent-relation receipt、live primary-token `TokenSessionId`、
   `AuthenticationId`/logon namespace 与 exact desktop access-check receipt；token-session receipt 同时绑定 exact
   HWINSTA/HDESK handles 以阻止同名/ABA splice；`ConfiguredRunnerStartupInfo<'owner>`
   borrow-bind Job+launch security 并把该 name 放入
   `STARTUPINFO.lpDesktop`。loader currentness 成功后、create 紧邻处必须再次 live validate token/session/logon、SD、
   HWINSTA/HDESK/access receipts 与 qualified name，且不得插入其他 fallible setup；这些 owner/isolation producers 以
   `Infallible` 保持 uninhabited，不能以 digest-only claim 代替；
10. Job 在 process 前创建并 set/query-back kill-on-close/process-count/job-memory，拒绝 breakaway flags；attribute-list
    两阶段 aligned 初始化，Job handle value 稳定且 owner-bound startup info 存活覆盖 create call；
11. 所有可失败 Job/attribute setup 完成后，consuming currentness backend 必须先绑定
    namespace/fence/统一 package+system content-lease generations/resolution、process machine/launch context、sealed KnownDLL/API-set/SxS/system-image
    echoes、session/grant/query generations、exact attempt/receipt，并重算 final start digest；
12. pre-create `DefinitiveRejected` 只有在 authenticated-negative 与 exact retained session/attempt/request/nonce/
    fence/content-lease generations 匹配时成立；错绑或缺失必须是 `OutcomeUncertain`。两者都 quarantine preparation/
    attempt/optional negative；同时返回 positive+negative 时也不得 definitive，positive receipt 留在 outcome-uncertain
    custody。positive/negative response 的 exact bytes+digest 都保留并重算，producer 仍 uninhabited；query success 和
    全部后续 failures 都保留 currentness evidence；
13. sealed/pre-create KnownDLL/API-set/SxS 字段不是 live OS observations；
    `explicit_authorized_release_required_but_unavailable`、live Windows resolution currentness、pre-resume
    loader-currentness/import-period retention 与 dynamic `LoadLibrary` enforcement 均为 resume blockers；
14. currentness 后的 launch-security live validation 与 `CreateProcessAsUserW` 紧邻；create 使用 exact token/SD/
    private-desktop/path/argv/environment/cwd 与 false handle inheritance；flags 必须包含 suspended/Unicode/no-window/
    extended-startup，不存在 post-create Assign fallback；
15. success 后必须 `IsProcessInJob`；raw NULL/alias 先识别，每个 distinct handle 只进入一次 `OwnedHandle`；
    PID/TID 分别从 process/thread handles 回读，creation time 只从 process handle 读取；
16. post-create failure 与 success Drop 都只能在 termination confirmed 后显式 Drop entire
    process+loader+统一 package+system lease+namespace graph；未确认时必须把整图 parked 在 `ManuallyDrop`，且不得独立释放
    handles、grants、leases 或 root-lock lease；confirmed 必须同时证明 `TerminateJobObject` 成功、root process signaled、Job signaled 与
    exact accounting `ActiveProcesses==0`；confirmed in-process Drop 不等于 persistent grant authorized release，当前
    recovery/release service 不存在；
17. post-create process-image FileId、process/object owner-label-ACL 与 private-desktop query-back 都保持 blocker；
18. source slice 不存在 `ResumeThread` 或通用 spawn；`node_local_authority_currentness`/
    `runtime_transition_authority`/`host_runtime_authority`/`v15_authenticated_session` 均为 `missing`。process start-material
    自己冻结的 runtime/health/Ready/Provider/route/Offer/Capacity/Execution/Attempt/Lease/usage/settlement/money effects
    均为 `none`，不得把 loader 的 18 项集合错误扩展到该较窄列表。

这些目标目前只作为未运行 Rust guard 与人工 source review 的目标，不能记为 passed。

## 4. 明确未验收矩阵

| 轴 | passed | failed | unrun | 当前结论 |
|---|---:|---:|---:|---|
| Rust 编译 / Windows 链接 | 0 | 0 | 1 | 未编译，Win32 签名与 feature 未由 compiler 证明 |
| source-contract Rust test | 0 | 0 | 1 | guard 已写但未运行 |
| share-none→locked loader load-set | 0 | 0 | 1 | linear leases/reopen receipts/五类 failure-custody shapes 已写；success producers 不存在 |
| successor-owned `ComputePluginRootLockLease` | 0 | 0 | 1 | exact owner shape 已写；external-borrow end 与 parked/leaked lifetime 未运行 |
| existing extraction directory access/share compatibility | 0 | 0 | 1 | typed retained-owner seam 已写；Windows access/share、identity/path、descendant 与 failure-custody matrix 未运行 |
| launch-path retained-handle candidate discovery | 0 | 0 | 1 | source seam 已写但未动态运行；没有 selected CWD 或 component grant |
| exact-context/prelease/request/GrantReady contracts | 0 | 0 | 1 | source written；真实 selector/parser/resolver/advancer producers missing，Shadow rejected |
| exact PE graph / launch-path / startup resolution | 0 | 0 | 1 | PE/parent-chain authorities uninhabited；post-lease sealer 与 launch-context producer 未验证 |
| grants / unified package+system FileId leases / anchor reopen | 0 | 0 | 1 | acquisition/query/reopen backends 均不存在；servicing generation 与 exact negative ownership 未动态验证 |
| live OS KnownDLL/API-set/SxS currentness | 0 | 0 | 1 | sealed/pre-create fields 只是 echoes；live backend 不存在 |
| pre-create loader currentness backend | 0 | 0 | 1 | exact attempt/receipt/failure contract 已写，trait 无实现，process 不可达 |
| authenticated query response bytes/digest | 0 | 0 | 1 | name-grant/system-image positive custody shapes已写；consuming producers uninhabited，未运行重算矩阵 |
| post-create live machine/WOW64 query-back | 0 | 0 | 1 | pre-create expected projection only；`IsWow64Process2`/equivalent receipt/backend missing |
| pre-resume loader currentness / dynamic module load | 0 | 0 | 1 | pre-create path-open 观测不封存 eventual imports；resume blockers 缺失 |
| launch security + private desktop producer | 0 | 0 | 1 | exact owner/lpDesktop/live-revalidation shapes 存在但 uninhabited；owner/label/account/object/desktop query-back 缺失 |
| `CreateProcessAsUserW` + atomic Job-list | 0 | 0 | 1 | 未在 Windows 运行 |
| nested Job / no-breakaway | 0 | 0 | 1 | 未运行 |
| termination / whole-graph park / recovery | 0 | 0 | 1 | terminate+process/Job signal+ActiveProcesses=0 四项 shape 已写；unconfirmed recovery/release service 不存在 |
| argv / Unicode / environment | 0 | 0 | 1 | 仅源码，未对真实 Runner 验证 |
| complete token/object isolation / sandbox | 0 | 0 | 1 | empty DACL 不拒绝 owner `WRITE_DAC`；完整 query-back/攻击矩阵不存在 |
| namespace explicit release / crash recovery | 0 | 0 | 1 | release policy 明示 unavailable；persistent grants/leases 无 release/recovery producer |
| CPU/VRAM/disk/network/uptime | 0 | 0 | 1 | resume blocker |
| authenticated IPC / health | 0 | 0 | 1 | 不存在 |
| runtime Store / recovery | 0 | 0 | 1 | 无 schema/table/writer |
| Ready / v15 / server verifier | 0 | 0 | 1 | 不存在 |
| Provider / market / money | 0 | 0 | 1 | effect=none |

`failed=0` 只表示没有执行失败项，不表示通过。

## 5. 未来动态故障矩阵

解除架构阶段禁令后，至少验证：

- share-none owned transition 的 name-grant/content-lease authenticated rejection、纯 pre-dispatch
  `BorrowOnlyNotTransitioned`、namespace-query definitive/uncertain quarantine 与 indexed
  `PostBarrierOutcomeUncertain`；wrong owner/session/attempt/request/nonce/FileId/digest negative 必须变为 uncertain；
  grant/lease/reopen/initial-or-final query 同时 positive+negative 时 positive receipt/owner 必须留在 uncertain custody；
  authenticated positive/negative response bytes/digest mutation、truncation 与 recompute mismatch；
- linear FileId lease、anchor-consuming reopen receipt、close-uncertain no-Drop、same path/different FileId、rename/swap/
  reparse/hardlink/share-mode、writable mapping、重哈希与 searched-name mutation；package+system 统一 lease-set 及 system
  servicing generation 必须穿越全部 failure/success/process custody；
- extraction-directory retained DELETE owner + narrow share-delete probe 的共存、同 FileId/path、descendant
  retained-parent create-new、失败保管，以及普通 managed-directory open 未被放宽；
- Runner/package-root/全部 plan-directory retained launch-path chains 的 granted access、volume/FileId/type/reparse、
  single-component/Volume-GUID canonical relation 与 failure admission custody；
- authenticated PE graph/exact launch-path selection+grants、normal/delay base-import symbol name/ordinal、descriptor/thunk
  ordinal、separate forwarder source-export/target-symbol/逐跳 evidence/cycle-depth、canonical merge rule，以及
  token/AppContainer、expected architecture/WOW64、未来独立的 true transitive system closure/cache-key collision closure、pre-lease parser
  FileId/sealed-digest/policy 与 post-lease same-handle reparse/真实 lease-generation composite、跨代 splice、KnownDLL
  named-section mapping、当前 API-set non-recursive host与未来独立 nested typed DAG、SxS activation-context/search-directory/FileId/section
  binding、filesystem servicing-generation lease 与 parent-relative retained file/open/section receipts、present disposition
  package/system FileId + system section/servicing-generation binding、external search-directory handle-derived ancestor/
  parent-share-grant/alias-currentness chain，以及 environment/search policy、cwd/creation flags 不同 launch context；
- pre-create namespace/fence/content-lease/resolution/expected-process-machine/session/grant/query attempt+receipt漂移，以及
  post-create live process machine/WOW64 query-back缺失或不一致；sealed
  KnownDLL/API-set/SxS echoes 与 live OS observation 差异；query definitive rejection 只接受 exact authenticated negative，
  同时 positive+negative 时 positive receipt 与 response bytes/digest 必须留在 uncertain custody；
- process path open 后、primary-thread resume 前的 loader 状态漂移、eventual import resolution 及 runtime `LoadLibrary`；
- restricted/AppContainer token type、integrity/SID/privilege/capability 漂移、adjust-handle 残留、default/NULL/wide
  DACL、owner SID/mandatory label/service-SID/account 变体、owner `WRITE_DAC`、effective ACL query-back、sibling
  reopen/ResumeThread/CreateRemoteThread 与 admin/SeDebug 边界；
- private window-station/desktop 创建、authenticated HDESK↔HWINSTA parent relation、qualified-name validation、
  live primary-token `TokenSessionId`、`AuthenticationId`/logon namespace、exact desktop access-check receipt、
  token-session receipt exact HWINSTA+HDESK handle binding、同名/ABA splice、`lpDesktop` owner lifetime、
  desktop/object query-back、loader-currentness 后紧邻 create 的 launch-security live revalidation 与 sibling UI-object access；
- 空格、反斜杠、引号、Unicode、NUL、超长 argv/cwd 及 environment secret non-inheritance；
- Job create/set/query/attribute-list/create/membership 各点失败；
- parent 已在兼容/不兼容 Job、旧 Windows 不支持 Job-list、breakaway 拒绝、process/memory query-back 漂移；
- NULL/aliased process/thread handles、PID/TID mismatch、creation time 失败、suspended child 早退；
- `TerminateJobObject` 失败、root process/Job wait timeout/failed、accounting query failure/nonzero、already-exited 与
  success/failure Drop；只有 terminate + root-process signaled + Job signaled + exact `ActiveProcesses==0` 才能确认；终止未确认时 entire
  process+loader+统一 package+system lease+namespace graph 必须 parked，不能发生 independent Drop；
- 外部 root borrow 结束、unconfirmed/leaked graph 与 recovery scan 期间 `ComputePluginRootLockLease` 必须继续保管；
- explicit authorized release、unconfirmed-process recovery、Job kill-on-close/orphan scan、多次并发准备、Node
  crash/restart 后 owners/grants/leases 残留检查；
- 后续 pre-resume currentness/IPC/enforcement/Store 形成后，resume 前任一 blocker 失败都保持
  suspended 或终止。

## 6. 负向验收

以下任一声明均为失败：

- 声称本批已创建、启动或运行真实 Runner；
- 把 suspended process custody 称为 Host runtime、完整 sandbox、transition receipt、health 或 Ready；
- 把 Job/token/empty DACL/private-desktop digest 称为 same-account sibling-safe、完整 launch isolation、CPU/VRAM/disk/
  network/uptime 或完整 grant enforcement；
- 声称 loader、exact PE graph/launch path、launch-security/private-desktop、live-OS/pre-create/pre-resume currentness、
  release/recovery 或 dynamic-load enforcement producer 已实现/可达；
- 把 launch-path candidate discovery receipt 称为 selected CWD、component grant、exact launch authority 或动态证明；
- 声称 pre-create currentness 已封存 suspended child 运行时才解析的 eventual imports；
- 在 loader currentness 后跳过紧邻 `CreateProcessAsUserW` 的 launch-security live validation，或在两者间插入 fallible setup；
- 把 sealed/pre-create KnownDLL/API-set/SxS echoes 称为 live OS currentness；
- 忽略 launch-context resolution binding、extraction-directory access/share prerequisite、owner SID/mandatory label/
  service-SID/account isolation、live token session/logon namespace、exact desktop access-check、effective ACL/
  private-desktop query-back 或 qualified `lpDesktop` owner binding；
- 忽略 Runner-basename/preloaded/bootstrap cache seed 的 expected process-machine/cache-key/immutable-section/evidence binding、
  当前 package-image closure/external leaves、未来 system recursive edge/cache-key closure、KnownDLL named-section immutable-image mapping、当前 API-set non-recursive terminal与未来 nested typed DAG、SxS
  activation-context/search-directory/FileId/section binding、filesystem system servicing-generation lease 与 parent-relative
  retained file/open/section receipts、external search-directory ancestor/parent-share-grant/alias-currentness receipt、present
  disposition exact binding 或 HDESK↔HWINSTA/token-session exact-handle binding；
- 用 package parser FileId/digest/lease generation/policy 任一单字段替代 canonical composite，或接受跨代 parser splice；
- 在缺少 exact authenticated-negative ownership 时声称 name-grant/content-lease/namespace/pre-create definitive rejection，
  在 grant/lease/reopen/pre-create query 同时 positive+negative 时丢弃 positive 并声称 definitive，或在 currentness
  failure 后 retry preparation；丢弃 authenticated response bytes/digest 或声称 response producer 可达；
- 丢失 pre-create query success、post-query failure 或 process success 中的 exact currentness attempt/receipt evidence；
- 终止未确认时独立 Drop process handles、launch-security、loader successor、统一 package+system FileId leases 或 namespace owners，或以
  Drop/session disconnect 冒充 explicit authorized release/recovery；
- 仅凭 root process `WAIT_OBJECT_0`、Job signal 或 `ActiveProcesses==0` 单项释放整图，或在 parked/leaked graph 尚存时
  因外部 root borrow 结束而释放 `ComputePluginRootLockLease`；
- 用 path、PID、caller digest、CLI sidecar 或 Runner `Started` 事件替代 owner custody；
- 声称 source-lineage 四项 gap 任一已经关闭；
- 声称 Provider active、route、Offer、Capacity、Execution、Attempt、Lease、计量、结算或资金效果；
- 声称编译、测试、Windows 动态矩阵、migration 或生产验收已经完成。
