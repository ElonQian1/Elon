---
title: 分布式算力激活控制面定向验收
status: current
reviewed_at: 2026-08-11
owners: backend, node, ai-economy
implementation_status: implementation_partially_verified
---

# 分布式算力激活控制面定向验收

## 验收结论

2026-08-11，激活控制面已通过一项基于真实 `Store::open` 临时 SQLite 的定向 Rust 状态链测试。该测试实际执行数据库全量迁移，并调用生产 Store/Service 代码，不使用伪造 Store 或跳过事务。

通过命令：

```text
scripts/validate-rust.ps1 -- test --manifest-path server/Cargo.toml --bin elon-server compute_federation_activation_service::control_plane_tests --locked
```

结果：1 项通过，0 项失败；验证指纹为 `8784595196ddae2a2b4a63f416f901cc379c58dc18f2c933f05694431bc2454b`。

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

## 未验证边界

本次没有验证真实 HTTP/MCP 请求、Bearer 与管理员角色隔离、并发竞争、真实节点证据采集、TCP 路由、生产磁盘升级、浏览器交互、部署或发布。PC `/compute-supply` 与 `/compute-activation` 仅已有严格类型、lint 和生产构建证据。`implementation_partially_verified` 不能解释为真实算力节点已上线，也不能解释为 Offer、任务派发或资金结算已发生。

权威合同继续由 `activation-evidence-api.md` 与 `activation-recovery-api.md` 维护；本页只记录验证证据，不重复定义状态机。
