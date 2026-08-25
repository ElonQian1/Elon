---
title: 节点插件测试 VFS 故障动态验收
status: current
reviewed_at: 2026-08-25
owners: node, security
design_status: design_frozen
implementation_status: implementation_not_dynamically_accepted
verification_status: targeted_local_tests_partially_passed
---

# 节点插件测试 VFS 故障动态验收

## 1. 当前证据强度

本验收只消费 [`node-plugin-vfs-fault-authority.md`](node-plugin-vfs-fault-authority.md) 冻结的 A2 case inventory，
不创建第二套 VFS authority，也不授权生产入口。当前可记录的事实严格为：

- `design_frozen / source_written / implementation_not_dynamically_accepted`；registration runner、Map ABI/raw reviewed-successor prefix、denominator-facing ABI fragment与source-neutral raw fragment均为 `source_review_only / implementation_uncompiled / implementation_unrun`；
- `elon-pc-node` 完整测试目标在 2026-08-12 基线修复后可编译；
- 与可见性修复直接相关的 targeted fault matrix 已运行并通过 5 项；
- A2a/A2b1 map/lock 的 commit-bound `SourceScope/SourceOwnerGraph v1` 已 `design_frozen/source_written/source_review_only/validator_uncompiled/unrun`；Map source-terminal template review ledger v1 也已 `source_written/source_review_only/validator_uncompiled/unrun`。既有六个 budget/FileSize/FileGrow resolved owner/stage boundary与专属`AbiValidation`关联共七个cross-link；本批新增source-neutral raw fragment，精确区分8个admission/prestate（7 rejection + 1 expected-type continuation）、typed operation后的2个outcome与8个abandon outcome，再将共享raw owner node拆为Map/Lock sibling。只有两个Map raw gate/abandon node由专属Map site闭合，使resolved cross-link从7增至9、Map-reachable graph pending从7降至5；两个Lock sibling仍Pending。Map raw projection fragment只包含8条fallback continuation与1条typed-frontier continuation，两个open frontier继续开放；denominator-facing ABI fragment仍严格是15个pre-raw terminal cell与1个`AbiRawDispatch` continuation，raw cell不加入这15个ABI terminal。ledger仍保留Pending/open boundaries与六个prestate-pending success candidates，没有source-exhaustive terminal set或完整successor trace；candidate typed schema与显式不完整的branch-atom scaffold也仅为source-written review输入，完整terminal universe、quotient、exact key set、`SourceBranch`、`Expected`、`CaseKey`、exclusion ledger与denominator仍为`source_review_pending/implementation_uncompiled/implementation_unrun`，不得记`StaticContract`或开放`WindowsDynamic`；production ABI/managed-fs/route/open保持未修改；
- A2b2 的 117 项 source-exhaustive inventory 全部仍是 `StaticContract`，`WindowsDynamic=0/117`；
- 宽范围 `sqlite_vfs_policy` 回归仍有失败，不能把 5 项局部通过写成 A2 完成。

owner 图只验证 baseline commit literal 形状、从 reviewed owner bytes 重算的 Git blob OID/规范化 SHA-256、symbol presence、ABI roots、逐 operation scope 可达性，以及 wrapper/promotion/callback/cold-prefix/loop/cleanup/quarantine/result projection 的有序结构；它不读取 `.git`，不能自动证明当前 checkout HEAD 等于 baseline，也不是 exact terminal inventory。Map review ledger 验证自己声明的 step ID materialization、owner/symbol/occurrence anchor、共享分支 call context、candidate disposition、pointer-flow 分层、cause/returned/stored/route 四轴、六个 success projection witness、非空 Pending/open boundaries，以及 Map-reachable pending exact set与九个 resolved owner/stage 关联的 exact owner/symbol-or-site witness link；ABI/raw prefix另验证 exact case/edge/endpoint set、terminal无后继、open frontier与 raw slot保留轴。它们都不验证完整 source coverage、端到端 trace、exclusion proof 或 denominator。candidate typed schema 与不完整 branch-atom scaffold、以及严格 test-only 的 RegistrationShutdown 8-selector actual/validator、进程隔离 runner与线性 evidence envelope保持既有边界。以上新增与改动源码都没有在本批编译或运行；历史编译和 5 项局部测试证据不得被重记为本批新证据。

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

## 2. Case 集合与完成条件

### 2.1 A2a/A2b1 map/lock

map/lock denominator 的唯一 quotient、scope、`CaseKey`、`SourceBranch`、`Expected` 与 legacy 28 非 denominator 边界由
[`authority §5.1`](node-plugin-vfs-fault-authority.md) 维护。本验收以后只消费 source/red-team review clean 的 frozen typed set，
不从 runner、历史 targeted 名称、intermediate candidate count 或实际可运行子集反推 denominator。

`SourceOwnerGraph v1` 是进入 terminal ledger 之前的结构门：它必须逐字匹配 reviewed owner blob/SHA/symbol，并保持两条 ABI root、逐 operation scope endpoint 可达、valid non-null Map output slot 的 fail-null、null slot no-write、独立 fallback/result-code、outer wrapper-before-route、promotion callback-before-operation callback、`ScopePending` cold Lock prior-Map witness、Unlock no-init、四段 budget 调用顺序、FileSize 只读/双操作、FileGrow mutation+poison/Extend-only、region loop、cause-separated cleanup rewrite、unsafe retention-before-completion 与 operation/completion error precedence。Map ledger 的 denominator-facing ABI fragment必须保持15个pre-raw terminal cell + 1个`AbiRawDispatch` continuation，并以两个rejection witness exact-link已扩展的`AbiMapValidation`；source-neutral raw fragment必须另行exact-set校验8个admission/prestate、2个post-operation outcome与8个abandon outcome，raw cell不得并入ABI的15个terminal。Map raw projection必须恰为8条fallback continuation + 1条typed Map operation frontier continuation；两个Map raw node已resolved，两个Lock sibling仍Pending，且typed-operation与raw-fallback custody/route两个open frontier都未关闭。正常返回后的`RawStateAccepted`与caught unwind后的`RawStateCaughtPanic`仍须标为`BeyondOpenFrontier(TypedMapOperation)`且不得出现在prefix DAG；caught-panic→abandon→fallback只能保存为带frontier标签的source cause，不能冒充reviewed successor。只有prefix-materialized raw rejection可进入“unavailable-null exit已知但custody/route尚未闭合”的frontier。cold witness只能引用ensure-node之前的Map返回，完整early-return universe与node-absent prestate partition仍Pending；FileSize site的`ObserveNotPresent`和ABI/raw之后的fault-finish后继仍只有ledger/stage关联，不构成完整branch/projection successor trace。present output slot必须callback-owned/non-alias/aligned/writable/live；非null file必须live/aligned/initialized/serialized，exact methods+state必须是本模块live envelope。违背这些premise的指针是UB，不进入有限case或terminal count。通过owner图或Map review ledger仍只允许写`source_written/source_review_only`；不能据此产生完整`SourceBranch`、`Expected`、`CaseKey`、denominator、`StaticContract`或`WindowsDynamic`。

当前 denominator 计数尚未冻结：

| Case family | Source review | StaticContract | WindowsDynamic | 完成条件 |
|---|---|---|---|---|
| Map | owner graph + template ledger + ABI/raw reviewed-successor prefix + ABI/raw denominator-facing fragments written; 5 graph Pending 与 open frontiers仍非空；full exact-set review pending | not counted | not opened | 每个 frozen Map `CaseKey` 各有且只有一个通过的 Windows dynamic record，且 actual 与 expected 逐字段相等。 |
| Lock | owner graph written; exact-set review pending | not counted | not opened | 每个 frozen Lock `CaseKey` 各有且只有一个通过的 Windows dynamic record，且 actual 与 expected 逐字段相等。 |
| **A2a/A2b1 map/lock 总计** | **exact-set review pending** | **not counted** | **not opened** | static/dynamic 两次集合比较均与最终 frozen key set 精确相等。 |

只有 authority frozen typed key、source-branch projection、expected record 与 exclusion ledger 在源码中存在并通过 exact-set review，
才可写 `StaticContract=N/N`；即使如此，本批未编译、未运行，也不得写成 `passed=N`、`verified` 或动态接受。现有 18 map + 10 lock
仅为 `static_subset/non_denominator`，不计入未来 N 的完成数，也不允许与新 inventory 相加成 `N+28`。

static 集合验收必须同时满足：authority frozen `BTreeSet<CaseKey>` 与源码集合 exact equality；每个 in-scope source terminal
leaf 恰好投影一个 key；每个 key 有非空 `SourceBranch` 集合和唯一 `Expected`；missing、extra、duplicate、unknown、无理由排除、
同 key 不同 expected、只比较长度或手写 actual 全部失败关闭。dynamic 集合以后必须再与同一 key set exact equality，不能修改
static expected 来迁就观察结果。

### 2.2 A2b2

动态验收必须与冻结 inventory 做集合相等比较，不能只挑选可运行子集：

| Case family | 冻结数量 | 当前 WindowsDynamic | 完成条件 |
|---|---:|---:|---|
| Barrier | 8 | 0 | callback admission、before/native/after fence、completion 与 success 全部逐项运行。 |
| Unmap | 49 | 0 | non-final/final、Keep/Delete、held-lock、detach、view/mapping/DMS/SHM file 与 delete authority 全部逐项运行。 |
| JointClose | 36 | 0 | SHM lift、main unlock/file close、callback、connection、route/logical-name retirement 全部逐项运行。 |
| Registry lifecycle | 16 | 0 | route observation/removal、logical-name claim/index/custody、quarantine 与成功清空全部逐项运行。 |
| Registration shutdown | 8 | 0 | callback、live route、quarantine、unregister before/injected-pre-native/after 与完整成功全部逐项运行。 |
| **A2b2 总计** | **117** | **0** | 117 个唯一 static case key 各有且只有一个通过的 Windows dynamic record。 |

RegistrationShutdown 的 8-case runner 源码现已铺设，但未编译、未运行且没有产出 record；该 family 仍为 `0/8`。它不能改变表中任何计数。

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

未来编译并执行这组动态测试前，执行者必须在编译时设置 `ELON_NODE_AGENT_GIT_SHA=<被测 checkout 的 exact commit>`；只接受 40 或 64 位小写 hex。源码只校验该值存在且格式正确，执行账本还必须独立重证其逐字等于被测 commit；缺失、格式错误或不相等均不得形成 record。

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

至少以同一 namespace、同一 main FileId、同一 runtime generation 和两个不同 route/SHM connection 建立竞争拓扑，并证明：

- non-final `xShmUnmap(delete=true|false)` 只 detach 当前 Connection；sibling 的 mapping、lock、route 与 callback 仍可用；
- held-lock unmap 在平台 teardown 前拒绝，持锁与 custody 不发生虚假释放；
- final Keep/Delete 按 ViewUnmap → MappingClose → DMS shared release → SHM file close → optional delete 的固定次序推进；
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

当前正式结论仍是：历史完整目标可编译且5项targeted fault matrix曾通过；本批map/lock owner图与含ABI/raw successor prefix、source-neutral raw fragment及Map raw projection的template ledger为`source_written/source_review_only/validator_uncompiled/unrun`，Map Pending=5/resolved cross-links=9且Lock raw sibling仍Pending；独立denominator-facing ABI fragment仍为`source_written/source_review_only/implementation_uncompiled/implementation_unrun`，完整terminal inventory仍为`source_review_pending/implementation_uncompiled/implementation_unrun`，尚不能记`StaticContract`或开放`WindowsDynamic`；registration runner为`source_review_only/implementation_uncompiled/implementation_unrun`。Registration `WindowsDynamic=0/8`、A2b2 `WindowsDynamic=0/117`，A2仍为`implementation_not_dynamically_accepted`。生产open、A1、v15、Runtime与Ready均未改变。
