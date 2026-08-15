---
title: 外部矿池 Adapter task-protocol production transport 验收
status: current
reviewed_at: 2026-08-16
owners: backend, security, ai-economy
implementation_status: implementation_partially_verified
verification_status: targeted_local_source_contracts_and_migration_verified
---

# 外部矿池 Adapter task-protocol production transport 验收

## 1. 当前证据强度

V273 当前接受 authority 合同的静态复核、完整 WSL2 GNU `elon-server` 测试目标编译及局部动态 migration 验证；
生命周期 UDF 借用与内部映射/校验可见性阻断已修复。统一 `task_protocol_production` 过滤器
`20 passed / 0 failed`，包含 18 项源码合同和 2 项 fresh/repeat/reopen + UDF 动态迁移，规范化指纹为
`77f262e8d2553a39465324f199e7c6b58a214633c0ddfd08db855b5ba8d7cce4`。该指纹按顺序连接 20 条稳定的
`test ... ok` 行及移除 filtered/time 字段后的结果行再计算 SHA-256。动态证据覆盖 exact 六张空表、零 view、
schema 稳定、六个完整性 UDF 畸形输入失败关闭、恢复入口持续 `eligible_rows=0` 和 V254 18 deny SQL 不变。
production runtime 仍未运行，计数为 `passed=0/failed=0`；状态为 `implementation_partially_verified /
targeted_local_source_contracts_and_migration_verified / production_runtime_unrun`。

本页只定义验收门；唯一语义来源是
[`external-pool-adapter-task-protocol-production-authority.md`](external-pool-adapter-task-protocol-production-authority.md)。

## 2. 必须命中的静态合同

当前源码与 migration 的静态审查必须同时证明：

- 唯一新增 env 是默认关闭的 `ELON_EXTERNAL_POOL_ADAPTER_ATTEMPT_DELIVERY_ENABLED`；没有 path、worker、poll、
  ingress、secret、credential或 bypass companion env；
- `true` 的 startup只要求 Linux `x86_64` 与 V270/V272 runtime/custody available；Store-private current authority
  必须按每个 candidate/attempt在同 connection、同 `checked_at` 重取，不能由 startup取得或缓存；任一缺失失败
  关闭且没有 generic HTTP/TLS、fixture、endpoint或旧 Adapter fallback；
- dormant production lane subject使用 authority 的 exact domain/roots，只能做内存分区；没有独立 lane/executor
  表，没有 `executor_id()`/字符串转换，也不能填充 ELTP executor/fence；
- production session roots按固定八项 raw digest顺序编码，root/KDF domains exact；KDF只在八项后追加
  `host_nonce/child_nonce`，V272公开/历史 receipt、process HMAC或synthetic subject不能替代第8项；
- exact八个 `--elon-task-production-*=` argv prefix、顺序与root映射逐项命中，值只接受64 lowercase nonzero
  SHA-256 hex；argv不含Provider/route/executor/fence/Secret/raw material；
- ELTP v1 kind/op、request/exchange domains、big-endian、reserved、ordinal、timeout、size与 exact-length合同原样
  复用；production exchange exact绑定 command/outbox/send-attempt/route/stable-executor/attempt-fence；
- V254 deny-fence inventory只做独立负向审计，不得冒充 per-exchange Attempt/Lease fence；
- worker只消费既有 v213 outbox；首个 v213 send-attempt与对应V273 exchange-attempt必须在同一
  `BEGIN IMMEDIATE`/同一commit完成，之后才可出网；任何 transaction/connection/authority不跨 await；
  当前无 v213 constructor、stable executor或route时固定 `eligible_rows=0`；
- durable schema exact只有 attempts、receipts、reconcile polls、event polls、batches、events六表；attempt/receipt/
  batch/event完全immutable，两个poll只有immutable intent+narrow CAS claim projection；没有第七张table/view、
  generic mutable head/queue、secret、signature、executor、route或public-ingress表；
- receipt一对一关闭 attempt；reconcile只恢复 remote-unknown，event cursor/batch/event root连续，duplicate replay可
  证明，gap/fork/conflict失败关闭；
- authenticated material只形成 Store-private V276 handoff，不开放 v211/v213/v215/Lease/Runner constructor；
- 没有 HTTP、MCP、WebSocket、callback、owner/admin、`/api/me`、loopback listener或通用上传 API；
- Provider=`registering`、V254 18 deny逐字在位、打开 fence=`0`，所有 Provider/route/executor/activation/market/
  settlement effect=`none`。

## 3. Startup 与 dormant reachability 动态矩阵

当前 startup/worker/session 动态项全部未运行。未来最低矩阵为：

| 验收面 | 必须证明 |
|---|---|
| env disabled | env absent/false 都不启动 worker/session/listener，不写六表，不改变既有 v213/Provider。 |
| env malformed | 非 exact boolean拒绝启动，不按 truthy、大小写或空白宽松解释。 |
| platform | true+Linux x86_64进入依赖检查；Windows/Android/macOS/其它架构统一 unavailable且零副作用。 |
| V270/V272 startup | runtime/custody unavailable时失败关闭；startup不得查询、缓存或声称current Store authority。 |
| candidate currentness | 每个candidate/attempt同connection/checked-at重取current authority；expired、revoked、restart-historical、wrong process/carrier/Prepared任一组合拒绝。 |
| dormant eligibility | 仅 V273/V274 前置时查询稳定返回 `eligible_rows=0`，无 network/ingress/attempt row；不得造 fixture行让它变成正数。 |
| cancellation | startup或poll取消不跳过 terminal cleanup，不把半 session恢复为 authority。 |

## 4. 八 roots、wire 与 authenticated ingress 矩阵

| 验收面 | 必须证明 |
|---|---|
| root order | 八项每一项正确时建立 session；swap、duplicate、missing、extra、hex-vs-raw、wrong V272 receipt均拒绝。 |
| KDF | exact roots domain与KDF domain、host/child nonce顺序命中；nonce/root任一 bit drift导致MAC失败。 |
| wire | BEGIN/REQUEST/RESPONSE/RECEIPT与五个operation通过；未知kind/op、reserved非0、ordinal回退/跳跃、oversize、timeout、delimiter/EOF/chunked/stream拒绝；BEGIN send至receipt构造共用单一absolute deadline，纯/同步/有界validator进入前、返回后与成功前均检查，越时返回令session terminal且无receipt，不宣称可抢占同步validator。 |
| production binding | command/outbox/send-attempt/route/executor/fence/request任一不一致拒绝；lane subject或V254 fence inventory替代executor/fence必须失败。 |
| ACK | 只有authenticated RECEIPT形成ACK material；HTTP status、EOF、日志、remote ref或本地timeout都不能形成。 |
| events | cursor、batch previous/root、event previous/root、remote sequence连续；exact replay去重，gap/fork/conflicting replay terminal。 |
| redaction | DB、日志、错误和诊断无raw body、Secret、credential、MAC key、nonce、target/SNI/SPKI/address、bearer或claim token。 |

## 5. 六表 persistence、retry 与 crash 矩阵

| 验收面 | 必须证明 |
|---|---|
| fresh/repeat/reopen migration | **局部通过**：exact六张空表、零view、schema稳定、六个UDF畸形输入失败关闭、恢复入口 `eligible_rows=0`、V254 18 trigger SQL不变。PK/FK/UNIQUE正向行、projection/no-delete/no-replace、四表no-update、两poll narrow CAS与trigger语义仍待动态验证。 |
| attempt-before-network | 首个v213 send-attempt与V273 exchange-attempt同一BEGIN IMMEDIATE、同commit；任一失败两者rollback，commit后才允许socket，无receipt一律remote-unknown。 |
| receipt replay | exact replay返回同row；同attempt不同digest、第二receipt、不同session/nonce/ordinal拒绝。 |
| reconcile | unknown prepare/commit只能reconcile，不盲重发；cancel ACK无tombstone时不形成no-start。 |
| event poll | poll→batch→events FK完整、cursor/root连续、事件唯一；单batch权威上限为256并由Domain、DDL与Store全量审计一致执行，257条及以上失败关闭；`empty` 零事件批保持cursor不变仍可完成poll；partial batch或冲突重开不能推进head；exact duplicate只exact readback既有 `new|empty` batch/event rows，不插第二批或再次推进cursor。 |
| concurrency | poll claim以narrow revision/generation CAS选winner；send-attempt/exchange-attempt原子pair以UNIQUE只允许一个winner，loser只exact replay或重新读currentness。 |
| crash/restart | DB commit/session custody gap、process restart、expired V270/V272均失败关闭；route/Adapter/V253/actor currentness漂移阻止新的prepare/commit，但已认证的同一历史根在cleanup horizon内仍可cancel/reconcile/event poll；历史行只保留审计且不授权新的send。 |
| direct SQL | 伪造attempt/receipt/poll/batch/event、绕过source/root/currentness、删除历史、更新四张immutable表、修改poll intent或绕过narrow CAS claim projection全部被DDL拒绝。 |

任何 positive fixture 都不得通过删除、关闭、缩窄或改名 V254 fence获得 external-pool route；V273 动态测试在
V275/V276 前只允许验证 disabled/unavailable/`eligible_rows=0`，不能制造真实 production send。

## 6. 不属于 V273 的验收

以下均保持后置，不能记入 V273 passed：

- v213 Adapter/version、credential、service actor、route authorization/capability/seal或其 constructor；
- stable executor binding、Provider active version、Start outbox、真实 eligible row或生产任务执行；
- V249/V254/V255/V258/V259/V270 activation-rooted active successor；
- V254 18-fence replacement、atomic Provider/route activation；
- V276 worker/ingress到v213 ACK/observation/accepted closure/Lease/Runner event的production reachability；
- Pool/Offer admission、usage、market、settlement、部署或跨进程可携带外签 authority。

## 7. 后继门与正式结论

后继顺序固定为 V274 active successor → V275 stable executor binding + atomic Provider/route activation → V276
production reachability。V276 必须重新执行本页 startup、root/wire、六表、crash/concurrency与 ingress矩阵；不能把
V273 source review、V272 conformance passed或V275 migration success当作 transport已验收。

V273 当前只能声明“default-off dormant production transport/ingress kernel 合同已冻结，迁移边界已局部动态验证”。
它不能声明 worker已运行、ACK/event已接入、Provider可激活或任务可派发。正式状态保持
`implementation_partially_verified / targeted_local_source_contracts_and_migration_verified /
production_runtime_unrun`、运行态
`passed=0/failed=0`、
`eligible_rows=0`、Provider=`registering`、18 fences unchanged。
