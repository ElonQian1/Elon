---
title: 外部矿池 Adapter 凭据独立签名验证回执验收
status: current
reviewed_at: 2026-08-12
owners: backend, security, ai-economy
implementation_status: implementation_partially_verified
---

# 外部矿池 Adapter 凭据独立签名验证回执验收

## V243 定向验证

执行 cargo test --manifest-path server/Cargo.toml --bin elon-server credential_verification --no-fail-fast。

结果为 2 项通过、0 失败、1767 项过滤：

- migration：真实迁移链可安装到 V243，V243 重复执行后表和 current view 均保持单一；
- HTTP：未登录和普通成员拒绝、未知凭据字段拒绝、精确挑战派生、错误 RSA 签名拒绝、正确签名创建、幂等重放、防 INSERT OR REPLACE、当前性查询、V242 公钥撤销后降级为历史、响应脱敏。

HTTP 用例从真实 Store 流程创建 V221 onboarding application、V222 staged admission、V241 active verifier 和 V242 active key，没有直接伪造 V243 上游记录。

## 基线异常

首次运行未限定二进制的同名测试时，仓库当前 elon-pc-node 测试目标存在 8 个与本功能无关的 SQLite VFS 测试类型可见性错误。V243 自身的测试越层访问问题已修复；随后限定 elon-server 的 V243 测试全部通过。该既有 PC 节点错误未在本批次修改。

## 未验证

- 生产数据库原位升级、并发压力、备份恢复、真实外部矿池/KMS、真实 credential resolver、网络故障和密钥托管；
- V243 报告自然到期的长时间等待测试；到期计算由数据库 current view 和领域时间窗覆盖；
- Adapter adoption/install、Provider 新版本、v213 route、worker/ACK、Runner、真实任务派发、计量、结算和付款；
- 管理 UI、MCP 和生产部署。

因此当前只能表述为“精确非 Bearer 凭据的短时独立签名验证回执已局部验证”，不能表述为“Adapter 已安装”或“外部算力已可执行”。
