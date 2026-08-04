---
title: 分布式算力 CapacityBucket 本人控制面
status: current
reviewed_at: 2026-08-04
owners: backend, node, ai-economy
implementation_status: implementation_uncompiled
---

# 分布式算力 CapacityBucket 本人控制面

## 1. 当前状态

本人 CapacityBucket 控制面已写入代码，但尚未编译、执行 v165 迁移或运行 HTTP/MCP 验证，状态固定为 `implementation_uncompiled`。它允许用户在本人 Provider 的当前 CapacityPool 版本下创建 open、零发行余额的交付窗口 Bucket，并读取当前账本余额；它不发行容量、不激活 Pool、不创建 Offer，也不允许消费者预留。

HTTP 与开放商业 MCP 共用 `compute_federation_capacity_bucket_service`，最终调用现有 `create_compute_capacity_bucket` 和只读 Store。分布式算力 MCP 已由 `compute_federation_mcp` 统一聚合，新增算力工具不再持续扩大 `open_commerce_mcp` 入口。

## 2. HTTP 接口

全部接口要求一龙用户 Bearer 会话，Provider 和 Pool 必须属于当前登录用户，Bucket 必须精确绑定该 Pool 的当前 epoch/revision。

| 方法 | 路径 | 作用 |
|---|---|---|
| POST | `/api/me/compute/providers/:provider_id/capacity-pools/:pool_id/buckets` | 创建一份 open、零余额 Bucket |
| GET | `/api/me/compute/providers/:provider_id/capacity-pools/:pool_id/buckets?limit=20` | 列出当前 Pool 版本的 Bucket 和余额 |
| GET | `/api/me/compute/providers/:provider_id/capacity-pools/:pool_id/buckets/:bucket_id` | 读取一份 Bucket 和余额 |

创建请求提供稳定 `bucket_id`、`window_id`、UTC 开始/结束时间及 Pool 已声明的 meter。Bucket 状态、初始发行量、窗口摘要、Bucket 摘要和创建时间由服务端固定。

## 3. MCP 工具

这些工具通过项目级开放商业 MCP 暴露，但资源归属仍以登录用户为准。

| 工具 | 类型 | 作用 |
|---|---|---|
| `compute_create_my_capacity_bucket` | 幂等写入 | 创建本人 open、零余额 Bucket |
| `compute_get_my_capacity_bucket` | 只读 | 读取本人一份 Bucket 和当前余额 |
| `compute_list_my_capacity_buckets` | 只读 | 列出本人当前 Pool 版本的 Bucket |

响应包含精确 Pool、窗口和 meter 绑定，以及 issued、available、held、active、consumed、retired、余额 revision 和已处理账本序号。这些数值只是当前账本投影，不代表 Provider 已通过验证或消费者已经可以购买。

## 4. 窗口与重放不变量

- 时间使用 UTC RFC3339 半开区间，结束必须晚于开始；
- 首次创建时窗口不能已经结束，但原样幂等重放在窗口结束后仍可读取原合同；
- 同一 Pool/epoch 的同一 `window_id` 必须始终绑定相同摘要和时间；
- 同一 Pool/epoch/meter 的未退役窗口不能重叠；
- meter、模式、量子和策略摘要必须来自当前 Pool 版本；
- 相同 `bucket_id` 只能重放同一 Pool、窗口和 meter 合同；
- 列表只返回当前 Pool revision，旧 revision 留给未来历史审计接口。

## 5. 尚未实现

- Cargo 编译、v165 迁移执行和 HTTP/MCP 真实调用验证；
- Supply Add/Withdraw 本人控制面；
- Bucket 关闭、退役和窗口批量管理；
- Provider/Pool 验证与受控激活；
- Offer、Price Snapshot、撮合、预留和任务派发；
- 实际用量验证、收益结算和链上投影。
