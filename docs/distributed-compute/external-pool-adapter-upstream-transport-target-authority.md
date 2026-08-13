---
title: 外部矿池 Adapter upstream transport target 惰性权威
status: current
reviewed_at: 2026-08-14
owners: backend, security, ai-economy
implementation_status: implementation_partially_verified
---

# 外部矿池 Adapter upstream transport target 惰性权威

## 1. 唯一语义：连接目标，不是连接或探测证据

V258 为 exact V255 runtime launch profile 追加 Provider-specific、append-only 的 upstream transport target。它补齐此前 V221 onboarding、V256 bundle manifest 与 V257 capsule 均没有的 transport target authority：canonical DNS hostname、nonzero TCP port、expected TLS leaf SPKI SHA-256 pin，以及 server-fixed broker transport policy root。

这条 durable authority 只回答“未来受管 broker 被允许尝试连接哪个目标、期望验证哪个 TLS identity”。它不解析 DNS，不声明 DNS answer 或 IP，不开 socket，不执行 TLS，不观察 certificate validity，不运行 upstream no-work probe，不启动 Adapter/Sidecar，不建立 IPC session，不读取或交付 config/credential secret，也不生成 route、service-actor authorization、Provider activation、runtime readiness、market admission、usage 或 settlement。target 固定 `upstream_transport_target_current_inert`；`broker_connect_ready`、`upstream_probe_observed`、`runtime_launch_ready`、`activation_ready` 全为 `false`，所有下游 effect 固定为 `none`。

V258 因而是 authenticated upstream probe 的必要前置根，却不是充分实现。只有 hostname/port/SPKI pin 的 durable authority 存在后，未来 broker 才可能诚实证明它解析、连接并认证了正确目标；本批没有 transport observation，不能把 caller-declared pin 或 inert currentness 描述成真实 TLS 身份验证。

## 2. server-fixed transport policy 与 privileged 输入

owner/admin policy GET 位于 exact binding/candidate/profile path，返回同一 server-fixed policy summary 和 `policy_digest`；调用方在 create body 中将它作为 `expected_target_policy_digest` 回传。policy 固定 server-owned broker、brokered TLS/TCP、canonical lowercase DNS hostname（拒绝 IP literal）、explicit nonzero port、fresh A/AAAA 与 public-unicast address gate、TLS 1.3、exact hostname/SNI、未来 broker 在 connect 时使用当时 server WebPKI trust roots 验证 chain、hostname 与 certificate time，再匹配 expected leaf SPKI pin、no proxy/redirect/0-RTT/client certificate、Sidecar no-network，以及 DNS/connect/TLS timeout 与 answer/attempt limits。V258 只封存这些 future-observation requirements；GET 不解析 DNS、不连接目标，也不证明 profile current，fresh create/currentness 才重验 current roots。

owner/admin create body 只能接受：

- `expected_profile_digest`、`expected_candidate_digest`、`expected_provider_binding_digest`、`expected_target_policy_digest`；
- `draft`，且只含 `dns_hostname`、`port`、`expected_tls_leaf_spki_sha256`；
- optional `expected_predecessor`，且只含 `target_id` 与 `target_digest`；
- `idempotency_key` 与 `confirm_upstream_transport_target=true`。

body 不得自报 actor、actor kind/ID、时间、状态、effect、policy object/field、SNI、DNS result、IP address、certificate validity、observed certificate、network outcome、probe outcome、route/service actor 或 readiness。`tls_server_name` 只能由 Store exact 派生为 canonical hostname。SPKI 是未来握手必须匹配的 expected pin，不是 observed certificate proof。authenticated session 决定 owner/admin actor，服务端决定时间、sequence、stable IDs/digests、policy 与 fixed inert effects。

首条 target 的 predecessor pair 必须全空；fresh successor 必须 exact 引用 structural latest target 的 ID/digest，形成单线 CAS。latest 即使已撤销，也可在 current profile及上游 roots 重新通过后成为新 successor 的 predecessor。exact create replay 只恢复相同 actor-bound historical receipt，不被后来 target/policy head 漂移改写。fresh revoke 只接受 expected target/profile digest、reason、idempotency 与 confirmation；它必须保持在 upstream、filesystem 或 policy 后续失效时仍可安全追加撤销，且不触发网络或下游状态。

## 3. Store 权威与 currentness

Service 用 path 中 binding/candidate/profile 找到私有 audit target，校验 exact ownership，并从 retained installation authority 重新形成 fresh Prepared。Store 在单一 `BEGIN IMMEDIATE` 边界重验 current V249 registry/installation、V254 candidate/delegation、V255 profile、Provider registering revision/digest/owner、server-fixed target policy、linear predecessor、actor-bound idempotency 与 canonical hostname/port/pin，然后服务端派生 target ID/digest/material digest、SNI、sequence/time/effects并追加 immutable receipt。任何漂移均失败关闭且零写入；HTTP summary 不能反向作为 Store authority。

currentness 重新消费 exact target digest 与 fresh Prepared，并要求 target 是未撤销 structural latest、profile/current policy与全部 lineage 当前。成功只返回 inert currentness；它不做 DNS/TLS/network observation。revoke 追加 immutable revocation，不修改 target，不永久阻断日后以已撤销 latest 为 predecessor创建新 target。

## 4. HTTP、权限与公开投影

owner/admin 路由同形：

- `GET /api/{me|admin}/compute/external-pool-provider-bindings/:provider_binding_id/activation-candidates/:candidate_id/runtime-launch-profiles/:profile_id/upstream-transport-policy`
- `POST /api/{me|admin}/compute/external-pool-provider-bindings/:provider_binding_id/activation-candidates/:candidate_id/runtime-launch-profiles/:profile_id/upstream-transport-targets`
- `GET /api/{me|admin}/compute/external-pool-provider-bindings/:provider_binding_id/activation-candidates/:candidate_id/runtime-launch-profiles/:profile_id/upstream-transport-targets/:target_id/currentness`
- `POST /api/{me|admin}/compute/external-pool-provider-bindings/:provider_binding_id/activation-candidates/:candidate_id/runtime-launch-profiles/:profile_id/upstream-transport-targets/:target_id/revocation`

`me` 只允许 binding owner，`admin` 只允许 platform `admin|owner`。fresh create/revoke 返回 `201`，exact replay 返回 `200`；无会话 `401`、无权限 `403`、语义非法 `400`、缺失 `404`、root/path/currentness/idempotency 冲突 `409`、malformed/unknown JSON `422`。

成功响应递归删除 `dns_hostname`、`port`、`tls_server_name`、`expected_tls_leaf_spki_sha256`、actor kind/ID、idempotency scope/key、confirmation 与 raw `receipt_json`。也不得泄露 DNS answers/IP、certificate、credential/config locator或 bytes、installation/entrypoint filesystem path。可公开 target/policy/profile/candidate/binding/provider stable IDs/digests、sequence/status、fixed inert effects 与 false readiness；这些只供审计/CAS，不传递 endpoint 或执行 authority。

## 5. 与 V255/V256/V257 及后续 session 的边界

V255 的 `u32_be_length_prefixed_utf8_jcs_v1` 只适用于 canonical JCS control frame，不能直接封装 V256 的 arbitrary config/credential bytes：这些 bytes 未承诺 UTF-8，且把 1 MiB config base64 后会超过 V255 1,048,576-byte frame limit并制造额外 secret string copies。V258 不修改 V255、不传 secret，也不能把这一 framing 名称解释为已有 sensitive-frame wire ABI。

下一批必须先增加 durable、server-fixed supervisor/session policy companion，明确 control 与 binary-sensitive frame 的双层 framing、长度上限、streaming authentication、KDF/key custody、nonce/sequence/transcript binding、timeout、resource/isolation/egress及 shutdown/reap。随后才能实现 authenticated child-only IPC no-work handshake；只有再由 server broker 按 V258 target 执行 DNS/TLS 并生成 authenticated observation，才可宣称真实 upstream probe。以上均不属于 V258。

V254 的 18 个 temporary absolute market deny trigger 必须名称与 SQL body逐字保留。V258 currentness 不能替代完整 admission gate；在 atomic activation、route/service actor、secret custody、broker/session/probe/runtime currentness 同批实现前，不得删除或缩窄这 18 项 fence。

## 6. 当前验证强度

V258 已随完整 `elon-server` 测试目标编译，并通过 11 项定向测试：7 项 migration/Store/ABI 合同、2 项源码边界合同和 2 项 owner/admin 进程内 Axum HTTP。验证指纹为 `bb3faae9295d682d573a5bc5a1d608be711a18c021be273120bf5181b1312aac`。本地 HTTP 夹具会在全新 SQLite 上执行当前 migration，但尚未单独覆盖 V257→V258 文件升级、重复 migration、文件重开、并发/崩溃、真实 TCP、DNS、TLS、公网或生产部署；因此状态只提升为 `implementation_partially_verified`，不代表 transport、probe、runtime 或 activation ready。
