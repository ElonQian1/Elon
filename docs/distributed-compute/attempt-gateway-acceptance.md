---
title: Attempt v211-v215 迁移与门卫验收证据
status: current
reviewed_at: 2026-08-11
owners: backend, ai-economy
implementation_status: implementation_unwired
---

# Attempt v211-v215 迁移与门卫验收证据

## 1. 验收结论

v211-v215 的 sealed Plan、Start command、route/credential/actor authority、投递 outbox、no-start recovery 与 accepted 本地提交闭包源码已经存在并可编译。本轮没有创建第二套 Attempt，也没有给生产不可构造的 sealed 类型增加测试后门。

当前可确认的是：完整 Store 可把真实文件 SQLite 迁移到包含 v211-v215 的当前 schema，关闭和两次重开后迁移版本、关键表与反向触发器仍完整；迁移重复执行不会产生第二条版本记录；不安全的旧 cleanup 与 accepted ACK 冲突会被 backfill 门卫拒绝。

状态仍为 `implementation_unwired`。现有测试没有构造可信 Adapter ACK，也没有执行 `ingest_verified_compute_attempt_adapter_ack` 的 accepted 成功链，因此不能把源码中的原子闭包描述为已运行的真实派发、远端接受或节点执行。

## 2. 服务端证据

2026-08-11 执行：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-rust.ps1 -- test --manifest-path server/Cargo.toml --bin elon-server compute_attempt_activation_migration --locked
```

结果：rebase 合入 v221/v222 后，4 项迁移与 backfill 门卫测试通过，验证指纹为 `f49f8182264eb9ae70a80754021f68f9d4b65eac3008b1cd868682ebdc4d5f1e`。覆盖：

- 内存 SQLite 可完整应用 v211-v215，并幂等重复应用当前迁移；
- 真实临时文件 Store 可执行完整迁移，关闭、重开、重复迁移并再次关闭重开；
- 两次文件重开后 v211-v215 各自只有一条迁移记录，关键表和反向触发器仍存在；
- 空 v214 投影允许升级，不安全的 cleanup 与 `accepted_applied` ACK 冲突会失败关闭。

## 3. 未验证边界

- sealed Plan、route authorization、service actor、credential 与 authenticated ACK 的生产构造器；
- `prepare_compute_attempt_start_dispatch` 到 accepted ACK、v185、application actor、Lease authority、commit outbox 和 application 的成功链执行；
- 真实 worker、网络发送、ACK ingress、远端 prepare/commit/reconcile/cancel 与 Runner；
- 生产数据库原位升级、异常断电/进程崩溃恢复、长期磁盘耐久性和高并发压力；
- 实际用量、验证、最终结算、Provider 提现或任何外部付款。

后续应先实现最窄的可信输入生产者，再用同一 Store 成功链补原子闭包与重开验收；不得绕过 sealed capability，也不得恢复 Provider 人工确认激活入口。
