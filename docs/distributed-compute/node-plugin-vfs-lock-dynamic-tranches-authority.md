---
title: 节点插件 VFS Lock 动态执行切片权威 V1
status: current
reviewed_at: 2026-09-01
owners: node, security
design_status: design_frozen
implementation_status: q5_q6_q7_q8_q9_source_written_uncompiled_unrun
verification_status: source_review_only_actual_not_run
authority_scope: backend-a2-map-lock-dynamic-quotient-authority-v1
---

# Node Plugin VFS Lock Dynamic Tranches Authority V1

## 1. Scope

本文维护 [`Map/Lock dynamic quotient authority`](node-plugin-vfs-map-lock-dynamic-quotient-authority.md)
中 Lock q5–q9 窄执行切片的精确成员、lower 路径 source contract、回执形状和隔离约束。父权威仍唯一维护完整
`8,668` 静态分母、商集冻结、reviewed inventory、`Qlock` 与生产门控；本文不创建第二套
CaseKey、Expected、manifest 或 acceptance 状态。

## 2. q5 native-acquire NodeLive busy

`a2lockq5` 只匹配 44 个 `LockNativeAcquire + NodeLive + native-busy + Completed` acquire
range：8 个 shared 单槽与 36 个 exclusive 连续范围。独立 Win32 handle 持锁跨越 installed
`xShmLock`；receipt 绑定同 FileId/不同 handle、真实 Busy、零状态漂移与 cleanup，拒绝 synthetic
Busy 和 same-handle overlap。

q5 catalog 为 44 rows，SHA-256=
`b12bd411f7fa63f822e65a679351dfc103a6368e2887355d5b03c530fc162e2f`；它仍是
uncompiled/unrun source contract。

## 3. q6 local sibling-contention completed

`LockLocalSiblingContentionCompletedV1` 精确匹配 44 个
`LockLocalState + Completed + BusyNoMutation` member：8 个
`LockShared + SiblingExclusiveContention` 单槽与 36 个
`LockExclusive + SiblingAnyContention` 连续范围；不得解析 `leaf_id` 或吸收 q2–q5。

双连接 sibling 先经 installed `xShmLock` 持锁：shared case 使用同槽 exclusive，exclusive case
逐槽使用 shared；selected 再在真实 coordinator gate 返回 `SQLITE_BUSY`。receipt 要求 selected
零 native call、零 mutation/poison drift、setup/cleanup ledger 隔离，并在 callback 后显式清理
sibling。该批当时的未运行 inventory 预期为 `2,842 present / 5,826 missing`，已被 q7 当前聚合值取代。

## 4. q7 callback-completion route unknown

`LockCallbackCompletionRouteUnknownV1` 精确匹配 192 个 ordinary
`completion=RouteUnknown` member，只含六个子族：

- `LockNativeAcquire + NodeLive + Acquired`：44；
- `LockNativeAcquire + NodeLive + NativeBusy`：44；
- shared-local acquire success：8；
- local sibling contention Busy：44；
- native release success：44；
- shared-local release success：8。

shared 只允许 8 个单槽，exclusive 只允许 36 个合法连续范围。不得吸收 unsafe-retention、
native-error、pre-admission 或其他 terminal；不得改变 Map 或 q1–q6 selector/version 语义。

每个 child 必须先经真实 installed `xShmLock` 路径形成 lower result 与严格 receipt，再由 exact
request/route 绑定的 test-only one-shot seam 调用 production `retain_terminal_custody` 移除该 Lock
route，最后调用真实 `callback.complete()` 并得到 `UnknownOrRetired`。顺序固定为：

```text
real lower result
  -> exact route/request/outcome one-shot claim
  -> production terminal-custody retention and route removal
  -> real callback completion
  -> route-unknown receipt
```

receipt 必须封口 callback begin=`1`、completion attempt=`1`、route removal=`1`、completion
unknown=`1`，并保留相应 native/local/sibling lower receipt。不得注入 completion 结果、不得在 lower
operation 前预删 route、不得以 synthetic native error 代替生产路径。wrong route/request/outcome 不得
消费 claim；exact claim 只能成功一次；Map classifier 固定为 `None`，ordinary `Err` 不进入该 seam。

q7 catalog 精确为 192 data rows、39,203 bytes、LF-only、无 UTF-8 BOM，按 canonical expected-row
顺序逐行绑定 path/action/range 与 case/full seals；static fixture 再将这些 tuple/seals 精确绑定到 frozen
authority leaf。SHA-256=
`e9f509d52d294405dd1a7ae528c514a31ba4e0758959374b633bdca2b571d631`。六组为
上述语义顺序的 `44/44/8/44/44/8`；catalog physical order 为 `44/44/44/44/8/8`，且与
q1–q6 catalog member seals 零重叠。

## 5. q8 local protocol own-overlap or not-held completed

`LockLocalProtocolOwnOverlapOrNotHeldCompletedV1` 的 source matcher 只接受 88 个
`LockLocalState + RequestValidation + ProtocolViolation + Completed` member：

- own-overlap 共 44：`LockShared` 8 个单槽，`LockExclusive` 36 个八槽内非空连续范围；
- shared-not-held 共 8：`UnlockShared` 8 个单槽；
- exclusive-not-held 共 36：`UnlockExclusive` 36 个八槽内非空连续范围。

matcher 必须同时复核 typed source site、prestate、operation、phase、timing、occurrence、完整
Expected、`first/count/mask` 与 committed case/full-record seals，不解析 `leaf_id`。它明确排除 36 个
`ExclusiveRangeMismatch + Completed` member，并排除上述语义的全部 `RouteUnknown` member；任何其他
range-mismatch、route-unknown、q1–q7、native error、stored poison 或 admission rejection 也不得被吸收。

q8 的 child/fixture/ledger/payload 源码只描述下列未来 actual 路径，当前没有执行回执：own-overlap 先由同一
selected WAL-main connection 经 installed `xShmLock` 成功取得 exact range，再在 observation ledger arm 后对
同一 range 发起第二次 installed `xShmLock`；not-held 从已 attach、held masks=`0/0` 的 active selected route
直接发起 installed unlock。production managed-fs guard 分别提供
`NODE_MANAGED_SQLITE_SHM_LOCK_TRANSITION_NOT_UNLOCKED`、
`NODE_MANAGED_SQLITE_SHM_SHARED_UNLOCK_NOT_HELD` 或
`NODE_MANAGED_SQLITE_SHM_EXCLUSIVE_UNLOCK_NOT_HELD` witness，并经 ABI 投影为
`SQLITE_IOERR_SHMLOCK`。selected lower-ledger source contract 要求 managed attempt=`1`、managed
success=`0`、native lock/unlock attempt=`0`、local transition=`0`、before/after masks 不漂移；setup 与
cleanup installed callbacks 只服务 own-overlap，且位于 selected ledger 之外。

source payload 还要求 target callback begin/completion 各一次，callback lease 正常释放，exact route 保持
active、未被 q7 seam 移除，registration/logical route 与 selected connection identity 不漂移；parent 只可从
child exit、canonical payload、environment 与 root cleanup 的闭合形状构造私有候选回执。调用方提交的
Expected、result code、lower counters 或 digest 均不能替代该路径。上述均是 source-written receipt shape，
不构成 actual receipt、Windows record、动态接受或生产开放事实。

q8 catalog 精确为 88 data rows（另 1 header）、15,716 bytes、LF-only、无 UTF-8 BOM，SHA-256=
`8cb3fcef3eb2f65fe54694396cdcff32aef576dc5f299879f5e072699428c936`；其 88 对 member seals 与 q3–q7
既有 2,920 对 member seals 的交集为 0。

## 6. q9 pre-managed callback rejection

`LockPreManagedCallbackRejectionV1` 冻结 528 个 member，且每个 member 都是一个独立 normalized
program group。它是 88 个合法 Lock request 与六个精确终态族的笛卡尔积；88 个 request 固定为
`LockShared` 8 个单槽、`LockExclusive` 36 个连续范围、`UnlockShared` 8 个单槽和
`UnlockExclusive` 36 个连续范围。六族各 88，且只允许下列配对：

| family | source / stimulus | completion | members/groups |
|---|---|---|---:|
| AdmissionRouteUnknown Direct | `RegistryCallbackAdmission / AdmissionRouteUnknown` | `Direct` | 88/88 |
| AdmissionCounterOverflow Direct | `RegistryCallbackAdmission / AdmissionCounterOverflow` | `Direct` | 88/88 |
| UnsupportedFileRole Completed | `AdapterDispatch / UnsupportedFileRole` | `Completed` | 88/88 |
| UnsupportedFileRole RouteUnknown | `AdapterDispatch / UnsupportedFileRole` | `RouteUnknown` | 88/88 |
| ShmDetached Completed | `AdapterDispatch / ShmDetached` | `Completed` | 88/88 |
| ShmDetached RouteUnknown | `AdapterDispatch / ShmDetached` | `RouteUnknown` | 88/88 |

matcher 必须全向量匹配 `root=Lock`、`operation/phase=CallbackAdmission`、`timing=BeforeCall`、
`occurrence=Natural`、`callback=XShmLock`、`fault_seam=RegistryAdmission`、
`observer=LockCallbackAndSnapshot`、`cleanup=ParentOwnedRoot` 和完整 Expected。action、first、count、mask
必须为 exact `Reached` 合法 request，mask 必须由 range 重算；initialization、held/sibling masks 全部为
`NotReached`，completion 只接受上表配对。Expected 的 `lock_effect` 必须按实际到达层级分裂：
`AdmissionRouteUnknown`、`AdmissionCounterOverflow` 两个 admission-direct 族为 `Unchanged`；
`UnsupportedFileRole`、`ShmDetached` 的四个 `AdapterDispatch` 族为 `NotReached`，禁止六族共用
`Unchanged`。catalog 按六个语义 shard 各保存 88 个
`(action, first, count, mask, case_key_sha256, full_record_sha256)` seal；不得用 `leaf_id`、branch 或展示文本分类。
528 个 member seals 与 q1–q8 的 3,122 个 source-present seals 必须零交集，528 个 normalized keys 也必须
唯一且与旧 source-present keys 零交集。

### 6.1 Production actual chain

所有六族的目标入口都是 installed SQLite ABI `xShmLock`，继而进入 registry pinned-file
`shm_lock -> with_shm`；不得直接调用 state、coordinator 或 managed-fs lower 来代替 installed callback。

- `AdmissionRouteUnknown`：fixture 先用 production `retain_terminal_custody` 移除 exact route；随后真实
  `begin_callback` 返回 `UnknownOrRetired`。没有 callback lease，dispatch/lower/completion 均不得到达。
- `AdmissionCounterOverflow`：test-only prime 只能在 exact active route、当前 callback count 为 0 且 shape
  合法时把计数预置为 `u32::MAX`；随后 production `begin_callback` 的真实 `checked_add` 失败、写入
  `CallbackCounterOverflow` terminal reason。prime 不是 actual，actual 是 production rejection。
- `UnsupportedFileRole`：真实 callback admission 成功后，`with_shm` 观察到 actual custody 为 `Main`，由
  production WalMain pattern check 返回 `UnsupportedFileRole`。这里的
  `ManagedWalMainSingleConnection` fixture 枚举只表示 managed single-connection harness class，不表示 actual
  custody；receipt 必须明确封口 `role=Main`，否则这 176 个 member 不得 admission。
- `ShmDetached`：fixture 必须经真实 attach/detach 得到 actual `WalMain` 且 `shm=None`；真实 admission 成功后，
  production `file.shm_mut()` 检查返回 `ShmDetached`。

Completed 两族不得移除 route，必须调用真实 `callback.complete()` 并观察成功。RouteUnknown 两族只能在
真实 `UnsupportedFileRole` 或 `ShmDetached` 已形成后，由 exact route/request/rejection 绑定的一次性 test-only
seam claim，再调用 production `retain_terminal_custody`，最后调用真实 `callback.complete()` 并观察
`UnknownOrRetired`。顺序固定为：

```text
installed xShmLock
  -> production callback admission
  -> production custody-role/shm-present rejection
  -> exact one-shot claim (RouteUnknown only)
  -> production terminal-custody retention and route removal (RouteUnknown only)
  -> production callback completion
  -> private actual receipt
```

operation rejection 在 production `(result, callback.complete())` 中保持优先，公开 SQLite 结果不足以区分内部
rejection 或 completion；因此 receipt 必须独立绑定 exact raw request/result、route/registration、callback lease、
actual role 与 shm-present、真实 rejection、completion result、terminal reason、route removal/retention、零
managed/native lock ledger、child exit 和 parent cleanup。seam 只能观察真实结果和安排上述 one-shot route
removal，不得注入 rejection、completion 或伪造 actual。

### 6.2 Explicit exclusion

3,432 个 native-acquire initialization-failure 静态 member 完全排除在 q9 之外：它们不是 q9 member、group、
catalog、matcher、runner 或 receipt，也不得计入本批 source-present。当前通用 fault controller 对其 exact
full vector 的可命中数为 0；未来只有独立 initialization namespace/native/DMS/cleanup controller 与真实
Windows 回执闭合后，才可另立 tranche。本批不得把 injected generic phase failure 写成 native actual。

## 7. Current evidence and production boundary

q7/q8 的既有 source scope 与 receipt 形状保持不变；q9 又新增六个 exact matcher/catalog shards、installed
callback runner、production-observation seam 和 source-level contracts。q1–q9 未运行 inventory 的 source-only
预期为 `3,650 present members / 3,650 present groups / 5,018 missing members / 4,490 missing groups /
8,668 total members / 8,140 total groups`，且 528 个 q9 group 必须全部为 singleton。没有 current reviewed
source-scope 或 inventory digest，member coverage 仍为 `0/8,668`。

本批把 `with_shm` 的生产实现拆入 `operations/shm.rs`，q1–q9 各 tranche 的 implementation closure 已纳入该
物理源码；但仓库级 `SourceOwnerGraph` 与 source-leaf frozen authority 仍绑定拆分前的物理快照。它们必须在本批
checkpoint 提交后，以新的 baseline 运行显式 ignored candidate generator，并人工复核 16 份 Map leaf、Map
manifest、Lock leaf 与 Lock manifest 共 19 份 frozen artifacts。由于当前架构铺设阶段明确禁止运行 Rust，本批
不留下只改 owner/needle、却没有同步重生成 frozen artifacts 的半套权威；这项全局刷新继续作为后续验收阻塞项。

本批没有运行 Cargo、编译、SQLite、Windows 或真实 runtime；因此仍是
`passed=0 failed=0 actual=not_run`，没有 actual record、reviewed inventory digest、frozen manifest、
`Qlock`（仍为 `unknown`）或 Windows numerator，`WindowsDynamic=not_opened`。最终 Lock 功能继续 blocked：
仍缺 5,018 members / 4,490 groups，且 compile/runtime/actual receipts/reviewed digest 全部缺失。q9 是
source-only、uncompiled、unrun，production 保持 closed。它不打开生产 VFS/open、
Runtime/Ready、Provider、Offer、Job、Attempt、Lease、dispatch、market、settlement 或 funds effects。
