---
title: 节点插件测试 VFS 故障合同权威
status: current
reviewed_at: 2026-08-11
owners: node, security
---

# 节点插件测试 VFS 故障合同权威

## 1. 权威范围与当前状态

本文是节点插件 A2 阶段的单一权威，只定义测试专用受管 SQLite VFS 的 SHM、联合关闭、同 namespace 多 Connection 和确定性故障注入合同。生产 SQLite 文件/目录、catalog 与 rollback 生命周期仍由 [`node-plugin-manifest-catalog-authority.md`](node-plugin-manifest-catalog-authority.md) 维护；Planning Snapshot 的依赖顺序仍由 [`node-plugin-planning-snapshot-authority.md`](node-plugin-planning-snapshot-authority.md) 维护；本机 schema 与 Store 仍由 [`node-plugin-local-authority.md`](node-plugin-local-authority.md) 维护。

A2 总合同已经冻结。源码现截止在 `implementation_uncompiled` 的 A2b1：A2a 的单 registration exact route 集合、共享 runtime 多 Connection fixture、callback-before-call 与四个 SHM teardown phase 仍保留；A2b1 又增加 exact registration/route 到 live WAL-main 的私有脚本安装桥，以及 `xShmMap` 初始化内部 phase、`xShmLock` acquire/release 的 before-call/after-success test-only seam 和 typed phase case。它未编译、未运行，也不是完整 A2；完整 `xShmUnmap` case、`xShmBarrier` 失败终态、联合 `xClose`、callback completion、route observe/retire、VFS unregister after-success 与逐 case 全量 custody 表仍属于 A2b2。本批不执行 SQLite Connection、Win32 故障注入、并发竞争、迁移或跨重启验证；既有 69 项 SQLite 专项与 3 项真实 Connection 成功路径证据不覆盖该增量。

生产 `ComputePluginHandleBoundAuthorityOpenIntent::open()` 必须继续固定返回 `COMPUTE_PLUGIN_HANDLE_BOUND_SQLITE_VFS_UNAVAILABLE`。A2 不提供生产 VFS 注册、process owner、live `sqlite3_file`、opened authority、A1 producer 或 v15 能力，也不改变 v14 blocked-only、Runtime stopped、`snapshot_ready=false` 与全部 side-effect false 的事实。

## 2. 已有基线与真实缺口

现有 managed-fs 已有以下低层形状：

- 一个 pinned namespace 只能消费成一个进程内 WAL/SHM runtime；runtime 按 exact main FileId 绑定多个本地 SHM connection ID，并拒绝第二个同目录 lock domain；
- SHM 内核已有固定 region budget、稳定映射、8 槽本地锁状态、DMS、OS byte-range lock、barrier、unmap/delete gate、typed failure phase 与永久 domain tombstone；
- registry/file-custody 已把 main、Journal/WAL sidecar、SHM lease、route 与 callback lease 不可拆持有；物理关闭失败会保留 exact custody，不能把 `Drop` 当作成功回执；
- 测试受管 VFS 已让单个真实 SQLite Connection 经 main、Journal/WAL、SHM 和 `xClose` 进入 exact route，并在正常关闭后退休 route、注销 VFS 和删除测试根。

A2a 源码已把一个 registration 扩为 exact logical-name route 集合，每条 Connection 独立 route/authorizer/custody 并共享一个 WAL/SHM runtime；同时增加 callback-before-call 与 SHM teardown 的 test-only one-shot fault 形状。A2b1 只把 exact route 上的 plan 经刚提升的 live WAL-main 绑定到 runtime generation + SHM connection ID，并把 map 初始化与 lock 平台动作放进相邻 test-only hook；typed records 固定两 Connection、两 route、六 logical-name，并声明 phase occurrence、map mode/region、lock action/range/pre-mask 与 DMS custody 的静态预期。它仍只是源码合同：没有完整 close/retirement 矩阵，也未动态证明非末连接关闭、domain terminal 后 sibling 行为或真实 Win32 custody 结果。

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

脚本只控制测试平台 seam，不能改变生产分支、放宽请求校验、跳过 callback lease、直接改 coordinator state，或为测试暴露可复用的 main/SHM/file custody 构造器。

## 5. A2 静态矩阵

完整 A2 源码合同至少逐项表达下表。当前 A2b1 覆盖 route 解析、多 Connection 成功/竞争形状、callback-before-call、四个 teardown phase、map 初始化与 lock acquire/release 的注入 seam，并为后一组写入固定拓扑的 typed phase records；它仍未覆盖表中完整 native-error/cleanup/close/registration case inventory。以后即使全部“表达”，也只表示存在静态 case/fixture 形状，不表示已经执行：

| 路径 | 必须覆盖的阶段 | 静态不变量 |
|---|---|---|
| `xShmMap` observe/extend | exact sibling open、DMS exclusive acquire/truncate/release、DMS shared acquire、file size/grow、mapping create、view map | 输出指针失败时清零；before-mutation 可分类失败；已知 mutation 不被改写为空操作；结果不确定永久 poison 并保留 node/mapping/file custody |
| `xShmLock` shared/exclusive | request validation、local sibling contention、OS lock acquire/release | 合法 contention 只返回 `SQLITE_BUSY` 且不 poison；非法 range/action 不调用平台；unlock 结果不确定不能清本地 mask或释放 custody |
| `xShmBarrier` | callback admission、barrier、callback completion | 无 SQLite result-code 返回通道；失败必须清 raw state一次并保留 terminal custody，不能伪造为 `SQLITE_IOERR` 或正常完成；当前属于 A2b2 |
| `xShmUnmap` 非末/末连接 | held-lock gate、connection detach、view unmap、mapping close、DMS shared release、SHM file close、delete authorization/delete | 持锁 unmap 拒绝；非末连接只 detach 自己；末连接才 teardown；delete 只在 exact runtime/main identity 与 Main-EXCLUSIVE gate 下成立 |
| WAL-main 联合 `xClose` | SHM unmap、main unlock、main file close、close callback completion、route observation/retirement | 顺序固定先 SHM 后 main；任一步失败都不继续伪造后继回执；raw state只消费一次；exact leases/custody 保留或隔离 |
| registration shutdown | outstanding callback、live route、quarantined custody、VFS unregister | 任一未闭合对象阻止正常释放；注销失败保留 table/name/context；测试根只有完整成功证明后才可删除 |

每个 case 必须静态声明预期 phase、class、SQLite code、route phase、domain terminal bit、剩余 Connection 数、是否保留 node/mapping/file/main/SHM lease、是否允许后续 callback，以及 raw state/route/custody 的精确一次性计数。只断言“返回非 OK”不足以验收。

## 6. 同 namespace 多 Connection

多 Connection 合同固定为：

- 首个与后续 main bind 都必须对账相同物理身份，并获得不同 SHM connection ID；每个 main 的首次 `xShmMap` 只提升自己的 route-local main custody；
- 同一 region 在 node generation 不变时映射稳定；不同 Connection 可以观察同一 SHM 内容，但任何 raw pointer 只能在对应 WAL-main custody 活着时借用；
- 同 Connection 的 shared/exclusive mask 与 sibling mask 分开记录；同槽冲突返回 `SQLITE_BUSY`，释放只能消费自己确实持有的 exact range；
- 非末 Connection 的 `xShmUnmap(delete=true)` 也只分离该 Connection，不删除、关闭或重建共享 SHM；另一 Connection 的映射、锁和 route 保持可用；
- 最后一条 Connection 只有在无本地 SHM 锁时才可 teardown。SHM delete 还必须携带同一 WAL-main、runtime generation、main identity 与 Main-EXCLUSIVE 事实；
- 任一结果不确定的 map/lock/unmap/close 必须把整个 FileId/domain 置为 terminal。其他 route 不能因为自身标量仍匹配而继续使用或另建 runtime；
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

A2b1 本批可接受的结论仅是：test-only API 不可从生产构造；exact registration/route 只能经 live WAL-main 私有 delegate 取得低层 target；map 初始化、lock acquire/release 与既有 teardown 子集具备 one-shot/fenced 形状；after-success 只在平台成功和本地 custody/mask 同步后终态化；typed records 是固定拓扑的静态 phase evidence；生产 `open()`、A1 producer 与协议均保持不可达。不得宣称完整 A2 matrix/source 已形成。验收仅允许文档检查、rustfmt 与静态 diff/搜索。

进入 A1 依赖顺序的生产 process owner/VFS 注册/open 阶段之前，必须先由 A2b2 补齐 `xShmBarrier`、完整 unmap、联合 close、callback/route/registration after-success 与全量逐 case custody 断言，再另批实际执行 Windows SHM map/lock/unmap、联合关闭平台故障矩阵和同 namespace 多 Connection 竞争，逐项证明 SQLite code、custody、route、domain tombstone 与资源释放。静态 A2b2 和动态证据任一缺失，都不得把 A2 标记完成或推进生产入口。
