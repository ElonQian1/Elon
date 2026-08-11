---
title: 外部矿池 Adapter Release Store 验收证据
status: current
reviewed_at: 2026-08-11
owners: backend, security, ai-economy
implementation_status: implementation_partially_verified
---

# 外部矿池 Adapter Release Store 验收证据

## 1. 验收结论

v222 的 release request、独立 review 与 immutable staged admission 已在真实临时文件 SQLite 上执行，不再只是已编译的 Store 源码。测试保持 Store-private 边界，没有新增 service、HTTP、MCP、PC 或测试后门。

该证据只证明平台元数据状态机、规范摘要、幂等历史和 SQLite 门卫。它不下载 artifact、不重算实现字节摘要、不验证签名、供应链、capability conformance 或 credential verifier，也不创建 v213 route、Provider 容量、Offer、任务派发或结算。

## 2. 服务端证据

2026-08-11 执行：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-rust.ps1 -Domain compute-external-pool-adapter-release-store -- test --manifest-path server/Cargo.toml --bin elon-server compute_external_pool_adapter_release::tests -- --nocapture
```

结果：2 项测试通过，验证指纹为 `50eeb7dda60582b4ba94f2d315bc9b42abd2cd5957b2661af7c6bb46fc682c7a`。覆盖：

- submit→独立管理员 approved review→staged apply；
- request、review、admission 三段精确幂等重放；
- 关闭数据库、重开后再次 exact readback 与重放；
- 同一 `adapter_id/release_version` 不能由第二个 request 替换；
- submitter 不能复核自己的 request；
- 错误确认语失败关闭；
- `changes_requested` 不能 apply；
- 同幂等身份改变 review note 不能改写历史。

## 3. 未验证边界

- service/API 的管理员角色证明、鉴权、脱敏响应和接口幂等；
- 多连接并发竞争、高并发压力、生产数据库原位升级和异常断电；
- artifact resolver、字节摘要重算、签名、供应链与 conformance；
- credential verifier registry、TTL/revocation、service actor 与 v213 route producer；
- 外部矿池 Adapter、网络、authenticated ACK/event、派发、用量和结算。

下一步若继续推进 v222，应先做 service/API 权限闭环；若推进真实外部矿池，必须先实现 artifact/verifier/route 的独立可信生产者，不能把 staged admission 直接升级成可执行权威。
