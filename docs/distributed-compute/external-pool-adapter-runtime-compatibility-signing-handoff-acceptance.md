---
title: 外部矿池 Adapter 运行时兼容性签名交接验收边界
status: current
reviewed_at: 2026-08-15
owners: backend, security, ai-economy
design_status: design_frozen
implementation_status: implementation_uncompiled
verification_status: source_review_only
---

# 外部矿池 Adapter 运行时兼容性签名交接验收边界

## 1. 当前批完成条件

- 只新增一条 platform-admin `POST .../:challenge_id/signing-handoff`；Body 为 exact 六字段、
  `deny_unknown_fields`，没有 `/api/me`、generic run、observation create/read、signer callback、MCP/PC/APK。
- startup config 只接受 enabled 的 exact ASCII `true|false`；unset/`false` 与 path 未设置共同表示 disabled。
  enabled=`true` 必须配绝对 cgroup parent path，启用缺 path、path 无效、禁用却设 path 都令启动失败。
- enabled startup 以 no-follow directory semantics 打开并验证 cgroup-v2 与 cpu/memory/pids controllers，
  只在 server 私有状态托管 FD；caller、response、日志和 durable evidence 不出现 path/FD/cgroup leaf。
- 固定路由在 disabled/custody unavailable 时经认证/角色检查后返回 `503`，不接受 caller fallback。
- handler 先重开重哈希 installed tree，在独立 V269 `BEGIN IMMEDIATE` current-source gate 消费同一
  Prepared 并审 current V249 binding、Provider `registering` 与 exact roots；提交后把该 Prepared 移交
  有自身 preflight/postflight 的 V268 Store-private runner。SQLite transaction 不跨文件执行、child wait
  或任何 network await，也不得把两个 gate 描述成同一个事务。
- Provider binding 只作为执行载体，不进入 Provider-neutral V268 observation、signature message 或 receipt。
  V269 gate 后的 Provider/binding 漂移不改变 neutral evidence；future readiness 必须重新消费 fresh authority。
- response 顶层只含 `schema`、`record_binding`、`signer_payload`、`replayed`；record binding 只含 observation
  ID/digest，signer payload 只含 schema、signature algorithm、V237 key record ID/digest/key ID、message
  Base64/digest 和 expiry。
- 管理员显式把 signer payload 交给独立 V237 signer，并调用既有 V268 record route；server 不持有私钥、
  不自动签名，不新增 signer registry、transport、worker、outbox、job、lease、retry 或 durable schema。
- V268 唯一 observation 提供 durable replay；physical execution、并发请求与断连后继续执行不具备
  exactly-once。连接取消不得跳过 shutdown/reap/cleanup；进程在 commit 前终止后允许重跑。
- V268/V267 验证强度、Provider `registering`、V254 18 deny、九项 effect=`none`、七项 readiness=false
  原样保持；不得宣称 activation、market admission 或 production readiness。

## 2. Exact ABI 静态合同

路由必须逐字为：

`/api/admin/compute/external-pool-adapter-registry-releases/:registry_release_id/runtime-compatibility-verifications/:challenge_id/signing-handoff`

Body 只允许
`expected_challenge_digest`、`provider_binding_id`、`expected_provider_binding_digest`、
`expected_installation_receipt_id`、`expected_installation_receipt_digest`、`confirm_signing_handoff`。
`confirm_signing_handoff` 必须是 JSON boolean `true`，不是字符串或 caller 自定义确认短语。
caller-supplied actor/scope/idempotency、key/signature、Prepared、observation、fixture、path/FD/cgroup、timeout、
timestamp/nonce、policy、Secret 和 production target 必须静态不可表达。

response 顶层 exact 四键；`record_binding` exact 两键；`signer_payload` exact 八键：`schema`、
`signature_algorithm`、`sandbox_verifier_key_record_id`、`sandbox_verifier_key_record_digest`、
`sandbox_verifier_key_id`、`signature_message_base64`、`signature_message_digest`、`expires_at`。
source contract 还必须锁定 JSON 不另行投影 PEM/public key/signature、完整 observation/fixture receipt、
Provider binding、installation、actor、process、本机 path、FD、PID 或 Secret。不得把
`signature_message_base64` 误称为脱敏摘要：其解码内容必须继续是 V268 exact domain-framed message，包含
runner execution ID、challenge nonce digest、V237 operator/product、source/launch digest、public fixture
delivery root、V249 release/material digest 与 neutral installation-content digest；管理员 courier 与 signer
可见这些非 Secret 根，且源码不得把消息写入日志或新增 durable 列。

## 3. 必须失败关闭的路径

- enabled 使用大小写变体、空白或非 `true|false`，enabled/path 只出现一半，禁用却设置 path；
- 相对 path、symlink、非目录、非 cgroup2、controller 缺失、不支持平台或 FD custody 丢失；
- unknown/malformed JSON、错误 confirmation、challenge/release/binding/installation 不存在或交叉归属；
- challenge、binding、installation receipt digest 错误，challenge 过期，已有 observation 与 durable challenge/
  lineage 冲突，V249/V237/Profile/runner/fixture catalog 不 current，Provider 不是 exact `registering`；
- binding 指向另一 release/installation，installed tree 发生 no-follow、type、mode、size、digest 或 identity 漂移；
- V267 launch/Yama/bootstrap/delivery/no-work/shutdown/reap/cgroup/scratch cleanup 任一步失败；
- 尝试把 Provider binding/Provider-specific installation identity 或 receipt/cgroup、production
  config/credential/target/Secret 写入 V268
  observation、signature message、receipt 或公开 response；
- 把 client disconnect 当取消 child/cleanup，或把 UNIQUE observation 宣称为 physical exactly-once；
- handoff route 直接接受 signature、调用私钥、自动提交 V268 record、创建 worker/outbox/新表或修改 Provider。

## 4. 静态检查与零效果

source contract 至少证明 startup config 解析与误配矩阵、FD 私有且 DTO 不可表达、固定 route 与
401/403/400/404/409/422/500/503 分类、blocking executor 边界、exact binding/live-FS audit、V268 private
runner 调用、两个独立 transaction gate、response allowlist 和 durable replay。还必须证明 migration 注册仍止于 V268，没有
`migration_v269`、新 table/view/trigger；server background workers 没有新增 signer/handoff worker；既有
V268 record route ABI 不漂移。

零效果静态矩阵必须覆盖 V213、service actor、Provider/current version、credential、target/Secret、route、
CapacityPool、Offer、Job、Reservation、Attempt、Start、usage、verification/settlement/Sui 均无写入，V254
18 个 trigger name 与 source SHA-256 不变。九项 effect 仍为 `none`，七项 readiness 仍为 false。

## 5. 后续动态矩阵（本批不得运行）

- Rust compile、startup env 正负矩阵、Linux cgroup2 no-follow/controller/FD lifetime 与 disabled `503`；
- fresh/repeat V268 migration、route 401/403/400/404/409/422/500/503、response JSON exact allowlist；
- real V249 installation audit、V267 derived-launch/Yama/session/no-work/shutdown/reap/cleanup；
- concurrent duplicate handoff、连接取消、response 丢失、observation commit 前后进程故障与 replay；
- real independent V237 signer、wrong/revoked/expired key、签名提交及 V268 currentness；
- V254 18 deny parity、V260-V268 regression，以及未来 current V268 + fresh V265 + atomic activation。

本批不运行上述命令，动态计数固定为 `passed=0`、`failed=0`。静态 diff、format、source-size 或文档
modularity 只能作为源码卫生证据，不能升级为编译、HTTP、SQLite、Linux runtime、signer 或生产验收。
在真实 signer transport/私钥托管另行设计前，不得把管理员 courier 描述为 unattended signer；在独立
atomic activation/admission 完成前，Provider 必须保持 `registering`。
