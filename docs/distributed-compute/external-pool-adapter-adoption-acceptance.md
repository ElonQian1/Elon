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

## 基线异常

未限定二进制的 Cargo 测试仍会编译仓库现有 `elon-pc-node` 测试目标，该目标存在 9 个与 V244 无关的受管 SQLite registry 测试类型私有重导出错误。限定 `--bin elon-server` 后，V244 代码和定向测试通过。本批次未修改该既有 PC 节点基线问题。

## 未验证

- 生产数据库原位升级、并发压力、备份恢复、真实 KMS/credential resolver 和真实外部矿池；
- V239/V243 报告自然到期的长时间等待测试；
- 制品安装、Provider 新版本/激活、v213 route、worker/ACK、Runner、真实任务派发、计量、结算和付款；
- 管理 UI、MCP 与生产部署。

因此当前只能表述为“精确外部矿池 Adapter 证据的可撤销采用授权已局部验证”，不能表述为“Adapter 已安装”或“外部算力已可接单”。
