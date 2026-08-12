---
title: 外部矿池 Adapter 采用授权与撤销验收
status: current
reviewed_at: 2026-08-12
owners: backend, security, ai-economy
implementation_status: implementation_partially_verified
---

# 外部矿池 Adapter 采用授权与撤销验收

## V244 定向验证

执行：

```powershell
cargo test --manifest-path server/Cargo.toml --bin elon-server compute_external_pool_adapter_adoption --no-fail-fast
cargo test --manifest-path server/Cargo.toml --bin elon-server adapter_adoption_http_test --no-fail-fast
```

结果：迁移定向测试通过；2 项真实 HTTP 链路测试通过。HTTP 用例通过既有 API 和 Store 流程形成 V221 onboarding、V222 staged admission、V227-V239 制品安全与动态沙箱证据、V241/V242 凭据验证器及 V243 短时签名凭据回执，没有直接伪造 V244 上游记录。

已覆盖：

- 未登录与普通成员拒绝，客户端注入操作人字段拒绝；
- 精确 V239/V243 采用、幂等重放、响应脱敏和 `INSERT OR REPLACE` 防护；
- 当前性返回 `adopted_current`，且 Provider、route、execution、settlement 效果均为 `none`；
- 管理员追加撤销终态后降级为 `historical_only`；
- V242 凭据验证公钥撤销后，未显式撤销的 V244 也因上游失效自动降级为历史。

## 后续基线修复

V244 验收时曾记录 `elon-pc-node` 测试目标的 9 个受管 SQLite registry 私有重导出编译错误。2026-08-12 后续修复只扩大 `cfg(test)` 类型与方法在最小共同测试模块祖先内的可见性，没有改变生产 API。`elon-pc-node` 完整测试目标已通过 `--no-run` 编译，直接受影响的故障矩阵已实际运行并通过 5 项测试。

该修复只清除了仓库编译基线对 V244 验收的干扰，不扩大 V244 能力边界。宽范围 `sqlite_vfs_policy` 运行回归仍暴露 A2b2/A2c 静态库存与进程隔离 runner 的独立失败；它们属于节点 VFS 动态验收缺口，不是 Adapter 已安装或外部算力已可接单的证据。

## 未验证

- 生产数据库原位升级、并发压力、备份恢复、真实 KMS/credential resolver 和真实外部矿池；
- V239/V243 报告自然到期的长时间等待测试；
- 制品安装、Provider 新版本/激活、v213 route、worker/ACK、Runner、真实任务派发、计量、结算和付款；
- 管理 UI、MCP 与生产部署。

因此当前只能表述为“精确外部矿池 Adapter 证据的可撤销采用授权已局部验证”，不能表述为“Adapter 已安装”或“外部算力已可接单”。
