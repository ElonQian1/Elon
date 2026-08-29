---
title: 节点插件测试 VFS 故障合同权威
status: current
reviewed_at: 2026-08-29
owners: node, security
---

# 节点插件测试 VFS 故障合同权威

## 1. 权威范围与当前状态

本文冻结节点插件 A2 测试专用受管 SQLite VFS 的 SHM、联合关闭、多 Connection 与故障注入合同。逐 case 证据见[`动态验收`](node-plugin-vfs-fault-acceptance.md)，Barrier 8 项见[`动态权威`](node-plugin-vfs-barrier-dynamic-authority.md)；生产文件生命周期、Planning 依赖与本机 Store 分别仍由[`manifest authority`](node-plugin-manifest-catalog-authority.md)、[`planning authority`](node-plugin-planning-snapshot-authority.md)和[`local authority`](node-plugin-local-authority.md)维护。

A2b2 的 117 项静态 inventory 合同已冻结，覆盖 `xShmBarrier`、非末/末 `xShmUnmap`、WAL-main 联合 `xClose`、route lifecycle 与 VFS unregister。A2a/A2b1 的 registration exact route、共享 runtime 多 Connection 与 exact route→live WAL-main 私有脚本桥继续沿用既有边界；candidate typed schema 与显式不完整的 branch-atom scaffold之外，现有 commit-bound `SourceScope/SourceOwnerGraph v1`、Map source-terminal template review ledger、仅覆盖ABI/raw的reviewed-successor prefix、止于`AbiRawDispatch`的denominator-facing ABI fragment、source-neutral raw fragment、Map-specific raw projection fragment、typed Map outer-result fragment、route/callback fragment与adapter composed child。typed outer fragment只冻结`NotPresent/Mapped/Failure/CaughtUnwind`四类外壳结果；caught unwind按两种canonical installed-envelope Drop结果展开，因此共有5个local cell。上述局部fragment不是完整raw terminal universe、`CaseKey`集合或denominator；map/lock quotient、完整terminal projection、`SourceBranch`、`Expected`、exclusion ledger、exact key set与denominator全部仍为`source_review_pending`，`StaticContract`不计数，`WindowsDynamic`不开放。2026-08-12后续基线修复后，`elon-pc-node`完整测试目标已可编译，与当时可见性修复直接相关的fault matrix已实际运行并通过5项测试；该历史证据不能覆盖本批ledger/fragment源码。

本批另在 `TypedMapOperation` 开放前沿之后冻结一个互不并入前缀 DAG 的 Map route/operation-callback normal-return 局部商；其入口进一步限定为 outer callback-fault wrapper 已 pass 且 inner file live。该商含1个route preparation rejection、1个operation callback admission rejection，以及admission成功后的operation result×callback completion `2 × 2`，合计6个local cell。operation `Err` 在 completion `Ok|Err` 两支都优先返回 typed Failure；operation `Ok` + completion `Err` 也返回 typed Failure；只有 operation `Ok` + completion `Ok` 可以继续到 adapter projection，且该 continuation 仍是 Pending，不是 `NotPresent` 或 `Mapped`。6个cell在局部cut都保持output null、installed raw slots、cleanup none与pointer write=0；outer controller reject/selected/inner-missing与caught unwind均不在本局部商内。该增量已随测试目标编译，并进入下述共享 A2b1 静态守卫 `4/4`；这不是局部 fragment 的 denominator 或动态通过数，不改变owner-graph Pending=5/resolved cross-links=9，也不关闭 `TypedMapOperation` 或 `RawFallbackCustodyAndRouteProjection`。

紧邻该6-cell parent，本批再写入 adapter control/result→ABI projection 的 exact 7-cell reviewed inventory：前5个 Failure cell逐一继承并固定 `SQLITE_IOERR_SHMMAP`、output null、installed raw slots、cleanup none与0 pointer write；其余两格是 Observe-only `NotPresent` 与防御守卫通过后的 Observe/Extend `Mapped`。`NotPresent` 固定 `SQLITE_OK`、null与0 write；guard-pass `Mapped`固定携带非拥有型 `NonNull` pointer并由唯一 ABI arm执行1次write，managed coordinator继续保管view/mapping。前5格继续区分 callback-admission rejection、operation rejection与callback-completion rejection；其中 operation `Ok` + completion `Err` 产生后丢弃的success payload custody保持Pending，7格不再按该payload的`NotPresent|Mapped`类型细分。`AdapterRegionMismatch`、`AdapterLengthMismatch` 与 `AdapterNullPointer` 另形成3条child-local guard review：前两条仍为Pending，只有NullPointer在私有`NonNull<u8>`字段→原样accessor的reviewed type envelope内排除；三条都不进入7格，也不升级shared parent disposition或完整exclusion ledger。因此7格不是双`Ok` continuation或guard rejection路径的穷尽分割。Mapped typed wrapper create→adapter carry→ABI write按commit-bound owner source顺序锚定；这不是raw OS pointer创建、payload lifetime或动态可达性证明。mapped/dropped payload底层custody、managed prestate、route/callback custody与完整source universe仍Pending。composed child仍位于open frontier之后，不新增prefix edge，不改变owner-graph Pending=5/resolved cross-links=9，也不生成denominator key、`StaticContract`或`WindowsDynamic`。

A2c 严格 `cfg(all(test, windows))` 的源码在既有 route-exact map/lock、两个 unregister bridge 与四个 direct `xShmUnmap(false)` physical-subset runner 之外，现包含 Barrier 与 `RegistrationShutdown` 各八个 frozen selector 的进程隔离 runner、完整 actual codec/validator 与线性 evidence envelope。每个 child 只贡献一条 allow-listed bounded report line，其中 semantic actual 是 canonical 脱敏 payload，PID/nonce 与 root/registration 只作为绑定材料；libtest 的其他 bounded 输出不构成证据。parent 重新验证 exact selector/81 个字段，并把 payload、同一真实 child 的有界 wait/confirmed exit、spawn PID+nonce、canonical root、真实 registration commitment、commit/Windows/卷/SQLite 环境和同一测试根删除收据逐字绑定后，才可形成不可 Clone/Serde 的 record；退出未确认时必须保留根目录。quarantined-custody case 由 registration lifecycle owner 消费真实 table/name/context custody，不能冒充 route `TerminalQuarantine`。map/lock candidate branch-atom scaffold、source-owner graph 与 Map template ledger 已实际编译并进入 A2b1 targeted filter；18 个变化 owner 刷新后为 `4 passed / 0 failed / 1699 filtered`。这 4 项只证明 legacy subset、incomplete scaffold、owner graph 与 template ledger 自洽，没有生成或接受任何 map/lock dynamic record；map/lock 尚不存在可计数的 `StaticContract` 或可开放的 `WindowsDynamic`。Barrier 与 Registration runner 已在 exact clean commit `1d57b8d98a1fed70fe40ad6f1575f4b856226857` 上各形成 8 条唯一正式 Windows dynamic record，因此两个 family 均为 `WindowsDynamic=8/8`、A2b2=`16/117`；其余 101 项与 map/lock dynamic 仍待。当前宽回归基线为 `121/121`。A2 仍是 `implementation_not_dynamically_accepted`；2026-08-12 的编译与 5 项局部通过不能替代逐 case 证据。

A2b1 的 current source baseline 为已推送源码提交 `bc44c81b1f77814d5a49724a9d0bdac963955a6b`；本批 18 个变化 owner 的 Git blob OID 与 LF 规范化 SHA-256 已重算并在 clean evidence commit 上通过 `4/4`。失败、审阅、指纹与复测元组统一见[`动态验收 § 当前 A2b1 静态守卫证据`](node-plugin-vfs-fault-acceptance.md#当前-a2b1-静态守卫证据)；该刷新不冻结完整 denominator，也不开放 map/lock 动态验收。

生产 `ComputePluginHandleBoundAuthorityOpenIntent::open()` 必须继续固定返回 `COMPUTE_PLUGIN_HANDLE_BOUND_SQLITE_VFS_UNAVAILABLE`。A2 不提供生产 VFS 注册、process owner、live `sqlite3_file`、opened authority、A1 producer 或 v15 能力，也不改变 v14 blocked-only、Runtime stopped、`snapshot_ready=false` 与全部 side-effect false 的事实。

## 2. 已有基线与真实缺口

现有 managed-fs 已有以下低层形状：

- 一个 pinned namespace 只能消费成一个进程内 WAL/SHM runtime；runtime 按 exact main FileId 绑定多个本地 SHM connection ID，并拒绝第二个同目录 lock domain；
- SHM 内核已有固定 region budget、稳定映射、8 槽本地锁状态、DMS、OS byte-range lock、barrier、unmap/delete gate、typed failure phase 与永久 domain tombstone；
- registry/file-custody 已把 main、Journal/WAL sidecar、SHM lease、route 与 callback lease 不可拆持有；物理关闭失败会保留 exact custody，不能把 `Drop` 当作成功回执；
- 测试受管 VFS 已让单个真实 SQLite Connection 经 main、Journal/WAL、SHM 和 `xClose` 进入 exact route，并在正常关闭后退休 route、注销 VFS 和删除测试根。

A2a/A2b1 源码已把一个 registration 扩为 exact logical-name route 集合，每条 Connection 独立 route/authorizer/custody 并共享一个 WAL/SHM runtime；exact route 上的 plan 经刚提升的 live WAL-main 绑定到 runtime generation + SHM connection ID，map 初始化与 lock 平台动作也进入相邻 test-only hook。commit-bound `SourceScope/SourceOwnerGraph v1`及Map template ledger现另有ABI/raw reviewed-successor prefix、15-terminal/1-continuation ABI fragment、source-neutral 8-admission/2-post-operation/8-abandon raw fragment、Map 8-fallback/1-typed-frontier projection，以及冻结4类outer outcome/5个local cell的typed Map outer-result fragment；该typed局部表仍保留Pending provenance且不闭合两个frontier。除此之外，map/lock terminal-inventory源码仍只有candidate typed schema与一份不完整的branch-atom review scaffold。owner图与这些局部fragment都没有冻结完整Map quotient或raw source universe，也没有形成完整terminal projection、`SourceBranch`、`Expected`、`CaseKey`、exclusion ledger、`StaticContract`或dynamic actual/runner。A2b2静态源码继续把barrier、unmap、联合close、registry lifecycle与registration shutdown分成独立typed leaves；它已可编译，但尚未通过完整动态验收，仍未证明非末连接分离、末连接teardown、domain terminal后sibling行为、物理关闭一次性、注销结果或真实Win32 custody。

既有 A2c partial bridge 不产生 `WindowsDynamic` evidence。它们仍只覆盖 route-exact callback-before、两个 unregister shape 和四个 direct SHM physical subset；不能观察完整 Case 的 Connection、indexed route、main、lease、callback/action counts、root-deletable 或独立 kernel receipt，历史编译/局部测试也不能外推其他 case。

Registration runner 的八个 selector 各自使用独立 child，actual 来自真实 lookup、route/session/lifecycle/custody 与 typed unregister receipt；parent 重验 selector、payload、child/root/registration、环境与 cleanup 后才形成 record。正式 clean-HEAD 元组、指纹与逐条结果统一见[`动态验收 § 当前 RegistrationShutdown 正式证据元组`](node-plugin-vfs-fault-acceptance.md#当前-registrationshutdown-正式证据元组)。该证据只关闭 RegistrationShutdown 的 8 个 case，不向其他 109 项、map/lock 或宽范围回归外推。

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

### 5.1 A2a/A2b1 map/lock quotient 与 typed inventory

当前有 candidate quotient/typed-key schema、一份显式不完整的 branch-atom scaffold、独立 `SourceOwnerGraph v1`，以及在 owner 图之后单独接线的 Map source-terminal template review ledger v1。owner 图以基线 commit literal `623bec6ed0fde7360d1f8ed7e0eb40d1b543e1ac` 标识本次审查快照；每个 production/test owner 同时保存 Git blob OID、规范化 LF 后的 SHA-256 与 symbol sentinel，validator 从同一份规范化源码重算 Git blob OID 与 SHA-256，并拒绝 symbol 缺失。validator 不读取 `.git`，因此 baseline literal 不声称自动证明当前 checkout HEAD 等于该 commit；实际提交收尾仍须在仓库侧复核 commit→blob 映射。图节点只声明 source owner、symbol、role、operation scope、epoch、expanded/typed-outcome/pending boundary 与极少数显式 state witness，edge 只声明 call、continuation、terminal return、cleanup rewrite、quarantine、abandon、callback completion/error precedence、loop back 与跨调用 state prerequisite；它不提供 `Excluded`，也不能构造 `CaseKey`、`Expected` 或计数。

该图已固定以下顺序边界：ABI map 会先尝试把 output 清为 null，且只有 audited `Mapped` 投影可写非空指针；若 output slot 本身为 null则清零是 no-op。Map ledger 的 reviewed-successor 前缀现进一步把 region、region-size、extend 三个 scalar gate 的八种 validity shape 与 present/absent output slot 做精确 `8 × 2 = 16` 分区：七种 invalid scalar shape 都在 raw dispatch 前返回 unavailable，present slot 已先写 null、absent slot不写；唯一全 valid shape在 absent slot返回 unavailable，在 present slot继续进入 raw gate。present output 必须是 SQLite callback-owned、non-alias、对齐、可写且存活覆盖回调的独立槽；dangling、wrong-layout、unaligned、read-only 或 alias raw-state storage的指针属于 UB premise，不是有限 terminal leaf。raw阶段只把 `file=null` 建模为 rejection；非 null file必须指向 live/aligned/initialized/serialized allocation，exact methods+non-null state必须指向本模块 live envelope，悬空或伪造指针同样属于 UB premise，不得伪装成 TypeMismatch/caught-panic。raw state reject/panic 的 fallback 与成功 pointer/Acquired/Busy 投影分节点，并绑定 `result_codes` owner；outer callback-fault wrapper 先于 route；每次 main map 都先执行 route preparation，并对有 plan 与无 plan分支执行独立的 promotion callback lease（无 plan 既可能是首个 Map，也可能是既有 WalMain 的幂等路径）；promotion completion 后才允许可选 fault-plan install/record，随后真实 map operation 再取得另一份 `with_shm` callback lease；两份 callback 不共享 process/owner/state completion 节点。unsafe SHM failure 先 retain/quarantine，再尝试 completion，operation error 始终优先 completion error。cold Lock acquire 的 `WalMain promoted + node absent + prior Map pre-ensure return` 只登记为 `ScopePending` typed state prerequisite，并同时要求 promotion-complete 与 coordinator pre-ensure-return 两个输入；ledger 见证已限制为真实 pre-ensure validation/platform-return 分支，但完整 early-return universe、node-absent prestate partition 与 terminal trace 仍未闭合。Unlock 不得进入 node initialization；region MappingCreate/ViewMap success 是 `1..k` continuation/loop，最终 region selection 才返回；exact-open failure custody、DMS close 与 ViewMap before/native cleanup rewrite 必须分 owner/边记录。Windows seam 只冻结 typed outcome boundary，`PlatformUnsupported` 与具体 OS error等价类仍未裁决。

Map review ledger 把已审阅分支保存为 commit-bound owner/symbol/needle occurrence；复用 generic converter 或 cleanup helper 的记录还必须携带 caller context。它把 continuation、structural join、candidate terminal、cleanup rewrite、candidate exclusion 与 Pending 分开，并独立保存 original cause、returned terminal、stored poison phase、route marker、retention 与 occurrence/multiplicity；typed pointer create/carry 与真正 ABI output write也分开。source-neutral raw fragment 在安全 pointer/envelope premise 内把 protected call 明确分为 8 个 admission/prestate（7 个 rejection 与 1 个 expected-type continuation）、typed operation cut 后的 normal-return/caught-unwind 2 个 outcome，以及 8 个 abandon outcome；它不携带 Map step/path，也不构造 `SourceBranch`、`Expected`、`CaseKey` 或 denominator。Map-specific raw projection fragment只把该局部合同投影为 8 条通往 raw-fallback open frontier 的 continuation与1条通往 typed Map operation open frontier 的 continuation；原先合并的 rejection-or-panic 和 broad abandon-rejected 不再作为一个 source step。新增typed Map outer-result fragment从唯一expected-type raw continuation出发，exact-set冻结3个normal-return cell（NotPresent保留null并返回`SQLITE_OK`、Mapped唯一写非null pointer并返回`SQLITE_OK`、Failure保留null并返回`SQLITE_IOERR_SHMMAP`）及2个caught-unwind cell（先清raw slots，再分别对应Drop完成或Drop unwind，统一保留null并返回fallback）。其source witness另按执行顺序锁定三条“ABI result arm→raw accepted→`run_code` normal-code forward” normal chain，以及一条“outer catch→`abandon_without_unwind` catch fence→installed Drop→fallback” unwind chain；复用的wrapper/helper projection witness同时锁定caller context；primary raw gate/abandon witness保持context-free，并由site/operation scope限定。该顺序只保存frontier之后的局部source关联，不向reviewed-prefix添加edge。该5-cell局部表只约束ABI output/result-code与canonical raw-slot/cleanup cut；route、callback、managed outcome provenance、prestate、prefix mutation及payload Drop内部custody全部仍为Pending，所以`TypedMapOperation` frontier仍开放，且不得把外壳投影倒推成raw→managed的reviewed successor edge。`RawStateAccepted`与`RawStateCaughtPanic`仍不得有prefix-DAG incident edge；caught-panic 到 installed abandonment 的关联继续只保存为带 `BeyondOpenFrontier(TypedMapOperation)` 标签的 exact source cause。`Occupied` 只属于 install 路径；aborting panic、伪造 envelope 与其他 UB不进入有限cell。只有 prefix-materialized raw rejection路径的 unavailable-null exit 已知，其custody/route retention仍停在raw-fallback open frontier，因此两个open frontier都没有被关闭。图与ledger继续对Map-reachable `PendingExpansion` 做exact-set相等比较；denominator-facing ABI fragment仍只把ABI `8 × 2`局部商规范化为15个pre-raw terminal cell与1个`AbiRawDispatch` continuation，后续fragment cell绝不追加到这15个ABI terminal，也不升级为frozen Case。该表的六个authority success family仍只是prestate-partition-pending candidate。上述fragment不是完整Map successor trace、raw terminal universe、`SourceBranch`、`Expected`、`CaseKey`、exact key set或denominator，也不产生`StaticContract`/`WindowsDynamic`。

独立的 route/callback normal-return fragment 只细化上述 typed-operation 内部的一段 source review。它以 exact 6-cell 集合固定 `1 + 1 + 2 × 2` 分区，并逐字锚定 route preparation result gate、route→test bridge→ABI adapter→registry operation 四段 dispatch、callback admission、managed failure映射、条件 quarantine 在 completion 之前、completion attempt 与三个 result arm。两个 operation-error cell共享 wildcard error-precedence arm，但 `callback.complete()` 仍在 tuple 求值时各尝试一次；两个未 admission cell的 completion attempt 为0。route rejection、admission rejection、operation error和completion error只投影到 typed Failure；唯一双成功 cell止于 adapter projection Pending。promotion/plan claim、fault install/probe内部，managed cause/prestate/mutation/loop/cleanup/quarantine分支，callback owner/route custody、adapter payload 与 payload Drop custody均未由这6个cell裁决。该 source chain 仍不得写成 reviewed-prefix edge、完整 terminal、denominator case、`StaticContract` 或动态证据。

上述 parent provenance 不是互斥标签，而是五个累计轴：route/promotion/fault internals、callback owner/route custody、managed cause/prestate/retention、adapter projection control flow与adapter payload custody；每轴只能为 `NotReached` 或 `Pending`，所有已到达轴必须继续 Pending，不能因选择更深轴而隐式关闭较早轴。除 route-preparation 直接拒绝外，所有已 dispatch 的 cell 都会回到 adapter：admission/operation/completion error 经过 `.map_err(drop)?` 的 error projection，双成功携带 payload，operation `Ok` + completion `Err` 还会产生并不再转交 `Ok(value)`；因此parent其余五格的两个adapter轴都必须为Pending，再由composed child仅把reviewed control-flow及无operation/no-pointer payload分支细化为Reviewed；completion拒绝前已产生但丢弃的success payload与Mapped底层payload custody都仍Pending。source witness 另把 `OuterFaultPass` 与 ABI Failure arm→raw accepted→normal-code forward回传绑定到前五个 Failure cell，把 `ManagedSqliteRegistryRoutedCallbackLease::complete` 的 `Result<(), ManagedSqliteRegistryProcessRouteRejection>` 值域及 `finish_callback` delegate绑定到四个 admitted cell；caller context逐段限定 route、bridge、adapter、registry `shm_map` 与 `with_shm`，同 owner 内的 gate/quarantine/completion/result-arm 顺序只能在各自 symbol span 内按 owner snapshot 的 needle offset复核。unsafe-retain helper 的三项 early-return predicate→marker→retain 顺序也必须在 helper symbol span 内锁定。六条 branch chain、`UnsupportedFileRole|ShmDetached` exact-fixture exclusions与unsafe-retain conditional custody只构成三组互斥的 source-step ID inventory，三组并集必须精确覆盖该 ledger selector；这不是三个运行时分支互斥的声明。

当前明确开放的 source review 至少包含：非 null C pointer premise；raw fallback 后的 slot/custody/route retention closure；typed Map外壳结果背后的route/managed/prestate/provenance；每个 managed phase 的 controller-internal error cross-product；Windows/non-Windows cfg 与 typed platform outcome；cold/warm、first/joiner、prefix mutation、symbolic region loop 和 lifetime occurrence；DMS acquire native-error/shared-busy 的 cleanup caller×close-outcome；callback/retention/physical-domain/route-marker closure；managed node-presence defensive leaves；native cleanup rewrite 的动态可观察性。此前两个 owner/stage 图缺口已闭合：四个 budget validator 分成 `ManagedTypes` owner 的 region-size、logical-end、existing-size、mapped-total；FileSize 与 Extend-only FileGrow 各为 typed outcome seam，前者 Observe/Extend 只读，后者独占 truncate mutation/poison。既有 ABI validation resolved record exact-link两个 rejection witness，另有六个 record exact-link六个 budget step及FileSize/FileGrow各四个 site step；共享raw owner node也已按operation scope拆成Map/Lock sibling，仅将两个Map raw gate/abandon node exact-link到专属Map site，使resolved cross-link保持9、Map-reachable pending保持5，并继续由冻结expectation、图中实际`PendingExpansion`、ledger三方exact equality保持。本批typed outer-result fragment不改变这两个计数，也不把任何Pending graph node标为resolved。对应Lock raw gate/abandon node仍是`PendingExpansion`，source-neutral fragment不得被Map投影冒充为Lock review closure。`ObserveNotPresent`仍只是FileSize site ledger witness；ABI/raw之外的fault-finish/后继并列stage edge也不证明successor或branch/projection已闭合。未来source review仍须按test-only exact VFS、受支持Windows、非默认exact registration、live WAL-main、exact route/callback owner及canonical region/range/pre-state重建完整边界，但不得冒充已审定source universe。

candidate quotient 拟只消去不改变权威可观察结果的 scalar instance：registration/route/runtime/Connection 的具体值、同一 range-shape 内等价 offset、同一 topology class 内等价 region index，以及产生相同终态/custody 的等价 OS errno。以下合并轴仍待 source/red-team review：

- path、semantic source branch group 与 operation：Map 的 `Observe|Extend`，Lock 的
  `LockShared|LockExclusive|UnlockShared|UnlockExclusive`；
- request/range shape、cold/warm 与 mapped/unmapped prefix、完整 initialization path、目标 Connection pre-state、sibling shared/exclusive relation；
- phase、occurrence、`native|before_call|after_success` timing，以及 after-success 的 known/uncertain class；
- SQLite channel/result、map output 的 null/non-null 结果、mutation/lock uncertainty、physical domain 与 exact route terminal；
- before/after topology、全部 custody/mask 和 platform/fault/callback/action 精确计数。

当前 source-written 的只是 candidate `CaseKey` schema，可表达 path、branch group、operation/mode、request/range shape、topology/prefix-mutation class、initialization path、sibling relation、cause/terminal phase、timing/class 与 occurrence，且不保存真实 registration、route、runtime、Connection、PID、path、pointer 或 handle。该 schema 仍为 `source_review_pending`；源码没有已冻结的 `CaseKey` 实例集合，也没有完整 `SourceBranch`、`Expected` 或 `StaticContract`。未来这三类材料必须分开保存 source terminal identity、完整预期结果，以及 key→source→expected 的线性关联；runner、Debug、默认计数器或调用方布尔值不得在运行时拼装 expected。

candidate branch-atom scaffold 仍只允许声称一个内部性质：对它自己已 materialize 的不完整 atom 列表，每个 atom 在该表内只出现一次并获得一个 candidate disposition。Map review ledger 进一步要求自己声明的 step ID 全部 materialize、commit-bound anchor 位于对应 symbol span、共享源码分支携带 caller context，保留非空 Pending/open-boundary 集合，并对五个 pending 与九个 resolved 图边界做 exact cross-ledger 关联；ABI/raw reviewed-successor prefix另冻结自己的 case/edge/endpoint、raw slot retention及 open-frontier集合。这些自一致性不证明记录覆盖 production source，不证明记录是完整 terminal leaf，也不证明 ABI/raw之外的 disposition/reachability/exclusion 判定或 candidate axes 正确。当前已知 gaps 至少包括：

- cold `xShmLock` 也穿过完整 node-initialization 图，尚未与 lock action 完成可达性交叉投影；
- `Observe|Extend` 的 operation-specific reachability 尚未分清，single-Connection 与 multi-Connection/sibling topology 也尚未分开；
- owner/symbol 与调用顺序已有 commit-bound 图，Map review ledger 也已分类一批具体 branch；只有 ABI/raw 前缀具有显式 successor edge，registry/controller/managed/cleanup/projection 尚未组合成端到端 terminal trace；
- candidate exclusion 仍缺 exact fixture predicate 与完整 proof witness；任一 review step、success candidate 或 cleanup template都不能被当成一个 Case 或 denominator 单元。

| Artifact | 当前状态 |
|---|---|
| `SourceScope/SourceOwnerGraph v1` | design-frozen/source-written/source-review-only/validator-uncompiled/unrun; owner-graph Map Pending=5, resolved cross-links=9; Lock raw siblings pending |
| Map source-terminal template review ledger v1 | source-written/source-review-only/validator-uncompiled/unrun; ABI/raw successor prefix + ABI denominator fragment + source-neutral raw fragment + Map raw projection + typed outer-result 5-cell + route/callback 6-cell + adapter composed 7-cell fragments written; two open frontiers remain; graph exact-link written |
| candidate quotient/key schema | source-written/source_review_pending |
| candidate branch-atom scaffold | source-written/incomplete/self-partition-only |
| ABI/raw prefix之外的 raw source universe 与完整 terminal projection | source_review_pending/not counted |
| `Expected` 与 exclusion ledger | source_review_pending/not counted |
| exact key set 与 denominator | source_review_pending/not counted |
| `StaticContract` | not counted |
| `WindowsDynamic` | not opened |

未来 inventory 至少必须显式包含 ABI 参数/输出拒绝，以及六个不可互换的 map success：Extend cold-create、Extend warm-create、
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

未来经冻结的 `CaseKey` 是 dynamic actual 的 join key，但当前 candidate schema/projection 都不是 actual。真实 Windows child 仍须另行观察并绑定 exact
registration/route/runtime/Connection、真实 pre/post state、平台结果、进程/环境和 cleanup receipt；静态 expected、source branch
或 review 不能构造 `WindowsDynamic` record。

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
- `StaticContract` 与 `WindowsDynamic` 是互斥 evidence kind；静态 inventory 记录只能是前者，已接受的 Barrier/RegistrationShutdown 动态记录另以 exact evidence envelope 形成后者。

静态 inventory 必须按集合相等验收，而不是只检查某 phase 至少出现一次：barrier 覆盖 callback admission、fence before/after、completion 与 success；非末 unmap 覆盖输入校验、shared/exclusive held-lock、`delete=true` 仍只 detach、detach before/after 与 completion；末连接覆盖 ViewUnmap、MappingClose、DMS shared release、SHM file close 的 before/native/after、Keep/Delete、delete authority、exact sibling delete、detach 与 completion；联合 close 必须把每条 managed-fs Keep/final 物理 unmap failure 一一投影为 `ShmUnmapLift` 并断言 main close 未开始，不得虚构 registry SHM callback-completion 投影，再覆盖 main unlock、main handle close、registry WAL-main close、唯一 close callback completion、connection observation、registry route removal 与 logical route removal；registration 覆盖 outstanding callback、live route、quarantined custody、route-index observation、unregister before/injected-pre-native/after 与完整成功。成功卸载后只有 registry route 和 logical-name 均为零、全部物理/lease custody 与 table/name/context 已释放时，测试根才可删除。

冻结源码的原十个静态 inventory 叶保持不变，另有 case-key、dynamic-registration 与 dynamic-barrier 证据模块；每叶继续受 `<500` 行硬预算约束。source-exhaustive case 总数固定为 117，其中 Barrier 8、Unmap 49、JointClose 36、Registry lifecycle 16、Registration shutdown 8。Barrier 单列 inner registry callback 前的 generic callback-wrapper before fault，联合 close 单列 begin-close 成功后的 Close callback admission rejection；Barrier/close callback completion 均覆盖 before/native/after，non-final Keep、final Keep/Delete 与 joint-close lift 均单列 ConnectionDetach after-success-uncertain；registry retirement 分开保存 owner-retire native failure 与 retire 成功后 receipt 发布失败，logical-name removal 分开保存 retirement receipt claim 失败与 claim 后 index/custody native failure。入口已通过 `managed_vfs.rs` 的 `cfg(test)` 模块声明接入。测试目标已编译，当前宽回归基线 `121/121`；编译和宽回归本身不计作动态证据，源码改动后须重跑。

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
- 禁止接入 A1 producer、endpoint、v15、Signer、Plan、PlanApply、download、Sidecar、Runtime、Ready、route、outbox、Lease 或派发。
- 禁止把静态 case、rustfmt、源码审阅、既有 69 项测试或单 Connection 正常关闭描述成 A2 动态故障证据。
- 禁止把编译成功、筛选后的局部测试或任一失败的宽范围回归写成完整 A2 验收；每次执行必须另记命令、平台、case 数与结果。

## 9. 静态验收与后续门槛

route/callback 6-cell 与 adapter composed 7-cell 的可接受表述仍分别止于局部 normal-return quotient 和 reviewed control/result inventory；唯一双成功分支、payload custody、defensive guards 与更深 managed provenance 不得外推为完整 terminal。两者已随共享 A2b1 静态守卫编译并通过，但没有 fragment-specific denominator、dynamic、migration 或 runtime 证据；逐格断言由本页前文与[`动态验收`](node-plugin-vfs-fault-acceptance.md)维护。

当前可接受的结论仅是：test-only API不可从生产构造；exact registration/route只能经live WAL-main私有delegate取得低层target；commit-bound `SourceOwnerGraph v1`与Map source-terminal template review ledger v1已source-written/source-review-only、targeted guard `4/4`，ABI fragment仍止于raw dispatch，source-neutral raw fragment与Map 8-fallback/1-typed-frontier projection仍止于typed-operation/raw-fallback两个open frontier；typed outer 5-cell、route/callback 6-cell与adapter composed 7-cell都只位于frontier之后，不形成prefix successor，也不裁决managed prestate、route/callback custody或completion拒绝前丢弃/Mapped的底层payload custody。两个Map raw node已resolved、两个Lock sibling仍Pending，owner-graph Pending=5/resolved cross-links=9。map/lock candidate typed schema与不完整branch-atom scaffold也已source-written并通过自身自洽守卫，但ledger Pending/open boundaries非空，完整terminal universe/successor trace、quotient、exact key set、`SourceBranch`、`Expected`、`CaseKey`、exclusion ledger和denominator仍为`source_review_pending/not_counted`，不能记`StaticContract`或`WindowsDynamic`；Barrier 与 Registration runner/evidence envelope 已各形成正式 `8/8`，Unmap `0/49`、JointClose `0/36`、Registry lifecycle `0/16`，其余 101 项与 map/lock dynamic 未完成。当前宽回归基线 `121/121`，改源码后须重跑；生产`open()`、A1 producer与协议仍不可达。历史编译和5项局部通过不能重记为其余 A2 动态验收。

进入 A1 依赖顺序的生产 process owner/VFS 注册/open 阶段之前，仍必须按[`动态验收`](node-plugin-vfs-fault-acceptance.md)另批实际执行 Windows SHM map/lock/unmap、联合关闭平台故障矩阵和同 namespace 多 Connection 竞争，并把每条动态观察与静态 case key 一一对应。当前 map/lock denominator 尚未 source-review clean，因此 dynamic 尚未开放；Barrier 与 Registration 均为 8/8，但 A2b2 仍只有 16/117，其余 101 项尚未完成。宽范围回归已有 clean baseline `121/121`，但每次源码改动后必须重证；静态源码和 Windows 动态证据任一缺失，都不得把 A2 标记完成或推进生产入口。
