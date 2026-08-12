---
title: 节点插件测试 VFS 故障合同权威
status: current
reviewed_at: 2026-08-11
owners: node, security
---

# 节点插件测试 VFS 故障合同权威

## 1. 权威范围与当前状态

本文是节点插件 A2 阶段的单一权威，只定义测试专用受管 SQLite VFS 的 SHM、联合关闭、同 namespace 多 Connection 和确定性故障注入合同。生产 SQLite 文件/目录、catalog 与 rollback 生命周期仍由 [`node-plugin-manifest-catalog-authority.md`](node-plugin-manifest-catalog-authority.md) 维护；Planning Snapshot 的依赖顺序仍由 [`node-plugin-planning-snapshot-authority.md`](node-plugin-planning-snapshot-authority.md) 维护；本机 schema 与 Store 仍由 [`node-plugin-local-authority.md`](node-plugin-local-authority.md) 维护。

A2 总合同已经冻结。源码现推进到 `implementation_uncompiled`、`implementation_unrun` 的 A2b2 静态闭合：A2a/A2b1 的 registration exact route、共享 runtime 多 Connection、exact route→live WAL-main 私有脚本桥与 map/lock 内部 phase 仍保留；本批又为 `xShmBarrier` 无返回码失败、非末/末 `xShmUnmap`、WAL-main 联合 `xClose`、callback completion、connection observation、registry route retirement、三 logical-name removal 与 VFS unregister 建立 exact fault/custody 形状和 typed case inventory。A2c 严格 `cfg(all(test, windows))` 的 runner 源码现含四条 foundation 路径：具名 test VFS、双 `rusqlite::Connection` 与 WAL SQL 触发 exact route 的 `Main/ShmMap/1/BeforeCall` 和 `Main/ShmLock/1/BeforeCall`；另以独立子进程触发 zero-route registration 的 VFS unregister before/after，源码区分仍 registered 与已真实 unregister 后 retained parts，子进程退出后父进程才清理测试根。本批再新增 actual→frozen expected 源码桥，以 retained-parts redacted witness 和 VFS lookup/routes/lifecycle/root 事实对可直接观察的 registration 字段逐项对账，但只映射 `RegistrationShutdown/VfsUnregister` 的 `BeforeCall`、`AfterSuccessKnown` 两个冻结 key。上述增量均未编译、未测试、未运行（`passed=0`），117 项动态进度仍为 0；也没有实现 native failure adapter，或执行 Win32 故障注入、并发竞争、迁移、跨重启验证。既有 69 项 SQLite 专项、3 项真实 Connection 成功路径证据和这些未执行 runner/bridge 源码都不能算作 `WindowsDynamic`，更不能替代逐 case Windows 动态证据。

生产 `ComputePluginHandleBoundAuthorityOpenIntent::open()` 必须继续固定返回 `COMPUTE_PLUGIN_HANDLE_BOUND_SQLITE_VFS_UNAVAILABLE`。A2 不提供生产 VFS 注册、process owner、live `sqlite3_file`、opened authority、A1 producer 或 v15 能力，也不改变 v14 blocked-only、Runtime stopped、`snapshot_ready=false` 与全部 side-effect false 的事实。

## 2. 已有基线与真实缺口

现有 managed-fs 已有以下低层形状：

- 一个 pinned namespace 只能消费成一个进程内 WAL/SHM runtime；runtime 按 exact main FileId 绑定多个本地 SHM connection ID，并拒绝第二个同目录 lock domain；
- SHM 内核已有固定 region budget、稳定映射、8 槽本地锁状态、DMS、OS byte-range lock、barrier、unmap/delete gate、typed failure phase 与永久 domain tombstone；
- registry/file-custody 已把 main、Journal/WAL sidecar、SHM lease、route 与 callback lease 不可拆持有；物理关闭失败会保留 exact custody，不能把 `Drop` 当作成功回执；
- 测试受管 VFS 已让单个真实 SQLite Connection 经 main、Journal/WAL、SHM 和 `xClose` 进入 exact route，并在正常关闭后退休 route、注销 VFS 和删除测试根。

A2a/A2b1 源码已把一个 registration 扩为 exact logical-name route 集合，每条 Connection 独立 route/authorizer/custody 并共享一个 WAL/SHM runtime；exact route 上的 plan 经刚提升的 live WAL-main 绑定到 runtime generation + SHM connection ID，map 初始化与 lock 平台动作也进入相邻 test-only hook。A2b2 静态源码继续把 barrier、unmap、联合 close、registry lifecycle 与 registration shutdown 分成独立 typed leaves；它仍只是未执行源码合同，尚未动态证明非末连接分离、末连接 teardown、domain terminal 后 sibling 行为、物理关闭一次性、注销结果或真实 Win32 custody。

A2c runner foundation 不产生 `WindowsDynamic` evidence。四条执行源码路径只接通 route-exact callback-before 与 registration unregister 的进程隔离形状；新增 bridge 仅把后两条的 actual registration facts 中可直接观察的字段对账到上述两个 frozen expected record，不改写 expected inventory。redacted witness 只表达 retained table/name/context，配合 lookup、zero routes、terminal lifecycle 与 child 内 root 保留事实；root 的存在性不等于操作系统删除失败证明，也不单独验收 frozen root-deletable custody。它不包含 raw handle、pointer 或 Connection ID，也不能外推为 SHM/file 的平台 after-success custody。`ShmLock` observation 仍不含具体 shared/exclusive 动作；现阶段不得从这些源码推断 native failure、barrier 无返回码、unmap、joint close、route lifecycle 或其余 115 key，117 项动态覆盖仍为 0。

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

完整 A2 源码合同至少逐项表达下表。当前 A2b2 已为表中 barrier/unmap/close/registration 建立 typed static inventory；即使这些记录在源码中全部“表达”，也只表示静态 case/fixture 形状存在，不表示它们已经编译或执行：

| 路径 | 必须覆盖的阶段 | 静态不变量 |
|---|---|---|
| `xShmMap` observe/extend | exact sibling open、DMS exclusive acquire/truncate/release、DMS shared acquire、file size/grow、mapping create、view map | 输出指针失败时清零；before-mutation 可分类失败；已知 mutation 不被改写为空操作；结果不确定永久 poison 并保留 node/mapping/file custody |
| `xShmLock` shared/exclusive | request validation、local sibling contention、OS lock acquire/release | 合法 contention 只返回 `SQLITE_BUSY` 且不 poison；非法 range/action 不调用平台；unlock 结果不确定不能清本地 mask或释放 custody |
| `xShmBarrier` | callback admission、barrier、callback completion | 无 SQLite result-code 返回通道；失败必须清 raw state一次并保留 terminal custody，不能伪造为 `SQLITE_IOERR` 或正常完成；当前属于 A2b2 |
| `xShmUnmap` 非末/末连接 | held-lock gate、connection detach、view unmap、mapping close、DMS shared release、SHM file close、delete authorization/delete | 持锁 unmap 拒绝；非末连接只 detach 自己；末连接才 teardown；delete 只在 exact runtime/main identity 与 Main-EXCLUSIVE gate 下成立 |
| WAL-main 联合 `xClose` | SHM unmap、main unlock、main file close、close callback completion、route observation/retirement | 顺序固定先 SHM 后 main；任一步失败都不继续伪造后继回执；raw state只消费一次；exact leases/custody 保留或隔离 |
| registration shutdown | outstanding callback、live route、quarantined custody、VFS unregister | 任一未闭合对象阻止正常释放；注销失败保留 table/name/context；测试根只有完整成功证明后才可删除 |

每个 case 必须静态声明预期 phase、class、SQLite code、route phase、domain terminal bit、剩余 Connection 数、是否保留 node/mapping/file/main/SHM lease、是否允许后续 callback，以及 raw state/route/custody 的精确一次性计数。只断言“返回非 OK”不足以验收。

### 5.1 A2b2 typed case schema 与完整 inventory

A2b2 不复用 A2b1 含混的单一 `remaining_connections`。每条 record 必须分开保存 `sqlite_connections`、`shm_connections`、`registry_routes` 与 `logical_names`，并包含：

- target scope 与 exact identity：route-scoped case 保存 registration ID、route ordinal、runtime generation、SHM connection ID、Main role、callback kind 与 phase occurrence；registration shutdown 只保存 registration identity，不虚构 route/runtime/SHM connection；另存 unmap mode、非末/末 topology 与 fault timing；
- SQLite channel（`xShmBarrier=VoidNoResultCode`，unmap failure=`SQLITE_IOERR`，close/close-lifecycle failure=`SQLITE_IOERR_CLOSE`，registration shutdown 与 logical-name removal 无 SQLite channel）、failure class、mutation/lock uncertainty、physical domain terminal、独立 registry route terminal、logical route 与 registration phase；
- node、view、mapping、DMS、SHM file、main file/lock owner、main/SHM/callback lease、registry entry、三 logical-name、VFS table/name/context 与 root-deletable custody；
- raw-state take/abandon/`pMethods` clear、callback begin/complete、selected action、SHM detach、main unlock/file close、registry close、connection observe、registry route remove、logical-name remove、VFS unregister 与 custody retain 的 attempt/success 精确计数；logical-name 另保存恰好三项的 removal count，fault selector 另保存 observe/trigger/pending 三项 one-shot 计数，`physical_retry` 必须恒为零；
- `StaticContract` 与 `WindowsDynamic` 是互斥 evidence kind；本批所有 record 只能是前者。

静态 inventory 必须按集合相等验收，而不是只检查某 phase 至少出现一次：barrier 覆盖 callback admission、fence before/after、completion 与 success；非末 unmap 覆盖输入校验、shared/exclusive held-lock、`delete=true` 仍只 detach、detach before/after 与 completion；末连接覆盖 ViewUnmap、MappingClose、DMS shared release、SHM file close 的 before/native/after、Keep/Delete、delete authority、exact sibling delete、detach 与 completion；联合 close 必须把每条 managed-fs Keep/final 物理 unmap failure 一一投影为 `ShmUnmapLift` 并断言 main close 未开始，不得虚构 registry SHM callback-completion 投影，再覆盖 main unlock、main handle close、registry WAL-main close、唯一 close callback completion、connection observation、registry route removal 与 logical route removal；registration 覆盖 outstanding callback、live route、quarantined custody、route-index observation、unregister before/native/after 与完整成功。成功卸载后只有 registry route 和 logical-name 均为零、全部物理/lease custody 与 table/name/context 已释放时，测试根才可删除。

冻结源码由 `a2b2_cases.rs` 与十个叶模块组成，每叶继续受 `<500` 行硬预算约束；source-exhaustive case 总数固定为 117，其中 Barrier 8、Unmap 49、JointClose 36、Registry lifecycle 16、Registration shutdown 8。Barrier 单列 inner registry callback 前的 generic callback-wrapper before fault，联合 close 单列 begin-close 成功后的 Close callback admission rejection；Barrier/close callback completion 均覆盖 before/native/after，non-final Keep、final Keep/Delete 与 joint-close lift 均单列 ConnectionDetach after-success-uncertain；registry retirement 分开保存 owner-retire native failure 与 retire 成功后 receipt 发布失败，logical-name removal 分开保存 retirement receipt claim 失败与 claim 后 index/custody native failure。入口已通过 `managed_vfs.rs` 的 `cfg(test)` 模块声明接入，但本批没有编译或运行这份静态 inventory。

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
- 禁止在本批运行编译、测试、SQLite/Win32 fixture、迁移或真实节点；以后执行时必须另记命令、平台、case 数与结果。

## 9. 静态验收与后续门槛

A2b2 本批可接受的结论仅是：test-only API 不可从生产构造；exact registration/route 只能经 live WAL-main 私有 delegate 取得低层 target；barrier/unmap/joint close/registry/registration 具备 one-shot/fenced 静态形状；after-success 只在平台或 registry mutation 成功并同步 custody 后终态化；typed records 是固定拓扑的静态 source evidence；生产 `open()`、A1 producer 与协议均保持不可达。验收仅允许文档检查、rustfmt 与静态 diff/搜索，不得宣称 A2 已完成动态验收。

进入 A1 依赖顺序的生产 process owner/VFS 注册/open 阶段之前，仍必须另批实际执行 Windows SHM map/lock/unmap、联合关闭平台故障矩阵和同 namespace 多 Connection 竞争，并把每条动态观察与静态 case key 一一对应，逐项证明 SQLite code或无返回码通道、custody、route、domain tombstone 与资源释放。当前 A2b2 未编译、未运行；静态源码和 Windows 动态证据任一缺失，都不得把 A2 标记完成或推进生产入口。
