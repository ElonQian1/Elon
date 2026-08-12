---
title: 外部矿池 Adapter Artifact 签名来源证明权威
status: current
reviewed_at: 2026-08-12
owners: backend, security, ai-economy
design_status: design_frozen
implementation_status: implementation_partially_verified
---

# 外部矿池 Adapter Artifact 签名来源证明权威

## 1. 冻结结论

v231 闭合 v222 admission、v227 服务端持有字节与 v230 当前 active RSA signer key 之间的 exact 签名证明。平台只在同一 Store 写事务中重审三项权威并成功验证 `rsa-pkcs1v15-sha256` 后，保存一份不可变 signed-provenance receipt。

该 receipt 只证明“已登记来源运营方的对应私钥，对这份 exact admission、source receipt、候选引用摘要、Artifact SHA-256 和长度完成了有效签名”。它不解析候选引用，不证明文件格式、安全、SBOM、conformance 或业务能力，也不创建 Adapter、credential verifier、route、worker、Lease、派发、用量或结算权限。

## 2. Exact 签名绑定

签名消息固定为：

```text
"ELON-EXTERNAL-POOL-ADAPTER-ARTIFACT-SIGNATURE-MESSAGE-V1"
+ 0x00
+ JCS(exact signature binding)
```

binding 必须包含：

- v222 `admission_id/admission_digest`、Adapter id 和 release version；
- `candidate_artifact_ref` 的 domain-separated SHA-256，不保存或返回原始引用；
- v227 `source_receipt_id/source_receipt_digest`、重开后 Artifact SHA-256 和字节长度；
- v230 `key_record_id/key_record_digest/key_id`、来源运营方和固定签名算法。

签名挑战返回待签消息 Base64、消息摘要和公开 binding。调用方提交签名时必须回传 exact 消息摘要；任何 admission、source、key 或 currentness 漂移都会使 fresh write 失败。签名和 receipt 都使用规范 Base64、RFC 8785 JCS、SHA-256 与 domain separation。

## 3. 权威与线性化边界

fresh write 在 `BEGIN IMMEDIATE` 中按顺序完成：

1. exact 读取且要求 admission 当前仍为 `staged`；
2. exact 审计 v227 source receipt 与 admission、摘要和长度；
3. exact 读取且要求 v230 signer key 当前仍为 `active`；
4. 重新构造挑战并核对消息摘要；
5. 从注册表读取规范公钥并验证 RSA 签名；
6. 保存 receipt、执行 SQLite exact-authority/JSON/不可变触发器并 exact readback；
7. 提交事务。

Service 在挑战、fresh write 和 currentness HTTP 前还会重开 quarantine CAS 文件并完整重算摘要和长度。文件与 SQLite 不是同一事务；签名只绑定 v227 已证明的内容地址。未来任何 Artifact consumer 仍必须在采用字节时重新打开当前 CAS 文件，不能只凭历史 receipt 使用文件。

## 4. 不可变账本与 currentness

v231 新增：

- `compute_external_pool_adapter_artifact_signed_provenance_receipts`：每个 admission/source 一份不可变 receipt，内部保存验签所需原始签名；
- `compute_external_pool_adapter_artifact_signed_provenance_current`：从 admission 与 signer key 的 current view 派生 `verified_current|historical_only`。

admission 终态或 signer key 撤销后，历史 receipt 不删除、不改写，只转为 `historical_only`。同幂等 scope/key 的 exact replay 可以读回历史 receipt，但不能恢复 admission 或 key 权限；fresh write、挑战和后续采用必须失败关闭。

HTTP currentness 还要求 CAS 字节仍能安全重开并通过摘要/长度复核。数据库 current view 只表达数据库权威；对外服务不会在 blob 缺失、非普通文件或内容漂移时继续返回有效状态。

## 5. 管理 API 与脱敏

首版只开放 `admin|owner` HTTP：

- `POST /api/admin/compute/external-pool-adapter-release-admissions/:admission_id/artifact-signed-provenance/challenge`；
- `POST /api/admin/compute/external-pool-adapter-release-admissions/:admission_id/artifact-signed-provenance`；
- `GET /api/admin/compute/external-pool-adapter-release-admissions/:admission_id/artifact-signed-provenance`。

写请求使用 `deny_unknown_fields`，actor 只从认证会话派生，并要求 `confirm_external_pool_adapter_artifact_signed_provenance`。首写返回 `201`，exact replay 返回 `200`；未登录、非管理员、结构错误、未知对象和 lineage/currentness/签名冲突分别失败关闭。

API 回执不得返回原始 RSA 签名、公钥 PEM、原始 candidate ref、绝对文件路径、幂等 scope/key、私钥、Token、Cookie 或 KMS 明文。公开摘要只保留验证材料摘要、签名摘要和 domain-separated candidate-ref 摘要。

## 6. P0 禁线

- 禁止把 `verified_current` 称为 Artifact 已安全、已通过 conformance 或 Adapter 可执行；
- 禁止把管理员上传的 v227 bytes 误称为已解析 candidate ref 或远端供应链证明；
- 禁止仅按运营方名称、`key_id` 或历史 activation 猜测 signer authority；
- 禁止在 key 撤销或 admission 终态后产生新的签名证明或自动换用另一把 key；
- 禁止从 receipt 直接创建或修改 v213 Adapter、credential、route authorization 或 execution seal；
- 禁止改变 Provider、Pool、Offer、Job、Reservation、Attempt、usage、资金或结算；
- 禁止向普通用户、Provider owner、MCP、PC 或 SDK 暴露首版管理写入口。

## 7. 后续采用门卫

后续 Artifact verifier 或 Adapter registry 不能把 v231 receipt 当作最终 trust。它必须另行闭合：

1. 当前 CAS 字节与 v231 binding 的 exact 重开复核；
2. Artifact 固定格式与 manifest 精确检查；v232 已完成这一子门，SBOM 和安全扫描仍缺；
3. sandbox conformance 与六能力声明验证；
4. credential verifier registry/currentness/revocation；
5. 平台 release actor 与 Provider route actor 分权；
6. exact v213 Adapter/version 和 route authorization 的独立采用事务。

v231 永远保持 `artifact_format_effect=none`、`conformance_effect=none`、`adapter_effect=none` 和 `route_effect=none`。后续 v232 独立收据可表达 `static_format_verified`，不得回写或升级 v231。

## 8. 当前验收状态

本地已通过完整测试目标编译及 4 项定向 Rust/SQLite/Axum 验收，覆盖 migration 重放、真实 2048 位 RSA 签名、错误签名零写入、幂等重放、数据库重开、终态 stale challenge、key 撤销历史化、SQL update/delete 阻断、角色鉴权、HTTP 脱敏和 CAS 文件丢失失败关闭。证据见 [`external-pool-adapter-artifact-signed-provenance-acceptance.md`](external-pool-adapter-artifact-signed-provenance-acceptance.md)。

真实 TCP、生产数据库升级、生产 signer/KMS、并发压力、进程崩溃/断电、真实部署、MCP/PC 和后续 Artifact verifier/Adapter 采用仍未验证。
