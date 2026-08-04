---
title: 分布式算力 Price Snapshot 控制面
status: current
reviewed_at: 2026-08-05
owners: backend, ai-economy
implementation_status: implementation_uncompiled
---

# 分布式算力 Price Snapshot 控制面

## 1. 当前状态

本人 HTTP、项目级 MCP 和 PC 工作区已写入代码，可基于当前 active Offer 发布或读取一份服务端规范化的不可变 Price Snapshot。代码尚未编译、执行 v171 迁移、运行 HTTP/MCP 验证或发布页面，状态固定为 `implementation_uncompiled`。

首版来源固定为 `fallback_curve`：它冻结 Offer 已声明的价格组件和费用规则，使现有 Job 候选查询可以发现该报价，但不代表真实成交价、指数价格、市场 mark 或期货曲线。

## 2. 接口

| 类型 | 名称或路径 | 作用 |
|---|---|---|
| HTTP POST | `/api/me/compute/providers/:provider_id/capacity-pools/:pool_id/offers/:offer_id/price-snapshots` | 显式确认后发布报价快照 |
| HTTP GET | `/api/me/compute/providers/:provider_id/capacity-pools/:pool_id/offers/:offer_id/price-snapshots?limit=20` | 按报价时间倒序列出 1 至 100 份快照 |
| HTTP GET | `/api/me/compute/providers/:provider_id/capacity-pools/:pool_id/offers/:offer_id/price-snapshots/:snapshot_id` | 所有者读取并审计快照 |
| MCP 写入 | `compute_publish_my_price_snapshot` | 发布规范化 fallback_curve 快照 |
| MCP 只读 | `compute_get_my_price_snapshot` | 读取本人 Offer 的快照 |
| MCP 只读 | `compute_list_my_price_snapshots` | 按稳定顺序列出本人 Offer 的快照 |

接口要求一龙用户会话，并复用 Offer 所有权检查。项目成员身份不能越权发布他人的报价。

### PC 入口

本人在 `/compute-supply` 的 Offer 详情中读取报价历史；只有当前 active Offer 显示发布入口。表单要求选择 Offer 自带的交付窗口，以币种金额填写消费者和 Provider 上限，并在本地精确转换为整数微单位；TTL、舍入方式、当前 Offer 版本与摘要、单次稳定幂等键和人工确认一并提交。返回结果展示来源、双方金额上限、失效时间和不可变摘要。

页面只提供本人控制面，不代替服务端重新核验所有权、active 状态、窗口、金额关系和有效期。关闭表单后再次创建会得到新的幂等键；同一次打开和重试保留原键，避免网络重试生成重复快照。

## 3. 请求与服务端固定字段

请求提供：

- 当前 Offer 的精确版本与摘要；
- Offer 内一个交付窗口 ID；
- 消费者和 Provider 最大整数微单位金额；
- 30 至 3600 秒 TTL；
- `half_up`、`half_even`、`floor` 或 `ceil` 舍入方式；
- 幂等键与 `confirm_publish=true`。

服务端固定并重新审计：

- 当前 Offer 必须为 active，且版本、摘要、Provider、Pool 和所有者一致；
- SKU、价格模式、币种、价格组件、费用规则和 instrument 引用来自 Offer；
- 交付窗口必须属于 Offer；
- Snapshot ID 与 Quote ID 由所有者、Provider、Pool、Offer 和幂等键确定性生成；
- 来源固定为 `fallback_curve`，来源版本绑定 Offer 版本，样本数固定为 0；
- 报价时间由服务端生成，失效时间不晚于请求 TTL、Offer 有效期或价格条款有效期；
- Snapshot 摘要和来源摘要由服务端生成。

相同幂等键重放时，会重新核对 Offer、窗口、金额、TTL、舍入方式、来源摘要和最终失效时间。任何字段变化都会被拒绝。列表按 `quoted_at DESC, snapshot_id ASC` 稳定排序，并逐份复用历史审计读取，不直接信任索引行。

## 4. 市场与资金边界

成功响应明确返回：

- `market_effect: "quote_candidate_enabled"`；
- `reservation_effect: "none"`；
- `capacity_effect: "none"`；
- `funds_effect: "none"`。

快照写入后，现有候选查询仍会要求当前 Offer 和 Provider 均为 active、快照未过期、Job 合同与预算匹配。发布快照本身不会创建 Job、选择候选、预留 Capacity Claim、冻结余额、创建 Reservation、派发 Attempt 或自动成交。

当 Offer 转为 draining、expired 或 revoked 后，候选查询因当前 Offer 不再 active 而排除其历史快照；快照本身仍作为不可变历史保留。

## 5. 尚未实现

- Cargo 编译、v171 迁移执行、并发幂等和 HTTP/MCP 真实调用验证；
- 平台签名价格源、trade/index/mark 接入与来源治理；
- 期货/远期曲线、订单簿、撮合、滑点和行情广播；
- 自动刷新、批量报价和到期调度；
- 非人民币 Broker、多币种结算、Sui 资产和真实资金清算。
