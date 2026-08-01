---
title: "链外影子结算 V1 API"
owner: backend
reviewed_at: 2026-08-02
status: active
decision_ref: "docs/decisions/task-shadow-settlement-v1.md"
---

# 链外影子结算 V1 API

所有接口要求当前用户是项目成员。设置接口要求角色为 editor、admin 或 owner。

## 项目概览

```http
GET /api/projects/{project_id}/economy/overview
```

返回全局开关、项目开关、`shadow_only`、近期用量凭证、结算意图、结算凭证和整数微元汇总。列表是有界近期视图，不是财务报表。

## 项目设置

```http
PATCH /api/projects/{project_id}/economy/settings
Content-Type: application/json

{"enabled": true}
```

项目启用不等于系统实际运行；全局环境变量仍须开启。关闭项目不会删除历史凭证。

## 凭证详情

```http
GET /api/projects/{project_id}/economy/settlements/{receipt_id}
```

返回结算凭证、对应意图、来源用量凭证及双分录交易。凭证不存在或不属于当前项目时返回错误。

## Sui 投影信封

```http
GET /api/projects/{project_id}/economy/settlements/{receipt_id}/sui-envelope
```

仅 `receipt_kind=standard` 的已对账影子凭证可生成。`correction_reversal` 和 `correction_replacement` 必须由未来适配器作为同一个原子纠正包处理，不能分别生成普通信封。响应只描述未来对象键和候选 PTB 步骤，固定为：

```json
{
  "schema": "task_economy.sui_projection.v1",
  "shadow_only": true,
  "network_submission": "not_submitted"
}
```

接口没有网络副作用，不接收钱包、私钥、Gas 或链 ID。

## Sui 链下投影包

项目编辑者可把已对账凭证保存为不可变链下投影包：

```http
POST /api/projects/{project_id}/economy/settlements/{receipt_id}/sui-projections
Content-Type: application/json

{"target_network":"testnet"}
```

`target_network` 只能是 `devnet`、`testnet` 或 `mainnet`。相同项目、凭证、网络和 schema 的请求幂等返回同一个包；摘要或信封发生变化时返回冲突，不覆盖历史包。

```http
GET /api/projects/{project_id}/economy/sui-projections
GET /api/projects/{project_id}/economy/sui-projections/{projection_id}
POST /api/projects/{project_id}/economy/sui-projections/{projection_id}/verify
```

列表和详情允许项目成员读取，复核要求编辑权限。复核重新计算来源凭证摘要和投影摘要，并返回 `verified` 或 `conflict`。当前所有响应都必须保持：

```json
{
  "integrity_status": "verified",
  "submission_readiness": "adapter_required",
  "network_submission": "not_submitted",
  "submission_attempts": 0
}
```

`adapter_required` 表示链下包已通过完整性复核，但项目尚未实现钱包、签名、Gas、交易广播或最终性确认。

## 影子结算争议

项目成员可以查看凭证的争议案件和事件：

```http
GET /api/projects/{project_id}/economy/settlements/{receipt_id}/disputes
```

项目编辑者可以对已对账凭证提出争议：

```http
POST /api/projects/{project_id}/economy/settlements/{receipt_id}/disputes
Content-Type: application/json

{
  "reason_code": "amount",
  "summary": "计量金额与节点原始记录不一致",
  "evidence_ref": "artifact:billing-evidence"
}
```

`reason_code` 只能是 `amount`、`provider_allocation`、`policy`、`source_evidence` 或 `other`。同一凭证同时只能有一个 `open` 案件；完全相同的请求幂等复用，内容漂移返回冲突。

```http
POST /api/projects/{project_id}/economy/disputes/{dispute_id}/withdraw
{"note":"证据仍需补充，先撤回"}

POST /api/projects/{project_id}/economy/disputes/{dispute_id}/resolve
{"decision":"accept","note":"确认需要另建纠正凭证"}
```

`decision` 只能是 `accept` 或 `reject`。`open` 与 `accepted` 会阻断 Sui 信封和新投影包，并使既有投影包返回 `submission_readiness=dispute_blocked`。接受争议仅确认需要纠正，不自动退款、冲正或修改原账本。

## 追加式纠正 Matter

读取某张原凭证的全部纠正流程：

```http
GET /api/projects/{project_id}/economy/settlements/{receipt_id}/corrections
```

项目编辑者可为已接受争议创建纠正 Matter。金额单位仍为人民币微元：

```http
POST /api/projects/{project_id}/economy/disputes/{dispute_id}/corrections
Content-Type: application/json

{
  "corrected_compute_amount_micros": 600000,
  "corrected_provider_amount_micros": 500000,
  "summary": "依据节点原始日志重新核对金额和节点分配",
  "evidence_ref": "artifact:correction-review"
}
```

节点金额不能高于计算金额。同一争议的相同活动请求幂等复用；内容变化返回冲突；已取消计划可重新创建。返回状态为 `matter_pending`，必须等待关联 Matter 完成并通过人工验收。

Matter 验收时系统会自动尝试过账。编辑者也可在修复暂时性错误后重试：

```http
POST /api/projects/{project_id}/economy/corrections/{correction_id}/finalize
```

过账要求全局和项目影子经济开关均已开启，并在同一事务内追加 `correction_reversal` 与 `correction_replacement` 两张已平衡凭证。原凭证不会修改。取消 Matter 只把纠正计划标记为 `canceled`，不写纠正凭证。

## Sui 纠正双腿链下投影包

项目编辑者可为已验收并完成过账的纠正准备一个同时绑定冲销与替换凭证的链下原子包：

```http
POST /api/projects/{project_id}/economy/corrections/{correction_id}/sui-projections
Content-Type: application/json

{"target_network":"testnet"}
```

单条冲销或替换凭证继续被普通投影接口拒绝。纠正包固定 `atomic_bundle=true`，并保存两条腿、来源包摘要、目标网络投影摘要和 `not_submitted` 状态。

```http
GET /api/projects/{project_id}/economy/sui-correction-projections
GET /api/projects/{project_id}/economy/sui-correction-projections/{projection_id}
POST /api/projects/{project_id}/economy/sui-correction-projections/{projection_id}/verify
```

列表和详情允许项目成员读取，准备和复核要求编辑权限。相同项目、纠正、网络和 schema 的请求幂等；任一纠正腿、来源摘要或信封不一致时复核为 `integrity_conflict`。替换凭证出现新的阻断争议时返回 `dispute_blocked`。当前接口不创建 PTB、不连接 Sui 网络，也不移动任何资金。

## 纠正链与当前有效凭证

项目成员可从任意标准、冲销或替换凭证解析完整纠正链：

```http
GET /api/projects/{project_id}/economy/settlements/{receipt_id}/lineage
```

响应包含 `root_receipt`、`requested_receipt`、`requested_position`、`effective_receipt`、`posted_corrections`、`non_posted_corrections`、`depth` 和 `effective_has_blocking_dispute`。只有已过账纠正会推动有效凭证；待验收和已取消计划只用于审计展示。

解析最多向前或向后遍历 32 层。循环、分叉、缺失替换凭证或关联不一致均返回错误，不修改任何数据。`effective_receipt` 表示当前影子事实，不是法定财务报表或真实余额。

## 自动写入点

| 现有事件 | 影子行为 |
|---|---|
| Assignment 记录合法 `compute_call_id` | 创建或重放用量凭证和待处理意图 |
| Matter 通过 Review Gate 并人工验收 | 为每个待处理意图创建一次已对账凭证和双分录 |
| Matter 验收前取消 | 作废待处理意图，不产生账本金额 |
| 开放商业能力调用成功 | 创建“已计量、未扣费”用量凭证，不创建资金结算 |

所有自动写入都是现有业务的旁路。失败会写告警，但不回滚主业务。
