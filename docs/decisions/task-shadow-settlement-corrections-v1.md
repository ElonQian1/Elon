---
title: "影子结算追加式纠正 Matter V1"
status: accepted
decided_at: 2026-08-02
owners:
  - backend
  - pc
implementation_refs:
  - "migration:124"
  - "file:server/src/task_settlement/correction_service.rs"
  - "file:server/src/store/task_settlement_corrections.rs"
  - "file:server/src/store/task_settlement_correction_posting.rs"
  - "file:pc-frontend/src/features/open-commerce/SettlementCorrections.tsx"
acceptance_ref: "docs/task-shadow-settlement-v1-acceptance.md"
---

# 影子结算追加式纠正 Matter V1

## 背景

争议案件已经能阻断有疑问凭证的 Sui 投影，但“接受争议”此前只确认问题存在，没有形成可执行、可验收的纠正流程。直接修改原凭证会破坏审计链；只追加一张新凭证又无法明确抵消旧事实。

## 决定

1. 只有 `accepted` 争议可以创建纠正计划。计划保存纠正后的计算金额、节点金额、平台金额、说明和证据引用，并原子创建独立 Matter。
2. 纠正状态固定为 `matter_pending`、`posted`、`canceled`。同一争议最多有一个待验收或已过账纠正；相同请求幂等复用，内容漂移返回冲突；取消后允许重新规划。
3. Matter 通过人工验收后，在同一数据库事务中追加两张凭证：`correction_reversal` 按原金额反向登记，`correction_replacement` 按纠正金额正向登记。两张凭证各自保持借贷平衡，任何一步失败都不提交。
4. 原凭证、争议、Matter 和全部事件均保持不可变或追加式。概览金额按“标准凭证 + 替换凭证 - 冲销凭证”计算，不删除、不覆盖历史。
5. Matter 取消只把待验收纠正标记为 `canceled`，不生成冲销或替换凭证。关闭影子经济开关时不得人工绕过过账，但取消仍可清理待验收计划。
6. 冲销凭证是会计反向记录，不能单独发起新争议；需要再次纠正时，应针对替换凭证建立新的争议与纠正链。
7. 单张纠正凭证不得进入普通 Sui 投影流程。未来链上适配器必须把冲销与替换作为同一个原子纠正包处理；V1 不生成该纠正包，也不提交网络。

## 权限与边界

- 项目成员可读取纠正流程；只有项目编辑者可创建和手工重试过账。
- 创建纠正不等于批准金额。Matter 必须先完成执行证据和人工验收。
- 本期只更新链外影子事实，不修改人民币余额、Token、节点提现、退款、赔付、合同收益或链上资产。
- 本期不实现真实会计凭证、税务处理、双人资金审批、Sui 纠正包或网络最终性。

## 不变量

```text
原凭证永不改写
纠正净额 = 替换金额 - 被冲销原金额
每个纠正腿借方合计 = 贷方合计
posted 必须同时拥有 reversal_receipt_id 和 replacement_receipt_id
canceled 不得拥有任何纠正凭证
普通 Sui 投影只接受 receipt_kind=standard
```
