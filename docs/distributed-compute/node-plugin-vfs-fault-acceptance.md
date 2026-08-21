---
title: 节点插件测试 VFS 故障动态验收
status: current
reviewed_at: 2026-08-22
owners: node, security
design_status: design_frozen
implementation_status: implementation_not_dynamically_accepted
verification_status: targeted_local_tests_partially_passed
---

# 节点插件测试 VFS 故障动态验收

## 1. 当前证据强度

本验收只消费 [`node-plugin-vfs-fault-authority.md`](node-plugin-vfs-fault-authority.md) 冻结的 A2 case inventory，
不创建第二套 VFS authority，也不授权生产入口。当前可记录的事实严格为：

- `design_frozen / source_written / implementation_not_dynamically_accepted`；本批新增 registration runner 为 `source_review_only / implementation_uncompiled / implementation_unrun`；
- `elon-pc-node` 完整测试目标在 2026-08-12 基线修复后可编译；
- 与可见性修复直接相关的 targeted fault matrix 已运行并通过 5 项；
- A2b2 的 117 项 source-exhaustive inventory 全部仍是 `StaticContract`，`WindowsDynamic=0/117`；
- 宽范围 `sqlite_vfs_policy` 回归仍有失败，不能把 5 项局部通过写成 A2 完成。

本批新增严格 test-only 的 RegistrationShutdown 8-selector actual/validator、进程隔离 runner 与线性 evidence envelope，并同步文档；没有编译或运行测试。历史编译和 5 项局部测试证据不得被重记为本批新证据，也不覆盖本批新增源码。

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

## 2. A2b2 Case 集合与完成条件

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

这里的 117 只统计 A2b2 barrier/unmap/close/registry inventory，不包含 A2a/A2b1 的 SHM map/lock。
map/lock 必须另建 source-exhaustive typed inventory，先冻结集合与数量，再逐项产生 Windows dynamic record；两套计数不得合并。

任何缺 case、重复 case、未知 case、static/dynamic key 不同构、只比较数量或把 partial bridge 记成完整 Case 都失败关闭。

## 3. 动态 evidence record

每个 Windows dynamic record 至少必须保存或在测试报告中逐字投影：

- commit、测试目标、Windows build、架构、文件系统/卷类型、bundled SQLite 版本和隔离 child identity；
- frozen case key、family、registration、route ordinal、runtime generation、SHM connection、role、callback、phase、
  occurrence、timing 与 unmap mode；
- 预期和实际 failure class、mutation/lock uncertainty、SQLite result code 或 `VoidNoResultCode`；
- fault selector 的 observed/triggered/pending 精确计数，以及 callback/action 的 attempted/succeeded 计数；
- before/after 的 Connection、route、logical-name、node、view、mapping、DMS、SHM file、main、lock 与 lease custody；
- physical domain tombstone、registry route terminal、registration phase、VFS table/name/context 与 root-deletable；
- child exit、parent cleanup 结果和最小脱敏诊断；不得记录 raw pointer、handle、Secret 或可复用 custody。

记录必须来自实际 callback/平台结果和受控 observer。静态 expected record、源码分支、Debug 输出、计数器默认值或测试手工
拼装的 post-state 不能冒充 actual。

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
| map failure before mutation | 输出指针清零；返回冻结 SQLite code；node/mapping/file custody 与 route phase逐项匹配 expected。 |
| map mutation known | 已完成 OS mutation 不得被描述为无副作用；本地 custody必须同步后终态化。 |
| map outcome uncertain | FileId/domain 永久 tombstone；同 domain sibling 不得重建 runtime 或继续 SHM。 |
| local lock contention | 合法 sibling 冲突只返回 `SQLITE_BUSY`，不触发脚本、不 poison、不篡改持锁 mask。 |
| lock release uncertainty | 不清本地 mask，不释放对应 custody；domain terminal 与后续 sibling 行为逐项匹配。 |
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

1. A2a/A2b1 的 SHM map/lock source-exhaustive inventory 已独立冻结，全部 Windows dynamic records通过，零缺失、额外或失败；
2. A2b2 的 117 项 Windows dynamic records 全部通过，集合与 static inventory 精确相等；
3. 宽范围 `sqlite_vfs_policy` 回归通过，既有 69 项成功路径不得退化；
4. source-contract 继续证明 fault script、fixture、pointer、observer 与 dynamic record只在测试边界可达；
5. 生产 `ComputePluginHandleBoundAuthorityOpenIntent::open()` 仍固定返回
   `COMPUTE_PLUGIN_HANDLE_BOUND_SQLITE_VFS_UNAVAILABLE`；
6. 从 A2 test-VFS 源码到 A1 producer、v15、PlanApply、work-admission enforcement、Sidecar、Runtime、Ready、route、outbox、Lease 或派发的生产调用边仍为零。

通过 A2 只允许进入独立的 production process owner/VFS/open 设计与实现阶段。它不把测试 VFS 提升为生产 VFS，也不让
`OpenedComputePluginLocalAuthority`、Planning snapshot 或 Ready 自动可构造。

## 9. 计数、报告与状态升级

- 每次执行报告必须分开列 `compiled`、targeted tests、map/lock inventory与dynamic、A2b2 StaticContract、A2b2 WindowsDynamic及wide regression；
- targeted tests 只能按真实测试数记账，不能映射为 `WindowsDynamic` case 数；
- 一个 dynamic case 失败时保留其 case key、最小脱敏差异和失败阶段，其余已通过 case 不改写为失败或未运行；
- 只有 map/lock 独立矩阵全部通过、A2b2 117/117、宽范围回归通过且 production isolation保持时，A2 才可从
  `implementation_not_dynamically_accepted` 升级；
- 任何证据缺失、环境不明、case key漂移、观察不完整或生产入口变化都维持失败关闭。

当前正式结论仍是：历史完整目标可编译且 5 项 targeted fault matrix 曾通过；本批 registration runner 未编译未运行，Registration `WindowsDynamic=0/8`、A2b2 `WindowsDynamic=0/117`，A2 未完成动态验收。
