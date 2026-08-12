---
title: 外部矿池 Adapter 凭据验证器身份验收
status: current
reviewed_at: 2026-08-12
owners: backend, security, ai-economy
implementation_status: implementation_partially_verified
---

# 外部矿池 Adapter 凭据验证器身份验收

## 编译

`elon-server` 定向测试目标已在合入 V240 后重新编译并通过；最终编译与行为验证使用下述 V241 定向测试指纹。

不限定 binary 的全工作区测试目标仍被既有 `elon-pc-node` SQLite VFS 测试可见性错误阻断；错误集中于 `ManagedTestFaultingFile` 和 registry lifecycle 类型的私有重导出，与 V241 文件无关，因此不能宣称全工作区编译通过。

## V241 定向测试

运行：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/validate-rust.ps1 -Domain v241-credential-verifier -- test --manifest-path server/Cargo.toml --bin elon-server credential_verifier -- --nocapture
```

结果：2 项通过、0 失败、1763 项过滤，合入远程 V240，并补齐文本主键非空、防替换与未知 JSON 字段 `422` 合同后的验证指纹为 `06792b322164ea271f2b59a1e2b8d5edc29760ab3fc9208be12eaac4c6f1332d`。

覆盖：

- V241 migration 连续执行两次，根表、转换表和 current view 唯一存在，文本主键显式非空且防替换触发器存在；
- 未登录和普通成员无法调用管理员接口；
- 未知 JSON 字段以 `422` 失败关闭；
- 注册、精确幂等重放、同一版本替换摘要失败关闭；
- 注册管理员不能自我激活，另一管理员可激活；
- active currentness、追加式撤销和 revoked currentness；
- HTTP 响应不出现幂等键、credential、bearer、token、secret、公钥或 verification receipt。

## 未验证

- 生产数据库原位升级、并发压力、真实 TCP、部署、备份恢复、MCP 和 PC 管理界面；
- 真实 credential verifier/KMS、凭据读取、TTL 验证回执和撤销传播；
- Adapter adoption/install、Sidecar IPC、v213 route、worker/ACK、Runner 和真实外部矿池；
- 真实派发、计量、结算、支付或链上资产。

因此 V241 只能表述为“凭据验证器实现身份和生命周期注册表已局部验证”，不能表述为“外部矿池凭据已经验证”或“Adapter 已上线”。
