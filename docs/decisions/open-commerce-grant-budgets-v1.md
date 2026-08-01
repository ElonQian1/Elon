---
title: 开放商业授权生命周期预算 V1
status: accepted
date: 2026-08-01
owners: backend, product
---

# 开放商业授权生命周期预算 V1

## 背景

固定时间窗配额控制调用速度，但不能限制一个 App 在整段授权期内最多调用多少次、最多形成多少计量金额。消费者 AI 或第三方 App 获得 Grant 后，商户仍需要明确控制这段信任关系的总风险敞口。

## 决定

1. Grant 可选设置 `max_invocations`、`max_amount_micros` 和 `budget_currency`。全部留空时沿用原有无限额行为。
2. 预算属于单个 Grant 的完整生命周期，不按时间窗重置。V1 不允许原地扩容；商户需要撤销旧 Grant 并重新授权，以保留明确审计边界。
3. 仅携带 Grant 的 `authorized` 能力调用消耗预算。公开能力和项目内能力不绑定 Grant 预算。
4. 新调用在进入处理器前使用数据库立即事务原子预留一次调用和当前能力单价。并发请求不能先读后写而共同越过上限。
5. 处理器成功时确认预留；处理器失败时在调用失败事务中释放预留。进程在预留后异常终止时保持已预留状态，按失败关闭处理，后续由独立对账能力处理。
6. 幂等重放在预算预留前返回原调用，不重复消耗预算。
7. 超限调用不进入处理器，保存为 `failed/grant_budget_exceeded`，计量单位与金额均为 0，并返回 `403`。
8. 金额预算按能力当前单价预留，只是链外计量上限，不表示资金已经扣除。金额预算币种必须与能力币种一致。

## 与调用配额的区别

- 调用配额回答“单位时间内最多多快”，超限可在窗口重置后重试，HTTP 为 `429`。
- Grant 预算回答“本次授权关系总共最多多少”，用尽后必须重新授权，HTTP 为 `403`。
- 两者同时存在时，先执行固定时间窗配额，再执行 Grant 生命周期预算。

## 安全与隐私

- Grant 创建和授权审批要求商户项目编辑权限。
- 预算预留、调用终态和释放位于数据库事务中；失败调用不会形成收费单位。
- 审计只记录商户、能力、Grant 和稳定错误码，不保存原始请求值。
- V1 不自动提高上限，不允许开发者 App 修改商户授予的预算。

## 非目标

V1 不执行真实扣款、退款、收入分配、链上锁仓或跨数据库分布式事务。进程崩溃后遗留预留的后续恢复边界由 `docs/decisions/open-commerce-invocation-recovery-v1.md` 单独决定。

## 实现入口

- Schema：`server/src/open_commerce_grant_budget_migration.rs`
- 原子预留与释放：`server/src/store/open_commerce_grant_budgets.rs`
- 领域规则：`server/src/open_commerce_grant_budget_service.rs`
- 调用终态：`server/src/store/open_commerce_invocations.rs`
- 商户与审批界面：`pc-frontend/src/features/open-commerce/OpenCommerceMerchantEditor.tsx`、`DeveloperCommercePortal.tsx`
- 验收：`docs/open-commerce-grant-budgets-v1-acceptance.md`
