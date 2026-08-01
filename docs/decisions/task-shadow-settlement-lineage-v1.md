---
title: "影子结算纠正链与有效凭证解析 V1"
status: accepted
decided_at: 2026-08-02
owners:
  - backend
  - pc
implementation_refs:
  - "file:server/src/task_settlement/lineage_service.rs"
  - "file:server/src/task_settlement/lineage_model.rs"
  - "file:pc-frontend/src/features/open-commerce/SettlementLineage.tsx"
acceptance_ref: "docs/task-shadow-settlement-v1-acceptance.md"
---

# 影子结算纠正链与有效凭证解析 V1

## 背景

追加式纠正会保留原凭证，并新增冲销与替换凭证。只看凭证列表时，用户或 AI 容易把冲销金额当成负收入、把已经被替换的凭证继续当作当前事实，或者忽略下一轮待验收纠正。系统需要一个只读、确定性的纠正链视图。

## 决定

1. 任意标准、冲销或替换凭证都可以作为查询入口。服务先沿 `correction_id` 向后回溯到根凭证，再从根凭证沿已过账纠正向前解析当前有效凭证。
2. 每个已过账步骤保留完整纠正详情、原凭证、冲销凭证和替换凭证；解析结果同时返回根凭证、请求凭证、请求位置、有效凭证和链深度。
3. `matter_pending` 和 `canceled` 纠正作为 `non_posted_corrections` 返回，但不改变当前有效凭证或金额。
4. 有效凭证存在 `open` 或 `accepted` 争议时返回 `effective_has_blocking_dispute=true`。这表示当前事实待核查，不自动选择尚未验收的计划金额。
5. 遍历上限为 32 层。发现循环关联、同一凭证多条已过账纠正、缺失替换凭证或双腿关联不一致时失败关闭，不猜测有效结果。
6. 查询是纯读取，不新增迁移，不修改凭证、争议、纠正、双分录、Sui 包或任何真实资金状态。

## 请求位置

`requested_position` 固定为以下一种：

- `effective_standard`：从未被纠正的标准凭证；
- `superseded_original`：已经被纠正的根标准凭证；
- `correction_reversal`：仅用于反向会计记录的冲销腿；
- `effective_replacement`：当前有效的替换凭证；
- `superseded_replacement`：后来又被纠正的中间替换凭证；
- `unknown`：发现未来未知种类时保守返回。

## 边界

“有效凭证”是影子账本中的当前追加式事实，不是删除或作废历史，也不代表真实余额、法定会计凭证、审计报告、退款、提现或链上最终性。
