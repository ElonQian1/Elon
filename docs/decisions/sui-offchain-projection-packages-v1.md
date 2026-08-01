---
title: "Sui 链下投影包与完整性复核 V1"
status: accepted
decided_at: 2026-08-01
owners:
  - backend
  - pc
implementation_refs:
  - "migration:122"
  - "file:server/src/task_settlement/sui_projection_service.rs"
  - "file:server/src/store/task_sui_projection_packages.rs"
  - "file:pc-frontend/src/features/open-commerce/SuiProjectionPackages.tsx"
acceptance_ref: "docs/task-shadow-settlement-v1-acceptance.md"
---

# Sui 链下投影包与完整性复核 V1

## 背景

影子结算 V1 已能临时生成 `not_submitted` 的 Sui 信封，但临时响应没有持久编号、目标网络、内容摘要或复核状态，不能作为未来网络适配器的稳定输入。

## 决定

1. 项目编辑者可以把已对账影子凭证准备成链下投影包，并明确选择 `devnet`、`testnet` 或 `mainnet` 作为目标网络。
2. 投影包保存固定 schema、来源凭证摘要、包含目标网络的投影摘要和完整信封。同一项目、凭证、目标网络和 schema 只对应一个包；完全相同的请求幂等复用，内容漂移拒绝覆盖。
3. 项目成员可查看投影包，编辑者可重新复核。复核会从不可变影子凭证重新生成 v1 投影；一致时标记 `verified`，不一致时标记 `conflict` 并阻断后续适配器使用。
4. 当前实现不具备网络适配器。所有投影包固定 `network_submission=not_submitted`、`submission_attempts=0`，`submission_readiness=adapter_required` 只表示链下内容完整，不表示可以直接发送交易。
5. 投影包不保存钱包、私钥、签名、Gas、交易摘要或链上对象 ID，也不修改人民币余额、节点收益、提现和影子账本。

## 安全边界

- 只有已对账且 `shadow_only` 的凭证可准备投影包。
- 摘要算法使用显式版本化字段，不依赖数据库行的隐式序列化顺序。
- 完整性复核只能改写尚未提交网络的包；未来适配器进入提交生命周期后必须使用独立的带租约状态机。
- 选择 `mainnet` 只是记录目标，不会连接主网、创建交易或承诺资产存在。

## 非目标

本期不安装 Sui SDK，不部署 Move Package，不管理密钥，不签名或广播交易，不实现 Gas 赞助、链上重试、最终性确认、跨链桥、代币发行或收入权益分配。
