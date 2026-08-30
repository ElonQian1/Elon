---
title: 节点插件 VFS Map/Lock 动态商集权威 V1
status: current
reviewed_at: 2026-08-31
owners: node, security
design_status: design_frozen
implementation_status: implementation_uncompiled
verification_status: not_run
authority_scope: backend-a2-map-lock-dynamic-quotient-authority-v1
---

# Node Plugin VFS Map/Lock Dynamic Quotient Authority V1

## 1. Authority scope and dependency

本文冻结 A2 Map/Lock 从完整静态叶集合到真实 Windows 执行类集合的唯一 V1 合同。它依赖
[`Map/Lock static denominator authority`](node-plugin-vfs-map-lock-static-denominator-authority.md)，
不重算、改写或缩小静态分母。静态合同已经闭合为 Map `43,476/43,476`、Lock
`8,668/8,668`；动态商只决定这些 included 静态成员如何形成可执行等价类。

动态分母分别记作 `Qmap` 与 `Qlock`。它们只能由本文规定的类型化生成器对完整静态记录机械投影、
精确分区并冻结 manifest 后得出。当前没有该生成器或冻结 manifest，所以两个值都是 `unknown`；
不得根据历史 18+10 case、leaf 名称、人工抽样或 Expected 摘要预填。

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

静态 source baseline 为 `47cb2652321b42cc9689319075d253fe2275ace1`。未来生成器必须先完整执行
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

## 8. Class catalog and member commitments

每个 `DynamicClassV1` 必须冻结：

- canonical class key、class-key semantic digest、由该 digest 唯一派生的 class ID、root、schema/projector version；
- 排序后的 `(case_key_sha256, full_record_sha256)` 成员列表、member count 与 member-set digest；
- canonical representative 的 case-key/full-record digest；
- typed source site、保留轴、显式消去轴及相应 proof kind/digest；
- `DynamicExpectedV1`、execution recipe、fixture/observer/cleanup schema；
- 所属 static manifest/ledger/source baseline 与全局 class catalog digest。

Representative 只能从成员中机械选择，V1 固定为按 `case_key_sha256`、再按 `full_record_sha256` 的
unsigned byte order最小者；不得按 `leaf_id`、测试名或人工偏好选择。代表成员只是该 class 的执行载体，
不是其他成员被忽略的理由；class record 必须绑定完整 member-set commitment。

V1 canonical digests 使用独立 domain：

```text
ELON-A2-MAP-LOCK-DYNAMIC-EXPECTED-V1
ELON-A2-MAP-LOCK-DYNAMIC-CLASS-KEY-V1
ELON-A2-MAP-LOCK-DYNAMIC-MEMBER-SET-V1
ELON-A2-MAP-LOCK-DYNAMIC-QUOTIENT-MANIFEST-V1
```

canonical encoding 必须长度分隔、枚举显式、整数定宽、成员按摘要字节排序；禁止 JSON map 顺序、Debug
文本、平台路径、pointer、进程 ID 或 locale 进入 digest。具体编码实现与 frozen bytes 由后续源码批次
提交；本文不虚报其已存在。

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

生成顺序固定为：验证完整静态流；观察每条 full record + typed descriptor；在内存中构建全部 classes；
复核两个 root 的精确分区与 canonical digests；最后才允许写候选文件。中途失败不得留下可被误认作
frozen 的部分 manifest。checked-in frozen manifest 必须经独立 review 后另批提交，且任何 static 或
projector 漂移都要求全量重生成与重审。

负向守卫至少覆盖缺成员、额外成员、重复成员、excluded 混入、空类、representative 非成员、成员摘要
漂移、class split/merge、semantic digest 漂移、未知 enum、leaf-text 分类、case-salted digest 误用、Map
ordinal 非法消去与 Lock range 非法消去。

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
quotient_projector=not_implemented
quotient_manifests=not_frozen
Qmap=unknown
Qlock=unknown
map_dynamic_member_coverage=0/43476
lock_dynamic_member_coverage=0/8668
windows_dynamic=not_opened
compilation=not_run_by_architecture_phase
runtime=not_run_by_architecture_phase
```

本文不完成 A2，不注册生产 VFS，不调用生产 open，不创建 Connection/Opened authority，不获取 process
fence，不启动 A1/v15/Runtime/Ready，不产生 Provider、route、Offer、Attempt、Lease、派发、市场、结算或
资金效果。后续源码、冻结 manifest、Windows evidence 与宽回归必须各自按独立批次验收。
