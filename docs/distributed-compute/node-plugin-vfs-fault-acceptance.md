---
title: 节点插件测试 VFS 故障动态验收
status: current
reviewed_at: 2026-08-29
owners: node, security
design_status: design_frozen
implementation_status: implementation_not_dynamically_accepted
verification_status: WindowsDynamic_32_of_117_Unmap_candidate_11_of_49_wide_161_of_161
---

# 节点插件测试 VFS 故障动态验收

## 1. 当前证据强度

本验收只消费 [`node-plugin-vfs-fault-authority.md`](node-plugin-vfs-fault-authority.md) 冻结的 A2 case inventory，
不创建第二套 VFS authority，也不授权生产入口。当前可记录的事实严格为：

- `design_frozen / source_written / implementation_not_dynamically_accepted`；Barrier 与 RegistrationShutdown runner 均已 `implementation_compiled / WindowsDynamic=8/8`，RegistryLifecycle runner 已 `implementation_compiled / WindowsDynamic=16/16`；Unmap SharedNonFinal 11 项已在 exact clean commit 上形成 `implementation_candidate=11/11`，但因整族 49 项原子门未满足，仍为正式 `WindowsDynamic=0/49`；Map ABI/raw reviewed-successor prefix、denominator-facing ABI fragment、source-neutral raw fragment、typed Map outer-result、route/callback与adapter composed fragment已随目标编译并进入共享 A2b1 targeted guard `4/4`，但仍为 `source_review_only / dynamic_unrun`；
- `elon-pc-node` 完整测试目标在 2026-08-12 基线修复后可编译；
- 与可见性修复直接相关的 targeted fault matrix 已运行并通过 5 项；
- A2a/A2b1 map/lock 的 commit-bound `SourceScope/SourceOwnerGraph v1` 与 Map source-terminal template review ledger v1 已 `design_frozen/source_written/source_review_only/validator_compiled/targeted_guard_4_of_4`。既有source-neutral raw fragment精确区分8个admission/prestate（7 rejection + 1 expected-type continuation）、typed operation后的2个outcome与8个abandon outcome；共享raw owner node已拆为Map/Lock sibling，只有两个Map raw gate/abandon node由专属Map site闭合，resolved cross-link保持9、Map-reachable graph pending保持5，两个Lock sibling仍Pending。Map raw projection fragment只包含8条fallback continuation与1条typed-frontier continuation；新增typed Map outer-result fragment另行exact-set冻结NotPresent、Mapped、Failure和CaughtUnwind四类外壳结果，其中caught unwind按Drop完成/Drop unwind展开，合计5个local cell。它只冻结initial null write、唯一Mapped pointer write、`SQLITE_OK`/`SQLITE_IOERR_SHMMAP`投影及canonical raw slots/cleanup；route、callback、managed outcome provenance、prestate、prefix mutation与payload Drop custody仍Pending，因此两个open frontier继续开放，reviewed-successor prefix不新增跨frontier edge。denominator-facing ABI fragment仍严格是15个pre-raw terminal cell与1个`AbiRawDispatch` continuation，后续cell不加入这15个ABI terminal。ledger仍保留Pending/open boundaries与六个prestate-pending success candidates，没有source-exhaustive terminal set或完整successor trace；candidate typed schema与显式不完整的branch-atom scaffold虽通过自身 self-consistency guard，仍仅为source-written review输入，完整terminal universe、quotient、exact key set、`SourceBranch`、`Expected`、`CaseKey`、exclusion ledger与denominator保持`source_review_pending/not_counted`，不得记`StaticContract`或开放`WindowsDynamic`；production ABI/managed-fs/route/open保持未修改；
- 本批 route/callback normal-return fragment 另在 outer callback-fault pass + live inner 的入口下，以 exact `1 + 1 + 2 × 2 = 6` cell 冻结 route preparation rejection、callback admission rejection 与 admitted operation/completion product；outer controller reject/selected/inner-missing均在集合外。operation `Err` 必须在 completion `Ok|Err` 两支都胜出；operation `Ok` + completion `Err` 也投影 typed Failure；只有双 `Ok` 可继续到 adapter projection Pending。6 个 cell 都保持 output null、installed raw slots、cleanup none、pointer write=0；caught unwind 不属于该集合。其 source witness 必须覆盖 route preparation gate、四段 operation dispatch、admission、completion attempt、error-precedence/completion-rejection/completed 三个 arm，并保持 conditional quarantine 位于 completion 之前。它已随目标编译并进入共享 A2b1 targeted guard `4/4`，但没有独立 denominator 或 dynamic record，不改变两个 open frontier、owner-graph Pending=5/resolved cross-links=9、`StaticContract` 或 `WindowsDynamic`；
- adapter composed child必须保持exact 7-cell reviewed control/result inventory：5个parent Failure逐格保持`SQLITE_IOERR_SHMMAP/null/installed/no-cleanup/0-write`，另有Observe-only NotPresent与防御守卫通过后的Observe/Extend Mapped两格。NotPresent固定`SQLITE_OK/null/0-write`；guard-pass Mapped固定`SQLITE_OK/non-null/1-write`，typed-wrapper value-flow必须逐层是`ManagedMapped=TypedPointerCreated`、`AdapterMapped=TypedPointerCarried`、`AbiMappedProjection=AbiPointerWritten`。admission、operation与completion rejection必须分型；operation `Ok` + completion `Err`还必须记录`SuccessPayloadDroppedBeforeAdapter`并保持payload custody Pending，不得把7格冒充payload-type完整分区。`AdapterRegionMismatch/LengthMismatch/NullPointer`必须恰为3条child-local guard review，disposition固定`Pending/Pending/ExcludedByNonNullTypeEnvelope`且不进入7格；因此7格不得被称为双Ok continuation或guard rejection路径的穷尽分割。source guard须锁定私有`NonNull<u8>`字段及原样accessor、ManagedMapping owner内唯一lexical constructor call、adapter两arm/一reject及ABI三arm，同时守卫shared parent ledger的Region Pending与Length/Null既有defensive disposition不变。该NullPointer排除只属于commit-bound reviewed type envelope，不是动态不可达或完整exclusion-ledger证明；dropped/mapped payload custody、managed prestate与route/callback custody继续Pending；
- A2b2 的 117 项 source-exhaustive inventory 全部仍是 `StaticContract`；Barrier 与 RegistrationShutdown 各提供 8 条、RegistryLifecycle 提供 16 条正式动态 record，因此 `WindowsDynamic=32/117`，其余 85 项待完成；Unmap 的 11 条 candidate marker 既不拆分 49 项原子分母，也不增加该正式计数；RegistryLifecycle 的 exact selector、实现与证据见 [`RegistryLifecycle 动态权威`](node-plugin-vfs-registry-lifecycle-dynamic-authority.md)；
- exact clean evidence commit `0c6fad06645f06a1d0053693b6d740dc841f03b1` 的宽范围 `sqlite_vfs_policy` 回归已真实通过 `161/161`。这不能把 Unmap candidate、其余 85 项或 Map/Lock 写成 A2 完成。

owner 图只验证 baseline commit literal 形状、从 reviewed owner bytes 重算的 Git blob OID/规范化 SHA-256、symbol presence、ABI roots、逐 operation scope 可达性，以及 wrapper/promotion/callback/cold-prefix/loop/cleanup/quarantine/result projection 的有序结构；它不读取 `.git`，不能自动证明当前 checkout HEAD 等于 baseline，也不是 exact terminal inventory。Map review ledger 验证自己声明的 step ID materialization、owner/symbol/occurrence anchor、共享分支 call context、candidate disposition、pointer-flow 分层、cause/returned/stored/route 四轴、六个 success projection witness、非空 Pending/open boundaries，以及 Map-reachable pending exact set与九个 resolved owner/stage 关联的 exact owner/symbol-or-site witness link；ABI/raw prefix另验证 exact case/edge/endpoint set、terminal无后继、open frontier与 raw slot保留轴。typed outer-result fragment的静态守卫还必须验证5-cell exact set、3 normal/2 unwind、唯一pointer write、canonical post-operation/abandon projection，以及四条有序source witness chain：三条ABI result arm→raw accepted→normal-code forward normal chain，和一条caught unwind→abandon catch fence→state-abandon witness→installed Drop→fallback unwind chain；复用的wrapper/helper projection witness必须携带exact caller context；primary raw gate/abandon witness继续保持context-free，并由site/operation scope限定。守卫还必须确认`TypedMapOperation` frontier和全部Pending provenance未被关闭。它们都不验证完整 source coverage、端到端 trace、exclusion proof 或 denominator。candidate typed schema 与不完整 branch-atom scaffold 保持既有 source-only 边界；这些 Map/Lock 源码已随目标编译，4 个 A2b1 静态自洽守卫通过，但仍无 dynamic record。严格 test-only 的 Barrier、RegistrationShutdown 与 RegistryLifecycle actual/validator、进程隔离 runner与线性 evidence envelope则已按下方正式证据元组完成编译和逐 case Windows 运行，但不改变 Map/Lock 边界。

### 当前 A2b1 静态守卫证据

- 首次真实运行：`VALIDATION_FINGERPRINT=6482de3afdddb8e8e9e97900d27489a3b5f16bbd1889360f18e8b47c026b05ca`，`2 passed / 2 failed / 1676 filtered`；owner graph 与 Map terminal ledger 均因 `source owner bytes changed after graph review` 失败。
- 漂移审阅：39 个 owner 仅 `ManagedFsRoot`、`ManagedWindowsPlatform` 两项受此前 4 个 loader 提交影响，共 38 行模块声明/重导出新增；SQLite symbols、graph node、edge 与 ledger 语义未变化。
- 当前快照刷新：source baseline 固定为已推送实现提交 `a75769029ba4abf5e30002f64846c0f7099d9ae7`；8 个既有变化 owner与新增 `AbiRawCloseWitness` owner 均重算 Git blob OID 与 LF 规范化 SHA-256，零偏差。Map/Lock graph 新增 operation-scoped state-abandon witness，fallback 保持原节点，Map-reachable Pending=5、resolved cross-links=9 不变。clean 验证在 `95d910f0dbc167138f913861efafa20ff11295cc`、`VALIDATION_FINGERPRINT=e7ea6855df7e6f0677a985d214dfcf467585e79c938c2a1e54b7ce7b6cdd4ad5` 上得到 `4 passed / 0 failed / 1722 filtered`。
- 本批 source-owner 续绑：Unmap 源码先独立固定为非自指 baseline `df38ff849d2b402bb818be51c01a11912f293a09`；后续 ledger commit `0c6fad06645f06a1d0053693b6d740dc841f03b1` 仅刷新 `FixtureFaultPlan`、`RegistryOperations`、`RegistryFileCustody`、`RegistryProcessOwner` 与 `RegistryOwner` 的 Git blob OID/LF 规范化 SHA-256。`VALIDATION_FINGERPRINT=34b96c5050f07bca172abc7509c76cf3d411d4226ad874a07947963d12d4f194`、receipt `4ed595d5938ed2d49dbafe449651b967902989b92db4f7022d28c86e9235bdf0` 得到 `4 passed / 0 failed / 1741 filtered`；owner/node/reachability、Pending=5、resolved cross-links=9 与两个 open frontier 均未扩张。
- 计数边界：这 4 项分别是 legacy non-denominator subset、incomplete branch-atom scaffold、source-owner graph 与 Map template ledger 自洽守卫；不形成完整 terminal universe、`CaseKey`、`Expected`、denominator、`StaticContract` 或 map/lock `WindowsDynamic`。

### 历史证据元组

当前可追溯的历史证据只绑定到修复提交 `db87877eb0d3712544e7cc5b30cc839047ec9be8`，原始结论见
[`external-pool-adapter-adoption-acceptance.md`](external-pool-adapter-adoption-acceptance.md#后续基线修复)：

- 编译结果：`elon-pc-node` 完整测试目标以 `--no-run` 成功；历史记录未保存完整 Cargo 命令、feature、toolchain 或平台元组；
- targeted 结果：下列 5 个 `fault_matrix.rs` 测试通过：
  `callback_fault_installation_is_bounded_unique_and_before_only`、
  `callback_fault_is_exact_to_route_role_and_occurrence`、
  `callback_fault_returns_before_call_without_invoking_inner_operation`、
  `callback_close_faults_are_exact_and_never_physically_retry_inner`、
  `same_seed_registrations_have_distinct_cross_fenced_logical_names`；
- 缺失字段：exact test command、Windows build、架构、卷/文件系统、bundled SQLite、child identity 和逐 case record 均未留存；
- 宽范围 `sqlite_vfs_policy` 回归仍失败，但该历史元组也没有保存可充当本验收动态 record 的完整失败清单。

因此这组历史证据只支撑“完整目标曾编译、5 个 targeted 测试曾通过”，不满足第 3 节 evidence record，不能计入
A2a/A2b1 map/lock 或 A2b2 的任何 `WindowsDynamic` case。后续执行必须从完整命令与平台元组重新建账。

### 当前 Barrier、RegistrationShutdown 与 RegistryLifecycle 正式证据元组

- 被测 clean HEAD：`0c6fad06645f06a1d0053693b6d740dc841f03b1`；
- Barrier：`VALIDATION_FINGERPRINT=d9c6773e6abbf18bf3e4571ad71a4e7855b34ca10fe19ac3e0f7a161db4acad3`，receipt `0b03eed10f84b1f38a670f847a84e36303842ed30d754cf2418954e48d87e9c3`；
- RegistrationShutdown：`VALIDATION_FINGERPRINT=e25f2176ffcffa6df24f552acfe3e6b6d4381248e4fdcddc251a75bd2c5a0354`，receipt `57e4b295f4ec799ba175cd2e1700ca2f8e24d40f0c115627c6760e2b3d37aa34`；
- RegistryLifecycle：`VALIDATION_FINGERPRINT=00e4b24a362dbe5c14b7ff13d91a82f618216abac2184e249bf7ac05838aa19a`，receipt `4dc32e7599e06a5a43bfedd9ce59950dd1b736e4d142d4dd65b5e96931dc28b3`；
- 环境：Windows build `10.0.26200`、`x86_64`、fixed NTFS、bundled SQLite `3.45.0`；
- 结果：Barrier 与 RegistrationShutdown 各为 `8 passed / 0 failed / 1737 filtered`，RegistryLifecycle 为 `16 passed / 0 failed / 1729 filtered`；共形成 32 条 family+selector 唯一的 `A2_WINDOWS_DYNAMIC_V2` record，全部逐字绑定上述 commit，且每条均为 `child_exit=0`、`parent_cleanup=deleted`；
- 计数：Barrier `WindowsDynamic=8/8`、RegistrationShutdown `WindowsDynamic=8/8`、RegistryLifecycle `WindowsDynamic=16/16`，A2b2 `WindowsDynamic=32/117`；其余 85 项与 A2a/A2b1 map/lock dynamic 继续 pending，A2 不升级完成状态。

此前缺少编译时 `ELON_NODE_AGENT_GIT_SHA` 的尝试、旧 fingerprint cache reuse 与 partial failure 均不计正式通过。只有上述 exact commit、三个强制执行指纹、环境和 32 条唯一 record 共同绑定的 clean-HEAD 运行计入当前 `8/8`、`8/8` 与 `16/16`。

### 当前 Unmap SharedNonFinal 实现候选证据

本批只实现冻结 Unmap 49 项中的 SharedNonFinal 11 项。它们使用真实 installed `xShmUnmap`、两个真实 SQLite Connection、exact route/SHM/custody 观察器与独立 child/root；candidate/formal 类型已隔离，候选 record 没有 `Display` 或到正式 `WindowsDynamicReportView` 的转换。

| 范围 | Validation fingerprint | Receipt | 结果 |
|---|---|---|---|
| `a2b2un1` schema/49 selector 双射 | `911f10c9bf0dd13fb217ed9d6821aa8a23691ec7c0dbfc051a9c0c5d5d49ef80` | `00dd5523bafff55ffc995da0ce6c292ebe212c6a39eb2d9c1677fb66fe3f559b` | `5 passed / 0 failed / 1740 filtered` |
| 线性 child/payload/receipt 绑定 | `d2f4d8840266501051df49c9ce18c7b9b93ee344fce6205a7eec0df13b17a275` | `404af3e750e65bd16a92d07361572d360936258d818b4bb77663616b619325cb` | `8 passed / 0 failed / 1737 filtered` |
| SharedNonFinal exact runner | `a879b1449f1c957f5a445f0a145eafd05cf9732f3d2dd54149419b5ab1e06e1f` | `b43d429d0e46259d06bd83a036ce9bcf87de22a0e6625d3d1833163275376999` | `11 passed / 0 failed / 1734 filtered` |
| managed SHM regression | `d862e2385eb0636696c4e5dd9706bea55651ce1327876bb5f14772c50a603549` | `61a8f19930e3ec59bf27da11b4882b260d9fb47a6d51d43bc118153db9fd7b71` | `11 passed / 0 failed / 1734 filtered` |
| registry regression | `7096f5452bc62f453a913acb6e768ed6ff8edc79a61c82cb8df61f2cbc87039f` | `172ba1408d16ac70ecc061855e80d63dacf112fc9e03e6d03c6d8e97e58727e3` | `45 passed / 0 failed / 1700 filtered`；隔离 child `1/1` |

- 被测 exact clean HEAD：`0c6fad06645f06a1d0053693b6d740dc841f03b1`；11 条输出均为唯一 `A2_UNMAP_IMPLEMENTATION_CANDIDATE_V1`，commit 全相同，`child_exit=0`、`parent_cleanup=deleted`，正式 `A2_WINDOWS_DYNAMIC_V2` 输出为 0。
- 提交前故意在 dirty checkout 强制运行的 `VALIDATION_FINGERPRINT=322e29f71b25ba19dc705404ac295b612e9c5f9ef64e661952296b6418ab1e4a` 中，11/11 child 与 81 字段 validator 已完成，但父层恰好只出现 11 次 `A2_UNMAP_CHECKOUT_NOT_CLEAN`；该失败元组不计通过，只证明 clean gate 失败关闭。
- 这 11 条记录只证明当前实现切片可真实运行；正式 Unmap 分子仍是原子的 `0/49`，不得写成 `11/49`。因此 A2b2 仍为 `32/117`、剩余 85；只有同一 exact clean commit 的 49 个 selector 与完整环境集合经 future family reducer 零缺失/重复/失败后，才允许一次性晋级为 `49/49`。
- 当前受控验证器合同沿用既有 A2 runner：parent 对 bounded stdout 中的 PID/nonce/root/registration/payload commitment 重验，并在运行时检查 compiled `ELON_NODE_AGENT_GIT_SHA` 等于 clean HEAD；它不是恶意 child 或敌对 build artifact 的密码学认证。验收只认 exact candidate/formal marker，不以 ambient child-mode libtest exit 代替 parent record；正式 49 项晋级前仍需保留 clean build/receipt，并可进一步升级为 peer-PID 单写者 IPC 与 artifact/input digest。

### 当前宽范围回归基线

- 被测 clean HEAD：`0c6fad06645f06a1d0053693b6d740dc841f03b1`；
- Rust 验证指纹：`VALIDATION_FINGERPRINT=2917ef9ffd7d5623387ebccc350eaf5c0649fc0bd5858d80ec065c45e2e6d069`，receipt `44a243aa8b9f315a508afcc9b089e1567ac03ea6dbec888e8928e88545f98247`；
- 命令范围：同一 PowerShell 进程先执行 `$env:ELON_NODE_AGENT_GIT_SHA = (git rev-parse HEAD).Trim()`，再执行 `scripts/validate-rust.ps1 -Domain agent-validation -Force -- test --manifest-path server/Cargo.toml --locked sqlite_vfs_policy -- --nocapture --test-threads=1`；
- 结果：主 `sqlite_vfs_policy` 集合 `161 passed / 0 failed / 1584 filtered`，同次进程隔离子运行均通过；
- 动态记录复核：同次输出含 Barrier 8 条、RegistrationShutdown 8 条与 RegistryLifecycle 16 条正式记录，以及 11 条 Unmap candidate；32 条正式 family+selector 与 11 条候选 selector 均唯一并逐字绑定上述 commit，全部为 `child_exit=0`、`parent_cleanup=deleted`，Unmap 正式 marker 为 0；
- 失败边界：同一指纹下首次未注入 `ELON_NODE_AGENT_GIT_SHA` 的强制尝试被 `A2_DYNAMIC_GIT_SHA_MISSING` 拒绝，不计正式证据；随后显式绑定提交并再次 `-Force` 的成功运行才构成本基线。指纹本身不编码该环境变量，因此必须同时核对 receipt 与记录内嵌 commit；
- 证据边界：该结果证明上述 exact clean HEAD 的宽范围回归健康。任何后续源码变更都必须在新 commit 上重新运行；不得仅凭宽回归增加 `WindowsDynamic` 计数。

## 2. Case 集合与完成条件

### 2.1 A2a/A2b1 map/lock

map/lock denominator 的唯一 quotient、scope、`CaseKey`、`SourceBranch`、`Expected` 与 legacy 28 非 denominator 边界由
[`authority §5.1`](node-plugin-vfs-fault-authority.md) 维护。本验收以后只消费 source/red-team review clean 的 frozen typed set，
不从 runner、历史 targeted 名称、intermediate candidate count 或实际可运行子集反推 denominator。

`SourceOwnerGraph v1` 是进入 terminal ledger 之前的结构门：它必须逐字匹配 reviewed owner blob/SHA/symbol，并保持两条 ABI root、逐 operation scope endpoint 可达、valid non-null Map output slot 的 fail-null、null slot no-write、独立 fallback/result-code、outer wrapper-before-route、promotion callback-before-operation callback、`ScopePending` cold Lock prior-Map witness、Unlock no-init、四段 budget 调用顺序、FileSize 只读/双操作、FileGrow mutation+poison/Extend-only、region loop、cause-separated cleanup rewrite、unsafe retention-before-completion 与 operation/completion error precedence。Map ledger 的 denominator-facing ABI fragment必须保持15个pre-raw terminal cell + 1个`AbiRawDispatch` continuation，并以两个rejection witness exact-link已扩展的`AbiMapValidation`；source-neutral raw fragment必须另行exact-set校验8个admission/prestate、2个post-operation outcome与8个abandon outcome，raw cell不得并入ABI的15个terminal。Map raw projection必须恰为8条fallback continuation + 1条typed Map operation frontier continuation；typed outer-result fragment必须恰为5个local cell：NotPresent、Mapped、Failure各1个normal-return，CaughtUnwind按installed Drop完成/Drop unwind拆2个。只有Mapped允许非null ABI write；NotPresent/Mapped返回`SQLITE_OK`，Failure和两种unwind返回`SQLITE_IOERR_SHMMAP`，normal-return保持installed raw slots，unwind先清slots再Drop。全部cell的route/managed/prestate/custody provenance与expected status必须保持Pending；两个Map raw node已resolved，两个Lock sibling仍Pending，typed-operation与raw-fallback custody/route两个open frontier都未关闭。正常返回后的`RawStateAccepted`与caught unwind后的`RawStateCaughtPanic`仍须标为`BeyondOpenFrontier(TypedMapOperation)`且不得出现在prefix DAG；caught-panic→abandon→fallback只能保存为带frontier标签的source cause，不能冒充reviewed successor。只有prefix-materialized raw rejection可进入“unavailable-null exit已知但custody/route尚未闭合”的frontier。cold witness只能引用ensure-node之前的Map返回，完整early-return universe与node-absent prestate partition仍Pending；FileSize site的`ObserveNotPresent`和ABI/raw之后的fault-finish/outer-result关联仍只有ledger/fragment事实，不构成完整branch/projection successor trace。present output slot必须callback-owned/non-alias/aligned/writable/live；非null file必须live/aligned/initialized/serialized，exact methods+state必须是本模块live envelope。违背这些premise的指针是UB，不进入有限case或terminal count。通过owner图或Map review ledger仍只允许写`source_written/source_review_only`；不能据此产生完整`SourceBranch`、`Expected`、`CaseKey`、denominator、`StaticContract`或`WindowsDynamic`。

route/callback fragment 的 exact-set 守卫还必须验证 6 个 branch 与 6 条 witness 一一对应，两个 operation-error branch 即使 completion outcome 不同也共享同一 wildcard error-precedence arm，而 completion expression 仍各求值一次；两个 admission 前失败 branch 的 completion attempt 必须为0。双成功 branch在parent中只能止于adapter projection Pending，再由独立composed child细分，不得把child冒充parent已闭合。只有route-preparation直接拒绝的cell可把adapter-control-flow与adapter-payload-custody两个轴都标为NotReached；其余五个已dispatch cell中的错误会经过adapter `.map_err(drop)?`，成功或completion拒绝还涉及返回payload，所以parent两个轴都必须为Pending。promotion/plan/fault内部、managed cause/prestate/retention、callback owner/route custody与Mapped payload底层custody必须继续标为未裁决；不得向reviewed-prefix DAG增加incident edge，也不得关闭`TypedMapOperation`或`RawFallbackCustodyAndRouteProjection`。

守卫必须把parent provenance作为五个累计 `NotReached|Pending` 轴比较，并让child只把adapter control-flow及无operation/no-pointer payload分支细化为Reviewed；completion拒绝前已产生但丢弃的success payload与Mapped payload custody都必须保持Pending，禁止互斥枚举掩盖已到达但仍未裁决的早期轴。每条 Failure chain还必须 exact-link `OuterFaultPass`→ABI Failure arm→normal-return raw accepted→normal-code forward；admitted chain必须锚定 routed callback `complete` 的 exact `Result<(), ManagedSqliteRegistryProcessRouteRejection>` 值域和 `finish_callback` delegate。route→bridge→adapter→registry→`with_shm`/`complete` 的 caller context必须逐段相等；同 owner 中 route gate先于dispatch、conditional quarantine先于completion、三个result arm位于completion attempt之后，且unsafe-retain helper必须保持三项early-return predicate→marker→retain顺序；这些顺序只能在对应 owner symbol span 内按真实 needle offset守卫，不能越过函数边界或只比较手写 ledger 顺序。六条branch chains、`UnsupportedFileRole|ShmDetached` exact-fixture exclusions与unsafe-retain conditional custody必须作为三组互斥的 source-step ID inventory，其并集与该 route/callback ledger selector精确相等；这不表示三类运行时分支互斥。每条chain的completion-attempt数量必须等于对应cell的0或1声明。

当前 denominator 计数尚未冻结：

| Case family | Source review | StaticContract | WindowsDynamic | 完成条件 |
|---|---|---|---|---|
| Map | owner graph + template ledger + ABI/raw reviewed-successor prefix + ABI denominator-facing fragment + source-neutral raw/Map projection + typed-outer 5-cell + route/callback 6-cell + adapter composed 7-cell fragments written；5 graph Pending 与2个open frontier仍非空；full exact-set review pending | not counted | not opened | 每个 frozen Map `CaseKey` 各有且只有一个通过的 Windows dynamic record，且 actual 与 expected 逐字段相等。 |
| Lock | owner graph written; exact-set review pending | not counted | not opened | 每个 frozen Lock `CaseKey` 各有且只有一个通过的 Windows dynamic record，且 actual 与 expected 逐字段相等。 |
| **A2a/A2b1 map/lock 总计** | **exact-set review pending** | **not counted** | **not opened** | static/dynamic 两次集合比较均与最终 frozen key set 精确相等。 |

只有 authority frozen typed key、source-branch projection、expected record 与 exclusion ledger 在源码中存在并通过 exact-set review，
才可写 `StaticContract=N/N`；当前 `4/4` 只证明上述静态 review artifacts 自洽，不得写成完整 denominator `passed=N`、dynamic verified 或动态接受。现有 18 map + 10 lock
仅为 `static_subset/non_denominator`，不计入未来 N 的完成数，也不允许与新 inventory 相加成 `N+28`。

static 集合验收必须同时满足：authority frozen `BTreeSet<CaseKey>` 与源码集合 exact equality；每个 in-scope source terminal
leaf 恰好投影一个 key；每个 key 有非空 `SourceBranch` 集合和唯一 `Expected`；missing、extra、duplicate、unknown、无理由排除、
同 key 不同 expected、只比较长度或手写 actual 全部失败关闭。dynamic 集合以后必须再与同一 key set exact equality，不能修改
static expected 来迁就观察结果。

### 2.2 A2b2

动态验收必须与冻结 inventory 做集合相等比较，不能只挑选可运行子集：

| Case family | 冻结数量 | 当前 WindowsDynamic | 完成条件 |
|---|---:|---:|---|
| Barrier | 8 | 8 | callback admission、before/native/after fence、completion 与 success 已全部形成唯一正式 record。 |
| Unmap | 49 | 0 | non-final/final、Keep/Delete、held-lock、detach、view/mapping/DMS/SHM file 与 delete authority 全部逐项运行。 |
| JointClose | 36 | 0 | SHM lift、main unlock/file close、callback、connection、route/logical-name retirement 全部逐项运行。 |
| Registry lifecycle | 16 | 16 | route observation/removal、logical-name claim/index/custody、quarantine 与成功清空已全部形成唯一正式 record。 |
| Registration shutdown | 8 | 8 | callback、live route、quarantine、unregister before/injected-pre-native/after 与完整成功已逐项形成唯一正式 record。 |
| **A2b2 总计** | **117** | **32** | Barrier、RegistrationShutdown 各 8 项且 RegistryLifecycle 16 项已通过；其余 85 个唯一 static case key 仍各需且只需一个通过的 Windows dynamic record。 |

Barrier、RegistrationShutdown 的 8-case runner 与 RegistryLifecycle 的 16-case runner 已在上方 exact clean HEAD 与环境中完成编译和逐项执行，32 个 family+selector 各形成一条唯一正式 record，因此三个 family 分别为 `8/8`、`8/8`、`16/16`，并把 A2b2 当前计数推进到 `32/117`。这不补足其他 85 项，也不改变 map/lock 的独立 denominator。

这里的 117 只统计 A2b2 barrier/unmap/close/registry inventory，不包含 A2a/A2b1 的 SHM map/lock。map/lock 的最终 N 与
A2b2 的 117 必须分栏记账；两套计数不得合并为一个 denominator，也不得互相补足缺失 dynamic record。

任何缺 case、重复 case、未知 case、static/dynamic key 不同构、只比较数量或把 partial bridge 记成完整 Case 都失败关闭。

## 3. 动态 evidence record

每个 Windows dynamic record 至少必须保存或在测试报告中逐字投影：

- commit、测试目标、Windows build、架构、文件系统/卷类型、bundled SQLite 版本和隔离 child identity；
- frozen case key、family、registration、route ordinal、runtime generation、SHM connection、role、callback、cause/terminal phase、
  occurrence、timing 与 unmap mode；
- 预期和实际 failure class、mutation/lock uncertainty、SQLite result code 或 `VoidNoResultCode`；
- fault selector 的 observed/triggered/pending 精确计数，以及 callback/action 的 attempted/succeeded 计数；
- before/after 的 Connection、route、logical-name、node、view、mapping、DMS、SHM file、main、lock 与 lease custody；
- physical domain tombstone、registry route terminal、registration phase、VFS table/name/context 与 root-deletable；
- child exit、parent cleanup 结果和最小脱敏诊断；不得记录 raw pointer、handle、Secret 或可复用 custody。

记录必须来自实际 callback/平台结果和受控 observer。静态 expected record、源码分支、Debug 输出、计数器默认值或测试手工
拼装的 post-state 不能冒充 actual。

`CaseKey` 本身不得携带真实 registration、route、runtime、Connection、PID、path、pointer 或 handle；这些只由 dynamic
record 的 child-scoped opaque binding 和 actual 字段证明。cleanup 改写 terminal phase/custody 时，record 必须同时保留 cause
phase 与 terminal phase，不能把 cleanup failure 归并回最初注入点或只报告最后一个通用 I/O 错误。

当前 registration source path 额外要求：child 只贡献一条 allow-listed bounded report line，其中 semantic actual 是 canonical positional payload，PID/nonce 与 opaque root/registration commitment 只作绑定材料；libtest 的其他 bounded 输出不构成证据。parent 必须用本地 frozen inventory 重新验证 selector 与全部字段，把同一 payload 的 exact bytes 和 sealed commitment 同时绑定到它线性持有的真实 Child/wait/exit、canonical root 与真实 registration identity，并在 child 退出后才采集环境、删除同一 root。最终 parent report 必须逐字投影这份已验证的 canonical actual 并同时保留 commitment；不可逆 commitment 不能替代逐字段报告。registration-level quarantined custody 必须由 lifecycle owner 真正消费 table/name/context；route 仍须保持 frozen Active/nonterminal 语义。任一 token、payload、child identity、root/registration binding 或环境元组不一致都不得形成 record。

编译并执行这组动态测试时，执行者必须在编译时设置 `ELON_NODE_AGENT_GIT_SHA=<被测 checkout 的 exact commit>`；只接受 40 或 64 位小写 hex。源码只校验该值存在且格式正确，执行账本还必须独立重证其逐字等于被测 commit；缺失、格式错误或不相等均不得形成 record。

## 4. 运行与隔离矩阵

| Case | 必须结果 |
|---|---|
| platform | 只在 `cfg(all(test, windows))` 的受支持 Windows 环境产生 `WindowsDynamic`。 |
| process isolation | outcome-uncertain、domain-terminal、registration terminal case 各自在独立 child 中运行；child 不重试 poisoned custody。 |
| exact VFS | 使用非默认、测试专用的 exact registration/VFS；默认 VFS、生产 open 或路径 facade 不参与。 |
| exact route | fault 只能经 live WAL-main 私有 delegate 绑定 exact route/runtime/SHM connection，不能用路径或全局开关选择。 |
| one shot | 每个脚本恰好 observe 一次、trigger 一次或按 expected 保持 pending；另一 Connection 和后续 callback 不得误触。 |
| real action | `before_call` 证明平台动作尚未开始；`after_success` 只有真实平台成功且 custody 已同步后才能触发。 |
| injected pre-native | `VfsUnregisterNativeRetryable` 固定产生 typed `SQLITE_BUSY`、`sqlite_call_performed=false`；只验受控 pre-native seam，不能计作真实 SQLite/native failure。 |
| cleanup | parent 只在 child 退出后清理测试根；只有完整成功 case 可以断言 root-deletable。 |

随机 panic、环境变量竞态、sleep 排序、进程全局 mutable flag、裸 Win32 handle 选择器和默认 VFS fallback 全部禁止。

## 5. SHM map、lock 与 barrier

| Case | 必须结果 |
|---|---|
| ABI validation/output | 非法 region/size/extend、null output、非法 offset/count/flags 在 managed action 前失败；map output 保持 null，callback/platform/fault count 与 expected 一致。 |
| map success | 分别证明 Extend cold-create、Extend warm-create、Extend reuse、Observe warm-create、Observe reuse、Observe not-present 六种语义；canonical region 值可 quotient，cold/warm、create/reuse 与 mapping prefix 不得互换。 |
| map failure before mutation | 输出指针清零；返回冻结 SQLite code；node/mapping/file custody 与 route phase逐项匹配 expected。 |
| map mutation known | 已完成 OS mutation 不得被描述为无副作用；本地 custody必须同步后终态化。 |
| map outcome uncertain | FileId/domain 永久 tombstone；同 domain sibling 不得重建 runtime 或继续 SHM。 |
| lock success | `LockShared/LockExclusive/UnlockShared/UnlockExclusive` 四动作分别覆盖 local success、shared coalescing、exact range/mask transition 与 OS acquire/release；不得用 exclusive shape代表 shared。 |
| local lock contention | 合法 sibling 冲突只返回 `SQLITE_BUSY`，不触发脚本、不 poison、不篡改持锁 mask。 |
| OS lock outcome | shared/exclusive sibling relation分别对账 success/contended/error；`SQLITE_BUSY` 与 I/O failure、known mutation及 uncertainty 不得合并。 |
| lock release uncertainty | 不清本地 mask，不释放对应 custody；domain terminal 与后续 sibling 行为逐项匹配。 |
| cleanup rewrite | DMS unlock/file close、mapping close或exact-open cleanup改写 terminal phase/custody时，保留 cause phase并命中独立 key；不得归并原失败 phase。 |
| barrier no-return | 通过真实无结果码通道执行；失败清 raw state一次并保留 terminal custody，不伪造 `SQLITE_IOERR` 或正常 completion。 |

map/lock foundation 的 targeted 通过只能证明对应局部路径，不得自动覆盖 Barrier、Unmap 或 JointClose case。

## 6. Unmap 与多 Connection

49 个 exact selector、`a2b2un1` payload、真实观察链和整族原子晋级门统一由[`Unmap 动态权威`](node-plugin-vfs-unmap-dynamic-authority.md)维护；本页只维护 A2 汇总验收，不创建第二套 selector 或局部分母。

至少以同一 namespace、同一 main FileId、同一 runtime generation 和两个不同 route/SHM connection 建立竞争拓扑，并证明：

- non-final `xShmUnmap(delete=true|false)` 只 detach 当前 Connection；sibling 的 mapping、lock、route 与 callback 仍可用；
- held-lock unmap 在平台 teardown 前拒绝，持锁与 custody 不发生虚假释放；
- final Keep/Delete 按 ViewUnmap → MappingClose → DMS shared release → SHM file close → optional exact sibling delete → connection detach → callback completion 的固定次序推进；
- 每个 before/native/after failure 都逐项对账已执行动作和剩余 custody，`physical_retry=0`；
- delete 仅在 exact runtime/main identity 与 Main-EXCLUSIVE authority 成立时成功；
- unsafe 或 outcome-uncertain SHM failure 终结整个 FileId/domain；纯 registry failure 不虚报 physical tombstone。

四条 direct `xShmUnmap(false)` physical-subset foundation 仍只是 partial bridge；在它们补齐完整 Case 的 Connection、indexed route、
main、lease、action count 与 root-deletable observation 之前，不能计入 49 项 Unmap 动态通过。

## 7. Joint close、route 与 registration

| Case | 必须结果 |
|---|---|
| SHM close failure | main unlock/file close 不开始；SHM failure、route、main 与 lease custody保留。 |
| main close failure | 已完成 SHM teardown不重试；main failure与精确 route custody保留。 |
| callback/registry failure after physical close | 不再次物理关闭；保留 close proof/lease并终结或隔离 exact route。 |
| successful close | raw state只消费一次，`pMethods`清空；exact route退休，sibling route与计数不减少。 |
| logical-name retirement | receipt claim、index mutation与custody释放分别按 frozen case验证，不能合并成“remove failed”。 |
| unregister failure | VFS table/name/context与registration owner继续保活；测试根不可删除。 |
| full shutdown | callbacks、routes、logical names、quarantine与物理 custody全为零后，才允许unregister成功与root-deletable。 |

SQLite `xClose` 的第二次调用、Drop、panic recovery 或测试清理不得产生第二次 OS close，也不得伪造成功 receipt。

## 8. Regression 与 production isolation

A2 完成必须同时满足：

1. A2a/A2b1 的 SHM map/lock `StaticContract=N/N` 已独立冻结且全部 `WindowsDynamic=N/N` 通过，static/dynamic 两次集合比较均零缺失、额外或失败；
2. A2b2 的 117 项 Windows dynamic records 全部通过，集合与 static inventory 精确相等；
3. 宽范围 `sqlite_vfs_policy` 回归通过，既有 69 项成功路径不得退化；
4. source-contract 继续证明 fault script、fixture、pointer、observer 与 dynamic record只在测试边界可达；
5. 生产 `ComputePluginHandleBoundAuthorityOpenIntent::open()` 仍固定返回
   `COMPUTE_PLUGIN_HANDLE_BOUND_SQLITE_VFS_UNAVAILABLE`；
6. 从 A2 test-VFS 源码到 A1 producer、v15、PlanApply、work-admission enforcement、Sidecar、Runtime、Ready、route、outbox、Lease 或派发的生产调用边仍为零。

通过 A2 只允许进入独立的 production process owner/VFS/open 设计与实现阶段。它不把测试 VFS 提升为生产 VFS，也不让
`OpenedComputePluginLocalAuthority`、Planning snapshot 或 Ready 自动可构造。

## 9. 计数、报告与状态升级

- 每次执行报告必须分开列 `compiled`、targeted tests、map/lock `StaticContract` 与 `WindowsDynamic`、A2b2 StaticContract、A2b2 WindowsDynamic及wide regression；
- targeted tests 只能按真实测试数记账，不能映射为 `WindowsDynamic` case 数；
- 一个 dynamic case 失败时保留其 case key、最小脱敏差异和失败阶段，其余已通过 case 不改写为失败或未运行；
- 只有 map/lock 独立矩阵全部通过、A2b2 117/117、宽范围回归通过且 production isolation保持时，A2 才可从
  `implementation_not_dynamically_accepted` 升级；
- 任何证据缺失、环境不明、case key漂移、观察不完整或生产入口变化都维持失败关闭。

当前正式结论仍是：历史完整目标可编译且5项targeted fault matrix曾通过；本批map/lock owner图与含ABI/raw successor prefix、source-neutral raw、Map raw projection、typed outer-result 5-cell、route/callback 6-cell及adapter composed 7-cell fragment的template ledger已`source_written/source_review_only/validator_compiled/targeted_guard_4_of_4`，owner-graph Pending=5/resolved cross-links=9且Lock raw sibling仍Pending，两个open frontier仍开放；完整terminal inventory仍为`source_review_pending/not_counted`，尚不能记`StaticContract`或开放 map/lock `WindowsDynamic`。Barrier 与 Registration runner 已在 exact clean HEAD 上分别正式通过 8 个 frozen selector，RegistryLifecycle runner 正式通过 16 个 frozen selector，三个 family 分别为 `WindowsDynamic=8/8`、`8/8`、`16/16`；Unmap SharedNonFinal 11 项已形成 implementation candidate，但正式 Unmap 仍为原子 `0/49`，所以 A2b2 仍是 `32/117`、其余 85 项与 map/lock dynamic 仍未完成。宽范围回归 clean baseline 为 `161/161`，但任何运行时源码改动后必须重跑；A2继续为`implementation_not_dynamically_accepted`。生产open、A1、v15、Runtime与Ready均未改变。

其中本批新增 route/callback normal-return 6-cell fragment 只属于上述 `source_written/source_review_only/validator_compiled/shared_a2b1_guard_passed` ledger：它没有新增 terminal、prefix edge、denominator key、静态计数或动态证据；唯一双成功分支仍是 adapter projection Pending。其余结论与计数不变。

紧邻的 adapter composed 7-cell child同样只属于`source_written/source_review_only/validator_compiled/shared_a2b1_guard_passed`：它保存exact 7-cell reviewed control/result inventory，其中两格是NotPresent与防御守卫通过后的Mapped；三条defensive guard另记为`Pending/Pending/NonNull-type-excluded` child-local review，故7格不穷尽双Ok continuation，并让completion拒绝前丢弃的success payload custody继续Pending。它没有升级shared parent disposition、关闭两个open frontier、改变owner-graph Pending=5/resolved cross-links=9，也没有形成完整exclusion ledger、successor trace、terminal universe、`StaticContract`或任何`WindowsDynamic` record。共享 A2b1 `4/4` 不得拆成该 child 的局部通过数，历史编译与5项局部测试也不得外推。
