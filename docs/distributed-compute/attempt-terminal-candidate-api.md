---
title: 分布式算力 Attempt Provider 终态候选控制面
status: current
reviewed_at: 2026-08-04
owners: backend, node, ai-economy
implementation_status: implementation_uncompiled
---

# 分布式算力 Attempt Provider 终态候选控制面

## 1. 当前状态

v189、追加式 Store、Service 与 HTTP 路由已经写入代码，但尚未编译、执行迁移或运行接口验证，状态固定为 `implementation_uncompiled`。本控制面只保存 Provider 对 `succeeded`、`failed` 或 `canceled` 的首次终态声明，形成待验证候选；它不是 Execution Receipt，也不会把 Lease、Job、Reservation 或 Capacity Claim 推进到终态。

本批不接 NodeAgent 线协议。`executor_terminal_ref` 与 `diagnostic_ref` 只是外部 Host 事件或诊断材料的引用；平台不读取正文、不验证签名，也不据此扣款、退款或生成 Provider 收益。

## 2. HTTP 接口

| 方法 | 路径 | 权限 | 作用 |
|---|---|---|---|
| POST | `/api/me/compute/providers/:provider_id/attempt-leases/:lease_id/terminal-candidate` | Provider 所有者 | 登记第一份 Provider 终态候选 |
| GET | `/api/me/compute/attempt-leases/:lease_id/terminal-candidate` | Provider 所有者或 Job 消费者 | 读取并审计终态候选 |

写请求必须提供当前 Lease 的精确 revision/digest/fencing、最新 v188 用量快照 ID/序号/摘要、外部终态引用、outcome、规范 reason code、可选诊断引用、可选输出摘要、结果工件、幂等键，并显式设置 `confirm_provider_declaration_only=true`。

## 3. 失败关闭条件

一次写入必须同时满足：

- 调用者仍是当前 Provider 所有者，Provider 状态为 `active` 或 `draining`；
- Lease 当前为已心跳且未越过软期限或硬期限的 `running`，请求精确匹配 revision、digest 与 fencing generation；
- v185 激活回执绑定的 Job 仍为 `running`、Reservation 仍为 `active`、Capacity Claim 仍为 `active`；
- 最新 v188 用量快照与当前 Lease、Job、Reservation、Claim 的 ID、版本和摘要完全一致；
- outcome 只允许 `succeeded`、`failed`、`canceled`，reason code 只使用稳定小写代码字符；
- 同一 Lease 尚无其他终态候选。

相同 Provider 幂等键只能重放相同规范请求。同一 Lease 的第一份候选一旦保存，后续不同终态声明不能覆盖；重复读取和重放都会重新计算请求、结果工件与事件摘要。

## 4. 输出合同

`succeeded` 候选必须遵守 Job 的 Workload 输出合同：

- 合同要求结果工件时，至少提供一份工件；
- 合同要求确定性摘要时，必须提供 64 位小写 SHA-256 输出摘要；
- 结果工件只接受 SHA-256，ID 不得重复，媒体类型必须匹配合同；
- 工件总字节数不得超过 `max_output_bytes`。

`failed` 或 `canceled` 候选不得携带最终输出摘要或结果工件，避免把部分结果伪装成已完成交付。工件位置仍只是引用；v189 不证明对象存在、可读取、未被替换或符合数据授权。

## 5. 不可变回执

每份回执固定终态候选 ID、Lease/Job/Reservation/Claim 精确绑定、最终 v188 用量快照、外部终态引用、outcome、reason code、输出摘要、规范排序后的结果工件及其摘要、请求摘要、事件摘要、操作者与服务端时间。表禁止更新和删除，第一份候选不能被迟到事件覆盖。

响应效果固定为：

- `verification_status: "unverified_provider_declaration"`；
- `execution_effect: "candidate_only"`；
- `lease_effect: "unchanged"`；
- `job_effect: "unchanged"`；
- `capacity_effect: "unchanged"`；
- `reservation_effect: "unchanged"`；
- `money_effect: "preauthorization_unchanged"`。

因此，v189 不表示任务成功、失败责任成立、消费者已获得结果、用量可信、容量已消费、余额已扣除或 Provider 已获得收入。

v190 已允许候选绑定的 Job 消费者登记第一份 `accepted/rejected/disputed` 审核证据，见 `docs/distributed-compute/attempt-consumer-review-api.md`。该证据不修改本候选，消费者 `accepted` 也不把 Provider 声明提升为平台验证结果。

v191 已允许平台管理员登记第一份终态观测和累计 meter 差异，见 `docs/distributed-compute/attempt-platform-observation-api.md`。平台观测同样不修改本候选，也不直接形成 verified usage 或可信终态。

v192 已允许平台管理员精确绑定 v189-v191，以首版保守策略记录 Verification 决定和 verified/compensable usage，见 `docs/distributed-compute/attempt-verification-api.md`。v193 可基于 accepted 决定另行签发 Execution Receipt，但两者都不修改本候选。

## 6. 尚未实现

- Cargo 编译、v189 迁移执行、HTTP 真实调用、并发和故障注入验证；
- NodeAgent Host 到云端的签名 Terminal 事件、outbox、断点续传和真实节点身份；
- 输出工件导入、服务端重算摘要、恶意内容扫描、数据授权和可读取性验证；
- 自动平台观测接线、独立验证器、Execution Receipt 自动签发、争议裁决与多策略治理；
- Lease `result_reported/verifying/terminal`、Job `verification_pending`、Capacity Claim `UsageConsumed` 等真实状态推进；
- 消费者扣款、Provider 收益、失败退款、重试、多次 Attempt、外部矿池、多币种和 Sui 链上资产。

## 7. 实现入口

- `server/src/store/compute_attempt_terminals.rs`
- `server/src/store/compute_attempt_terminals/`
- `server/src/compute_attempt_terminal_migration.rs`
- `server/src/compute_federation_attempt_service.rs`
- `server/src/compute_federation_attempt_api.rs`
