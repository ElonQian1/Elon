---
title: 消费者可携带数据包 V4 商户身份声明
status: accepted
date: 2026-08-03
owners: backend, pc, product
---

# 消费者可携带数据包 V4 商户身份声明

## 背景

V3 能携带关系和调用凭证，但关系中的商户 ID 只在来源运营环境内有效。消费者迁移到新环境后，只能按名称手工寻找目标商户，无法利用商户已发布的私钥持有证明缩小候选范围。

## 决定

1. 新导出使用 `open_commerce.consumer_portability_export.v4` 和 `open_commerce.consumer_portability_payload.v4`。继续受理完整配对的 V1、V2、V3 历史包。
2. V4 可为关系引用的来源商户携带最多 3 枚导出时有效的 RSA SPKI SHA-256 指纹。不携带公钥 PEM、私钥或商户内部数据。
3. 身份声明必须引用包内真实存在的商户关系，商户不得重复，指纹必须是去重的小写 64 位摘要。
4. 导入端只有在来源运营方包签名验证通过时，才使用包内指纹查找当前已发布商户。普通完整性包不产生可信候选。
5. 候选匹配的权威固定为 `trusted_operator_package_plus_matching_possession_key`，表示“可信来源包中记录的指纹与当前商户指纹一致”。
6. 消费者仍必须手工选择和确认候选。映射保存 `trusted_operator_key_match` 或 `not_verified` 及匹配指纹，便于后续审计。
7. 指纹匹配不恢复旧关系或 Grant，目标商户仍需审批一笔全新授权。

## 信任边界

- 指纹匹配证明两个节点在各自注册时能证明持有同一私钥，不证明当前工商主体、经营资质或实际控制人。
- 来源运营方签名只代表导出时对包内记录负责，不是全网统一信任根。
- 密钥撤销后不再产生新候选，历史映射仍保留当时证据。
- 本版未实现 DID、证书链、密钥轮换签名链或全网撤销同步。

## 实现入口

- `server/src/open_commerce_portability_model.rs`
- `server/src/open_commerce_portability_service.rs`
- `server/src/open_commerce_portability_adoption_service.rs`
- `server/src/open_commerce_portability_reauthorization_service.rs`
- `server/src/open_commerce_portability_identity_match_migration.rs`
- `pc-frontend/src/features/open-commerce/ConsumerPortabilityReauthorization.tsx`
