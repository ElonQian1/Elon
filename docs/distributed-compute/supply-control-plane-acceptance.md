---
title: 分布式算力本人供给控制面验收
status: current
reviewed_at: 2026-08-11
owners: backend, ai-economy
implementation_status: implementation_partially_verified
---

# 分布式算力本人供给控制面验收

## 已验证范围

- 全新磁盘 SQLite 执行当前迁移后，可按真实服务链创建 `user_node` Provider、`registering` Pool，以及同一交付窗口的 consumable/reusable 双 meter Bucket。
- Provider 和 Pool 相同合同重放复用既有记录；同 ID 改换声明或资源合同失败关闭。
- 一次请求可向两个 Bucket 原子追加供给，重放复用同一账本事务；随后可把 `available` 原子撤入 `retired`。
- 未显式确认、数量不满足最小量子、同幂等键更换数量及超出 available 的撤回均失败，失败后账本与余额不产生部分写入。
- 其他用户不能读取供给者 Provider；Pool 审计可从双分录重算出健康余额和两笔事务。
- 关闭并重开同一 SQLite 文件后，Provider、Pool、Bucket 余额和健康审计保持一致。

## 验证证据

- 测试模块：`server/src/compute_federation_supply_control_plane_tests.rs`
- 定向命令：`test --manifest-path server/Cargo.toml --bin elon-server compute_federation_capacity_supply_service::tests --locked`
- 结果：2 项通过，0 项失败。
- 验证指纹：`49dbf7d302089db3dc76cba540f135156e7f5cc8881e8b35a08833347a2d9ef0`

## 未验证边界

- HTTP Bearer 会话、路由参数和响应错误映射尚未通过进程内 Axum 回归。
- 开放商业 MCP 的工具发现、参数解析、项目成员与本人资源双重边界尚未运行验证。
- PC `/my-compute-settlement` 和 `/compute-supply` 尚未执行严格类型检查、生产构建、浏览器交互或发布。
- 本验收没有节点联网、硬件观测、Provider/Pool 激活、Offer 发布、容量预留、Attempt 派发、真实用量、收益结算、外部付款或 Sui 链上资产。
