---
title: 开放商业消费者多来源偏好合并 V1
status: accepted
owner: open-commerce
reviewed_at: 2026-08-03
implementation_status: implementation_uncompiled
---

# 开放商业消费者多来源偏好合并 V1

## 背景

同一消费者可能从多个运营方或设备导入自己的可携带数据包。单包字段级采用只能比较一个来源与当前档案，不能说明不同来源是否对同一字段给出不同值，也不能保留最终值来自哪个包的证明。客户端自行拼接会绕过服务端修订检查和审计，因此需要独立的多来源合并流程。

## 决定

1. 每次预演必须选择 2 至 10 个当前用户和目标项目拥有的隔离导入记录，且每个包都必须包含低敏偏好档案。
2. 服务端逐字段展示当前值和所有来源候选值，计算不同候选值数量并显式标记冲突，不自动选择“最新”“可信”或任一运营方的值。
3. 可合并字段仍固定为 `categories`、`tags`、`city`、`max_unit_price_micros` 和 `prefer_public`。用户为需要改变的字段明确选择一个来源，未选择字段保持当前值。
4. 应用时重新读取所有导入包和当前偏好档案，要求当前修订与预演一致，并拒绝重复字段、未知字段、未参与预演的来源及相对当前值未变化的选择。
5. 合并后的偏好与采用记录在同一事务中写入。记录保存全部参与包、逐字段来源、运营方、包 ID、信任状态、信封摘要、载荷摘要、采用前后快照和修订号。
6. 回滚必须再次由用户确认，且当前修订必须仍等于合并结果修订；合并后已有修改时失败关闭。
7. 单包采用记录继续保留，V161 使用独立表保存多来源来源证明，不改变 V141 的读取和回滚语义。

## 边界

- 信任状态只作为来源证据，不自动决定冲突结果，也不提高数据权限。
- 本能力不恢复商户关系、Grant、披露或删除请求，不合并订单、支付、退款、履约、ERP、CRM 或商户私有数据。
- V1 处理“当前档案与多个静态来源”的显式字段选择，不是带共同祖先的三方历史合并。
- V1 不包含跨设备审批、多人投票、自动优先级或远端运营方同步。
- 当前实现未编译、未执行 V161 迁移、未运行接口或 PC 构建，状态为 `implementation_uncompiled`。

## 实现入口

- `server/src/open_commerce_portability_merge_model.rs`
- `server/src/open_commerce_portability_merge_service.rs`
- `server/src/store/open_commerce_consumer_portability_merges.rs`
- `server/src/open_commerce_portability_merge_api.rs`
- `server/src/open_commerce_portability_merge_migration.rs`
- `pc-frontend/src/features/open-commerce/ConsumerPortabilityMergePanel.tsx`
- `docs/open-commerce-consumer-portability-multi-source-merge-v1-acceptance.md`
