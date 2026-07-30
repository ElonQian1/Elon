---
title: "链外影子结算 V1 API"
owner: backend
reviewed_at: 2026-07-30
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

仅已对账影子凭证可生成。响应只描述未来对象键和候选 PTB 步骤，固定为：

```json
{
  "schema": "task_economy.sui_projection.v1",
  "shadow_only": true,
  "network_submission": "not_submitted"
}
```

接口没有网络副作用，不接收钱包、私钥、Gas 或链 ID。

## 自动写入点

| 现有事件 | 影子行为 |
|---|---|
| Assignment 记录合法 `compute_call_id` | 创建或重放用量凭证和待处理意图 |
| Matter 通过 Review Gate 并人工验收 | 为每个待处理意图创建一次已对账凭证和双分录 |
| Matter 验收前取消 | 作废待处理意图，不产生账本金额 |
| 开放商业能力调用成功 | 创建“已计量、未扣费”用量凭证，不创建资金结算 |

所有自动写入都是现有业务的旁路。失败会写告警，但不回滚主业务。
