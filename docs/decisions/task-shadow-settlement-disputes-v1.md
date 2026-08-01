---
title: "影子结算争议案件与投影阻断 V1"
status: accepted
decided_at: 2026-08-01
owners:
  - backend
  - pc
implementation_refs:
  - "migration:123"
  - "file:server/src/task_settlement/dispute_service.rs"
  - "file:server/src/store/task_settlement_disputes.rs"
  - "file:pc-frontend/src/features/open-commerce/SettlementDisputes.tsx"
acceptance_ref: "docs/task-shadow-settlement-v1-acceptance.md"
---

# 影子结算争议案件与投影阻断 V1

## 背景

影子经济层已经保存用量、策略、凭证、双分录和 Sui 链下投影，但此前没有正式争议对象。参与者发现计量金额、节点分配、策略或来源证据有误时，只能在聊天中说明，无法形成稳定、可复核的处理证据。

## 决定

1. 项目编辑者可对已对账影子凭证提出争议，原因限定为金额、节点分配、策略、来源证据或其他，并可引用 Matter、Artifact 或审计记录。
2. 争议状态固定为 `open`、`accepted`、`rejected`、`withdrawn`。每个状态变化都追加不可删除事件，案件保留提出者、处理者、说明和时间。
3. 同一凭证同时最多有一个待审核争议，完全相同的重复请求幂等返回原案件；内容不同则拒绝覆盖。驳回或撤回后可以重新提出，接受后不得再次建案。
4. `open` 和 `accepted` 均阻断原凭证生成 Sui 信封、准备新投影包，并把已有投影包的就绪状态派生为 `dispute_blocked`。驳回或撤回后解除阻断。
5. 接受争议不修改原凭证、双分录、节点收益或任何真实资金。金额纠正已按 `docs/decisions/task-shadow-settlement-corrections-v1.md` 实现为独立 Matter，以及同一事务内追加的冲销与替换凭证。

## 权限与边界

- 项目成员可以读取案件与事件；V1 只有项目编辑者可以提出、撤回或审核。
- V1 允许同一编辑者提出和处理，事件会完整记录参与者；生产资金治理仍应引入双人复核或独立审核角色。
- 争议摘要和证据引用不得包含密钥、原始订单明细、客户隐私或数据库转储。
- 本期不实现自动退款、自动冲正、真实赔付、法定仲裁、链上投票或收入权益调整。
