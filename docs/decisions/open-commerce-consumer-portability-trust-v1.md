---
title: 开放商业消费者自主管理运营方签名信任 V1
status: accepted
date: 2026-08-03
owners: backend, sdk, product
---

# 开放商业消费者自主管理运营方签名信任 V1

## 背景

隔离导入 V1 能证明数据包内容和摘要一致，但来源运营方只是用户填写的标签。若没有独立签名，接收方无法区分“文件未损坏”和“确实由某个已信任运营方签发”。平台也不能自封为全网身份权威，或替用户决定应该信任谁。

## 决定

1. 信任根由消费者自主建立。消费者在目标项目登记来源运营方 RSA 公钥，服务端将公钥规范化为 SPKI PEM，并以 SPKI DER 的完整 SHA-256 作为 `key_id`。
2. 仅接受 2048–8192 位 RSA 公钥和 `rsa-pkcs1v15-sha256`。公钥按当前用户、目标项目、来源运营方和 `key_id` 隔离；可追加轮换新密钥，旧密钥只能撤销，不能静默恢复。
3. 来源运营方用 SDK `signConsumerPortabilityPackage` 在本地私钥环境签名。私钥不上传平台。固定签名消息绑定协议版本、来源运营方、`key_id`、导出版本、包 ID、来源项目、幂等键、负载 SHA-256 和创建时间。
4. 带签名导入必须找到当前用户显式信任且仍有效的完全匹配公钥，并通过 RSA-SHA256 验证；算法、Key ID、公钥、来源标签或签名任一不匹配都失败关闭。
5. 通过签名的隔离快照标记为 `trusted_operator_signature_verified`。同一信封此前若按无签名方式导入，可在有效签名通过后只升级信任证明，不改写包内容。
6. 撤销公钥只阻止后续导入建立新信任，不追溯改写历史审计事实。历史状态表达“导入当时该密钥有效且签名通过”，不表达密钥当前仍有效。

## 边界

- 用户登记公钥不等于平台或政府认证该运营方真实身份，首次公钥分发仍需线下或其他可信渠道核验。
- 签名不证明包内商户响应、订单、金额、履约或支付是真实世界事实，只证明持有对应私钥的一方签过固定消息。
- 签名导入仍是隔离快照，不自动恢复关系、Grant、偏好、ERP、订单或结算。
- 当前没有证书颁发机构、透明日志、硬件密钥、远程证明、链上登记或全网信任联盟。

## 实现入口

- `server/src/open_commerce_portability_trust_model.rs`
- `server/src/open_commerce_portability_trust_service.rs`
- `server/src/store/open_commerce_consumer_portability_trust.rs`
- `server/src/open_commerce_portability_trust_api.rs`
- `server/src/open_commerce_portability_trust_migration.rs`
- `sdk/open-commerce-connector/src/portability-signature.js`
- `pc-frontend/src/features/open-commerce/ConsumerPortabilityTrustKeys.tsx`
- `pc-frontend/src/features/open-commerce/ConsumerPortabilityImports.tsx`
