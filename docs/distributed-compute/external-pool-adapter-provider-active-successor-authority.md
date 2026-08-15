---
title: 外部矿池 Adapter Provider active successor 权威
status: current
reviewed_at: 2026-08-16
owners: backend, security, ai-economy
design_status: design_frozen
implementation_status: implementation_partially_verified
verification_status: targeted_local_source_contracts_and_migration_verified
---

# 外部矿池 Adapter Provider active successor 权威

## 1. 唯一语义：activation-rooted、但在 V275 前 dormant

V274 冻结 external-pool Provider 从 `registering` 进入 exact active 后的可续签 current-authority overlay。
它把稳定 activation origin 与短时 runtime/task-protocol 证据分层，供 V275 原子激活与后续 refresh 消费；它不激活
Provider，也不签发 executor、route、credential、capability、fence、Attempt 或 Lease。

V274 只有 Store-private domain/DDL/transaction ABI，没有 HTTP、MCP、WebSocket、owner/admin、startup、worker、
public DTO 或通用 Store facade。V275 之前两张表必须保持零行，诊断 view 也必须返回零行；不得用短 TTL admin
`POST`、fixture、migration seed 或 direct SQL 提前创建 pending/active successor。

本批只安装dormant Domain/DDL/Store-private ABI。V270-equivalent committed-active observation minter、V253 ordinary
projected-active branch、successor refresh/revoke/current consumer与所有生产 effect都必须在durable V275 activation
witness/closure存在前失败关闭；pre-V275仅允许non-authorizing pending genesis observation/transition overlay。

## 2. Stable activation root

`activation_root_digest` 是结构性 origin，不是可续签 receipt。domain bytes 固定为：

```text
ELON-EXTERNAL-POOL-ADAPTER-PROVIDER-ACTIVE-SUCCESSOR-ACTIVATION-ROOT-V1
```

摘要固定为 `SHA256(domain || 0x00 || RFC8785_JCS(envelope))`。外层 envelope 的字段集合与类型属于 V274
合同；其中两份 Provider JSON 必须先按第 3 节生成 exact bytes，再作为字符串和 SHA-256 同时绑定，不能把
Provider 改用 RFC8785/JCS 重序列化。envelope exact 包含：

1. `provider_id`、`provider_owner_account_id`；
2. `source_registering_provider_id`、`source_registering_provider_policy_revision`、
   `source_registering_provider_json`、`source_registering_provider_digest`；
3. `initial_active_provider_id`、`initial_active_provider_policy_revision`、
   `initial_active_provider_json`、`initial_active_provider_digest`；
4. V249 `provider_binding_id`、`provider_binding_digest`、`registry_release_id`、
   `registry_release_digest`、`registry_release_material_digest`、`installation_receipt_id`、
   `installation_receipt_digest`、`installation_content_digest`；
5. V254 `candidate_id`、`candidate_digest`、`delegation_id`、`delegation_digest`、`service_actor_id`、
   `logical_adapter_id`、`logical_adapter_binding_digest`、`logical_projection_compatibility_digest`、
   `route_adapter_projection_id`；
6. V255 `profile_id`、`profile_digest` 与 `launch_policy_digest`；
7. V258 `target_id`、`target_digest` 与 `target_policy_digest`；
8. V259 `companion_id`、`companion_digest` 与 `supervisor_session_policy_digest`；
9. `entrypoint_capsule_policy_digest` 与 `launch_image_sha256`；
10. `task_protocol_profile_digest`、V273 `lane_subject_digest` 与 server-fixed
    `task_production_carrier_policy_digest`。

这些字段使同一 root 不能静默换 release、projection、launch profile、target、supervisor/entrypoint policy 或
production lane；任何结构字段变化都要求新的 activation origin，而不是在 refresh 中偷换。

稳定 root 明确排除：V250/V252/V253/V268/V270/V272 的可续签 receipt，process epoch/nonce/HMAC/TTL，
V263/V273 Secret delivery root 或 Secret 正文，ELSP/ELTP session root，executor、route authorization、future v213
service-actor registration/authorization receipt、六 capability、route seal、V254 deny/per-attempt fence、V275
activation witness/receipt、
command/outbox/send-attempt/Attempt/Lease，以及任何 network observation。这里的
`route_adapter_projection_id`只是V249 structural projection identity，不是v213 route authority。排除项不能被复制成
“稳定等价 root”。

## 3. Exact Provider JSON 与 logical-to-projection transition

Store 只接受当前 source `ComputeProvider` typed value，不接受 caller JSON。`initial_active_provider_json` 的唯一
生成算法是：复制 exact registering Provider；保持 provider/owner/kind/release/config/settlement 与其它稳定字段；
把 `policy_revision` 设为紧邻 revision，把 status 设为 `active`，把 `updated_at` 设为 preflight 冻结的
`activation_target_updated_at`，并把
`provider.adapter.adapter_id` 从 V249 logical adapter ID 改为 exact `route_adapter_projection_id`；然后调用
`serde_json::to_string(&ComputeProvider)`。digest 继续使用现有 Provider SHA-256 ABI。

因此必须同时满足：

- registering live Provider 的 adapter ID 等于 logical adapter ID；
- adjacent/exact active live Provider 的 adapter ID 等于 `route_adapter_projection_id`；
- release 与 credential lineage 继续锚定 logical adapter binding；
- `logical_adapter_id` 与 projection ID 由 compatibility digest 关联，永远不得断言二者相等；
- 只有 source `policy_revision` 的紧邻 active target 可作为 V275 genesis；caller 自报 active JSON、跳 revision、
  settlement/owner/release/config 漂移或 logical ID 继续充当 active adapter 都失败关闭。

## 4. Exact durable shape：两张 immutable 表与一个诊断 view

V274 durable namespace exact 只有：

```text
compute_external_pool_adapter_provider_active_successor_receipts
compute_external_pool_adapter_provider_active_successor_revocations
compute_external_pool_adapter_provider_active_successor_current
```

`receipts` 完全 immutable，按组持久化以下 exact scalar/canonical projection：

- identity/lineage：`active_successor_receipt_id`、`provider_binding_id`、`activation_root_digest`、
  `successor_sequence`、nullable `predecessor_active_successor_receipt_id`；
- structural origin：第 2 节全部 activation-root 字段、outer root JSON 与 digest；
- live Provider：`evidence_provider_id`、`evidence_provider_policy_revision`、`evidence_provider_json`、
  `evidence_provider_digest`；
- credential evidence：V253 `reattestation_receipt_id`、`reattestation_receipt_digest` 与
  `credential_observed_provider_id`、`credential_observed_provider_policy_revision`、
  `credential_observed_provider_json`、`credential_observed_provider_digest`；
- active runtime observation：`runtime_observation_id`、`runtime_observation_digest`、
  `runtime_observed_provider_id`、`runtime_observed_provider_policy_revision`、
  `runtime_observed_provider_json`、`runtime_observed_provider_digest`、`observation_started_at`、
  `observation_completed_at`、
  `observation_expires_at`；
- task-protocol evidence：V272 `task_protocol_conformance_run_receipt_id`、
  `task_protocol_conformance_run_receipt_digest`、`task_protocol_conformance_expires_at`；
- process custody：`process_custody_epoch_digest`、`process_custody_nonce_digest`、
  `process_custody_seal_digest`；
- activation closure：每个durable row required的 `activation_witness_id`、`activation_witness_digest`；pending
  overlay不写表，因此不存在nullable prepare row；
- receipt：`activation_target_updated_at`、final `evidence_checked_at`、`receipt_json`、`receipt_digest`、
  `created_at=evidence_checked_at`。

V274 的 dual-time 顺序 exact 为
`source.updated_at <= activation_target_updated_at <= observation_started_at <= observation_completed_at <= evidence_checked_at < observation_expires_at`。
preflight 用第一时间冻结 target/root，外部证据完成后的 final transaction 用第二时间重验 currentness；不得把二者
折叠回单一 `checked_at`，也不得用 final 时间改写 Provider target bytes。

V275实现前V274两张base table保证零row，因此V275 migration可失败关闭地rebuild V274 receipt shape：把旧
`checked_at`语义正式重命名为`activation_target_updated_at`，新增`evidence_checked_at`，并固定
`created_at=evidence_checked_at`；V275 parent table/UNIQUE先建立，再给V274 witness/root三元组安装指向V275
receipt/root三元组的immediate FK。不得保留歧义alias、反向FK或迁移任何非零历史row。

Genesis 的 credential-observed Provider pair 可以是 source registering pair，而 runtime/task-protocol evidence
pair 必须绑定 planned adjacent active target；两组 pair 不得被错误强等。V275 transaction 证明 transition 后才能
commit genesis。后续 refresh 则必须把两组 evidence 都重取到 exact current active Provider revision。

`revocations` 也完全 immutable，exact 持久化 `active_successor_revocation_id`、target receipt ID/digest、
`provider_binding_id`、`activation_root_digest`、actor ID/role、reason code、idempotency/confirmation digest、
`revoked_at`、canonical revocation JSON/digest。两表都禁止 `UPDATE`、`DELETE`、`INSERT OR REPLACE`、分叉
sequence/predecessor、跨 binding/root target 与 scalar/canonical 漂移；exact replay 只能 readback 原行。

`compute_external_pool_adapter_provider_active_successor_current` 只做 relational head/revocation/expiry 诊断，
其状态文字必须是 `relationally_current_requires_process_custody_and_active_root_reproof`。它不能验证 Prepared
filesystem、同进程 HMAC、current V253/V270-equivalent/V272 evidence 或 live Provider，因此不得作为 producer、
activation、route、dispatch 或 admission authority，也不得命名为 `ready`。
它不得依赖V249/V253/V270/V272 registering-only current view；只可从V274两张base table与direct historical/
Provider facts计算关系诊断，避免currentness递归。

当前 source-reviewed Store-private ABI 只暴露
`prepare_external_pool_adapter_provider_active_successor_target_on`：它接收调用者已有的 `&Transaction` 与同一
`activation_target_updated_at`，只生成 registering source、planned adjacent projected-active target和 stable activation root。返回的
Prepared target是non-authorizing、transaction-bound、non-Clone、non-Debug、non-Serde，不能从Store root公开
re-export；当前源码另有 dormant private read/audit 与 process-custody seam，但没有 Store facade、row producer、
current authority、active observation minter或 V272 active carrier。V275 implementation须让final producer另收服务端
生成的`evidence_checked_at`。任何Transaction/Connection/
Prepared authority都不得跨外部I/O，只可携带owned non-authorizing plan，并在final重建同一target/root。

`append_external_pool_adapter_provider_active_successor_genesis_on`、
`append_external_pool_adapter_provider_active_successor_refresh_on`、
`require_current_external_pool_adapter_provider_active_successor_on` 与
`append_external_pool_adapter_provider_active_successor_revocation_on` 是 V275 active/restart 与后继消费所需的
reserved ABI names；V274 本批对这些名字有零定义、零调用。V275 必须先提供 opaque durable atomic activation
witness，才可在同一 `BEGIN IMMEDIATE` transaction内加入 genesis producer，并实现 fresh successor/restart；V276
才接 route renewal与reachability consumer。不得把当前
structural target helper改述为 pending row、genesis receipt或 active authority。

## 5. Renewable active-successor evidence

每个 current receipt 都必须在 final `evidence_checked_at` 重新证明 stable root 未漂移，并消费：

1. exact immutable V249/V254/V255/V258/V259 historical roots、unique structural head/unrevoked facts与V274
   active-historical reproof；不得调用其registering-only current wrappers；
2. genesis消费current registering V253 transition proof；future refresh才消费durable V275 witness-gated
   projected-active credential currentness；
3. 一次 fresh、Provider-specific、V270-equivalent active runtime observation；
4. 一份 fresh current V272 conformance receipt及其 Store-private execution carrier；
5. exact V273 lane subject、`task_production_carrier_policy_digest` 与静态 production roots。

V270-equivalent observation 不是历史 registering V270 receipt 的重命名。它必须重新消费 Prepared
filesystem、runtime/target/supervisor、config/credential、authenticated no-work、shutdown/reap/cgroup/scratch cleanup，
并绑定 activation root 与 evidence Provider pair。`observation_expires_at` 不得晚于 completion 后 15 秒，也不得
晚于任何输入 evidence；Store final reproof 时必须尚未过期。

V272 canonical receipt继续 Provider-neutral。只有 Store-private carrier增加 activation-root-gated active subject；
Provider/activation/carrier字段不得写入 V272 public JSON、receipt digest 或 process HMAC。Genesis carrier绑定 planned
adjacent active target并保持 pending；V275 commit exact active Provider与durable activation witness后才可成为 current。
carrier digest使用V275冻结的`ELON-EXTERNAL-POOL-ADAPTER-TASK-PROTOCOL-ACTIVE-CARRIER-V1`与exact十字段material；
不含kind/time、V274 identity、process seal/session/Secret。genesis与active-refresh由不同typed constructors/current
reproof隔离，不得互换。
重启及后续 active carrier必须直接消费durable V275 activation witness、historical activation root与live
projected-active Provider，再由V274 wrapping；它永远不得先要求current V274 receipt，否则会形成V272↔V274递归。
V272 registering carrier与active carrier不能互换。

## 6. Purpose-separated custody：pending 到 committed

Active runtime observation 使用独立于 V270/V272 的 purpose-separated process HMAC。外部 filesystem/network/child
观察先在 SQLite 事务外完成；final `BEGIN IMMEDIATE` 前形成 non-Clone、non-Debug、non-Serde Prepared overlay。

external observation完成时只有non-authorizing evidence。final`BEGIN IMMEDIATE`取得writer lock、生成
`evidence_checked_at`并fresh typed重验Provider、stable root、V253、V270-equivalent observation、V272 carrier与全部
结构根后，才可注册one-shot V275 plan并把exact V274 seal mint/remember为`pending`；writer lock/该时间前不得存在
pending plan或seal。17项mutation与same-tx readback后commit；same connection postcommit exact readback成功才把seal
提升为`committed`并discard plan。rollback、commit不确定或postcommit readback失败均不得promote；pending永不授权，
显式删除不是安全性的前提。进程重启后不得根据durable receipt重建seal。

任何 SQLite transaction、connection、Prepared/Store authority都不得跨 filesystem、network、child 或 async await；
final reproof消费typed current authority，不接受raw-result wrapping。

## 7. Narrow currentness bridges；禁止 broad reinterpretation

V274 只允许 activation-root-gated 窄分支，不重写 V249-V270 历史语义：

- V249 原 registering carrier仍要求 live adapter=logical；active carrier单独要求 exact projection存在、live
  adapter=`route_adapter_projection_id` 与 compatibility root exact，不能把历史 registering receipt改称 active；
- V253 registering path保持不变。Genesis使用单独的Store-private transition-proof helper，消费current registering
  V253 receipt与planned projected adjacent target；它不需要、也不能要求预先存在V274 row。普通active
  `current.rs`、`challenge.rs`及相应DDL current/guard分支则直接受durable V275 activation witness+historical
  activation root约束，要求live adapter=projection；active分支永远不检查logical==projection，release/credential
  lineage仍为logical；witness不存在时，包括既有logical-active形状在内一律失败关闭；
- V254/V255/V258/V259/V270 existing receipt、API、current view 与历史 row保持原义；V274 使用新的显式
  Store-private active reproof，不把它们的 registering projection普遍改成 active；
- V272 只扩 Store-private carrier，canonical receipt和公开 API 不增加 Provider authority。

任何窄桥都不能让 old public DTO、diagnostic view 或历史 receipt单独成为 current active successor。

## 8. Lineage、revocation 与 restart

每个 `provider_binding_id + activation_root_digest` 只有一条全局线性 lineage：genesis sequence=1且 predecessor
为空；successor必须引用 exact unrevoked head；相同 actor-bound idempotency只允许 exact replay。live Provider 的
任何 active `policy_revision`变化，包括 settlement-only变化，都会令旧 successor historical；恢复 current 必须 fresh V253、
fresh active runtime observation、fresh V272与新 successor。V275负责实现该active successor/restart path；V276只负责
route renewal/reachability。V274 自身没有独立可运行 producer。`draining|quarantined|disabled` 一律失败关闭。

Revocation只终止 future prepare/commit/refresh消费，不修改 Provider、route、market、settlement或历史 task facts。
revoked/expired head可以作为 fully re-proven successor的结构 predecessor，但本身永远不 current；cleanup horizon内
的历史 authenticated attempt cleanup继续只靠其 immutable roots，不能被 V274 revocation抹去。

Restart 后 durable activation root与旧 receipts只保留 predecessor/audit价值。即使 wall-clock TTL 未过，所有旧
process custody立即失效；必须 fresh Prepared filesystem reproof、fresh V270-equivalent observation与新 purpose
seal、fresh V272 run/current receipt，并 append新 successor。诊断 view、旧 epoch或重新计算 HMAC都不能恢复 authority。

## 9. V275 genesis transaction 与 V276 reachability

首个 durable successor只能由 V275 在一个 `BEGIN IMMEDIATE` 中产生。该 transaction 在
`evidence_checked_at`重验
V274 Prepared overlay，并在同一原子闭包内完成 stable executor binding、exact route projection Adapter/version、
v213 route credential、V253 projected-active transition proof、service actor、route authorization、六 capability、
seal、紧邻 active Provider、durable V275 activation witness、V274 genesis与V254 exact
`9 pending-plan permits / 9 absolute denies` replacement；全部 exact readback 后一次 commit。这里列举
的是同一事务必须闭合的集合，不承诺错误的行级先后。任一失败全部 rollback，不能先 commit Provider、successor、
executor或route中的任意子集，也不能在 transaction 内出网。V275 canonical/DDL不得保存V274 receipt identity或
反向FK；V275自身`(activation_receipt_id, activation_receipt_digest, activation_root_digest)`为UNIQUE parent，V274
`(activation_witness_id, activation_witness_digest, activation_root_digest)`以immediate FK引用它，所以同事务先写
V275、后写V274即可闭合，不形成摘要环。只有Adapter/credential内部既有cycle保留deferred FK。

V274 不提供上述 V275 constructors。V275 也不连接 V273 dormant worker。V275实现active V253与fresh V274
successor/restart，但不续签route；只有 V276 可在每个 candidate/attempt
同connection按其独立currentness time重验current V274/V275 authority后，让 V273消费真实v213 eligible rows并执行production
reachability验收。

## 10. Zero effect 与当前证据

V274 不创建/修改 Provider、Adapter、route、Pool、Offer、Job、Reservation、Attempt、Lease、usage、market或
settlement；不读取生产 Secret，不启动 child/network，不改变 V273 `eligible_rows=0`。Provider保持
`registering`；V254 exact 18 temporary absolute deny逐字不变，打开 fence=`0`。

当前状态为
`design_frozen / implementation_partially_verified / targeted_local_source_contracts_and_migration_verified`。统一定向
验收 7/7 通过：5 项源码合同与 2 项动态迁移覆盖 fresh/repeat/reopen、精确两表一诊断 view、V275 前零行、18 个
V254 fence 不变、完整性 UDF 对畸形输入和未登记 process seal 失败关闭；验证指纹为
`9c363ccc6271005b6154d6ae230a34ed2da97b8335c130e9d67998d7632c9ffe`。这不是正向 successor producer 或运行态
证据；HTTP、startup、filesystem、Linux child、network、HMAC 正向 custody、crash、concurrency、V275 transaction
和 production activation/dispatch 均未运行，生产计数仍为 `passed=0 / failed=0`，继续 NO-GO。
