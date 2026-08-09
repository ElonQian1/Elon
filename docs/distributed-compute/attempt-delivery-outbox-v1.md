---
title: 分布式算力 Attempt Delivery Outbox V1
status: current
reviewed_at: 2026-08-10
owners: backend, node, ai-economy
implementation_status: implementation_uncompiled
---

# 分布式算力 Attempt Delivery Outbox V1

## 1. 目的与当前边界

v213 在 v211 Start command 和 v212 sealed Execution Plan 之后增加耐久派发 custody 与恢复证据层。v214 再形成 rejected observation/ACK/proof、quarantine cancel/reconcile pair、cancel 解锁 reconcile，以及 final reconcile+tombstone/proof 的本地原子闭包。它解决的不是“如何发一次 HTTP 请求”，而是先落 command 还是先碰远端、哪个 worker 拥有工作、网络结果未知时如何恢复，以及什么证据才足以证明远端从未开始执行。

v213-v214 只形成本地领域合同、SQLite 账本、Store 状态核和反向门卫；不实现 concrete Adapter、resolver、worker、网络、定时器、公网 ACK ingress、Bearer 凭据解析、Runner 事件或 external-pool onboarding。所有可认证远端 observation、Lease authority 和发送 authority 仍由字段私有、不可 Clone、不可反序列化且没有生产构造器的 sealed 类型承载，因此真实投递与远端恢复继续不可达。accepted happy issuer 也仍固定 unavailable。本批未编译、执行迁移、运行数据库或测试。

## 2. 先耐久、后远端

目标生产顺序固定为：

1. 由 sealed Plan、Provider 授权主体、route authorization 和 dormant lease-authority handle 组装 Start；
2. 在一个 `BEGIN IMMEDIATE` 中同时写入不可变 command 与 `prepare` outbox；
3. 后台 worker 领取 outbox 工作租约；claim 只代表本地工作所有权；
4. 真正外呼前再次重验当前 route、credential、Plan/source、预算、deadline 和 fencing，并先追加 send-attempt；
5. 事务提交后才允许未来 transport 使用该次不可 Clone send authority；
6. 远端 provisional prepare 成功后，本地在一个事务内写 authenticated observation、ACK、v185 activation、application、actor receipt、LeaseAuthorityBinding 和 `commit` outbox；当前 accepted issuer 在任何 mutation 前固定失败；
7. commit outbox 耐久后，未来 worker 才可解析并交付真实 Lease authority。

旧的 `Adapter.prepare_start(command) -> validated command -> Store` 顺序被禁止。它会在 command 尚未耐久时产生远端 prepared、平台 absent 的崩溃窗。

## 3. Route authorization 真源

v211 的 endpoint 或 Provider Adapter 引用不是可执行实现证明。v213 把下列历史事实分开保存：

- Adapter registry version：实现摘要、release version、支持的 Provider kind、verifier 和能力修订；
- credential version 与追加式 revocation：只保存非 bearer reference、verification receipt 和 TTL，不保存密钥、Token、签名 URL 或 bearer 正文；
- route authorization receipt：exact Provider/owner/route、endpoint 或 server-adapter 形状、Adapter registry version、config revision/digest、credential version、executor、批准人与 service actor；
- ordered capability set 与一对一 seal。

固定最低能力是 `authenticated_ack`、`authenticated_events`、`cancel_no_start`、`idempotent_commit`、`prepare` 和 `reconcile`。每项保存 exact resolved revision；v212 Plan 的 minimum revision 只能被相同或更高的 registry revision满足。Provider config digest 仍是长度有界的 opaque exact value，不被冒充为 SHA-256。

历史 authorization 只供审计和 cleanup；新 prepare/commit 发送必须重验当前 Adapter revision、credential 未撤销且未过期。普通 route 轮换不能自动赋权给旧 outbox；安全撤销也不能让已有未知远端状态失去 reconcile/cancel 的 cleanup 路径。

## 4. 五类派发账本

| 账本 | 保存的事实 | 明确不代表 |
|---|---|---|
| LeaseAuthorityBinding | exact command/Plan/ACK/application/Lease/fence/route、非 bearer ref、audience/scopes 与有效期 | 数据库持有可运行的 bearer 凭据 |
| Start outbox | `prepare/commit/cancel/reconcile` 不可变 payload、受控状态投影和 claim 信息 | 已发送，或远端只执行一次 |
| send-attempt | 某次外呼前已提交 exact request、claim generation 和 route binding | 远端收到、拒绝或成功 |
| remote observation | Adapter-authenticated prepare/commit/cancel/reconcile 事实、remote ref/sequence、verifier 与时间 | 本地 timeout、HTTP 成功码或字符串引用本身可信 |
| no-start proof | `local_never_sent`、final prepare rejection 或 authenticated never-committed tombstone | 当前本地没有 activation，或 cancel ACK 等于已停止 |

outbox 网络语义固定为 at-least-once。状态只表达本地 custody：`blocked`、`pending`、`claimed`、`in_flight_unknown`、`delivery_observed`、`abandoned_no_send`、`quarantined`。未知发送结果只进入 reconcile，不能转入盲重试；不使用 `exactly_once`、`stopped` 等超出证据的词。

## 5. Claim 与 send-attempt

claim 使用 CAS revision/generation 和 token SHA-256；数据库不保存明文 token。claim 过期但尚无 send-attempt时，可以重新进入 pending，或在严格终止后产生 local-never-sent 候选。claim 本身永远不是网络或执行授权。

worker 在外呼前必须用独立事务追加 send-attempt，并把 outbox 推进为 `in_flight_unknown`。从这一刻起，即使进程在 socket 调用前后崩溃，平台也不能推断“没有发送”。没有 observation 的 attempt 只能走 reconcile，不能创建新的 remote execution，也不能退款或释放容量。

## 6. 远端状态与 no-start 证明

远端 Start 协议只允许：

```text
absent -> prepared -> committed -> running / terminal_after_run
   |          |
   +----------+-> terminal_no_start（final reconcile + no-commit tombstone）
```

`prepared` 必须幂等、自动到期且尚不可执行；只有收到绑定 exact LeaseAuthorityBinding 的 commit 才可运行。cancel response 只能表示远端收到取消请求，并解锁 reconcile；它不能直接生成 `terminal_no_start`。no-start 只来自：

- 本地证明 prepare 从未形成任何 send-attempt；
- authenticated final prepare rejection；
- authenticated final reconcile attestation，且带 durable no-commit tombstone。

v214 已把后两类证据形成 Store-local 原子 issuer：final prepare rejection 会把 authenticated observation、`rejected` ACK 与 `prepare_rejected` proof 同事务写入；final reconcile 会把 authenticated `never_committed` observation、durable no-commit tombstone 与 `remote_never_committed` proof 同事务写入。它们只产 exact proof，不自动调用 v176 Finish。

一旦存在 v185 activation、dispatch application 或 commit send-attempt，就不能回退到 v176 的“从未执行”退款路径。后续即使确认零用量，也必须走独立的可信补偿/终态 kernel。

## 7. ACK、application 与 commit 闭包

远端 prepare accepted 仍只是 provisional。目标平台只允许在一个 `BEGIN IMMEDIATE` 中完成：

1. 写 authenticated `prepare_response(prepared)` observation；
2. 写 v211 `accepted_applied` ACK；
3. 调唯一 v185 activation kernel；
4. 计算 application；
5. 写 service-actor companion receipt；
6. 写 exact LeaseAuthorityBinding；
7. 先写 `commit` outbox；
8. 写 application；
9. exact readback 后统一 commit。

DDL 反向门要求新 application 同时看见 LeaseAuthorityBinding、commit outbox 和 actor receipt。这样本地 activation 已成功、却没有恢复 remote commit 的耐久权威，不能提交。当前 happy-path issuer 仍固定返回 `COMPUTE_ATTEMPT_ACCEPTED_ACK_V213_ISSUER_UNAVAILABLE`，所以该目标闭包尚不会发生 mutation。

v214 已形成 quarantine cleanup scaffold：过期或 source/budget/route 漂移的 accepted ACK 先在同一事务生成 `cancel` pending 与其后的 `reconcile` blocked outbox，再写 `quarantined` ACK。authenticated cancel observation 只把 exact reconcile 从 `blocked` 原子解锁为 `pending`；它不创建 no-start proof，也不解除 v176 门卫。只有后续 authenticated final reconcile+tombstone 产生的 exact proof 才可被 v176 重审计。

Store 派生的 proof `recorded_at` 与 outbox 状态转换 `updated_at` 由本地时钟生成，不接受调用方伪造；observation 行的 `recorded_at` 仍来自不可改摘要的 sealed authenticated envelope，并须通过顺序与有效期校验。

## 8. Provider owner 与 service actor

v211/v185 的 `activated_by_user_id` 保持兼容语义：它是 Provider owner 的合同授权主体快照，不是后台 worker 的实际身份。v213 另存一对一 actor receipt，exact 绑定 command、Plan、application、Lease、Provider owner/revision/digest、`service_actor_id` 与 authorization basis；被绑定的 actor authorization 固定 `service_actor_kind=platform_dispatch_service`，并由同一 Provider owner 签发。

HTTP 登录用户 ID 不得冒充 service actor。未来 no-start compensation 也不能复用 v187 并伪写 `aborted_by_user_id`；它需要自己的 verified proof 与 service-actor kernel。

## 9. v176 与旧写入口

v176 只有在 Reservation 没有 Start command，或引用同一 command/reservation/Job/Claim/budget/Lease/fence 的 exact no-start proof 时才可退款和归还 held capacity。command 到期、claim 过期、worker 失联、ACK 缺失、quarantined 或 cancel ACK 都继续失败关闭。

旧 Provider-owner 人工 Renew 和“确认未开始执行”的 Abort 写入口固定返回 `COMPUTE_ATTEMPT_RENEW_GATEWAY_NOT_READY` 与 `COMPUTE_ATTEMPT_ABORT_GATEWAY_NOT_READY`；GET、列表和历史回执仍可读取。真实 Adapter 启用前，Renew/Cancel/Runner event 必须拥有各自的 durable command、认证 observation、fencing 和 recovery 账本。

## 10. External pool 边界

external pool 只能使用 `server_adapter + adapter_execution`，不能伪装成用户节点或 endpoint ReadyCapability。v213 可表达 registry/authorization 历史形状，但本批不创建或激活 external-pool Provider，不签发 credential，不创建 Pool/Offer，不解析 concrete Adapter，也不开放派发或结算。

后续需要独立管理员/系统 onboarding：绑定 Provider owner、结算主体、Adapter registry/config、credential verification、协议能力、撤销与 cleanup-only 语义，并形成审核回执。现有 endpoint-only 激活计划和通用 Store 写方法不得作为旁路。

## 11. 仍未实现

- Adapter resolver、真实 credential verifier/KMS 与 bearer 解析；
- user-node、managed-cluster、external-pool transport 和 worker；
- authenticated ACK/event 公网入口、reconcile/cancel 网络协议与 crash injection；
- accepted application、actor receipt、LeaseAuthorityBinding 和 commit outbox 的可用 issuer；
- Lease authority 带外交付、Runner event、Renew 与可信零用量补偿；
- external-pool onboarding、Provider 激活和 service-managed capacity；
- 编译、迁移、SQLite trigger、并发、接口和真实远端验证。

上游可执行闭包见 `attempt-execution-plan-v1.md`，v211 command/ACK 与 v185 原子边界见 `attempt-execution-gateway-v1.md`，Broker 退款边界见 `broker-api.md`。
