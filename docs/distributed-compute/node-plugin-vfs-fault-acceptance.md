---
title: 节点插件测试 VFS 故障动态验收
status: current
reviewed_at: 2026-09-01
owners: node, security
design_status: design_frozen
implementation_status: implementation_not_dynamically_accepted
verification_status: MapLockProjector_prior_verified_runner_admission_and_private_execution_current_source_not_run_Q_unknown_Windows_not_opened_A2b2_117_of_117
---

# 节点插件测试 VFS 故障动态验收

## 1. 当前证据强度

本验收只消费 [`node-plugin-vfs-fault-authority.md`](node-plugin-vfs-fault-authority.md) 冻结的 A2 case inventory，
不创建第二套 VFS authority，也不授权生产入口。当前可记录的事实严格为：

- `design_frozen / implementation_not_dynamically_accepted`；Map `StaticContract=43476/43476`、Lock `StaticContract=8668/8668` 已由独立静态权威验证；typed projector/candidate 只有前序编译与测试基线，本批 sealed runner-admission current source 已写但未编译、未运行，quotient manifests 仍未冻结，`Qmap/Qlock=unknown`、`WindowsDynamic=not_opened`；Barrier 与 RegistrationShutdown runner 均为 `WindowsDynamic=8/8`，RegistryLifecycle=`16/16`，Unmap=`49/49`，JointClose=`36/36`，A2b2=`117/117`；
- `elon-pc-node` 完整测试目标在 2026-08-12 基线修复后可编译；
- 与可见性修复直接相关的 targeted fault matrix 已运行并通过 5 项；
- 静态闭合前的 `SourceScope/SourceOwnerGraph v1`、Map terminal review ledger 与局部 ABI/raw/route/callback/adapter fragments 只保留为 provenance；它们已被完整 static denominator graph、leaf ledger、Expected、exclusion 与 exact-set guards 取代，不能再把当前 Map/Lock 静态状态降回 `source_review_pending/not_counted`，也不能作为 dynamic class 的文本分类输入；
- 本批 route/callback normal-return fragment 另在 outer callback-fault pass + live inner 的入口下，以 exact `1 + 1 + 2 × 2 = 6` cell 冻结 route preparation rejection、callback admission rejection 与 admitted operation/completion product；outer controller reject/selected/inner-missing均在集合外。operation `Err` 必须在 completion `Ok|Err` 两支都胜出；operation `Ok` + completion `Err` 也投影 typed Failure；只有双 `Ok` 可继续到 adapter projection Pending。6 个 cell 都保持 output null、installed raw slots、cleanup none、pointer write=0；caught unwind 不属于该集合。其 source witness 必须覆盖 route preparation gate、四段 operation dispatch、admission、completion attempt、error-precedence/completion-rejection/completed 三个 arm，并保持 conditional quarantine 位于 completion 之前。它已随目标编译并进入共享 A2b1 targeted guard `4/4`，但没有独立 denominator 或 dynamic record，不改变两个 open frontier、owner-graph Pending=5/resolved cross-links=9、`StaticContract` 或 `WindowsDynamic`；
- adapter composed child必须保持exact 7-cell reviewed control/result inventory：5个parent Failure逐格保持`SQLITE_IOERR_SHMMAP/null/installed/no-cleanup/0-write`，另有Observe-only NotPresent与防御守卫通过后的Observe/Extend Mapped两格。NotPresent固定`SQLITE_OK/null/0-write`；guard-pass Mapped固定`SQLITE_OK/non-null/1-write`，typed-wrapper value-flow必须逐层是`ManagedMapped=TypedPointerCreated`、`AdapterMapped=TypedPointerCarried`、`AbiMappedProjection=AbiPointerWritten`。admission、operation与completion rejection必须分型；operation `Ok` + completion `Err`还必须记录`SuccessPayloadDroppedBeforeAdapter`并保持payload custody Pending，不得把7格冒充payload-type完整分区。`AdapterRegionMismatch/LengthMismatch/NullPointer`必须恰为3条child-local guard review，disposition固定`Pending/Pending/ExcludedByNonNullTypeEnvelope`且不进入7格；因此7格不得被称为双Ok continuation或guard rejection路径的穷尽分割。source guard须锁定私有`NonNull<u8>`字段及原样accessor、ManagedMapping owner内唯一lexical constructor call、adapter两arm/一reject及ABI三arm，同时守卫shared parent ledger的Region Pending与Length/Null既有defensive disposition不变。该NullPointer排除只属于commit-bound reviewed type envelope，不是动态不可达或完整exclusion-ledger证明；dropped/mapped payload custody、managed prestate与route/callback custody继续Pending；
- A2b2 的 117 项 source-exhaustive inventory 全部已有正式 `WindowsDynamic` record：Barrier 与 RegistrationShutdown 各 8 条、RegistryLifecycle 16 条、Unmap 49 条、JointClose 36 条，因此 `WindowsDynamic=117/117`；历史 Unmap 11 条 candidate marker 既不拆分 49 项原子分母，也不重复增加正式计数；各族 exact selector、实现与证据分别见 [`Barrier 动态权威`](node-plugin-vfs-barrier-dynamic-authority.md)、[`RegistryLifecycle 动态权威`](node-plugin-vfs-registry-lifecycle-dynamic-authority.md)、[`Unmap 动态权威`](node-plugin-vfs-unmap-dynamic-authority.md)与[`JointClose 动态权威`](node-plugin-vfs-joint-close-dynamic-authority.md)；
- exact clean runtime-source commit `bfa1a1180d220e9a4c8e39251414fc9a1b0a9ace` 的宽范围 `sqlite_vfs_policy` 回归已真实通过 `266/266`，关闭 A2b2；后续 static denominator delivery 又在 exact clean commit `31943fee5f7343e1194255a72805762603b320ca` 上验证 Map `43,476/43,476`、Lock `8,668/8,668`，wide 结果为 `301 passed / 0 failed / 1 expected ignored / 1592 filtered`。两者都不构成 dynamic quotient manifest、Windows class record、完整 A2 或生产入口。

owner 图只验证 baseline commit literal 形状、从 reviewed owner bytes 重算的 Git blob OID/规范化 SHA-256、symbol presence、ABI roots、逐 operation scope 可达性，以及 wrapper/promotion/callback/cold-prefix/loop/cleanup/quarantine/result projection 的有序结构；它不读取 `.git`，不能自动证明当前 checkout HEAD 等于 baseline，也不是 exact terminal inventory。Map review ledger 验证自己声明的 step ID materialization、owner/symbol/occurrence anchor、共享分支 call context、candidate disposition、pointer-flow 分层、cause/returned/stored/route 四轴、六个 success projection witness、非空 Pending/open boundaries，以及 Map-reachable pending exact set与九个 resolved owner/stage 关联的 exact owner/symbol-or-site witness link；ABI/raw prefix另验证 exact case/edge/endpoint set、terminal无后继、open frontier与 raw slot保留轴。typed outer-result fragment的静态守卫还必须验证5-cell exact set、3 normal/2 unwind、唯一pointer write、canonical post-operation/abandon projection，以及四条有序source witness chain：三条ABI result arm→raw accepted→normal-code forward normal chain，和一条caught unwind→abandon catch fence→state-abandon witness→installed Drop→fallback unwind chain；复用的wrapper/helper projection witness必须携带exact caller context；primary raw gate/abandon witness继续保持context-free，并由site/operation scope限定。守卫还必须确认`TypedMapOperation` frontier和全部Pending provenance未被关闭。它们都不验证完整 source coverage、端到端 trace、exclusion proof 或 denominator。candidate typed schema 与不完整 branch-atom scaffold 保持既有 source-only 边界；这些 Map/Lock 源码已随目标编译，4 个 A2b1 静态自洽守卫通过，但仍无 dynamic record。严格 test-only 的 Barrier、RegistrationShutdown 与 RegistryLifecycle actual/validator、进程隔离 runner与线性 evidence envelope则已按下方正式证据元组完成编译和逐 case Windows 运行，但不改变 Map/Lock 边界。

### 当前 A2b1 静态守卫证据

- 首次真实运行：`VALIDATION_FINGERPRINT=6482de3afdddb8e8e9e97900d27489a3b5f16bbd1889360f18e8b47c026b05ca`，`2 passed / 2 failed / 1676 filtered`；owner graph 与 Map terminal ledger 均因 `source owner bytes changed after graph review` 失败。
- 漂移审阅：39 个 owner 仅 `ManagedFsRoot`、`ManagedWindowsPlatform` 两项受此前 4 个 loader 提交影响，共 38 行模块声明/重导出新增；SQLite symbols、graph node、edge 与 ledger 语义未变化。
- 历史前序快照刷新：source baseline 固定为当时已推送实现提交 `a75769029ba4abf5e30002f64846c0f7099d9ae7`；8 个既有变化 owner与新增 `AbiRawCloseWitness` owner 均重算 Git blob OID 与 LF 规范化 SHA-256，零偏差。Map/Lock graph 新增 operation-scoped state-abandon witness，fallback 保持原节点，Map-reachable Pending=5、resolved cross-links=9 不变。clean 验证在 `95d910f0dbc167138f913861efafa20ff11295cc`、`VALIDATION_FINGERPRINT=e7ea6855df7e6f0677a985d214dfcf467585e79c938c2a1e54b7ce7b6cdd4ad5` 上得到 `4 passed / 0 failed / 1722 filtered`。
- 前序 JointClose source-owner 续绑：完整源码固定为变基后主线祖先中的非自指 baseline `e3663e109039f38477de4d6ab5cd57483dbd0541`；ledger evidence commit `bfa1a1180d220e9a4c8e39251414fc9a1b0a9ace` 重算全部 40 个 owner，其中 17 个随 JointClose 实现变化、23 个保持不变，226 个 anchor tuple 零缺失。`VALIDATION_FINGERPRINT=f65d234bfb21a0351862d396cc4f1a9c4030d872d68fc571ad57a14529f1e3d7`、receipt `0239e121c80711e443f2ea7059485333a770bb47c5e53a79b48962583570a518` 得到 `4 passed / 0 failed / 1854 filtered`；owner/node/reachability、Pending=5、resolved cross-links=9 与两个 Map/Lock open frontier 均未扩张。
- 前序 Map q3 source-owner 续绑基线为 `4edfcbcb32518fed8f93157b1983222f5f8ef74e`；q4 `MapRegionLoopSuccessV1` 的已知 clean source baseline 为 `10aa60fb42488854657dd30a4240ad5f949c894d`。这些只是前序 Map provenance，不是 current Lock 执行证据。
- current Lock stored-poison matcher 已覆盖 `UnsafeRetentionSucceededThenRouteUnknown` 与
  `UnsafeRetentionRouteUnknownThenRouteUnknown` 两个 completion；每族 15 profiles×88 action/range=1,320 frozen
  members。两份 TSV 各为 1,320 member rows/237,857 bytes，SHA-256 分别为
  `4da94c20e91d97a0082116879718b1ccf0271eb235ed785e65a2e36e7a949d85` 与
  `df931ad7725843098f228d07d9798d79e92f2beec4e1c23e83fc89219dfa1396`；route-unknown sibling 另有 test-only
  route-preemption actual runner/selector/payload 源码桥。本批仍是
  `source_written/source_review_only/implementation_uncompiled/implementation_unrun`，未 Cargo、未 Windows/真实运行，
  `passed=0 failed=0 actual=not_run`；不得复用前序 `4/4`、`36/36`、fingerprint 或 receipt。
- 计数边界：这 4 项分别是 legacy non-denominator subset、incomplete branch-atom scaffold、source-owner graph 与 Map template ledger 自洽守卫；不形成完整 terminal universe、`CaseKey`、`Expected`、denominator、`StaticContract` 或 map/lock `WindowsDynamic`。
- current Lock source 已推进到 q12 `LockNativeAcquireCreatedFirstExclusiveReleaseErrorV1`：CreatedFirst DMS release outcome uncertain 精确增加 88 个 singleton normalized groups，使 q1–q12 source-present=`3756 members/3756 groups`；完整 initialization umbrella=`3432 members/2904 groups`，q12 取 88/88 后 remaining=`3344/2816`。精确 matcher、production seam、`controlled_fault_actual`/natural actual、custody 与 receipt 边界只见[`Lock tranches`](node-plugin-vfs-lock-dynamic-tranches-authority.md)。q12 与既有 q5–q11 均未编译、未运行，`controlled_fault_actual=source_only_unrun`，不构成 actual、Windows record、动态接受或生产开放事实；q9–q12 source changes 对应的 19-artifact global frozen/source-owner refresh 继续独立 deferred。

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

### 历史前序 families 正式证据元组

- 被测 clean HEAD：`99fcea71214dedd4e11f21dd20ffee4f1109402c`；
- Barrier、RegistrationShutdown 与 RegistryLifecycle 均在同一强制执行宽回归中复验：`VALIDATION_FINGERPRINT=437aeb7b85dc2a7e60def3709530c18831b72dcc06c59ee0dec9e736deb70ab8`，receipt `44a243aa8b9f315a508afcc9b089e1567ac03ea6dbec888e8928e88545f98247`；
- 环境：Windows build `10.0.26200`、`x86_64`、fixed NTFS、bundled SQLite `3.45.0`；
- 结果：同次宽回归顶层为 `161 passed / 0 failed / 1584 filtered`；输出中 Barrier、RegistrationShutdown 各有 8 条、RegistryLifecycle 有 16 条 family+selector 唯一的 `A2_WINDOWS_DYNAMIC_V2` record，共 32 条，全部逐字绑定上述 commit，且每条均为 `child_exit=0`、`parent_cleanup=deleted`；
- 当时计数：Barrier `WindowsDynamic=8/8`、RegistrationShutdown `WindowsDynamic=8/8`、RegistryLifecycle `WindowsDynamic=16/16`，A2b2 `WindowsDynamic=32/117`；该历史批次不包含后来 Unmap 49/49 的实现或证据。

此前缺少编译时 `ELON_NODE_AGENT_GIT_SHA` 的尝试、旧 fingerprint cache reuse 与 partial failure 均不计正式通过。只有上述 exact commit、三个强制执行指纹、环境和 32 条唯一 record 共同绑定的 clean-HEAD 运行计入前序 families 的 `8/8`、`8/8` 与 `16/16`；该历史阶段由后文独立 Unmap formal 证据推进到 `81/117`，当前再由 JointClose formal 证据推进到 `117/117`。

### 历史 Unmap SharedNonFinal 实现候选证据

此前批次只实现冻结 Unmap 49 项中的 SharedNonFinal 11 项。它们使用真实 installed `xShmUnmap`、两个真实 SQLite Connection、exact route/SHM/custody 观察器与独立 child/root；candidate/formal 类型已隔离，候选 record 没有 `Display` 或到正式 `WindowsDynamicReportView` 的转换。以下元组保留为历史 provenance，不覆盖当前 formal 49。

| 范围 | Validation fingerprint | Receipt | 结果 |
|---|---|---|---|
| `a2b2un1` schema/49 selector 双射 | `437aeb7b85dc2a7e60def3709530c18831b72dcc06c59ee0dec9e736deb70ab8` | `44a243aa8b9f315a508afcc9b089e1567ac03ea6dbec888e8928e88545f98247` | 宽回归内 `5/5` |
| 线性 child/payload/receipt 绑定 | `437aeb7b85dc2a7e60def3709530c18831b72dcc06c59ee0dec9e736deb70ab8` | `44a243aa8b9f315a508afcc9b089e1567ac03ea6dbec888e8928e88545f98247` | 宽回归内 `8/8` |
| SharedNonFinal exact runner | `53e48d5ad854a773879ee40cb373b1d8bd8e027509baf2c63413bed2ff6082e6` | `b43d429d0e46259d06bd83a036ce9bcf87de22a0e6625d3d1833163275376999` | `11 passed / 0 failed / 1734 filtered` |
| managed SHM regression | `0e93765e006fe03c1594fb188894f91cea05d73052cfcf73039604d6be4e59d4` | `61a8f19930e3ec59bf27da11b4882b260d9fb47a6d51d43bc118153db9fd7b71` | `11 passed / 0 failed / 1734 filtered` |
| registry regression | `437aeb7b85dc2a7e60def3709530c18831b72dcc06c59ee0dec9e736deb70ab8` | `44a243aa8b9f315a508afcc9b089e1567ac03ea6dbec888e8928e88545f98247` | 宽回归覆盖既有 `45/45`；隔离 child `1/1` |

- 被测 exact clean HEAD：`99fcea71214dedd4e11f21dd20ffee4f1109402c`；11 条输出均为唯一 `A2_UNMAP_IMPLEMENTATION_CANDIDATE_V1`，commit 全相同，`child_exit=0`、`parent_cleanup=deleted`，正式 `A2_WINDOWS_DYNAMIC_V2` 输出为 0。
- 提交前故意在 dirty checkout 强制运行的 `VALIDATION_FINGERPRINT=322e29f71b25ba19dc705404ac295b612e9c5f9ef64e661952296b6418ab1e4a` 中，11/11 child 与 81 字段 validator 已完成，但父层恰好只出现 11 次 `A2_UNMAP_CHECKOUT_NOT_CLEAN`；该失败元组不计通过，只证明 clean gate 失败关闭。
- 这 11 条记录只证明当时实现切片可真实运行；它们从未把正式 Unmap 写成 `11/49`，也不与后续 formal 49 重复计数。
- 当前 49 个逐-selector 回归测试仍可各自输出一条 `A2_UNMAP_IMPLEMENTATION_CANDIDATE_V1`，因此 current wide 日志含 49 条 candidate marker；它们只扩展了候选覆盖，不是第二套 formal records，也不重复增加 numerator。
- 受控验证器合同沿用既有 A2 runner：parent 对 bounded stdout 中的 PID/nonce/root/registration/payload commitment 重验，并在运行时检查 compiled `ELON_NODE_AGENT_GIT_SHA` 等于 clean HEAD；它不是恶意 child 或敌对 build artifact 的密码学认证。验收只认 exact candidate/formal marker，不以 ambient child-mode libtest exit 代替 parent record。

### 当前 Unmap 49/49 正式证据

- 首次正式通过的 provenance commit 为 `da62f95b09287b79bc1f4c23780b95993cdd85a0`；当前同源复验绑定 exact clean HEAD `bfa1a1180d220e9a4c8e39251414fc9a1b0a9ace`，运行前后完整 porcelain（含 untracked）均为空，HEAD 未变化。
- 命令范围：绑定上述 HEAD 后，通过 `scripts/validate-rust.ps1 -Force` 对 `--bin elon-pc-node` 运行 exact filter `node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::abi::connection_fixture::managed_vfs::a2c_unmap_runner::unmap_windows_dynamic_family_49 -- --exact --nocapture --test-threads=1`。
- 当前 exact family fingerprint：`3236060c03844db467a44c199762910083478a9a6e39d1fe9b5d3234964b3c65`；external validation receipt profile `1402c0e3ec6e88b904e1ef2d5a4e03f92e33c63801af39e7ea73be034a1052a8`；强制 fresh build/run，未复用旧 receipt。
- 环境：Windows build `10.0.26200`、`x86_64`、fixed NTFS、bundled SQLite `3.45.0`。
- 机械结果：authority 49-selector 集合、顺序与唯一性全部一致；每条 wire 恰好 83 tokens，版本 `a2b2un1`，selector 与 case 绑定，81 个数值均为 canonical `u64`；49 个 actual commitment 全部重算匹配，child/root/registration commitment 各 49 unique。
- 当前运行结果：49 条均为 `child_exit=0`、`parent_cleanup=deleted`；family marker 恰好 1 条，`cases=49`、commit 匹配、`checkout=clean`，cohort `sha256:4a05aea74a678c8b7c97e99593c7acae278f2fe1a42cced6f4806040f8f2f14f` 与 family seal `sha256:752ace27902a46c77d4b3a66d25a31e9ad043e7d9280bcb70fa8eaeb315840f8` 独立重算一致；outer libtest `1 passed / 0 failed / 1857 filtered`，candidate marker 与 child marker 泄漏均为 0。
- 证明边界：Delete 请求变体证明 shared evaluator 的错误参数拒绝与 untouched 正确请求重检，不声称上游 ABI 自然携带畸形值；post-raw SQL receipt 只证明 exact `Connection`/预编译常量 VM 仍可 step，不证明 pager、数据库、VFS 或退休 SHM 可用。

### 当前 JointClose 36/36 正式证据

- 被测 exact clean runtime-source HEAD：`bfa1a1180d220e9a4c8e39251414fc9a1b0a9ace`；正式 family 捕获前后 checkout 均为 clean，记录内 commit 与编译时 `ELON_NODE_AGENT_GIT_SHA` 逐字相等。
- 命令范围：通过 `scripts/validate-rust.ps1 -Force` 对 `--bin elon-pc-node` 运行精确函数过滤器 `node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::abi::connection_fixture::managed_vfs::a2b2_joint_close_runner::joint_close_windows_dynamic_family_36 -- --nocapture --test-threads=1`。
- validation fingerprint：`eea57714b0e970062726b044fb24e8e8a02a27c0a6a22204dfc13fa264c3ba3f`；external validation receipt：`1c33cfadd8f0c80383fd8c0165be869a22e17b1f09f3bfae4e5d533ce5f683b6`；强制 fresh build/run，未复用旧 receipt。
- 环境：Windows build `10.0.26200`、`x86_64`、fixed NTFS、bundled SQLite `3.45.0`。
- 机械结果：36 个 frozen selector 各出现一次，零 missing/extra/duplicate/failed；每个 child 使用独立 PID/nonce/root/registration，canonical actual 与 commitment 由 parent 重算并与 real installed `xClose`、Windows native receipt、allocation-bound raw witness、SHM/main/route/callback custody及 terminal ledger 对账。
- family marker：`cases=36/36`、`checkout=clean`，cohort `sha256:c6954ed6eff83342e27125b4c3bab2ca37778358eb25e30fd1cd206611f9ca3f`，seal `sha256:b0e6407b25cb2dd4d679fed4c9712bf818d88adec0023fd419723baead158f17`，clean-commit fingerprint `sha256:e48ad03b8362d4ddde97facd557d88fa125f758d37dae462afbc1be40dc83911`；outer libtest `1 passed / 0 failed / 1857 filtered`。
- 同一 runtime-source commit 上，Unmap `49/49`（fingerprint `3236060c03844db467a44c199762910083478a9a6e39d1fe9b5d3234964b3c65`）、managed SHM `11/11`（`f92bf48597250d0af5019a6224cc0c579a2deb4af306ff2d365656becaa4f994`）、A2b1 owner guard `4/4`、production target `cargo check`（`2a68ba62bb304eb76b8a5da6e910177cbe2bd301336369519d80be27a1681a43`）及下述宽回归均通过。JointClose 因而原子从 `0/36` 晋级 `36/36`，A2b2 从 `81/117` 晋级 `117/117`；Map/Lock 与生产激活不在这次晋级内。

### 当前宽范围回归基线

- 被测 clean runtime-source HEAD：`bfa1a1180d220e9a4c8e39251414fc9a1b0a9ace`；
- Rust 验证指纹：`VALIDATION_FINGERPRINT=76060efb1b4d5d5f076d74fb3771a1e0fd5eb9bd0d85e087ae32cb8b2139c19a`，receipt `99df40f7cf016514805ac6e731380cb35b029120c2259b0ce2539591b0ba8ebd`；
- 命令范围：同一 PowerShell 进程先执行 `$env:ELON_NODE_AGENT_GIT_SHA = (git rev-parse HEAD).Trim()`，再执行 `scripts/validate-rust.ps1 -Force -- test --manifest-path server/Cargo.toml --locked --bin elon-pc-node sqlite_vfs_policy -- --nocapture --test-threads=1`；
- 结果：主 `sqlite_vfs_policy` 集合 `266 passed / 0 failed / 1592 filtered`，同次进程隔离子运行均通过；
- 动态记录复核：同次输出复验 Barrier、RegistrationShutdown、RegistryLifecycle、Unmap 与 JointClose formal family；A2b2 共 117 个唯一 selector 全部通过，Unmap 与 JointClose 各有一条 `checkout=clean` family seal并逐字绑定上述 commit，记录均为 `child_exit=0`、`parent_cleanup=deleted`。逐-selector candidate marker只属于非正式回归通道，不重复增加 numerator；
- 失败边界：必须显式绑定提交并 `-Force`，且同时核对 receipt、记录内嵌 commit 与 clean family seal；fingerprint 本身不编码 `ELON_NODE_AGENT_GIT_SHA`，不能脱离这些字段单独充当动态证据。一次未限定 `--bin elon-pc-node` 的尝试在编译无关服务二进制时发生 LLVM 内存不足、测试尚未运行，因此不计证据；
- 证据边界：该结果证明上述 exact clean HEAD 的宽范围回归健康。任何后续相关运行时源码变更都必须在新 commit 上重新运行；纯文档或只约束文档字段的 source-contract guard 更新不改变该运行时基线。不得仅凭宽回归增加 `WindowsDynamic` 计数。

## 2. Case 集合与完成条件

### 2.1 A2a/A2b1 map/lock

Map/Lock 静态 scope、`CaseKey`、`SourceBranch`、`Expected`、exclusion 与 legacy 28 非 denominator 边界只由
[`static denominator authority`](node-plugin-vfs-map-lock-static-denominator-authority.md) 维护。本验收消费已验证的 Map 43,476 与 Lock 8,668 included exact set，不从 runner、历史 targeted 名称或可运行子集反推或缩小它。

动态执行集合由独立[`dynamic quotient authority`](node-plugin-vfs-map-lock-dynamic-quotient-authority.md)和
[`acceptance`](node-plugin-vfs-map-lock-dynamic-quotient-acceptance.md)维护。它必须把全部 included members 精确、不交地分入 typed classes，冻结 `Qmap/Qlock` 后每个 class 恰好产生一条真实 Windows record。typed projector/candidate 有前序基线；sealed runner-admission current source 为未编译、未运行，manifest 尚未冻结，两个 Q 均为 `unknown`。

| Family | StaticContract | DynamicQuotientMemberCoverage | WindowsDynamic | 当前门槛 |
|---|---:|---:|---:|---|
| Map | `43476/43476` | `0/43476` | `not_opened` | 验证 current source，补齐其余 `42,955` 个 program，冻结 exact class/member manifest，机械得到 `Qmap` |
| Lock | `8668/8668` | `0/8668` | `not_opened` | q12 `LockNativeAcquireCreatedFirstExclusiveReleaseErrorV1`=88 exact singleton；unrun inventory members/groups=`3756/3756 present, 4912/4384 missing, 8668/8140 total`；initialization umbrella=`3432/2904`、remaining=`3344/2816`；无 actual，`Qlock=unknown`；q9–q12 source changes 对应的 19-artifact refresh 独立 deferred；详见 [Lock tranches](node-plugin-vfs-lock-dynamic-tranches-authority.md) |
| **Map/Lock aggregate** | **verified** | **not_started** | **not_opened** | **逐 class Windows exact-set 后才可形成 `Q/Q`** |

#### 2.1.1 静态闭合前 review provenance（历史）

以下 owner graph、局部 fragment、Pending/frontier 与 candidate 表仅解释静态合同的演进；它们不再控制当前静态计数，也不得被 dynamic projector 通过解析文本复用。

`SourceOwnerGraph v1` 是进入 terminal ledger 之前的结构门：它必须逐字匹配 reviewed owner blob/SHA/symbol，并保持两条 ABI root、逐 operation scope endpoint 可达、valid non-null Map output slot 的 fail-null、null slot no-write、独立 fallback/result-code、outer wrapper-before-route、promotion callback-before-operation callback、`ScopePending` cold Lock prior-Map witness、Unlock no-init、四段 budget 调用顺序、FileSize 只读/双操作、FileGrow mutation+poison/Extend-only、region loop、cause-separated cleanup rewrite、unsafe retention-before-completion 与 operation/completion error precedence。Map ledger 的 denominator-facing ABI fragment必须保持15个pre-raw terminal cell + 1个`AbiRawDispatch` continuation，并以两个rejection witness exact-link已扩展的`AbiMapValidation`；source-neutral raw fragment必须另行exact-set校验8个admission/prestate、2个post-operation outcome与8个abandon outcome，raw cell不得并入ABI的15个terminal。Map raw projection必须恰为8条fallback continuation + 1条typed Map operation frontier continuation；typed outer-result fragment必须恰为5个local cell：NotPresent、Mapped、Failure各1个normal-return，CaughtUnwind按installed Drop完成/Drop unwind拆2个。只有Mapped允许非null ABI write；NotPresent/Mapped返回`SQLITE_OK`，Failure和两种unwind返回`SQLITE_IOERR_SHMMAP`，normal-return保持installed raw slots，unwind先清slots再Drop。全部cell的route/managed/prestate/custody provenance与expected status必须保持Pending；两个Map raw node已resolved，两个Lock sibling仍Pending，typed-operation与raw-fallback custody/route两个open frontier都未关闭。正常返回后的`RawStateAccepted`与caught unwind后的`RawStateCaughtPanic`仍须标为`BeyondOpenFrontier(TypedMapOperation)`且不得出现在prefix DAG；caught-panic→abandon→fallback只能保存为带frontier标签的source cause，不能冒充reviewed successor。只有prefix-materialized raw rejection可进入“unavailable-null exit已知但custody/route尚未闭合”的frontier。cold witness只能引用ensure-node之前的Map返回，完整early-return universe与node-absent prestate partition仍Pending；FileSize site的`ObserveNotPresent`和ABI/raw之后的fault-finish/outer-result关联仍只有ledger/fragment事实，不构成完整branch/projection successor trace。present output slot必须callback-owned/non-alias/aligned/writable/live；非null file必须live/aligned/initialized/serialized，exact methods+state必须是本模块live envelope。违背这些premise的指针是UB，不进入有限case或terminal count。通过owner图或Map review ledger仍只允许写`source_written/source_review_only`；不能据此产生完整`SourceBranch`、`Expected`、`CaseKey`、denominator、`StaticContract`或`WindowsDynamic`。

route/callback fragment 的 exact-set 守卫还必须验证 6 个 branch 与 6 条 witness 一一对应，两个 operation-error branch 即使 completion outcome 不同也共享同一 wildcard error-precedence arm，而 completion expression 仍各求值一次；两个 admission 前失败 branch 的 completion attempt 必须为0。双成功 branch在parent中只能止于adapter projection Pending，再由独立composed child细分，不得把child冒充parent已闭合。只有route-preparation直接拒绝的cell可把adapter-control-flow与adapter-payload-custody两个轴都标为NotReached；其余五个已dispatch cell中的错误会经过adapter `.map_err(drop)?`，成功或completion拒绝还涉及返回payload，所以parent两个轴都必须为Pending。promotion/plan/fault内部、managed cause/prestate/retention、callback owner/route custody与Mapped payload底层custody必须继续标为未裁决；不得向reviewed-prefix DAG增加incident edge，也不得关闭`TypedMapOperation`或`RawFallbackCustodyAndRouteProjection`。

守卫必须把parent provenance作为五个累计 `NotReached|Pending` 轴比较，并让child只把adapter control-flow及无operation/no-pointer payload分支细化为Reviewed；completion拒绝前已产生但丢弃的success payload与Mapped payload custody都必须保持Pending，禁止互斥枚举掩盖已到达但仍未裁决的早期轴。每条 Failure chain还必须 exact-link `OuterFaultPass`→ABI Failure arm→normal-return raw accepted→normal-code forward；admitted chain必须锚定 routed callback `complete` 的 exact `Result<(), ManagedSqliteRegistryProcessRouteRejection>` 值域和 `finish_callback` delegate。route→bridge→adapter→registry→`with_shm`/`complete` 的 caller context必须逐段相等；同 owner 中 route gate先于dispatch、conditional quarantine先于completion、三个result arm位于completion attempt之后，且unsafe-retain helper必须保持三项early-return predicate→marker→retain顺序；这些顺序只能在对应 owner symbol span 内按真实 needle offset守卫，不能越过函数边界或只比较手写 ledger 顺序。六条branch chains、`UnsupportedFileRole|ShmDetached` exact-fixture exclusions与unsafe-retain conditional custody必须作为三组互斥的 source-step ID inventory，其并集与该 route/callback ledger selector精确相等；这不表示三类运行时分支互斥。每条chain的completion-attempt数量必须等于对应cell的0或1声明。

以下是静态闭合前的历史状态表，已被上述 current table 与 static denominator authority 取代：

| Case family | Source review | StaticContract | WindowsDynamic | 完成条件 |
|---|---|---|---|---|
| Map | owner graph + fragments written；当时仍有 Pending/frontier | historical not counted（已取代） | historical not opened | 已取代：当前使用 exact quotient member partition + 每 class 一条 record。 |
| Lock | owner graph written；当时 exact-set review pending | historical not counted（已取代） | historical not opened | 已取代：当前使用 exact quotient member partition + 每 class 一条 record。 |
| **A2a/A2b1 map/lock 总计** | **historical review pending（已取代）** | **historical only** | **not opened** | **当前合同见本节 current table。** |

只有 authority frozen typed key、source-branch projection、expected record 与 exclusion ledger 在源码中存在并通过 exact-set review，
才可写 `StaticContract=N/N`；当前 `4/4` 只证明上述静态 review artifacts 自洽，不得写成完整 denominator `passed=N`、dynamic verified 或动态接受。现有 18 map + 10 lock
仅为 `static_subset/non_denominator`，不计入未来 N 的完成数，也不允许与新 inventory 相加成 `N+28`。

历史 static 集合验收要求后来已由 frozen static guards 完成：authority frozen `BTreeSet<CaseKey>` 与源码集合 exact equality；每个 in-scope source terminal
leaf 恰好投影一个 key；每个 key 有非空 `SourceBranch` 集合和唯一 `Expected`；missing、extra、duplicate、unknown、无理由排除、
同 key 不同 expected、只比较长度或手写 actual 全部失败关闭。当前 dynamic exact-set 法则不是“一条静态 key 一条 record”，而是 class member sets 的不交并集精确等于该 frozen key set；不能修改 static expected 来迁就观察结果。

### 2.2 A2b2

动态验收必须与冻结 inventory 做集合相等比较，不能只挑选可运行子集：

| Case family | 冻结数量 | 当前 WindowsDynamic | 完成条件 |
|---|---:|---:|---|
| Barrier | 8 | 8 | callback admission、before/native/after fence、completion 与 success 已全部形成唯一正式 record。 |
| Unmap | 49 | 49 | non-final/final、Keep/Delete、held-lock、detach、view/mapping/DMS/SHM file 与 delete authority 已全部逐项形成唯一正式 record。 |
| JointClose | 36 | 36 | SHM lift、main unlock/file close、callback、connection、route/logical-name retirement 已全部逐项形成唯一正式 record。 |
| Registry lifecycle | 16 | 16 | route observation/removal、logical-name claim/index/custody、quarantine 与成功清空已全部形成唯一正式 record。 |
| Registration shutdown | 8 | 8 | callback、live route、quarantine、unregister before/injected-pre-native/after 与完整成功已逐项形成唯一正式 record。 |
| **A2b2 总计** | **117** | **117** | 五个 family 的 static/dynamic key 集合精确相等，零缺失、额外、重复或失败。 |

Barrier、RegistrationShutdown 的 8-case runner、RegistryLifecycle 的 16-case runner、Unmap 的 49-case runner 与 JointClose 的 36-case runner 已完成编译和逐项执行，因此五个 family 分别为 `8/8`、`8/8`、`16/16`、`49/49`、`36/36`，并把 A2b2 当前计数推进到 `117/117`。这不补足或改变 map/lock 的独立 denominator。

这里的 117 只统计 A2b2 barrier/unmap/close/registry inventory，不包含 A2a/A2b1 的 SHM map/lock。map/lock 的最终 N 与
A2b2 的 117 必须分栏记账；两套计数不得合并为一个 denominator，也不得互相补足缺失 dynamic record。

任何缺 case、重复 case、未知 case、static/dynamic key 不同构、只比较数量或把 partial bridge 记成完整 Case 都失败关闭。

## 3. 动态 evidence record

每个 Windows dynamic record 至少必须保存或在测试报告中逐字投影：

- commit、测试目标、Windows build、架构、文件系统/卷类型、bundled SQLite 版本和隔离 child identity；
- frozen evidence identity：A2b2 保存 case key；Map/Lock 保存 quotient class ID、representative 与完整 member-set commitment；另存 family、registration、route ordinal、runtime generation、SHM connection、role、callback、cause/terminal phase、
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
| map success | 分别证明 Extend cold-create、Extend warm-create、Extend reuse、Observe warm-create、Observe reuse、Observe not-present 六种语义；Map ordinal/regions-to-create 在 V1 默认保留，只有版本化可执行全域证明才可消去；cold/warm、create/reuse 与 mapping prefix 不得互换。 |
| map failure before mutation | 输出指针清零；返回冻结 SQLite code；node/mapping/file custody 与 route phase逐项匹配 expected。 |
| map mutation known | 已完成 OS mutation 不得被描述为无副作用；本地 custody必须同步后终态化。 |
| map outcome uncertain | FileId/domain 永久 tombstone；同 domain sibling 不得重建 runtime 或继续 SHM。 |
| lock success | `LockShared/LockExclusive/UnlockShared/UnlockExclusive` 四动作分别覆盖 local success、shared coalescing、exact range/mask transition 与 OS acquire/release；不得用 exclusive shape代表 shared。 |
| local lock contention | 合法 sibling 冲突只返回 `SQLITE_BUSY`，不触发脚本、不 poison、不篡改持锁 mask。 |
| OS lock outcome | shared/exclusive sibling relation 分别对账 success/contended/error；q5–q8 的真实 lower/guard 要求保持不变。q9 是 pre-managed callback rejection，q10 是 raw/registry/managed lower 前的 ABI scalar direct rejection，q11 是受控 raw-state rejection；三者不得伪装成 OS lower。q12 则在真实 CreatedFirst DMS exclusive-release 点调用一次 `UnlockFileEx` 并故意不读取 BOOL，保留 `ExclusiveOutcomeUncertain` 与 quarantine；该未运行 source seam 只能形成 `controlled_fault_actual`，不得冒充 natural actual。精确门槛只见 [Lock tranches](node-plugin-vfs-lock-dynamic-tranches-authority.md)。`SQLITE_BUSY` 不得与 I/O failure、mutation 或 uncertainty 合并。 |
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

1. A2a/A2b1 的 SHM map/lock `StaticContract=43476/43476` 与 `8668/8668` 保持冻结；quotient manifests 对全部静态 members 完成精确、不交分区并得到 `Qmap/Qlock`，随后每个 frozen class 恰好一条正式 Windows record，分别原子形成 `WindowsDynamic=Qmap/Qmap` 与 `Qlock/Qlock`；
2. A2b2 的 117 项 Windows dynamic records 全部通过，集合与 static inventory 精确相等；
3. 宽范围 `sqlite_vfs_policy` 回归通过，既有 69 项成功路径不得退化；
4. source-contract 继续证明 fault script、fixture、pointer、observer 与 dynamic record只在测试边界可达；
5. 生产 `ComputePluginHandleBoundAuthorityOpenIntent::open()` 仍固定返回
   `COMPUTE_PLUGIN_HANDLE_BOUND_SQLITE_VFS_UNAVAILABLE`；
6. 从 A2 test-VFS 源码到 A1 producer、v15、PlanApply、work-admission enforcement、Sidecar、Runtime、Ready、route、outbox、Lease 或派发的生产调用边仍为零。

通过 A2 只允许进入独立的 production process owner/VFS/open 设计与实现阶段。它不把测试 VFS 提升为生产 VFS，也不让
`OpenedComputePluginLocalAuthority`、Planning snapshot 或 Ready 自动可构造。

## 9. 计数、报告与状态升级

- 每次执行报告必须分开列 `compiled`、targeted tests、Map/Lock `StaticContract`、`DynamicQuotientMemberCoverage`、`Qmap/Qlock`、`WindowsDynamic`、A2b2 StaticContract、A2b2 WindowsDynamic及wide regression；
- targeted tests 只能按真实测试数记账，不能映射为 `WindowsDynamic` case 数；
- 一个 dynamic unit 失败时，A2b2 保留 case key；Map/Lock 保留 class ID、representative/member-set commitment、最小脱敏差异和失败阶段；其余记录不伪造成整族已接受；
- 只有 map/lock 独立矩阵全部通过、A2b2 117/117、宽范围回归通过且 production isolation保持时，A2 才可从
  `implementation_not_dynamically_accepted` 升级；
- 任何证据缺失、环境不明、case key漂移、观察不完整或生产入口变化都维持失败关闭。

当前正式结论：Map `StaticContract=43476/43476`、Lock `StaticContract=8668/8668`。Lock q12 `LockNativeAcquireCreatedFirstExclusiveReleaseErrorV1` 为 source-only 88 exact singleton；q1–q12 未运行 inventory（members/groups）=`3756/3756 present, 4912/4384 missing, 8668/8140 total`，完整 initialization umbrella=`3432/2904`，q12 取 88/88 后 remaining=`3344/2816`。current source=`source_written/source_review_only/implementation_uncompiled/implementation_unrun`（uncompiled/unrun），`passed=0 failed=0 actual=not_run`、`controlled_fault_actual=source_only_unrun`；无 actual inventory/receipt/record、reviewed digest 或 frozen manifest，coverage=`0/43476 + 0/8668`、`Qmap/Qlock=unknown`、`WindowsDynamic=not_opened`。q9–q12 source changes 对应的 19-artifact global frozen/source-owner refresh 仍独立 deferred；A2 仍为 `implementation_not_dynamically_accepted`，production closed。q9–q12 细节只见[`Lock tranches`](node-plugin-vfs-lock-dynamic-tranches-authority.md)。

### 9.1 静态闭合前 fragment provenance（历史）

其中本批新增 route/callback normal-return 6-cell fragment 只属于上述 `source_written/source_review_only/validator_compiled/shared_a2b1_guard_passed` ledger：它没有新增 terminal、prefix edge、denominator key、静态计数或动态证据；唯一双成功分支仍是 adapter projection Pending。其余结论与计数不变。

紧邻的 adapter composed 7-cell child同样只属于`source_written/source_review_only/validator_compiled/shared_a2b1_guard_passed`：它保存exact 7-cell reviewed control/result inventory，其中两格是NotPresent与防御守卫通过后的Mapped；三条defensive guard另记为`Pending/Pending/NonNull-type-excluded` child-local review，故7格不穷尽双Ok continuation，并让completion拒绝前丢弃的success payload custody继续Pending。它没有升级shared parent disposition、关闭两个open frontier、改变owner-graph Pending=5/resolved cross-links=9，也没有形成完整exclusion ledger、successor trace、terminal universe、`StaticContract`或任何`WindowsDynamic` record。共享 A2b1 `4/4` 不得拆成该 child 的局部通过数，历史编译与5项局部测试也不得外推。
