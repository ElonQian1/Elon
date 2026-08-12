---
title: 外部矿池 Adapter Artifact 签名密钥注册表验收
status: current
reviewed_at: 2026-08-12
owners: backend, security, ai-economy
implementation_status: implementation_partially_verified
---

# 外部矿池 Adapter Artifact 签名密钥注册表验收

## 已实现

- v230 新增 immutable key root、独立管理员 activation、不可逆 revocation 和派生 current view；
- 接受 2048..8192 位 SPKI/PKCS#1 RSA PEM，服务端统一为 SPKI LF，并以规范 DER 的 SHA-256 生成全局唯一 `key_id`；
- 登记者不能激活自己的 key；撤销后不能恢复，原 activation 只允许 exact replay；
- 三类写入都使用 `BEGIN IMMEDIATE`、JCS/SHA-256 摘要、稳定幂等键、服务端 UTC nanoseconds、JSON projection 和 no-update/no-delete 门卫；
- 管理 HTTP 支持登记、激活、撤销和 currentness，actor 只来自认证会话，回执不返回 PEM 或幂等材料；
- Store 内提供非序列化、不可复制的 exact active-key authority，供后续 signed provenance 在同一事务中重验。

## 验证结果

完整服务端检查通过：

```text
command: cargo check --manifest-path server/Cargo.toml -p elon-server --bin elon-server
fingerprint: 13824245bf10e506101f1fb762f3c1f118ac4d043b91fcb810a8f7aa45103aae
status: success
```

5 项定向 Rust/SQLite/Axum 测试通过：

```text
filter: external_pool_adapter_artifact_signing_key
fingerprint: c13d1ff10b7082364b62d9eb4df4fbb6c2ebcb29819545bbe87edd440cb20231
receipt: 08c87de1970cc528dcf350a5a9fbf077157720f1902870ff0ea800c230144125
status: success
```

覆盖 fresh/repeat migration、文件重开、三段状态机、登记和 activation exact replay、撤销后 activation 历史重放、同 DER 去重、四眼限制、双连接 activation 竞争、SQL update/delete 阻断，以及未登录、普通用户、admin、owner、未知字段、脱敏和 404 HTTP 行为。

## 仍未实现或验证

- v230 不读取、下载或验证 Artifact signature，也不绑定 v222 admission、v227 bytes、candidate ref、长度、SBOM 或 conformance；
- 没有签名 provenance receipt、生产 signer/KMS、证书链、密钥轮换 grace window 或自动撤销传播；
- 没有创建 Adapter、credential verifier、v213 route、worker、ACK、Runner、容量、派发、用量或结算；
- 未做真实 TCP、PC/MCP、生产数据库副本升级、生产密钥、部署、崩溃/断电和高并发压力验收。

因此 `active` 只能表述为“该 RSA 公钥当前具备后续验签资格”，不得表述为“Artifact 已可信”或“Adapter 可执行”。
