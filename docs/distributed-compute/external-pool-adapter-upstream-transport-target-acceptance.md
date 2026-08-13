---
title: 外部矿池 Adapter upstream transport target 验收边界
status: current
reviewed_at: 2026-08-14
owners: backend, security, ai-economy
implementation_status: implementation_partially_verified
verification_status: local_target_verified
---

# 外部矿池 Adapter upstream transport target 验收边界

## 本批状态

V258 已随完整 `elon-server` 测试目标编译，并通过 11 项定向测试：7 项 migration/Store/ABI 合同、2 项源码边界合同和 2 项 owner/admin 进程内 Axum HTTP，`11 passed / 0 failed`。验证指纹为 `bb3faae9295d682d573a5bc5a1d608be711a18c021be273120bf5181b1312aac`。本地 HTTP 夹具使用全新 SQLite 并执行当前 migration；未执行 DNS、TLS、socket/network、process spawn、secret/config读取或交付、IPC/session、child handshake、no-work或 upstream probe、route/service actor、Provider activation、runtime、market、usage或 settlement。

## 已覆盖的本地正向矩阵

- 全新 SQLite 上执行当前 migration；migration/Store 源码合同固定 append-only/current view、V249/V254/V255 lineage、注册与完整 receipt 投影，并逐字检查 V254 18 个 deny trigger；
- owner/admin policy GET 返回 exact 同一 server-fixed policy/digest，零 DNS/TLS/network/write，且不把 historical path auth 误称为 profile currentness；
- fresh create 在单事务中消费 current V249/V254/V255、Provider、policy、fresh Prepared 与 optional structural latest predecessor，owner/admin fresh `201`，exact actor-bound replay `200`；
- body 唯一 target fields 是 canonical `dns_hostname`、nonzero `port`、`expected_tls_leaf_spki_sha256`；Store exact 派生相同 SNI、IDs、digests、sequence/time/status/effects；
- optional predecessor 全空创建首条、exact latest pair 创建 successor；已撤销 latest仍可在重新通过 current roots后作为 predecessor恢复新 target，旧 authority 保留 immutable history；
- currentness 重审 fresh Prepared、exact path/target digest、current profile/policy/head/revocation，返回 `broker_connect_ready=false`、`upstream_probe_observed=false`、`runtime_launch_ready=false`、`activation_ready=false`；
- revoke fresh `201`、exact replay `200`，在后来 upstream/profile/policy/FS drift 后仍可按 historical exact authority追加撤销；
- 全部公开响应递归隐藏 hostname、port、SNI、SPKI pin、actor、idempotency、confirmation、receipt JSON与任何 locator/path/secret，仅保留 stable IDs/digests、inert effects 和 false readiness。

## 仍待补充的动态矩阵

- 仍需 V257→V258 文件数据库原位升级、重复 migration、文件重开、并发 create/revoke、异常崩溃与生产数据库副本验收；
- 仍需扩大 HTTP 失败矩阵，覆盖 malformed/unknown JSON、所有非法 hostname/port/pin、path/root/currentness/idempotency 漂移和事务前后表计数；
- 仍需真实 TCP 但保持无 DNS/TLS 外呼的路由验收；真实 DNS/TLS/network 只允许在后续 broker observation 批次中实施，不能混入 V258。

所有失败场景必须比较事务前后 target/revocation 及所有下游表计数，证明零半写；不能用 response 中的 `none` 代替数据库差分。

## 源码与数据库零副作用合同

源码扫描必须拒绝 `std::process::Command`、`tokio::process`、socket/TCP、HTTP client、DNS resolver、TLS connector、secret resolver、bundle secret delivery、probe与 activation调用。V258 只允许 target/revocation自身发生预期追加；Provider 仍为 `registering`，V213 Adapter/credential/authorization/service actor/route 与 Pool/Offer/Job/Reservation/Attempt/Start/usage/settlement表必须零写。

migration 源码合同必须对 V254 18 个 temporary absolute deny trigger 做 exact source parity，而不是只断言数量。只有未来同一原子批次完成完整 admission gate，才可替换这些 fence。

## 尚缺的 runtime/session companion

V258 target 不足以执行真实 authenticated no-work upstream probe。V255 UTF-8 JCS frame 与 V256 arbitrary secret bytes 不兼容；1 MiB config 也不能用 base64塞回同一 1 MiB frame。下一批仍需 server-fixed supervisor/session policy companion，明确 binary sensitive frames、KDF/key custody、nonce/sequence/transcript、timeouts/limits、isolation/egress、failure shutdown与child reap；其后才是 child-only IPC，再之后才是绑定 V258 authority 的 broker DNS/TLS observation与 upstream probe。

因此本批不得宣称 production transport、authenticated session、secret-safe framing、probe、readiness或activation已验收；当前允许状态是 `implementation_partially_verified / local_target_verified / 11 passed`。
