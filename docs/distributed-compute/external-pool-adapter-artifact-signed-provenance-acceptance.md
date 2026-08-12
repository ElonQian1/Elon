---
title: 外部矿池 Adapter Artifact 签名来源证明验收
status: current
reviewed_at: 2026-08-12
owners: backend, security, ai-economy
implementation_status: implementation_partially_verified
---

# 外部矿池 Adapter Artifact 签名来源证明验收

## 已实现

- v231 immutable signed-provenance receipt 与 `verified_current|historical_only` 派生 view；
- exact v222 admission、v227 source receipt、candidate-ref 摘要、Artifact SHA-256/长度和 v230 active key 的签名绑定；
- domain-separated JCS 挑战、真实 `rsa-pkcs1v15-sha256` 验签、消息/签名/材料/receipt 摘要；
- `BEGIN IMMEDIATE` fresh write、SQLite exact-authority/JSON projection/no-update/no-delete/no-replace 门卫和 exact readback；
- challenge、record 和 currentness 前的 quarantine CAS 字节重开与全量哈希；
- admin/owner challenge、record、currentness HTTP，响应不泄露原始签名、公钥 PEM、candidate ref、路径或幂等材料。

## 验证结果

最终定向命令：

```text
cargo test --manifest-path server/Cargo.toml --bin elon-server signed_provenance --no-fail-fast --locked --offline
fingerprint: dedb895723ae77d4fca0735210e89e8c4ec9124bc8fa562da74a9681a99affd9
evidence: D:\rust\shared\rust-cache-v2\validation-v1\evidence\dedb895723ae77d4fca0735210e89e8c4ec9124bc8fa562da74a9681a99affd9\summary.json
status: success
```

结果为 `4 passed; 0 failed`，覆盖：

- v231 fresh/repeat migration；
- 真实 2048 位 RSA 正确签名、错误签名零写入、exact replay 和 SQLite 文件重开；
- signer key 撤销后 currentness 转为 `historical_only`，新挑战失败，历史 receipt 不改写；
- admission 在挑战后进入终态时 fresh write 失败；
- SQL update/delete 阻断；
- HTTP 未登录、普通成员、admin/owner、脱敏、创建/重放/currentness；
- 当前 CAS blob 删除后 currentness HTTP 返回冲突，不误报 `verified_current`。

测试目标的编译同时覆盖 v231 生产代码。廉价门禁结果为 `git diff --check`、`SOURCE_SIZE_GUARD=passed` 和 Rust format 通过。

## 仍未实现或验证

- 没有验证 Artifact 格式、manifest、SBOM、安全扫描、sandbox conformance 或六项业务能力；
- 没有 credential verifier registry、Adapter registry、v213 route authority、worker/ACK、Runner 或真实派发；
- 没有真实 TCP、PC/MCP、生产数据库副本升级、生产 signer/KMS、真实外部矿池、部署、崩溃/断电和高并发压力证据；
- 没有改变 Provider、Pool、Offer、Job、Reservation、Attempt、用量、资金或结算。

因此 v231 只能表述为“来源运营方注册私钥对 exact Artifact 绑定的签名已验证”，不能表述为“Artifact 已可信”或“Adapter 已可执行”。
