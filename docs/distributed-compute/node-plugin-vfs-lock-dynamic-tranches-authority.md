---
title: 节点插件 VFS Lock 动态执行切片权威 V1
status: current
reviewed_at: 2026-09-02
owners: node, security
design_status: design_frozen
implementation_status: q5_q6_q7_q8_q9_q10_q11_q12_q13_q14_q15_q16_q17_q18_source_written_uncompiled_unrun
verification_status: source_review_only_actual_not_run
authority_scope: backend-a2-map-lock-dynamic-quotient-authority-v1
---

# Node Plugin VFS Lock Dynamic Tranches Authority V1

## 1. Scope

本文维护 [`Map/Lock dynamic quotient authority`](node-plugin-vfs-map-lock-dynamic-quotient-authority.md)
中 Lock q5–q18 current source 与 q19 frozen requirement 的精确成员、lower 路径 source contract、回执形状和
隔离约束。父权威仍唯一维护完整
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

## 7. q10 ABI scalar rejection

`LockAbiScalarRejectionV1` 的 source set 精确为 7 个 member / 7 个 singleton normalized group，来自
`offset_valid/count_valid/flags_valid` 三个 typed validity 轴的 `2^3-1` 个非全真组合。全真组合必须继续进入
后续 raw/registry/managed 路径，不属于 q10；七个非法组合之间不得合并，也不得按 leaf ID、展示文本或枚举位置
分类。

matcher 必须完整绑定 `root=Lock`、`source=LockAbiBoundary`、typed `LockAbi` stimulus、
`prestate=NotReached`、`operation/phase=AbiValidation`、`timing=BeforeCall`、`occurrence=Natural`、
`fixture=AbiRawOnly`、`callback=XShmLock`、`fault_seam=AbiBoundary`、
`observer=LockCallbackAndSnapshot`、除 `completion=Reached(Direct)` 外其余 Lock axes 均为 `NotReached`，并精确复核
Expected 与 frozen case/full-record seals。任一 validity 轴、Expected、source site 或 seal 漂移都必须失败关闭。

冻结成员文件为 `abi_scalar_rejection_members.v1.tsv`，固定 5 列
`offset/count/flags/case_key_sha256/full_record_sha256`、7 行数据、`1114` bytes、LF-only、无 BOM，SHA-256 为
`6458242a0140730d87f019340ceb9bf1a378f1bbac714b9c7982db9b64216280`。七个 case/full-record seal 与 q1–q9
既有成员的交集必须均为零。

未来 actual 只能从真实 installed SQLite ABI `xShmLock` 进入 production scalar gate：
`offset` 转 `u8`、`count` 转非零 `u8`、`flags` 转 exact Lock action，任一转换失败便直接返回
`SQLITE_IOERR_SHMLOCK`。该返回必须发生在 `file_state::run_code`、raw-state admission/dereference、registry
callback admission、managed-fs lower 与 native/local lock 之前；installed methods/state 只是调用前已经存在且保持不变的
夹具，`NotReached` 指其 admission/custody effect 未到达，并不表示方法表或状态从未安装。私有 receipt 因而必须绑定原始三个 scalar、exact validity
组合、返回码、零 callback/managed/native/local attempt、零 route/custody/mask mutation 与 parent cleanup；调用方提交
Expected 或结果码不能构造 actual。

current source 已把该 receipt 形状落实为两份线性、一次性观测账本。ABI 账本在确认 exact installed methods 与
type-erased state identity 后，绑定 live file address、调用线程、原始三元组和单调 observation id，并由 production
`xShmLock` 本身依序写入 `Entry -> ScalarRejected | RunCodeEntered -> Returned`；q10 只接受真实三项 validity、
`Entry=1 / ScalarRejected=1 / RunCodeEntered=0 / Returned=1` 与真实 `SQLITE_IOERR_SHMLOCK`。同一 child 同时为
exact route 武装 `AbiRejected` no-entry ledger；任一 registry `Event::Entry`（包括错 request）都会污染并拒绝消费，
只有完整 18 槽零事件向量可封口。registry entry 是 managed/native/local 的支配前哨，所以不需要、也禁止为了观测而
先创建 SHM target；target before/after 必须都不存在。全局 ABI ledger 只允许一个 active guard，错文件、错线程、
错 tuple、重复/乱序、stale guard、并发占用或重放都只能使 child 失败关闭，不能产生 receipt。

以上只是 7/7 exact singleton 的 source contract；本批没有编译或执行，未生成 actual receipt、Windows record
或 reviewed inventory digest。3,432 个 native-acquire initialization-failure member 与 q9 一样完全排除在 q10
之外，不能以 ABI scalar rejection 代替。

## 8. q11 raw-state rejection

`LockRawStateRejectionV1` 精确承接冻结 Lock authority 中 q10 后的下一段连续边界：两个
`lock.raw.excluded.invalid-*-pointer` leaf 只定义 C memory-safety premise，继续 excluded 且永不进入 runner；
其后的 11 个 `lock.raw.terminal.*` member 全部进入 q11，并逐个形成 singleton normalized group。下一个 frozen
terminal 已由 q9 的 `AdmissionCounterOverflow Direct` catalog 承接，因此 q11 不能向后吸收或跳过成员。

matcher 只能由完整 typed 语义构造 case。十个 raw rejection 必须绑定
`source=RawStateAbandon`、`stimulus=LockRaw(exact RawStateV1)`、`prestate=NotReached`、
`operation=RawAbandon`、`phase=RawAdmission`、`timing=Cleanup`、`fault_seam=RawState`、
`observer=CustodyAndCleanup`；`HandleBoundFileMissing` 单独绑定 `source=AdapterDispatch`、
`operation=AdapterDispatch`、`phase=Adapter`、`timing=BeforeCall`、
`observer=LockCallbackAndSnapshot`。所有 case 还必须精确匹配 `completion=Direct | RawDropCompleted |
RawDropUnwindCaught`、完整 Expected、case/full-record seals 与 `ParentOwnedRoot` cleanup。leaf id、行号、展示文本
和 catalog row position 均不得参与分类。

冻结成员文件为 `raw_state_rejection_members.v1.tsv`，固定五列
`source_site/raw_state/completion/case_key_sha256/full_record_sha256` 与 11 行数据。它覆盖：六个
raw-slot validation direct、两个 payload-missing drop-completed、一个 other-type payload drop-completed、一个
other-type payload drop-unwind-caught 与一个 handle-bound file missing direct。两个 pointer exclusion 没有
selector、case enum 或控制器入口。该文件固定为 `2,092 bytes`，SHA-256 为
`b57a57bec7aa00c29b842c5307a6a5569ecbe251713edb7363e9416cca6d648d`。

future actual source seam 必须进入真实 installed SQLite ABI `xShmLock`：合法固定 scalar 先进入
`file_state::run_code`，再走 production `raw_state::with_installed_state`、失败后的
`abandon_installed_state`、slot clear/retain、envelope/payload Drop 或 adapter file-missing fallback。独立 32 槽
one-shot ledger 必须绑定 live file/null sentinel、线程、exact case、raw slots、validation/type/payload、abandon、
drop completed/unwind、callback return 与 observation id；错文件、错线程、错 case、乱序、重复、并发占用或
未消费 guard 全部失败关闭。同一 exact route 的 `RawRejected` no-entry ledger 必须保持全零，证明 registry、
managed/native/local lower 未被误入。

11 个 raw premise 在安全生产状态机中都不可自然到达，因此只允许 test-only、Windows、memory-safe 的受控
unsafe fixture 构造；corrupt/retained state 不得正常 close，只能由隔离 child 退出形成终止边界。未来真实运行
所得证据也只能标记 `controlled_fault_actual`：它证明真实 production callback/control/cleanup 在 synthetic
premise 下的行为，不是自然生产可达、普通 runtime coverage 或用户现场 actual。current source 仍未编译、
未运行，没有 child receipt、Windows record 或 coverage 增量。

## 9. q12 CreatedFirst DMS exclusive-release outcome uncertain

`LockNativeAcquireCreatedFirstExclusiveReleaseErrorV1` 是 3,432 个 native-acquire initialization-failure
member 的首个冻结连续纵切，不是完整 initialization umbrella 的改名。它精确包含 88 个 member / 88 个
singleton normalized group：8 个 `LockShared` 单槽和 36 个 `LockExclusive` 八槽内非空连续 range，分别配对
两个 unsafe terminal completion：

- `retention.succeeded.terminal.route-unknown`；
- `retention.route-unknown-prior-quarantine.terminal.route-unknown`。

matcher 必须全向量匹配 `source=LockNativeAcquire`、合法 `first/count/mask`、CreatedFirst initialization、
`phase/fault_site=DmsExclusiveRelease`、`timing=AtCall`、`occurrence=Natural`、
`class=OutcomeUncertainPoisoned`、`cleanup_rewrite=false`、file retained、mutation uncertain、
lock uncertain、DMS `ExclusiveOutcomeUncertain`、DMS native lock/unlock=`1/1` 与完整 Expected/seals。
两个 terminal 只允许由 typed completion 区分；leaf id、显示文本、catalog 行号和 case-salted digest 不得参与
分类。current q12 catalog 与 q1–q11 的 3,668 个 source-present seal 零交集，且 88 个 normalized key 全部唯一。

未来 controlled-fault actual 的顺序固定为：fresh private root、cold WAL-main attach 且 coordinator node 与 SHM
file 都不存在；installed `xShmLock` 进入 production managed lock；真实 sibling open 得到 Created；真实 DMS
exclusive `LockFileEx` 成功；真实 truncate(0) 成功；dedicated one-shot controller 在 production
exclusive-release 点调用一次真实 `UnlockFileEx`，但故意不读取 BOOL，形成
`ReturnReceiptUnavailable`；coordinator 安装 `ExclusiveOutcomeUncertain` 并 poison/retain，registry 再走
production unsafe quarantine。第二个 completion 只额外复用 exact route/request/outcome 绑定的 q3/q4 route
preemption，不得注入 initialization failure 或 callback completion 结果。

initialization receipt 必须绑定 exact target/thread/request/case/stage、cold prestate、ordered open/DMS/truncate/
release events、DMS native=`1/1`、poison/custody、pending=`0` 与 consumed/finished。requested Lock range 的
one-shot ledger 必须独立封口 managed attempt=`1`、managed success=`0`、native lock/unlock/local=`0`；DMS
字节的 native `1/1` 永不计入请求 range。错 target/thread/request/case/stage、重复或乱序、未消费 receipt、
额外 lower attempt 或 pending 非零全部失败关闭。poisoned handle 不得正常 close/unlock；child 必须保留它直到
进程退出，parent 只能在确认 child exit 后删除 private root。所有未来 payload 只能写
`controlled_fault_actual`，不得写 natural actual。

完整 `LockNativeAcquireInitializationFailureV1` 仍包含 39 个 base shape × 44 request × 2 terminal =
3,432 members，并按 cleanup rewrite 归一为 2,904 groups（2,464 singleton、352 size-2、88 size-3）。机械
分期保持 Open `792/792`、DMS exclusive acquire `528/440`、first-process truncate/release `528/528`、shared
acquire `1,584/1,144`；q12 首段后仍有 3,344 members / 2,816 groups 的 initialization namespace
planned-missing。首段不得推导 umbrella、reviewed inventory、`Qlock`、Windows numerator 或生产许可。

## 10. q13 ExistingFirst DMS exclusive-release outcome uncertain

`LockNativeAcquireExistingFirstExclusiveReleaseErrorV1` 是紧邻 q12 的第二个 initialization
vertical slice，不是 q12 别名，也不扩张完整 umbrella。它精确包含 88 个 member / 88 个
singleton normalized group：8 个 `LockShared` 单槽和 36 个 `LockExclusive` 八槽内非空连续 range，
分别配对与 q12 相同的两个 unsafe terminal completion：

- `retention.succeeded.terminal.route-unknown`；
- `retention.route-unknown-prior-quarantine.terminal.route-unknown`。

matcher 必须全向量匹配 `source=LockNativeAcquire`、合法 `first/count/mask`、ExistingFirst initialization、
`phase/fault_site=DmsExclusiveRelease`、`timing=AtCall`、`occurrence=Natural`、
`class=OutcomeUncertainPoisoned`、`cleanup_rewrite=false`、file retained、mutation uncertain、
lock uncertain、DMS `ExclusiveOutcomeUncertain`、DMS native lock/unlock=`1/1` 与完整 Expected/seals。
`occurrence=Natural` 只是冻结静态 descriptor 轴；未来为得到可观察的返回值不确定证据而启用的 one-shot
controller 仍是受控故障缝，所得证据只能标记 `controlled_fault_actual`，绝不得写为 natural actual。

future controlled-fault actual 的前置顺序固定为：fresh private root 的 exact runtime/namespace 上先通过 test-only typed helper
创建并关闭物理 SHM，precreation receipt 必须证明 `was_created=true`、file/close kind 均为 SHM 且
identity digest 存在；再从 cold WAL-main attach 进入 production managed lock，并由 initialization controller 绑定
`was_created=false` 的 ExistingFirst open observation。随后顺序与 q12 同类型但不共用 case/receipt：真实 DMS
exclusive `LockFileEx` 成功，真实 truncate(0) 成功，production exclusive-release 点一次调用
`UnlockFileEx` 且故意不读 BOOL，安装 `ExclusiveOutcomeUncertain`，再 poison/retain 并进入 production
unsafe quarantine。第二 completion 只可复用 exact route/request/outcome 绑定的 q3/q4 route preemption。

q13 receipt 必须复用与 q12 同型但 case-bound 的 exact target/thread/request/case/stage、ordered DMS/truncate/release、
DMS native=`1/1`、poison/custody、requested-range no-entry ledger、pending=`0` 与 consumed/finished，并额外绑定物理预创建
receipt 和 ExistingFirst open observation。它必须拒绝 CreatedFirst、缺失/无效/未关闭的 precreation receipt、
错序、重复、额外 lower attempt 或任何 receipt/case swap。poisoned handle 不得正常
close/unlock；child 必须保留它直到进程退出，parent 只能在确认 child exit 后删除 private root。

q13 catalog 精确为 88 rows / 18,474 bytes，SHA-256=
`03b883842b3fd1886779dcb07573521d14ca3125894b38c4d341a462137424f4`；88 个 case/full digest 各自唯一，与
q1–q12 的 3,756 个 source-present seal 零交集。q12/q13 共承接 176 members / 176 groups 后，
完整 initialization umbrella 仍有 3,256 members / 2,728 groups planned-missing。这一静态缩减不产生
reviewed inventory、`Qlock`、coverage、Windows numerator 或生产许可。

## 11. q14 CreatedFirst DMS truncate outcome uncertain, release succeeded

`LockNativeAcquireCreatedFirstTruncateErrorReleaseSucceededV1` 是紧邻 q12/q13 的第三个 initialization
vertical slice，不是既有 release-error slice 的别名，也不扩张完整 umbrella。它精确包含 88 个 member /
88 个 singleton normalized group：8 个 `LockShared` 单槽和 36 个 `LockExclusive` 八槽内非空连续 range，
分别配对与 q12/q13 相同的两个 unsafe terminal completion：

- `retention.succeeded.terminal.route-unknown`；
- `retention.route-unknown-prior-quarantine.terminal.route-unknown`。

matcher 必须全向量匹配 `source=LockNativeAcquire`、合法 `first/count/mask`、CreatedFirst initialization、
`phase/fault_site=DmsTruncate`、`timing=AtCall`、`occurrence=Natural`、
`class=OutcomeUncertainPoisoned`、`cleanup_rewrite=false`、file retained、mutation uncertain、
lock certain、DMS `Released`、DMS native lock/unlock=`1/1`、release success observed 与完整 Expected/seals。
`occurrence=Natural` 只冻结静态 descriptor 轴；未来 one-shot truncate controller 形成的证据仍只能写入
`controlled_fault_actual`，不得写成 natural actual。

future controlled-fault actual 的顺序固定为：fresh private root、cold WAL-main attach 且 coordinator node 与 SHM
file 都不存在；production managed open 得到 CreatedFirst；真实 DMS exclusive `LockFileEx` 成功；dedicated
one-shot controller 在同一 production truncate validation 点调用一次真实 `File::set_len(0)`，故意不读取其
`Result`/return receipt，再形成 typed `ReturnReceiptUnavailable`；normal production exclusive release 随后真实
`UnlockFileEx` 成功且 receipt 必须观察该成功；coordinator 安装 DMS `Released`，再以 mutation
uncertain、lock certain 状态 poison/retain 并进入 production unsafe quarantine。第二 completion 只可复用
exact route/request/outcome 绑定的 q3/q4 route preemption，不得注入 initialization failure 或 callback
completion 结果。

q14 receipt 必须绑定 exact target/thread/request/case/stage、cold prestate、CreatedFirst open、ordered
DMS-lock/truncate-error/release-success events、DMS native=`1/1`、DMS `Released`、poison/custody、requested-range
no-entry ledger、pending=`0` 与 consumed/finished。它必须拒绝 ExistingFirst、错序、重复、未观察 release success、
额外 lower attempt 或任何 receipt/case swap。poisoned handle 不得正常 close/unlock；child 必须保留它直到进程
退出，parent 只能在确认 child exit 后删除 private root。

q14 catalog 精确为 88 rows / 17,594 bytes，SHA-256=
`95f1c8e40da35ac23cc46e20310178c2aef09adacf777155d9b67e5802d69abd`；88 个 case/full digest 各自唯一，与
q1–q13 的 3,844 个 source-present seal 零交集。q12/q13/q14 共承接 264 members / 264 groups 后，
完整 initialization umbrella 仍有 3,168 members / 2,640 groups planned-missing。这一静态缩减不产生
reviewed inventory、`Qlock`、coverage、Windows numerator 或生产许可。

## 12. q15 ExistingFirst DMS truncate outcome uncertain, release succeeded

`LockNativeAcquireExistingFirstTruncateErrorReleaseSucceededV1` 是 q14 的 ExistingFirst 对切和第四个
initialization vertical slice，不是 q13 release-error 或 q14 CreatedFirst 的别名。它精确包含 88 个 member /
88 个 singleton normalized group：8 个 `LockShared` 单槽和 36 个 `LockExclusive` 八槽内非空连续 range，
分别配对同一两个 unsafe terminal completion：

- `retention.succeeded.terminal.route-unknown`；
- `retention.route-unknown-prior-quarantine.terminal.route-unknown`。

matcher 必须全向量匹配 `source=LockNativeAcquire`、合法 `first/count/mask`、ExistingFirst initialization、
`phase/fault_site=DmsTruncate`、`timing=AtCall`、`occurrence=Natural`、
`class=OutcomeUncertainPoisoned`、`cleanup_rewrite=false`、file retained、mutation uncertain、
lock certain、DMS `Released`、DMS native lock/unlock=`1/1`、release success observed 与完整 Expected/seals。
`occurrence=Natural` 仍只冻结静态 descriptor 轴；未来 one-shot truncate controller 的证据只能写入
`controlled_fault_actual`，不得写成 natural actual。

future controlled-fault actual 的前置顺序固定为：fresh private root 的 exact runtime/namespace 上先通过 test-only typed
helper 创建并关闭物理 SHM，precreation receipt 必须证明 `was_created=true`、file/close kind 均为 SHM 且 identity
digest 存在；再从 cold WAL-main attach 进入 production managed open 并观察 `was_created=false` 的 ExistingFirst。
随后真实 DMS exclusive `LockFileEx` 成功；dedicated one-shot controller 在同一 production truncate validation 点
调用一次真实 `File::set_len(0)`，故意不读取 `Result`/return receipt，形成 typed
`ReturnReceiptUnavailable`；normal production exclusive release 再真实 `UnlockFileEx` 成功且 receipt 必须观察该
成功；coordinator 安装 DMS `Released`，以 mutation uncertain、lock certain 状态 poison/retain 并进入 production
unsafe quarantine。第二 completion 只可复用 exact route/request/outcome 绑定的 q3/q4 route preemption。

q15 receipt 必须绑定 exact target/thread/request/case/stage、物理预创建 receipt、ExistingFirst open observation、
ordered DMS-lock/truncate-error/release-success events、DMS native=`1/1`、DMS `Released`、poison/custody、
requested-range no-entry ledger、pending=`0` 与 consumed/finished。它必须拒绝 CreatedFirst、缺失/无效/未关闭的
precreation receipt、错序、重复、未观察 release success、额外 lower attempt 或任何 receipt/case swap。poisoned
handle 不得正常 close/unlock；child 必须保留它直到进程退出，parent 只能在确认 child exit 后删除 private root。

q15 catalog 精确为 88 rows / 17,682 bytes，SHA-256=
`d8ac9985f433b9fbd089a8804136f107cf37bd21566b5852611e98674f52911b`；88 个 case/full digest 各自唯一，与
q1–q14 的 3,932 个 source-present seal 零交集。q12–q15 共承接 352 members / 352 groups 后，完整
initialization umbrella 仍有 3,080 members / 2,552 groups planned-missing。这一静态缩减不产生 reviewed
inventory、`Qlock`、coverage、Windows numerator 或生产许可。

## 13. q16 CreatedFirst DMS truncate outcome uncertain, release failed

`LockNativeAcquireCreatedFirstTruncateErrorReleaseFailedV1` 是 q14 的 CreatedFirst 路径上紧邻 release-success
纵切的第五个 initialization vertical slice，不是 q12 direct release-error 或 q14 release-success 的别名。它精确
包含 88 个 member / 88 个 singleton normalized group：8 个 `LockShared` 单槽和 36 个 `LockExclusive`
八槽内非空连续 range，分别配对同一两个 unsafe terminal completion：

- `retention.succeeded.terminal.route-unknown`；
- `retention.route-unknown-prior-quarantine.terminal.route-unknown`。

matcher 必须全向量匹配 `source=LockNativeAcquire`、合法 `first/count/mask`、CreatedFirst initialization、
`phase/fault_site=DmsExclusiveRelease`、`timing=Cleanup`、`occurrence=Natural`、
`class=OutcomeUncertainPoisoned`、`cleanup_rewrite=false`、terminal disposition=`CleanupRewritten`、file retained、
mutation uncertain、lock uncertain、DMS `ExclusiveOutcomeUncertain`、DMS native lock/unlock=`1/1` 与完整
Expected/seals。`cleanup_rewrite=true` 属于后续 file-close failure 重写，绝不能被本 program 吸收；
`occurrence=Natural` 仍只冻结静态 descriptor 轴，未来 two-stage one-shot controller 的证据只能写入
`controlled_fault_actual`，不得写成 natural actual。

future controlled-fault actual 的顺序固定为：fresh private root、cold WAL-main attach 且 coordinator node 与 SHM
file 都不存在；production managed open 得到 CreatedFirst；真实 DMS exclusive `LockFileEx` 成功；dedicated
one-shot truncate controller 在同一 production truncate validation 点调用一次真实 `File::set_len(0)`，故意不读
其 `Result` 并形成第一份 typed `ReturnReceiptUnavailable`；随后 dedicated cleanup-release controller 在同一 case
的 cleanup exclusive-release 点调用一次真实 `UnlockFileEx`，故意不读 BOOL 并形成第二份 typed
`ReturnReceiptUnavailable`。coordinator 必须安装 DMS `ExclusiveOutcomeUncertain`，再以 mutation/lock 均 uncertain
状态 poison/retain 并进入 production unsafe quarantine。第二 completion 只可复用 exact route/request/outcome
绑定的 q3/q4 route preemption，不得注入 initialization failure 或 callback completion 结果。

q16 receipt 必须绑定 exact target/thread/request/case/stage、cold prestate、CreatedFirst open、ordered
DMS-lock/truncate-result-unavailable/cleanup-release-bool-unavailable events、DMS native=`1/1`、DMS
`ExclusiveOutcomeUncertain`、terminal `CleanupRewritten`、poison/custody、requested-range no-entry ledger、
pending=`0` 与 consumed/finished。它必须拒绝 ExistingFirst、`cleanup_rewrite=true`、双回执缺失/换序/重复、
额外 lower attempt 或任何 receipt/case swap。poisoned handle 不得正常 close/unlock；child 必须保留它直到进程
退出，parent 只能在确认 child exit 后删除 private root。

q16 catalog 精确为 88 rows / 18,386 bytes，SHA-256=
`f51b7d0df30afda4a8844ee4c4e227a6cf757b2f7b0c16e0bc0e8dbfaad05718`；88 个 case/full digest 各自唯一，与
q1–q15 的 4,020 个 source-present seal 零交集。q12–q16 共承接 440 members / 440 groups 后，完整
initialization umbrella 仍有 2,992 members / 2,464 groups planned-missing。这一静态缩减不产生 reviewed
inventory、`Qlock`、coverage、Windows numerator 或生产许可。

## 14. q17 ExistingFirst DMS truncate outcome uncertain, release failed

`LockNativeAcquireExistingFirstTruncateErrorReleaseFailedV1` 是 q16 的 ExistingFirst 对切和第六个
initialization vertical slice，不是 q13 release-error、q15 release-succeeded 或 q16 CreatedFirst 的别名。它只承接
`dms.existing-first.truncate-error.release-failed` 的 88 个 member / 88 个 singleton normalized group：8 个
`LockShared` 单槽和 36 个 `LockExclusive` 八槽内非空连续 range，分别配对同一两个 unsafe terminal completion：

- `retention.succeeded.terminal.route-unknown`；
- `retention.route-unknown-prior-quarantine.terminal.route-unknown`。

因此 shared/exclusive 分量精确为 `16+72`，两个 completion 各 `44/44`。matcher 必须全向量匹配
`source=LockNativeAcquire`、合法 `first/count/mask`、ExistingFirst initialization、
`phase/fault_site=DmsExclusiveRelease`、`timing=Cleanup`、`occurrence=Natural`、
`class=OutcomeUncertainPoisoned`、`cleanup_rewrite=false`、terminal disposition=`CleanupRewritten`、file retained、
mutation/lock uncertain、DMS=`ExclusiveOutcomeUncertain`、DMS native lock/unlock=`1/1` 与完整 Expected/seals。
CreatedFirst、`cleanup_rewrite=true`、file-close rewrite 或任何 tuple/case swap 都不得被 q17 吸收。

future controlled-fault actual 的前置顺序固定为：fresh private root 的 exact runtime/namespace 上先由 test-only
typed helper 物理创建并关闭 SHM；precreation receipt 必须证明 `was_created=true`、file/close kind 均为 SHM 且
identity digest 存在。随后从 cold WAL-main attach 进入 production managed open，并观察
`was_created=false` 的 ExistingFirst；真实 DMS exclusive `LockFileEx` 成功后，dedicated truncate controller 在
production truncate validation 点调用一次真实 `File::set_len(0)` 且故意不读 `Result`，形成第一份 typed
`ReturnReceiptUnavailable`；dedicated cleanup-release controller 再在同一 case/stage 调用一次真实
`UnlockFileEx` 且故意不读 BOOL，形成第二份 unavailable receipt。coordinator 必须安装 DMS
`ExclusiveOutcomeUncertain`，以 mutation/lock uncertain 状态 poison/retain 并进入 production unsafe quarantine。

q17 receipt 必须绑定 exact target/thread/request/case/stage、物理 precreation、cold ExistingFirst observation、
ordered DMS-lock/truncate-result-unavailable/cleanup-release-bool-unavailable、DMS native=`1/1`、terminal
`CleanupRewritten`、poison/custody、requested-range no-entry ledger、pending=`0` 与 consumed/finished。它必须拒绝
缺失/无效/未关闭的 precreation receipt、CreatedFirst、双回执换序/缺失/重复、`cleanup_rewrite=true`、额外 lower
attempt 或任何 receipt/case swap。poisoned handle 不得正常 close/unlock；child 必须保留文件直到退出，parent 只能
在确认 child exit 后删除 private root。`occurrence=Natural` 仍只冻结静态 descriptor 轴；未来证据只能写
`controlled_fault_actual`，不得写成 natural actual。

`a2lockq17` payload 预期为 172 values，即在 q16 有序双回执形状上再绑定 8 项 physical-precreation 值。
q17 catalog 精确为 88 rows / 18,474 bytes，SHA-256=
`5ce129843d33b279c9ec70dd282d59cc79455c8f1a1b652718bb04b72777adff`；88 个 case/full digest 各自唯一，与
q1–q16 的 4,108 个 source-present seal 零交集。q12–q17 共承接 528 members / 528 groups 后，完整 initialization
umbrella 仍有 2,904 members / 2,376 groups planned-missing。这一静态缩减不产生 reviewed inventory、`Qlock`、
coverage、Windows numerator 或生产许可。

## 15. q18/q19 DMS shared busy, close succeeded

CreatedFirst q18 current source 与 ExistingFirst q19 frozen requirement 的完整 selector、真实 same-FileId
distinct-handle contention、target close、分账 ledger、wire、catalog 和排除边界统一由
[`Lock DMS shared-busy tranches authority`](node-plugin-vfs-lock-shared-busy-tranches-authority.md) 维护。

q18 当前精确承接 `dms.created-first.shared-busy.close-ok` 的 88/88 singleton，协议 `a2lockq18`=186，
catalog=18,122 bytes/SHA-256 `4f78ff1678c93b1c06bad92e838423e4202598fd8e0b5b83f79cde0c528a07cd`。
q19 已冻结为 `LockNativeAcquireExistingFirstSharedBusyCloseSucceededV1`、
`dms.existing-first.shared-busy.close-ok`、`a2lockq19`=194 与 88-row/18,210-byte catalog target，但源码尚未开始；
只有 q19 source inventory 和 evidence 真正落位后才可把计划聚合写成 present `4,372/4,372`、missing
`4,296/3,768`、q12–q19=`704/704`、initialization remaining=`2,728/2,200`。当前事实仍停在 q18。

## 16. Current evidence and production boundary

q7–q10 的既有 source scope 与 receipt 形状保持不变；q11 的 raw-state 11 个 exact singleton、production
raw admission/abandon/drop ledger、受控 fixture、isolated child/runner/payload 与 source-level contracts 继续存在。
q12 current source 已闭合 typed matcher/catalog、dedicated initialization controller、production one-shot
`UnlockFileEx` seam、requested-range no-entry ledger、isolated child/runner/payload 与 source-scope 接线。catalog
精确为 88 rows / 18,386 bytes，SHA-256=`51d675ee9b2fe990b71a924a6f7cf016c6738e7f88872449f91a20ba6d2566df`；
与 frozen 88-member 目标逐 `(case_key_sha256, full_record_sha256)` 相等，88 个 case/full digest 各自唯一。
q13 current source 又闭合 ExistingFirst-only matcher/catalog、typed physical-precreation receipt、case-aware initialization
controller、同一 production one-shot release seam、requested-range no-entry ledger、isolated child/runner/payload 与
source-scope 接线；其 88-row catalog 与上述 18,474-byte/SHA-256 目标精确一致。q14 current source 再闭合
CreatedFirst-only matcher/catalog、dedicated truncate controller、production DMS truncate one-shot seam、release-success
observation、requested-range no-entry ledger、isolated child/runner/payload 与 source-scope 接线；其 88-row catalog
与上述 17,594-byte/SHA-256 目标精确一致。q15 current source 进一步闭合 ExistingFirst-only matcher/catalog、
typed physical-precreation receipt、case-aware truncate controller、同一 production DMS truncate one-shot seam、
release-success observation、requested-range no-entry ledger、isolated child/runner/payload 与 source-scope 接线；其
88-row catalog 与上述 17,682-byte/SHA-256 目标精确一致。q16 current source 再闭合 CreatedFirst-only
matcher/catalog、two-stage truncate/cleanup-release controllers、production DMS truncate 与 cleanup-release one-shot
seams、双 unavailable receipt、requested-range no-entry ledger、isolated child/runner/payload 与 source-scope 接线；其
88-row catalog 与上述 18,386-byte/SHA-256 目标精确一致。q17 current source 进一步闭合 ExistingFirst-only
matcher/catalog、typed physical-precreation receipt、case-aware two-stage controllers、cold `was_created=false`
observation、production DMS truncate/cleanup-release one-shot seams、有序双 unavailable receipt、requested-range
no-entry ledger、isolated child/runner/payload 与 source-scope 接线；其 88-row catalog 与上述 18,474-byte/SHA-256
目标精确一致。q18 current source 再闭合 CreatedFirst-only shared-busy matcher/catalog、same-FileId distinct
holder/target contention source seam、分账 native ledger、explicit target close-success receipt、requested-range
no-entry ledger、isolated child/runner/payload 与 source-scope 接线；其 88-row catalog 与上述
18,122-byte/SHA-256 目标精确一致。这些都是未编译、未运行的 source contract，不是 current actual。

因此 q1–q18 未运行 inventory 的 source-only 预期为 `4,284 present members / 4,284 present groups /
4,384 missing members / 3,856 missing groups / 8,668 total members / 8,140 total groups`，且 q9 的 528 个、
q10 的 7 个、q11 的 11 个与 q12–q18 各 88 个 group 都是 singleton。完整 initialization umbrella 仍有
2,816 members / 2,288 groups planned-missing；没有 current reviewed inventory digest，member coverage 仍为
`0/8,668`。

q9 曾把 `with_shm` 的生产实现拆入 `operations/shm.rs`；q10 更新 ABI scalar gate observation，q11 新增
真实 raw-state rejection/cleanup observation source，q12/q13 又新增 CreatedFirst/ExistingFirst managed initialization 与 Windows release seam，
q14/q15 再新增 CreatedFirst/ExistingFirst truncate-error/release-succeeded seams，q16/q17 新增
CreatedFirst/ExistingFirst truncate-error/cleanup-release-failed 有序双 unavailable receipt seams；q17 另绑定
physical precreation 与 cold ExistingFirst observation，q18 新增 CreatedFirst shared-busy/close-success 的真实
same-FileId distinct-handle contention source seam 与 holder/target 分账。
仓库级 `SourceOwnerGraph` 与 source-leaf frozen authority 仍绑定此前物理快照。q9–q14 checkpoint 后本应以新
baseline 运行显式 ignored candidate generator，并人工复核 16 份 Map leaf、Map manifest、Lock leaf 与 Lock manifest
共 19 份 frozen artifacts；该 refresh 现已逾期并继续独立 deferred，q15–q19 均不做或替代它。未来编译或 runtime
acceptance 前的重生成/人工复核必须同时覆盖 q15–q19，禁止只改 owner/needle、却没有同步重生成 frozen artifacts 的
半套权威。

本批没有运行 Cargo、编译、SQLite、Windows 或真实 runtime；因此仍是
`passed=0 failed=0 actual=not_run`，没有 actual record、reviewed inventory digest、frozen manifest、
`Qlock`（仍为 `unknown`）或 Windows numerator，`WindowsDynamic=not_opened`。最终 Lock 功能继续 blocked：
仍缺 4,384 members / 3,856 groups，且 compile/runtime/actual receipts/reviewed digest 全部缺失。q11 是
11/11、q12–q18 各是 88/88 exact singleton source-only；八者均 uncompiled/unrun，`controlled_fault_actual` 仍只是
未运行 source seam，production 保持 closed。它不打开生产 VFS/open、
Runtime/Ready、Provider、Offer、Job、Attempt、Lease、dispatch、market、settlement 或 funds effects。
