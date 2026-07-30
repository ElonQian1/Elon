---
title: "AI 任务与开放商业链外影子结算 V1"
status: accepted
decided_at: 2026-07-30
owners:
  - project
implementation_refs:
  - "file:server/src/task_settlement/"
  - "file:server/src/store/task_settlements.rs"
  - "migration:110"
acceptance_ref: "docs/task-shadow-settlement-v1-acceptance.md"
---

# AI 任务与开放商业链外影子结算 V1

## 决定

项目接受一个独立、链外、只读映射生产事实的影子经济层。它把节点执行、Assignment、Matter 人工验收和开放商业调用转换为幂等用量凭证、结算意图、不可变结算凭证和双分录账本。

V1 不发行代币、不连接钱包、不部署 Move 合约、不移动资金，也不修改现有人民币余额、Token 用量、节点收益和提现逻辑。现有生产账本继续是资金真源。

## 触发规则

功能只有同时满足以下条件才运行：

1. 服务端环境变量 `ELON_TASK_SHADOW_SETTLEMENT_ENABLED=true`；
2. 当前项目由 editor、admin 或 owner 显式启用。

任一条件不满足时，现有任务与开放商业流程保持原行为，不生成影子记录。

## 事实来源

- Assignment 用量只从 `node_compute_runs` 读取，不信任客户端自报金额。
- `compute_call_id`、Assignment 节点和 Matter 归属必须一致。
- 计算成本和节点收益沿用已经发生的整数人民币事实，仅转换为整数微元展示。
- 开放商业调用只生成“已计量、未扣费”用量凭证，不伪造真实结算。
- Matter 通过既有 Review Gate 并由有权限成员人工验收后，待处理意图才可过账。
- Matter 在过账前取消时，只生成作废凭证；已经发生的原始算力事实仍保留。

## 账本不变量

1. 同一项目和来源只能有一个用量凭证。
2. 同一项目和幂等键只能有一个结算意图。
3. 同一意图只能有一个结算凭证，历史凭证不得原地改写。
4. 所有金额使用非负整数微元，节点收益不得高于本次真实成本。
5. 每笔已过账交易借方总额必须等于贷方总额。
6. 影子层失败只能告警，不得阻断 Assignment 结算、Matter 验收或商业调用主流程。

## Sui 边界

V1 可以把一张已对账影子凭证投影成 Sui 数据信封，包含项目、意图、凭证对象键和候选 PTB 步骤。信封固定声明 `network_submission: not_submitted`。

该投影用于稳定未来适配器输入，不代表已经：

- 安装 Sui SDK；
- 创建钱包或保管私钥；
- 部署 Move Package；
- 提交测试网或主网交易；
- 发行 NET、CREDIT、RevenuePosition 或任何公司权益。

未来 Sui 适配器只能消费已对账凭证，不能绕过链外权限、验收、幂等和争议规则。

## 资产分层保持不变

- 服务成本和节点补偿属于生产计费事实。
- 影子凭证属于验证和对账事实。
- CREDIT、NET、RevenuePosition 与公司股权是不同权利，不能合并成一个代币。
- 合同收入权益必须按独立收入来源、期限、上限和实际到账建模，不能由本 ADR 自动推出。
