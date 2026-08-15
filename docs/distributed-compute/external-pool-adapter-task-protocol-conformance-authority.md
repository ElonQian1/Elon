---
title: 外部矿池 Adapter task-protocol conformance 权威
status: current
reviewed_at: 2026-08-15
owners: backend, security, ai-economy
design_status: design_frozen
implementation_status: implementation_compiled
verification_status: source_review_only
---

# 外部矿池 Adapter task-protocol conformance 权威

## 1. 唯一语义：受控六能力实跑，不是 production route

V272 为 exact V249 Provider-neutral release 增加一次 server-owned、stateful、受控的 task-protocol
conformance run。server 使用固定 protocol profile、固定 synthetic fixture catalog、私有 execution carrier 和
非生产 fixture subject，实际行使 `authenticated_ack`、`authenticated_events`、`cancel_no_start`、
`idempotent_commit`、`prepare`、`reconcile` 六项能力；只有完整 session、shutdown、reap 与 cleanup 成功后，
才追加一份极短时、可撤销的 Provider-neutral receipt。

这不是 V252 的 release 声明或 sandbox test-plan `passed` 重放，也不是 V268 的 no-work compatibility
observation。caller 不能上传 observation、transcript、结果或 `passed=true`；六项结果只能由 server-owned
runner 和 stateful oracle 从实际 exchange 导出。receipt 证明 exact release 在同一进程 custody 中完成过一次
受控协议运行，不证明 production upstream、route producer、worker、stable executor、Provider activation、
market execution、usage 或 settlement 已存在。

V272 不新增独立 executor receipt。`fixture_lane_subject` 与 `fixture_executor_subject` 由 server 按独立 domain
从 exact release、protocol profile 和 fixture catalog 派生，固定标记
`non_production_no_v213_authority`；它们不是、也不得转换为 v213 `executor_id`、projection Adapter ID、
service actor、process identity、Provider binding 或未来 production executor authority。

## 2. 独立且默认关闭的 process custody

V272 冻结两项独立启动环境合同：

- `ELON_EXTERNAL_POOL_ADAPTER_TASK_PROTOCOL_CONFORMANCE_ENABLED`；
- `ELON_EXTERNAL_POOL_ADAPTER_TASK_PROTOCOL_CONFORMANCE_CGROUP_PARENT_PATH`。

enabled 只接受 exact ASCII `true|false`，未设置等于 `false`。disabled 时仍设置 cgroup path，或 enabled 时
path 缺失、空、非绝对、不能按 no-follow directory semantics 打开，均令 server 启动失败。enabled 只支持
Linux x86-64；cgroup-v2 parent 必须是 server 私有 delegated subtree，并具备固定 cpu、memory、pids
controller。不得退化为 caller path、宿主 root cgroup、V269 fixture custody 或 V270 production readiness
custody。

实现可以复用既有 custody primitive，但 V272 必须拥有独立 runtime instance、locked/zeroize-on-drop process
HMAC key、随机 custody epoch 和 committed-seal registry；V269/V270 的 enabled 状态、key、epoch、path 或
receipt 均不是 V272 authority。key、epoch 原值、FD、path 与 registry entry 不持久化、不序列化、不进入
日志或响应。固定 HTTP 路由始终注册，并先完成认证与角色检查；随后若 V272 disabled 或 custody unavailable，
返回 `503 Service Unavailable`，不得执行对象探测、文件审计或 run。

## 3. Exact roots 与 execution carrier 隔离

canonical receipt 只保存 Provider-neutral roots：

- exact current V249 registry release ID/digest/material digest、installation content digest、implementation、
  entrypoint、source/package/admission 与固定六能力 capability-set digest；
- exact current V250 re-attestation ID/digest、intelligence snapshot digest 与 expiry；
- exact current V252 receipt ID/digest、sandbox policy、ordered test-plan digest 与 report expiry；
- exact current V268 verification receipt ID/digest、run observation ID/digest、Profile V2、runner/fixture
  catalog、source capsule、launch capsule 与 verification expiry；
- V272 task-protocol profile/catalog ID、revision、digest，以及派生的 synthetic fixture lane/executor subject。

ELTP v1 session root array 固定为以下 14 个 32-byte digest，顺序是 ABI，不得排序、删项、重复、改名或
改用 JSON object 遍历顺序：

1. `supervisor_session_policy_digest`；
2. `task_protocol_profile_digest`；
3. `run_nonce_digest`；
4. `fixture_catalog_digest`；
5. `registry_release_digest`；
6. `installation_content_digest`；
7. `capability_set_digest`；
8. `sandbox_reattestation_receipt_digest`；
9. `runtime_compatibility_verification_receipt_digest`；
10. `source_capsule_sha256`；
11. `launch_image_sha256`；
12. `public_fixture_delivery_root`；
13. `synthetic_fixture_lane_digest`；
14. `synthetic_fixture_executor_digest`。

root transcript 与 session KDF salt 的 exact domain bytes 分别是
`elon.external_pool_adapter.task_protocol_conformance.session.roots.v1\0` 和
`elon.external_pool_adapter.task_protocol_conformance.session.kdf_salt.v1\0`。14 个 raw digest 必须按上述顺序
送入各自 domain-separated derivation；不得使用显示字符串、Provider carrier、delivery attempt 或 V239
receipt补位。第 1、10、11 项从 exact V268/Profile V2 chain提取；第 12 项必须由本次 V272 run 使用
fresh delivery nonce、对 exact V268-controlled public config/credential bytes 重新执行 authenticated delivery 后
生成。V268 receipt digest 已递归承诺其历史 delivery root，随机 nonce 使该旧 root 不可也不得伪装成 V272
fresh delivery root。第 13、14 项固定是 non-production/no-v213-authority synthetic roots。

V239 只保留为 V249 installation/adoption 的历史 ancestry 和六能力顺序的旧来源说明。V239 receipt
ID/digest/expiry 不是 V272 canonical receipt 的直接 authority root，不要求 current，也不参与 V272 TTL；
不得因 V239 已自然到期拒绝一个 otherwise exact current V249/V250/V252/V268 chain。V252 与 V268 的
Store-private current authority仍须按各自合同重验它们绑定的 V237 verifier key、签名和完整 lineage；V272
不复制、替换或弱化这些外签根。

run request 另外携带 exact `provider_binding_id/expected_provider_binding_digest` 与
`expected_installation_receipt_id/expected_installation_receipt_digest` 作为私有 execution carrier。Service 必须
取得 fresh `PreparedExternalPoolAdapterInstallation`，Store preflight/final trigger path 必须审计 carrier 仍
属于路由中的 exact V249 release，且 installation content、source 与 launch identity 逐项相等。carrier 的
Provider、owner、binding、installation receipt ID/digest、policy/config 与 projection roots不得进入 canonical
receipt、HMAC material、derived view 或公开 JSON。这样一次 Provider-specific 实跑只能证明 neutral release，
不能给该 Provider 铸造 route 或 activation authority。

## 4. ELTP v1 wire/profile 与八次 exact exchange

ELTP control payload 固定前缀为 `ELTP|version=1|kind|flags=0`。kind 只有 `BEGIN=1`、`REQUEST=2`、
`RESPONSE=3`、`RECEIPT=4`；operation 只有 `PREPARE=1`、`IDEMPOTENT_COMMIT=2`、
`CANCEL_NO_START=3`、`RECONCILE=4`、`AUTHENTICATED_EVENTS=5`。`authenticated_ack` 由每次
`RECEIPT` 聚合证明，不是第六个 operation，也不得新增 op code。所有整数 big-endian、reserved bits/bytes
为 0，ordinal 从 1 严格递增且上限 64，单次 exchange timeout 不得超过 15 秒。

request digest 的 exact domain bytes 是 `elon.external_pool_adapter.task_protocol.request.v1\0`，依次绑定 op、
command、outbox-operation、route、synthetic executor、fence roots和 request body length/SHA-256；delivery
attempt 不进入 request digest，避免与未来 v213 send-attempt identity 形成摘要循环。exchange digest 的 exact
domain bytes 是 `elon.external_pool_adapter.task_protocol.exchange.v1\0`，绑定 ELSP transcript、ordinal/op/
nonce、request/delivery roots，以及 request/response/observation 各自 length/SHA-256。

request body 与 observation 各自最多 262,144 bytes，raw upstream request 最多 65,536 bytes，response 最多
262,144 bytes。v1 只接受 exact-length frame；delimiter、EOF framing、chunked、streaming 或暴露 generic TLS
stream 均为协议降级并失败关闭。MAC、14-root、nonce、ordinal、长度、reserved 或 semantic 任一 mismatch，
整个 session立即 terminal，不得跳过坏 exchange继续生成 receipt。

fixture catalog 固定使用 synthetic command A/B，并严格完成八次 exchange：

| Ordinal | Operation 与 oracle 判定 |
|---|---|
| 1 | A `prepare`：只允许 `absent -> prepared`，返回非空 `refA`、`remote_seq=1`。 |
| 2 | A `idempotent_commit`：只允许 `prepared -> committed`，`start_count=1`、`remote_seq=2`。 |
| 3 | A same-idempotency commit replay：必须返回同 `refA/remote_seq=2`，`start_count` 仍为 1；只有 authenticated receipt 成功后，runner 才把本地 certainty 从 `clear` 转为 `unknown_after_remote_acceptance` 并保存不可伪造 marker。 |
| 4 | A `reconcile`：必须消费第 3 步同一 uncertainty marker，观察 `committed|running` 后转为 `resolved_by_reconcile`；不得重新发送 commit。 |
| 5 | A `authenticated_events`：primary inventory 仍只有 exact `started#1`、`terminal#2` 两个唯一事件，cursor 与 previous/event roots 连续；同一响应另带逐字段相同的 replay batch，固定分类为 `exact_duplicate_batch_replay`、batch count=1、replay root=primary inventory root，且唯一 `event_count` 仍为 2。 |
| 6 | B `prepare`：只允许 `absent -> prepared`，返回非空 `refB`。 |
| 7 | B `cancel_no_start`：只返回 nonterminal cancellation ACK；没有 tombstone，不得在此声称 no-start。 |
| 8 | B `reconcile`：必须返回 `terminal_no_start` 与 exact `no_commit_tombstone`，且 B `start_count=0/event_count=0`。 |

最终六能力 observation inventory 从这八份 host-authenticated exchange receipt派生：prepare 来自 1/6，
idempotent commit 来自 2/3，reconcile 来自 4/8，authenticated events 来自 5，cancel-no-start 来自 7/8，
authenticated ACK聚合全部八次 receipt。每条 observation 保存 operation/request/response digest与 size、
remote state/sequence/tombstone、ACK/event roots及 oracle transition/start/event counters；不得用六个 `passed`
字符串代替逐步 roots/counters，raw request/response/event/fixture bytes不落库。

## 5. 唯一执行顺序与 2-table/1-view durable shape

V272 只新增 append-only conformance receipt、append-only revocation 和 derived currentness view；不得新增
challenge、observation、mutable head、running、failed、executor 或 signature table。唯一执行顺序是：

1. admin authentication/role、strict request shape 与 startup custody gate 通过；Service 取得首份 fresh
   Prepared execution carrier；
2. 初始 `BEGIN IMMEDIATE` 重验 exact V249/V250/V252/V268、profile/catalog、carrier、structural predecessor
   与 actor-bound idempotency 后提交；任何 SQLite transaction 或 connection 都不跨 child/session await；
3. 事务外由 server 派生 fixture lane/executor subject，启动 exact V268 source/launch image，并由 stateful
   oracle 完整运行第 4 节矩阵；必要的 pinned execution handle 只受 process custody 管理；
4. 完成 authenticated shutdown、bounded pidfd reap、cgroup leaf cleanup 与 scratch cleanup；请求取消不能
   取消已经开始的 terminal cleanup；
5. cleanup 后重新打开、重哈希 execution carrier，取得 fresh Prepared；在新的 `BEGIN IMMEDIATE` 与同一
   fresh `checked_at` 中重验全部 roots/head/revocation/TTL/carrier，构造 Provider-neutral receipt material；
6. process HMAC seal 先进入 pending registry，transaction 追加 receipt 并 commit 后才提升为 committed seal；
   commit 前 crash 没有 row且可安全重跑，rollback 的 pending seal永不授权；DB 已 commit、promote 前的 row
   只能 historical，同一进程 exact pending replay可以完成 promote，重启后不得恢复或重新解释为 current。

physical run 发生在两个事务之间，因此不宣称 exactly-once。并发相同 predecessor 可能重复只读 synthetic
run，但最终 UNIQUE/CAS 只允许一个 durable successor；竞争者只能恢复 exact replay 或重新以 latest head 运行。

## 6. Canonical receipt、process seal 与 TTL

receipt 使用独立 domain-separated RFC 8785 JCS/SHA-256 schema、exact scalar projection、receipt integrity、
no-update/no-delete/no-replace guard。`canonical_receipt_digest` 对不含 process seal 的完整 Provider-neutral
material 求摘要；其中必须保存第 3 节 exact 14-root order、ELTP v1 profile与第 4 节八次 exchange inventory。
HMAC message 必须至少绑定该 digest、完整 task observation root、session/delivery/transcript
inventory roots、post-cleanup `checked_at`、`expires_at` 与 custody epoch digest。最终
`receipt_integrity_digest` 再覆盖 canonical digest 与 HMAC seal，避免自引用。

canonical material 还必须包含 run nonce digest、sequence/predecessor、started/completed/recorded 时间、四项
cleanup 结果，以及固定 `non_production_no_v213_authority`、`activation_ready=false` 和全业务 effect=`none`。
HMAC key/epoch bytes、committed-seal registry entry、raw transcript、Secret/config/credential、target/hostname/
SNI/SPKI/address、execution carrier、actor/idempotency/confirmation、signature/private key、v213 command/outbox/
send/route/lease/executor authority 或 production fence token 均不得进入 canonical evidence或公开 projection。
actor-bound replay 审计材料可作为 receipt table 的私有关系列保存，但不属于 canonical Provider-neutral receipt。

`expires_at` 必须 exact 等于以下最早值：

```text
min(
  post_cleanup_checked_at + 15 seconds,
  current V250 expires_at,
  current V252 report_expires_at,
  exact current V268 verification_expires_at
)
```

final insert 开始与 commit 前均须满足 `checked_at < expires_at`。V239 expiry、V270 readiness expiry 或 caller
时间不得加入、替代或放大这个窗口；receipt 不能续期，fresh successor 必须重新 physical run。

## 7. Lineage、currentness、revocation 与 consumer authority

receipt 按 `registry_release_id` 保持全局单线：唯一 genesis、唯一 `(release, sequence)`，每个 predecessor 最多
一个 successor。fresh run 必须 exact 引用 structural latest ID/digest；latest 即使已过期或撤销也可作为新 run
的历史 predecessor。相同 actor/scope/idempotency 与 immutable request material 的 exact replay 返回已有 row，
material 漂移固定冲突且不得重跑。

derived view 与 GET 仅给诊断：关系条件命中时唯一可使用的状态措辞是
`relationally_current_requires_process_custody_and_prepared_reproof`，不得显示 `current`、`ready` 或可消费。
即使 structural head、未撤销、未过期且 V249/V250/V252/V268/profile/catalog relationally exact，仍需同进程
epoch/seal 与 fresh Prepared reproof。server 重启会丢失独立
HMAC key/epoch/registry，所有旧 receipt 即使墙钟未到期也立刻 historical；数据库副本或公开 GET 不能恢复
authority。

未来 V273 producer 或 activation consumer 必须在同一 connection、同一 `checked_at` 取得 non-Clone、
non-Debug、non-Serde 的 Store-private current conformance authority，并另外提交该 Provider 的 fresh Prepared
execution carrier。Store 再重验 relational head、revocation、TTL、committed process seal、全部上游 roots，以及
carrier 与 neutral source/launch/content identity exact；diagnostic view、receipt JSON 或缓存结果均不能代替。

fresh revocation 只要求 historical exact receipt、structural latest、未撤销、platform-admin actor-bound
idempotency、reason 与 confirmation；不要求 process custody、carrier、filesystem 或上游 evidence仍 current。
revocation 只追加终态，不修改 receipt、不启动 run，也不触发 Provider、route 或 market effect。

## 8. Exact 三条 HTTP 路由

完整 surface 仅限具有 durable `users` actor 的 platform `admin|owner`；`OWNER_TOKEN` 生成的虚拟
`local-owner` 没有可外键审计的 durable actor，三条路由统一返回 `403`。没有 Provider-owner `/api/me`、
profile、challenge、observation、runner、executor 或通用上传端点：

- `POST /api/admin/compute/external-pool-adapter-registry-releases/:registry_release_id/task-protocol-conformance-runs`；
- `GET /api/admin/compute/external-pool-adapter-registry-releases/:registry_release_id/task-protocol-conformance-runs/currentness`；
- `POST /api/admin/compute/external-pool-adapter-registry-releases/:registry_release_id/task-protocol-conformance-runs/:run_receipt_id/revoke`。

create body 使用 `deny_unknown_fields`，只接受 expected registry release、V252、V268、protocol profile/catalog
digests，第 3 节 execution carrier，optional predecessor pair、idempotency key 与
`confirm_task_protocol_conformance_run=true`；predecessor 不允许半对。caller 不得提交 V250、V239、actor、
scope、time、nonce、fixture subject、observation、transcript、HMAC、expiry、effect 或 result。revocation body只含
expected receipt digest、reason、idempotency key 与 confirmation。

fresh create/revoke 返回 `201`，exact replay 返回 `200`；认证、角色、shape、语义、缺失、root/currentness/
lineage/idempotency、unavailable 与内部错误分别沿 `401/403/422/400/404/409/503/500` 边界。公开响应递归
移除第 6 节全部私有 material；GET 不返回可移植 bearer authority。

## 9. 后继顺序与严格 fences

V272 不写 v213 Adapter/version、credential、service actor、route authorization/capability/seal、Start outbox、
ACK/event ingress 或 production worker；也不分配 stable executor，不推进 Provider，不创建 Pool、Offer、Job、
Reservation、Attempt、usage 或 settlement。Provider 必须保持 exact `registering`，V254 18 个 temporary
absolute deny 逐字保留，打开 fence 数为 `0`；process/session/secret-delivery/broker-connect/upstream-probe/
runtime-launch/route/execution/activation readiness全部为 false，全部业务 effect=`none`。

后继顺序固定为：V272 controlled task-protocol conformance → V273 actual v213 producer/worker 与 authenticated
ACK/event ingress → V274 activation-rooted active refresh/successor → V275 stable executor binding 与 atomic
Provider/route activation。V272 足以作为同进程 Store-private release conformance 输入，但单独不足以创建 route
或 activation；若未来需要跨进程、离线或第三方可携带的证明，必须新增独立外签协议，不能把 process HMAC
receipt 升格。V237 challenge 外签在 V272 没有新增安全事实，因为 runner、oracle 与 observation 都由同一
server custody产生；当前硬安全边界是 same-process HMAC anti-forgery、fresh roots/Prepared 和短 TTL。

## 10. 当前实现与验证现实

当前已冻结 authority 与实现源码切面：V272 migration 的 2 tables/1 view、task protocol profile/catalog 与
runner/oracle、Store current authority、Service/redaction、三条 API 及 source contracts。修复 SQLite UDF 借用
生命周期和错误转换推断后，完整 Windows `elon-server` test target 已编译，指纹为
`702357846db905001886551903686f32f5a1d49461498b4802e366011d129eb9`；尚未执行专属
migration/ELTP/HTTP/SQLite/Linux fixture、启动 child 或连接 upstream。正式状态为
`source_review_only / implementation_compiled / implementation_unrun`，`passed=0 / failed=0`。

源码或文档存在不能证明六能力实际运行、process HMAC、TTL race、revocation、重启失效、carrier 隔离或 18
fences 已动态验收。验收门见
[`external-pool-adapter-task-protocol-conformance-acceptance.md`](external-pool-adapter-task-protocol-conformance-acceptance.md)。
