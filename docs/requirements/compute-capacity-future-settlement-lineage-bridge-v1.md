---
title: capacity_future 交付结算谱系桥 V1 需求
version_status: current
status: accepted
reviewed_at: 2026-09-02
owners: backend, ai-economy
feature_id: compute-capacity-future-settlement-lineage-bridge-v1
implementation_status: source_contract_written_uncompiled_unrun
---

# `capacity_future` 交付结算谱系桥 V1 需求

## 目标

一龙需要从一笔已经结算的任务 Lease，可信解释该任务是否消费了未来容量合约，以及它所引用的
CapacityInstrument、CapacityCommitment、exercised DeliveryAllocation、锁价 Price Snapshot、执行、验证、
结算和内部释放事实。V1 建立一份 API-free、reference-only、可 canonicalize 的 retained lineage，并由
Store 在单一只读快照内从历史 owner 重建后封存。

该 bridge 只解决历史可追溯性，不成为新的价格、用量、容量、结算或资金 owner。它不能证明当前仍在
pending、Provider 已收款、余额已提现或外部付款已经完成。

## 唯一 owner 与组合边界

- v238 CapacityInstrument 拥有标准合约、activation 和 exact Offer adoption/publication。
- v225 CapacityCommitment 拥有 future 容量承诺、父 Claim 与锁价根。
- v228 DeliveryAllocation 拥有 Grant、whole-only exercise、父 Claim release 和子 Reservation Claim hold。
- v171 Price Snapshot 拥有不可变锁价，不能被当前曲线或任务结束价替代。
- F0 execution/verification/settlement/release carriers 继续拥有 Provider、Offer、Job、Reservation、Lease、
  Execution Receipt、VerificationDecision、SettlementReceipt 和可选 v198 release 的历史引用。
- bridge 只组合上述 owner；不得创建平行 Instrument、Claim、Receipt、ClearingReceipt 或第二套 F0 profile。

这些是技术前置，不虚构为尚未登记的 Feature Registry dependency。

## Domain 合同

1. envelope 固定 exact 6 keys、独立 schema/kind/domain、RFC 8785 JCS 和 domain-separated SHA-256。
2. 输入最多 262144 bytes；对象拒绝未知字段，revision/version 只允许 I-JSON 安全正整数，时间使用
   canonical UTC nanoseconds，opaque digest 只允许 64 位小写十六进制。
3. `pricing_mode=capacity_future`、`currency=CNY`；reference effect 固定，其余状态、容量、验证、结算、
   资金和提款 effect 全部固定为 `none`。
4. parser 和 source builder 只产生 untrusted/projected 值；只有 Store owner 重建后才能形成 private seal。
5. pending 与 available 使用封闭 tagged union，不使用 `Option`/`null` 伪造未知经济终态。

## Store 合同

- 唯一 facade 只接受 `lease_id`，在一次 `TransactionBehavior::Deferred` 内完成全部 owner 读取。
- 无 settlement、Snapshot 不是逐字 `capacity_future`、或该 Reservation 确实没有 exercised v228 时返回
  `None`；一旦 Reservation 存在 exercised v228，Claim 索引漂移、重复 owner、缺失 owner 或摘要错接必须
  作为 integrity failure，不能降格为“不适用”。
- historical Instrument retired、Offer draining 或 current head 前移不能否定 exact retained history；禁止
  current/latest fallback。
- Commitment、Claim 与 Instrument 的 meter 集合、顺序、granularity 和共同正整数 multiplier 必须一致；
  whole-only parent release 与 child hold 必须守恒。
- v193 必须精确绑定 v192 VerificationDecision event digest；v192 verification-role 与 v195
  settlement-role usage digest 必须分别由各自 owner 公式从同一 v193 readings 重算，禁止直接比较两套摘要。
- v195 consumer 必须绑定 Allocation consumer；payee 必须由 exact historical Provider 的
  `settlement_account_id` 审计，未配置时才回退 Provider owner，不得把两者直接判等。
- Store seal 保持 crate-visible、private-field、non-Clone、non-Serde，并携带不进入 canonical envelope 的
  participant/project scope。

## 非目标

- 不新增 Order、Trade、Position、ClearingReceipt、指数价、标记价、自动撮合或真实价格源。
- 不计算 shortfall、unused、奖励、处罚、差额或 Provider 可提现收益。
- 不改变 v192、v195 或 F0 的 schema、canonical bytes、digest domain 和 owner 公式。
- 不新增 table、migration、writer、Service、HTTP、MCP、PC、网络、运行时或资金动作。
- 不接入 `spot`、`index_locked` 或 `capacity_forward`，也不把所有 `capacity_future` Reservation 解释为
  exercised DeliveryAllocation。

## 第一批源码范围

第一批完成正式需求、Feature Registry 登记、Domain/Store source contract、cross-owner 等式门、历史 owner
适用性门和静态守卫。按当前架构铺设约束，不运行 Cargo、rustfmt、Rust 测试、SQLite、migration、服务、
网络或真实资金流程；`compiled=0`、`run=0`、`passed=0`、`failed=0`。

因此第一批必须以 `blocked` 收口，阻塞原因是动态证据被明确推迟，而不是源码缺少声明式实现。不得推进到
`implemented`、`verified` 或 `released`。

## 验收标准

1. 正式 current requirement 绑定权威与验收文档，明确本 feature 是容量 owner 到既有 F0 carrier 的组合桥。
2. exact 6-key envelope、独立 schema/domain、JCS/SHA-256、字节/数值边界和固定 zero-effect 值失败关闭。
3. Store facade 仅接收 Lease ID，并在单一 Deferred snapshot 重建 v238→v225→v228→F0→v195 和可选 v198。
4. 真正无 exercised v228 才返回 `None`；存在后的 Claim 错接、cardinality、owner 缺失和摘要漂移均失败关闭。
5. retired/draining 历史 owner 可审计，禁止 current/latest fallback。
6. meter、顺序、granularity、共同 multiplier 和 parent-release/child-hold 守恒精确成立。
7. v193→v192 decision digest 和其余跨 owner 根逐字闭合；v195 payee 由 historical Provider 的
   settlement-account fallback 规则审计，不错误等同 Provider owner。
8. v192/v195 usage digest 分别复用原 owner 公式，不比较彼此，也不用 declared/observed usage 替代。
9. pending/available 是封闭历史分支，不外推 current pending、withdrawn、external paid 或 clearing。
10. 本批只有 source/document evidence；无 migration、API、写入或经济效果，并保持 uncompiled/unrun、
    `passed=0/failed=0` 和 Feature Registry `blocked` 状态。

## 后续动态完成门槛

未来解除 `blocked` 至少需要完整 Rust target 编译、fresh/repeat migration、Store/SQLite 正反例、cross-splice、
corruption、participant scope、pending/available、数据库重开和精确通过计数。若以后增加 Service/API，还必须
独立验证鉴权、项目隔离、响应脱敏和零业务写入。旧 v171/v192/v195/v198/v225/v228/v238/F0 的通过记录
不能冒充本 bridge 的动态证据。
