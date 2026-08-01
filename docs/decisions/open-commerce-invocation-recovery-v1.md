---
title: 开放商业孤儿调用与 Grant 预算恢复 V1
status: accepted
date: 2026-08-02
owners: backend, product
---

# 开放商业孤儿调用与 Grant 预算恢复 V1

## 背景

开放商业调用会先保存 `started` 记录，再执行配额、Grant 预算预留和能力处理器。正常成功或失败会在同一事务内确认或释放预算；但进程重启、任务异常退出或运行时未返回终态时，调用和预算预留可能长期停留，导致商户授权额度被永久占用。

## 决定

1. 服务器启动时，所有仍为 `started` 的调用都视为失去内存执行上下文，原子失败关闭。
2. 运行期间默认每 30 秒扫描一次；超过 120 秒仍为 `started` 的调用视为执行租约过期。扫描周期可由 `OPEN_COMMERCE_INVOCATION_RECONCILE_SECS` 调整，但不得低于 5 秒。
3. 同一 SQLite 立即事务必须共同完成：调用失败、已预留 Grant 次数和金额退回、预留记录置为 `released`、追加脱敏审计事件。
4. 启动恢复使用错误码 `server_restart_interrupted`，运行期过期使用 `invocation_lease_expired`；审计动作统一为 `invocation.recovered_failed`。
5. 没有预算预留的公开、项目内或无限额调用也必须失败关闭，但不改动 Grant 计数。
6. 调用终态不可被迟到处理结果覆盖；预算预留还必须在事务内复核调用仍为 `started`，避免恢复后的迟到流程重新占额。
7. 商户运行时请求上限为 15 秒，120 秒恢复阈值用于容纳调度和数据库延迟，不代表处理器可以无限执行。

## 安全边界

- 恢复审计只记录商户、能力、错误码及是否释放预算，不写原始请求值或处理结果。
- 如果外部商户运行时已产生副作用但平台未收到可信终态，调用仍失败关闭；重试必须依赖商户运行时的调用编号和幂等键核查，不能自动假定成功。
- 本决定不执行真实扣款、退款、赔偿、争议裁决、链上交易或跨数据库分布式回滚。

## 实现入口

- 原子恢复：`server/src/store/open_commerce_invocation_recovery.rs`
- 启动与周期调度：`server/src/open_commerce_invocation_recovery.rs`
- 迟到预算防护：`server/src/store/open_commerce_grant_budgets.rs`
- 验证：`server/src/open_commerce_invocation_recovery_tests.rs`
