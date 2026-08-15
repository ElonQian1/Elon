---
title: 外部矿池 Adapter 签名运行时兼容性验证权威
status: current
reviewed_at: 2026-08-15
owners: backend, security, ai-economy
implementation_status: implementation_compiled
verification_status: source_review_only
---

# 外部矿池 Adapter 签名运行时兼容性验证权威

## 1. 唯一语义：Provider-neutral 的真实受控执行证据

V268 发布独立的 Linux x86-64 runtime compatibility Profile V2，并把 exact V249
registry release、content-addressed installed bytes、V257 source capsule、V267 derived launch
image、current V259 supervisor/session V2、固定 catalog 路径/约束下由 release manifest 声明的 public
fixture 和 V237 sandbox verifier key 组合成一条可撤销、可过期的签名兼容性验证链。server 固定的是
catalog path、role、size bound 与验证规则，不是 fixture bytes 的作者身份。

这条链是 release 级兼容性证据，不是某个 Provider 的生产 readiness。它不得读取或持久化
V256 production config/credential、credential commitment、V258 hostname/SPKI/address、V259
Provider companion/session delivery root，或任何由生产 Secret 派生的摘要。V268 交付时规划由未来
atomic activation 另取 fresh Provider-specific V255/V258/V259/V265 authority；V270 已把这组瞬时
证明收敛为 cleanup 后、同进程短时有效的 Store-private current readiness authority，并 exact 绑定
current V268 receipt 的 release、installation content、source/launch image 和 policy roots。未来
activation 必须在独立原子事务消费该 V270 authority；V268 自身不能替代该事务。

## 2. Profile V2、server-fixed runner/catalog constraint 与 release-declared fixture

Profile V1 的 schema、ID、revision、JSON 和 digest 原样冻结。Profile V2 使用新的 schema、ID、
revision 和 digest，绑定 current V255 runtime-launch policy、V258 broker transport policy、V259
supervisor/session policy V2、V257 source-capsule policy V1，以及新的 server-fixed compatibility
runner/catalog constraint。任何 catalog 漂移都使旧 V2 receipt historical-only，不得在 revision 不变时
静默重解释。

runner/fixture policy 固定使用 package 内四个 `resource`：

- `compatibility/v2/config.bin`；
- `compatibility/v2/credential.bin`；
- `compatibility/v2/no-work-request.bin`；
- `compatibility/v2/no-work-response.bin`。

fixture bytes 由 release manifest 声明且必须公开、非生产；其 path、role、SHA-256、size 与 package
manifest exact，读取
使用安装审计保留的 no-follow file handle。runner 只把 config/credential 借给 authenticated child，
对 child 产生的 request 做逐字节 exact match，再返回 exact response。它不解析 DNS、不建立 TLS、
不连接 upstream，也不接收 caller-supplied fixture、observation、timestamp、nonce 或 policy object。

V268 使用独立的 11-root child session ABI，按序为
`supervisor_session_policy_digest`、`runtime_compatibility_profile_digest`、`challenge_digest`、
`runner_policy_digest`、`fixture_catalog_digest`、`sandbox_verifier_key_record_digest`、
`registry_release_digest`、`installation_content_digest`、`source_capsule_sha256`、
`launch_image_sha256`、`public_fixture_delivery_root`。root domain 与 KDF salt domain 分别是 exact bytes
`elon.external_pool_adapter.runtime_compatibility_verification.session.roots.v1\0` 和
`elon.external_pool_adapter.runtime_compatibility_verification.session.kdf_salt.v1\0`；11 个 argv prefix
按相同顺序固定为 `--elon-runtime-compatibility-session-policy=`、
`--elon-runtime-compatibility-profile=`、`--elon-runtime-compatibility-challenge=`、
`--elon-runtime-compatibility-runner-policy=`、`--elon-runtime-compatibility-fixture-catalog=`、
`--elon-runtime-compatibility-sandbox-verifier-key-record=`、
`--elon-runtime-compatibility-registry-release=`、
`--elon-runtime-compatibility-installation-content=`、
`--elon-runtime-compatibility-source-capsule=`、`--elon-runtime-compatibility-launch-image=`、
`--elon-runtime-compatibility-public-delivery=`。legacy production 6-root constructor/domain/prefix 原样保留；
V268 不得把 production target、companion、bundle/Secret 槽位伪装进 compatibility roots。

## 3. 四段 durable evidence chain

V268 持久化四类 append-only 记录和一个诊断 view：challenge、server-run observation、V237-signed
verification receipt、revocation，以及 derived currentness。所有 receipt 使用独立 domain-separated
SHA-256、RFC 8785/I-JSON、deny-unknown shape、exact scalar projection、no-update/no-delete guard。

1. admin challenge body 只提交 expected V249 release/Profile V2/runner/fixture catalog digests、exact V237
   key record ID/digest/key ID、显式 structural predecessor、idempotency key 与 confirmation。Store 在一个
   `BEGIN IMMEDIATE` 中逐项复核这些 caller-expected roots 都是 current exact authority，由 server 注入
   authenticated platform-admin actor/scope，并生成 32-byte CSPRNG nonce 与 5 分钟窗口；同一 challenge
   在 DB 中最多只能产生一个 durable observation/receipt。不得隐式选择“最新”或假设 V237 只有一个
   active key。
2. Store-private runner 只接受 challenge identity、exact prepared installed files 和预先委派的 cgroup-v2
   authority。它重验 challenge/current roots，派生并执行 exact V267 launch image，完成 authenticated
   bootstrap、public fixture delivery、exact no-work exchange、authenticated shutdown、bounded reap 和
   cleanup，成功后才在新的 `BEGIN IMMEDIATE` 中插入一个 observation。任何 launch、protocol、timeout、
   expiry、stderr、shutdown、reap 或 cleanup 失败都不形成 observation。
   Prepared binding 还必须逐项等于内嵌 V249 release 的 admission ID/digest、package receipt
   ID/digest/material digest、source receipt ID/digest 与 capability-set digest；probe timeout 必须同时等于
   runner policy 的 `max_probe_timeout_ms`、server constant 和 current session policy，不允许两套时限漂移。
3. observation 固定状态 `server_run_observed_no_authority`。它保存 public fixture digests、source/launch
   capsule digests、launch size、ordered observations、ELNW receipt roots 与 zero violation counters；不保存
   raw fixture bytes、Secret、production target roots，也绝不把 `signature_message_base64` 或
   `signature_message_digest` 写入 canonical observation/DB。Store 只能在 final observation digest 形成后
   生成瞬时 signature challenge 作为 private runner return，并在 record 时从 durable challenge + observation
   重新构造；这样避免 observation digest 与 signature-message digest 循环。
4. signed receipt API 只能提交 existing observation ID/digest、expected signature-message digest、V237
   RSA-PKCS1v1.5-SHA256 signature、idempotency key 与 confirmation。Store 从 durable observation重建
   message，以该 challenge 绑定的 exact current V237 key 验签并原子插入 receipt。API 没有 create-run-
   observation DTO，也没有 HTTP runner route。

本批只定义 Store-private 两阶段 runner seam。private run receipt 能把最小 `run_observation_id`、
`run_observation_digest`、`signature_message_base64` 与 `signature_message_digest` 交给未来可信的
server-owned orchestration；这些瞬时签名字段不进入 canonical observation/DB，也不进入五条公开 HTTP
response。production orchestrator 与 independent-signer handoff caller 本批明确后置，因此 challenge
签发后没有已接线的 server-owned 调用路径去取得 prepared installation/cgroup authority、执行 run 并完成
外部签名交接。不得把 durable types、private runner 或 record route 的源码存在解释为端到端 signed
workflow 已可达或已闭合，也不得用新增 run/observation HTTP route 绕过该缺口。

这里的 `single-use` 不是 physical exactly-once。runner 在 preflight transaction 提交后、observation commit
transaction 开始前执行受限本地 fixture；并发调度同一 challenge 可能重复 physical run，最终 DB UNIQUE/CAS
只允许首个 durable observation，竞争者必须返回该首 row 的 replay。未来 production orchestrator 必须按
challenge 串行并使用幂等恢复，不得把 durable single-use 宣称为进程级 one-shot，也不得让 SQLite
transaction 跨越 child execution 或任何 network await。

receipt 按 `registry_release_id` 保持单线 CAS lineage：唯一 genesis、唯一 `(release, sequence)`、一个
predecessor 只能有一个 successor。并发 challenge 可以存在，但只有 exact structural predecessor 的
一条 receipt 可以成功。receipt 有固定短 TTL；过期、key/policy/profile/release 漂移或 revocation 都使
其 historical-only。

## 4. API、公开投影与 authority 边界

公开面仅限 platform-admin：Profile V2 GET、challenge POST、signed verification POST、currentness GET
和 revocation POST。Provider owner 没有 release-neutral 写权限；不存在 `/api/me` 对称路由。

- `GET /api/admin/compute/external-pool-adapter-runtime-compatibility-profile-v2`；
- `POST /api/admin/compute/external-pool-adapter-registry-releases/:registry_release_id/runtime-compatibility-verifications/challenge`；
- `POST /api/admin/compute/external-pool-adapter-registry-releases/:registry_release_id/runtime-compatibility-verifications`；
- `GET /api/admin/compute/external-pool-adapter-registry-releases/:registry_release_id/runtime-compatibility-verifications/currentness`；
- `POST /api/admin/compute/external-pool-adapter-registry-releases/:registry_release_id/runtime-compatibility-verifications/:verification_receipt_id/revoke`。

以上是完整 HTTP 面；不存在 observation GET/POST、runner/run、owner 或通用上传端点。

challenge JSON 冻结为
`{expected_registry_release_digest,sandbox_verifier_key_record_id,expected_sandbox_verifier_key_record_digest,expected_sandbox_verifier_key_id,expected_profile_digest,expected_runner_policy_digest,expected_fixture_catalog_digest,expected_predecessor?:{verification_receipt_id,verification_receipt_digest},idempotency_key,confirm_challenge}`；
Service 将 predecessor 映射为 Store 的 optional ID/digest 对，禁止半对。signed verification JSON 冻结为
`{run_observation_id,expected_run_observation_digest,expected_signature_message_digest,signature_base64,idempotency_key,confirm_verification}`；
revocation JSON 冻结为
`{expected_verification_receipt_digest,reason,idempotency_key,confirm_revocation}`。所有 body
`deny_unknown_fields`；caller 不能提交 actor、scope、nonce、时间、observation 内容、policy/key material、
fixture bytes、source/launch roots、Secret 或 endpoint root。

缺失或无效认证返回 401；已认证但不是 platform `admin|owner` 返回 403；JSON shape 返回 422，字段语义
返回 400，资源/归属不存在返回 404，root/currentness/lineage/idempotency/signature 冲突返回 409，内部
序列化或存储故障返回 500。fresh write 返回 201，exact replay 返回 200；没有 Provider-owner `/api/me`
surface。

公开 JSON 不得暴露 challenge nonce、signature message、signature、完整 observations、source/launch
capsule digest、runner internals、actor/idempotency/confirmation 或 raw receipt JSON。currentness view/GET
只是诊断，不是 trusted consumer authority。未来 consumer 必须在同一 connection、同一 checked_at
重新验证 head、revocation、expiry、V249、V237、Profile/runner/fixture catalog，并以 fresh retained
files 复核 source/launch identity；Store-private authority 必须 non-Clone、non-Debug、non-Serde。

challenge/verification durable shape 内嵌完整 authoritative V249 `registry_release`，其 manifest 的 generic
`files[].sha256` 会间接暴露 source identity；因此公开递归投影必须整键删除 `registry_release` 与
`fixture_resources`，不能只按 `entrypoint_sha256` 名称过滤。Profile V2 的 server-fixed policy/catalog
投影不受这个规则影响，currentness 只返回单独定义的 safe summary。

## 5. 明确没有的权限

V268 的 runtime-compatibility、Adapter、credential、Provider、route、activation、execution、usage、
market 与 settlement effect 全部为 `none`；process/session/secret-delivery/broker-connect/upstream-probe/
runtime-launch/activation readiness 全部为 false。Provider 保持 `registering`，不写 V213 command/outbox/
route，不创建 service actor，不触发任务、算力、用量、市场、结算或 Sui 动作；V254 的 18 项 temporary
absolute deny 必须逐字保留。

atomic Provider activation 和 market admission 明确后置。V268 批次最初要求先成功 shutdown/reap，
再组合 current V268 durable receipt 与 fresh V265 Provider-specific observation；V270 已把该要求实现为
cleanup 后才可形成、最多 15 秒且需同进程 HMAC custody reproof 的 Store-private current readiness
authority。后续 activation 必须消费该 V270 authority，不能回退为直接信任 V268/V265，也不能让
SQLite transaction 跨 upstream network await。

V254 的 temporary deny 不能因 V268 或 V270 source 存在而删除。未来替代至少要求：事务外形成已成功
清理的 V270 receipt；事务内同一 connection/`checked_at` 消费其 Store-private current authority，并由该
authority 重验 current V249/V254/V255/V258/V259/V250/V252/V253/V268 与 exact
installation/source/launch/profile/runner/fixture lineage；只有随后同事务写入的 Provider/route authority 才能
获得 activation effect。CapacityPool/Offer/market admission 仍须自己的 current capacity/price/admission roots
与原子写集合，不能由 activation receipt 顺带授予。后续 migration 必须为 18 个 insert/update/version/direct-
SQL edge 逐项安装等价 replacement guard，并通过 fresh/reopen/concurrency/crash/digest-drift/revocation/expiry
动态矩阵后，才可按已覆盖 edge 最小化移除对应 fence；未被完整替代的 fence 继续保留。

## 6. 当前验证现实

V268 已随完整 Windows product check 与 WSL2 `elon-server` test target 编译；尚未执行其专属
migration、单元/HTTP/signature/SQLite lineage 或 Linux runner fixture，也未读取真实 Secret 或
连接生产网络。当前状态为 `source_review_only / implementation_compiled / implementation_unrun`，
专属动态计数仍为 `passed=0`、`failed=0`。V269 默认关闭的 admin courier caller 已随完整目标编译，
但 unattended signer transport、私钥托管和自动签名闭环仍未接线，端到端 signed workflow 仍不可达；
源码存在和编译通过不能证明 runner、signature、SQLite guard 或 kernel confinement 已动态验收。
