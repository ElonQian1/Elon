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
- 进程内 Axum 已验证 Bearer 登录门卫、Provider/Pool/Bucket 创建、双 meter 供给追加与幂等重放、审计和账本历史；其他用户不能读取本人资源，未确认追加不会写入余额。
- 分布式算力 MCP 聚合器已验证 Provider、Pool、Bucket、Supply 工具发现及只读/破坏性注解，并可完成追加、重放、撤回和审计；未知工具不被误接管，其他用户不能读取 Provider。

## 验证证据

- 测试模块：`server/src/compute_federation_supply_control_plane_tests.rs`
- 定向命令：`test --manifest-path server/Cargo.toml --bin elon-server compute_federation_capacity_supply_service::tests --locked`
- 结果：2 项通过，0 项失败。
- 验证指纹：`49dbf7d302089db3dc76cba540f135156e7f5cc8881e8b35a08833347a2d9ef0`
- 接口测试模块：`server/src/compute_federation_supply_interface_tests.rs`
- 接口定向命令：`test --manifest-path server/Cargo.toml --bin elon-server compute_federation_mcp::interface_tests --locked`
- 接口结果：3 项通过，0 项失败。
- 接口验证指纹：`ac337fa26cd1ffaf9d845f0f5c1b8ef8179e135adf745ede572cbb290deebd29`

## 未验证边界

- HTTP/MCP 已完成进程内接口验收，但未启动真实 TCP 服务、浏览器或跨进程客户端；通用算力资源仍按登录用户而非项目成员共享。
- PC `/my-compute-settlement` 和 `/compute-supply` 已通过严格类型检查、lint、生产构建和 bundle budget；真实 HTTP、浏览器交互与发布仍未验证，见 `pc-compute-build-acceptance.md`。
- 本验收没有节点联网、硬件观测、Provider/Pool 激活、Offer 发布、容量预留、Attempt 派发、真实用量、收益结算、外部付款或 Sui 链上资产。
