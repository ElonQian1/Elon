---
title: 分布式算力到期结算释放队列与管理员批处理
status: current
reviewed_at: 2026-08-05
owners: ai-economy, backend
---

# 分布式算力到期结算释放队列与管理员批处理

## 1. 当前实现

到期候选扫描、独立 Service、管理员 HTTP 路由以及 v202 追加式批次意图/完成回执已经写入代码，但尚未编译、执行迁移或运行接口验证，状态固定为 `implementation_uncompiled`。该能力不创建第二套释放账本；它只发现已满 72 小时且尚无 v198 Release Receipt 的 Settlement，再逐笔复用现有 v198 原子释放入口。v202 只保存本次扫描和处理报告，不直接移动余额。

## 2. HTTP 路由

| 方法 | 路径 | 调用者 | 作用 |
|---|---|---|---|
| GET | `/api/admin/compute/settlement-releases/due` | 平台 `admin/owner` | 读取有界的到期候选及挑战阻断原因 |
| POST | `/api/admin/compute/settlement-releases/due` | 平台 `admin/owner` | 对当前 eligible 候选逐笔执行 v198 内部释放 |
| GET | `/api/admin/compute/settlement-releases/batches` | 平台 `admin/owner` | 按稳定游标读取批次意图、完成状态和审计摘要 |

到期候选 GET 与 POST 的 `limit` 默认 50，服务端限制为 1 至 100；两者都接受响应返回的可选不透明 `cursor`。页面使用 `settled_at + lease_id` 的稳定 keyset 顺序扫描，响应返回 `total_due_candidates`、`has_more` 和可选 `next_cursor`。总数只统计当前截止时间内尚无 v198 Release 的结构化记录，不代表这些记录都已通过挑战门卫或逐项审计。无效、损坏或版本不匹配的游标失败关闭。POST 必须显式确认每一笔只执行 v198 的 `pending -> available` 内部转账，并且只处理当前游标页。

POST 还接受可选 `idempotency_key`。同一管理员使用相同幂等键重试时，服务端复用首次保存的原候选页；若已有完成回执则直接返回原报告并标记 `replayed=true`，若只有意图则重新逐笔调用既有 v198 幂等入口。未提供幂等键的兼容调用由服务端生成新键，不能获得跨网络重试的批次级幂等保证。

## 3. 候选与门卫

候选查询只选择 `settled_at + 72 小时` 已到期且尚无 Release Receipt 的 Settlement。每一项随后重新审计完整 v195 Settlement Receipt，并读取当前 v196-v199 挑战门卫：

- `none`、`rejected`、`withdrawn` 和 `accepted_corrected` 标记为 `eligible`；
- `open` 或尚未完成 v199 纠正的 `accepted` 保留在队列中，但标记阻断原因；
- 审计不一致、时间异常或依赖损坏时整个读取失败关闭，不返回未经验证的候选。

该队列是实时派生视图，不保存“待处理任务”，也不保证读取后状态不会发生并发变化。游标只定位稳定排序键，不冻结候选快照；调用方处理一页后应继续使用该页返回的 `next_cursor`，或回到第一页重新读取。真正写入时，v198 会再次执行全部回执、时间、挑战、余额和 revision 检查。

## 4. 批处理语义

批处理为每个 eligible Settlement 生成由 Settlement Receipt ID 派生的稳定幂等键，并逐笔调用 `release_compute_attempt_settlement`：

- 每一笔拥有独立 `BEGIN IMMEDIATE` 事务；
- 某一笔失败不会回滚此前已经成功的释放；
- blocked 项进入 `skipped`，写入失败或并发状态变化进入 `failed`；
- 成功项返回完整 v198 Release Receipt；
- 报告同时给出扫描数、eligible 数、扫描时总数、`has_more`、`next_cursor` 以及成功、跳过和失败明细。

因此，返回批处理报告不表示“整批原子成功”。调用方必须逐项处理结果，必要时重新读取到期队列。

## 5. 批次历史与中断语义

v202 把一次管理员操作拆成两类不可变记录：

- 执行前先保存批次意图、请求摘要和原候选页摘要；
- 全部逐笔尝试结束后，再追加完成报告及其摘要；
- 两张表都禁止更新和删除，历史读取会重新计算请求、候选页和报告摘要；
- 历史按 `started_at + batch_run_id` 倒序分页，状态只分 `completed` 与 `incomplete`。

`incomplete` 只表示完成回执尚未写入。由于每笔 v198 使用独立事务，进程可能在部分释放成功后中断，因此不得把 `incomplete` 解释为“没有资金变化”。重试必须复用原幂等键，由 v198 单笔回执决定哪些项目是首次释放、哪些是幂等重放。

## 6. 资金与自动化边界

该能力只把符合门卫的 Provider 和平台净收益从内部 `pending` 转入内部 `available`：

- 不创建提款申请；
- 不移动 withdrawn；
- 不调用银行、钱包、支付机构或 Sui 网络；
- 不证明现金到账或链上最终性；
- 不启动后台定时器或无人值守自动清算。

它是平台管理员显式触发的有界运维入口。未来后台调度器即使接入，也必须复用同一候选审计与 v198 单笔原子释放内核，不能绕开挑战门卫。

## 7. 尚未实现

- Cargo 编译、HTTP 真实调用、并发竞争和故障注入验证；
- 后台定时扫描、任务租约、失败退避和运维告警；
- PC 管理页已按服务端游标逐页读取、显示本页/总数、复用失败重试幂等键并展示批次历史；源码尚未构建、视觉验收或发布；
- accepted 挑战的非金额补救和 available 事后追索；
- 真实提款、外部支付、自动对账、多币种或 Sui 链上结算。

## 8. 代码入口

- `server/src/store/compute_settlement_release_candidates.rs`
- `server/src/compute_federation_settlement_release_batch_service.rs`
- `server/src/compute_federation_settlement_release_batch_api.rs`
- `server/src/compute_settlement_release_batch_migration.rs`
- `server/src/store/compute_settlement_release_batch_runs.rs`
- `server/src/store/compute_attempt_settlement_releases.rs`
- `pc-frontend/src/features/compute-settlement/ComputeSettlementPage.tsx`

单笔原子释放合同见 `docs/distributed-compute/attempt-settlement-release-api.md`，账户与提款队列见 `docs/distributed-compute/settlement-account-view-api.md`。
