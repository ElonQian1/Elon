---
title: 分布式算力到期结算释放队列与管理员批处理
status: current
reviewed_at: 2026-08-05
owners: ai-economy, backend
---

# 分布式算力到期结算释放队列与管理员批处理

## 1. 当前实现

到期候选扫描、独立 Service 和管理员 HTTP 路由已经写入代码，但尚未编译、执行迁移或运行接口验证，状态固定为 `implementation_uncompiled`。该能力不增加迁移，也不创建第二套释放账本；它只发现已满 72 小时且尚无 v198 Release Receipt 的 Settlement，再逐笔复用现有 v198 原子释放入口。

## 2. HTTP 路由

| 方法 | 路径 | 调用者 | 作用 |
|---|---|---|---|
| GET | `/api/admin/compute/settlement-releases/due` | 平台 `admin/owner` | 读取有界的到期候选及挑战阻断原因 |
| POST | `/api/admin/compute/settlement-releases/due` | 平台 `admin/owner` | 对当前 eligible 候选逐笔执行 v198 内部释放 |

GET 与 POST 的 `limit` 默认 50，服务端限制为 1 至 100。POST 必须显式确认每一笔只执行 v198 的 `pending -> available` 内部转账。

## 3. 候选与门卫

候选查询只选择 `settled_at + 72 小时` 已到期且尚无 Release Receipt 的 Settlement。每一项随后重新审计完整 v195 Settlement Receipt，并读取当前 v196-v199 挑战门卫：

- `none`、`rejected`、`withdrawn` 和 `accepted_corrected` 标记为 `eligible`；
- `open` 或尚未完成 v199 纠正的 `accepted` 保留在队列中，但标记阻断原因；
- 审计不一致、时间异常或依赖损坏时整个读取失败关闭，不返回未经验证的候选。

该队列是实时派生视图，不保存“待处理任务”，也不保证读取后状态不会发生并发变化。真正写入时，v198 会再次执行全部回执、时间、挑战、余额和 revision 检查。

## 4. 批处理语义

批处理为每个 eligible Settlement 生成由 Settlement Receipt ID 派生的稳定幂等键，并逐笔调用 `release_compute_attempt_settlement`：

- 每一笔拥有独立 `BEGIN IMMEDIATE` 事务；
- 某一笔失败不会回滚此前已经成功的释放；
- blocked 项进入 `skipped`，写入失败或并发状态变化进入 `failed`；
- 成功项返回完整 v198 Release Receipt；
- 报告同时给出扫描数、eligible 数以及成功、跳过和失败明细。

因此，返回批处理报告不表示“整批原子成功”。调用方必须逐项处理结果，必要时重新读取到期队列。

## 5. 资金与自动化边界

该能力只把符合门卫的 Provider 和平台净收益从内部 `pending` 转入内部 `available`：

- 不创建提款申请；
- 不移动 withdrawn；
- 不调用银行、钱包、支付机构或 Sui 网络；
- 不证明现金到账或链上最终性；
- 不启动后台定时器或无人值守自动清算。

它是平台管理员显式触发的有界运维入口。未来后台调度器即使接入，也必须复用同一候选审计与 v198 单笔原子释放内核，不能绕开挑战门卫。

## 6. 尚未实现

- Cargo 编译、HTTP 真实调用、并发竞争和故障注入验证；
- 后台定时扫描、任务租约、失败退避和运维告警；
- 管理工作台 UI、游标分页、总数和批次历史；
- accepted 挑战的非金额补救和 available 事后追索；
- 真实提款、外部支付、自动对账、多币种或 Sui 链上结算。

## 7. 代码入口

- `server/src/store/compute_settlement_release_candidates.rs`
- `server/src/compute_federation_settlement_release_batch_service.rs`
- `server/src/compute_federation_settlement_release_batch_api.rs`
- `server/src/store/compute_attempt_settlement_releases.rs`

单笔原子释放合同见 `docs/distributed-compute/attempt-settlement-release-api.md`，账户与提款队列见 `docs/distributed-compute/settlement-account-view-api.md`。
