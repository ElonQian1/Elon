---
title: 外部矿池 Adapter stable executor 与原子激活权威
status: current
reviewed_at: 2026-08-16
owners: backend, security, ai-economy
design_status: design_frozen
implementation_status: implementation_uncompiled
verification_status: source_review_only
---

# 外部矿池 Adapter stable executor 与原子激活权威

## 1. 唯一语义：一次原子激活的永久历史事实

V275 冻结 external-pool 从 exact `registering` Provider 到 projected-active Provider、stable executor 与
Provider-specific v213 route closure 的唯一原子边界。它只记录“这些对象曾在同一事务一起激活”的永久历史事实，
不把该事实命名为 current、ready、reachable 或 dispatchable。

V275 durable namespace exact 为一张完全 immutable 表：

```text
compute_external_pool_adapter_atomic_activation_receipts
```

exact 为 `1 table / 0 view / 0 revocation table`。不得增加 mutable head、current view、queue、pending row、shadow
receipt 或 activation revocation。receipt 无论多新都永不单独 current；停止 future work 依赖 live Provider、v213
route/credential/capability 与 V274 renewable process authority 的失败关闭，不能删除、撤销或改写 V275 历史。

本批是 docs-first freeze，没有 Rust、DDL、Store、migration 或 runtime 实现。当前数据库仍无 V275 表/row，Provider
仍为 `registering`，V273 `eligible_rows=0`；本页不得被解释为已经激活。

## 2. 权威 DAG 与无环 V274 witness

V275 只沿以下方向消费权威：V249/V254/V255/V258/V259 structural roots → V274 planned target/root；current
registering V253 → non-authorizing projected transition；fresh V270-equivalent observation 与 fresh V272 private carrier
→ final activation transaction。V271 只提供 V221 logical source 到 V249 projection 的历史来源 lineage。

V274 receipt canonical 保存 V275 的 `activation_receipt_id + activation_receipt_digest` 作为 activation witness；V275
canonical **不得**保存 V274 `active_successor_receipt_id` 或 `receipt_digest`，也不得间接把二者放进 executor、v211
binding、idempotency 或 nested JSON。摘要依赖因此只有 `V274 -> V275`；反向摘要依赖禁止。

关系闭包也严格单向：V275表在自身
`(activation_receipt_id, activation_receipt_digest, activation_root_digest)` 建立UNIQUE；V274 receipt的
`(activation_witness_id, activation_witness_digest, activation_root_digest)`以immediate foreign key引用该三元组。
final transaction先插入完整V275 receipt，再插入引用它的V274 genesis/successor；跨版本边只允许上述immediate方向。V275
canonical和DDL都不含V274 receipt pair或指向V274的foreign key；缺失V275 parent、跨root、wrong digest或第二个
冲突V275 witness全部rollback。V274 child witness三元组不得UNIQUE；同一activation root的合法V274
genesis/successor链可多row复用exact同一V275 witness triple。wrong digest/root、同sequence、同predecessor sibling或
为同root换另一V275 witness仍失败。只有Adapter/credential等同事务内部既有相互闭包继续使用其自身deferred FK，
不能把该模式反转到V274/V275。

## 3. 双时间与不得跨 I/O 持有 SQLite authority

V274 preflight 在任何 filesystem/network/child I/O 前，由服务端生成 `activation_target_updated_at`，从 exact current
registering source typed Provider 生成 planned adjacent projected-active Provider JSON/digest 与 stable activation root。
target 的 `updated_at` exact 等于 `activation_target_updated_at`；不得用稍后的 observation 或 commit 时间改写 target。

外部证据完成后，final Store transaction 才生成 `evidence_checked_at`。时间序必须满足：

```text
source_provider.updated_at
  <= activation_target_updated_at
  <= observation_started_at
  <= observation_completed_at
  <= evidence_checked_at
  <  observation_expires_at
```

`activation_target_updated_at` 冻结 exact target/root，`evidence_checked_at` 冻结最终 currentness reproof；二者不能
合并成 `checked_at`。final transaction 必须从 live source、V249/V254 roots 与同一
`activation_target_updated_at` 重新生成逐字相同 target/root，并在 `evidence_checked_at` 重验所有可续签输入。source
revision/digest、target bytes、root、evidence head 或 expiry 任一漂移都失败。

不得让 `Transaction`、`Connection`、Prepared Store authority、V274 purpose seal 或 pending-plan capability 跨外部 I/O/
`await`。进程内可携带的preflight intent只是owned、non-authorizing、non-Serde value；它不是V275 pending plan。
final reproof 消费各层的
typed current authority，不用 raw-result wrapping 冒充 freshness。

## 4. Stable executor 与 projected v211 binding

stable executor 使用两个 exact uppercase domain：

```text
ELON-EXTERNAL-POOL-STABLE-EXECUTOR-ID-V1
ELON-EXTERNAL-POOL-STABLE-EXECUTOR-BINDING-V1
```

ID material 是 RFC8785 JCS object，字段 exact 为 `provider_binding_id`、`provider_binding_digest`、
`activation_root_digest`、`route_adapter_projection_id`、`service_actor_id` 与
`task_production_carrier_policy_digest`。计算：

```text
executor_id_hash = SHA256(ID_DOMAIN || 0x00 || RFC8785_JCS(id_material))
executor_id      = "external_pool_executor_" || lowercase_hex(executor_id_hash)
```

binding material 在上述字段之外 exact 加入 `executor_id`、`logical_projection_compatibility_digest`、
`projected_v211_adapter_binding_digest` 与 V273 `lane_subject_digest`。计算：

```text
stable_executor_binding_digest =
  SHA256(BINDING_DOMAIN || 0x00 || RFC8785_JCS(binding_material))
```

两份 material 都不得含 process/session/Secret、V270/V272 renewable receipt、V274 receipt identity、V275 receipt
identity、route credential、wall-clock expiry、Attempt 或 Lease。stable executor 不是 projection Adapter ID、logical
Adapter ID、service actor、lane subject、OS process、session 或 worker instance；任何替代或复用都失败关闭。

projected v211 Adapter binding 不发明新 domain，exact 复用既有
`ELON-COMPUTE-ATTEMPT-ADAPTER-BINDING-V1`，对既有 v211 canonical binding shape 做 domain-separated SHA-256。
该 shape 仍 exact 只有既有 `provider_id`、`provider_kind`、`route_kind`、`endpoint`、`adapter_id`、
`adapter_version`、`adapter_config_revision` 与 `adapter_config_digest`；V275 只令
`adapter_id=route_adapter_projection_id`，并固定既有语义下的 `provider_kind=external_pool`、
`route_kind=server_adapter`，其余字段从 planned Provider/route exact 投影。v211 shape没有 Provider revision/digest/status或
executor slot，不得虚构字段。planned active Provider pair与stable executor只由V275 activation-route binding和receipt
另行共同绑定。任何仍填logical Adapter或把process ID/V273 lane subject塞进v211 binding均拒绝。

V271/V254 的 `logical_adapter_binding_digest` 只保留 release/credential/source lineage；它不得再等于
`route_binding_digest` 或 `adapter_binding_digest`。V275 route、seal、future v211 command 三处必须逐字使用同一个
`projected_v211_adapter_binding_digest`，而 logical digest 只能作为独立 lineage 字段。

Store-private active-carrier digest domain exact为
`ELON-EXTERNAL-POOL-ADAPTER-TASK-PROTOCOL-ACTIVE-CARRIER-V1`。RFC8785 material字段exact只含：
`schema="compute_federation.external_pool_adapter_task_protocol_active_carrier.v1"`、`provider_binding_id`、
`provider_binding_digest`、`activation_root_digest`、`target_active_provider_id`、
`target_active_provider_policy_revision`、`target_active_provider_digest`、`route_adapter_projection_id`、
`task_protocol_conformance_run_receipt_id`、`task_protocol_conformance_run_receipt_digest`。digest exact为
`SHA256(domain || 0x00 || RFC8785_JCS(material))`，全部字段都能从V275 receipt逐字重算。

carrier material明确不含kind/time、V274 receipt identity、process seal/epoch/HMAC、session/wire或Secret。genesis carrier
与ordinary projected-active refresh carrier必须由不同non-interchangeable typed constructors产生：genesis绑定planned
target与same-transaction transition，refresh直接消费durable V275 witness/historical root/live active Provider并重取
current V272。相同digest shape不允许跨constructor复用；typed current reproof失败即拒绝。

## 5. Exact Provider transition 与 v213 closure

Provider target 继续只由 typed `ComputeProvider` 生成：复制 exact registering Provider，保持 ID/owner/kind/release/
config/settlement 和其它稳定字段；`policy_revision=source+1`、`status=active`、
`updated_at=activation_target_updated_at`，Adapter ID 从 logical ID 切换为 `route_adapter_projection_id`，最后只用
`serde_json::to_string(&ComputeProvider)` 与既有 Provider digest ABI。caller JSON、caller timestamp、跳 revision、
logical-active、owner/release/config/settlement 漂移都拒绝。

同一 V275 transaction exact 创建/更新的业务对象只有：既有 external-pool Provider 的 adjacent active UPDATE 与
version row、projection Adapter root/version、owner-delegated service actor authorization、route credential root/version、
`source_kind=external_pool_onboarding` 的 route authorization、固定六项 route capability、route seal，以及 V275/V274
receipts。不得 INSERT 新 active Provider，不得改变 Provider identity/kind，不得创建 Pool、Offer、Job、Attempt、Start、
usage 或 settlement。

route source 仍 exact 回到 V221 application/review/request 与 V249 binding；Adapter ID 是 projection，route/adapter
binding digest 是第 4 节 projected v211 digest。Provider active pair、route、credential、actor、六 capability、seal 与
receipt 必须一次 commit；任何“先 active 后补 route”“先 route 后补 receipt”或跨 transaction repair 都是 P0。

V275另冻结三份不可自选的exact摘要，均为`SHA256(domain || 0x00 || RFC8785_JCS(material))`：

1. projected transition proof domain exact为
   `ELON-EXTERNAL-POOL-ADAPTER-CREDENTIAL-PROJECTED-ACTIVE-TRANSITION-PROOF-V1`；material字段exact为
   `schema="external_pool_adapter_credential_projected_active_transition_proof_v1"`、V249
   `provider_binding_id/provider_binding_digest`、`activation_root_digest`、source Provider ID/revision/JSON/digest、
   target Provider ID/revision/JSON/digest、registering V253 receipt ID/digest、`logical_adapter_id`、
   `route_adapter_projection_id`、`evidence_checked_at`；
2. idempotency domain exact为`ELON-EXTERNAL-POOL-ADAPTER-ATOMIC-ACTIVATION-IDEMPOTENCY-V1`；material字段exact为
   `actor_kind="provider_owner"`、
   `actor_user_id=activation_root.activation_root.provider_owner_account_id`、provider binding ID/digest、
   `activation_root_digest`、`scope="external_pool_adapter_atomic_activation"`、
   `key=activation_root_digest`；
3. confirmation literal exact为`I_CONFIRM_EXTERNAL_POOL_ADAPTER_ATOMIC_ACTIVATION`，domain exact为
   `ELON-EXTERNAL-POOL-ADAPTER-ATOMIC-ACTIVATION-CONFIRMATION-V1`；material exact绑定该literal、同一actor kind/user ID、
   `idempotency_digest`、provider binding ID/digest与`activation_root_digest`。

actor、user ID、scope、key、confirmation、任一material字段及三份domain都不能由caller选择或覆盖。canonical/table
列可命名`activated_by_actor_user_id`，但值必须逐字来自
`activation_root.activation_root.provider_owner_account_id`，不得造`owner_user_id` alias。Store从authenticated Provider
owner与activation root派生并final重算；wrong owner、admin代签、任意自定义key/confirmation或旧V253 logical-
active proof都失败关闭。

## 6. Exact receipt projection

`compute_external_pool_adapter_atomic_activation_receipts` 完全 immutable，拒绝 UPDATE、DELETE、REPLACE、canonical
drift 与非 exact replay。每行 exact scalar/canonical projection 分组如下：

- identity：`activation_receipt_id`、`provider_binding_id`、`provider_binding_digest`、
  `activation_root_digest`；
- Provider transition：source/target 各自的 provider ID、policy revision、canonical JSON 与 digest；
- dual time：`activation_target_updated_at`、`evidence_checked_at`；
- V253 genesis input：registering re-attestation receipt ID/digest 与 non-authorizing projected transition-proof JSON/digest；
- stable executor：`executor_id`、ID material JSON/hash、binding material JSON/digest；
- projected v211 binding：canonical binding JSON 与 `projected_v211_adapter_binding_digest`；
- v213 closure：projection Adapter root/version、service actor authorization、route credential root/version、route authorization、六个
  ordered capability 与 route seal 的各自 ID/digest；
- renewable evidence snapshot：V270-equivalent observation ID/digest、started/completed/expires time，以及 V272
  conformance receipt ID/digest、private active-carrier material JSON/digest；
- audit/idempotency：`activated_by_actor_kind="provider_owner"`、`activated_by_actor_user_id`（值取上述exact root字段）、
  idempotency scope/key/material/digest、confirmation literal/material/digest、`receipt_json`、
  `activation_receipt_digest` 与 `created_at=evidence_checked_at`。

receipt canonical domain exact 为 `ELON-EXTERNAL-POOL-ADAPTER-ATOMIC-ACTIVATION-RECEIPT-V1`，digest 为
`SHA256(domain || 0x00 || RFC8785_JCS(receipt_without_digest))`。receipt JSON 不接受 unknown/missing/nullable
字段；六 capability 按 v213 固定顺序逐项投影，不能只存一个未经展开的声明摘要。

明确排除：V274 receipt ID/digest、pending-plan token、process HMAC/nonce/epoch、Secret/bearer、session/wire bytes、
raw observation result、Pool/Offer/market/settlement 与 V276 reachability。V275 receipt 保存可续签 evidence 的历史
snapshot不延长其 TTL，也不把它们变成 current。

## 7. Receipt-integrity/pending-plan UDF 与 18-fence 精确矩阵

receipt canonical完整性函数 exact 冻结为 deterministic one-argument
`elon_v275_external_pool_adapter_atomic_activation_receipt_is_exact(json)`；它只解析并重算第6节固定canonical/scalar
projection，不读时间、registry、数据库或外部状态，注册flags exact为
`SQLITE_UTF8 | SQLITE_DETERMINISTIC | SQLITE_INNOCUOUS`。pending permit判定则exact冻结为non-deterministic variable-arity
`elon_v275_external_pool_adapter_atomic_activation_pending_plan_matches(...)`，注册flags只能是
`SQLITE_UTF8 | SQLITE_INNOCUOUS`，绝无`SQLITE_DETERMINISTIC`。后者只查询当前进程 registry 中由 Store在final
writer transaction取得`evidence_checked_at`并完成fresh typed reproof后注册、一次性消费的exact plan；plan绑定provider binding/root、source/target Provider bytes、executor/v211
binding、九类 planned row 的全部 scalar/canonical bytes、V275 witness pair 与 dual time。token 不进入 SQL、receipt、API
或 caller DTO，migration 不 seed registry，函数缺失/报错/返回非 1 一律 deny。restart 后 registry 为空；旧 plan、跨
connection、wrong row、部分 replay、manual UDF 注册或 direct SQL 都不能授权。

18 个 V254 trigger 名与覆盖面必须全部保留。V275 只把以下九个改成“exact pending plan match 才 permit”：

| # | Trigger | V275 结果 |
|---:|---|---|
| 1 | `v254_external_pool_provider_activation_fence` | exact existing registering Provider adjacent-active UPDATE permit |
| 5 | `v254_external_pool_provider_version_active_fence` | exact adjacent active version INSERT permit |
| 6 | `v254_external_pool_candidate_projection_adapter_fence` | exact projection Adapter INSERT permit |
| 7 | `v254_external_pool_candidate_projection_adapter_version_fence` | exact projection Adapter version INSERT permit |
| 8 | `v254_external_pool_candidate_service_actor_fence` | exact delegated service actor authorization permit |
| 9 | `v254_external_pool_route_credential_fence` | exact route credential permit |
| 10 | `v254_external_pool_route_authorization_fence` | exact projected route authorization permit |
| 11 | `v254_external_pool_route_capability_fence` | exact six ordered capability rows permit |
| 12 | `v254_external_pool_route_seal_fence` | exact route seal permit |

以下九个继续 absolute deny，绝不查询 pending plan：

| # | Trigger | 永久结果 |
|---:|---|---|
| 2 | `v254_external_pool_provider_insert_active_fence` | active Provider INSERT deny |
| 3 | `v254_external_pool_provider_identity_update_fence` | Provider identity UPDATE deny |
| 4 | `v254_external_pool_provider_kind_update_fence` | Provider kind UPDATE deny |
| 13 | `v254_external_pool_capacity_pool_insert_active_fence` | CapacityPool active INSERT deny |
| 14 | `v254_external_pool_capacity_pool_update_active_fence` | CapacityPool active UPDATE deny |
| 15 | `v254_external_pool_capacity_pool_version_active_fence` | CapacityPool active version deny |
| 16 | `v254_external_pool_offer_insert_market_fence` | Offer draft/active INSERT deny |
| 17 | `v254_external_pool_offer_update_market_fence` | Offer draft/active UPDATE deny |
| 18 | `v254_external_pool_offer_version_market_fence` | Offer draft/active version deny |

因此精确结论是 `9 pending-plan permits / 9 absolute denies`，不是“18 deny 已删除”或“market fence 已打开”。全部写
位于同一Store kernel，但授权分离：V275 receipt与九类业务写只由V275 one-shot plan覆盖；V274 append只由独立V274
pending purpose-seal UDF覆盖。两份process authority exact绑定同root/witness/dual time但互不替代；绕开kernel、用V275
plan授权V274或用V274 seal授权九类业务写的direct SQL永远失败。

## 8. Final atomic transaction

流程 exact 分五步：

1. preflight 生成 `activation_target_updated_at`与non-authorizing planned Provider/root intent，释放 SQLite authority后才
   取得 external observation；observation完成时仍只有non-authorizing completed evidence；
2. final `BEGIN IMMEDIATE`取得writer lock后才生成`evidence_checked_at`，重读live source/structural roots、current
   registering V253、fresh V270-equivalent observation、fresh V272 carrier，并重建同一target/root、route与receipt bytes；
3. fresh typed reproof全部成功后，才注册one-shot V275 pending plan并mint/remember V274 pending purpose seal；二者
   exact绑定已生成的`evidence_checked_at`与完整planned bytes；writer lock/该时间之前不得存在pending plan或seal；
4. 执行17项mutation与same-transaction exact readback；V275 receipt先写、引用其witness pair的V274 genesis后写，
   V274→V275 immediate FK闭合后commit；
5. commit后仍在same connection做exact readback，成功才promote V274 seal并discard V275 plan。rollback、commit不确定、
   postcommit readback失败或connection漂移都不得promote；plan必须discard/失效，durable rows也不因未promote而current。

成功事务的row mutation inventory exact为`16 INSERT + 1 CAS UPDATE = 17`：service actor authorization 1；route
Adapter root+version 2；route credential root+version 2；route authorization 1；六 capability 6；route seal 1；Provider
version 1；V275 receipt 1；V274 genesis 1；再CAS UPDATE exact existing registering Provider pair为adjacent
projected-active pair 1。stable executor是V275 receipt/binding内的稳定权威，不另插executor表。V253新receipt、outbox、
Attempt、Pool、Offer、usage、settlement均为0。

任一 guard、expiry、precommit readback、foreign key或 commit 失败都不得留下 Provider、route、executor、V275/V274
row中的任意子集。commit/promote gap只允许same connection对同exact committed rows做postcommit readback后promote，
不得再写第二份receipt；未promote seal永不授权。
SQLite transaction内禁止外部 I/O。Provider 与 route 原子 commit，但 V275 不接 V273 consumer；成功后
`eligible_rows` 仍必须为 `0`。

## 9. V253、V274 freshness 与 restart

Genesis 的 V253 transition helper只消费 current registering receipt与 planned adjacent projected-active target，不要求
current或既有 V274 row。commit 后 ordinary projected-active V253 challenge/current直接以 durable V275 witness、
historical activation root与live projected-active Provider为门；它不得先要求current V274，避免 V253↔V274 cycle。
V271 logical digest继续只做credential/release lineage。

V274 current authority仍必须 fresh：每次消费重做最长15秒的 V270-equivalent active observation、fresh V272 private
carrier、live Provider/root reproof与process custody。V275历史receipt不能替代这些证据。restart 后旧V270/V272/
V274 seal、epoch、HMAC及currentness全部失效；V275必须允许从durable V275 witness+historical root+live projected-active
Provider出发，重新取得fresh observation/carrier并append V274 successor，而不能要求旧 V274 receipt仍current。

V275 只复用仍 live/current 的 exact genesis route；它不续签、替换或重建过期/撤销 route。route renewal、worker
reachability及重新接通 V273 `eligible_rows` 全部属于 V276。若 restart 时 route 已不 current，V275路径失败关闭并等待
V276；不得用永久 activation receipt 续命。

## 10. 无 activation revocation 与 cleanup

V275 没有 revocation对象或 current head。Provider进入`draining|quarantined|disabled`、route credential/relevant
authorization撤销、capability/seal失效或V274过期/撤销会阻止future work，但V275 receipt仍保持逐字历史事实。
同理，V274 process cleanup只清理pending seal/短时custody，不删除V275 row。

激活前失败只允许 rollback和best-effort清理进程内 pending plan/seal；安全性不依赖显式清理。激活后的历史
authenticated Attempt、ACK/event、no-start/reconcile/settlement cleanup继续由各自immutable root决定，不能因route或
Provider停止而抹去，也不能从V275 receipt推断任务曾被派发。

## 11. V276 reachability 与零经济效果

V276 才负责 route renewal、per-candidate/per-attempt same-connection current reproof、V273 worker/ingress reachability、
ELTP send/ACK/event与 `eligible_rows>0` 的正向动态验收。V275 不开放 HTTP/MCP/WebSocket/PC入口，不读取生产Secret，
不启动 child/network，不领取 outbox，也不创建 Pool/Offer/Job/Reservation/Attempt/Lease/usage/market/settlement。

正式状态为 `design_frozen / source_review_only / implementation_uncompiled / implementation_unrun`，
`passed=0 / failed=0`。这是 FORMAL DOCS FREEZE；没有 compile、migration、SQLite、restart、direct-SQL、fault injection
或 production runtime 证据，Provider=`registering`、`eligible_rows=0`。
