---
title: 消费者可携带数据包 V5 删除证明
status: accepted
date: 2026-08-03
owners: backend, pc, product
implementation_status: implementation_uncompiled
---

# 消费者可携带数据包 V5 删除证明

## 背景

V4 已能携带关系、续期链、删除请求、低敏偏好、调用凭证和商户身份声明，但商户在请求完成后追加的外部删除证明仍只能在线查看。消费者如果不能把这些证明带走，其数据自主权仍依赖当前运营环境。

不能直接给 V4 增加字段：V1-V4 都通过规范 JSON 和 SHA-256 绑定内容，静默改变序列化结果会破坏历史摘要兼容。因此删除证明必须进入新的 V5 包。

## 决定

1. 新导出使用 `open_commerce.consumer_portability_export.v5` 和 `open_commerce.consumer_portability_payload.v5`。
2. V5 在 V4 基础上增加 `data_erasure_evidence`。每条可携带证明包含证明 ID、请求 ID、商户 ID、类型、外部系统、回执编号、回执 SHA-256、摘要、来源边界和创建时间。
3. 导出快照在同一数据库只读事务内读取关系、请求和证明；证明最多 5000 条，不携带商户项目编辑者账号或原始回执。
4. 每条证明必须引用包内状态为 `completed` 的真实删除请求，且商户 ID 一致；证明 ID 不得重复。
5. `source_authority` 必须为 `merchant_supplied_unverified`，`platform_verified` 必须为 `false`。V5 不提升原证明的信任强度。
6. 继续完整受理配对的 V1、V2、V3 和 V4 包。新字段使用缺省空数组且空值不序列化，保证旧包规范 JSON 不增加字段。
7. V1-V4 如果显式携带 V5 删除证明字段，验证失败关闭；不能通过伪造旧版本绕过 V5 规则。
8. 导入继续保存为当前用户和项目独享的隔离快照，不恢复关系，不合并证明，不写商户 ERP，也不改变当前删除请求状态。

## 非目标

V5 不迁移原始回执文件、外部系统凭据、订单、支付、联系方式或商户私有数据；不验证外部删除、不创建全网信任根、不进行自动合并、链上锚定或赔付。

## 实现入口

- 数据模型与版本验证：`server/src/open_commerce_portability_model.rs`、`open_commerce_portability_service.rs`
- 同事务快照：`server/src/store/open_commerce_consumer_portability.rs`
- 隔离导入摘要：`server/src/open_commerce_portability_import_model.rs`
- PC：`pc-frontend/src/features/open-commerce/ConsumerPortabilityExports.tsx`、`ConsumerPortabilityImports.tsx`
- 验收：`docs/open-commerce-consumer-portability-exports-v5-acceptance.md`

## 当前状态

代码已形成，但按快速开发策略尚未编译或测试。当前状态为 `implementation_uncompiled`，不能声称 V1-V5 摘要兼容、导出、导入或界面已经运行验证。
