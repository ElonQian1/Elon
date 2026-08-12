---
title: 外部矿池 Adapter Artifact 签名密钥注册表权威
status: current
reviewed_at: 2026-08-12
owners: backend, security, ai-economy
design_status: design_frozen
implementation_status: implementation_partially_verified
---

# 外部矿池 Adapter Artifact 签名密钥注册表权威

## 1. 冻结结论

v230 建立平台管理的 Artifact signer 信任密钥注册表。它只回答：某个来源运营方的 exact RSA 公钥是否经过两个不同平台管理员登记和激活，以及该信任根当前是否已撤销。

注册表不读取、下载或验签 Artifact，不解析 `candidate_artifact_ref`，不验证 SBOM、供应链或 conformance，也不创建 Adapter、credential verifier、route、worker 或执行权限。`active` 只表示该公钥可以被后续签名 provenance producer 采用，不表示任何候选 Artifact 已可信。

## 2. 状态机与四眼要求

每个规范公钥形成一份 immutable key root，状态从账本派生：

```text
pending_activation -> active -> revoked
```

- `pending_activation`：一名当前 `admin|owner` 登记来源运营方和规范 RSA 公钥；
- `active`：另一名当前 `admin|owner` 精确激活同一 key root；
- `revoked`：任一当前 `admin|owner` 追加不可逆撤销回执。

`activated_by_admin_user_id` 必须不同于 `created_by_admin_user_id`。撤销不删除 key root 或 activation，不允许恢复；继续信任必须登记另一把公钥。相同 DER 公钥的 SHA-256 `key_id` 全局唯一，不能借改名或换 PEM 排版重复登记。

## 3. 公钥与算法合同

首版只接受：

- `algorithm=rsa-pkcs1v15-sha256`；
- 2048 至 8192 位 RSA 公钥；
- SubjectPublicKeyInfo 或 PKCS#1 PEM 输入，服务端统一规范为 SPKI LF PEM；
- `key_id=sha256(canonical SPKI DER)`，固定 64 位小写十六进制；
- `source_operator` 为 trim 后 1..160 字符且不含控制字符。

公钥不是秘密，但 HTTP 回执不返回 PEM；私钥、证书私钥、Token、Cookie、API key、签名 URL 和 KMS 明文一律拒绝进入该注册表。v230 不接受调用方提供 `key_id`、actor、状态或服务端时间。

## 4. 不可变账本与 current view

v230 只新增：

1. `compute_external_pool_adapter_artifact_signing_keys`：immutable key root、规范公钥和登记回执；
2. `compute_external_pool_adapter_artifact_signing_key_activations`：每个 key root 最多一份独立管理员 activation；
3. `compute_external_pool_adapter_artifact_signing_key_revocations`：每个已激活 key root 最多一份 append-only revocation；
4. `compute_external_pool_adapter_artifact_signing_key_current`：派生 `pending_activation|active|revoked`，同时保留 root、activation 和 revocation 摘要。

三类写账本都使用 RFC 8785 JCS、SHA-256、domain-separated digest、稳定幂等 scope/key、服务端 UTC nanoseconds、JSON exact projection、no-update、no-delete 和 no-replace 门卫。创建、激活、撤销和 exact replay 都必须在 `BEGIN IMMEDIATE` 的线性化点重审来源和 currentness。

## 5. 管理 API

首版只开放平台管理员 HTTP，不开放 Provider owner、MCP、PC 或 SDK：

- `POST /api/admin/compute/external-pool-adapter-artifact-signing-keys`：登记 key root；
- `POST /api/admin/compute/external-pool-adapter-artifact-signing-keys/:key_record_id/activate`：由另一管理员激活；
- `POST /api/admin/compute/external-pool-adapter-artifact-signing-keys/:key_record_id/revoke`：不可逆撤销；
- `GET /api/admin/compute/external-pool-adapter-artifact-signing-keys/:key_record_id/currentness`：读取脱敏 current view。

所有写 body 必须 `deny_unknown_fields`。actor 从认证会话派生。登记、激活和撤销分别要求：

- `confirm_external_pool_adapter_artifact_signing_key_registration`；
- `confirm_external_pool_adapter_artifact_signing_key_activation`；
- `confirm_external_pool_adapter_artifact_signing_key_revocation`。

撤销原因必须 trim 后 8..2000 字符。首写返回 `201`，exact replay 返回 `200`；未知对象返回 `404`，角色失败返回 `401/403`，材料或 currentness 冲突返回 `409`，结构或确认错误返回 `400`。

## 6. 并发、幂等与恢复

- 同 scope/key、同 exact 请求返回原回执；任一材料变化必须冲突；
- 不同 key 并发登记同一 DER 公钥只能有一个成功；
- 创建者与激活者竞争或同一 actor 尝试自激活必须失败；
- activation 与 revocation 按提交顺序串行；未激活 key 不能撤销；
- 已撤销 key 的 activation replay只允许 exact 历史读回，不恢复 `active`；
- 进程重启和 migration 重放不得改变任何摘要、actor 或派生状态。

## 7. v231 signed provenance consumer

v231 signed provenance producer 已在验证 RSA 签名前和保存 receipt 的同一 Store 事务中重新读取 exact `key_record_id/key_record_digest/key_id`，并要求 current view 仍为 `active`。签名验证绑定 exact admission、v227 source receipt、`candidate_artifact_ref` 摘要、Artifact 字节摘要和长度；只按 `key_id`、运营方名称或历史 activation 猜测信任会失败关闭。权威边界见 [`external-pool-adapter-artifact-signed-provenance-authority.md`](external-pool-adapter-artifact-signed-provenance-authority.md)。

key 随后撤销时，v231 保留历史签名 receipt 并派生为 `historical_only`；不得自动恢复、自动切换另一把 key 或继续产生 Adapter/route authority。

## 8. P0 禁线与非目标

- 禁止把 Artifact signer key 与 v222 的 `expected_credential_verifier` 混为一谈；前者验证发布方签名，后者是未来外部矿池凭据验证意图；
- 禁止把 `active` key 称为 Artifact 已验签、provenance 已证明或 Adapter 已可信；
- 禁止在 v230 中保存或验证 Artifact signature、解析候选引用、联网、解压、加载或执行代码；
- 禁止创建或修改 v213 Adapter/version、credential、route authorization、capability 或 seal；
- 禁止改变 Provider、Pool、Offer、Job、Attempt、usage、资金或结算；
- 禁止用 fixture、公钥存在性或管理员身份代替后续真实签名验证；
- 禁止因 key 撤销删除历史 receipt 或自动撤销已派发任务；后续 consumer 必须各自实现 currentness 门卫。

## 9. 验收门槛

升级为 `implementation_partially_verified` 至少需要：完整服务端编译，fresh/repeat/旧库 upgrade、两次文件重开、三段状态机、四眼激活、exact replay、并发竞争、撤销 currentness、SQL 不可变门卫，以及管理员/owner/普通用户/未登录 HTTP 专项。真实 TCP、生产数据库升级、生产部署、MCP、PC、签名 provenance 和 Artifact 采用仍须单独报告。

上述本地门槛已于 2026-08-12 通过 5 项定向测试和完整服务端编译，状态升级为 `implementation_partially_verified`。证据和未覆盖范围见 [`external-pool-adapter-artifact-signing-key-acceptance.md`](external-pool-adapter-artifact-signing-key-acceptance.md)。
