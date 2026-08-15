---
title: 外部矿池 Adapter 运行时兼容性签名交接权威
status: current
reviewed_at: 2026-08-15
owners: backend, security, ai-economy
design_status: design_frozen
implementation_status: implementation_compiled
verification_status: source_review_only
---

# 外部矿池 Adapter 运行时兼容性签名交接权威

## 1. 唯一语义：显式管理员 courier 闭合 V268 可达性

V269 只为 V268 增加一条 server-owned、同步执行的管理员签名交接入口。平台管理员先创建
V268 durable challenge，再提交一个 exact V249 Provider binding 作为受控执行载体；server 使用
启动时托管的 delegated cgroup-v2 parent、重开并审计 content-addressed installation，调用 V268
Store-private runner。成功 shutdown、reap 与 cleanup 后，入口返回 durable observation 的最小
record binding 和给独立 V237 signer 的瞬时 signer payload。管理员充当 courier，把 payload 交给
独立 signer，再把签名提交到既有 V268 signed-verification route。

V269 不托管私钥，不调用 HSM/KMS，不实现 signer transport、自动签名、后台 worker、outbox、租约或
自动重试。它也不新增业务表、view 或 migration；V268 challenge、唯一 observation、signed receipt、
revocation 与 currentness 继续是唯一 durable truth。V269 没有新的 signer registry，所用 key 必须是
challenge 已精确绑定并在执行前后仍 current 的 V237 key。

Provider binding 只是打开一份已安装字节的执行载体。它必须精确指向 URL 中的 Provider-neutral V249
release 和 caller-expected installation receipt，但不得进入 V268 canonical observation、signature message
或 signed receipt。由此形成的证据仍是 Provider-neutral release evidence，不是该 Provider 的 durable
readiness。

## 2. 默认关闭的启动托管 cgroup authority

入口受两个启动环境项共同约束：

- `ELON_EXTERNAL_POOL_ADAPTER_RUNTIME_COMPATIBILITY_SIGNING_HANDOFF_ENABLED` 未设置或 exact ASCII
  `false` 时禁用；只接受 exact ASCII `true|false`，其他值令 server 启动失败；
- `ELON_EXTERNAL_POOL_ADAPTER_RUNTIME_COMPATIBILITY_CGROUP_PARENT_PATH` 只在 enabled 为 exact `true`
  时允许出现，并且必须是非空绝对路径。启用但缺失/无效，或禁用却仍设置 path，都属于启动误配并
  失败关闭。

启用时 server 必须在启动阶段以 no-follow directory semantics 打开该绝对路径，验证它位于 cgroup v2，
且 delegated parent 已启用 cpu、memory、pids controllers；不支持的平台、symlink、非目录、错误文件系统、
缺 controller 或打开失败都必须阻止启动。验证后的 directory FD 由 server 私有、按进程生命周期托管；
每次执行只借用或安全复制该 authority。环境 path、FD、cgroup leaf、本机路径和验证细节不得进入请求、
响应、日志、durable evidence 或签名材料。

固定路由始终注册。功能未启用或运行时 custody 不可用时，在完成认证与角色检查后返回
`503 Service Unavailable`；不得回退到 caller-supplied path、宿主 root cgroup、普通目录或较弱 supervisor。

## 3. Exact 管理 API 与同步执行次序

唯一新增路由为：

`POST /api/admin/compute/external-pool-adapter-registry-releases/:registry_release_id/runtime-compatibility-verifications/:challenge_id/signing-handoff`

它只允许已认证的平台 `admin|owner`。Body 使用 `deny_unknown_fields`，exact 六字段为：

```json
{
  "expected_challenge_digest": "<lowerhex64>",
  "provider_binding_id": "<id>",
  "expected_provider_binding_digest": "<lowerhex64>",
  "expected_installation_receipt_id": "<id>",
  "expected_installation_receipt_digest": "<lowerhex64>",
  "confirm_signing_handoff": true
}
```

caller 不能提交 actor、scope、idempotency key、V237 key、signature、observation、fixture、Prepared、
path/FD、cgroup、timeout、policy、timestamp、nonce、Secret 或 production target。server 必须执行：

1. 通过 Store-owned installation audit 重新打开并重哈希 caller 指定的 sealed content-addressed tree，形成
   不可序列化的 Prepared authority；不得隐式选择“最新”binding；
2. 在 V269 自己的 `BEGIN IMMEDIATE` current-source gate 中消费该 Prepared，重验 URL release/challenge、
   expected challenge digest、current V249 Provider binding、binding digest、installation receipt ID/digest、
   Provider `registering`，并要求 neutral release、admission/package/source/capability-set 与 installation-content
   roots exact。该事务只决定本次 trigger 是否获准执行，随后提交，不铸造 readiness；
3. 把同一份 Prepared 移交既有 V268 private runner。V268 在自己的独立 preflight `BEGIN IMMEDIATE` 中
   重载 exact challenge 与 durable observation 状态；只有尚无 observation 时，fresh 分支才再验
   challenge window/current neutral roots 和 challenge 绑定的 exact V237 key，已有 exact observation 时则
   直接进入 replay。V269 外层 gate 已在调用 runner 前验过当前 challenge 根，且会在返回
   signer payload 前再做 final current/expiry readback；
4. 仅 fresh 分支从启动托管 FD 取得 delegated cgroup authority，并在 SQLite transaction 外的
   blocking execution context 同步执行，完成 exact V267
   derived launch、authenticated bootstrap、四份 public fixture delivery、ELNW no-work、authenticated
   shutdown、bounded reap 与全部 cleanup；
5. fresh 分支由 V268 postflight `BEGIN IMMEDIATE` 复验 neutral roots，并只在全部成功后插入
   唯一 durable observation；replay 分支不创建 cgroup leaf、不再执行 child，直接使用 exact durable observation；
   API 再从 durable challenge + final observation 得到瞬时 signer payload。

任何等待 child、文件系统或本机执行期间都不得持有 SQLite transaction，也不得在 response 形成前启动
production upstream、读取 V256 Secret 或写 Provider/route。该 POST 可能长时间占用一个管理请求，但不
转化为后台 job；server 必须把 blocking work 与 async request executor 隔离。

## 4. 最小 response 与独立签名交接

成功 response 顶层只允许四个键：

```json
{
  "schema": "compute_federation.external_pool_adapter_runtime_compatibility_signing_handoff.v1",
  "record_binding": {
    "run_observation_id": "<id>",
    "run_observation_digest": "<lowerhex64>"
  },
  "signer_payload": {
    "schema": "compute_federation.external_pool_adapter_runtime_compatibility_signer_payload.v1",
    "signature_algorithm": "rsa-pkcs1v15-sha256",
    "sandbox_verifier_key_record_id": "<id>",
    "sandbox_verifier_key_record_digest": "<lowerhex64>",
    "sandbox_verifier_key_id": "<lowerhex64>",
    "signature_message_base64": "<canonical base64>",
    "signature_message_digest": "<lowerhex64>",
    "expires_at": "<canonical UTC nanoseconds>"
  },
  "replayed": false
}
```

`record_binding` 是管理员随后调用既有 V268 record route 所需的 observation ID/digest；
`signer_payload` 是交给 challenge 所绑定独立 V237 signer 的完整最小材料。signer 返回的 signature 不经过
本路由，仍由管理员通过既有 V268 route 提交
`run_observation_id`、`expected_run_observation_digest`、`expected_signature_message_digest`、
`signature_base64`、`idempotency_key` 与 confirmation。V269 JSON 不另行投影 PEM、公钥、签名、完整
fixture/observation receipt、Provider/installation identity、actor、path、FD、PID 或 Secret。需要明确的是，
`signature_message_base64` 不是不透明脱敏摘要：解码后就是 V268 冻结的 domain-framed 签名消息，按设计
包含 runner execution ID、challenge nonce digest、V237 operator/product、source/launch digest 与 public
fixture delivery root，以及 V249 release/material 和 neutral installation-content digest 等非 Secret 签名根。
管理员 courier 与独立 signer 都可以读取这些根；它们只能作为
短时签名请求处理，不得进入普通日志或新增持久化列。

V268 的 observation UNIQUE/CAS 是重放真源。相同 challenge 的重试若已有 durable observation，必须返回
同一 `record_binding` 和从 durable challenge + observation 重建的同一 signer payload，并置
`replayed=true`；不得再次把 caller-selected Provider binding 写进证据或改换 signer key。该保证只是 durable
single-use，不是 physical exactly-once。并发请求可能重复执行受限本地 fixture；连接取消也不能取消已经
开始的 blocking child cleanup，physical run 可能继续并提交 observation。若进程在 observation commit 前
终止，重试还可能再次 physical run。最终最多一条 durable observation，竞争者回放首 row。

V249 current-source gate 只在执行触发点证明所选 binding/Prepared 可用。它与 V268 preflight/postflight
不是同一个事务；gate 提交后 Provider 或 binding 漂移不会被写入、也不会反向污染 Provider-neutral V268
evidence。未来 Provider-specific readiness/activation 必须在自己的 fresh authority 和原子事务中重新消费
current binding/Prepared，不能复用这次 trigger admission。

## 5. 错误、脱敏与 authority 边界

缺失/无效认证为 `401`，非平台 `admin|owner` 为 `403`，非法字段语义为 `400`；release、challenge、
binding 不存在，challenge 不属于 URL release，或 installation receipt ID 不匹配为 `404`；对象
存在但 digest、currentness、binding lineage、exact-root、conflicting lineage state 或 live-FS 漂移为 `409`；
malformed/unknown JSON 为 `422`，
功能不可用为 `503`，内部序列化、存储或执行故障为 `500`。
fresh observation handoff 返回 `201`，durable replay 返回 `200`。错误响应不得暴露对象是否存在给未授权
caller，也不得带 stderr、fixture、signature message、key/operator/product、Provider/installation、path、
FD、PID、cgroup 或内部 receipt JSON。

V269 不增加 `/api/me`、observation GET/POST、generic run、signer callback、MCP、PC 或 APK surface。
管理员 courier 是显式人工信任边界；response 可被拿走或过期，系统不会在其后无人值守续签。即使后续
V268 record 成功，证据仍会因 expiry、successor、revocation、V249/V237/Profile/runner/catalog 漂移而
historical-only。

## 6. 明确没有的效果与当前验证现实

V269 沿用 V268 九项 effect=`none` 和全部 readiness=`false`：不创建或修改 V213 command/outbox、
service actor、Provider、credential、target/Secret、route、CapacityPool、Offer、Job、Attempt、usage、
settlement 或 Sui authority。Provider 保持 `registering`，V254 18 个 temporary absolute deny 逐字保留；
九项 effect 和七项 readiness 不得因 handoff 可达而升级。V269 也不等于 production private-key custody、
unattended signer worker、Provider-specific readiness、atomic activation 或 market admission。

V269 已随完整 Windows product check 与 WSL2 `elon-server` test target 编译；尚未运行其 startup、
HTTP/SQLite/Linux child 或 signer 专属矩阵，也未连接 signer 或生产网络。状态为
`source_review_only / implementation_compiled / implementation_unrun`，专属动态计数仍为
`passed=0`、`failed=0`。源码与文档只表达一条可达的人工交接合同，不证明
startup custody、live-FS audit、V267 runtime、V237 signer 或断连恢复已经动态验收。
