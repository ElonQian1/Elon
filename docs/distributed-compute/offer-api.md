---
title: 分布式算力 Offer 草稿、发布与安全退场控制面
status: current
reviewed_at: 2026-08-05
owners: backend, ai-economy
implementation_status: implementation_uncompiled
---

# 分布式算力 Offer 草稿、发布与安全退场控制面

## 1. 当前状态

本控制面及对应 PC 工作区已写入代码，但尚未编译、执行 v182-v184 迁移、运行 HTTP/MCP 验证或发布，状态固定为 `implementation_uncompiled`。本人可创建、连续修订或撤销规范化 `draft` Offer；平台管理员可原子发布、将 active Offer 转为 draining，并在依赖清理后转入 expired 或 revoked 终态。

HTTP 与开放商业 MCP 共用 `compute_federation_offer_service`，最终写入已有 v170 版本化 Offer Registry。服务端从 Provider、Pool 和 Bucket 当前版本生成规范合同、SKU 摘要与 Offer 摘要，调用方不能自行声称 active 状态或改写供给身份。

## 2. HTTP 接口

全部接口要求一龙用户 Bearer 会话，并核对 Provider 属于当前登录用户。

| 方法 | 路径 | 作用 |
|---|---|---|
| POST | `/api/me/compute/providers/:provider_id/capacity-pools/:pool_id/offers` | 创建或幂等重放一份规范化 draft Offer |
| GET | `/api/me/compute/providers/:provider_id/capacity-pools/:pool_id/offers?limit=20` | 列出该 Provider/Pool 下的 Offer |
| GET | `/api/me/compute/providers/:provider_id/capacity-pools/:pool_id/offers/:offer_id` | 读取一份 Offer 并重新审计当前投影和历史版本 |
| POST | `/api/me/compute/providers/:provider_id/capacity-pools/:pool_id/offers/:offer_id/revoke` | 按精确版本和摘要执行 `draft -> revoked` |
| POST | `/api/me/compute/providers/:provider_id/capacity-pools/:pool_id/offers/:offer_id/revise` | 按精确版本和摘要追加 draft 下一版本 |
| GET | `/api/me/compute/providers/:provider_id/capacity-pools/:pool_id/offers/:offer_id/publication` | 所有者读取该 Offer 的发布回执 |
| GET | `/api/me/compute/providers/:provider_id/capacity-pools/:pool_id/offers/:offer_id/drain` | 所有者读取该 Offer 的安全退场回执 |
| GET | `/api/me/compute/providers/:provider_id/capacity-pools/:pool_id/offers/:offer_id/expiration` | 所有者读取 expired 回执 |
| GET | `/api/me/compute/providers/:provider_id/capacity-pools/:pool_id/offers/:offer_id/revoke` | 所有者读取 active Offer 的 revoked 回执；同路径 POST 仍只撤销 draft |
| GET | `/api/admin/compute/offers?limit=20` | 管理员列出当前 draft 待审队列 |
| GET | `/api/admin/compute/offers/:offer_id` | 管理员读取待审 Offer 完整合同 |
| GET/POST | `/api/admin/compute/offers/:offer_id/publication` | 管理员读取回执，或显式确认后原子发布 |
| GET/POST | `/api/admin/compute/offers/:offer_id/drain` | 管理员读取回执，或显式确认后执行 `active -> draining` |
| GET/POST | `/api/admin/compute/offers/:offer_id/expire` | 管理员读取回执，或对已到期且无活动预留的 draining Offer 执行 `expired` |
| GET/POST | `/api/admin/compute/offers/:offer_id/revoke` | 管理员读取回执，或对无活动预留的 draining Offer执行 `revoked` |

## 2.1 PC 工作区

本人在 `/compute-supply` 的 Pool 详情中管理 Offer 草稿：页面列出当前 Pool 的合同，可从 open Bucket 创建常用 `spot/CNY` 草稿，并按当前版本和摘要修订或撤销。页面不会替用户发布 Offer，也不会生成 Price Snapshot、预留容量或移动资金。

仅 `admin/owner` 可见的 `/compute-offers` 提供待发布草稿队列和按 Offer ID 精确打开入口。管理员可在核对当前版本、摘要和依赖边界后显式确认发布，或将 active Offer 转为 draining，再在服务端依赖检查通过后终结为 expired/revoked；页面展示本次写入返回的不可变回执摘要。所有操作仍由服务端合同判定，前端权限隐藏不替代后端授权。

创建请求提供业务意图，包括幂等键、SKU 类别、模型与运行时、资源参数、Bucket 容量、执行限制、授权范围、价格条款与有效期。`confirm_create` 必须为 `true`。

## 3. MCP 工具

工具加入现有项目级开放商业 MCP：`/api/projects/:project_id/open-commerce/mcp`。Provider 归属仍按登录用户判断，项目成员身份不能越权读写他人 Offer。

管理员发布和退场不向 MCP 开放。AI 可协助准备和审阅草稿，但不能代替平台 `admin/owner` 通过 HTTP 执行最终状态变更。

| 工具 | 类型 | 作用 |
|---|---|---|
| `compute_create_my_offer_draft` | 显式确认的幂等写入 | 创建服务端规范化 draft Offer |
| `compute_get_my_offer` | 只读 | 读取本人 Provider/Pool 下一份 Offer |
| `compute_list_my_offers` | 只读 | 列出本人 Provider/Pool 下的 Offer |
| `compute_revoke_my_offer_draft` | 显式确认的幂等写入 | 仅撤销本人当前 draft Offer |
| `compute_revise_my_offer_draft` | 显式确认的幂等写入 | 完整替换本人当前 draft 合同并追加下一版本 |

## 4. 创建前置条件

服务端同时要求：

- Provider 属于当前用户且当前状态为 `active`；
- Provider 具有 Endpoint 或 Adapter 路由；
- Provider 具有 verified 硬件摘要和最后验证时间，且信任等级不得为 `self_declared`；
- CapacityPool 属于同一用户和 Provider，当前状态为 `active`；
- 每个 Bucket 属于当前 Pool 精确版本，状态为 `open`；
- 容量覆盖每个交付窗口与 meter 的完整矩阵，数量和价格粒度符合已有合同校验。

这些条件使 registering、quarantined 或纯自声明供给无法通过本人入口创建新草稿。历史 Offer 仍可由所有者读取，不因 Provider 后续隔离而丢失审计记录。

## 5. 服务端固定字段

创建时以下内容不由客户端决定：

- `offer_id` 由当前用户、Provider、Pool 和幂等键确定性生成；
- `offer_version` 固定从 1 开始，`status` 固定为 `draft`；
- Provider ID、类型和资源档案摘要来自当前 Provider/Pool；
- Pool 精确版本、Bucket 绑定、交付窗口、区域和 meter 集合来自已审计 Store；
- 授权策略修订号固定从 1 开始；
- SKU 摘要、Offer 摘要和首次创建时间由服务端生成。

同一幂等键只能重放同一份规范合同。如果重放请求的业务字段变化，服务端拒绝把稳定 Offer ID 重绑到另一份合同。

撤销要求 `expected_offer_version`、`expected_offer_digest` 和 `confirm_revoke=true`。服务端只允许当前 `draft -> revoked`，以连续下一版本保留不可变历史；对同一前置版本的重放会重新审计撤销前的 draft 摘要。本入口明确拒绝 active、draining、expired 或已被其他合同终结的 Offer。

修订要求完整提交替换合同、`expected_offer_version`、`expected_offer_digest` 和 `confirm_revise=true`。服务端重新解析当前 active Provider/Pool/Bucket，追加连续 draft 版本并递增授权策略修订号；重放必须与预期历史和已写入合同完全一致。Offer ID、Provider、Pool、SKU ID 与 SKU 摘要属于稳定身份，不能原地改变；如需改变任务类型、模型/运行时身份、区域或 meter 集合，应创建新的 Offer。修订不会发布、生成 Price Snapshot、预留容量或移动资金。

## 6. 原子发布与回执

管理员发布请求必须提供 `expected_offer_version`、`expected_offer_digest`、`idempotency_key` 和 `confirm_publish=true`。写入在一个 `BEGIN IMMEDIATE` 事务中完成：

1. 重新审计当前 draft 版本和摘要。
2. 复用 Offer Registry 核验 active Provider、active Pool、open Bucket、已发行容量、合同有效期与状态转换。
3. 追加连续下一版本的 active Offer，不覆盖 draft 历史。
4. 写入 v182 `compute_offer_publications` 不可变回执，绑定草稿/活动版本、Provider 策略版本、审批人、时间与规范摘要。

任一步失败时，active 版本和发布回执整体回滚。相同幂等键或已发布 Offer 只能重放同一 draft 合同；回执后续读取只审计不可变历史，不要求 Offer 永远停留在 active。

## 7. 安全退场与回执

管理员退场请求必须提供当前 active Offer 的 `expected_offer_version`、`expected_offer_digest`、非空原因、`idempotency_key` 和 `confirm_drain=true`。v183 在一个 `BEGIN IMMEDIATE` 事务中追加连续下一版本的 draining Offer，并保存 `compute_offer_lifecycle_events` 不可变回执；回执绑定退场前后版本、摘要、原因、执行人和时间。

draining 状态用于停止接受新的报价和预留。现有候选查询只接受当前 active Offer，因此转换成功后该 Offer 不再进入新候选；已有 Reservation、Claim、Attempt 和余额均不在本事务中修改。该边界避免把“停止新增业务”误解为“强制取消已有履约”。所有者只有只读权限，管理员写入口不向 MCP 开放。

终态请求使用同样的精确版本、摘要、原因、幂等键和 `confirm_terminal=true` 约束，只接受当前 draining Offer。v184 为 Reservation 依赖检查增加查询索引；只要任意历史版本仍关联 `pending` 或 `active` Reservation，整个终态事务就失败。`expired` 还要求当前时间不早于 `valid_until`，提前退出只能选择 `revoked`。转换只追加 Offer 下一版本和 v183 生命周期回执，不替调用方取消预留、归还 Claim 或退款。

## 8. 市场与资金边界

草稿创建、读取和撤销响应返回 `market_effect: "none"`。发布回执返回 `offer_effect: "active"`，同时明确返回 Price Snapshot、容量移动和资金效果均为 `none`。

active Offer 只是生成报价的前置合同。现有候选查询从未过期 Price Snapshot 出发，并要求快照精确绑定当前 active Offer 和 active Provider；因此只发布 Offer 不会自动出现在消费者可锁价候选中。draining 回执返回 `quote_candidate_effect: "excluded_from_new_quotes"`、`reservation_effect: "preserved"`、`attempt_effect: "none_direct"` 和 `funds_effect: "none"`。整个控制面不会：

- 生成 Price Snapshot、曲线或可交易工具；
- 创建 Claim/Reservation 或预留容量；
- 冻结余额、移动资金或触发链上动作；
- 派发 Attempt、下发节点命令或证明节点在线。

撤销 draft 只关闭未发布意图，不解释为取消 active 供给、退还 Reservation 或触发赔付。

`draft` 只是可审计的商业与容量意图，不是对消费者的可交易承诺。

## 9. 尚未实现

- Cargo 编译、迁移执行、并发幂等和 HTTP/MCP 真实调用验证；
- 自动终态调度，以及已有 Reservation 的自动取消、退款和 Claim 归还；
- 真实价格源、期货曲线、批量报价和自动撮合；fallback_curve Price Snapshot 见 `price-snapshot-api.md`；
- 容量动态校准、Attempt 派发、用量验证和运行中结算；
- 外部矿池适配器、多币种、Sui 资产和真实提现。
