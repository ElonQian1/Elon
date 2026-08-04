---
title: 分布式算力结算账户审计视图与提款队列
status: current
reviewed_at: 2026-08-05
owners: ai-economy, backend
---

# 分布式算力结算账户审计视图与提款队列

## 1. 当前实现

Provider 本人结算账户视图和平台管理员提款队列已经写入独立 Store、Service 与 HTTP 路由，但尚未编译或运行接口验证，状态固定为 `implementation_uncompiled`。该能力复用 v195、v198-v201 数据表，不增加新迁移，也不修改任何余额。

## 2. HTTP 路由

| 方法 | 路径 | 调用者 | 作用 |
|---|---|---|---|
| GET | `/api/me/compute/providers/:provider_id/settlement-account` | Provider 所有者 | 读取经不可变账本重建的本人结算账户 |
| GET | `/api/admin/compute/settlement-withdrawals` | 平台 `admin/owner` | 按状态读取有界提款处理队列 |

管理员队列默认 `status=pending`、`limit=50`。状态只允许 `all`、`pending`、`cancelled`、`rejected` 或 `external_paid_attested`，单次上限为 100。每个队列项都重新审计 v200 Request Receipt；存在终态时同时重审计 v201 Terminal Receipt。

## 3. 账户视图

账户视图返回：

- CNY `pending`、`available`、`disputed` 和 `withdrawn` 微单位余额；
- 当前账户 revision 与更新时间；
- 提款申请数量和申请总额；
- 尚待终态的申请数量与冻结额；
- 取消、拒绝和外部已付款声明数量；
- 已返还 available 的总额和外部已付款声明总额；
- `projection_digest` 与 `audit_status=verified_from_append_only_ledgers`。

尚未产生收益的 Provider 返回零余额视图，不要求预先创建账户行。多个 Provider 若显式共享同一个 `settlement_account_id`，视图按该结算账户汇总，而不是伪造 Provider 独占余额。

## 4. 账本重建

每次读取分别重建：

- `pending = v195 Provider pending credits - v199 corrections - v198 releases`；
- `available = v198 available credits - v200 reserves + v201 returns`；
- `withdrawn = v200 reserves - v201 returns`；
- `disputed = 0`，因为当前挑战门卫没有把资金转入独立 disputed 余额。

提款生命周期还必须满足：

- `withdrawn = pending_terminal_micros + external_paid_attested_micros`；
- `requested = returned + pending_terminal + external_paid_attested`。

全部运算使用有溢出检查的整数微单位。当前账户投影、不可变账本和生命周期聚合任一不一致时读取失败关闭，不返回未经验证的余额。

## 5. 非付款边界

账户视图和队列均为只读能力：

- 不审批或拒绝申请；
- 不触发银行、钱包、支付机构或 Sui 网络；
- 不自动验证外部证据；
- 不把 `external_paid_attested` 解释为平台已经验证付款；
- 不改变 v200/v201 唯一终态规则。

## 6. 尚未实现

- Cargo 编译和 HTTP 真实调用；
- 游标分页、队列总数、处理时效和风险标签；
- 管理工作台 UI、通知和批量处理；
- 外部付款适配器、回执核验和自动对账；
- 平台账户提款、多币种及 Sui 链上资产视图。

## 7. 代码入口

- `server/src/store/compute_settlement_account_views.rs`
- `server/src/compute_federation_settlement_account_service.rs`
- `server/src/compute_federation_settlement_account_api.rs`

提款申请和唯一终态分别见 `docs/distributed-compute/settlement-withdrawal-request-api.md` 与 `docs/distributed-compute/settlement-withdrawal-terminal-api.md`。
