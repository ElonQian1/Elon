---
title: UserNode Provider Binding Root 权威
status: current
reviewed_at: 2026-08-21
owners: backend, security, node-agent, ai-economy
design_status: design_frozen
implementation_status: implementation_uncompiled
verification_status: source_review_only
---

# UserNode Provider Binding Root 权威

## 1. 唯一结论与当前现实

V279 只把一个用户明确授权的节点安装身份与一个既有 `user_node` Provider 固定为不可变、
一对一的 identity binding。它关闭此前 `node_binding_ref` 只是调用方任意字符串的缺口，供后续激活申请在
写事务内重新证明节点、所有者、安装、endpoint credential、当前共享同意与 Provider 的因果关系。

V279 不创建 Provider；调用方必须先通过既有本人 Provider 控制面取得
`registering/self_declared/user_node` Provider。Binding receipt 不是当前性、在线、ReadyCapability、路由、
Offer、容量、Attempt、Lease、执行回执或结算授权。本批 Domain、migration、Store/API 与正式 authority/acceptance 源码当前为
`design_frozen / source_written / source_review_only / implementation_uncompiled /
implementation_unrun`、`passed=0 / failed=0`；没有编译、测试、执行 migration、SQLite、runtime 或 network。

## 2. 与既有链的边界

- 通用 Provider、Offer、Job、Reservation、Attempt、Lease 与 Receipt 合同继续复用既有实现，V279 不复制。
- 节点已有 endpoint credential/session 与插件 sharing consent/delivery/ACK；V279 只引用其 durable identity，
  不把 session permit 或 ACK 提升为 compute authority。
- V277/V278 只服务 `external_pool + server_adapter`；V279 固定为
  `user_node + provider_endpoint` 的身份根，不读取或写入 V273/V274/V277/V278 对象。
- v14 endpoint profile 永久保持 blocked-only。V279 不新增 v15、Ready V2、Host/Sidecar/Runtime 或节点 wire。
- `ValidatedComputeAttemptStartDispatch`、v212 execution capability 与 v213 outbox 的生产构造继续后移。

## 3. Domain ABI、schema 与 digest

固定字符串：

```text
schema = compute_federation.user_node_provider_binding.v1
canonicalization = rfc8785_jcs
digest_algorithm = sha256
confirmation = confirm_user_node_provider_binding
identity domain = ELON-COMPUTE-USER-NODE-PROVIDER-BINDING-ID-V1
request domain = ELON-COMPUTE-USER-NODE-PROVIDER-BINDING-REQUEST-V1
material domain = ELON-COMPUTE-USER-NODE-PROVIDER-BINDING-MATERIAL-V1
receipt domain = ELON-COMPUTE-USER-NODE-PROVIDER-BINDING-RECEIPT-V1
binding effect = identity_binding_recorded
all downstream effects = none
```

Domain exact symbols：

```text
USER_NODE_PROVIDER_BINDING_SCHEMA_V1
USER_NODE_PROVIDER_BINDING_CANONICALIZATION
USER_NODE_PROVIDER_BINDING_DIGEST_ALGORITHM
USER_NODE_PROVIDER_BINDING_CONFIRMATION
UserNodeProviderBindingMaterial
UserNodeProviderBindingReceiptV1
UserNodeProviderBindingMaterial::new(...)
build_user_node_provider_binding_receipt(material)
validate_user_node_provider_binding_receipt(receipt)
canonical_user_node_provider_binding_request_digest(...)
canonical_user_node_provider_binding_receipt_json_and_digest(receipt)
user_node_provider_binding_json_is_canonical(json)
user_node_provider_binding_receipt_from_json(json)
```

`UserNodeProviderBindingMaterial::new` 参数顺序与类型固定为：

```text
provider_id: String
provider_genesis_digest: String
node_id: String
owner_user_id: String
installation_identity_digest: String
endpoint_installation_binding_digest: String
source_endpoint_credential_id: String
source_endpoint_credential_revision: i64
source_endpoint_credential_digest: String
source_consent_receipt_id: String
source_consent_policy_revision: i64
source_consent_policy_digest: String
source_authorization_ref: String
source_authorization_revision: i64
source_authorization_digest: String
idempotency_scope: String
idempotency_key: String
recorded_at: String
```

构造器固定 Provider genesis revision 为 1、`bound_at=recorded_at`、confirmation 和 effects；调用方不能传
这些结论。时间为规范 UTC nanoseconds。所有 revision 必须是 I-JSON safe 正整数，authorization revision 必须等于
consent policy revision；所有摘要为 64 位小写 SHA-256。

`binding_id` 是 identity-domain 对以下 exact JCS shape 的 64 位小写 SHA-256，不使用随机 ID，也不包含会轮换的
credential/consent revision：

```json
{
  "provider_id": "...",
  "node_id": "...",
  "provider_genesis_digest": "...",
  "installation_identity_digest": "...",
  "endpoint_installation_binding_digest": "..."
}
```

Request digest 只覆盖 `provider_id/node_id/owner_user_id/confirmation/idempotency_scope/
idempotency_key`。因此 exact replay 可在读取当前 source 前比较原请求；credential、consent 或时间变化不会把历史
成功请求误报成另一请求。

Receipt/material 字段私有，只能经 getters 读取。它们可反序列化用于 durable readback，但每次使用必须先执行完整
validator；它们不是 sealed current authority。
Store readback 应调用 `user_node_provider_binding_receipt_from_json`；该入口不只反序列化，还会完整复算并要求输入字节
逐字等于 canonical JCS。

## 4. Canonical JSON shape

顶层 exact keys：

```text
schema
binding_digest
binding_material_digest
canonicalization
digest_algorithm
binding
```

`binding` exact keys：

```text
binding_id
provider_id
provider_genesis_policy_revision
provider_genesis_digest
node_id
owner_user_id
installation_identity_digest
endpoint_installation_binding_digest
source_endpoint_credential_id
source_endpoint_credential_revision
source_endpoint_credential_digest
source_consent_receipt_id
source_consent_policy_revision
source_consent_policy_digest
source_authorization_ref
source_authorization_revision
source_authorization_digest
confirmation
idempotency_scope
idempotency_key
request_digest
bound_at
recorded_at
binding_effect
provider_effect
capacity_effect
offer_effect
readiness_effect
route_effect
execution_effect
settlement_effect
```

`binding_material_digest` 是 material-domain 对完整 `binding` 的摘要；`binding_digest` 是 receipt-domain 对
顶层 receipt 且 `binding_digest=""` 的摘要。`binding_json` 是包含两个摘要的完整 receipt JCS，不是 JSON 内的
自引用字段。确定性 validator `user_node_provider_binding_json_is_canonical` 必须 parse、拒绝未知字段、完整复算四个
domain，并逐字比较 canonical JSON。

## 5. 唯一 durable object

V279 exact 只新增：

```text
compute_user_node_provider_bindings
```

它是一张 immutable history/root table，`0 view / 0 revocation / 0 mutable head`。精确 37 列：

```text
binding_id
binding_schema
binding_digest
binding_json
binding_material_digest
canonicalization
digest_algorithm
provider_id
provider_genesis_policy_revision
provider_genesis_digest
node_id
owner_user_id
installation_identity_digest
endpoint_installation_binding_digest
source_endpoint_credential_id
source_endpoint_credential_revision
source_endpoint_credential_digest
source_consent_receipt_id
source_consent_policy_revision
source_consent_policy_digest
source_authorization_ref
source_authorization_revision
source_authorization_digest
confirmation
idempotency_scope
idempotency_key
request_digest
bound_at
recorded_at
binding_effect
provider_effect
capacity_effect
offer_effect
readiness_effect
route_effect
execution_effect
settlement_effect
```

`binding_id`、`provider_id`、`node_id` 各自唯一；`(idempotency_scope,idempotency_key)` 唯一。Binding 是永久
一对一 identity root；重新安装或改绑必须使用未来独立 supersession 合同，V279 不覆盖历史 row。

Migration 必须注册 deterministic arity-1 UDF
`elon_v279_user_node_provider_binding_is_exact`，并以同名既有 trigger 的 drop/create 得到 fresh/repeat/reopen
一致 inventory。INSERT source guard 必须核对 Provider genesis、当前 endpoint credential、当前 consent 与
authorization；UPDATE、DELETE、REPLACE 全部拒绝。缺 UDF、NULL/0/error 或直接写入畸形 JSON 均失败关闭。

## 6. Store transaction 与 replay

首次写入固定为：

1. `BEGIN IMMEDIATE`；
2. 先按 idempotency scope/key 读历史，完整 request/row/JCS/digest 相同则 0-write `ExactReplay`；
3. 读取 current Provider 与 revision-1 genesis，要求 owner exact、kind=`user_node`、status=`registering`；
4. 读取 current active endpoint credential，核对 node/owner/install 与 installation binding；
5. 读取 current plugin sharing consent，要求 `plugin_runtime_requested=true`、authorization 三元组完整且安装摘要相同；
6. 拒绝 node 或 Provider 的另一 binding；
7. 构造 Domain receipt，单 INSERT；
8. exact row/JCS readback，并在同一事务重新执行 current reproof；
9. commit 后只返回 owned receipt 与 `Inserted|ExactReplay` disposition。

Exact replay 不消费 mutation plan，不要求历史 source 仍 current，也不写时间或更新 row。新幂等键指向已绑定的 node 或
Provider 必须冲突，不能把现有 receipt 改造成新请求。

## 7. Current reproof 与 activation integration

当前源码中的 Store-private `CurrentUserNodeProviderBindingAuthority<'tx,'conn>` 只能由同一 transaction 的 reader 形成，
不得放入 Domain、HTTP、MCP、WebSocket 或 Android DTO，也不得 Clone/Debug/Serde。每次消费必须重新证明：

- receipt、material、identity、request 与 canonical JSON 全部 exact；无 fork/duplicate；
- current Provider 从记录的 revision-1 genesis 连续演进，kind/owner/provider ID 未改变；
- current active endpoint credential 仍属于相同 node/owner/install，installation binding digest 不变；
- current sharing consent 仍启用 plugin runtime，authorization 与当前 policy exact，installation digest 不变。

Credential 或 consent revision 可以合法推进；创建时 source 字段保留历史审计，不要求永远等于 current revision。
共享关闭、endpoint credential 撤销或安装身份变化返回 `None`；root/digest/fork/重复映射返回 `Err`。

Fresh user-node activation submission 必须在既有 activation request 的 `BEGIN IMMEDIATE` 内把
`node_binding_ref` 解释为 exact `binding_id`，重证 binding 并核对申请锁定的 current Provider revision/digest，
然后才 INSERT。0-write activation-request exact replay仍先返回历史 row。`managed_cluster` 保持旧语义；
`external_pool` 不进入本人接口。

该整合只把节点绑定材料从“任意字符串”收紧为 durable fact。ReadyCapability、route proof 与 hardware digest 仍是待审核
材料；V279 不使 activation preflight 成功，不推进 Provider/Pool 状态。

## 8. 公共边界、结果与非目标

未来公共入口最多提供本人 bind/read，响应只能公开 binding identity/digest、node/provider、replayed、派生 current 与稳定
阻断码。不得公开 credential secret、bearer、raw install ID、endpoint address 或 session token。Android 的明确同意与
状态展示属于后续 UI 切片；本批不修改 Android，也不把“安装/登录”描述为自动贡献算力。

V279 的唯一正向可观察 durable effect 是 `identity_binding_recorded`。以下始终为 `none`：Provider、Capacity、
Offer、Readiness、Route、Execution、Settlement。它不创建/更新 Provider、Pool、Offer、Job、Reservation、Claim、
v212 plan、v213 outbox、Lease、Runner event、usage、Execution Receipt 或任何余额。

生产 Ready 仍依赖 production VFS/A1/A2、v15、signed work-admission、Host enforcement、Sidecar/Runtime/health、
Ready V2 server verification、CPU-only numeric contract、provider-endpoint route、generic gateway constructor、节点
wire/ACK/Lease 与 Runner bridge。在这些完成前，V279 不能声明节点 online/Ready、Offer 可交易、attempt eligible 或任务
可执行。
