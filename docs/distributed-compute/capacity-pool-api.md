---
title: 分布式算力 CapacityPool 本人控制面
status: current
reviewed_at: 2026-08-04
owners: backend, node, ai-economy
implementation_status: implementation_uncompiled
---

# 分布式算力 CapacityPool 本人控制面

## 1. 当前状态

本人 CapacityPool 控制面已写入代码，但尚未编译、执行 v165 迁移或运行 HTTP/MCP 验证，状态固定为 `implementation_uncompiled`。它允许用户在本人 `user_node` 或 `managed_cluster` Provider 下登记、读取、列出和审计共享物理资源边界，但只创建 `registering` Pool，不激活 Provider、不发行容量、不创建交付窗口、Bucket、Offer 或 Price Snapshot。

HTTP 与开放商业 MCP 共用 `compute_federation_capacity_pool_service`，最终写入既有 CapacityPool Registry。服务端固定初始 `capacity_epoch=1`、`pool_revision=1`、状态、时间和全部摘要，客户端不能直接提交摘要或生命周期状态。

## 2. HTTP 接口

全部接口要求一龙用户 Bearer 会话，且路径中的 Provider 必须属于当前登录用户。

| 方法 | 路径 | 作用 |
|---|---|---|
| POST | `/api/me/compute/providers/:provider_id/capacity-pools` | 在本人 Provider 下登记一份 registering Pool |
| GET | `/api/me/compute/providers/:provider_id/capacity-pools?limit=20` | 列出该 Provider 的 Pool 脱敏视图 |
| GET | `/api/me/compute/providers/:provider_id/capacity-pools/:pool_id` | 读取该 Provider 的一份 Pool 脱敏视图 |
| GET | `/api/me/compute/providers/:provider_id/capacity-pools/:pool_id/audit` | 重算当前 epoch 账本并返回一致性报告 |
| GET | `/api/me/compute/providers/:provider_id/capacity-pools/:pool_id/ledger-transactions?limit=20&before_sequence=...` | 分页读取当前 epoch 的脱敏账本历史 |

创建请求提供稳定 `pool_id`、仅用于生成摘要的 `resource_scope_key`、区域、32 KiB 以内的 JSON 资源档案和 1 至 64 项计量策略。计量策略明确 meter 名称、`consumable/reusable` 模式和正整数最小量子。

## 3. MCP 工具

这些工具加入项目级开放商业 MCP：`/api/projects/:project_id/open-commerce/mcp`。Pool 与 Provider 仍归登录用户所有，项目成员身份不能越权读取其他成员的供给资源。

| 工具 | 类型 | 作用 |
|---|---|---|
| `compute_create_my_capacity_pool` | 幂等写入 | 在本人 Provider 下登记 registering Pool |
| `compute_get_my_capacity_pool` | 只读 | 读取本人一份 Pool 脱敏视图 |
| `compute_list_my_capacity_pools` | 只读 | 列出本人 Provider 的 Pool |
| `compute_audit_my_capacity_pool` | 只读 | 重算本人当前 Pool epoch 的账本并返回一致性报告 |
| `compute_list_my_capacity_ledger_transactions` | 只读 | 分页列出本人当前 Pool epoch 的脱敏事务与双分录 |

## 4. 摘要与隐私边界

服务端对资源范围密钥、资源档案、每项 meter 策略和完整 Pool 合同分别生成 SHA-256 摘要，并按 meter 名称排序后保存规范合同。读取时重新计算资源档案摘要，发现存储内容与摘要不一致即失败关闭。

响应只返回 Pool/Provider ID、状态、epoch、revision、合同摘要、区域、计量策略和创建时间。它不返回 `resource_scope_key` 或原始 `resource_profile`，因此 AI 和普通客户端只能知道资源合同的摘要及可计量能力，不能借查询接口获得底层硬件明细。

相同 `pool_id` 只能重放同一 Provider、资源范围、档案、区域和 meter 合同；同一 Provider 的相同资源范围也只能绑定一个 Pool。已有 Pool 属于另一 Provider 或合同不同即拒绝。

## 5. 账本审计边界

审计入口只读取当前 Pool 的 `capacity_epoch`，从不可变 LedgerTransaction/Leg 重算每个 Bucket 的 issued、available、held、active、consumed 和 retired 余额，再与物化投影、revision 和 ledger sequence 对比。报告返回交易数、分录数、Bucket 差异和总体 `healthy` 状态，不修改 Pool、账本或余额。

`healthy=true` 只表示账本分录、守恒关系与当前投影在内部一致，不证明硬件真实存在、节点在线、性能达到声明、路由可达或平台已经完成 verified 审核。该报告可作为未来激活审核的材料之一，但不能单独触发激活或 Offer 发布。

账本历史入口按 `ledger_sequence` 倒序返回事务 ID/摘要、事件类型、交付窗口、服务端时间和原始双分录，并使用 `next_before_sequence` 继续翻页。它省略 subject、消费者、Claim、Offer、Job、Reservation、Attempt、幂等键和请求摘要，避免供给者通过审计接口获得消费者或内部业务关系。历史查询同样只读，不构成硬件验证、收益结算或链上证明。

## 6. 失败关闭边界

- Provider 不属于当前用户时拒绝；
- `external_pool` 必须由服务端 Adapter 管理，本人接口拒绝；
- Provider 不是 `registering` 或 `active` 时拒绝新增 Pool；
- Pool 始终从 `registering`、epoch 1、revision 1 创建；
- 客户端不能提交状态、摘要、发行数量、余额或服务端时间；
- 资源档案不是对象或超过 32 KiB、meter 重复、模式不受支持或量子非正数时拒绝；
- 查询到的 Pool 不属于路径 Provider 时拒绝。

## 7. 尚未实现

- Cargo 编译、v165 迁移执行和 HTTP/MCP 真实调用验证；
- Pool 版本更新、epoch 轮换的本人控制面；
- CapacityBucket 与 Supply Add/Withdraw 控制面已写入，边界见 `docs/distributed-compute/capacity-bucket-api.md` 和 `docs/distributed-compute/capacity-supply-api.md`；
- Provider/Pool 激活证据申请与人工审核控制面已写，见 `docs/distributed-compute/activation-evidence-api.md`；真实观测验证和 approved 后受控激活仍未实现；
- Offer、Price Snapshot、自动撮合和真实任务派发；
- 实际用量验证、Provider 收益和链上结算。
