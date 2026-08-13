---
title: 外部矿池 Provider 激活候选与 owner delegation 权威
status: current
reviewed_at: 2026-08-13
owners: backend, security, ai-economy
implementation_status: implementation_uncompiled
---

# 外部矿池 Provider 激活候选与 owner delegation 权威

## 1. 唯一语义：candidate 不是 activation

V254 只追加 owner-issued platform service actor delegation 与 Provider-specific 静态兼容候选。它复用 exact V249 Provider binding，并在写入前重新打开 sealed installation tree，取得 fresh `PreparedExternalPoolAdapterInstallation`。成功后 Provider 必须仍是原 revision 的 `registering`；`candidate_current_not_activation_ready` 只表示静态根当前，不表示 Adapter、route、market 或 runtime 已可用。

V254 不创建或写入 v213 Adapter/version、credential、authorization/capability/seal、service actor authorization、route 或 Start outbox。delegation 保存的是未来原子激活可能消费的 owner 意图；它不是 v213 runtime authorization。静态 compatibility digest 不是已创建 route，也不证明 secret resolver、KMS/gateway、Sidecar transport、Runner、ACK/event、可信计量或结算存在。

因此 V254 不写 `active`，不创建 CapacityPool、Offer、Snapshot、Job、Reservation、Execution Plan、Attempt、Start send/ACK、usage 或 settlement。任何把 candidate、delegation、preflight `200` 或 `inputs_current` 描述为生产可用、市场可售或可派发的实现都是权限升级缺陷。

## 2. 原子 candidate 写入

owner POST 只接受 path 中的 Provider binding，加上 expected V249 binding/release digest、幂等键和显式确认。请求体 `deny_unknown_fields`，不得接受 actor ID/kind/phase、projection ID、Provider ID、route ID、logical binding digest 或任何时间。Store 在一个 `BEGIN IMMEDIATE` 中重新消费 exact V249 current authority 与 Prepared，服务端派生 delegation/candidate/service actor/compatibility digest/sequence/timestamp，然后一起追加 delegation 和 candidate；任一步失败均零写入。

candidate 不保存 V250、V252、V253 的 receipt ID/digest 或 TTL head。它固定保存 `activation_closure_status=activation_closure_not_implemented`；当前状态即使为 `candidate_current_not_activation_ready`，`activation_ready` 也必须为 false。owner 撤销追加唯一 revocation，不改写历史 candidate/delegation，不触发 Provider、route、market 或 runtime 副作用。

## 3. currentness 与 dynamic preflight

static currentness 每次从 candidate 的非序列化 audit target 取得 installation binding，重新审计同一内容寻址目录，再由 Store 检查 exact V249 roots、原 registering Provider revision/digest、owner、未撤销 delegation、无 route projection collision 和 closure 未实现。owner API 必须先比对 authenticated user 与 target owner；admin API 只允许平台 `admin|owner`。path binding 与 target binding 不一致固定冲突。

dynamic preflight 在一次 Store 调用内使用同一个规范 `checked_at` 消费：

- fresh V249 Provider binding current authority 与 live-FS Prepared；
- exact current V250 vulnerability re-attestation；
- exact current V252 sandbox re-attestation；
- exact current V253 Provider credential re-attestation；
- 未撤销的 static candidate/delegation。

成功只返回 `inputs_status=inputs_current`、`activation_closure_status=activation_closure_not_implemented`、`activation_ready=false`。这份公开 receipt 不是 Store-private current authority，不能缓存后带入另一事务，也不能使 Provider active。dynamic heads 不进入 candidate 持久化，因此过期、撤销、successor 或 root 漂移会在下一次 preflight 立即失败关闭。

## 4. HTTP、鉴权与脱敏

owner surface：

- `POST /api/me/compute/external-pool-provider-bindings/:provider_binding_id/activation-candidates`
- `GET /api/me/compute/external-pool-provider-bindings/:provider_binding_id/activation-candidates/:candidate_id/currentness`
- `GET /api/me/compute/external-pool-provider-bindings/:provider_binding_id/activation-candidates/:candidate_id/preflight`
- `POST /api/me/compute/external-pool-provider-bindings/:provider_binding_id/activation-delegations/:delegation_id/revocation`

admin 只有 currentness/preflight GET，位于同形 `/api/admin/compute/external-pool-provider-bindings/...`；admin 不能替 owner 创建或撤销 delegation。fresh write 为 `201`，exact replay 为 `200`；无会话 `401`、非 owner/非 admin `403`、语义非法 `400`、对象缺失 `404`、root/currentness/path/idempotency 漂移 `409`、malformed/unknown JSON `422`。历史或已撤销 candidate 的 currentness 是 `409`，不是可用的历史读取。

所有成功响应递归移除 service actor ID、route projection ID、owner/actor 字段、幂等 scope/key、confirmation、credential locator/ref、receipt JSON 与本机路径。公开 candidate ID/digest 只是调用 currentness/preflight/revoke 所需的不透明历史坐标，不是 route、actor 或 execution authority。

## 5. 下一批 atomic activation 的最小门卫

V254 之后仍是 NO-GO。下一批若要推进 `registering -> active`，必须在同一 `BEGIN IMMEDIATE` 内重新消费 V249 Prepared、同次 current V250/V252/V253、未撤销 candidate/delegation、精确 owner-issued service actor、Provider-specific v213 compatibility/route authority，并原子创建所有 runtime authority与 Provider 新版本。secret resolver、sidecar/transport、Start send/ACK 生产实现没有 readiness/currentness 证明时不得激活。

V254 已安装 temporary absolute deny：`external_pool` CapacityPool 的 `active` insert/update/version，以及 Offer 的 `draft|active` insert/update/version 均由数据库触发器失败关闭，并覆盖 direct SQL；Provider kind 进入或离开 `external_pool` 也固定失败，避免借 legacy 身份绕过。下一 atomic activation 批不得简单删除这些 fence，而必须在同一事务显式替换为完整 admission gate：验证 Provider exact current `active` revision/digest、activation authority current、runtime readiness current、route/credential/service actor current 后，才允许 CapacityPool 从任何前置状态进入 `active`，或 Offer 从任何前置状态进入 `draft|active`。legacy Provider 继续走原路径；新 gate 只在 `provider_kind=external_pool` 时启用。直接 SQL、已有 Offer 状态推进、version/replay 都必须由约束/触发器或同一 Store kernel 覆盖，不能只在 HTTP 层检查。
