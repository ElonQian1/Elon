---
title: "Sui 纠正双腿链下投影包 V1"
status: accepted
decided_at: 2026-08-02
owners:
  - backend
  - pc
implementation_refs:
  - "migration:125"
  - "file:server/src/task_settlement/sui_correction_projection.rs"
  - "file:server/src/task_settlement/sui_correction_projection_service.rs"
  - "file:server/src/store/task_sui_correction_projection_packages.rs"
  - "file:pc-frontend/src/features/open-commerce/SuiCorrectionProjectionPackages.tsx"
acceptance_ref: "docs/task-shadow-settlement-v1-acceptance.md"
---

# Sui 纠正双腿链下投影包 V1

## 背景

普通 Sui 链下投影包只表达单张标准凭证。影子纠正由冲销和替换两张凭证共同构成；若适配器分别读取或只提交其中一张，会丢失纠正净额并破坏审计语义。因此，纠正不能复用普通单笔投影。

## 决定

1. 只有状态为 `posted`、关联 Matter 已人工验收，且同时存在合法 `correction_reversal` 和 `correction_replacement` 的纠正可以准备原子投影包。
2. 包同时绑定纠正 ID、Matter ID、原凭证 ID、两张纠正凭证 ID、两条腿完整字段及各自来源摘要，并固定 `atomic_bundle=true`。
3. 来源包摘要同时覆盖纠正金额、Matter 验收状态、原凭证摘要、冲销摘要和替换摘要；投影摘要再绑定目标网络和版本化信封。
4. 同一项目、纠正、目标网络和 schema 只对应一个包。相同请求幂等复用；任一 ID、摘要或信封漂移均返回冲突，不覆盖历史。
5. 复核从当前不可变纠正记录和三张关联凭证重新计算；一致为 `verified`，不一致持久标记 `conflict`。替换凭证出现新的待审核或已接受争议时，就绪状态派生为 `dispute_blocked`。
6. 原争议保持 `accepted` 不阻断纠正包，因为该包正是其已验收的追加式解决方案；后续争议必须针对替换凭证继续形成新的纠正链。
7. 当前包固定 `network_submission=not_submitted`、`submission_attempts=0`。`adapter_required` 只表示链下内容完整，不表示已经签名、广播或上链。

## 原子边界

未来网络适配器必须在一个受控提交单元中完成以下语义：

```text
验证纠正 Matter 验收与来源包摘要
  -> 同时读取冲销和替换凭证
  -> 只应用两者形成的净影子纠正
  -> 发出一个纠正事件
```

适配器不得只提交一条腿，也不得把包解释为真实转账指令。

## 非目标

本期不安装 Sui SDK，不创建 PTB，不管理钱包、私钥或 Gas，不签名、广播或确认交易，不移动人民币、Token、节点收益、退款或收入权益。
