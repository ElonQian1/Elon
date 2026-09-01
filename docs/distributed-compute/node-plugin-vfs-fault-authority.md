---
title: 节点插件测试 VFS 故障合同权威
status: current
reviewed_at: 2026-09-02
owners: node, security
---

# 节点插件测试 VFS 故障合同权威

## 1. 权威范围与当前状态

本文冻结节点插件 A2 测试专用受管 SQLite VFS 的 SHM、联合关闭、多 Connection 与故障注入合同。逐 case 证据见[`动态验收`](node-plugin-vfs-fault-acceptance.md)；已拆出的 family 权威为[`Barrier 8`](node-plugin-vfs-barrier-dynamic-authority.md)、[`RegistryLifecycle 16`](node-plugin-vfs-registry-lifecycle-dynamic-authority.md)、[`Unmap 49`](node-plugin-vfs-unmap-dynamic-authority.md)与[`JointClose 36`](node-plugin-vfs-joint-close-dynamic-authority.md)。生产文件生命周期、Planning 依赖与本机 Store 仍分别由[`manifest authority`](node-plugin-manifest-catalog-authority.md)、[`planning authority`](node-plugin-planning-snapshot-authority.md)和[`local authority`](node-plugin-local-authority.md)维护。

A2b2 的 117 项静态 inventory 与 Windows dynamic 已冻结为 `117/117`。A2a/A2b1 的 source-exhaustive 静态合同也已由独立[`Map/Lock static denominator authority`](node-plugin-vfs-map-lock-static-denominator-authority.md)闭合：Map `43,476/43,476`、Lock `8,668/8,668`，所有 Pending/frontier 已在该权威的完整图、ledger、Expected 与 exclusion guards 下清零。动态执行不再要求每个静态 CaseKey 各跑一次；唯一后续合同是[`Map/Lock dynamic quotient authority`](node-plugin-vfs-map-lock-dynamic-quotient-authority.md)与其[`acceptance`](node-plugin-vfs-map-lock-dynamic-quotient-acceptance.md)。当前 `Qmap/Qlock=unknown`、quotient manifests 未冻结、`WindowsDynamic=not_opened`，所以 A2 仍为 `implementation_not_dynamically_accepted`。

下两段仅保留静态闭合前的 fragment provenance；它们不再控制当前 Map/Lock source universe、CaseKey、Expected、exclusion 或 StaticContract 计数。

本批另在 `TypedMapOperation` 开放前沿之后冻结一个互不并入前缀 DAG 的 Map route/operation-callback normal-return 局部商；其入口进一步限定为 outer callback-fault wrapper 已 pass 且 inner file live。该商含1个route preparation rejection、1个operation callback admission rejection，以及admission成功后的operation result×callback completion `2 × 2`，合计6个local cell。operation `Err` 在 completion `Ok|Err` 两支都优先返回 typed Failure；operation `Ok` + completion `Err` 也返回 typed Failure；只有 operation `Ok` + completion `Ok` 可以继续到 adapter projection，且该 continuation 仍是 Pending，不是 `NotPresent` 或 `Mapped`。6个cell在局部cut都保持output null、installed raw slots、cleanup none与pointer write=0；outer controller reject/selected/inner-missing与caught unwind均不在本局部商内。该增量已随测试目标编译，并进入下述共享 A2b1 静态守卫 `4/4`；这不是局部 fragment 的 denominator 或动态通过数，不改变owner-graph Pending=5/resolved cross-links=9，也不关闭 `TypedMapOperation` 或 `RawFallbackCustodyAndRouteProjection`。

紧邻该6-cell parent，本批再写入 adapter control/result→ABI projection 的 exact 7-cell reviewed inventory：前5个 Failure cell逐一继承并固定 `SQLITE_IOERR_SHMMAP`、output null、installed raw slots、cleanup none与0 pointer write；其余两格是 Observe-only `NotPresent` 与防御守卫通过后的 Observe/Extend `Mapped`。`NotPresent` 固定 `SQLITE_OK`、null与0 write；guard-pass `Mapped`固定携带非拥有型 `NonNull` pointer并由唯一 ABI arm执行1次write，managed coordinator继续保管view/mapping。前5格继续区分 callback-admission rejection、operation rejection与callback-completion rejection；其中 operation `Ok` + completion `Err` 产生后丢弃的success payload custody保持Pending，7格不再按该payload的`NotPresent|Mapped`类型细分。`AdapterRegionMismatch`、`AdapterLengthMismatch` 与 `AdapterNullPointer` 另形成3条child-local guard review：前两条仍为Pending，只有NullPointer在私有`NonNull<u8>`字段→原样accessor的reviewed type envelope内排除；三条都不进入7格，也不升级shared parent disposition或完整exclusion ledger。因此7格不是双`Ok` continuation或guard rejection路径的穷尽分割。Mapped typed wrapper create→adapter carry→ABI write按commit-bound owner source顺序锚定；这不是raw OS pointer创建、payload lifetime或动态可达性证明。mapped/dropped payload底层custody、managed prestate、route/callback custody与完整source universe仍Pending。composed child仍位于open frontier之后，不新增prefix edge，不改变owner-graph Pending=5/resolved cross-links=9，也不生成denominator key、`StaticContract`或`WindowsDynamic`。

A2c 严格 `cfg(all(test, windows))` 的进程隔离 runner 现覆盖 Barrier、`RegistrationShutdown` 各八项、RegistryLifecycle 十六项、Unmap 四十九项与 JointClose 三十六项；parent 对 exact selector/canonical payload、child exit、环境绑定和测试根删除收据做 fail-closed 验证。五族分别保持 `8/8`、`8/8`、`16/16`、`49/49`、`36/36`；JointClose 在 exact clean runtime-source commit `bfa1a1180d220e9a4c8e39251414fc9a1b0a9ace` 上正式闭合，A2b2=`117/117`，同提交宽回归=`266/266`。RegistryLifecycle 的 raw `xClose`、Unmap 与 JointClose 的 exact Windows native/custody/connection-VM 观察边界、route ledger、拒绝 seam 与逐项指纹只由各自[`RegistryLifecycle 动态权威`](node-plugin-vfs-registry-lifecycle-dynamic-authority.md)、[`Unmap 动态权威`](node-plugin-vfs-unmap-dynamic-authority.md)、[`JointClose 动态权威`](node-plugin-vfs-joint-close-dynamic-authority.md)和[`动态验收`](node-plugin-vfs-fault-acceptance.md)解释；独立 Map/Lock dynamic quotient/Windows 门禁未闭合，所以 A2 仍是 `implementation_not_dynamically_accepted`。

A2b1 前序 JointClose source baseline `e3663e109039f38477de4d6ab5cd57483dbd0541` 的 owner graph 曾在 ledger evidence commit `bfa1a1180d220e9a4c8e39251414fc9a1b0a9ace` 通过 `4/4`；该历史回执不外推到本批。后续 q3 source-only baseline 曾续绑为 `4edfcbcb32518fed8f93157b1983222f5f8ef74e`，五个受 Map lifecycle 接线影响的 owner Git OID/LF SHA-256 当时已重算；它同样不是本批 Lock 的 current baseline。本批尚无可引用的 clean source commit，守卫、编译和运行均未执行，`passed=0 failed=0 actual=not_run`。本批 source-only 接线不改变 fallback、Pending=5 或 resolved cross-links=9，也不开放 map/lock 动态验收。前序指纹与复测元组见[`动态验收 § 当前 A2b1 静态守卫证据`](node-plugin-vfs-fault-acceptance.md#当前-a2b1-静态守卫证据)。

生产 `ComputePluginHandleBoundAuthorityOpenIntent::open()` 必须继续固定返回 `COMPUTE_PLUGIN_HANDLE_BOUND_SQLITE_VFS_UNAVAILABLE`。A2 不提供生产 VFS 注册、process owner、live `sqlite3_file`、opened authority、A1 producer 或 v15 能力，也不改变 v14 blocked-only、Runtime stopped、`snapshot_ready=false` 与全部 side-effect false 的事实。

## 2. 已有基线与真实缺口

现有 managed-fs 已有以下低层形状：

- 一个 pinned namespace 只能消费成一个进程内 WAL/SHM runtime；runtime 按 exact main FileId 绑定多个本地 SHM connection ID，并拒绝第二个同目录 lock domain；
- SHM 内核已有固定 region budget、稳定映射、8 槽本地锁状态、DMS、OS byte-range lock、barrier、unmap/delete gate、typed failure phase 与永久 domain tombstone；
- registry/file-custody 已把 main、Journal/WAL sidecar、SHM lease、route 与 callback lease 不可拆持有；物理关闭失败会保留 exact custody，不能把 `Drop` 当作成功回执；
- 测试受管 VFS 已让单个真实 SQLite Connection 经 main、Journal/WAL、SHM 和 `xClose` 进入 exact route，并在正常关闭后退休 route、注销 VFS 和删除测试根。

A2a/A2b1 的 exact logical-name routes 共享 WAL/SHM runtime，但每条 Connection 独立 route/authorizer/custody；完整 terminal projection、`SourceBranch`、`Expected`、`CaseKey`、exclusion 与 StaticContract 已闭合为 Map `43476/43476`、Lock `8668/8668`。typed quotient 只有前序验证基线；current runner-admission source 未编译、未运行。Map q4 `MapRegionLoopSuccessV1` 为 511 frozen/509 net-new，未运行 inventory 预期 `521` present、`42,955` missing。Lock q18/q19 分别为 CreatedFirst/ExistingFirst shared-busy/close-ok 88=`16+72` singleton（44/44 completions）；q19 先物理预建 SHM，再以 cold `was_created=false` 进入 ExistingFirst。两批 future controlled-fault seam 均须让同 `FileId` 的 distinct holder/target handles 产生真实争用、分账 attempts 并显式观察 target close success；q19 协议=`a2lockq19`（194 scalars），catalog=88 rows/18,210 bytes、SHA-256=`eb318d91edbd0bbcd7e68ff626504a007a3f3c96d5eb60b965c9e362a421eee8`。精确 matcher、seam 与证据等级只见[`shared-busy tranches authority`](node-plugin-vfs-lock-shared-busy-tranches-authority.md)。Lock q1–q19 inventory（m/g）=`4372/4372 present,4296/3768 missing,8668/8140 total`；q12–q19=`704/704`，initialization remaining=`2728/2200`。以上均为 `source_written/source_review_only/implementation_uncompiled/implementation_unrun`，`passed=0 failed=0 actual=not_run`；coverage=`0/8668`，无 actual/reviewed digest/frozen manifest，`Qlock=unknown`、`WindowsDynamic=not_opened`、production/economic closed。19-artifact refresh 接受前必须覆盖 q15–q19；A2b2=`117/117` 不能替代该门。

历史 A2c partial bridge 不产生 `WindowsDynamic` evidence。它们只覆盖 route-exact callback-before、两个 unregister shape 和四个 direct SHM physical subset；不能替代当前五个正式 family 的完整 record，也不能外推 Map/Lock、生产 open 或其他 case。

Registration runner 的八个 selector 各自使用独立 child，actual 来自真实 lookup、route/session/lifecycle/custody 与 typed unregister receipt；parent 重验 selector、payload、child/root/registration、环境与 cleanup 后才形成 record。正式 clean-HEAD 元组、指纹与逐条结果统一见[`动态验收 § 历史前序 families 正式证据元组`](node-plugin-vfs-fault-acceptance.md#历史前序-families-正式证据元组)。该证据只关闭 RegistrationShutdown 的 8 个 case，不向其他 109 项、map/lock 或宽范围回归外推。

## 3. Test-only 拓扑与寿命

A2 fixture 必须保持以下所有权拓扑：

1. 一个测试 registration 独占一个 pinned namespace、一个 WAL/SHM runtime、一个不设为默认值的 VFS 名和一个进程寿命 registry owner；
2. 每个 SQLite Connection 各自取得不可复用的 route、opaque main logical name、authorizer context、main/sidecar leases 和 route-local callback accounting；
3. 所有 Connection 的物理 main 必须重验为同一 namespace、同一 main FileId 与同一 SHM runtime generation，但 route token、SHM connection ID 和 raw SQLite allocation 必须互不相同；
4. registration 与 callback table 必须活得比全部 Connection、raw file state、route callback 和失败 custody 更久；存在任一未关闭或隔离对象时不得正常注销或释放 context；
5. fixture 的普通析构只允许失败关闭并保留必要对象，不能伪造 Connection close、route retirement、VFS unregister 或测试根可删除。

多 Connection 解析必须由 registration 内的 exact opaque logical-name 路由完成。禁止回退到路径、canonicalize、默认 VFS、FileId 后验补救或“找不到 route 就使用唯一 route”；未知、已退休、跨 registration 或 suffix/role 不匹配一律失败关闭。

## 4. 确定性故障脚本

故障脚本只能存在于 `cfg(test)` 可达边界，并由 exact fixture/runtime/route 保管。以下是完整 A2 的目标合同。callback wrapper 仍只支持 `before_call`；低层脚本只能由 exact route 的 main wrapper 在 `promote_main_to_wal` 后、真实 `shm_map` 前，经私有 file-custody delegate 安装，target 只能从 live WAL-main 推导 runtime generation + SHM connection ID。A2b1 在原四个 teardown phase 外覆盖 `ExactSiblingOpen`、DMS exclusive/truncate/release/shared、`FileSize`/`FileGrow`、`MappingCreate`/`ViewMap` 与 `LockAcquire`/`LockRelease`；其中只读 `FileSize` 明确只允许 `before_call`，不能伪造 after-success mutation。它不暴露 raw custody、connection ID 或 pointer。未覆盖的 close/route/registration phase 不得从 A2b1 推断为已实现。每个完整注入点至少绑定：

- 不透明 fault ID、runtime generation、route/Connection 身份与 main/Journal/WAL/SHM role；
- callback kind、`ManagedSqliteShmFailurePhase`、同阶段 occurrence ordinal 与 `before_call` 或 `after_success` 时点；
- 预期 failure class、是否可能已发生 mutation、锁结果是否不确定，以及唯一允许的 SQLite result code；
- one-shot 消费状态和脱敏 observation；同一脚本不得因重试、另一 Connection 或另一测试误触第二次。

禁止使用进程全局可变布尔值、环境变量、时间竞态、随机 panic 或裸 Win32 handle 作为注入选择器。`before_call` 只能表达本次平台动作尚未发生；`after_success` 只能在真实平台成功且本地 custody 已同步后激活，并按 seam 能证明的事实分类为 `MutatedButKnown` 或 `OutcomeUncertainPoisoned`，两者都必须终态化且不能声称回滚了已执行的 OS 动作。锁竞争是正常 `BUSY`，不得用故障脚本伪造成 I/O 成功、mutation 或 poison。

脚本只控制测试平台 seam，不能改变生产分支、放宽请求校验、跳过 callback lease、直接改 coordinator state，或为测试暴露可复用的 main/SHM/file custody 构造器。A2b2 lifecycle selector 必须继续绑定 exact registration、route、role、callback/phase、occurrence 与 before/after 时点；允许 after-success 的前提是对应物理或 registry mutation 已先同步，随后立即隔离 exact route 并保留线性证据。direct unmap 的 unsafe/domain-terminal failure 与 joint-close 物理 failure 都先隔离 exact route，再尝试完成 callback；该 completion 必须记为 attempt=1/success=0，并保留一份 callback lease。`domain_terminal` 只记录 managed SHM/FileId coordinator tombstone；纯 registry rejection，以及已成功清空 SHM 后的 main unlock/file-close failure，只能 terminalize exact route/custody，不得虚报 SHM domain poison。

## 5. A2 静态矩阵

完整 A2 源码合同至少逐项表达下表。当前 A2b2 已为表中 barrier/unmap/close/registration 建立 typed static inventory；源码“表达”本身只表示静态 case/fixture 形状存在。Barrier 与 RegistrationShutdown 另有各 8 条正式 Windows dynamic record；其余 family 不得从编译或历史 targeted 结果外推动态通过：

| 路径 | 必须覆盖的阶段 | 静态不变量 |
|---|---|---|
| `xShmMap` observe/extend | exact sibling open、DMS exclusive acquire/truncate/release、DMS shared acquire、file size/grow、mapping create、view map | 输出指针失败时清零；before-mutation 可分类失败；已知 mutation 不被改写为空操作；结果不确定永久 poison 并保留 node/mapping/file custody |
| `xShmLock` shared/exclusive | request validation、local sibling contention、OS lock acquire/release | 合法 contention 只返回 `SQLITE_BUSY` 且不 poison；非法 range/action 不调用平台；unlock 结果不确定不能清本地 mask或释放 custody |
| `xShmBarrier` | callback admission、barrier、callback completion | 无 SQLite result-code 返回通道；失败必须清 raw state一次并保留 terminal custody，不能伪造为 `SQLITE_IOERR` 或正常完成；当前属于 A2b2 |
| `xShmUnmap` 非末/末连接 | held-lock gate、connection detach、view unmap、mapping close、DMS shared release、SHM file close、delete authorization/delete | 持锁 unmap 拒绝；非末连接只 detach 自己；末连接才 teardown；delete 只在 exact runtime/main identity 与 Main-EXCLUSIVE gate 下成立 |
| WAL-main 联合 `xClose` | SHM unmap、main unlock、main file close、close callback completion、route observation/retirement | 顺序固定先 SHM 后 main；任一步失败都不继续伪造后继回执；raw state只消费一次；exact leases/custody 保留或隔离 |
| registration shutdown | outstanding callback、live route、quarantined custody、VFS unregister | 任一未闭合对象阻止正常释放；注销失败保留 table/name/context；测试根只有完整成功证明后才可删除 |

每个 case 必须静态声明预期 phase、class、SQLite code、route phase、domain terminal bit、剩余 Connection 数、是否保留 node/mapping/file/main/SHM lease、是否允许后续 callback，以及 raw state/route/custody 的精确一次性计数。map/lock typed record 必须进一步把 Connection 数拆成 `sqlite_connections/shm_connections/registry_routes/logical_names`，不得继续复用一个含混数字。只断言“返回非 OK”不足以验收。

### 5.1 A2a/A2b1 map/lock static inventory 与 dynamic quotient

当前静态唯一真源是 static denominator authority：Map included `43,476`、excluded `281,085`、source universe `324,561`；Lock included `8,668`、excluded `53,774`、source universe `62,442`。旧 18 Map + 10 Lock 继续只是 `legacy_non_denominator`，不得重复相加。

动态商集不改变上述任何静态记录。它必须从完整 typed `LeafRecordV1` 与同源 typed terminal descriptor 机械生成精确、不交、非空的 class partition；不得解析 `leaf_id` 或按测试名分类。每个静态 included member 恰好属于一个 class，每个 frozen class 将来恰好形成一条真实 Windows record。typed projector 的前序存在与本批未验证的 sealed admission source 都不能产生 Q；`Qmap/Qlock` 在 exact manifests 冻结前保持 `unknown`。

Map ordinal/regions-to-create 与 Lock `first/count/mask` 在 V1 默认保留；后者参与 native byte-range offset，前者改变可观察计数。完整 class key、member commitment、DynamicExpected、代表选择、canonical digest、Windows evidence 与 atomic reducer 规则只由 dynamic quotient authority 维护。

#### 5.1.1 静态闭合前 review provenance（历史）

本小节余下的 candidate、Pending、frontier 与局部 cell 描述是 `31943fee5f7343e1194255a72805762603b320ca` 以前的审查输入。它们解释静态合同如何演进，但不得覆盖当前 frozen static authority，也不得作为新 dynamic projector 的文本分类输入。

前序 owner graph、Map terminal review ledger、ABI/raw、route/callback 与 adapter fragments 曾用于定位 source-review gaps；它们已被 static denominator authority 的完整 typed graph、leaf ledger、Expected、exclusion 与 exact-set guards 取代。历史 candidate/Pending/frontier 只保留在 Git 与实现源码中，不再复制到本聚合页，也不得用于 dynamic projector 的 leaf-text 分类。

当前动态商只允许消去 run nonce、临时根、registration/route/runtime/Connection/PID 等 harness binding；source site、operation、phase/timing/occurrence、prestate、Map ordinal、Lock `first/count/mask`、mutation/uncertainty、custody 与 observable counts 必须按独立 dynamic quotient authority 保留。

该历史 review 要求后来已由 frozen inventory 吸收；inventory 至少显式包含 ABI 参数/输出拒绝，以及六个不可互换的 map success：Extend cold-create、
Extend reuse、Observe warm-create、Observe reuse、Observe not-present；另须覆盖 cold initialization 与 warm mapping prefix，
mapping/view 的 before/native/after 结果，以及 shared/exclusive acquire/release 的本地成功、同 Connection
transition、shared coalescing、sibling shared/exclusive contention、OS acquire/release success/contended/error。normal contention 仍是
`SQLITE_BUSY` 且 fault count 为零；对满足 C memory contract 的非 null output slot，map 任一失败都必须证明 slot 保持 null；null slot没有写入位置，不能虚报一次 null write。`PlatformUnsupported`、defensive 与 inactive 分支只有在端到端可达性复核后才可进入 exclusion ledger；当前 scaffold 和 Map review ledger 的 disposition 都不是冻结排除结论。具体 family 数与 denominator 只可在 raw source trace、terminal projection、`Expected` 和 exclusion review 全部 clean 后，
由本页和[`动态验收`](node-plugin-vfs-fault-acceptance.md)共同冻结；中间候选数不是 authority。

错误后的 native cleanup 是独立 terminal leaf，不能按最初失败 phase 合并：DMS truncate 失败后的 unlock 若不确定，terminal
phase 是 `DmsExclusiveRelease` 且 custody 为 `ExclusiveOutcomeUncertain`；DMS exclusive/shared acquire 失败后的 close 若失败，
terminal phase 是 `FileClose` 并保留 quarantined handle；ViewMap before/native 失败后的 mapping close 若失败，terminal phase 是
`MappingClose` 并保留 mapping；ExactSiblingOpen 失败后的 file close 若失败，terminal phase 同样是 `FileClose`。Map review ledger 已能分别记录 original cause、returned terminal、stored poison 与 route marker，并为已审分支保存若干 cleanup rewrite；仍为 Pending 的 stored/route projection、未展开分支和缺失 trace 不允许据此宣称 terminal universe 或 map/lock denominator。

现有 `a2b1_cases` 的 18 个 map + 10 个 lock 记录只允许标记为 `static_subset/non_denominator`。它们只覆盖 phase presence、
部分 exclusive lock shape 与局部 custody，不能因数量 28、源码存在或历史 targeted 通过而升级为 frozen denominator、
`StaticContract` 完成数或 `WindowsDynamic`。新 inventory 必须逐条吸收或显式取代这 28 条语义，不能保留第二套并行 case owner。

当前 frozen `CaseKey` 只作为 dynamic class 的静态成员承诺，不再要求与 actual 一对一。真实 Windows child 仍须另行观察并绑定 exact registration/route/runtime/Connection、真实 pre/post state、平台结果、进程/环境和 cleanup receipt；静态 Expected、source branch 或 review 不能构造 `WindowsDynamic` record。

`SourceOwnerGraph v1` 的 owner snapshot 已覆盖 production ABI
`server/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_abi/io_shm.rs`；callback admission/begin/completion 与 custody 的真实 owner
`server/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/file_custody/abi.rs`、
`server/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/file_custody/operations.rs` 与
`server/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/file_custody.rs`；test-only exact route bridge
`server/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/route_file.rs`，以及 managed owner
`server/src/node_agent_managed_fs/sqlite_namespace_shm/types.rs`、`mapping.rs`、`node_initialization.rs`、`locking.rs`、
`test_faults/{api,controller,operation,mapping}.rs` 的 map/lock 路径，并覆盖 ABI callback slot/raw-state/result-code、outer fault wrapper、route plan/promotion、process-owner callback chain、failure custody、managed namespace I/O/close、Windows module selection/re-export 与 Windows map/lock typed seams。snapshot/edge 图已冻结，但该列表和图都不是 raw terminal universe。candidate scaffold 与 owner 图的 test-only source owner 是
`server/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/a2b1_cases.rs` 与其
`a2b1_cases/` 子模块；`managed_vfs.rs` 只允许保留 `cfg(test)` 模块 wiring。现有
`a2_dynamic_evidence` 是 RegistrationShutdown-specific child/record，不得因命名通用而冒充 map/lock actual。除上述 test-only
inventory 外，本批 Map review ledger 只修改 test-only `a2b1_cases` 子树与文档，不修改 ABI、managed-fs、route bridge、production open 或任何 Host 调用边。

### 5.2 A2b2 typed case schema 与完整 inventory

A2b2 不复用 A2b1 含混的单一 `remaining_connections`。每条 record 必须分开保存 `sqlite_connections`、`shm_connections`、`registry_routes` 与 `logical_names`，并包含：

- target scope 与 exact identity：route-scoped case 保存 registration ID、route ordinal、runtime generation、SHM connection ID、Main role、callback kind 与 phase occurrence；registration shutdown 只保存 registration identity，不虚构 route/runtime/SHM connection；另存 unmap mode、非末/末 topology 与 fault timing；
- SQLite channel（`xShmBarrier=VoidNoResultCode`，unmap failure=`SQLITE_IOERR`，close/close-lifecycle failure=`SQLITE_IOERR_CLOSE`，registration shutdown 与 logical-name removal 无 SQLite channel）、failure class、mutation/lock uncertainty、physical domain terminal、独立 registry route terminal、logical route 与 registration phase；
- node、view、mapping、DMS、SHM file、main file/lock owner、main/SHM/callback lease、registry entry、三 logical-name、VFS table/name/context 与 root-deletable custody；
- raw-state take/abandon/`pMethods` clear、callback begin/complete、selected action、SHM detach、main unlock/file close、registry close、connection observe、registry route remove、logical-name remove、VFS unregister 与 custody retain 的 attempt/success 精确计数；logical-name 另保存恰好三项的 removal count，fault selector 另保存 observe/trigger/pending 三项 one-shot 计数，`physical_retry` 必须恒为零；
- `StaticContract` 与 `WindowsDynamic` 是互斥 evidence kind；静态 inventory 记录只能是前者，已接受的 Barrier/RegistrationShutdown/RegistryLifecycle/Unmap/JointClose 动态记录另以 exact evidence envelope 形成后者。

静态 inventory 必须按集合相等验收，而不是只检查某 phase 至少出现一次：barrier 覆盖 callback admission、fence before/after、completion 与 success；非末 unmap 覆盖输入校验、shared/exclusive held-lock、`delete=true` 仍只 detach、detach before/after 与 completion；末连接覆盖 ViewUnmap、MappingClose、DMS shared release、SHM file close 的 before/native/after、Keep/Delete、delete authority、exact sibling delete、detach 与 completion；联合 close 必须把每条 managed-fs Keep/final 物理 unmap failure 一一投影为 `ShmUnmapLift` 并断言 main close 未开始，不得虚构 registry SHM callback-completion 投影，再覆盖 main unlock、main handle close、registry WAL-main close、唯一 close callback completion、connection observation、registry route removal 与 logical route removal；registration 覆盖 outstanding callback、live route、quarantined custody、route-index observation、unregister before/injected-pre-native/after 与完整成功。成功卸载后只有 registry route 和 logical-name 均为零、全部物理/lease custody 与 table/name/context 已释放时，测试根才可删除。

冻结源码的原十个静态 inventory 叶保持不变，另有 case-key、dynamic-registration、dynamic-barrier、dynamic-registry-lifecycle、dynamic-unmap 与 dynamic-joint-close 证据模块；每叶继续受 `<500` 行硬预算约束。source-exhaustive case 总数固定为 117，其中 Barrier 8、Unmap 49、JointClose 36、Registry lifecycle 16、Registration shutdown 8。Barrier 单列 inner registry callback 前的 generic callback-wrapper before fault，联合 close 单列 begin-close 成功后的 Close callback admission rejection；Barrier/close callback completion 均覆盖 before/native/after，non-final Keep、final Keep/Delete 与 joint-close lift 均单列 ConnectionDetach after-success-uncertain；registry retirement 分开保存 owner-retire native failure 与 retire 成功后 receipt 发布失败，logical-name removal 分开保存 retirement receipt claim 失败与 claim 后 index/custody native failure。入口已通过 `managed_vfs.rs` 的 `cfg(test)` 模块声明接入。测试目标已编译，当前 clean runtime-source commit 的宽回归基线为 `266/266`；编译和宽回归本身不重复增加动态分子，运行时源码改动后须重跑。

## 6. 同 namespace 多 Connection

多 Connection 合同固定为：

- 首个与后续 main bind 都必须对账相同物理身份，并获得不同 SHM connection ID；每个 main 的首次 `xShmMap` 只提升自己的 route-local main custody；
- 同一 region 在 node generation 不变时映射稳定；不同 Connection 可以观察同一 SHM 内容，但任何 raw pointer 只能在对应 WAL-main custody 活着时借用；
- 同 Connection 的 shared/exclusive mask 与 sibling mask 分开记录；同槽冲突返回 `SQLITE_BUSY`，释放只能消费自己确实持有的 exact range；
- 非末 Connection 的 `xShmUnmap(delete=true)` 也只分离该 Connection，不删除、关闭或重建共享 SHM；另一 Connection 的映射、锁和 route 保持可用；
- 最后一条 Connection 只有在无本地 SHM 锁时才可 teardown。SHM delete 还必须携带同一 WAL-main、runtime generation、main identity 与 Main-EXCLUSIVE 事实；
- 任一 managed SHM/FileId coordinator 内结果不确定的 map/lock/unmap/SHM close 必须把整个 FileId/domain 置为 terminal。SHM 已清空后的 main close 或纯 registry lifecycle failure 只隔离 exact route/custody，不得虚报 domain tombstone；其他 route 也不能越过真实 tombstone 继续使用或另建 runtime；
- 一条 Connection 成功关闭只退休其 exact route；不得减少另一 route 的 callback/lease 计数，也不得提前注销共享 VFS。

本批不承诺线程调度公平性、跨进程 WAL、shared-cache、临时文件、崩溃恢复或真实 SQLite 并发语句行为；这些只能由以后实际运行证据证明。

## 7. 联合关闭失败语义

SQLite `xClose` 必须先从 fresh raw allocation 消费并清除 exact state/`pMethods`，再进入一次线性物理关闭；同一 allocation 的第二次 `xClose` 必须失败且不能重试 OS 关闭。

WAL-main 若仍持有 SHM connection，关闭顺序固定为：验证无 SHM 锁、unmap/detach、必要时末连接 teardown、关闭 main 锁域与句柄、完成 close callback、观察 Connection 关闭、退休 exact route。若 SHM 阶段失败，main close 不得开始；若 SHM 已成功而 main close 失败，必须保留 main failure、runtime generation 与 registry leases；若物理关闭成功但 callback/route bookkeeping 失败，不得再次物理关闭，也不得宣称 route 已退休。

失败回调只能返回 SQLite 错误码和最小脱敏诊断。底层 typed failure、raw handle quarantine、main/SHM leases 与 route custody必须留在 registry/process domain，不能经 ABI error、Debug、serde 或测试断言泄露给调用方。

## 8. 明确禁线

- 禁止修改或调用生产 `open()`，禁止新增生产 VFS table、注册器、process owner、live route 或非空生产 `pMethods`。
- 禁止把 test registration、测试 nonce、fault script、Connection fixture、SHM pointer或关闭计数包装成 opened authority/A1 custody。
- 禁止接入 A1 producer、endpoint、v15、Signer、Plan、PlanApply、download、Sidecar、Runtime、Ready、Provider、route、Offer、Job、Attempt、outbox、Lease、派发、市场、结算或资金效果。
- 禁止把静态 case、rustfmt、源码审阅、既有 69 项测试或单 Connection 正常关闭描述成 A2 动态故障证据。
- 禁止把编译成功、筛选后的局部测试或任一失败的宽范围回归写成完整 A2 验收；每次执行必须另记命令、平台、case 数与结果。

## 9. 静态验收与后续门槛

当前静态验收已闭合为 Map `43476/43476`、Lock `8668/8668`；projector/candidate 只保留前序验证基线。Lock q12–q19 八个 initialization slices 各为 source-only 88 singleton；q18/q19 是 CreatedFirst/ExistingFirst shared-busy/close-ok 对切，均要求 same-FileId distinct holder/target 的真实 contention、attempt 分账与 explicit target close success，q19 另要求物理预建和 cold ExistingFirst，不能以注入 busy 或 `Drop` 代替。未运行 inventory（m/g）=`4372/4372 present,4296/3768 missing,8668/8140 total`，q12–q19=`704/704`，initialization remaining=`2728/2200`。current source=`source_written/source_review_only/implementation_uncompiled/implementation_unrun`，`passed=0 failed=0 actual=not_run`；无 actual/reviewed digest/frozen manifest，`Qmap/Qlock=unknown`、coverage=`0/43476 + 0/8668`、`WindowsDynamic=not_opened`、production/economic closed。19-artifact refresh 接受前须覆盖 q15–q19。只有补齐全部 program source、冻结 exact quotient manifests 并在同一 clean Windows cohort 逐 class 产生正式记录，才可形成 `Qmap/Qmap` 与 `Qlock/Qlock`；q9–q19 细节只见[`Lock tranches`](node-plugin-vfs-lock-dynamic-tranches-authority.md)。

### 9.1 静态闭合前 fragment 结论（历史）

以下两段中的“当前”只指静态闭合前的审查快照，用于 provenance；不得据此把已经 verified 的静态合同降回 pending。

route/callback 6-cell 与 adapter composed 7-cell 的可接受表述仍分别止于局部 normal-return quotient 和 reviewed control/result inventory；唯一双成功分支、payload custody、defensive guards 与更深 managed provenance 不得外推为完整 terminal。两者已随共享 A2b1 静态守卫编译并通过，但没有 fragment-specific denominator、dynamic、migration 或 runtime 证据；逐格断言由本页前文与[`动态验收`](node-plugin-vfs-fault-acceptance.md)维护。

当前可接受的结论仅是：test-only API不可从生产构造；exact registration/route只能经live WAL-main私有delegate取得低层target；commit-bound `SourceOwnerGraph v1`与Map source-terminal template review ledger v1已source-written/source-review-only、targeted guard `4/4`，ABI fragment仍止于raw dispatch，source-neutral raw fragment与Map 8-fallback/1-typed-frontier projection仍止于typed-operation/raw-fallback两个open frontier；typed outer 5-cell、route/callback 6-cell与adapter composed 7-cell都只位于frontier之后，不形成prefix successor，也不裁决managed prestate、route/callback custody或completion拒绝前丢弃/Mapped的底层payload custody。两个Map raw gate/abandon node已resolved、两个Lock sibling仍Pending，owner-graph Pending=5/resolved cross-links=9；raw-state abandon witness只补上真实Drop前的source order，不关闭frontier。map/lock candidate typed schema与不完整branch-atom scaffold也已source-written并通过自身自洽守卫，但ledger Pending/open boundaries非空，完整terminal universe/successor trace、quotient、exact key set、`SourceBranch`、`Expected`、`CaseKey`、exclusion ledger和denominator仍为`source_review_pending/not_counted`，不能记`StaticContract`或`WindowsDynamic`；Barrier、RegistrationShutdown、RegistryLifecycle、Unmap 与 JointClose runner/evidence envelope 已分别形成正式 `8/8`、`8/8`、`16/16`、`49/49`、`36/36`，A2b2 为 `117/117`。当前 clean runtime-source 宽回归基线 `266/266`，运行时源码改动后须重跑；生产`open()`、A1 producer与协议仍不可达。历史编译和5项局部通过不能重记为 Map/Lock 动态验收。

### 9.2 当前剩余门槛

进入 A1 依赖顺序的 production process owner/VFS register/open 阶段之前，仍必须按[`dynamic quotient acceptance`](node-plugin-vfs-map-lock-dynamic-quotient-acceptance.md)冻结 `Qmap/Qlock`、完整 member partition 与逐 class Windows evidence。A2b2 五族已分别为 `8/8`、`8/8`、`16/16`、`49/49`、`36/36`，合计 `117/117`；Map/Lock 静态已完成，但 quotient 与 Windows dynamic 尚未开放。任何运行时源码变更仍须在 exact clean commit 上重证受影响 targeted 与 wide regression；动态商或 Windows evidence 任一缺失，都不得把 A2 标记完成或推进生产入口。
