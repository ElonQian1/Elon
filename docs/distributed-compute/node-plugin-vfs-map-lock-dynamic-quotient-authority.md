---
title: 节点插件 VFS Map/Lock 动态商集权威 V1
status: current
reviewed_at: 2026-09-01
owners: node, security
design_status: design_frozen
implementation_status: typed_projector_candidate_prior_compiled_map_and_lock_pre_manifest_program_inventory_and_reviewed_source_admission_bridge_source_written_uncompiled_unrun
verification_status: prior_targeted_unit_36_passed_current_map_lock_inventory_and_source_admission_not_run_review_digests_and_manifests_not_frozen
authority_scope: backend-a2-map-lock-dynamic-quotient-authority-v1
---

# Node Plugin VFS Map/Lock Dynamic Quotient Authority V1

## 1. Authority scope and dependency

本文冻结 A2 Map/Lock 从完整静态叶集合到真实 Windows 执行类集合的唯一 V1 合同。它依赖
[`Map/Lock static denominator authority`](node-plugin-vfs-map-lock-static-denominator-authority.md)，
不重算、改写或缩小静态分母。静态合同已经闭合为 Map `43,476/43,476`、Lock
`8,668/8,668`；动态商只决定这些 included 静态成员如何形成可执行等价类。

动态分母分别记作 `Qmap` 与 `Qlock`。它们只能由本文规定的类型化生成器对完整静态记录机械投影、
精确分区并冻结 manifest 后得出。typed descriptor、projector、内存 catalog/manifest builder 与两遍原子
candidate 入口在前序基线已实现并通过编译和定向单元测试；本批又写入 sealed runner admission，以及覆盖
Map `RegionSizeBudget`、`RegionCountBudget`、`LogicalSizeBudget` 三类 `Completed` 请求和 Lock 四种 action 下完整
`RangeOverflow`、`EndPastEight`、合法 `SharedMultiSlot` 请求拒绝矩阵的 source-only executable program family、
私有 actual receipt 和 parent/child/cleanup 合同。本批又为 Lock 写入 104 个 positive lifecycle program：native
acquire `44`、native release `44`、shared-local acquire/release `16`；它们以 same-target pre/post、exact
runtime generation/SHM connection identity、selected/sibling snapshot 与一次性 native/local ledger 形成完整的源级
observation/native-receipt 接线。current source 进一步把“语义 program 分组”
“源码 program 存在”“reviewed source admission”与
“actual execution verified”拆开，新增完整 Map/Lock 两根各自的 pre-manifest execution-program inventory，以及
`reviewed inventory -> source-program admission provider -> catalog/manifest binding` 的失败关闭源码桥。只有
完整 inventory 不含任何 planned-missing，且其 body digest 与独立 review 后 checked-in 的 expected digest
精确相等，provider 才可构造；原始 inventory status 不能直接授权 catalog。该路径尚未编译、未运行，当前
Map 保留前序 `MapSingleRegionLifecycleV1` 六成员纵切：Empty/Reuse/TargetMissing 各自的
Observe/Extend 路径必须由真实 installed `xShmMap`、same-target pre/post、一次性 Map ledger、raw output 与受管
selected-region pointer 的仅进程内等值验证以及 parent/child/cleanup 回执共同证明。本批再新增独立 q4
`MapRegionLoopSuccessV1`：Created-first empty Extend `n=1..256` 与 Node-live target-missing Extend `n=1..255`
两个 exact 子族共 511 frozen members，扣除 q3 已覆盖的两个 ordinal-001 后净新增 509；q4 固定精确 N 次有序
mapping ledger、完整 typed matcher、逐成员 seal catalog 与 source digest。Map source test 因而预期有
`521` 个 source-present member/group、`42,955` 个 planned-missing member；Lock source test
预期有 `114` 个 source-present member/group、`8,554` 个 planned-missing member；两根都没有 checked-in
reviewed inventory digest。
默认 Map/Lock producers、完整 candidate 与 manifest 路径未因此接通；
没有任何商 manifest 被成功生成、复核或冻结，所以两个值仍是 `unknown`。

本合同是动态执行压缩合同，不是静态覆盖压缩合同：每个 included 静态 CaseKey 仍必须被完整承诺且
恰好出现一次；每个冻结动态类将来才恰好对应一条真实、进程隔离的 Windows 记录。

既有 Unmap/JointClose 等 verified family authority 中关于“Map/Lock denominator 未开放”的句子只描述
各自 evidence commit 当时的历史聚合基线；这些不可改写的已验证需求正文不控制当前 Map/Lock 状态。
当前静态事实以 static denominator authority 为准，当前剩余门只以本文和聚合 A2 current 文档为准。

## 2. Frozen static inputs

V1 的输入只允许来自静态权威已经验证的完整 typed record stream。冻结事实为：

| Root | Included | Excluded | Source universe | Leaf ledger SHA-256 | Manifest SHA-256 |
|---|---:|---:|---:|---|---|
| Map | 43,476 | 281,085 | 324,561 | `0a756fe7f48ba5fb4634f8f2716d482e1382152f29ad7e300dd411c96e205333` | `0c51c3abe52f1a4f5ad1217c79ebd7393188452ff09659739ca6e1d93d205c19` |
| Lock | 8,668 | 53,774 | 62,442 | `23610b46e8217d396aea7a5367c2eed93f54c2488178d2ee8aa80c121425f082` | `c690c2f5b78b68201bd5c0eacd4e6489e87bb4c6abf8ab584aa24e443795491e` |

静态 source baseline 为 `47cb2652321b42cc9689319075d253fe2275ace1`。当前 candidate 入口先完整执行
静态 manifest、ledger、source owner、CaseKey、Expected 与 full-record seal 校验；任一静态漂移都使
动态候选整体失败，不允许部分读取、自动修复或按旧 manifest 继续生成。

checked-in leaf TSV 只有 leaf identity、outcome 与摘要，无法恢复 decisions、SourceBranch 或 Expected，
所以它只能用于成员承诺复核，不能作为分类输入。分类器必须消费随静态图构建产生的完整
`LeafRecordV1` 及与该 terminal 同源产生的 typed terminal descriptor；不得事后解析 `leaf_id`、node id、
测试名、wire selector、debug label、列表位置或源码展示文本来恢复语义。

Excluded 叶永不进入动态商。它们仍由静态权威完整承诺，但不得成为 class member、representative 或
Windows numerator。

## 3. Dynamic execution equivalence relation

对同一 root 的两个 included terminal record `a`、`b`，V1 只允许以下等价关系：

```text
a ~ b  <=>  DynamicClassProjectorV1(a) == DynamicClassProjectorV1(b)
```

投影结果必须是版本化、完全类型化的 `DynamicClassKeyV1`，至少包含：

```text
schema_version
root = Map | Lock
typed source site
typed stimulus and prestate
typed operation / phase / timing / occurrence
typed execution and cleanup recipe
DynamicExpectedV1
```

所有未到达的后继轴必须写成其 enum 的 `NotReached`，不能使用 `None`、空字符串、通配符或默认值混淆
“未读取”“任意合法值”和“未知”。任何无法投影、未知 enum、未支持 harness 或未证明可执行的 class
都使整次生成失败关闭；不能跳过、降级为 excluded 或并入相邻类。

不同生产 source site 不因 SQLite 返回值或 Expected 恰好相同而合并。source site 必须来自 typed
descriptor，不来自 symbol 字符串解析。输入拒绝、自然成功、callback fault、managed/native fault、
after-success fault、cleanup rewrite 与 quarantine 是不同 execution recipe，除非版本化证明显式允许
等价。

## 4. Identity erasure boundary

允许 alpha-rename 的只有每次真实运行必须重新分配、且不改变生产语义的 harness/runtime binding：run
nonce、临时根、registration ID、route ordinal、runtime generation、SHM connection ID 与 child PID。
这些值不得进入 class key，但必须在每条 dynamic record 的 environment/identity commitment 中保持同一
场景内精确绑定。

`CaseKey`、`leaf_id`、family id 和代表成员摘要同样不得进入 class key；否则每个静态叶都会人为成为
单独类。它们只用于成员承诺、反向索引与可审计追踪。现有 case-key-salted Expected/source 摘要也不能
作为语义相等键；实现必须增加不含 CaseKey、domain-separated 的 canonical semantic digest。

以下内容不得作为 alpha-renamed identity 消去：生产 source site、callback/role、operation、phase、
timing、occurrence、fault class、prestate、topology、mutation/uncertainty、lock effect、DMS state、
custody、cleanup 路径及任何可观察计数。

## 5. Map class laws

Map key 必须至少保留实际到达路径上的 mode、region request/operation、cold/warm prestate、node/profile、
file/size state、DMS initialization、mapping/view state、fault site、phase、timing、occurrence、terminal
projection、cleanup rewrite、payload custody 与完整 Expected observation。

六个成功语义不得互相合并：Extend cold-create、Extend warm-create、Extend reuse、Observe warm-create、
Observe reuse、Observe not-present。不同 source site 即使产生相同 `SQLITE_OK` 也保持不同。

Map loop ordinal 与 regions-to-create 默认是保留轴，因为它们会改变 mapping/view 操作计数。只有一个
独立、版本化、可执行的全域证明同时证明该轴对所有成员的 DynamicExpected、fixture recipe、observer
与 cleanup 均无影响，才允许在未来 schema 版本中消去；V1 不授予该消去。

## 6. Lock class laws

Lock key 必须至少保留 action、`first`、`count`、`mask`、held/sibling prestate、local coalescing 或
contention、native acquire/release 路径、fault site、phase、timing、occurrence、lock effect/uncertainty、
DMS state、custody、cleanup 与完整 Expected observation。

`LockShared`、`LockExclusive`、`UnlockShared`、`UnlockExclusive` 不互相合并。`first/count/mask` 在 V1
中绝不消去：`first` 参与真实 Windows byte-range offset，range 与 mask 也决定 exclusive table、冲突
和释放行为。正常 sibling contention 的 `SQLITE_BUSY` 与注入或 native I/O failure 必须保持不同类。

## 7. DynamicExpected and execution recipe

`DynamicExpectedV1` 是从完整静态 `ExpectedV1` 加 typed terminal descriptor 机械投影出的动态观测合同，
不是 runner 手写的预期。它至少完整保留：SQLite channel/result、disposition、typed phase、failure、
mutation、lock effect/uncertainty、DMS lock、raw slot、route/callback、file/mapping/view/payload custody和
全部 operation counts；并补充真实 fixture 所需的 topology、terminal/callback permission、lease/custody、
fault-selector 与 cleanup observation。

实现必须为每一新增动态字段声明来源：静态字段的直接投影、typed descriptor 的确定性补全，或仅存在于
actual/environment envelope 的 runtime binding。不得笼统声称 static Expected 与 dynamic actual
“所有字段相等”；验收必须逐字段区分 projected expected、runtime-bound identity 与独立 actual。

每个 class 同时冻结一个可执行 recipe：fixture/prestate、真实 callback 入口、fault seam、occurrence、
独立 observer、expected poststate、child retention 与 parent cleanup。没有可执行 recipe 的类不能从商
manifest 中删除，也不能被计为动态完成；其存在会让 `WindowsDynamic` 保持未完成。

真实 producer descriptor 还必须先通过 root-specific、有限且闭合的 typed coherence 关系；source site、
stimulus、prestate、operation、phase、timing、occurrence、fixture/callback/seam/observer/cleanup 与关键 axes
不能跨合法 tuple 拼接。未知 tuple、跨 root capability gap、Supported/Missing 混合状态，以及同一 catalog
内不同 Missing gap 都失败关闭。新增的 capability 形状严格限于 Map `RegionSizeBudget`、`RegionCountBudget`、
`LogicalSizeBudget` 且 completion 为 `Completed`；它们只允许进入私有 receipt 复验路径，不把 producer 声明
本身变成许可。此闭合只证明 producer
元数据没有被错配；它不证明正式 runner 或 observation 已存在。

producer coherence 之后还必须由 projector 内部从同一个已验证 semantic key 机械编译 root-specific runner
plan。plan commitment 域隔离地绑定 projector schema、root、capability-normalized descriptor digest、有序
required stages 与 exact planned-missing gap；producer 不能提交、自签或替换该 plan。裸
`RunnerCapabilityV1::Supported` 只是声明，不是 permit；普通 `resolve_v1` 仍拒绝裸声明。仅
`project_validated_dynamic_terminal_with_map_execution_v1` 可消费私有 `MapRunnerExecutionReceiptV1`；当它与
exact member、normalized descriptor、内部 plan、program implementation 和 execution commitment 全部精确
绑定时，上述三类 request-budget/`Completed` 窄路径才可形成
`RunnerAdmissionDecisionV1::Supported { implementation_sha256, execution_sha256 }`。缺 receipt、跨 root/plan、
semantic drift 或任一 commitment 替换都失败关闭。

该窄 program family 的 source design 固定 parent 拥有隔离根与最终删除责任，child 按 stimulus 绑定的
region/size 执行受管 VFS Map callback 并
形成实际结果，parent 只在 child terminal/exit 与 cleanup receipt 都绑定后消费私有 actual receipt。它仍是
未编译、未运行的程序合同，不是 Windows record。默认 Map producers 仍全量签发
`Missing(QuotientRunnerNotIntegrated)`，Lock 默认 producers 仍签发
`Missing(LockObservationIncomplete)`。只有精确命中 Map 三预算、q3 六个 single-region lifecycle 或 q4
`MapRegionLoopSuccessV1` exact program，以及 Lock 的
10 个 request-validation / 104 个 positive lifecycle program，且持有私有、进程隔离 actual receipt 的
`Supported` descriptor 才可通过 program-local 准入；
因此当前完整 candidate 仍没有
class 被放行。

### 7.0 `MapSingleRegionLifecycleV1` bounded tranche

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

### 7.1 Pre-manifest execution-program inventory

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
以及 8 个单槽 shared-local acquire 与 8 个 single-slot shared-local release。其他 Map program
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

### 7.2 Reviewed source-program admission bridge

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
planned-missing；Lock source test 预期 `8,668` member 中有 `114` 个 source-present、`8,554` 个
planned-missing；两根 reviewed inventory digest 均尚未 checked-in/frozen。因此 provider authority 不可构造，
full Map/Lock candidate 必须在 catalog/manifest 前分别原子失败；该结论没有运行证据，current source 仍为
`passed=0 failed=0 actual=not_run`。

## 8. Class catalog and member commitments

每个 `DynamicClassV1` 必须冻结：

- canonical class key、class-key semantic digest、由该 digest 唯一派生的 class ID、root、schema/projector version；
- 排序后的 `(case_key_sha256, full_record_sha256)` 成员列表、member count 与 member-set digest；
- canonical representative 的 case-key/full-record digest；
- typed source site、保留轴、显式消去轴及相应 proof kind/digest；
- `DynamicExpectedV1`、execution recipe、fixture/observer/cleanup schema；
- 所属 static manifest/ledger/source baseline 与全局 class catalog digest。

实现中的 catalog、classes、membership map 与 reverse index 均保持 private；调用方不能取得可变 class
集合后自行改写成员归属。成功 bundle 必须同时带有三类相互独立的冻结承诺：

1. root/schema-bound、按 member seal 排序的 `member -> class ID digest` commitment；它由私有 class
   union 重建，并与 manifest 的 exact reverse index 相互校验；
2. root/schema/static-manifest/included-count/entry-count-bound、按 member seal 排序的
   `member -> normalized full descriptor semantic key digest` commitment。normalized digest 完整覆盖 root、
   source site、stimulus、prestate、operation、phase、timing、occurrence、recipe、axes 与
   `DynamicExpectedV1`，只把 recipe capability 归一化，因此同 root、同 phase 的 descriptor swap 仍会
   造成 commitment drift。
3. root-bound、按 member seal 排序的 `member -> normalized descriptor digest + runner plan digest + exact gap`
   admission commitment；它必须与 descriptor binding 的 member 和 normalized semantic digest 精确一致，
   并同时进入当前 blocker receipt、未来 catalog 与 quotient manifest body。

pre-manifest program inventory 的 program membership/catalog/body commitments 与上述 quotient bundle 分离；
它们只能表达 program 规划状态，不能作为第四种 quotient manifest commitment 偷渡部分 catalog。

reviewed source-program admission binding 同样不改变 class 等价关系。它只证明完整 inventory 已经 source-present、
经独立 digest review，且每个 catalog member 精确消费其所属 program receipt；manifest 必须绑定它，但不能把它
解释成 `Supported` 或 actual execution commitment。

在 quotient bundle 中，capability 归一化只服务于 descriptor-binding commitment；pre-manifest inventory 的
归一化只形成非授权 program identity。默认完整 producer inventory 仍要求 Map 为
`Missing(QuotientRunnerNotIntegrated)`、Lock 为 `Missing(LockObservationIncomplete)`。仅窄 Map program-local
descriptor 可声明 `Supported`，且仍须私有 actual receipt 精确复验；gap 互换和未密封声明继续失败关闭。

Representative 只能从成员中机械选择，V1 固定为按 `case_key_sha256`、再按 `full_record_sha256` 的
unsigned byte order最小者；不得按 `leaf_id`、测试名或人工偏好选择。代表成员只是该 class 的执行载体，
不是其他成员被忽略的理由；class record 必须绑定完整 member-set commitment。

V1 canonical digests 使用独立 domain：

```text
ELON-A2-MAP-LOCK-DYNAMIC-EXPECTED-V1
ELON-A2-MAP-LOCK-DYNAMIC-CLASS-KEY-V1
ELON-A2-MAP-LOCK-DYNAMIC-MEMBER-SET-V1
ELON-A2-MAP-LOCK-DYNAMIC-QUOTIENT-MANIFEST-V1
ELON-A2-MAP-LOCK-DYNAMIC-RUNNER-PLAN-V1
ELON-A2-MAP-LOCK-DYNAMIC-RUNNER-ADMISSION-BINDING-V1
ELON-A2-MAP-LOCK-EXECUTION-PROGRAM-ID-V1
ELON-A2-MAP-LOCK-EXECUTION-PROGRAM-INVENTORY-SOURCE-SCOPE-V1
ELON-A2-MAP-LOCK-EXECUTION-PROGRAM-MEMBERSHIP-V1
ELON-A2-MAP-LOCK-EXECUTION-PROGRAM-CATALOG-V1
ELON-A2-MAP-LOCK-EXECUTION-PROGRAM-INVENTORY-V1
```

canonical encoding 必须长度分隔、枚举显式、整数定宽、成员按摘要字节排序；禁止 JSON map 顺序、Debug
文本、平台路径、pointer、进程 ID 或 locale 进入 digest。上述 class key、DynamicExpected、member set、
class catalog、reverse index 与 manifest canonical encoding 已实现并通过定向单元测试；这只证明 builder
和 guard 的实现，不代表 frozen bytes、`Qmap/Qlock` 或正式商 manifest 已存在。

projector provenance commitment 精确纳入 producer coherence 的
`producer_coherence/{map,map_axes,lock,lock_axes}.rs`、`descriptor_binding.rs`、
`membership_commitment.rs`、`runner_admission.rs`、
`runner_admission/{canonical,map,map_program,map_program/request_budget,lock,lock_program,lock_program/request_validation,lock_program/lifecycle}.rs`；
其中任一接受关系或 commitment 编码变化都必须触发 projector provenance drift 和全量重审。

同一 commitment 还绑定真实执行 envelope：`a2_dynamic_evidence` 的 child/capture/environment/cleanup 与
Map/Lock runner，managed VFS 的 registration/connection/multi-connection/route/callback/fault wrapper，registry
bridge/custody，installed `sqlite_vfs_abi`，以及 managed-fs 的 module dispatch、Windows lock/SHM、coordinator、
types、initialization、mapping、snapshot、fault controller/operation/mapping 和一次性 Lock ledger。Lock lifecycle
implementation digest 使用上述 projector 全集中与 q2 Lock lifecycle execution 直接相关的固定子集，并另加
one-based program tag；因此 exact-target observer、installed ABI、
native/local path 或 parent/child cleanup 的直接语义依赖发生变化时，摘要必须漂移，不能只绑定 q2 自身文件。

pre-manifest inventory 自身另以 source-scope commitment 纳入 `program_inventory.rs`、
`program_inventory/{builder,model}.rs`、`program_inventory_canonical.rs`、`projector.rs`、
`runner_admission.rs` 与 `runner_admission/{canonical,map,map_program,lock}.rs`。该 commitment 只是源码谱系，
不能替代 inventory body、manifest 或执行回执。

每个 root 的 quotient manifest 还必须冻结：`Qmap` 或 `Qlock`、static included/excluded/source-universe
计数与摘要、projector schema/digest、class-key-set digest、membership map digest、representative map
digest、class catalog digest，以及反向 `static member -> class` 索引摘要。

## 9. Exact partition guards

冻结商集前必须同时成立：

```text
union(MapClass.members)  == frozen Map included members
union(LockClass.members) == frozen Lock included members
all class member sets are pairwise disjoint and non-empty
class_count == distinct projected class-key count
all canonical class keys and derived class IDs are unique within one root
missing == extra == duplicate == excluded_member == 0
unknown_projection == unexecutable_class == 0
each member CaseKey/full-record digest matches the frozen static ledger
each representative is exactly one member of its class
each member reprojects to exactly its class key
```

生成顺序固定为两遍原子入口：第一遍完成 frozen static ledger/root manifest 的全部校验，不向动态
catalog 暴露记录；随后必须先验证完整、checked-in reviewed inventory authority，任何 planned-missing 或 digest
不一致都在 catalog 观察前失败。第二遍重复同一 frozen gate，并把已经逐叶验证的 full record + typed descriptor
及其一次性 source-program admission receipt 只流入私有、可丢弃的内存 catalog。两遍 binding 都承诺 exact
`(case_key_sha256, full_record_sha256)` member-pair set 且必须完全相等。随后 catalog/manifest guard 从实际
class union 重算 member-pair set，重算 class key，检查 class/member 唯一性、排序、representative、反向索引
和全部 canonical digests；只有全部成功才返回内存 bundle。当前没有候选文件 writer，因此失败不会留下
可被误认作 frozen 的部分 manifest。checked-in frozen manifest 必须经独立 review 后另批提交，且任何
static 或 projector 漂移都要求全量重生成与重审。

前序验证事实严格限于：当时实现已编译，定向单元测试已通过；Lock 全量 `8,668` 成员 candidate gate 已按预期
完成 exact frozen ingress/typed projection，并因 `LockObservationIncomplete` 原子失败关闭；Map 全量
`43,476` 成员 candidate gate 也已完成 exact frozen ingress/typed projection，并因
`QuotientRunnerNotIntegrated` 原子失败关闭。Map 前序为六个 single-region lifecycle member 写入 q3
runner/ledger/receipt；本批又以独立 q4 `MapRegionLoopSuccessV1`、精确 N 次有序 ledger、typed matcher、逐成员
seal 与 source digest 净新增 509 个 source-present member。其真实阻塞已细化为剩余 `42,955` 个 planned-missing、current source 未编译/未运行和
reviewed inventory digest 缺失。Lock 的真实
阻塞是完整 observation 尚未实现；二者都在 class catalog 或 manifest 冻结前失败，因此不产生
`Qmap/Qlock`、member coverage 或 Windows numerator。Lock 当前的真实阻塞已细化为剩余 `8,554` 个
planned-missing member、current source 未编译/未运行和 reviewed inventory digest 缺失；不能再把 104 项已完成的
源级 observation/native-receipt 接线表述为 actual verification。

上述回执全部是本批 Map/Lock program/receipt 改动前的 prior baseline。本批 current source-only baseline
已冻结为 `3e9ddf1860d8a744ccab62222622689d12fdc80a`，旧 `4edfcbcb32518fed8f93157b1983222f5f8ef74e` 仅是前序 q3 baseline；当前只达到
`source_written/source_review_only/implementation_uncompiled/implementation_unrun`。Map 三预算、q3 六成员、q4
511-member semantic scope/净新增509、55 项 q3/q4 共享运行时 source closure 与 171 项唯一 projector provenance，以及 10+104 个 Lock 窄 program、私有
actual receipt、program inventory、reviewed source-program admission provider、catalog/manifest binding 与负向
测试源码均为 `passed=0/failed=0/not_run`；不得把 prior `36/36` 或 exact blocker 回执当作 current-source 验证。

前序基线验证回执为：

```text
dynamic_quotient_targeted=36/36
dynamic_quotient_fingerprint=aa96751fc2388adcf02469bac883ddf49583f5ffbfcf29252f781cff24da22f1
lock_exact_8668=expected_blocker_LockObservationIncomplete
lock_exact_8668_fingerprint=a31c60597be461b3d90a2b54c91fd3d7faa1fb1ba7ade981401793701bf4bd7d
map_bootstrap_only=expected_descriptor_binding_commitment_drift_not_passed
map_bootstrap_fingerprint=cfeb50fb2b6652bad6d800806d23545c359e3883a8a4c1c9b3a9954cb390b69d
map_bootstrap_actual_commitment=d3ba08a5ba0019f9ccda99ace8b580ef06eb4d6653ba80c0db5497bec51bd870
map_exact_43476=expected_blocker_QuotientRunnerNotIntegrated
map_exact_43476_fingerprint=1540e34b6e4271e39771583162e228bfa604da8e47af18cf231558065afd5b80
```

其中 Map bootstrap 只证明冻结前的 descriptor-binding commitment 会精确漂移；它是预期失败诊断，不能记为
通过。随后已将实际 Map commitment
`d3ba08a5ba0019f9ccda99ace8b580ef06eb4d6653ba80c0db5497bec51bd870` 与 Lock commitment
`0cc951c8c979608fb9861167f8d880a74fd2e042c4d2cd42673100e14083e8ef` checked-in 冻结，且分别被最终
exact gate 接受。Lock/Map 全量回执仍只证明 exact blocker 与原子失败关闭，不是商 manifest 或真实执行支持。

负向守卫至少覆盖缺成员、额外成员、重复成员、excluded 混入、空类、representative 非成员、成员摘要
漂移、class split/merge、semantic digest 漂移、未知 enum、leaf-text 分类、case-salted digest 误用、Map
ordinal 非法消去与 Lock range 非法消去；还必须覆盖 naked `Supported`、缺少或替换私有 actual receipt、跨 root
plan、同 root plan swap、non-capability semantic drift 与 admission binding digest 漂移。本批已写窄 Map/Lock
receipt/准入及相应负向测试源码；完整 `43,476 + 8,668` source integration、validator tamper acceptance 与
Windows execution 仍待补齐，全部均未运行。

## 10. Windows evidence and atomic reducer

商 manifest 冻结后，每个 class 恰好接受一条真实 `cfg(all(test, windows))`、进程隔离记录。child 必须
执行真实安装的受管 VFS callback 链并独立观察 actual；不能直接调用 coordinator、重放静态 Expected、
用注入错误代替 native receipt，或让 parent 合成 outcome。

每条 record 至少绑定 exact clean Git SHA、Windows build/arch/filesystem/SQLite、class ID、代表成员、
完整 member-set commitment、static/quotient manifest、真实 identity/environment、actual payload 与语义
digest、child exit、unsafe custody retention、parent root cleanup 和外部验证 fingerprint。

Reducer 只接受同一 exact checkout、同一受支持环境与一个完整 cohort 的全 class exact set。缺失、失败、
重复、未知、跨 commit、跨 manifest 或 cleanup 未闭合都使整族保持未完成；部分记录不能写成分数完成。
只有完整通过时才允许报告：

```text
Map  DynamicQuotientMemberCoverage=43476/43476  WindowsDynamic=Qmap/Qmap
Lock DynamicQuotientMemberCoverage=8668/8668    WindowsDynamic=Qlock/Qlock
```

静态成员覆盖与动态执行分母是两个独立维度，禁止再次把 `43,476` 或 `8,668` 直接用作
WindowsDynamic denominator。

## 11. Current state and forbidden inference

```text
design=design_frozen
static_map=43476/43476
static_lock=8668/8668
typed_terminal_descriptor=implemented_prior_compiled
quotient_projector=implemented_prior_compiled_current_source_modified_uncompiled_unrun
atomic_candidate=implemented_two_pass_in_memory_prior_compiled
canonical_catalog_manifest_guards=implemented_prior_targeted_unit_passed
producer_coherence=closed_typed_relations_mixed_state_and_gap_rejected
sealed_runner_admission_plan=source_written_source_review_only_uncompiled_unrun
map_three_request_budget_completed_programs=source_written_private_actual_receipt_parent_child_cleanup_uncompiled_unrun
map_single_region_lifecycle_programs=source_written_6_installed_xshmmap_exact_target_one_shot_ledger_pointer_equality_parent_child_cleanup_uncompiled_unrun
map_region_loop_success_v1=source_written_q4_two_exact_families_511_frozen_members_509_net_new_exact_n_ordered_mapping_ledger_typed_matcher_per_member_seal_source_digest_uncompiled_unrun
map_supported_admission=private_exact_receipt_binding_only_source_contract_not_run
map_pre_manifest_program_inventory=source_written_full_root_two_pass_non_authorizing_uncompiled_unrun
map_program_inventory_status=planned_missing_or_source_present_receipt_required_only
map_program_inventory_digest=not_generated_not_frozen
map_program_inventory_member_and_group_counts=unknown_not_run
map_program_inventory_unrun_test_expectation=members_43476_source_present_members_521_source_present_groups_521_planned_missing_members_42955
map_reviewed_inventory_digest=not_checked_in_not_frozen
map_source_program_admission_provider=source_written_fail_closed_uncompiled_unrun
map_source_program_admission_precondition=all_members_and_groups_source_present_plus_exact_checked_in_reviewed_digest
map_source_program_admission_current=unconstructible_unrun_source_expectation_planned_missing_members_42955_and_reviewed_digest_absent
map_catalog_manifest_inventory_binding=source_written_uncompiled_unrun
map_actual_execution_order=post_manifest_frozen_representative_only
map_default_producers=all_missing_quotient_runner_not_integrated
lock_request_validation_programs=source_written_10_private_actual_receipt_parent_child_cleanup_uncompiled_unrun
lock_positive_lifecycle_programs=source_written_104_native_acquire_44_native_release_44_shared_local_16_observation_native_receipt_wired_uncompiled_unrun
lock_supported_admission=private_exact_receipt_binding_only_source_contract_not_run
lock_pre_manifest_program_inventory=source_written_full_root_two_pass_non_authorizing_uncompiled_unrun
lock_program_inventory_status=planned_missing_or_source_present_receipt_required_only
lock_program_inventory_digest=not_generated_not_frozen
lock_program_inventory_member_and_group_counts=unknown_not_run
lock_program_inventory_unrun_test_expectation=members_8668_source_present_members_114_source_present_groups_114_planned_missing_members_8554
lock_reviewed_inventory_digest=not_checked_in_not_frozen
lock_source_program_admission_provider=source_written_fail_closed_uncompiled_unrun
lock_source_program_admission_current=unconstructible_unrun_source_expectation_planned_missing_members_8554_and_reviewed_digest_absent
lock_default_producers=all_missing_lock_observation_incomplete
runner_admission_raw_supported=fail_closed_without_private_exact_receipt_not_run
dynamic_quotient_targeted=prior_passed_36_of_36_current_source_not_run
map_candidate_gate=prior_passed_expected_fail_closed_43476_current_source_not_run
lock_candidate_gate=prior_passed_expected_fail_closed_8668_current_source_inventory_blocker_not_run
map_bootstrap_descriptor_binding=expected_pre_freeze_drift_not_passed
map_descriptor_binding=frozen_d3ba08a5ba0019f9ccda99ace8b580ef06eb4d6653ba80c0db5497bec51bd870_exact_gate_accepted
lock_descriptor_binding=frozen_0cc951c8c979608fb9861167f8d880a74fd2e042c4d2cd42673100e14083e8ef_exact_gate_accepted
map_blocker=program_inventory_incomplete_and_unreviewed_quotient_runner_not_integrated
lock_blocker=program_inventory_incomplete_and_unreviewed_lock_observation_incomplete
quotient_manifests=not_frozen
Qmap=unknown
Qlock=unknown
map_dynamic_member_coverage=0/43476
lock_dynamic_member_coverage=0/8668
windows_dynamic=not_opened
map_region_loop_windows_execution=not_run
compilation=not_run
targeted_unit_tests=not_run_passed_0_failed_0
windows_runtime=not_opened
next_recommended_tranche=lock_stored_poison_one_outcome_1320
```

本文不完成 A2，不注册生产 VFS，不调用生产 open，不创建 Connection/Opened authority，不获取 process
fence，不启动 A1/v15/Runtime/Ready，不产生 Provider、route、Offer、Attempt、Lease、派发、市场、结算或
资金效果。后续 program inventory 运行、完整源码 program、独立 review/freeze、source-program admission、商
manifest、manifest 后 Windows evidence 与宽回归必须各自按顺序独立验收。
