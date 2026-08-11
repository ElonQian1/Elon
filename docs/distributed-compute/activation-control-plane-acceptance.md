---
title: 分布式算力激活控制面定向验收
status: current
reviewed_at: 2026-08-11
owners: backend, node, ai-economy
implementation_status: implementation_partially_verified
---

# 分布式算力激活控制面定向验收

## 验收结论

2026-08-11，激活控制面先通过一项基于真实 `Store::open` 临时 SQLite 的 Store/Service 状态链测试，随后又通过算力 MCP 聚合器接口专项。两组测试均执行生产迁移和生产 Service，不使用伪造 Store 或平行状态机。

通过命令：

```text
scripts/validate-rust.ps1 -- test --manifest-path server/Cargo.toml --bin elon-server compute_federation_activation_service::control_plane_tests --locked
```

结果：1 项通过，0 项失败；验证指纹为 `8784595196ddae2a2b4a63f416f901cc379c58dc18f2c933f05694431bc2454b`。

接口专项命令：

```text
scripts/validate-rust.ps1 -Domain compute-activation-interface -- test --manifest-path server/Cargo.toml --bin elon-server compute_federation_mcp:: -- --nocapture
```

结果为 `CARGO_OK`，验证指纹为 `5761a96c8b3ef0241c63bd2b96e239a2643adeb1fca61bcb1e24d569cd2b395e`。专项验证普通用户看不到且不能直调 22 个管理员激活 MCP 工具，未登录和普通用户不能访问管理员 HTTP；owner MCP、管理员 HTTP/MCP 可在同一 Store 上完成证据提交与批准、计划准备与幂等重放、第二人复核、原子激活、紧急隔离、恢复计划复核与废止重做、原子恢复。关闭并重开文件数据库后，Provider/Pool 当前状态以及恢复应用和废止回执保持一致。

## 已验证状态链

1. 商户本人登记 `registering/self_declared` Provider、CapacityPool 和双 meter Bucket。
2. 商户提交绑定 Provider/Pool 精确版本与健康账本摘要的激活证据申请。
3. 第一名管理员批准证据并准备不可变激活计划。
4. 计划准备人不能复核自己的计划；第二名管理员可按精确计划摘要生成追加式复核回执。
5. 预检通过后，应用事务把 Provider revision 1 推进为 active revision 2、Pool 推进为 active，并把申请和计划写入终态。
6. 紧急隔离事务把 Provider 推进为 quarantined revision 3、Pool 推进为 quarantined，原申请、计划和应用回执保持可审计。
7. 第一份恢复计划经第二人复核后被显式废止；回执返回 `recovery_effect=plan_superseded`，Provider、Pool、节点和资金均无变化。
8. 第二份恢复计划不会继承旧复核；复核前预检稳定返回 `plan_review_missing`。
9. 第二份计划经独立复核后原子恢复 Provider active revision 4 和 Pool active；恢复不发布 Offer、不发节点命令、不移动资金。

## 已执行门禁

- `git diff --check`；
- 源码文件规模检查；
- 全仓库 Rust 格式检查；
- `elon-server` 测试二进制编译和链接；
- 临时 SQLite 全量迁移；
- V177-V181、V203-V205 相关 Store/Service 状态和回执审计。
- 本人 5 个 MCP 工具、管理员 22 个 MCP 工具和 18 条 HTTP 路由的共享 Service 合同；
- Bearer 未登录、普通用户、平台 `admin/owner` 的角色隔离，以及写操作显式确认和摘要门卫；
- 文件数据库关闭重开后的当前投影和追加式回执读取。

## 未验证边界

本次验证的是进程内 HTTP/MCP，不是监听端口的真实 TCP。尚未验证并发压力、异常断电、生产数据库副本升级、真实节点证据采集、TCP 路由、浏览器交互、部署或发布。PC `/compute-supply` 与 `/compute-activation` 仅已有严格类型、lint 和生产构建证据。`implementation_partially_verified` 不能解释为真实算力节点已上线，也不能解释为 Offer、任务派发或资金结算已发生。

权威合同继续由 `activation-evidence-api.md` 与 `activation-recovery-api.md` 维护；本页只记录验证证据，不重复定义状态机。
