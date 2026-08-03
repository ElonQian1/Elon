---
title: Sui 离线预检任务与短时租约 V1
status: accepted
owner: backend
reviewed_at: 2026-08-03
implementation_status: implementation_uncompiled
---

# Sui 离线预检任务与短时租约 V1

## 背景

V158 已能签发离线预检机器身份并接收摘要匹配的追加式报告，但仍要求人工先下载交接包，再把文件交给机器工具。为了让外部预检工具持续工作，需要一个明确的任务队列，同时必须防止它演变成后台签名、广播或无限占用任务的链上执行器。

## 决定

1. 项目编辑者只能从当前可导出的标准投影包或原子纠正包显式创建预检任务。入队时重新复核投影并固定目标网络、投影摘要和交接摘要。
2. 机器使用现有项目级预检适配器身份领取任务；只会看到自身允许的投影类型和目标网络。
3. 单次租约为 60 至 900 秒，处理硬截止时间为 1 小时。租约明文只返回一次，服务端只保存 SHA-256 和末尾提示。
4. 领取前服务端只读重建交接包。投影不可导出、争议阻断或摘要漂移时，任务进入 `blocked`，不会把过期内容交给机器。
5. 机器可以续租、主动释放或提交 `passed/rejected`。释放后任务重新等待领取；过期租约也会在下一次领取前回收。
6. 预检报告插入和任务完成必须在同一数据库事务内发生，并继续沿用适配器与幂等键的不可覆盖语义。
7. PC 工作台可以从投影包显式入队、查看任务状态并取消 `pending/blocked` 任务；不能取消正在执行或已经完成的任务。

## 接口

- 项目端：`GET/POST /api/projects/{project_id}/economy/sui-preflight-jobs`
- 项目端：`POST /api/projects/{project_id}/economy/sui-preflight-jobs/{job_id}/cancel`
- 机器端：`POST /api/economy/sui-preflight/jobs/claim`
- 机器端：`POST /api/economy/sui-preflight/jobs/{job_id}/renew|release|complete`

## 边界

- 所有机器接口继续受 `ELON_SUI_OFFLINE_PREFLIGHT_ENABLED` 控制并默认关闭。
- 任务不含钱包、私钥、签名、PTB、Gas、RPC、交易摘要、对象 ID 或最终性证明。
- 领取、续租、释放、完成和 `passed` 都不授权链上提交，也不移动真实资金。
- 当前实现尚未编译、执行 V159 迁移、调用 API、验证并发租约或检查 PC 页面。

## 实现引用

- `server/src/task_sui_preflight_job_migration.rs`
- `server/src/task_settlement/sui_preflight_job_*.rs`
- `server/src/store/task_sui_preflight_job_*.rs`
- `pc-frontend/src/features/open-commerce/SuiPreflightJobsPanel.tsx`
- `docs/sui-offline-preflight-job-leases-v1-acceptance.md`
