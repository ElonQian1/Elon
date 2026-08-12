---
title: 外部矿池 Adapter 凭据验证器身份权威
status: current
reviewed_at: 2026-08-12
owners: backend, security, ai-economy
implementation_status: implementation_partially_verified
---

# 外部矿池 Adapter 凭据验证器身份权威

## 1. 目的和边界

V241 为 V222 `expected_credential_verifier` 增加独立、非秘密的实现身份注册表。平台管理员登记 `verification_kind + verifier_id + verifier_revision + verifier_digest`，另一名管理员激活，之后只能追加撤销。它回答“平台认可哪一个精确验证器实现”，不回答“某份外部矿池凭据是否有效”。

V237 管理的是签署 V239 沙箱报告的 RSA 信任密钥；V241 管理的是未来检查外部矿池端点凭据的验证器实现身份。二者用途、生命周期和存储均独立，不得复用或互相替代。

## 2. 不可变身份

同一 `verification_kind + verifier_id + verifier_revision` 只能绑定一个 SHA-256 `verifier_digest`。实现发生任何变化都必须增加 revision，不能覆盖旧记录。注册、激活与撤销均保存 RFC 8785 JCS 投影摘要、幂等键、管理员身份和纳秒 UTC 时间；数据库触发器拒绝更新、删除、`INSERT OR REPLACE` 身份碰撞、JSON 投影漂移、自我激活和未激活先撤销，文本主键也显式拒绝空值。

状态只有：

- `pending_activation`：已登记，但不能被后续验证流程采用；
- `active`：通过双人审批，可供后续凭据验证回执阶段精确引用；
- `revoked`：历史记录保留，fresh 验证与采用必须失败关闭。

## 3. 管理接口

仅平台 `admin|owner` 可调用：

- `POST /api/admin/compute/external-pool-adapter-credential-verifiers`
- `POST /api/admin/compute/external-pool-adapter-credential-verifiers/:verifier_record_id/activate`
- `POST /api/admin/compute/external-pool-adapter-credential-verifiers/:verifier_record_id/revoke`
- `GET /api/admin/compute/external-pool-adapter-credential-verifiers/:verifier_record_id/currentness`

响应返回实现坐标、记录摘要和生命周期，不返回幂等材料，也不存在 credential、bearer、token、secret、公钥或验证回执字段。

## 4. 明确没有实现

V241 不存储或读取端点 bearer，不连接 KMS，不执行验证器，不签发 credential verification receipt，不采用 Adapter，不写 v213 route，不启动 worker/ACK，也不产生派发、计量或结算效果。

V242 已独立建立与精确 active V241 实现绑定的签名公钥权威，详见 `external-pool-adapter-credential-verifier-key-authority.md`。下一阶段 V243 仍必须建立限时凭据验证挑战与回执：只绑定非 bearer 的凭据查找引用承诺、精确 V241/V242 当前权威、验证结果、到期时间和撤销语义。随后 Adapter adoption 事务才能同时采用精确 V222 admission、V227-V240 制品证据和 fresh 凭据验证回执；任何历史记录都不能单独授权执行。
