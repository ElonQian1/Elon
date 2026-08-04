---
title: 分布式算力 Provider 本人控制面
status: current
reviewed_at: 2026-08-04
owners: backend, node, ai-economy
implementation_status: implementation_uncompiled
---

# 分布式算力 Provider 本人控制面

## 1. 当前状态

本人 Provider 控制面已写入代码，但尚未编译、执行迁移或运行 HTTP/MCP 验证，状态固定为 `implementation_uncompiled`。它只允许登录用户登记、读取和列出本人拥有的自我声明 Provider，不代表节点已经联网、通过验证、具备可调度路由或能够发布算力 Offer。

HTTP 与开放商业 MCP 共用 `compute_federation_provider_service`，最终写入同一份版本化 Provider Registry。创建时，所有者、结算账户、初始状态、信任层级、策略修订号和服务端时间均由服务端固定，客户端不能自行提升信任或激活供给。

## 2. HTTP 接口

全部接口要求一龙用户 Bearer 会话，只能读取当前登录用户自己的 Provider。

| 方法 | 路径 | 作用 |
|---|---|---|
| POST | `/api/me/compute/providers` | 登记本人 `user_node` 或 `managed_cluster` Provider |
| GET | `/api/me/compute/providers?limit=20` | 列出本人 Provider 的脱敏视图 |
| GET | `/api/me/compute/providers/:provider_id` | 读取本人一份 Provider 的脱敏视图 |

创建请求可以声明稳定 `provider_id`、类型、显示名称、区域、任务类型、加速器类型、允许的数据分类、流式与检查点能力，以及可选硬件摘要。本人接口不接受 `external_pool`；该类型必须由服务端适配器管理。

## 3. MCP 工具

这些工具加入现有项目级开放商业 MCP：`/api/projects/:project_id/open-commerce/mcp`。Provider 归属仍按登录用户判断，不能借项目成员身份读取其他成员的供给记录。

| 工具 | 类型 | 作用 |
|---|---|---|
| `compute_create_my_provider` | 幂等写入 | 登记本人 self-declared、registering Provider |
| `compute_get_my_provider` | 只读 | 读取本人一份 Provider 脱敏视图 |
| `compute_list_my_providers` | 只读 | 列出本人 Provider 脱敏视图 |

## 4. 服务端固定的信任边界

新 Provider 一律以以下状态创建：

- `owner_account_id` 与 `settlement_account_id` 固定为当前登录用户；
- `status` 固定为 `registering`；
- `trust_tier` 固定为 `self_declared`；
- `policy_revision` 固定从 1 开始；
- `endpoint` 与 `adapter` 固定为空；
- observed/verified 硬件摘要及观测、验证时间固定为空；
- 创建和更新时间由服务端生成。

因此，用户提交的硬件摘要只是声明，不是平台观测证据或可信验证结果。Provider 只有经过后续独立的节点绑定、路由证明、证据观测、平台验证和激活流程，才可能进入可调度供给面。

## 5. 脱敏响应与幂等规则

本人接口返回 Provider ID、类型、名称、状态、信任层级、区域、策略修订、能力、证据概要、是否存在路由、规范摘要和是否为重放。它不返回路由地址、凭据引用、适配器配置或内部结算账户。

相同 `provider_id` 可以重放相同声明；如果类型、名称、区域、能力或声明硬件摘要不同，服务端拒绝把同一 ID 改绑到另一份供给声明。已存在 Provider 不属于当前用户时同样拒绝。

## 6. 尚未实现

- Cargo 编译、迁移执行和 HTTP/MCP 真实调用验证；
- 真实 PC 节点、企业集群或外部矿池与 Provider 的绑定；
- Endpoint/Adapter 路由提案、证明、审批、轮换和撤销；
- 节点绑定、ReadyCapability、路由与硬件观测摘要的申请及人工审核控制面已写，见 `docs/distributed-compute/activation-evidence-api.md`；真实证据采集、密码学验证、信任升级和激活仍未实现；
- Offer active 审批与发布、Price Snapshot 生成和报价暴露；本人 CapacityPool、供给账本和 draft Offer 控制面已另行写入代码，但仍为 `implementation_uncompiled`；
- 节点在线状态、动态容量、任务派发、用量验证和收益结算。
