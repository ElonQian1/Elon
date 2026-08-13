---
title: 外部矿池 Adapter Provider-specific 惰性 runtime launch profile 权威
status: current
reviewed_at: 2026-08-14
owners: backend, security, ai-economy
implementation_status: implementation_locally_verified
---

# 外部矿池 Adapter Provider-specific 惰性 runtime launch profile 权威

## 1. 唯一语义：launch profile 不是 launch 或 readiness

V255 只为 exact V254 activation candidate 追加一条 Provider-specific、服务端固定策略的 runtime launch profile。它把未来 Sidecar 可能消费的 executable、IPC、resolver、secret custody、probe、isolation、resource、network 与 timeout 约束封存为不可变摘要；状态固定为 `launch_profile_current_inert`，profile 效果固定为 `runtime_launch_profile_recorded_inert`，revocation 效果固定为 `runtime_launch_profile_revoked`。profile 与 revocation 的 `adapter_effect`、`runtime_effect`、`provider_effect`、`credential_effect`、`route_effect`、`execution_effect`、`usage_effect`、`market_effect` 和 `settlement_effect` 全部固定为 `none`。

profile 不执行 entrypoint，不创建进程、IPC session 或网络连接，不读取、解析或解析到任何 credential/config secret，不调用 resolver backend，不运行 probe，也不证明这些生产组件存在。它不激活 Provider，不创建 v213 Adapter/credential/authorization/service actor/route authority，不生成 runtime readiness，也不授权 Pool、Offer、Snapshot、Job、Reservation、Attempt、Start、usage 或 settlement。

V250 漏洞、V252 沙箱与 V253 credential re-attestation 是短 TTL 动态声明，与 durable launch policy 正交，既不是 V255 POST 输入，也不持久化进 profile。成功的 V255 currentness 只证明静态根和惰性 profile 当前，绝不能解释为 V254 dynamic preflight 当前或 activation/runtime/market ready。

## 2. 最窄请求与服务端策略

owner 与 platform admin 的 POST 都位于 exact Provider binding、candidate path 下。请求体仅接受：

- `expected_candidate_digest` 与 `expected_provider_binding_digest`；
- 服务端当前 catalog root `expected_launch_policy_digest`；
- 可选 `expected_predecessor` 对象；若存在则只含 `profile_id` 与 `profile_digest`，二者不可半空；
- `idempotency_key` 与 `confirm_runtime_launch_profile=true`。

调用方不得提交 policy 对象或其中任何字段，不得提交 Provider/release/installation/entrypoint/actor/route/resolver backend/credential locator/path/time/status/effect。策略 ID/revision及 executable、IPC、resolver、custody、probe、isolation、resource、network、timeout 和 limit 全由服务端固定 catalog 派生；authenticated session 决定 `recorded_by_actor_kind` 与 actor user ID。owner path 固定记录 Provider owner，admin path 固定记录 platform admin，不能由 body 自报或重放时漂移。

owner/admin 还提供同形只读 runtime-launch-policy GET。它返回 server-fixed policy 的公开 summary 与 `launch_policy_digest`；policy builder 不访问数据库，endpoint 只用 historical candidate audit target 做 auth/path ownership 且绝不写库。该 GET 不宣称 candidate/delegation current，current root 只在 fresh create/successor Store transaction 内重验。create 的 expected digest 必须来自这份当前服务端 policy，而不是调用方自行拼装 policy。

optional predecessor 是线性 successor CAS：首条 profile 的 pair 必须全空；已有 structural latest profile 时 fresh successor 必须精确引用其 ID/digest。latest 即使已撤销也可作为 predecessor 恢复新 profile，但 fresh successor 仍须重新通过 current V249 Prepared、V254 candidate/delegation 与 server policy；撤销只令旧 profile historical，不永久禁用 Provider。若要永久终止上线意图，应撤销 V254 delegation/upstream。旧 profile 保留为不可变历史但不再 current。fresh create/successor 的 expected root、predecessor、actor、policy 或幂等 material 任一漂移都失败关闭且零写入。exact replay 只验证同 actor/idempotency material 对应的历史 exact profile 与 fresh Prepared 字节，不因后来 upstream current head 或 server policy 换代而改写既有结果。

## 3. fresh filesystem audit 与原子 Store 门卫

Service 先用 candidate 的非序列化 audit target 校验 path binding/candidate 和 owner，然后重新打开 V249 sealed installation tree，取得 fresh `PreparedExternalPoolAdapterInstallation`；raw installation root、entrypoint path 与 retained handles 不穿过 HTTP。Store 在一个 `BEGIN IMMEDIATE` 内重新消费：

- exact current V249 registry Provider binding/release 与 fresh Prepared；
- exact current、未撤销且仍为 `candidate_current_not_activation_ready` 的 V254 candidate/delegation；
- exact Provider `registering` revision/digest/owner 与无 route projection collision；
- 服务端固定 launch-policy digest 与可选 current predecessor；
- actor-bound idempotency material。

Store 还从 historical onboarding 私下重建 credential subject，只允许 `vault_ref` scheme 且 commitment 必须与 V249/Prepared exact；raw locator 永不离开私有 authority，unsupported `gateway_ref` 等 scheme 失败关闭。Store 服务端派生 profile ID/digest/material digest、entrypoint path digest、sequence、timestamp、policy 与 fixed effects，然后原子追加 profile；current head 从不可变 lineage 派生，不另设可漂移的 mutable head 表。任何检查或持久化失败都不留下 profile 半状态。它不能消费公开 currentness JSON 作为 authority，也不能把先前 filesystem audit 或 V254 preflight 缓存到另一事务。

撤销同样是追加式 authority：body 仅接受 expected profile/candidate digest、reason、idempotency、confirmation。fresh revoke 只要求历史 profile/candidate 逐字 exact，且 profile 仍是结构上的 latest、未撤销 head；它不重新要求 upstream、filesystem 或 server policy current，避免失效 authority 无法被安全终止。Store 在同一事务追加唯一 revocation 并终止 head，不修改历史 profile，不触发 runtime 或下游状态；revoke exact replay 只消费历史 exact revocation material。

## 4. HTTP、权限与公开投影

owner 与 admin 使用同形路由：

- `POST /api/{me|admin}/compute/external-pool-provider-bindings/:provider_binding_id/activation-candidates/:candidate_id/runtime-launch-profiles`
- `GET /api/{me|admin}/compute/external-pool-provider-bindings/:provider_binding_id/activation-candidates/:candidate_id/runtime-launch-policy`
- `GET /api/{me|admin}/compute/external-pool-provider-bindings/:provider_binding_id/activation-candidates/:candidate_id/runtime-launch-profiles/:profile_id/currentness`
- `POST /api/{me|admin}/compute/external-pool-provider-bindings/:provider_binding_id/activation-candidates/:candidate_id/runtime-launch-profiles/:profile_id/revocation`

`me` 要求 authenticated user 是 binding owner；`admin` 要求平台角色 `admin|owner`。每个 handler 都校验 path binding/candidate/profile 的 exact ownership。fresh POST/revoke 为 `201`，exact replay 为 `200`；无会话 `401`、权限不足 `403`、语义非法 `400`、缺失 `404`、root/path/currentness/idempotency 冲突 `409`、malformed 或 unknown JSON `422`。

所有成功响应递归移除 raw entrypoint/path/installation locator、credential locator/ref/commitment、resolver backend root或 locator、actor kind/ID、幂等 scope/key、confirmation 与 private receipt JSON；`entrypoint_relative_path` 同样是私有 filesystem authority，绝不得公开。可公开稳定 `credential_ref_scheme`，但不可公开 locator 或 commitment。公开 entrypoint path digest、policy/profile/candidate/binding stable ID/digest 及 `adapter_effect=none`、`runtime_effect=none`、`usage_effect=none` 等 fixed effects 只用于审计和后续 exact currentness，不是可执行参数或 authority transfer。

## 5. V254 market fence 原样保留

V255 不激活 Provider，因此 V254 覆盖 `external_pool` CapacityPool active 与 Offer draft/active insert、update、version、direct SQL 及 Provider-kind legacy 绕行的 18 个 temporary absolute deny trigger 必须逐字保留。launch profile currentness 不能缩窄或替代这些 fence，也不能作为 market admission gate。

只有未来同一批真正实现 atomic Provider activation、route/service actor、secret resolver/transport readiness 与 runtime currentness 时，才可在同一 migration/transaction 边界把 absolute deny 原子替换为 external-pool 专用 admission gate；不能先删除、分批修改或只在 Service/HTTP 检查。legacy Provider 路径必须保持原语义。

## 6. 本地实现与验证状态

截至 2026-08-14，V255 Domain、migration、Store、Service 与 owner/admin HTTP 已通过 `elon-server` 定向测试：共命中 `12 passed / 0 failed / 1875 filtered out`。其中 7 项覆盖 fresh/repeat migration、DDL/Store ABI、完整 profile/policy 投影、exact roots、lineage/current view、不可变性以及 V254 18 个 absolute deny 的源码逐字保护；5 项覆盖 owner/admin 创建、策略公开投影、currentness、linear successor 修复、filesystem drift 后撤销与递归脱敏。正式验证指纹为 `e6919db4d7535bae1e8fc4017e1c7e829a3ad0ce23407e3c29c636a5557c0575`，收据摘要为 `e4a5779153d0f60de92d05d18e037ee80c2547223261b23f9030b796a1835da8`。

该证据只证明本机 Rust/SQLite 与进程内 Axum 定向链路，不包含真实 TCP、生产数据库、生产 secret resolver、Sidecar/IPC/transport、真实外部矿池、probe/Runner/ACK、Provider activation、market admission、可信计量或结算。`launch_profile_current_inert` 仍不得解释为 runtime ready 或 production ready。
