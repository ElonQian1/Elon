---
title: 外部矿池 Adapter 凭据验证器签名公钥验收
status: current
reviewed_at: 2026-08-12
owners: backend, security, ai-economy
implementation_status: implementation_partially_verified
---

# 外部矿池 Adapter 凭据验证器签名公钥验收

## V242 定向验证

`elon-server` 测试二进制已成功重新编译。受管验证器因仓库根目录没有受跟踪 `Cargo.lock` 而在 Cargo 启动前拒绝运行，随后使用仓库日志封装器和受跟踪的 `server/Cargo.lock` 执行两项真实测试：

- migration：1 项通过、0 失败、1766 项过滤；
- HTTP：1 项通过、0 失败、1766 项过滤。

迁移测试覆盖重复迁移、两张追加式表、current view、文本主键非空和关键门卫存在。HTTP 测试覆盖未登录/普通成员拒绝、未知字段 `422`、V241 创建者不能自登记公钥、另一管理员精确登记、幂等重放、`INSERT OR REPLACE` 拒绝、active currentness、父 V241 撤销传播、公钥撤销与响应脱敏。

## 未验证

- 生产数据库原位升级、并发压力、真实 TCP、部署、备份恢复、MCP 与 PC 管理界面；
- 私钥保管、签名进程、真实 credential resolver/KMS、外部端点认证和限时验证回执；
- Adapter adoption/install、v213 route、worker/ACK、Runner、真实派发、计量、结算和付款。

因此 V242 只能表述为“精确凭据验证器签名公钥和撤销传播已局部验证”，不能表述为“外部凭据已经验证”或“Adapter 已上线”。
