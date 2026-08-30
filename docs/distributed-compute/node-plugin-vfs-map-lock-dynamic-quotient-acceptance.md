---
title: 节点插件 VFS Map/Lock 动态商集验收 V1
status: current
reviewed_at: 2026-08-31
owners: node, security
design_status: design_frozen
implementation_status: implementation_uncompiled
verification_status: not_run
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

设计冻结不等于实现、编译、生成 manifest 或动态通过。当前只完成第 1 步。

## 2. Gate A — frozen static ingress

- [x] 静态 Map `43,476/43,476`、Lock `8,668/8,668` 已由独立权威冻结。
- [x] Map/Lock source universe、included/excluded 计数、ledger 与 manifest digest 已记录。
- [ ] 生成器先验证 exact static manifest、ledger、source baseline、CaseKey、Expected 与 full-record seal。
- [ ] 生成器只观察 included terminal full record；excluded 不进入 class builder。
- [ ] 任一 static drift、missing、extra、duplicate、unknown 或 unproved exclusion 使整次生成失败。

验收证据必须显示输入计数恰为 Map 43,476 与 Lock 8,668；读取 checked-in TSV 后解析 `leaf_id` 获得语义
不算通过。

## 3. Gate B — typed projector and erasure law

- [ ] 每个 terminal 在图构建时产生与其同源的 typed descriptor。
- [ ] projector 输入是完整 `LeafRecordV1` + typed descriptor，不暴露 `leaf_id`、family/test/debug 文本。
- [ ] `DynamicClassKeyV1` 完整类型化 root、source site、stimulus/prestate、operation/phase/timing/occurrence、
      execution recipe 与 `DynamicExpectedV1`。
- [ ] 未到达轴显式为 typed `NotReached`；没有通配、空值或 unknown fallback。
- [ ] semantic digest 不吸收 CaseKey，且与现有 case-key-salted摘要使用不同 domain。
- [ ] 不同 source site 不因相同 SQLite/Expected 合并。
- [ ] 仅 run nonce、临时测试文件系统根、registration/route/runtime/connection/PID 等 harness binding 可 alpha-rename，
      并仍由每条 actual environment commitment 精确绑定。

Map 必须证明 mode、六类 success、prestate、fault/cleanup/custody 轴均按 authority 保留；ordinal 与
regions-to-create 在 V1 默认保留。Lock 必须证明 action、`first/count/mask`、prestate、contention、native
offset/effect 与 custody 均保留。任何未版本化、未执行的“看起来等价”人工消去都失败。

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

- [ ] 每个 member 同时绑定 `case_key_sha256` 与 `full_record_sha256`。
- [ ] class ID 由 canonical class-key digest 唯一派生，不能另设人工 selector 造成同 key 多 class。
- [ ] Representative 是按两个摘要字节序机械选择的最小成员。
- [ ] 每个 member 重投影后得到本 class 的 exact canonical key。
- [ ] manifest 绑定 static source baseline/ledger/manifest、projector version/digest、class-key set、membership、
      representative map、class catalog 与反向索引摘要。
- [ ] canonical bytes 长度分隔、enum 显式、整数定宽、成员排序稳定，和平台路径/locale/Debug 无关。
- [ ] 写候选发生在整流和全部 guard 成功之后；失败不留下 frozen-looking partial file。
- [ ] 独立 review 复核生成器、frozen bytes、`Qmap/Qlock` 与 checked-in digest。

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
- 把一个合法 class 人为 split，或把两个不同 typed key 的 class merge；
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
| `DynamicQuotientMemberCoverage` | `0/43476` | `0/8668` | projector/manifest 尚未实现与冻结 |
| quotient denominator | `Qmap=unknown` | `Qlock=unknown` | 不得预估或人工填写 |
| `WindowsDynamic` | `not_opened` | `not_opened` | 尚无正式 class record |
| compile/runtime | `not_run` | `not_run` | 架构铺设阶段按要求未执行 |

禁止用 `43476/43476` 或 `8668/8668` 表示 Windows dynamic；禁止把一条 representative record 解释为
在没有 frozen member commitment 时天然覆盖其他静态成员。

## 9. Production isolation and current verdict

当前 verdict：`design_frozen / implementation_uncompiled / implementation_unrun / WindowsDynamic=not_opened`。

本功能不注册生产 VFS，不调用 production open，不创建 Connection/Opened authority，不接 A1/v15、Runtime、
Ready、Provider、route、Offer、Attempt、Lease、派发、市场、结算或资金。任何 Gate A-F 未闭合时，A2 都
继续是 `implementation_not_dynamically_accepted`。
