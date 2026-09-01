---
title: 节点插件 VFS Map/Lock 动态商集权威 V1
status: current
reviewed_at: 2026-09-01
owners: node, security
design_status: design_frozen
implementation_status: typed_projector_candidate_prior_compiled_map_and_lock_q12_pre_manifest_program_inventory_reviewed_source_admission_and_exact_private_execution_bridge_source_written_uncompiled_unrun
verification_status: prior_targeted_unit_36_passed_current_map_lock_q12_inventory_source_admission_and_private_execution_not_run_review_digests_and_manifests_not_frozen
authority_scope: backend-a2-map-lock-dynamic-quotient-authority-v1
---

# Node Plugin VFS Map/Lock Dynamic Quotient Authority V1

## 1. Authority scope and dependency

本文冻结 A2 Map/Lock 从完整静态叶集合到真实 Windows 执行类集合的唯一 V1 合同。它依赖
[`Map/Lock static denominator authority`](node-plugin-vfs-map-lock-static-denominator-authority.md)，
不重算、改写或缩小静态分母。静态合同已经闭合为 Map `43,476/43,476`、Lock
`8,668/8,668`；动态商只决定这些 included 静态成员如何形成可执行等价类。

动态分母分别记作 `Qmap` 与 `Qlock`。它们只能由本文规定的类型化生成器机械投影、精确分区并冻结
manifest 后得出。current source 将 program grouping、source presence、reviewed admission 与 actual execution
分层，任何 planned-missing 或 reviewed digest 缺失都会在 manifest 前失败关闭。未运行 inventory 的 Map
预期为 `521 present / 42,955 missing / 43,476 total`。Lock q1–q11 已有 3,668 个 source-present
program；q12 `LockNativeAcquireCreatedFirstExclusiveReleaseErrorV1` 又把 CreatedFirst + DMS exclusive
release outcome uncertain 的 44 个合法 acquire request × 两个 unsafe terminal completion 精确写成 88 个
singleton normalized group，故 q1–q12 未运行 inventory 预期为
`3,756 present members / 3,756 present groups / 4,912 missing members / 4,384 missing groups /
8,668 total members / 8,140 total groups`。完整 initialization umbrella 仍缺 3,344 members / 2,816
groups。q9–q12 的成员、production/controlled-fault actual chain、receipt 和排除边界由
[`Lock dynamic tranches authority`](node-plugin-vfs-lock-dynamic-tranches-authority.md) 维护。两根均未编译、
未运行，没有 checked-in reviewed inventory digest、manifest 或 actual acceptance。
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
10 个 request-validation / 104 个 positive lifecycle program / 两个 stored-poison completion 各
1,320 个 exact program，且持有私有、进程隔离 actual receipt 的
`Supported` descriptor 才可通过 program-local 准入；
因此当前完整 candidate 仍没有
class 被放行。

### 7.0 Bounded programs and reviewed inventory authority

Map/Lock 的 bounded execution programs、pre-manifest program inventory 与 reviewed
source-program admission 已按职责拆入
[`Execution program inventory authority`](node-plugin-vfs-execution-program-inventory-authority.md)。
该子权威只维护 source completeness 与非授权准入；本文继续唯一维护 class laws、manifest、
exact partition、`Qmap/Qlock`、Windows evidence 和生产门控。
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
`runner_admission/{canonical,map,map_program,map_program/request_budget,lock,lock_program,lock_program/request_validation,lock_program/lifecycle}.rs`，
以及 Lock stored-poison 两 completion 与 q5–q12 exact tranche 的 program/catalog/source-scope/runner seams；
其中任一接受关系或 commitment 编码变化都必须触发 projector provenance drift 和全量重审。

同一 commitment 还绑定真实执行 envelope：`a2_dynamic_evidence` 的 child/capture/environment/cleanup 与
Map/Lock runner，managed VFS 的 registration/connection/multi-connection/route/callback/fault wrapper，registry
bridge/custody，installed `sqlite_vfs_abi`，以及 managed-fs 的 module dispatch、Windows lock/SHM、coordinator、
types、initialization、mapping、snapshot、fault controller/operation/mapping 和一次性 Lock ledger。Lock lifecycle
implementation digest 使用上述 projector 全集中与 q2 Lock lifecycle execution 直接相关的固定子集，并另加
one-based program tag；因此 exact-target observer、installed ABI、
native/local path 或 parent/child cleanup 的直接语义依赖发生变化时，摘要必须漂移，不能只绑定 q2 自身文件。
Stored-poison implementation digest 同样绑定完整执行闭包，而 quotient projector provenance 只追加该族的
新增、名称唯一 source delta；两者不得互相替代或用重复名称改变 manifest 语义。

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
runner/ledger/receipt；后续又以独立 q4 `MapRegionLoopSuccessV1`、精确 N 次有序 ledger、typed matcher、逐成员
seal 与 source digest 净新增 509 个 source-present member。其真实阻塞已细化为剩余 `42,955` 个 planned-missing、current source 未编译/未运行和
reviewed inventory digest 缺失。Lock 的真实
阻塞是完整 observation 尚未实现；二者都在 class catalog 或 manifest 冻结前失败，因此不产生
`Qmap/Qlock`、coverage 或 Windows numerator。Lock q3/q4 各 1,320、q5/q6 各 44、q7 为 192、q8 为
88、q9 为 528、q10 为 7、q11 为 11、q12 为 88；仍缺 4,912 members / 4,384 groups、编译/运行、actual
receipts 与 reviewed digest。q9 的 pre-managed production rejection/completion observation、q10 的 ABI
scalar installed-callback receipt、q11 的 production raw-state rejection/cleanup ledger，以及 q12 在真实
CreatedFirst DMS release 点故意丢弃 `UnlockFileEx` BOOL 的 `controlled_fault_actual` receipt 都只是
source-only shape。

上述回执全部早于 current Map/Lock program/receipt source。Map q4 的已知 clean source baseline
`10aa60fb42488854657dd30a4240ad5f949c894d` 只是前序 Map provenance，不是本批 Lock 执行证据；当前只达到
`source_written/source_review_only/implementation_uncompiled/implementation_unrun`。Map q3/q4 与 Lock 10+104、q3-q12 的 receipt/inventory/admission/binding source 均为 `passed=0/failed=0/not_run`；prior `36/36` 不是 current 验证。

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
lock_positive_lifecycle_programs=source_written_q2_104_native_acquire_44_native_release_44_shared_local_16_observation_native_receipt_wired_uncompiled_unrun
lock_stored_poison_retention_succeeded_v1=source_written_q3_1320_frozen_members_catalog_rows_1320_bytes_237857_sha256_4da94c20e91d97a0082116879718b1ccf0271eb235ed785e65a2e36e7a949d85_uncompiled_unrun
lock_stored_poison_route_unknown_then_route_unknown_v1=source_written_q4_1320_frozen_members_catalog_rows_1320_bytes_237857_sha256_df931ad7725843098f228d07d9798d79e92f2beec4e1c23e83fc89219dfa1396_route_preemption_bridge_uncompiled_unrun
lock_native_acquire_node_live_native_busy_completed_v1=source_written_q5_44_members_8_shared_single_slot_36_exclusive_contiguous_real_distinct_handle_win32_byte_range_contention_exact_receipt_uncompiled_unrun
lock_native_acquire_busy_catalog=rows_44_sha256_b12bd411f7fa63f822e65a679351dfc103a6368e2887355d5b03c530fc162e2f
lock_local_sibling_contention_completed_v1=source_written_q6_44_8_shared_sibling_exclusive_36_exclusive_sibling_any_real_two_connection_local_busy_zero_selected_native_or_mutation_uncompiled_unrun
lock_callback_completion_route_unknown_v1=source_written_q7_192_native_acquire_acquired_44_native_acquire_busy_44_shared_local_acquire_8_sibling_busy_44_native_release_success_44_shared_local_release_8_real_lower_receipt_then_exact_one_shot_route_removal_then_production_completion_unknown_uncompiled_unrun
lock_callback_completion_route_unknown_catalog=rows_192_bytes_39203_sha256_e9f509d52d294405dd1a7ae528c514a31ba4e0758959374b633bdca2b571d631
lock_local_protocol_own_overlap_or_not_held_completed_v1=source_written_q8_88_exact_members_uncompiled_unrun_details_in_lock_tranches_authority
lock_pre_managed_callback_rejection_v1=source_written_q9_528_exact_singleton_members_and_groups_six_families_of_88_uncompiled_unrun_details_in_lock_tranches_authority
lock_abi_scalar_rejection_v1=source_written_q10_7_exact_singleton_members_offset_count_flags_validity_2_pow_3_minus_1_installed_xshmlock_direct_rejection_uncompiled_unrun_details_in_lock_tranches_authority
lock_raw_state_rejection_v1=source_written_q11_11_exact_singleton_members_after_2_pointer_safety_premise_exclusions_installed_xshmlock_production_raw_admission_abandon_adapter_custody_ledger_uncompiled_unrun_details_in_lock_tranches_authority
lock_native_acquire_created_first_exclusive_release_error_v1=source_written_q12_88_exact_singleton_members_44_legal_acquire_requests_times_2_unsafe_terminal_completions_real_created_first_dms_lock_and_truncate_real_unlockfileex_bool_receipt_deliberately_unread_controlled_fault_actual_only_uncompiled_unrun_details_in_lock_tranches_authority
lock_supported_admission=private_exact_receipt_binding_only_source_contract_not_run
lock_pre_manifest_program_inventory=source_written_full_root_two_pass_non_authorizing_uncompiled_unrun
lock_program_inventory_status=planned_missing_or_source_present_receipt_required_only
lock_program_inventory_digest=not_generated_not_frozen
lock_program_inventory_member_and_group_counts=unknown_not_run
lock_program_inventory_unrun_test_expectation=members_8668_groups_8140_source_present_members_3756_source_present_groups_3756_planned_missing_members_4912_planned_missing_groups_4384
lock_reviewed_inventory_digest=not_checked_in_not_frozen
lock_source_program_admission_provider=source_written_fail_closed_uncompiled_unrun
lock_source_program_admission_current=unconstructible_unrun_source_expectation_planned_missing_members_4912_planned_missing_groups_4384_compile_runtime_actual_receipts_and_reviewed_digest_absent
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
WindowsDynamic=not_opened
map_region_loop_windows_execution=not_run
lock_q3_q4_stored_poison_q5_native_busy_q6_local_sibling_contention_q7_callback_completion_route_unknown_q8_local_protocol_rejection_q9_pre_managed_callback_rejection_q10_abi_scalar_rejection_q11_raw_state_rejection_q12_created_first_exclusive_release_error_windows_execution=not_run
global_source_leaf_authority_scope=static_confirmed_preexisting_q9_q12_drift_19_artifact_refresh_independently_deferred_production_sources_including_current_managed_namespace_and_managed_shm_root_not_rebound_separate_rebind_required_before_compile_or_runtime_acceptance
compilation=not_run
cargo=not_run
targeted_unit_tests=not_run_passed_0_failed_0
windows_runtime=not_opened
current_batch_actual_inventory_receipt_and_windows_record=not_run_not_generated_not_accepted
lock_production=closed
```

本文不完成 A2，不注册生产 VFS，不调用生产 open，不创建 Connection/Opened authority，不获取 process
fence，不启动 A1/v15/Runtime/Ready，不产生 Provider、route、Offer、Job、Attempt、Lease、派发、市场、结算或
资金效果。后续 program inventory 运行、完整源码 program、独立 review/freeze、source-program admission、商
manifest、manifest 后 Windows evidence 与宽回归必须各自按顺序独立验收。
