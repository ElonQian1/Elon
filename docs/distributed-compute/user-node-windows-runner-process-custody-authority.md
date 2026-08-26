---
title: UserNode Windows Runner 进程监管前置 V1 权威草案
status: draft
reviewed_at: 2026-08-26
owners: node, compute, windows
proposed_feature_id: compute-user-node-windows-runner-process-custody-v1
registration_status: unregistered_feature_workflow_unavailable
design_status: draft_frozen
implementation_status: source_draft_uncompiled
verification_status: source_review_only
---

# UserNode Windows Runner 进程监管前置 V1 权威草案

## 1. 本批结论

本批只冻结并写入 **Windows suspended child + atomic Job + pre-create loader-currentness custody
prerequisite**。它不是 Host start、runtime transition、完整 sandbox、active health 或 Ready authority。当前源码没有
loader load-set 成功 producer、restricted/AppContainer/private-desktop launch-security producer 或
`WindowsRunnerPreCreateLoaderCurrentnessBackend` 实现，所以 Win32 backend 仍是 default-blocked/unreachable。
pre-resume loader-currentness 与动态 module-load enforcement 也不存在；源码中存在
`CreateProcessAsUserW` 不等于真实进程已经创建或 eventual imports 已被封存。

状态固定为：

- design: `draft_frozen`；
- implementation: `source_written/source_review_only/implementation_uncompiled`；
- runtime: `implementation_unrun`；
- code acceptance: `passed=0/failed=0`；
- persistence: `migration/table/writer=none/none/none`；
- feature registration: `unregistered_feature_workflow_unavailable`。

## 2. 为什么先落进程监管前置

现有 `ComputePluginHost` 只运行 legacy in-process LLM；`lifecycle.rs` 只有 DTO 与纯 transition predicate；
`runner_events.rs` 只有 Runner-originated payload，没有认证 IPC。直接增加 `RuntimeStartReceipt` 会让调用方用
scalar/DTO 冒充进程事实。正确首片必须至少持有：

1. 从 `DurableWorkAdmittedPluginSlot<'root>` 按值转换且不再保留旧 share-none owner 的单一 successor；successor 自有
   exact `ComputePluginRootLockLease`，不依赖外部 root borrow 存活；
2. 同一 Runner 文件身份、全量重哈希、exact PE graph/launch-path、startup/import resolution、searched-name
   namespace grants 与线性 FileId content leases；resolution 必须绑定 token/AppContainer、target architecture/WOW64、empty-environment/search
   policy、cwd 与 creation flags；
3. 从精确 grant 生成、回读并封存的 restricted/AppContainer primary token 与 process/thread SD，以及 exact private
   window-station/desktop owners 和 validated qualified desktop name；
   owner SID、mandatory label、service-SID/account isolation 及 object ACL query-back 均必须显式证明，empty DACL 不足够；
4. 匿名、不继承、禁止 breakaway 的 Job Object；
5. 由 `PROC_THREAD_ATTRIBUTE_JOB_LIST` 将 Job 原子附加到新 child 的 aligned attribute-list；
6. 所有可失败 Job/attribute setup 后的 consuming loader-currentness query，以及 query 后、
   `CreateProcessAsUserW` 紧邻处的 launch-security live validation；
7. `CreateProcessAsUserW(CREATE_SUSPENDED)` 返回的 distinct process/primary-thread owned handles，以及从句柄回读的
   PID、TID 与 creation `FILETIME`；
8. path-based process open 后的 pre-resume loader-currentness；普通 imports 多在 primary thread 后续运行时解析，
   pre-create 观测不能取代该 resume gate；
9. 失败时的 definitive/outcome-uncertain quarantine、currentness evidence 与全部 process/loader/lease/namespace/OS
   authority 线性 custody；终止未确认时整图 parked，不返回 retry permit。

进程草案只写入上述 sealed/type/backend 边界；loader、launch-security、pre-create query 与 pre-resume
currentness/enforcement producer 均未实现，因此没有可达 production call path。

## 3. Owner graph

私有 owner graph 固定为：

`LoaderLockedWorkAdmittedPluginSlot + SealedWindowsRunnerLaunchSecurity`
→ `ValidatedWindowsRunnerProcessPreparation`
→ `CreateJobObjectW + Set/QueryInformationJobObject + PROC_THREAD_ATTRIBUTE_JOB_LIST`
→ consuming `WindowsRunnerPreCreateLoaderCurrentnessBackend::query_current_and_seal`
→ `LoaderCurrentWindowsRunnerProcessPreparation + WindowsRunnerPreCreateLoaderCurrentness`
→ owner-bound `ConfiguredRunnerStartupInfo` with qualified `STARTUPINFO.lpDesktop`
→ post-currentness launch-security live validation（紧邻 create）
→ `CreateProcessAsUserW(CREATE_SUSPENDED | EXTENDED_STARTUPINFO_PRESENT)`
→ `IsProcessInJob + process/thread identity query-back`
→ `PreparedComputePluginRunnerProcess`。

`LoaderLockedWorkAdmittedPluginSlot` 是唯一 admission successor：authority 侧保留 receipts/trusted-time/raw downloads/
staging seal 与 exact 自有 `ComputePluginRootLockLease`，image 侧保留全 package typed handles、Runner ordinal、工作目录和 namespace prerequisite。最终
process custody 不实现 `Clone`/Serde，继续携带 `'root`，并拥有该 successor、launch token/SD/private desktop、pre-create
currentness evidence、Job、process、primary thread 和 process identity。没有 `ResumeThread` 入口；Drop 只有在终止确认后
才显式释放整图，终止未确认则把整个 process+loader+lease+namespace authority graph 留在 `ManuallyDrop` 中 parked。
外部 root borrow 结束不会释放 successor 自有 root-lock lease；unconfirmed/leaked graph 继续保管该 lease。
pre-create evidence 只针对 process path open 时点，不声称 eventual imports 已被封存。

## 4. Sealed load-set 的 owned contract 与 producer 缺口

现有 `PinnedManagedFile` 在 Windows 以 read/write/delete、share=0 打开。share mode 不可原地修改；原 handle 存活时
无法再开 loader-compatible data handle，关闭原 handle 后短借用 API又不能表达 reopen/identity/hash 任一点失败的
outcome uncertainty。因此未来 bridge 不能是 path/raw-handle getter，也不能是 `&mut` 短借用 transition。

相邻 source-only loader 草案已把方向冻结为 owned linear transition contract：

- 按值消费 exact admitted/archive Runner custody，不能接收 caller path、digest、index；
- 先取得与 exact session/parent/name/disposition/generation 绑定的 searched-name/launch-path grants，再取得 package
  files 与普通 filesystem system images 的统一 linear content-lease set，在 lease 下 same-handle rehash，最后执行
  consuming generation query；每种 definitive/authenticated rejection 都必须保留与 exact owner/attempt/session/
  request/nonce 匹配的 authenticated-negative receipt；
- 将每项 package-file content lease 移入 parent-relative metadata identity anchor，system-image lease 保留在 resolution
  owner；reopen 必须消费该 anchor 与 replacement handle 才能产生证明 lease continuity 的 receipt；post-close dispatch
  本身只持有 anchor-only attempt custody，replacement 是 backend outcome；
- 原 package-root/plan-directory handles retained/wrapped，只对 package files close/reopen；reopen 后重证
  volume/FileId/type/reparse/link/size/digest/path；
- startup/import resolution 绑定 normal/delay/forwarder、package/system/KnownDLL/API-set/SxS/search-name 和 exact
  launch context；PE authority 必须先以 Runner basename 与 authenticated process preloaded/bootstrap set 预种
  process-machine/cache-key/immutable-section/evidence-bound module cache，再要求 true reachable closure、per-importer continuous interleaved edge order 与
  resolved-module cache-key collision closure；package PE parser material identity 是 FileId + sealed digest + lease
  generation + immutable policy 的 canonical composite，parser evidence/edges 不得跨代 splice；KnownDLL named sections
  映射到 immutable image sections；API-set
  contract→host 递归终止于 exact authenticated preloaded module、KnownDLL、普通 filesystem search 或 SxS terminal；SxS 绑定
  activation-context receipt、exact search directory、FileId 与 immutable section；普通 filesystem system images
  保留 servicing-generation-bound immutable content lease 与 parent-relative file/open/section receipts。present
  searched-name disposition 直接绑定 package FileId，或 system FileId + immutable section + servicing generation；实际
  package+system lease generations 只在随后 consuming query/aggregate 中绑定；whole searched-name namespace authority
  单独保留；external System/Windows/SxS search directory 还保留 full handle-derived ancestor chain、parent share/grant
  contract 与 namespace-alias currentness receipt，并纳入 `startup_import_resolution_profile_digest`；
- 五类 failure custody 分别是 name-grant acquisition、content-lease acquisition、纯 pre-dispatch
  `BorrowOnlyNotTransitioned`、namespace-query definitive/uncertain quarantine 与 indexed
  `PostBarrierOutcomeUncertain`；initial/final query 的 positive-but-invalid returned receipt 同样进入
  outcome-uncertain custody 并保留；grant/lease/reopen/initial-or-final query 同时返回 positive+negative 时不得
  definitive，positive receipt/owner 必须保留；所有 authenticated positive/negative response 均保留 exact bytes +
  response digest 并在 classification 前重算核对，对应 producer 仍 uninhabited；close ambiguity 使用
  `ManuallyDrop` quarantine，不运行 ordinary `File` Drop。

但 cache-seed/exact PE graph、launch-path parent-chain 和 resolution authority 以 `Infallible` 保持不可构造，name-grant/
FileId-lease/query/reopen producer 也不存在。既有 extraction directory creation 可能以 DELETE access 配合
share-read/write，后续 traversal/reopen 又缺 `FILE_SHARE_DELETE`；本 slice 未修改或验证该 predecessor，
`existing_extraction_directory_access_share_compatibility=missing/unverified`，不得声称 loader 输入可达。精确边界见
[`user-node-windows-runner-loader-load-set-authority.md`](user-node-windows-runner-loader-load-set-authority.md)。

## 5. Launch security 前置

受限 token 不能在 child 创建后补装。默认 process/thread DACL 可能允许 sibling 按 PID/TID reopen 并恢复或注入
suspended child；显式 empty DACL 也不等于 same-account sibling-safe，因为 object owner 仍隐含保留
`WRITE_DAC`。因此 launch security 必须是 **create prerequisite**：

- `SealedWindowsRunnerLaunchSecurity` 私有字段、无 constructor，不暴露可调 token handle；其 owner graph 按值持有
  exact `OwnedPrivateWindowStation`、`OwnedPrivateDesktop`、authenticated HDESK↔HWINSTA parent-relation receipt 与
  validated NUL-terminated qualified desktop name；
- primary token 必须是 restricted 或 AppContainer primary token，且 future producer 必须关闭所有 adjust handles，只保留
  `CreateProcessAsUserW` 必需的 least-rights unique handle；
- future producer 必须 query-back integrity、restricted/AppContainer SID、privilege、capability 与 token type，并封存
  canonical profile digest；
- private-desktop authority 必须绑定 live primary-token `TokenSessionId`、`AuthenticationId`/logon namespace 与
  对 exact window-station/desktop access 的 authenticated access-check receipt；token-session receipt 必须同时绑定
  exact HWINSTA 与 exact HDESK handle，阻止同名/ABA splice。这些 receipt 与 HDESK↔HWINSTA parent relation、
  qualified desktop name 必须属于同一 owner graph；
- process/thread SD 使用显式对齐的 immutable self-relative buffer；调用紧前重验有效性、精确长度、
  `SE_SELF_RELATIVE`、DACL present/non-NULL/non-defaulted 且 ACE count 为零；
- future producer 还必须封存并 query-back process/thread object owner SID、mandatory integrity label、
  service SID 或独立 account isolation、精确有效 ACL/owner semantics，证明同账户 sibling 不能获得
  `WRITE_DAC`/resume/injection 权限；
- `ConfiguredRunnerStartupInfo<'owner>` 同时 borrow-bind exact Job 与 launch-security owners，并将上述 qualified name
  传给 `STARTUPINFO.lpDesktop`；因此不能用孤立 digest 或 caller string 代替 private desktop custody；
- `SECURITY_ATTRIBUTES.bInheritHandle=FALSE`，`CreateProcessAsUserW.bInheritHandles=FALSE`。

源码已写 owner/label/access-check/private-desktop sealed fields、exact window-station/desktop owner shapes、live primary-token
session/logon-namespace 与 desktop access-check fields、parent-relation receipt、qualified-name validation 与 `lpDesktop`
borrow wiring；但对应 producer 由 `Infallible` 保持 uninhabited，且 post-create effective
object-security/private-desktop query-back、explicit release/recovery 都不存在。这些都是 `missing/unverified`
launch-security prerequisites；管理员、SeDebug 与 Host 自身 process-handle 安全也未验收，不能把此边界称为
complete launch isolation 或 sandbox。

## 6. Pre-create loader-currentness custody

`runtime_process_custody/namespace_query.rs` 负责当前性 type/failure contract。在 input encoding、launch-security
validation 以及所有可失败 Job creation/configuration/attribute-list setup 结束后，`windows.rs` 必须在
create 序列末端按值消费 `ValidatedWindowsRunnerProcessPreparation`，调用
`WindowsRunnerPreCreateLoaderCurrentnessBackend::query_current_and_seal`。

成功的 `WindowsRunnerPreCreateLoaderCurrentness` 必须同时绑定并验证：

- `startup_import_namespace_authority_digest`、fence-generation set、统一 package+system content-lease-generation set 与
  `startup_import_resolution_profile_digest`；
- KnownDLL OS-build identity/Object Manager section generation、API-set schema/host authority 与 SxS activation-context
  identity；
- process-machine/launch-context material，以及 exact query attempt/receipt 的 session、request、nonce、fence/content-lease
  generation bindings；
- kernel driver session identity、grant generation、query generation 与
  `explicit_authorized_release_required_but_unavailable` policy；
- original/base start material 与包含上述观测的 recomputed final start digest。

这些 KnownDLL/API-set/SxS/system-image 字段只是 loader-sealed echoes，不是 live OS observation；
`live_windows_resolution_currentness` 仍是 resume blocker。definitive rejection 只有在 authenticated-negative receipt 与
retained exact namespace session/query attempt/request/nonce/fence/content-lease generations 全部匹配时成立；缺失或错绑的
negative 必须归入 outcome uncertainty；pre-create query 同时返回 positive+negative 时也不得 definitive，positive
receipt 必须随 preparation/query attempt/negative 进入 non-reusable outcome-uncertain quarantine。authenticated
positive/negative response 都必须保留 exact response bytes + response digest 并在分类前重算；response/backend producer
仍 uninhabited。查询成功后，
`LoaderCurrentWindowsRunnerProcessPreparation` 被 process
success 和每个后续 failure 继续持有，不丢失 currentness evidence。当前 trait 无实现，没有 backend
producer，process path 仍不可达。

loader-currentness 成功后，必须在 `CreateProcessAsUserW` 紧邻处再次 live validate retained launch-security owner：
primary-token type/session/logon identity、process/thread SD、exact HWINSTA/HDESK parent/access receipts 与 qualified
desktop name 全部不得漂移；live validation 与 create 之间不得插入其他 fallible setup。该顺序形状已写，但 launch-security
producer 仍 uninhabited，因此不形成可达 create path。

该观测只针对紧随其后的 path-based process open。primary thread 仍为 suspended，普通 imports 多在它后续
运行时解析；因此还需独立、consuming pre-resume loader-currentness/enforcement backend，并在 import resolution
期间继续持有权威。该 backend 同样不存在，pre-create evidence 绝不表示 eventual imports 已 sealed。

## 7. Atomic Job 与 suspended process 顺序

源码顺序固定为：

1. 在 OS 副作用前完成 absolute path、Windows argv quoting、空 allowlist environment、数值转换及 launch-security
   query-back；
2. 创建匿名 Job，设置并回读 `KILL_ON_JOB_CLOSE`、signed `max_processes` active-process limit 和 signed
   `max_memory_bytes` job-memory limit；回读不得出现 breakaway flags；
3. 用 aligned RAII buffer 两阶段初始化 attribute-list，把唯一 Job handle 以 `PROC_THREAD_ATTRIBUTE_JOB_LIST` 写入；
4. 所有 Job/attribute setup 完成后，消费 preparation 执行 pre-create loader-currentness query 并重算 final
   start digest；definitive/uncertain failure 均 quarantine；
5. query 成功后创建 borrow-bound startup info，将 retained qualified desktop name 置入 `STARTUPINFO.lpDesktop`；随后
   live validate exact launch-security owner，并在无其他 fallible setup 的紧邻位置，以 exact application path、mutable
   argv、显式空 Unicode environment、handle-derived cwd、restricted primary token 与 process/thread security attributes
   调用 `CreateProcessAsUserW`；
6. flags 固定为 `CREATE_SUSPENDED | CREATE_UNICODE_ENVIRONMENT | CREATE_NO_WINDOW |
   EXTENDED_STARTUPINFO_PRESENT`，不得存在 post-create `AssignProcessToJobObject` fallback；
7. success 返回即必须已在 Job；每个 distinct non-NULL raw handle 各一次进入 `OwnedHandle`，alias/partial contract anomaly
   进入 fail-closed rollback；
8. suspended 状态下以 `IsProcessInJob` 回读，并从 process/thread handles 分别回读 PID/TID、creation `FILETIME` 与 live
   状态；
9. 返回持有 pre-create currentness evidence 的 inert custody，绝不 resume。

显式空 environment 只避免继承 Node 进程秘密，不是未来运行环境。认证 IPC/bootstrap 必须另行定义排序 allowlist，才
可能替换此占位边界。

## 8. Enforcement 边界

本批只形成 restricted/AppContainer token sealed prerequisite、empty-DACL child objects、kill-on-close、process-count 与
job-memory 的源码基线。以下仍固定为 resume blocker：

- authenticated IPC bootstrap；
- exact PE graph、launch-path parent-chain、FileId lease/name-grant/query/reopen 成功 producers；
- live Windows KnownDLL/API-set/SxS resolution currentness；
- caller privilege/primary-token lineage authority 与 exact target-token access-check receipts；
- cwd volume-GUID `CreateProcessAsUserW` compatibility；
- namespace-fence explicit authorized release、crash recovery、termination-outcome recovery 与 orphan recovery service；
- post-create process-image FileId 及 process/object owner/label/ACL/private desktop query-back；
- pre-resume startup/import loader currentness 与 import-resolution-period authority retention；
- runtime `LoadLibrary`/等价 dynamic module-load enforcement；
- CPU millicore、VRAM、disk、network 与 Sidecar uptime enforcement；
- durable stopped→starting Store transaction 与 commit-unknown recovery。

由于 primary thread 永不 resume，本批不能把 Job/token 称为完整 sandbox 或 signed grant enforcement receipt。未来任一
blocker 未关闭时，都必须保持 process suspended 或终止，不能降级启动。

## 9. 失败与 outcome uncertainty

在 currentness query 前的 input/launch-security/Job/attribute failure 只关闭无 child 的匿名 Job，并保留整份
validated preparation。query definitive rejection/outcome uncertainty 将 preparation 与可选 query-attempt 隔离为
`WindowsRunnerPreCreateLoaderCurrentnessUnusableCustody`，不返回 retry。query 成功后，任何 binding validation、
尤其 post-currentness launch-security live validation、`CreateProcessAsUserW` 或 post-create failure，都保留
`LoaderCurrentWindowsRunnerProcessPreparation` 及重算的
start digest。Job-list 令 successful create 返回的 child 在出现任何 post-create anomaly 前就属于 kill-on-close
Job；不存在未 assigned child 分支，也不得 fallback。

post-create 失败先 `TerminateJobObject`，再以 retained process handle 执行 `TerminateProcess` 兜底；后者不单独构成
release proof。handle contract anomaly 会先在 raw 层识别 NULL/alias，只对每个 distinct returned handle 建立一次 owner。
termination release 只有在 exact 四项同时成立时才 confirmed：`TerminateJobObject` 成功、root process wait 为 signaled、
Job wait 为 signaled、`QueryInformationJobObject(JobObjectBasicAccountingInformation)` 回读
`ActiveProcesses == 0`；任一缺失、超时、失败或非零都保持 unconfirmed。若不能确认，
failure 把 `WindowsRunnerPostCreateCustody`（rollback handles + loader-current preparation）、recovery key 及其内含的
launch security、统一 package+system FileId immutable leases、namespace grants/session 全部放入
`ManuallyDrop<WindowsRunnerUnconfirmedProcessCustody>`；不再分别 Drop Job/process/thread、root-lock lease 或任何 loader owner。
success custody 的 Drop 也采用同一规则：只有确认终止才显式 Drop 整个 `WindowsRunnerLiveProcessCustody`，否则整图
parked。当前没有恢复这些 parked owners、重试确认终止或执行 explicit authorized release 的 backend/service；这是
blocker，而不是 leak-free/recovered 证据。即使终止已确认，in-process Drop 也不等于 persistent namespace grant 的
authorized release。failure 不提供 retry extractor。

PID、path、receipt、candidate health、CLI sidecar record 或 `ComputePluginFetchProcessFence` 都不能恢复 start 权限。

## 10. 不变式与零效果

start material 只绑定 work-admission source/receipt、installation/plugin/slot/release、Plan/grant、Runner path/digest/size/
FileId、`startup_import_resolution_profile_digest`、`startup_import_namespace_authority_digest`、entrypoint argv、
launch token/SD/owner-label/access-check/private-desktop binding digests、live primary-token session/logon namespace 与 exact
desktop access-check binding、resource/permission ceiling、runtime generation before
与 authority/process/clock fences；这些 digests 不代替 retained launch owners 或 query-back。resolution
profile 还必须绑定 token/AppContainer、architecture/WOW64、empty-environment/search policy、cwd 与 creation
flags；pre-create currentness 再绑定 namespace/fence/content-lease/resolution/OS-build/KnownDLL/API-set/SxS/
process-machine/session/grant/query generations、exact attempt/receipt 与 unavailable release policy，并重算 final start
digest。KnownDLL/API-set/SxS 字段只是 echoes；这些 material 不等于 live OS currentness、eventual import resolution 或
resume authority。它同时把以下效果
固定为 `none`：

- runtime phase、runtime generation、health、Ready；
- Provider、route、Offer、Capacity、Execution、Attempt、Lease；
- usage、settlement、money。

本批不改 `lifecycle.rs`、local-authority schema、migration、writer、Ready builder、v14/v15、HTTP/MCP/Wire 或控制
WebSocket。source-lineage 的 local currentness、runtime transition、Host runtime 与 v15 session 四项 gap 全部保持
`missing`。

## 11. 后续顺序

1. 先修正并动态验证 `existing_extraction_directory_access_share_compatibility`，再实现 loader name-grant/FileId-lease/
   query 与 anchor-consuming package-file reopen/recovery；
2. 实现 authenticated exact PE graph、launch-path parent-chain、launch-context-bound startup/import resolution 与 whole
   searched-name namespace producers，完成 Windows TOCTOU/share/import/path matrix；
3. 实现 consuming pre-create loader-currentness backend，验证 definitive/uncertain quarantine 与全 failure/success evidence
   retention；
4. 实现 launch-security producer，创建并保留 exact private window-station/desktop owners，完整 query-back token/
   integrity/SID/privilege/capability、owner/label/account/effective ACL 与 qualified desktop binding，并做 sibling
   reopen/resume/injection 矩阵；
5. 实现 live OS resolution currentness、pre-resume loader-currentness/import-period authority retention、dynamic
   `LoadLibrary` enforcement，以及 process-image/object/private-desktop query-back；
6. 实现 termination-unconfirmed whole-graph recovery 与 namespace-fence explicit authorized release/crash recovery；
7. 冻结 authenticated IPC/bootstrap 与 CPU/VRAM/disk/network/uptime enforcement，形成 durable Store transaction/recovery；
8. 只有上述 blockers 和同一 process custody 全部成功才新增受控 resume，再实现 active health、Ready
   source currentness、v15 session 与 server verifier。

任何后续步骤都不能把本批 source draft 的存在描述为 Host runtime gap 已关闭。
