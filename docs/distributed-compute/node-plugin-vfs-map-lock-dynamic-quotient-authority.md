---
title: 节点插件 VFS Map/Lock 动态商集权威 V1
status: current
reviewed_at: 2026-08-31
owners: node, security
design_status: design_frozen
implementation_status: typed_projector_candidate_prior_compiled_pre_manifest_program_inventory_source_written_uncompiled_unrun
verification_status: prior_targeted_unit_36_passed_current_source_inventory_not_run_not_frozen_manifests_not_frozen
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
candidate 入口在前序基线已实现并通过编译和定向单元测试；本批又写入 sealed runner admission，以及仅覆盖
Map `RegionCountBudget/Completed` 的 source-only executable program、私有 actual receipt 和 parent/child/cleanup
合同。current source 进一步把“语义 program 分组”“源码 program 存在”“actual execution verified”拆成三层，
新增完整 Map 根的 pre-manifest execution-program inventory 源码；但该路径尚未编译、未运行，也没有冻结
inventory digest。默认 Map/Lock producers、完整 candidate 与 manifest 路径未因此接通；
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
内不同 Missing gap 都失败关闭。唯一新增的 capability 形状是 Map `RegionCountBudget` 且 completion 为
`Completed`；它只允许进入私有 receipt 复验路径，不把 producer 声明本身变成许可。此闭合只证明 producer
元数据没有被错配；它不证明正式 runner 或 observation 已存在。

producer coherence 之后还必须由 projector 内部从同一个已验证 semantic key 机械编译 root-specific runner
plan。plan commitment 域隔离地绑定 projector schema、root、capability-normalized descriptor digest、有序
required stages 与 exact planned-missing gap；producer 不能提交、自签或替换该 plan。裸
`RunnerCapabilityV1::Supported` 只是声明，不是 permit；普通 `resolve_v1` 仍拒绝裸声明。仅
`project_validated_dynamic_terminal_with_map_execution_v1` 可消费私有 `MapRunnerExecutionReceiptV1`；当它与
exact member、normalized descriptor、内部 plan、program implementation 和 execution commitment 全部精确
绑定时，窄 `RegionCountBudget/Completed` 路径才可形成
`RunnerAdmissionDecisionV1::Supported { implementation_sha256, execution_sha256 }`。缺 receipt、跨 root/plan、
semantic drift 或任一 commitment 替换都失败关闭。

该窄 program 的 source design 固定 parent 拥有隔离根与最终删除责任，child 执行受管 VFS Map callback 并
形成实际结果，parent 只在 child terminal/exit 与 cleanup receipt 都绑定后消费私有 actual receipt。它仍是
未编译、未运行的程序合同，不是 Windows record。默认 Map producers 仍全量签发
`Missing(QuotientRunnerNotIntegrated)`，Lock producers 与准入保持
`Missing(LockObservationIncomplete)`；因此当前完整 candidate 没有 class 被放行。

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
找到了实现，仍须后续正式 receipt；当前 matcher 只认既有 Map `RegionCountBudget/Completed` 的 Observe/Extend
形状。其他 Map program 继续是 `PlannedMissing(QuotientRunnerNotIntegrated)`，Lock 不在本批 inventory 入口
范围内。matcher 只把明确的 `UnsupportedProgram` 归为 planned missing；plan/binding 等内部错误必须携带 exact
member 使整次 inventory 失败，禁止 fail-open-as-missing。

inventory context 必须绑定 static baseline/source scope/ledger/manifest、included/excluded/source-universe、
exact included member-pair set、projector schema/source scope、frozen descriptor binding 与 inventory source scope。
排序后的 `member -> program_id` reverse index、每个 program 的 plan/status/member-set、group/member 计数和
总 inventory body 分别使用 domain-separated canonical digest。builder 必须从 program group union 独立重建
reverse index 并与逐叶 membership 相等；任一 missing member、extra、duplicate、collision、binding drift 或
空 group 都使 inventory 原子失败。

该 inventory 不是 quotient manifest，也不选择正式 representative，不产生 `Qmap`、
`DynamicQuotientMemberCoverage`、Windows record 或任何 runtime permit。只有以后完整 Map program inventory
全部达到 source-present、独立 review 冻结其 digest，并由 quotient manifest context 反向绑定后，才可进入
manifest 冻结；actual Windows execution 仍必须在 manifest 后按 frozen class representative 发生。

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
`producer_coherence/map.rs`、`producer_coherence/map_axes.rs`、`producer_coherence/lock.rs`、
`producer_coherence/lock_axes.rs`，以及 `descriptor_binding.rs`、`membership_commitment.rs`、
`runner_admission.rs`、`runner_admission/{canonical,map,map_program,lock}.rs`；其中任一
接受关系或 commitment 编码变化都必须触发 projector provenance drift 和全量重审。

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
catalog 暴露记录；第二遍重复同一 frozen gate，并把已经逐叶验证的 full record + typed descriptor 只流入
私有、可丢弃的内存 catalog。两遍 binding 都承诺 exact
`(case_key_sha256, full_record_sha256)` member-pair set 且必须完全相等。随后 catalog/manifest guard 从实际
class union 重算 member-pair set，重算 class key，检查 class/member 唯一性、排序、representative、反向索引
和全部 canonical digests；只有全部成功才返回内存 bundle。当前没有候选文件 writer，因此失败不会留下
可被误认作 frozen 的部分 manifest。checked-in frozen manifest 必须经独立 review 后另批提交，且任何
static 或 projector 漂移都要求全量重生成与重审。

前序验证事实严格限于：当时实现已编译，定向单元测试已通过；Lock 全量 `8,668` 成员 candidate gate 已按预期
完成 exact frozen ingress/typed projection，并因 `LockObservationIncomplete` 原子失败关闭；Map 全量
`43,476` 成员 candidate gate 也已完成 exact frozen ingress/typed projection，并因
`QuotientRunnerNotIntegrated` 原子失败关闭。Map 的真实阻塞是 quotient runner 尚未集成，Lock 的真实
阻塞是完整 observation 尚未实现；二者都在 class catalog 或 manifest 冻结前失败，因此不产生
`Qmap/Qlock`、member coverage 或 Windows numerator。

上述回执全部是本批 Map program/receipt 改动前的 prior baseline。本批 current source 只达到
`source_written/source_review_only/implementation_uncompiled/implementation_unrun`，新增窄 program、私有
receipt、准入与负向测试均为 `passed=0/failed=0/not_run`；不得把 prior `36/36` 或 exact blocker 回执当作
current-source 验证。

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
plan、同 root plan swap、non-capability semantic drift 与 admission binding digest 漂移。本批已写窄 Map
receipt/准入及相应负向测试源码；完整 `43,476` integration、validator tamper acceptance 与 Windows execution
仍待补齐，全部均未运行。

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
map_region_count_budget_completed_program=source_written_private_actual_receipt_parent_child_cleanup_uncompiled_unrun
map_supported_admission=private_exact_receipt_binding_only_source_contract_not_run
map_pre_manifest_program_inventory=source_written_full_root_two_pass_non_authorizing_uncompiled_unrun
map_program_inventory_status=planned_missing_or_source_present_receipt_required_only
map_program_inventory_digest=not_generated_not_frozen
map_program_inventory_member_and_group_counts=unknown_not_run
map_program_inventory_unrun_test_expectation=members_43476_source_present_members_2_source_present_groups_2_planned_missing_members_43474
map_default_producers=all_missing_quotient_runner_not_integrated
lock_runner_admission=unchanged_all_missing_lock_observation_incomplete
runner_admission_raw_supported=fail_closed_without_private_exact_receipt_not_run
dynamic_quotient_targeted=prior_passed_36_of_36_current_source_not_run
map_candidate_gate=prior_passed_expected_fail_closed_43476_current_source_not_run
lock_candidate_gate=prior_passed_expected_fail_closed_8668_current_source_not_run
map_bootstrap_descriptor_binding=expected_pre_freeze_drift_not_passed
map_descriptor_binding=frozen_d3ba08a5ba0019f9ccda99ace8b580ef06eb4d6653ba80c0db5497bec51bd870_exact_gate_accepted
lock_descriptor_binding=frozen_0cc951c8c979608fb9861167f8d880a74fd2e042c4d2cd42673100e14083e8ef_exact_gate_accepted
map_blocker=quotient_runner_not_integrated
lock_blocker=lock_observation_incomplete
quotient_manifests=not_frozen
Qmap=unknown
Qlock=unknown
map_dynamic_member_coverage=0/43476
lock_dynamic_member_coverage=0/8668
windows_dynamic=not_opened
compilation=current_source_not_run
targeted_unit_tests=current_source_not_run_passed_0_failed_0
windows_runtime=not_opened
```

本文不完成 A2，不注册生产 VFS，不调用生产 open，不创建 Connection/Opened authority，不获取 process
fence，不启动 A1/v15/Runtime/Ready，不产生 Provider、route、Offer、Attempt、Lease、派发、市场、结算或
资金效果。后续 program inventory 运行与冻结、完整源码 program、商 manifest、Windows evidence 与宽回归
必须各自按独立批次验收。
