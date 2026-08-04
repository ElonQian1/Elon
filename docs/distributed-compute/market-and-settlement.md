---
title: 分布式算力市场与期货锁价结算
status: current
reviewed_at: 2026-08-04
owners: ai-economy, backend
---

# 分布式算力市场与期货锁价结算

## 1. 市场目标

一龙不只做一次任务的低价路由，而要形成未来算力的价格发现和交付市场：需求方可以提前锁定某种 AI 能力在未来时间窗的价格与容量，供给方可以出售闲置节点或集群的标准化容量，一龙负责撮合、验证、交割和清算。

本文只定义技术与经济合同。法规、牌照和地域合规不属于本轮设计范围，也不改变核心合同抽象。

## 2. 不采用“一个 TFLOPS 价格”

不同 GPU、模型、精度、运行时、上下文和信任等级无法仅靠理论 FLOPS 公平比较。市场以 `ComputeSku` 为最小可交易单位：

```text
task_kind
+ model_family / model_digest / tokenizer_digest
+ runtime_family / precision
+ context_or_shape_bucket
+ verification_tier / sla_tier
+ region_or_data_zone
+ delivery_window
```

同一模型不同量化、不同上下文长度或不同验证等级是不同 SKU。Offer 可以覆盖多个相邻档位，但撮合前必须归一成确定 SKU。

## 3. 计量单位

| 任务 | 基础计量 |
|---|---|
| LLM / 对话 | 输入 Token 与输出 Token 分开计价 |
| Embedding / Rerank | 输入 Token 或经过验证的文档单元 |
| 图像生成 | 模型摘要 + 分辨率档 + steps + 图片数 |
| 视频生成 | 模型摘要 + 分辨率档 + 帧/秒数 + steps |
| 批评测 | 样本数 + 模型/运行时约束 |
| 通用 GPU 批任务 | 挑战基准归一化 GPU-second + 显存档 |
| 容量交割 | 合格 SKU 实例数 × 时间 × 可用率/吞吐承诺 |

最终结算只能消费 `verified_usage`。Provider 声明值和平台观测值用于验证、诊断和争议，不直接覆盖最终值。

## 4. 价格模式

核心合同预留四种模式：

- `spot`：任务提交附近的即时供需价格；
- `index_locked`：锁定某个公开指数快照；
- `capacity_forward`：双方锁定未来窗口的容量和价格；
- `capacity_future`：标准化合约通过市场成交，任务消费持仓交割出的 Price Snapshot。

项目首期按 `capacity_future` 参考价格结算：Broker 在 Reservation 前锁定一个不可变 Price Snapshot。即使第一阶段的价格曲线由平台算法发布、尚无连续订单簿，合同仍使用期货交付窗口和版本化曲线，不回退到任务结束时的现货价。

## 5. 算力期货价格曲线

每个 SKU 和交付窗口维护一条版本化曲线。算法可以从已接受成交、可用供给、排队需求、完成概率、能源/带宽代理成本和季节性生成参考价：

```text
future_reference = spot_index
                 + capacity_scarcity_premium
                 + delivery_risk_premium
                 + time_and_seasonality_adjustment
```

实现中不使用浮点数。各项以整数微单位和基点计算，保存曲线版本、观察窗口、样本量和回退来源。没有足够成交时，按上个有效指数、Provider 报价中位数和平台保底曲线依次回退，并在快照中记录来源。

## 6. Price Snapshot

Reservation 必须绑定不可变快照，至少包含：

- `snapshot_id`、`schema_version`、`pricing_mode`；
- `sku_id`、曲线/合约版本和交付窗口；
- 计价单位与币种/平台积分单位；
- 消费者输入、输出或容量单价；
- Provider 输入、输出或容量应得单价；
- 验证、传输、存储和平台费用规则；
- 创建时间、报价失效时间和四舍五入规则。

所有价格使用 `i64/u64` 微单位，比例使用 basis points；禁止 `f32/f64`。历史任务永远引用原快照，不因未来曲线变化而重算。

2026-08-04 已形成 v171 不可变 Price Snapshot Registry：合同校验会核验快照与 active Offer、Provider、SKU、交付窗口和价格条款的精确绑定，约束 trade/index/mark/fallback 来源及观察窗口，并用 checked `i128` 检查整数金额上限。Store 可登记和读取完整快照；相同快照 ID 只能精确重放，quote ID 唯一，读取会按历史 Offer 复核摘要与索引字段，数据库触发器拒绝更新和删除。v175 本地 Broker 会复核并锁定 quoted Job 已选择的既有 CNY 快照；Registry 仍没有价格源注册、期货曲线、报价生成或 HTTP。

## 7. 双价格腿与平台价差

一笔任务同时冻结消费者价格腿和 Provider 价格腿：

```text
consumer_charge
  = verified_units × locked_consumer_unit_price
  + transfer_fee + storage_fee + verification_fee

provider_payable
  = compensable_units × locked_provider_unit_price
  + availability_bonus + acceptance_bonus
  - delivery_penalty

platform_margin
  = consumer_charge - provider_payable - third_party_cost
```

`verified_units` 与 `compensable_units` 可以不同。例如结果正确但超过已承诺资源上限时，消费者按验证用量支付，Provider 只能按合同上限获得补偿。任何修正通过追加式纠正回执完成，不覆盖原回执。

## 8. 指数、标记价和成交价

每个 SKU 同时区分：

- `trade_price`：真实订单成交价；
- `index_price`：已接受交付与可执行 Offer 的稳健统计；
- `mark_price`：持仓与风险计算使用的平滑价格；
- `delivery_price`：到期交割生成 Price Snapshot 的最终锁定价。

一龙算力指数（YCI）优先使用真实接受的成交，以容量加权中位数聚合；排除明显离群、自成交、未交付和被验证拒绝的样本。所有排除原因进入可审计指数批次。

## 9. 容量合约

标准合约 `CapacityInstrument` 包含 SKU、合约单位、交付窗口、最小可用率、最小吞吐、允许区域、验证等级和结算单位。

容量市场底层统一使用已接受的共享 CapacityPool 与追加式账本设计，见 `docs/decisions/distributed-compute-capacity-ledger-v1.md` 和 `docs/distributed-compute/capacity-ledger.md`。Offer 只声明静态出售上限；发布、复制或续期 Offer 不会铸造任何可用容量。只有 Pool bucket 的发行事件进入账本后才形成余额，所有现货 Reservation 与未来 Commitment 必须争用同一容量真源。

V1 每份 Reservation 只绑定一个 Pool、一个精确 UTC 半开交付窗口 `[starts_at, ends_at)` 和多个 meter；不在一个 Reservation 内跨 Pool 或跨窗口。领域合同、reducer、v165-v176 SQLite schema、容量 Store、版本化 Provider/Offer/Job/Reservation Registry，以及不可变 Price Snapshot Registry 已写入但未编译、未执行迁移。v175/v176 已形成平台人民币余额的原子 Reserve 和未执行任务退款终态；本人 HTTP 和项目级 MCP 已写但未运行，价格源、报价生成、Attempt、运行中任务和真实用量结算仍未实现。

市场对象分层：

- `Order`：买/卖方向、限价、数量、有效期；
- `Trade`：撮合后的不可变成交；
- `Position`：账户在某合约上的净持仓和已分配数量；
- `CapacityCommitment`：卖方为交付锁定的 Offer 容量；
- `DeliveryAllocation`：把买方持仓转成具体 Job 可消费的配额；
- `ClearingReceipt`：到期交付、差额、奖励与处罚的汇总回执。

同一份卖方容量不能同时支持多个未被净额化的承诺。Commitment 的 Reservation 必须由数据库或等价一致性边界原子完成。

## 10. 从期货价格到任务结算

1. Planner 把任务需求归一为 SKU 与交付窗口；
2. Market 选择用户已有 Delivery Allocation，或从期货曲线生成可接受快照；
3. Broker 原子冻结消费者预算、所选 Pool/交付窗口的容量和 Price Snapshot；
4. Attempt 执行并产生声明/观测/验证回执；
5. Verification 决定 `verified_usage` 与 `compensable_usage`；
6. Settlement 依据快照生成双价格腿和平台价差；
7. 资金/积分先进入 `pending`，过验证窗口后进入 `available`；争议进入 `disputed`；
8. 到期合约由 Clearing Receipt 汇总交付与差额。

Provider 收益不能在收到节点自报终态时立即成为可提取余额。

## 11. 失败、重试和交付

- Reservation 未派发即到期：释放预算和容量，不产生执行用量；
- Attempt 失败但 Job 重试成功：按合同分别记录失败成本与成功交付，不重复向消费者计算目标用量；
- Provider 未达到容量承诺：从卖方保证资源或待结算收益中计算交付差额，并由 Broker 买入替代容量；
- 需求方未消费已购买容量：按合约规则过期或进入允许的转售窗口，不回写历史成交；
- 验证争议：冻结相关 Provider 应得和合约交割，追加纠正 Receipt 后再清算。

## 12. 演进顺序

1. 版本化 Price Snapshot 注册、持久化与整数微单位（代码已写，尚未验证接线）；
2. 平台发布的价格源、期货曲线、报价生成和固定交付窗口；
3. Provider Capacity Commitment 与需求方 Delivery Allocation；
4. 限价订单簿、成交、持仓和净额；
5. YCI 指数、标记价、替代交付和自动清算；
6. 跨公司、跨矿池的统一容量市场。

前一步生成的 Job、Receipt 和 Snapshot 必须能被后一步继续读取，不能为了市场升级重写历史账本。

## 13. 当前未验证声明

本文是已接受的目标市场合同。当前代码已写入 v175 平台人民币余额预授权、v176 未执行任务严格退款，以及本人 HTTP 和项目级 MCP 控制面，但均为 `implementation_uncompiled`，尚未执行迁移或运行验证。价格源、报价生成、Attempt、验证用量、运行中结算、期货曲线、订单簿、持仓和真实清算仍未实现；现有代码不执行生产资金或积分移动。
