---
title: 分布式算力 Offer 本人草稿控制面
status: current
reviewed_at: 2026-08-04
owners: backend, ai-economy
implementation_status: implementation_uncompiled
---

# 分布式算力 Offer 本人草稿控制面

## 1. 当前状态

本人 Offer 草稿控制面已写入代码，但尚未编译、执行迁移或运行 HTTP/MCP 验证，状态固定为 `implementation_uncompiled`。它只为当前用户已激活的 Provider 和 CapacityPool 创建服务端规范化 `draft` Offer，并允许所有者按精确版本撤销未发布草稿；它不是 Offer 发布、报价生成或算力交易。

HTTP 与开放商业 MCP 共用 `compute_federation_offer_service`，最终写入已有 v170 版本化 Offer Registry。服务端从 Provider、Pool 和 Bucket 当前版本生成规范合同、SKU 摘要与 Offer 摘要，调用方不能自行声称 active 状态或改写供给身份。

## 2. HTTP 接口

全部接口要求一龙用户 Bearer 会话，并核对 Provider 属于当前登录用户。

| 方法 | 路径 | 作用 |
|---|---|---|
| POST | `/api/me/compute/providers/:provider_id/capacity-pools/:pool_id/offers` | 创建或幂等重放一份规范化 draft Offer |
| GET | `/api/me/compute/providers/:provider_id/capacity-pools/:pool_id/offers?limit=20` | 列出该 Provider/Pool 下的 Offer |
| GET | `/api/me/compute/providers/:provider_id/capacity-pools/:pool_id/offers/:offer_id` | 读取一份 Offer 并重新审计当前投影和历史版本 |
| POST | `/api/me/compute/providers/:provider_id/capacity-pools/:pool_id/offers/:offer_id/revoke` | 按精确版本和摘要执行 `draft -> revoked` |

创建请求提供业务意图，包括幂等键、SKU 类别、模型与运行时、资源参数、Bucket 容量、执行限制、授权范围、价格条款与有效期。`confirm_create` 必须为 `true`。

## 3. MCP 工具

工具加入现有项目级开放商业 MCP：`/api/projects/:project_id/open-commerce/mcp`。Provider 归属仍按登录用户判断，项目成员身份不能越权读写他人 Offer。

| 工具 | 类型 | 作用 |
|---|---|---|
| `compute_create_my_offer_draft` | 显式确认的幂等写入 | 创建服务端规范化 draft Offer |
| `compute_get_my_offer` | 只读 | 读取本人 Provider/Pool 下一份 Offer |
| `compute_list_my_offers` | 只读 | 列出本人 Provider/Pool 下的 Offer |
| `compute_revoke_my_offer_draft` | 显式确认的幂等写入 | 仅撤销本人当前 draft Offer |

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

## 6. 市场与资金边界

成功响应显式返回 `market_effect: "none"`。本入口不会：

- 将 Offer 变为 `active` 或加入消费者候选集；
- 生成 Price Snapshot、曲线或可交易工具；
- 创建 Claim/Reservation 或预留容量；
- 冻结余额、移动资金或触发链上动作；
- 派发 Attempt、下发节点命令或证明节点在线。

撤销 draft 只关闭未发布意图，不解释为取消 active 供给、退还 Reservation 或触发赔付。

`draft` 只是可审计的商业与容量意图，不是对消费者的可交易承诺。

## 7. 尚未实现

- Cargo 编译、迁移执行、并发幂等和 HTTP/MCP 真实调用验证；
- draft 修订、撤销、人工审批与 `active` 发布状态机；
- Price Snapshot 生成、报价曲线、自动撮合与候选暴露；
- 容量动态校准、Attempt 派发、用量验证和运行中结算；
- 外部矿池适配器、多币种、Sui 资产和真实提现。
