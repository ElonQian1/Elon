---
title: 节点插件 VFS Map/Lock 动态商集验收 V1
status: current
reviewed_at: 2026-09-01
owners: node, security
design_status: design_frozen
implementation_status: typed_projector_candidate_prior_compiled_map_and_lock_pre_manifest_program_inventory_reviewed_source_admission_and_exact_private_execution_bridge_source_written_uncompiled_unrun
verification_status: prior_targeted_unit_36_passed_current_map_and_lock_source_inventory_admission_and_private_execution_not_run_review_digests_and_manifests_not_frozen
---

# Node Plugin VFS Map/Lock Dynamic Quotient Acceptance V1

## 1. Acceptance boundary

本页是 [`dynamic quotient authority`](node-plugin-vfs-map-lock-dynamic-quotient-authority.md) 的可执行
验收清单。静态分母事实只由
[`static denominator authority`](node-plugin-vfs-map-lock-static-denominator-authority.md) 维护；聚合 A2
状态由 [`fault acceptance`](node-plugin-vfs-fault-acceptance.md) 消费。本页不创建第二套静态 CaseKey、
Expected 或 source universe。

本功能分三次独立晋级：

1. 设计冻结：本文与 authority current，`Qmap/Qlock=unknown`；
2. 商 manifest 冻结：类型化 projector、精确分区与 frozen bytes 全部通过，机械得到 `Qmap/Qlock`；
3. Windows 动态接受：每个冻结 class 一条正式记录，整族原子形成 `Q/Q`。

当前仍只完成第 1 次晋级；同时已完成第 2 次晋级所需的部分基础设施，包括 typed descriptor/projector、
两遍原子 candidate、exact member-pair set 与 catalog/manifest canonical guard；这些只有前序编译、定向单元
测试和两个 root 完整失败关闭门禁的 prior baseline。本批新增 sealed admission，以及覆盖 Map
`RegionSizeBudget`、`RegionCountBudget`、`LogicalSizeBudget` 三类 `Completed` 请求的 source-only executable
program family、私有 actual receipt 和 parent/child/cleanup 合同；前序再新增 Map
`MapSingleRegionLifecycleV1` exact 六成员纵切，绑定 installed `xShmMap`、same-target pre/post、一次性 Map
ledger、NotPresent sentinel clear、mapped raw output 与受管 selected-region pointer 的仅进程内相等性以及
parent/child/cleanup。本批新增独立 q4 `MapRegionLoopSuccessV1`：Created-first empty Extend `n=1..256` 与
Node-live target-missing Extend `n=1..255` 两个 exact 子族，共 511 frozen members、扣除 q3 两项后净新增509；
q4 绑定精确 N 次有序 mapping ledger、完整 typed matcher、逐成员 seal catalog、source digest 与私有
parent/child/cleanup receipt。同时保留 Lock
`RangeOverflow`、`EndPastEight` 与 shared action 专属 `SharedMultiSlot` 的 exact 10-member managed
request-validation executable family，绑定真实安装的 `xShmLock`、原始 flags、`SQLITE_IOERR_SHMLOCK`、
raw-slot 不变、连接存活与 parent/child/cleanup；本批再新增 104 个 positive lifecycle source-wired program：
native acquire 44、native release 44、shared-local acquire/release 16，并绑定 same-target pre/post、exact
runtime/SHM identity、selected/sibling snapshot 与一次性 native/local ledger。另新增完整 Map/Lock 两个 root 的两遍
pre-manifest execution-program inventory，以及
`reviewed inventory -> source-program admission provider -> catalog/manifest binding` 的失败关闭源码桥。inventory
仍只区分 `PlannedMissing` 与 `SourcePresentReceiptRequired`，不签发 `Supported`；只有完整无 planned-missing 且
body digest 精确匹配 checked-in reviewed digest 时，provider authority 才可构造。current source 未编译、未运行，
source test 对 Map 预期有 `521` 个 source-present member/group、`42,955` 个 planned-missing member；对 Lock
本批又新增 source-only `LockStoredPoisonRetentionSucceededV1`，仅取
`UnsafeRetentionSucceededThenRouteUnknown`，15 profiles×88 action/range=1,320 frozen members，因而预期有
`1,434` 个 source-present member/group、`7,234` 个 planned-missing member。sibling
`UnsafeRetentionRouteUnknownThenRouteUnknown` 的另 1,320 仍 missing。两个 reviewed digest 均未
checked-in/frozen，也尚未成功产出
或冻结任何真实商 manifest。

## 2. Gate A — frozen static ingress

- [x] 静态 Map `43,476/43,476`、Lock `8,668/8,668` 已由独立权威冻结。
- [x] Map/Lock source universe、included/excluded 计数、ledger 与 manifest digest 已记录。
- [x] candidate 第一遍先验证 exact static manifest、ledger、source baseline、CaseKey、Expected 与 full-record seal。
- [x] 第二遍只把已验证的 included terminal full record 投入 projection/class；excluded 只进入拒绝守卫。
- [x] 任一 static drift、missing、extra、duplicate、unknown 或 unproved exclusion 使整次生成失败。

验收证据必须显示输入计数恰为 Map 43,476 与 Lock 8,668；读取 checked-in TSV 后解析 `leaf_id` 获得语义
不算通过。

## 3. Gate B — typed projector and erasure law

- [x] 每个 terminal 在图构建时产生与其同源的 typed descriptor；Map `43,476` 与 Lock `8,668` 均已由
      全量 candidate gate 证明 typed descriptor 可达；这不证明 executable runner 已存在。
- [x] projector 输入是完整 `LeafRecordV1` + typed descriptor，不暴露 `leaf_id`、family/test/debug 文本。
- [x] `DynamicClassKeyV1` 完整类型化 root、source site、stimulus/prestate、operation/phase/timing/occurrence、
      execution recipe 与 `DynamicExpectedV1`。
- [x] 未到达轴显式为 typed `NotReached`；没有通配、空值或 unknown fallback。
- [x] semantic digest 不吸收 CaseKey，且与现有 case-key-salted摘要使用不同 domain。
- [x] 不同 source site 不因相同 SQLite/Expected 合并。
- [x] root-specific producer coherence 对全部真实 typed tuple 失败关闭：source/stimulus/prestate/operation/
      phase/timing/occurrence/recipe/关键 axes 不可跨 tuple 拼接；未知 tuple、跨 root Missing gap、mixed
      Supported/Missing state 与 mixed Missing gap 均被拒绝。source-level `Supported` 形状严格限于 Map
      `RegionSizeBudget`、`RegionCountBudget`、`LogicalSizeBudget` 三类 `Completed` 请求、Map q3 exact 六成员
      single-region lifecycle、q4 `MapRegionLoopSuccessV1` 两个 bounded exact 子族，以及上述 Lock exact
      10-member 请求校验族、104-member positive lifecycle 族和
      `LockStoredPoisonRetentionSucceededV1` 1,320-member exact 族，且均仍须进入 root-specific 私有 actual receipt 复验。
- [x] projector provenance 覆盖 `producer_coherence/{map,map_axes,lock,lock_axes}.rs`、
      `descriptor_binding.rs`、`membership_commitment.rs`、`runner_admission.rs` 与
      `runner_admission/{canonical,map,map_program,lock,lock_program}.rs`、Map/Lock program 子模块、
      `projector/lock_execution.rs` 与 `a2_dynamic_evidence/{map_runner,lock_runner}` 及其 request/lifecycle 子模块，
      并覆盖 stored-poison catalog/source-scope/TSV、`a2lockq3` child/runner/fixture/payload/test-fault seam、
      Map/Lock lower ledger、installed callback observation 与 exact-target observer owner。
- [x] current source 从 validated/coherent typed key 内部编译 root-bound plan；producer 无法提交 plan。普通
      `resolve_v1` 仍拒绝裸 `Supported`；只有 root-specific execution projector 消费的私有
      `MapRunnerExecutionReceiptV1` 或 `LockRunnerExecutionReceiptV1` 与 exact member、normalized descriptor、
      plan、implementation/execution commitment 全部绑定时，才形成 `RunnerAdmissionDecisionV1::Supported`。
      跨 root/plan/stimulus/action swap 与 drift 失败关闭。
- [ ] 仅 run nonce、临时测试文件系统根、registration/route/runtime/connection/PID 等 harness binding 可 alpha-rename，
      并仍由每条 actual environment commitment 精确绑定。

Map 必须证明 mode、success、prestate、fault/cleanup/custody 轴均按 authority 保留；q4 还必须证明
`occurrence=ordinal=regions_to_create`、target-region 关系和 exact profile。Lock 必须证明 action、`first/count/mask`、prestate、contention、native
offset/effect 与 custody 均保留。任何未版本化、未执行的“看起来等价”人工消去都失败。

### 3.1 Gate B1 — non-authorizing pre-manifest program inventory

- [x] Source implementation：完整 Map/Lock root 各自复用两遍 frozen ingress，每个 included member 恰好进入一个
      capability-normalized execution program group；excluded member 不得进入 inventory。
- [x] Source implementation：`program_id` 使用独立 domain 绑定 root、projector schema、normalized descriptor
      digest 与内部 `plan_sha256`；它不是 dynamic class ID，也不复用 descriptor digest。
- [x] Source implementation：状态仅允许 `PlannedMissing(exact_gap)` 或
      `SourcePresentReceiptRequired { implementation_sha256 }`；本层没有 `Supported`、`ExecutionVerified` 或 runtime permit。
- [x] Source implementation：program group union 独立重建 reverse index，并与逐叶 membership 精确相等；
      missing/extra/duplicate/collision/binding drift/empty group 或 matcher 内部错误全部原子失败。
- [x] Test source：正式 denominator gate 与内部 bundle 测试锁定 Map 预期总成员 `43,476`，exact matcher 只允许
      `RegionSizeBudget`、`RegionCountBudget`、`LogicalSizeBudget` 三类 `Completed` 请求的 Observe/Extend 六组、
      各一 member；并允许 Empty/Reuse/TargetMissing 的 Observe/Extend 六个 exact q3 lifecycle member，以及
      `MapRegionLoopSuccessV1` 的 Created-first empty Extend 256 项和 Node-live target-missing Extend 255 项。
      511 项逐一绑定 typed descriptor、Expected 与 checked-in member seal，其中两个 ordinal-001 已由 q3 覆盖，
      故 source-present 为 `521` member / `521` group，其余 `42,955` member 保持 planned missing；leaf/case/full-record、
      profile、ordinal、regions-to-create、target-region、Expected、source digest 或 ledger receipt 任一扩张均失败关闭。
- [x] Test source：q4 `a2mapq4` selector 固定覆盖 511 项且使用固定 payload 宽度；lower ledger 对两族分别只接受
      `n=1..256` 与 `n=1..255`，并要求每次 `MappingCreate -> ViewMap -> Record` 精确有序重复 N 次，乱序、交错、
      少报、额外、错误 target/request/path 或未完成均失败关闭。
- [x] Test source：Lock 预期总成员 `8,668`；exact matcher 保留 Lock/Unlock × Shared/Exclusive 的
      `RangeOverflow`、`EndPastEight`，以及 Lock/Unlock × Shared 的 `SharedMultiSlot` 共 `10` member / `10` group，
并保留 native acquire 44、native release 44、shared-local acquire/release 16，既有合计为 `114` member / `114`
      source-present group；任何范围、prestate、operation、Expected 或 native/local receipt identity 扩张均失败关闭。
- [x] Test source：`LockStoredPoisonRetentionSucceededV1` 只匹配
      completion=`UnsafeRetentionSucceededThenRouteUnknown`，15 profiles 和 88 个合法 action/range 精确形成
      1,320 members；TSV 含 1,320 member rows、为 237,857 bytes，SHA-256 必须为
      `4da94c20e91d97a0082116879718b1ccf0271eb235ed785e65a2e36e7a949d85`。加上既有 114 后，Lock 未运行
      inventory 预期为 `1,434` source-present member/group 与 `7,234` planned-missing member；
      `UnsafeRetentionRouteUnknownThenRouteUnknown` 的另 1,320-member sibling 必须继续 missing。
- [x] Test source：`a2lockq3` 回执源码绑定 exact member/plan/implementation、installed `xShmLock`、
      `SQLITE_IOERR_SHMLOCK`、pre/poison/post snapshot、零 lower lock attempt、unsafe custody retention、route removal、
      registration/root 与 parent cleanup；selector/payload、profile、range、terminal custody 或 digest 漂移均失败关闭。
- [ ] Current-source verification：编译并运行上述 gate，记录实际 counts 与 inventory digest；本批按架构阶段约束
      保持 `passed=0 failed=0 actual=not_run`；这些 `[x]` 仅表示源码验收项已写，不表示实际测试通过。
- [ ] Freeze：每个 root 的完整 program source 全部 source-present 后，由独立 review 分别冻结 inventory
      bytes/digest，并让同 root quotient manifest context 绑定该 digest。当前不得执行此项。

Gate B1 只建立 manifest 前的程序规划分区；即使将来运行成功，也不能把 `43,476` 称为 `Qmap`、member
coverage 或 Windows numerator。

### 3.2 Gate B2 — reviewed source-program admission

- [x] Source implementation：raw inventory bundle、单个 `SourcePresentReceiptRequired` 或调用方提交的 digest
      不能直接授权 catalog；provider authority 只能从完整 inventory 与 checked-in reviewed digest 的 exact match 构造。
- [x] Source implementation：构造前重验 root/static/projector/descriptor context、program group union、reverse
      index、membership/catalog/body digest，并要求 planned-missing member/group 均为零、source-present 计数覆盖完整分母。
- [x] Source implementation：每个私有 source-program admission receipt 交叉绑定 reviewed inventory digest、
      member seal、`program_id`、normalized descriptor、`plan_sha256` 与 `implementation_sha256`；missing、extra、
      duplicate、member/program swap 或任一 commitment 替换都原子失败。
- [x] Source implementation：catalog 保留 producer 的 exact Missing runner-admission receipt 与 semantic key，只在
      source-program admission 精确成立后形成 class；本 gate 不生成 `Supported`、execution digest 或 Windows record。
- [x] Source implementation：quotient manifest context/body 绑定 reviewed inventory membership/catalog/body 与
      source-program admission commitment，不能跨 inventory 或 manifest 重放 member→program→class 关系。
- [ ] Current-source verification：本批不编译、不运行 Cargo、Windows 或真实 runtime；
      `passed=0 failed=0 actual=not_run`。
- [ ] Current full admission：source test 预期 Map `42,955`、Lock `7,234` 个 member 仍 planned-missing，且两个 root
      均没有 checked-in reviewed inventory digest，因此 provider authority 不可构造，full candidate 必须在
      catalog/manifest 前原子失败。

Gate B2 是 source completeness 和 review provenance，不是 actual execution。生产 actual-execution path 当前未开放；
现有 `#[cfg(all(test, windows))]` helper 只是 implementation fixture，不是 acceptance authority。未来验收要求真实
root-specific execution receipt 只能在 quotient manifest 冻结后，由 frozen class 的 canonical representative
在 Windows child 中产生。

## 4. Gate C — exact quotient manifests

每个 root 的候选必须通过以下 machine-readable 断言：

```text
class_count == Qroot
class_count == distinct projected class-key count
all canonical class keys and derived class IDs are unique within one root
all classes non-empty
union(class.members) == exact frozen included member set
pairwise intersection(class.members) == empty
missing == extra == duplicate == excluded_member == 0
unknown_projection == unexecutable_class == 0
representative_not_member == 0
member_reprojection_mismatch == 0
```

- [x] Implementation/fixture：每个 member 同时绑定 `case_key_sha256` 与 `full_record_sha256`，两遍 binding 和实际 class union 都重算 exact pair-set digest。
- [x] Implementation/fixture：class ID 由重算后的 canonical class-key digest 唯一派生，不能另设人工 selector 造成同 key 多 class。
- [x] Implementation/fixture：Representative 是按两个摘要字节序机械选择的最小成员。
- [x] Implementation/fixture：每个成功投影的 member 只进入其 exact canonical key 对应的 class。
- [x] Implementation/fixture：catalog、classes、membership map 与 reverse index 均为 private；root/schema-bound、
      排序后的 `member -> class ID digest` commitment 从实际 class union 重建并与 reverse index 精确相等。
- [x] Implementation/fixture：第二个冻结承诺绑定 root、schema、static manifest、included/entry count 与排序后的
      `member -> normalized full descriptor semantic key digest`；normalized key 只归一 capability，保留其余完整
      descriptor/class-key 语义，因此同 root、同 phase descriptor swap 也会失败。
- [x] Implementation/fixture：默认 producers 未放宽；Map 仍全量产生
      `Missing(QuotientRunnerNotIntegrated)`，Lock 仍只产生 `Missing(LockObservationIncomplete)`。
- [x] Source implementation：独立 runner-admission commitment 按 member 绑定 normalized descriptor、
      root-specific plan digest 与 exact decision；窄 Map/Lock `Supported` 均绑定 root-specific 私有
      implementation/execution digest，其余路径保留 exact gap。commitment 进入 receipt、catalog 和 manifest body；
      本项尚未运行验证。
- [x] Source implementation：reviewed inventory digest、program membership/catalog commitment 与 source-program
      admission binding 进入 manifest context/body；raw inventory status 不能替代该绑定，本项尚未运行验证。
- [x] Implementation/fixture：内存 manifest builder 绑定 static source baseline/ledger/manifest、projector version/digest、class-key set、membership、
      representative map、class catalog 与反向索引摘要。
- [x] Implementation/fixture：canonical bytes 长度分隔、enum 显式、整数定宽、成员排序稳定，和平台路径/locale/Debug 无关。
- [x] Implementation/fixture：两遍 frozen gate、catalog 和全部 manifest guard 成功后才返回内存 bundle；当前无 writer，失败不留下 frozen-looking partial file。
- [ ] 独立 review 复核生成器、frozen bytes、`Qmap/Qlock` 与 checked-in digest。

Gate C 的前序实现与隔离于生产 projector 的 test-only catalog/manifest fixture 定向单元测试通过不等于
real-root 商 manifest 通过。默认完整 producer inventory 仍精确产生上述 root-specific Missing capability。Lock 全量
candidate 的 prior baseline 已验证 exact `8,668` 输入后按预期因 `LockObservationIncomplete` 失败关闭；Map 全量
candidate 的 prior baseline 已验证 exact `43,476` 输入后按预期因 `QuotientRunnerNotIntegrated` 失败关闭。current
source 新增的 reviewed source-admission path 尚未运行；按 source contract，当前 Map/Lock inventory 都不完整且
未 review，必须更早原子失败，不能把 prior blocker 回执改写成 current result。两个 root 均没有成功形成可冻结 catalog，Gate C
整体仍未闭合。

前序基线测试事实如下；fingerprint 只标识相应验证回执，不提升验收层级。已知 Map q4 source baseline
`10aa60fb42488854657dd30a4240ad5f949c894d` 仅是前序 Map provenance，不是 current Lock stored-poison 执行证据；current
admission、program-inventory 与 reviewed source-admission source 均为 `not_run`，本批
`passed=0/failed=0`：

| Verification | Result | Fingerprint / actual |
|---|---|---|
| `dynamic_quotient::` | `36/36` passed | `aa96751fc2388adcf02469bac883ddf49583f5ffbfcf29252f781cff24da22f1` |
| Lock exact full gate | `8,668`，精确阻断于 `LockObservationIncomplete` | `a31c60597be461b3d90a2b54c91fd3d7faa1fb1ba7ade981401793701bf4bd7d` |
| Map bootstrap-only | 预期 `DescriptorBindingCommitmentDrift`，**不得记为通过** | fingerprint `cfeb50fb2b6652bad6d800806d23545c359e3883a8a4c1c9b3a9954cb390b69d`；actual commitment `d3ba08a5ba0019f9ccda99ace8b580ef06eb4d6653ba80c0db5497bec51bd870` |
| Map exact full gate | `43,476`，精确阻断于 `QuotientRunnerNotIntegrated` | `1540e34b6e4271e39771583162e228bfa604da8e47af18cf231558065afd5b80` |

Lock/Map exact full gate 的“通过”只表示预期 blocker 与失败关闭路径精确成立；没有 class catalog、manifest、
member coverage 或 Windows record 因此获得通过状态。

只有本 gate 完整通过后，才允许把 `Qmap/Qlock` 从 `unknown` 改成数字，并报告：

```text
Map  DynamicQuotientMemberCoverage=43476/43476
Lock DynamicQuotientMemberCoverage=8668/8668
```

这仍不增加 Windows numerator。

## 5. Gate D — negative and mutation guards

必须有独立负向测试逐项证明以下变异失败关闭：

- 删除、增加、重复一个 included member，或混入 excluded member；
- 空 class、representative 非成员、成员摘要漂移、反向索引漂移；
- class key、Expected、source site、execution recipe 或 canonical ordering 漂移；
- 同 root、同 phase 下交换任一完整 descriptor semantic axis，或只改 member→class / member→descriptor binding；
- 把一个合法 class 人为 split，或把两个不同 typed key 的 class merge；
- 构造 Supported/Missing 混合状态、跨 root capability gap 或同 catalog 的不同 Missing gap；
- 裸 `Supported` 无私有 actual receipt、跨 root plan、同 root plan swap、非 capability 语义漂移，或
  member/normalized descriptor/plan/decision/implementation/execution admission binding 被替换；
- raw `SourcePresentReceiptRequired` 绕过完整性门、planned-missing 非零仍构造 provider、未 checked-in 或错误的
  reviewed inventory digest、member/program swap、缺失/重复/额外 source-program receipt，或 inventory/
  admission commitment 未进入 manifest body；
- 使用 `leaf_id`/test name/list index 分类，或使用 case-key-salted digest 当 semantic key；
- 未知 enum 被默认化、unexecutable class 被跳过、`NotReached` 被当作 arbitrary；
- 消去 Map ordinal/regions-to-create，或消去 Lock `first/count/mask`；
- static manifest、ledger、source baseline 或 projector version 不一致仍尝试加载。

负向测试必须断言没有部分 manifest、没有计数晋级、没有 Windows record 被接受。

## 6. Gate E — DynamicExpected and real Windows record

每个冻结 class 必须有一个 process-isolated child 场景，通过真实安装的受管 VFS callback 链执行其 canonical
representative。验收逐字段比较：

1. projected static fields：与 `DynamicExpectedV1` exact equal；
2. runtime-bound identity：与本 child 的 registration/route/runtime/connection/root commitment exact equal；
3. independently observed actual：SQLite result、native receipt、topology、custody、counts、terminal state 与
   cleanup exact equal。

每条 record 必须绑定 class ID、完整 member-set digest、representative、static/quotient manifest、exact clean
Git SHA、Windows build/arch/filesystem/SQLite、child PID/nonce、actual semantic digest、child exit、unsafe custody
retention、parent root deletion receipt 与 validation fingerprint。

本批 source-only Map/Lock program 已把 parent-owned isolated root、Windows child execution、child terminal/exit 与
parent cleanup receipt 设计进私有 actual receipt。Map 覆盖三类 request-budget/`Completed`、q3 exact 六成员
single-region lifecycle 与 q4 `MapRegionLoopSuccessV1`；q3 要求 setup 在 ledger arm 前完成、恰好一次 target callback、NotPresent sentinel
clear，或 mapped raw output 与 lower selected pointer/length/region/generation 全部一致，且地址不得进入 payload、
digest、日志或错误文本。q4 独立协议进一步绑定两个 exact profile、target-region/ordinal/regions-to-create、精确 N 次
有序 mapping 事件、selected pointer relation、511 项 member seal 与 source digest。Lock 保留 exact
10-member managed request-validation 族，并新增 104-member positive lifecycle 族。前者绑定真实安装的 `xShmLock`
拒绝结果、raw-slot 不变与 registration custody；后者绑定 same-target pre/post、exact runtime/SHM identity、
selected/sibling masks 及一次性 native/local ledger。新增 `LockStoredPoisonRetentionSucceededV1` 又以
`a2lockq3` 绑定 poison profile/action/range、installed callback 失败、零 lower attempt、retained terminal custody、
route removal 与 parent cleanup。源码尚未编译或运行，`actual=not_run`，没有生成上述正式
record，也不能增加 class/member coverage 或 Windows numerator。

以下不算真实 record：直接调用 coordinator；从 Expected 合成 actual；仅观察注入器返回；复用另一 class
或 A2b2 family 的 record；同进程 panic 测试；缺 child exit 或 parent cleanup；跨 commit、跨 manifest、跨
environment 拼接。

## 7. Gate F — atomic family reducer and aggregate A2

Reducer 对 Map 与 Lock 各自只接受同一 cohort、checkout、environment、static manifest 与 quotient manifest
下的 exact class set。任一 missing、failed、duplicate、unknown、digest mismatch、cleanup failure 或 unsupported
class 都保持整族未完成；部分通过不能展示为 accepted numerator。

全部 class 正式通过后才允许原子报告：

```text
Map  StaticContract=43476/43476
     DynamicQuotientMemberCoverage=43476/43476
     WindowsDynamic=Qmap/Qmap

Lock StaticContract=8668/8668
     DynamicQuotientMemberCoverage=8668/8668
     WindowsDynamic=Qlock/Qlock
```

随后必须在同一 clean evidence commit 上重跑受影响 targeted、完整 `elon-pc-node` 测试目标与 wider
`sqlite_vfs_policy` regression，保存命令、环境、pass/fail/ignored、fingerprint 和 external receipt。只有
Map、Lock、既有 A2b2 `117/117` 与宽回归均闭合，聚合 A2 才可由其总权威另批晋级。

## 8. Reporting matrix

| Dimension | Map current | Lock current | Allowed interpretation |
|---|---:|---:|---|
| `StaticContract` | `43476/43476` | `8668/8668` | 静态 source-exhaustive合同已闭合 |
| typed projector/candidate | `prior compiled; current source uncompiled/unrun` | `prior compiled; current source uncompiled/unrun` | 实现存在，不等于 current source 已验证或 manifest 冻结 |
| sealed runner admission | `source written; exact receipt-only 6 request-budget + 6 q3 lifecycle + q4 511-member semantic scope/509 net-new support; default gap retained; not run` | `source written; exact receipt-only 10 validation + 104 lifecycle + 1320 stored-poison retention-succeeded; default gap retained; not run` | raw `Supported` 不能成为 permit；current actual 仍缺 |
| narrow executable program | `MapRegionLoopSuccessV1 q4: two exact families, exact N ordered mapping ledger, typed matcher, per-member seal/source digest; source-only; uncompiled/unrun` | `LockStoredPoisonRetentionSucceededV1: 15 profiles×88 action/range=1320, only UnsafeRetentionSucceededThenRouteUnknown, q3 exact receipt; source-only; uncompiled/unrun` | Lock 既有 10 validation+104 lifecycle 保留；源码形状不是动态执行证据 |
| stored-poison member catalog | `n/a` | `1320 member rows / 237857 bytes / SHA-256 4da94c20e91d97a0082116879718b1ccf0271eb235ed785e65a2e36e7a949d85` | 只是 checked-in source seal；不是 reviewed inventory 或 runtime receipt |
| pre-manifest program inventory | `full-root two-pass source written; unrun expectation: 43476 total, 521 source-present members/groups, 42955 planned-missing members` | `full-root two-pass source written; unrun expectation: 8668 total, 1434 source-present members/groups, 7234 planned-missing members` | 两个 digest 均未生成/冻结；不是 quotient、coverage 或 Windows evidence |
| reviewed source-program admission | `provider + catalog/manifest binding source written; reviewed digest absent` | `root-specific provider + catalog/manifest binding source written; reviewed digest absent` | 均未编译/运行；零 planned-missing + exact reviewed digest 才可构造；不是 actual execution |
| full candidate gate | `prior 43476 checked; current source expected to fail at 42955 missing` | `prior 8668 checked; current source expected to fail at 7234 missing` | current source 未运行；prior baseline 只证明当时失败关闭，不产生 quotient denominator |
| current blocker | `42955 planned-missing source expectation + no reviewed inventory digest + actual not run` | `7234 planned-missing source expectation + no reviewed inventory digest + actual not run` | stored-poison 对称 sibling 的 1320 项仍 missing；两个 root 的全局 blocker 均未解除 |
| frozen descriptor binding | `d3ba08a5ba0019f9ccda99ace8b580ef06eb4d6653ba80c0db5497bec51bd870`；checked-in / exact gate accepted | `0cc951c8c979608fb9861167f8d880a74fd2e042c4d2cd42673100e14083e8ef`；checked-in / exact gate accepted | descriptor binding 已冻结；quotient manifest 仍未冻结 |
| `DynamicQuotientMemberCoverage` | `0/43476` | `0/8668` | 尚无 frozen class/member commitment |
| quotient denominator | `Qmap=unknown` | `Qlock=unknown` | 不得预估或人工填写 |
| `WindowsDynamic` | `not_opened` | `not_opened` | 尚无正式 class record |
| current-batch compile/run/Windows execution | `not_run / not_run / not_run` | `not_run / not_run / not_run` | `passed=0 failed=0`；不是 Windows 动态证据 |
| global source-leaf authority scope | `shared blocker` | `5 production sources have baseline drift; ManagedNamespace/ManagedShmRoot continue to change in this tranche` | 静态审核事实；本批不扩张去重绑，未来编译或真实验收前须单独续绑，不构成新运行证据 |

禁止用 `43476/43476` 或 `8668/8668` 表示 Windows dynamic；禁止把一条 representative record 解释为
在没有 frozen member commitment 时天然覆盖其他静态成员。

## 9. Production isolation and current verdict

当前 verdict：`design_frozen / typed_projector_candidate_prior_compiled / MapRegionLoopSuccessV1=q4 two exact families, 511 frozen members, 509 net-new / LockStoredPoisonRetentionSucceededV1=15 profiles x 88 action-ranges, 1320 frozen members, sibling 1320 still missing / current_source=written_uncompiled_unrun / compile=Cargo=Windows=runtime=not_run / current unrun expectations: Map source-present=521 and planned-missing=42955; Lock source-present=1434 and planned-missing=7234 / passed=0 failed=0 actual=not_run / reviewed inventory digests and quotient manifests=not_frozen / Qmap=unknown / Qlock=unknown / member_coverage=0/43476+0/8668 / WindowsDynamic=not_opened / production_effect=none`。

本功能不注册生产 VFS，不调用 production open，不创建 Connection/Opened authority，不接 A1/v15、Runtime、
Ready、Provider、route、Offer、Attempt、Lease、派发、市场、结算或资金。任何 Gate A-F 未闭合时，A2 都
继续是 `implementation_not_dynamically_accepted`。
