---
title: 分布式算力结算账户审计视图与提款队列
status: current
reviewed_at: 2026-08-05
owners: ai-economy, backend
---

# 分布式算力结算账户审计视图与提款队列

## 1. 当前实现

Provider 本人结算账户视图、按 Provider 过滤的本人提款队列、平台结算账户视图和管理员全局提款队列已经写入独立 Store、Service 与 HTTP 路由，但尚未编译或运行接口验证，状态固定为 `implementation_uncompiled`。这些能力复用 v195、v198-v201 数据表，不增加新迁移，也不修改任何余额。

## 2. HTTP 路由

| 方法 | 路径 | 调用者 | 作用 |
|---|---|---|---|
| GET | `/api/me/compute/providers/:provider_id/settlement-account` | Provider 所有者 | 读取经不可变账本重建的本人结算账户 |
| GET | `/api/me/compute/providers/:provider_id/settlement-withdrawal-queue` | Provider 所有者 | 按状态读取并重审计指定本人 Provider 的提款队列 |
| GET | `/api/admin/compute/settlement-account` | 平台 `admin/owner` | 读取平台算力市场结算账户审计投影 |
| GET | `/api/admin/compute/settlement-withdrawals` | 平台 `admin/owner` | 按状态读取有界提款处理队列 |

本人和管理员队列默认 `status=pending`、`limit=50`。状态只允许 `all`、`pending`、`cancelled`、`rejected` 或 `external_paid_attested`，单次上限为 100。本人入口先验证 Provider 账户归属，在 SQL 层限制 Provider ID，并对每个返回回执再次核对 Provider，不能跨 Provider 读取。每个队列项都重新审计 v200 Request Receipt；存在终态时同时重审计 v201 Terminal Receipt。

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

## 5. 平台账户视图

平台管理员可读取固定账户 `platform:compute_market` 的 CNY 投影。服务端分别从以下不可变账本重建：

- v195 `platform_pending` 贷记形成累计平台价差；
- v199 `platform_pending_reversal` 借记形成累计纠正冲减；
- v198 `platform_pending_release` 借记与 `platform_available_credit` 贷记必须笔数和金额同时守恒；
- `pending = 累计价差 - 累计纠正 - 累计释放`；
- `available = 累计释放`。

视图同时返回各类 Posting 数量、累计金额、账户 revision、更新时间、投影摘要和审计状态。平台账户当前没有提现入口，因此 `disputed` 与 `withdrawn` 必须为零；任一投影不一致时读取失败关闭。

## 6. PC 管理入口

`pc-frontend/src/features/compute-settlement/` 已写入两个页面源码。所有登录用户可从 `/my-compute-settlement` 切换本人 Provider，查看 pending/available/withdrawn、按终态读取本人队列、申请把 available 转入 withdrawn，并在二次确认后取消 pending 申请。只有本地用户角色为 `admin/owner` 时显示 `/compute-settlement` 管理导航；服务端仍独立执行最终鉴权。管理员页读取平台账户、到期释放候选和全局提款队列，并可在二次确认后调用到期逐笔释放。对于 pending 提款，管理员还可选择拒绝并把 withdrawn 全额返还 available，或在确认系统外付款已经完成后登记证据类型、公开引用和 64 位证据摘要。两页的写请求都精确携带既有摘要并复用 v198-v201 幂等与唯一终态门卫。页面尚未执行 TypeScript 构建、浏览器视觉验收或发布，不能描述为线上可用。

## 7. 非付款边界

账户视图和队列接口本身均为只读能力。PC 本人页另行复用 v200 申请与 v201 取消，管理页复用 v201 管理员终态，但仍遵守以下非付款边界：

- 不触发银行、钱包、支付机构或 Sui 网络；
- 不自动验证外部证据；
- 不把 `external_paid_attested` 解释为平台已经验证付款；
- 不允许在未确认系统外付款已完成、未提供证据摘要或证据引用可能含秘密时登记已付款声明；
- 不允许通过平台账户视图提取平台 available；
- 不改变 v200/v201 唯一终态规则。

## 8. 尚未实现

- Cargo 编译和 HTTP 真实调用；
- 游标分页、队列总数、处理时效和风险标签；
- PC 本人页与管理页的构建、视觉验收、发布和实时通知；
- 外部付款适配器、回执核验和自动对账；
- 平台账户提款、多币种及 Sui 链上资产视图。

## 9. 代码入口

- `server/src/store/compute_settlement_account_views.rs`
- `server/src/store/compute_platform_settlement_account_view.rs`
- `server/src/compute_federation_settlement_account_service.rs`
- `server/src/compute_federation_settlement_account_api.rs`
- `pc-frontend/src/features/compute-settlement/`

提款申请和唯一终态分别见 `docs/distributed-compute/settlement-withdrawal-request-api.md` 与 `docs/distributed-compute/settlement-withdrawal-terminal-api.md`。
