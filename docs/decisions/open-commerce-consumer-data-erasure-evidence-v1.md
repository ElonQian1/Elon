---
title: 开放商业消费者删除请求外部证明 V1
status: accepted
date: 2026-08-03
owners: backend, product
implementation_status: implementation_uncompiled
---

# 开放商业消费者删除请求外部证明 V1

## 背景

消费者删除请求 V1 已能原子撤销关系并记录商户处理状态，但 `merchant_attested_completed` 只能证明商户提交过完成声明，不能说明 ERP、CRM、收银或会员系统实际执行了什么。直接把完成状态改名为“删除证明”会制造错误信任；强制旧客户端同时提交外部回执又会破坏既有状态机。

V1 因此在完成声明之外增加独立、追加式的证明账本。它让商户把自己持有的外部回执摘要交还消费者核验，同时始终保留“商户提供、平台未核验”的来源边界。

## 决定

1. 原删除请求状态机和接口保持不变；只有状态已为 `completed` 的请求可以附加证明。
2. 一项请求可以对应多个外部系统证明，适合分别记录 ERP、CRM、会员和收银系统的处理结果。
3. 每条证明固定保存证明类型、外部系统、回执编号、原始回执 SHA-256、摘要、提交时间和商户项目编辑者身份。响应不向消费者暴露提交者用户 ID。
4. `source_authority` 固定为 `merchant_supplied_unverified`，`platform_verified` 固定为 `false`。商户必须显式确认这一边界才能提交。
5. 平台不保存原始回执、外部系统凭据或待删除数据。SHA-256 只绑定商户持有的原始内容，不证明内容真实、签名有效或外部删除完成。
6. 相同请求、外部系统和回执摘要的重复提交返回既有记录，不重复追加证明或审计；不同摘要可以形成新的追加记录，历史不覆盖。
7. 商户项目成员可读取本商户证明，只有编辑者可提交；消费者只读取本人项目、本人请求所关联的证明。
8. PC 在已完成请求下提供证明登记和查看入口；消费者与商户两端均持续展示“平台未核验”。

## 非目标

V1 不连接美团、抖音、ERP、CRM 或会员系统执行删除，不下载或验证原始回执，不提供可信时间戳、外部签名、法律合规认证、争议裁决、赔付、真实资金或链上锚定。

## 实现入口

- 模型：`server/src/open_commerce_data_erasure_evidence_model.rs`
- 迁移：`server/src/open_commerce_data_erasure_evidence_migration.rs`
- 存储：`server/src/store/open_commerce_data_erasure_evidence.rs`
- 领域服务：`server/src/open_commerce_data_erasure_evidence_service.rs`
- HTTP：`server/src/open_commerce_data_erasure_evidence_api.rs`
- PC：`pc-frontend/src/features/open-commerce/DataErasureEvidenceList.tsx`、`MerchantDataErasureEvidenceForm.tsx`
- 验收：`docs/open-commerce-consumer-data-erasure-evidence-v1-acceptance.md`

## 当前状态

代码已提交，但按快速开发策略未运行 Rust 编译、PC 构建、TypeScript 检查、V160 迁移、接口、权限、幂等或界面验证。当前状态只能标记为 `implementation_uncompiled`。
