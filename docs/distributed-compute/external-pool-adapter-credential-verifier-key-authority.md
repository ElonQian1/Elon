---
title: 外部矿池 Adapter 凭据验证器签名公钥权威
status: current
reviewed_at: 2026-08-12
owners: backend, security, ai-economy
implementation_status: implementation_partially_verified
---

# 外部矿池 Adapter 凭据验证器签名公钥权威

## 1. 目的和边界

V242 为精确且当前 active 的 V241 凭据验证器实现登记独立 RSA 签名公钥。它回答“未来的凭据验证报告应由哪把公钥验签”，不回答“某份外部矿池凭据是否有效”。公钥登记不会读取 credential、连接外部端点、签发验证回执、采用 Adapter 或授权执行。

V242 与 V237 沙箱验证者公钥用途不同，且数据库门卫禁止与供应方签名、漏洞扫描、沙箱验证公钥复用。V241 实现登记管理员不能为该实现登记公钥；另一名平台管理员必须绑定完整的 `verifier_record_id + verifier_record_digest + verification_kind + verifier_id + verifier_revision + verifier_digest`。

## 2. 不可变生命周期

RSA 公钥接受 SPKI 或 PKCS#1 PEM 输入，服务端统一规范化为 SPKI PEM，并以 DER SHA-256 生成 `key_id`。只支持 `rsa-pkcs1v15-sha256` 和 2048 至 8192 位密钥。根记录不可更新、删除或替换；轮换必须新增公钥，撤销只能追加一次不可变回执。

当前状态为：

- `active`：V241 精确实现仍 active，且公钥没有撤销；
- `revoked`：公钥已有追加式撤销；
- `verifier_not_current`：绑定的 V241 实现已失效，公钥随父权威失败关闭。

私有 Store current authority 只允许后续 V243 在同一 SQLite 连接中精确消费公钥和父实现当前性，不是 HTTP DTO，也不能单独授权执行。

## 3. 管理接口

仅平台 `admin|owner` 可调用：

- `POST /api/admin/compute/external-pool-adapter-credential-verifier-keys`
- `POST /api/admin/compute/external-pool-adapter-credential-verifier-keys/:key_record_id/revoke`
- `GET /api/admin/compute/external-pool-adapter-credential-verifier-keys/:key_record_id/currentness`

响应只返回公钥指纹、精确父实现坐标、记录摘要和生命周期，不返回 PEM、幂等材料、credential、credential ref、bearer、token 或 secret。

## 4. 下一阶段

V243 才能创建服务器派生的限时挑战，绑定非 bearer 凭据定位引用的承诺、精确 onboarding/application、admission、V241 实现和 V242 公钥，并验签生成可撤销、可过期的 credential verification receipt。V242 本身不能表述为“凭据已经验证”。
