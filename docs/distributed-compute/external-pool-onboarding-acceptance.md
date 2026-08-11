---
title: 外部算力池 Onboarding Store 验收证据
status: current
reviewed_at: 2026-08-11
owners: backend, security, ai-economy
implementation_status: implementation_partially_verified
---

# 外部算力池 Onboarding Store 验收证据

## 1. 验收结论

v221 的 owner request、独立 review 与 immutable apply 已在真实临时文件 SQLite 上执行。approved apply 会在同一事务登记一份 exact `external_pool/registering/self_declared` Provider 与不可变 application；失败路径不会登记 Provider。

该证据只证明 Store 元数据和 Provider 注册原子性。owner/admin 登录角色、脱敏与进程内 HTTP 证据见 [`external-pool-onboarding-api-acceptance.md`](external-pool-onboarding-api-acceptance.md)，MCP 证据见 [`compute-management-mcp-acceptance.md`](compute-management-mcp-acceptance.md)；这些证据都不读取 credential、不连接外部矿池、不签发 v213 route，也不创建容量、Offer、任务或结算。

## 2. 服务端证据

2026-08-11 执行：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-rust.ps1 -Domain compute-external-pool-onboarding-store -- test --manifest-path server/Cargo.toml --bin elon-server compute_external_pool_onboarding::tests -- --nocapture
```

结果：2 项测试通过，验证指纹为 `a512cc2e59b1d9038fed71a363120f4556d089a92288630a08736762db3bdd77`。覆盖：

- owner submit→独立管理员 approved review→immutable apply；
- apply 与 Provider revision/digest 在同一事务精确绑定；
- request、review、application 三段精确幂等重放；
- 关闭数据库、重开后再次 exact readback 与重放；
- Provider 以 `external_pool/registering/self_declared` 形状持久化；
- owner 不能复核自己的 request；
- 错误 apply 确认语失败关闭；
- `changes_requested` 不能 apply 且不会登记 Provider；
- 同幂等身份改变 review reason 不能改写历史。

## 3. 未验证边界

- 生产部署中的 owner/admin 角色证明、鉴权与接口并发；进程内 Service/HTTP 与 MCP 证据分别见上述两份验收文档；
- 多连接并发、生产数据库原位升级、异常断电和高并发压力；
- non-bearer credential resolver、verifier、TTL/revocation 与 service actor；
- Adapter release admission、artifact verification 与 v213 route producer 的联合闭包；
- 外部矿池网络、authenticated ACK/event、容量、派发、用量和结算。

下一步不能因为 Provider 已登记为 `registering` 就把它升级为 active、可报价或可派发；仍须先闭合 Adapter artifact、verifier、credential、service actor 与 route currentness。
