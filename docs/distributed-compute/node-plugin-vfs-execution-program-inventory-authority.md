---
title: 节点插件 VFS 执行程序清单与准入权威 V1
status: current
reviewed_at: 2026-09-01
owners: node, security
design_status: design_frozen
implementation_status: map_q4_lock_q15_source_written_uncompiled_unrun
verification_status: source_review_only_current_not_run
authority_scope: backend-a2-map-lock-dynamic-quotient-authority-v1
---

# Node Plugin VFS Execution Program Inventory Authority V1

## 1. Scope

本文从 [`Map/Lock dynamic quotient authority`](node-plugin-vfs-map-lock-dynamic-quotient-authority.md)
按职责拆出 bounded execution programs、pre-manifest program inventory 与 reviewed source-program
admission。父权威继续唯一维护动态 class laws、manifest、exact partition、`Qmap/Qlock`、Windows
evidence 与生产门控；本文不创建第二套 static denominator、class catalog、acceptance 或生产许可。
任何 source-present 状态仍只是非授权源码完整性，不能推导 actual execution、Windows record 或生产开放。

## 2. `MapSingleRegionLifecycleV1` bounded tranche

前序 Map source-program 只允许一个版本化的六成员正向纵切。它必须通过真实安装的 `xShmMap`、
same-target pre/post snapshot、受管 selected-region identity、一次性 append-only Map ledger、进程隔离 child
和 parent-owned cleanup 形成私有 receipt；不得仅凭静态 descriptor、输出槽非空或拓扑差值合成 actual。
六个 exact frozen member 固定为：

| Case | Frozen leaf | `(case_key_sha256, full_record_sha256)` | Exact setup / target |
|---|---|---|---|
| Empty Observe | `map.observe.managed.initialization.success.created-first-shared.post-init.regions-empty.region-size-unset.observe-not-present.projection.terminal.success` | `a44f8f31f8f4092841f57c4e3586be10c9cc05ff25b339208f666165598d7b4f`, `424691056ffbe9dfebe62bb074cdd7efae59e835a4c7a66d1f9834ab0c5f2f70` | fresh root；`region=0,size=32768,extend=0`；`NotPresent` |
| Empty Extend | `map.extend.managed.initialization.success.created-first-shared.post-init.regions-empty.region-size-unset.extend-grow.succeeded.region-loop.ordinal-001.target.projection.terminal.success` | `defda99fb645966594b8533a3f5adac34b8eb839e6e4820763625f32eb662be1`, `a55b7b0dc96e4976c4d572ddd4df9dc70b636e8139eba007963df68e9ae40c92` | fresh root；`region=0,size=32768,extend=1`；new region 0 |
| Reuse Observe | `map.observe.managed.initialization.success.node-live.post-init.target-reusable.region-size-same.size-sufficient.reuse.projection.terminal.success` | `7ae2948267174f5ea61d989a0df291357491c8a33d5492d3f131873fe4efd084`, `299cf22640bb8ec1a223d02d43c661c7520ea6afce807c624973f859f1f9e7f5` | setup region 0 Extend，随后 arm；`region=0,extend=0`；reuse region 0 |
| Reuse Extend | `map.extend.managed.initialization.success.node-live.post-init.target-reusable.region-size-same.size-sufficient.reuse.projection.terminal.success` | `5059658ac68f5bc70f462f47858aa569832b0b9a16054eb48994b51441ba5e6b`, `badec397dcf34c9acaaba97307f9af780614da52e1424a44104b01a59dd622b2` | setup region 0 Extend，随后 arm；`region=0,extend=1`；reuse region 0 |
| TargetMissing Observe | `map.observe.managed.initialization.success.node-live.post-init.regions-nonempty-target-missing.region-size-same.observe-not-present.projection.terminal.success` | `fed169f7187f449ed5f1214f6774390a3e347f50fb23d83869b2f14659f9d13c`, `16aa376b8d866079a1977650b649fe9f3759bbf880c958f71f03e2f7d0e6efc5` | setup region 0 Extend，随后 arm；`region=1,extend=0`；`NotPresent` |
| TargetMissing Extend | `map.extend.managed.initialization.success.node-live.post-init.regions-nonempty-target-missing.region-size-same.extend-grow.succeeded.region-loop.ordinal-001.target.projection.terminal.success` | `9d9029a3b76ba38a64ef8d10325c66ec1555ddcf56003389ee2e9bd649964398`, `124e6ed42b0714c8bfa22b7b2122329a21656421e11354112bcabf878b48c53a` | setup region 0 Extend，随后 arm；`region=1,extend=1`；new region 1 |

setup 必须在 ledger arm 前完成，不能污染 target receipt。每例恰好接受一次 callback begin/complete 和一次
target action；ledger 必须交叉绑定 runtime generation、SHM connection、request、prestate path、managed
outcome 与 selected-region identity。`NotPresent` 必须证明 output 由 sentinel 真正清空；mapped outcome 必须证明
output 与 lower receipt 的 selected-region pointer、length、region 和 generation 一致，而不是只证明“非空”。
finish/cancel 必须消费并 disarm exact target，错误 target/request/path、重复、缺失、额外或未完成序列全部失败关闭。

本纵切明确排除 fault、`MappingClose`、`PlatformUnsupported`、unsafe retention、callback completion rejection、
ordinal `>1` 和其他 initialization profile。完成源码接线后，未运行 inventory 的预期只能从 Map
`6 source-present / 43,470 planned-missing` 变为 `12 source-present / 43,464 planned-missing`；它仍不产生
reviewed inventory digest、quotient manifest、`Qmap`、member coverage、Windows record 或生产 permit。

`MapRegionLoopSuccessV1` 在该 q3 之上使用独立 `a2mapq4` 协议，只允许 Created-first empty Extend
`regions_to_create=1..256` 与 Node-live target-missing Extend `regions_to_create=1..255`；分别要求
`target_region=n-1` 与 `target_region=n`，并绑定 `occurrence=ordinal=regions_to_create=n`。q4 lower ledger 必须按
`MappingCreate -> ViewMap -> Record` 精确重复 N 次，拒绝乱序、交错、少报、额外事件和越界；typed matcher
必须同时复核完整 profile/Expected，511 行 catalog 逐项绑定 `(case_key_sha256, full_record_sha256)`，implementation
digest 必须包含 q4 runner、payload、ledger、catalog 与直接 production source scope。该族语义覆盖 511 frozen
members，但 q3 已有两项重叠，故 inventory 净新增 509，变为 `521 source-present / 42,955 planned-missing`。
它仍只是源码与静态证据；编译、测试、Windows execution 均为 `not_run`。

## 3. Lock stored-poison paired bounded tranches

两个 Lock source-program 都只允许 coordinator stored-poison prestate 在 `BeforeCall/Natural` 进入 quarantine。每个 completion
都由 15 个已冻结 poison profile 乘 88 个合法 action/range 形状构成 1,320 members；shared lock/unlock
只允许单槽，exclusive lock/unlock 允许八槽内 36 个非空连续 range。两族只以
`UnsafeRetentionSucceededThenRouteUnknown` 与 `UnsafeRetentionRouteUnknownThenRouteUnknown` 两个 typed completion 区分，成员集不交。

`a2lockq3` 私有 receipt 源码要求真实 installed `xShmLock` 在 poison 被精确注入后返回
`SQLITE_IOERR_SHMLOCK`，poisoned snapshot 在 callback 前后不变，lower receipt 证明 managed/native/local lock attempt 均为零。
terminal receipt 必须同时绑定 unsafe custody 成功保留、active route 已移除、failure-custody terminal route、
registration 仍在且 root 留给 parent cleanup；任一轴不等都失败关闭。这是将来 Windows child record 的
源码合同，不是 actual execution。

q3 的 `a2lockq3` version、selector 语义、135-value wire width 与 native-receipt digest domain 保持不变；但
`implementation_sha256` 必须绑定本批扩展后的完整 stored-poison source scope，因此 current q3 完整 payload
字节会按设计漂移，未来仍须与 q4 一起重新编译、执行、复核。不得把“wire/native 合同未扩宽”误写成旧 payload
或 implementation seal 可直接复用。

对称 sibling 的 test-only route-preemption runner/selector/payload 源码桥只在同一 exact callback 已 admission 且 installed `xShmLock` 返回 `SQLITE_IOERR_SHMLOCK` 后、unsafe-retention lookup 前 one-shot 移除 route，再绑定 callback completion route-unknown、零 lower lock attempt、terminal custody 与 parent cleanup。它同样只是未编译、未运行的非授权 actual-receipt 源码形状。
retention-succeeded 与 route-unknown sibling catalogs 各保持 1,320 rows/237,857 bytes，SHA-256 为 `4da94c20e91d97a0082116879718b1ccf0271eb235ed785e65a2e36e7a949d85`、`df931ad7725843098f228d07d9798d79e92f2beec4e1c23e83fc89219dfa1396`，逐行绑定 action/range/profile 与 `(case_key_sha256, full_record_sha256)`。

## 4. Lock q5–q15 exact tranches

q5–q15 的精确成员、lower/receipt source contract、catalog 与隔离约束由
[`Lock dynamic tranches authority`](node-plugin-vfs-lock-dynamic-tranches-authority.md) 维护。q9 只新增
`LockPreManagedCallbackRejectionV1` 的 528 个 singleton member/group：88 个合法 request ×
AdmissionRouteUnknown Direct、AdmissionCounterOverflow Direct、UnsupportedFileRole Completed/RouteUnknown、
ShmDetached Completed/RouteUnknown 六族。q10 只新增 `LockAbiScalarRejectionV1` 的 7 个 singleton：
`offset/count/flags` validity 的七个非全真组合；完整 typed matcher、Expected 与 frozen member seals 必须
精确相等，真实 installed `xShmLock` 必须在进入 raw state、registry callback 或 managed lower 前由 production
ABI scalar gate 返回 `SQLITE_IOERR_SHMLOCK`。q11 只新增 `LockRawStateRejectionV1` 的 11 个 singleton：
两个 invalid-pointer premise 保持 excluded；typed matcher、seals、memory-safe child 和 production
`xShmLock` raw admission/abandon/adapter/cleanup 的精确合同只由 Lock tranches authority 维护。未来回执只能标为
`controlled_fault_actual`，不得冒充自然生产可达或普通 coverage。完整 3,432-member initialization umbrella
仍在 q11 外；q12–q15 各承接 88 个，余下 3,080 members / 2,552 groups 保持 planned-missing。

q12 的首个冻结纵切命名为 `LockNativeAcquireCreatedFirstExclusiveReleaseErrorV1`，只承接 initialization
namespace 中 `dms.created-first.exclusive-release-error` 的 88 个 singleton：44 个合法 native-acquire
request（8 个 `LockShared` 单槽与 36 个 `LockExclusive` 连续非空 range）乘两个 unsafe terminal：
`retention.succeeded.terminal.route-unknown` 与
`retention.route-unknown-prior-quarantine.terminal.route-unknown`。完整 typed shape 固定为
`phase/fault_site=DmsExclusiveRelease`、`path=CreatedFirst`、`timing=AtCall`、
`class=OutcomeUncertainPoisoned`、`mutation=Uncertain`、`lock_uncertain=true`、file retained、
`DMS=ExclusiveOutcomeUncertain`、DMS native lock/unlock=`1/1`；requested Lock range 的 native/local
attempt 必须全零。未来 actual 必须通过 dedicated one-shot initialization controller，在 production
CreatedFirst exclusive-release 点执行一次真实 `UnlockFileEx` 且故意不读取返回 BOOL，再由 typed receipt、
Lock no-entry ledger、unsafe quarantine、隔离 child exit 与 parent-owned cleanup 闭合；只能标记
`controlled_fault_actual`。current source 已同时具备 matcher/catalog、dedicated initialization controller、
production one-shot seam、requested-range no-entry ledger、isolated child/runner/payload 与全部 source-scope 接线；
catalog 为 88 rows / 18,386 bytes，SHA-256=`51d675ee9b2fe990b71a924a6f7cf016c6738e7f88872449f91a20ba6d2566df`。

q13 的第二个冻结纵切命名为 `LockNativeAcquireExistingFirstExclusiveReleaseErrorV1`，只承接
initialization namespace 中 `dms.existing-first.exclusive-release-error` 的 88 个 singleton：同样由 44 个
合法 native-acquire request 乘两个 unsafe terminal 组成，但 `path=ExistingFirst`，不得被 q12
CreatedFirst 匹配器或回执吸收。future controlled-fault actual 必须先用 typed precreation receipt 证明物理 SHM
已创建并关闭，再从 cold WAL-main attach 观察 `was_created=false`，随后进入与 q12 同类型但
case-bound 的 DMS lock/truncate/release、requested-range no-entry ledger、quarantine、child exit 与 parent cleanup。
`UnlockFileEx` BOOL 仍必须故意不读，因此只能形成 `controlled_fault_actual`，不得冒充 natural actual。
q13 catalog 为 88 rows / 18,474 bytes，SHA-256=`03b883842b3fd1886779dcb07573521d14ca3125894b38c4d341a462137424f4`。

q14 的第三个冻结纵切命名为 `LockNativeAcquireCreatedFirstTruncateErrorReleaseSucceededV1`，只承接
initialization namespace 中 `dms.created-first.truncate-error.release-succeeded` 的 88 个 singleton：同样由
44 个合法 native-acquire request 乘两个 unsafe terminal 组成，完整 typed shape 固定为
`phase/fault_site=DmsTruncate`、`path=CreatedFirst`、`timing=AtCall`、
`class=OutcomeUncertainPoisoned`、`mutation=Uncertain`、`lock_uncertain=false`、file retained、
`DMS=Released`、DMS native lock/unlock=`1/1`、release success observed；requested Lock range 的
native/local attempt 必须全零。future controlled-fault actual 必须在同一 production truncate validation 点
调用一次真实 `File::set_len(0)` 且故意不读取其 `Result`/return receipt，形成 typed
`ReturnReceiptUnavailable`，随后 normal production `UnlockFileEx` 成功必须被 receipt 观察；再由 typed receipt、
Lock no-entry ledger、unsafe quarantine、隔离 child exit 与 parent-owned cleanup 闭合。它只能标记
`controlled_fault_actual`，不得冒充 natural actual。q14 catalog 为 88 rows / 17,594 bytes，SHA-256=
`95f1c8e40da35ac23cc46e20310178c2aef09adacf777155d9b67e5802d69abd`。

q15 的第四个冻结纵切命名为 `LockNativeAcquireExistingFirstTruncateErrorReleaseSucceededV1`，只承接
initialization namespace 中 `dms.existing-first.truncate-error.release-ok` 的 88 个 singleton：仍由相同 44 个
合法 native-acquire request 乘两个 unsafe terminal 组成，typed shape 与 q14 相同但固定
`path=ExistingFirst`。future controlled-fault actual 必须先用 typed precreation receipt 证明物理 SHM 已创建并
关闭，再从 cold WAL-main attach 观察 `was_created=false`；随后在 production truncate validation 点调用一次真实
`File::set_len(0)` 且故意不读 `Result`，形成 `ReturnReceiptUnavailable`，并观察 normal production
`UnlockFileEx` 成功与 DMS `Released`。requested Lock range 的 native/local attempt 必须全零；它仍只能标记
`controlled_fault_actual`，不得冒充 natural actual。q15 catalog 为 88 rows / 17,682 bytes，SHA-256=
`d8ac9985f433b9fbd089a8804136f107cf37bd21566b5852611e98674f52911b`。

完整 `LockNativeAcquireInitializationFailureV1` 仍是 3,432 members / 2,904 normalized groups，而不是
q12–q15 四段的别名。其机械分期为 Open `792/792`、DMS exclusive acquire `528/440`、first-process
truncate/release `528/528`、shared acquire `1,584/1,144`；四批共 352 个 source-only member/group 只建立可扩展 typed
controller 的四个连续 vertical slice，后续 3,080 members / 2,552 groups 继续 planned-missing，不得由已写纵切推导完成。

聚合未运行 inventory 预期现为 `4,020 present members /
4,020 present groups / 4,648 missing members / 4,120 missing groups / 8,668 total members /
8,140 total groups`；member coverage 仍为 `0/8,668`，无 actual
record、reviewed digest、`Qlock` 或 Windows numerator。

## 5. Pre-manifest execution-program inventory

商 manifest 之前必须先有一层独立、非授权的 `ExecutionProgramInventoryV1`。它复用同一两遍 frozen ingress，
但在第二遍只调用完整 typed semantic preparation 与 source-only inventory classifier，不调用 actual
`resolve_v1`/receipt validation，也不把 Missing 变成 projection failure。每个 included member 必须恰好映射到
一个 capability-normalized program identity；identity 完整保留
root、typed descriptor、axes、Expected 与内部编译的 runner plan，只把 capability 归一为该 root 的 planned
gap。`program_id` 使用独立 domain，精确绑定 root、projector schema、normalized descriptor digest 与
`plan_sha256`；它既不是 dynamic class ID，也不直接复用 descriptor digest。CaseKey/full-record digest只进入
member binding，不进入 program identity。

inventory 状态只有两种：

```text
PlannedMissing(exact_gap)
SourcePresentReceiptRequired { implementation_sha256 }
```

本层禁止出现 `Supported` 或 `ExecutionVerified`。`SourcePresentReceiptRequired` 只表示 exact source matcher
找到了实现，仍须后续正式 receipt；当前 matcher 只认 Map `RegionSizeBudget`、`RegionCountBudget`、
`LogicalSizeBudget` 三类 `Completed` 请求各自的 Observe/Extend 形状、上述六个 exact single-region lifecycle
形状、`MapRegionLoopSuccessV1` 两个 bounded exact 子族，以及 Lock 四种 action 的
`RangeOverflow`、`EndPastEight` 和仅 shared action 可用的 `SharedMultiSlot` 直接拒绝形状，再加精确的 104 个
positive lifecycle 形状：8 槽内 36 个非空连续 exclusive range 与 8 个单槽 shared range 的 native acquire/release，
以及 8 个单槽 shared-local acquire 与 8 个 single-slot shared-local release；Lock matcher 又接受
stored-poison 两个 typed completion 各 15 profiles×88 action/range=1,320 个 exact member。其他 Map program
继续是 `PlannedMissing(QuotientRunnerNotIntegrated)`，其他 Lock program 继续是
`PlannedMissing(LockObservationIncomplete)`。matcher 只把明确的 `UnsupportedProgram` 归为 planned missing；
plan/binding 等内部错误必须携带 exact member 使整次 inventory 失败，禁止 fail-open-as-missing。

Lock 窄 program family 的 raw 输入固定为：range overflow 使用 `offset=255,count=1`；end-past-eight 使用
`offset=8,count=1`；shared-multi-slot 使用 `offset=0,count=2`。flags 必须由 exact action 机械映射到 SQLite
`LOCK/UNLOCK × SHARED/EXCLUSIVE` 位，且 shared-multi-slot 禁止匹配 exclusive action。child 必须真实调用已安装
`xShmLock`，观察 `SQLITE_IOERR_SHMLOCK`、原始 slots 未变、未创建 SHM target、连接仍存活与 VFS 注册仍在；
parent 只在 exact child exit、root/registration/payload/environment 与删除回执全部闭合后产生私有 execution
receipt。调用方提交的 result code、digest 或 expected vector 均不能构造该 receipt。

Lock positive lifecycle child 必须把 setup 与 cleanup 排除在 observation ledger 之外；native 项只接受 exact
selected target 上一次匹配 acquire/release，local sibling-coalesced 项只接受零 native call，并绑定同一 target 的
before/after masks、runtime generation、SHM connection ID 与 lower receipt。该设计已写入源码，但未编译、未执行；
它不产生 actual receipt、Windows record 或 `Supported` 验收事实。

inventory context 必须绑定 static baseline/source scope/ledger/manifest、included/excluded/source-universe、
exact included member-pair set、projector schema/source scope、frozen descriptor binding 与 inventory source scope。
排序后的 `member -> program_id` reverse index、每个 program 的 plan/status/member-set、group/member 计数和
总 inventory body 分别使用 domain-separated canonical digest。builder 必须从 program group union 独立重建
reverse index 并与逐叶 membership 相等；任一 missing member、extra、duplicate、collision、binding drift 或
空 group 都使 inventory 原子失败。

该 inventory 不是 quotient manifest，也不选择正式 representative，不产生 `Qmap/Qlock`、
`DynamicQuotientMemberCoverage`、Windows record 或任何 runtime permit。只有以后相应 root 的完整 program
inventory 全部达到 source-present、独立 review 冻结其 digest，并由 quotient manifest context 反向绑定后，
才可进入 manifest 冻结。Map 与 Lock review digest 独立，不能跨 root 复用。生产 actual-execution path 当前仍
未开放；验收规则要求未来 actual Windows execution 只能在 manifest 后按 frozen class representative 发生。

## 6. Reviewed source-program admission bridge

current source 另写入一条 pre-manifest、非执行型的 source-program admission bridge。它不能从裸
`ExecutionProgramInventoryV1`、单个 `SourcePresentReceiptRequired` 或调用方提交的 digest 构造许可。non-test
production authority 的唯一入口必须同时验证：inventory root/static/projector/descriptor context 与本次两遍
frozen ingress 完全相等；
`planned_missing_member_count == planned_missing_group_count == 0`；source-present member/group 计数分别等于
完整 member/group 计数；program group union、reverse index、membership/catalog/body digest 全部重算一致；
并且 `inventory_sha256` 精确等于独立 review 后 checked-in 的 expected digest。任一条件不成立都在 catalog
观察前原子失败，不保留部分 class 或 frozen-looking manifest。

通过上述门后，私有 provider 才能按 frozen member 发放一次性 source-program admission receipt。receipt 必须
交叉绑定 reviewed inventory digest、member seal、`program_id`、capability-normalized descriptor digest、
`plan_sha256` 与 `implementation_sha256`；missing、extra、duplicate、member/program swap、plan 或 implementation
替换均使整次 candidate 失败。catalog 保留 producer 的 exact planned-missing runner-admission receipt 与原始
semantic key，只在另一个 source-program admission receipt 精确成立后把该 member 纳入 class partition；本层
不得制造 `RunnerAdmissionDecisionV1::Supported`、execution digest 或 Windows record。

成功的 quotient manifest context/body 必须反向绑定 reviewed inventory body、program membership/catalog
commitment 与 source-program admission binding，使同一 member 的 static seal、descriptor、program 与 class
归属不能跨 inventory 或 manifest 重放。这是 manifest source completeness 的前置证明，不是 actual execution。
本批源码尚未把 actual receipt 绑定到 manifest/class；现有 `#[cfg(all(test, windows))]` helper 只是
implementation fixture，不是 acceptance authority。验收规则要求未来 actual
`MapRunnerExecutionReceiptV1`/`LockRunnerExecutionReceiptV1` 只能在相应 manifest 冻结后，由 frozen class 的
canonical representative 执行真实 Windows child 后产生。

当前该 bridge 只能失败关闭：Map source test 预期 `43,476` member 中有 `521` 个 source-present、`42,955` 个
planned-missing；Lock source test 预期 `8,668 members / 8,140 groups` 中有
`4,020 present members/groups`、`4,648 missing members / 4,120 missing groups`；两根 reviewed inventory
digest 均尚未 checked-in/frozen。因此 provider authority 不可构造，
full Map/Lock candidate 必须在 catalog/manifest 前分别原子失败；该结论没有运行证据，current source 仍为
`passed=0 failed=0 actual=not_run`。Lock coverage=`0/8668`、`Qlock=unknown`、
`WindowsDynamic=not_opened`、production closed；q9–q14 checkpoint 的 19-artifact global frozen/source-owner
refresh 已逾期并继续独立 deferred，未来编译或 runtime acceptance 前的重生成/复核必须覆盖 q15。
