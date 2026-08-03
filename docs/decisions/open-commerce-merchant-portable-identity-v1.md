---
title: 开放商业商户可携带身份 V1
status: accepted
date: 2026-08-03
owners: backend, pc, sdk, product
---

# 开放商业商户可携带身份 V1

## 背景

不同运营方中的商户 ID 不同，名称、地址和简介也可能重名或变更。只靠文本相似度无法安全判断两个商户节点是同一业务主体。因此先提供一种由商户自己持有私钥、可在不同环境重复证明的稳定公钥指纹。

## 决定

1. 商户在本地生成 3072 位 RSA 密钥，私钥只由商户保管，平台只接收公钥和持有证明签名。
2. 持有证明绑定协议版本、当前项目 ID、商户 ID 和公钥 SHA-256 指纹，防止把同一签名直接复用到其他商户。
3. 服务端验证 RSA PKCS#1 v1.5 + SHA-256 签名、公钥规格和指纹后才保存记录。每次读取再次检查摘要与持有证明。
4. 每个商户最多保留 3 个有效公钥，便于有限轮换。已撤销公钥保留历史并不可重新启用。
5. 商户发布开放目录后，目录只暴露有效公钥指纹、算法和验证时间，不暴露私钥。
6. 指纹相同只证明两个节点在注册时持有同一私钥，不等于工商认证、法人认证、域名认证或平台保证。

## 失败关闭边界

- 私钥遗失时平台不找回。商户只能撤销旧指纹并建立新身份。
- 公钥指纹不自动合并项目、商户、关系、Grant、ERP、订单或结算。
- 本批次未把指纹写入消费者可携带数据包，也未实现跨运营方自动候选匹配。这些由后续独立版本承接。
- PC 端的本地密钥仅保留在当前内存，刷新或关闭页面后不可恢复。

## 实现入口

- `server/src/open_commerce_merchant_identity_model.rs`
- `server/src/open_commerce_merchant_identity_service.rs`
- `server/src/store/open_commerce_merchant_identity.rs`
- `server/src/open_commerce_merchant_identity_api.rs`
- `pc-frontend/src/features/open-commerce/MerchantPortableIdentityPanel.tsx`
- `pc-frontend/src/features/open-commerce/merchantIdentityProof.ts`
- `sdk/open-commerce-connector/src/merchant-identity.js`
