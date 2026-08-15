---
title: 外部矿池 Adapter task-protocol production transport 权威
status: current
reviewed_at: 2026-08-16
owners: backend, security, ai-economy
design_status: design_frozen
implementation_status: implementation_partially_verified
verification_status: targeted_local_source_contracts_and_migration_verified
---

# 外部矿池 Adapter task-protocol production transport 权威

## 1. 唯一语义：默认关闭的 dormant production kernel

V273 冻结 external-pool Adapter 的 production transport、authenticated ACK/event ingress、reconcile 与 event
polling kernel。它复用 V272 已冻结的 ELTP v1 wire，但不复用 V272 synthetic fixture lane/executor，也不把受控
conformance receipt 提升为 production route 或 executor authority。

本批只有默认关闭的生产 kernel 合同。它不创建 v213 command、outbox、route、credential、service actor、
authorization、capability、seal、Lease 或 executor，也不新增这些对象的 constructor；未来只消费已经耐久化的
exact v213 command/outbox/route authority，并通过既有 v213 Store gate把首个 send-attempt与V273 exchange-attempt
原子成对记录。当前 Provider 仍为 `registering`，没有 stable executor binding，也没有可消费的 external-pool route，
因此 production eligibility 的固定观测值是 `eligible_rows=0`，worker 只能 dormant，不能外呼或接纳 ingress。

## 2. 唯一 startup gate 与 fail-closed 平台边界

V273 唯一新增环境开关是：

```text
ELON_EXTERNAL_POOL_ADAPTER_ATTEMPT_DELIVERY_ENABLED
```

它默认不存在或为 `false`。不得再增加 V273 专用 path、worker-count、poll interval、ingress address、secret、
credential 或 compatibility bypass 环境变量；资源、超时、target、launch、Secret 与 custody 全部来自既有权威。

值为 `true` 仍不等于可发送。startup 必须同时满足：

1. 当前平台是 Linux `x86_64`；Windows、Android、macOS、其它架构和未知 target 统一 unavailable；
2. V270 Provider-specific runtime readiness 的 runtime/custody 可用；
3. V272 task-protocol conformance 的 runtime/custody 可用。

任一条件不满足都不得降级到 generic HTTP/TLS、V272 fixture、旧 Adapter、user-node endpoint 或未认证回调。
startup 不取得、缓存或长期持有 Store-private current authority；每个 candidate/attempt 才按第 6 节在同一
connection、同一 `checked_at` 重新取得 V270/V272 current authority并闭合全部 roots。当前即使显式设置为
`true`，worker 启动后的 eligibility 仍因 Provider/route/executor 不可达而得到 `eligible_rows=0`。

## 3. Non-authoritative production lane subject

dormant worker 可在内存中派生一个仅用于分区、去重和审计的 production lane subject：

```text
domain = ELON-EXTERNAL-POOL-PRODUCTION-LANE-SUBJECT-V1

subject = RFC8785_JCS({
  provider_id,
  provider_owner_account_id,
  provider_binding_id,
  provider_binding_digest,
  registry_release_id,
  registry_release_digest,
  route_adapter_projection_id,
  logical_adapter_binding_digest,
  logical_projection_compatibility_digest
})

lane_subject_digest = SHA256(domain || 0x00 || subject)
```

该 digest 不是 receipt、bearer、fence、route、Lease 或 executor authority；V273 不为它新增独立表，也不提供
`executor_id()`、`Into<String>` 或写入 v213 executor 字段的转换。它只能让 dormant worker 确认“若未来闭合，
这些 Provider/binding/projection roots 属于同一生产 lane”。

每次 ELTP production exchange 仍必须绑定来自 future V275/V276 current authority 的真实 executor/fence roots。
lane subject 不得填充这两个位置，所以当前没有 stable executor 时 exchange constructor 必须返回 ineligible，而
不是临时生成 executor 字符串。

## 4. Exact 八项 production session roots

production ELSP/ELTP session roots 固定为以下八个 raw 32-byte digest，顺序属于 ABI：

1. `supervisor_session_policy_digest`；
2. `runtime_launch_profile_digest`；
3. `task_protocol_profile_digest`；
4. `upstream_transport_target_digest`；
5. `supervisor_session_policy_companion_digest`；
6. `launch_image_sha256`；
7. `ephemeral_task_secret_delivery_root`；
8. `task_protocol_conformance_run_receipt_digest`。

受管 child argv 同样固定为八项且顺序一致；每个值都是 64 个 lowercase、nonzero SHA-256 hex：

1. `--elon-task-production-policy=` → `supervisor_session_policy_digest`；
2. `--elon-task-production-runtime-profile=` → `runtime_launch_profile_digest`；
3. `--elon-task-production-protocol-profile=` → `task_protocol_profile_digest`；
4. `--elon-task-production-target=` → `upstream_transport_target_digest`；
5. `--elon-task-production-companion=` → `supervisor_session_policy_companion_digest`；
6. `--elon-task-production-launch-image=` → `launch_image_sha256`；
7. `--elon-task-production-secret-delivery=` → `ephemeral_task_secret_delivery_root`；
8. `--elon-task-production-conformance-receipt=` → current exact
   `task_protocol_conformance_run_receipt_digest`。

argv 只携带 root digest；第 7 项是既有 authenticated Config/Credential delivery exact root，不是 Secret 正文。
Provider、route、executor、fence、credential、target地址与任何 raw material均不得进入这八个 argv。argv hex解码为
raw 32-byte digest后才进入 session transcript/KDF。

session-root 与 KDF domain bytes 分别固定为：

```text
elon.external_pool_adapter.task_protocol.production.session.roots.v1\0
elon.external_pool_adapter.task_protocol.production.session.kdf_salt.v1\0
```

编码必须按上述顺序追加八个解码后的 raw digest，不能排序、去重、用 hex 文本替代或增加长度前缀。KDF salt
在八项 roots 后依次追加 fresh `host_nonce`、`child_nonce`。第 8 项只接受同进程 Store-private current V272 run
receipt digest；公开 JSON、诊断 view、历史 receipt、V272 process HMAC、fixture lane/executor 或 carrier 字段均不能
替代。

八项数组没有 command、outbox、route、executor 或 fence，因为这些是 per-exchange 事实；也没有 Provider policy
revision、candidate、actor、credential、idempotency key、时间或 receipt seal，避免短时/currentness 变化改写
session identity。

## 5. ELTP v1 复用与 production exchange binding

V273 原样复用 V272 的 exact-length ELTP v1 control wire：`ELTP|version=1|kind|flags=0`，kind 仍只有
`BEGIN=1`、`REQUEST=2`、`RESPONSE=3`、`RECEIPT=4`；operation 仍只有 `PREPARE=1`、
`IDEMPOTENT_COMMIT=2`、`CANCEL_NO_START=3`、`RECONCILE=4`、`AUTHENTICATED_EVENTS=5`。
`authenticated_ack` 继续由每次 authenticated `RECEIPT` 证明，不新增 operation。

request/exchange digest 继续使用 V272 已冻结的 domain：

```text
elon.external_pool_adapter.task_protocol.request.v1\0
elon.external_pool_adapter.task_protocol.exchange.v1\0
```

production request 必须 exact 绑定 operation、command、outbox operation、route、stable executor、attempt fencing
与 request body length/SHA-256。exchange 再绑定 session transcript、ordinal、operation、nonce、request、
delivery-attempt，以及 request/response/observation 各自 length/SHA-256。这里的 `fence` 是 exact Attempt/Lease
fencing root，不是 V254 18 个 deny trigger 的名称、数量或 inventory digest；18-fence inventory只能独立做负向
审计，不能填充 ELTP fence 字段。

big-endian、reserved=0、ordinal `1..64`、单 exchange 最长 15 秒，以及 V272 的 request/response/observation
size 上限全部保持。15 秒是从 BEGIN send 到 receipt 构造完成的单一 absolute deadline；semantic validator 必须
是纯、同步、有界校验，进入前、返回后与成功产出 receipt 前都重验剩余时间。同步 validator 本身不可被抢占，
但任何越过 deadline 的返回都必须令 session terminal且不得产出 receipt；本批不把这种事后失败关闭冒充硬实时
抢占。delimiter、EOF、chunked、streaming、generic TLS stream、未知 kind/op、MAC/root/nonce/ordinal/length
mismatch 都令 session terminal。

## 6. Eligibility、claim 与唯一网络顺序

production candidate 只能来自已经耐久化的 v213 outbox，不接受 API body、V272 fixture 或 caller 自报 command。
Store 查询必须同时证明：

- Provider 为 exact active external-pool successor；
- stable executor binding、route authorization、projection Adapter、credential、service actor、六 capability 与 seal
  全部 exact；`prepare`/`idempotent_commit` 还必须证明 Adapter、V253 credential reattestation 与 actor 当前有效，
  `cancel_no_start`/`reconcile`/`authenticated_events` 则只消费同一组不可变历史根并受 cleanup horizon 限制，
  不得因 head 正常轮换或 V253 业务有效期结束而丢失清理能力；
- command、Plan、Lease/reservation、outbox operation、executor 与 fencing generation exact；
- outbox 状态与 claim generation 允许本次 operation，且没有冲突的 send/exchange attempt；
- 对该 candidate/attempt 在同 connection、同 `checked_at` 取得 V270/V272 Store-private current authority，并
  证明第 4 节八项 roots exact；
- operation-specific deadline、credential TTL 与 cleanup horizon 未越界。

V273 不提供上述 v213 sealed 类型的 constructor，因此当前查询结果必须是 `eligible_rows=0`。未来达到可达性后，
唯一顺序是：claim existing outbox → 在同一个 `BEGIN IMMEDIATE`、同一 `checked_at` 内重验全部 current authority，
同时 append 首个 v213 send-attempt 与对应 V273 exchange-attempt，exact readback后一次 commit → 事务外执行 ELTP →
fresh transaction 验证 authenticated receipt → append receipt/poll/event facts → 交给未来 V276 Store-private ingress
consumer。send-attempt 或 exchange-attempt 任一写入/回读失败必须让二者同时 rollback；二者之间不能出现 commit。
任何 SQLite transaction、connection 或 sealed Store authority 都不得跨 network/child await。

`prepare` 或 `commit` 的结果未知只能进入 reconcile；不能盲重发产生新的 remote execution。event polling 只允许
在 exact committed/running remote identity 上推进 cursor。cancel ACK只证明请求被接收，仍需 reconcile 的
`terminal_no_start + no_commit_tombstone` 才能形成 no-start 候选。

## 7. Exact 六表 durable shape

V273 的 durable shape 只允许六张表：attempt、receipt、batch、event四表完全 immutable；reconcile/event poll各自是
immutable poll intent加 narrow CAS claim projection。不得新增 generic mutable head/queue、executor、route、secret、
signature、public-ingress、currentness view 或通用 observation 表：

| 表 | 主键/关系 | 保存的最小事实 |
|---|---|---|
| `compute_external_pool_adapter_task_exchange_attempts` | `exchange_attempt_id` PK | operation、source pair、command/outbox/send-attempt/route/executor/fence/request digests、session transcript、target、Secret-delivery、V272 run pair、delivery-attempt digest、started time。 |
| `compute_external_pool_adapter_task_exchange_receipts` | `exchange_receipt_id` PK；`exchange_attempt_id` UNIQUE FK | ordinal/op/nonce、全部 attempt identity reproof、request/response/observation length+SHA-256、exchange root、authenticated/received/recorded times。 |
| `compute_external_pool_adapter_task_reconcile_polls` | `reconcile_poll_id` PK | immutable exact uncertain exchange/send-attempt、remote identity、poll ordinal/request root；只允许 claim revision/generation/token-digest/owner/expiry/status 的 narrow CAS projection更新。 |
| `compute_external_pool_adapter_task_event_polls` | `event_poll_id` PK | immutable exact route/executor/fence/remote identity、requested cursor、poll ordinal；只允许同形状 narrow CAS claim projection更新。 |
| `compute_external_pool_adapter_task_event_batches` | `event_batch_id` PK | event-poll FK、batch/cursor/previous root、batch root、replay classification 与 event count。 |
| `compute_external_pool_adapter_task_events` | `event_id` PK | batch FK、event ordinal/type/remote sequence、previous/event root与 canonical event digest。 |

六表都必须有 exact JSON/scalar projection、domain-separated digest、no-delete/no-replace、FK/UNIQUE、lineage 与
source/currentness guard。attempt/receipt/batch/event必须 no-update；poll 的 immutable intent列必须 no-update，只有
列举的 claim projection能以 exact revision/generation CAS改变，禁止任意状态、payload、root、cursor或remote identity
更新。attempt 与首个 v213 send-attempt同事务先于网络耐久；receipt只能一对一关闭一个 attempt。poll/batch/event
只能引用 authenticated receipt lineage，不能接受本地 timeout、HTTP status、日志或 caller JSON冒充。

表中只保存长度、摘要、有限状态与远端 opaque ID；raw request/response/event body、credential locator、Secret、
MAC key、nonce 原值、target hostname/SNI/SPKI/address、bearer、claim token 与 process HMAC均不得落库。

## 8. Authenticated ACK/event ingress 与 Store-private handoff

ACK ingress 只来自通过 production session MAC、八项 roots、transcript、nonce、ordinal 与 exchange digest验证的
ELTP `RECEIPT`。event ingress 还必须验证 poll cursor、batch previous root、event previous root、remote sequence与
exact duplicate replay；gap、fork、conflicting replay、cursor rollback或同 event ID 不同 digest统一 terminal。
单个 authenticated batch 最多包含 256 个 event；Domain validator、DDL 与 Store recovery full-row audit 必须使用
同一个权威上限。batch 的 durable `replay_classification` 只允许 `new|empty`：`empty` 必须是零 event、空 inventory且
cursor before/after exact相同；exact duplicate 只能 exact readback 已存在的同一 `new|empty` batch/event rows，
不插入第二份 `exact_replay` batch，也不再次推进 cursor。

V273 可以定义 non-Clone、non-Debug、non-Serde 的 Store-private authenticated exchange/event material，但不能为
v211 ACK、v213 remote observation、v215 accepted closure、Lease authority或 Runner event开放 constructor。六表
receipt只是未来 V276 consumer 的输入证据，不是这些对象本身。

本批不新增 HTTP、MCP、WebSocket、callback、owner/admin collection、`/api/me`、通用上传或 polling API。网络 ingress
仅存在于受管 child 与 server 的 authenticated ELTP session 内；loopback/public listener 都不属于 V273。

## 9. Retry、crash 与 recovery

物理 exchange 不是 exactly-once。attempt 已提交后无 receipt，无论进程在 socket 前还是后崩溃，都只能标记为
remote outcome unknown并进入 reconcile；不得删除 attempt、回到 local-never-sent 或直接创建第二个 commit。
同一 authenticated receipt exact replay返回同一 durable row；同 attempt 不同 digest、同 remote sequence 不同
event root或同 cursor fork必须失败关闭。

startup/reopen 必须从六表与既有 v213 outbox派生工作，不保存明文 claim token或可恢复 session key。server 重启后
旧 process custody失效；只有 fresh V270/V272 authority、fresh session与 exact durable lineage才能继续 reconcile/
event poll。cleanup、credential expiry或 route revocation不能抹去历史，但会阻止新的 prepare/commit。

## 10. Zero authority expansion 与后继顺序

V273 不创建或激活 Provider、Pool、Offer、Job、Reservation、Attempt、Lease、usage、market 或 settlement；不签发
stable executor，不创建 route Adapter/version、credential、service actor、authorization、capability、seal 或 Start
outbox。Provider 必须保持 `registering`，V254 18 个 temporary absolute deny逐字保留，打开 fence 数为 `0`。

后继顺序固定为：

1. V274 冻结 Store-private activation-rooted active successor：exact两张immutable表+一个非权威诊断view，stable
   root排除Secret/session/executor/route，renewable evidence消费最长15秒V270-equivalent observation与fresh V272
   carrier；V275前零行、无public producer，Provider仍`registering`；
2. V275 在同一个原子事务内消费V274 pending overlay，签发stable executor、写exact projected-active Provider/route
   closure、append首个V274 successor并替换18 fences；
3. V276 才把本页 dormant worker/ingress handoff接到真实 v213 eligible rows并做 production reachability验收。

V274 docs/DDL、V275 executor/route 或 migration success都不能倒推 V273 transport已动态通过；V276 也不能跳过
V270-equivalent/V272/V274 current Store-private reproof。

## 11. 当前实现与验证现实

V273 当前严格为 `design_frozen / implementation_partially_verified /
targeted_local_source_contracts_and_migration_verified / production_runtime_unrun`。生命周期 UDF 借用与内部
映射/校验可见性编译阻断已修复，完整 WSL2 GNU `elon-server` 测试目标已编译；统一过滤器 `21 passed / 0 failed`
包含 18 项源码合同和 3 项动态迁移，规范化指纹为
`8f4bd0e305416b13ffba92bbd3e576bb20c56c7ff4465215b32a6318db7d58cc`。动态证据覆盖 fresh/repeat/reopen、
exact 六张空表、零 view、schema 稳定、六个完整性 UDF 接受 exact canonical envelope 并拒绝畸形或摘要未重算的 envelope、恢复入口持续
`eligible_rows=0`，以及 V254 18 deny SQL 不变。production runtime 仍为 `passed=0/failed=0`，没有 startup、
Linux child、ELTP、network、crash、concurrency、正向 ledger row 或 ingress 运行证据。唯一正式结论是 dormant
production transport/ingress 合同已冻结、可编译且 migration 边界已局部动态验证；
`eligible_rows=0`、Provider=`registering`、18 deny unchanged，production dispatch 与 atomic activation继续 NO-GO。
