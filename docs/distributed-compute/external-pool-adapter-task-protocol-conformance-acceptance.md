---
title: 外部矿池 Adapter task-protocol conformance 验收
status: current
reviewed_at: 2026-08-15
owners: backend, security, ai-economy
implementation_status: implementation_partially_verified
verification_status: targeted_local_contract_migration_http_and_wsl2_oracle_verified
---

# 外部矿池 Adapter task-protocol conformance 验收

## 1. 当前证据强度

V272 当前接受统一 `task_protocol_conformance` 的 `21 passed / 0 failed`：18 项源码合同、2 项 Windows
SQLite 和 1 项真实 Axum 门卫。它覆盖 fresh/repeat/reopen、exact 2-table/1-view、18 个 V254 fence、完整性
UDF，以及 401/403/422 先于 unavailable 503，指纹为
`d08956fe46177ab11ea9038ce3306eff9e30e7fb09d5c07673e4dd12412ad45b`。WSL2 GNU server target 另有
direct stateful-oracle `3 passed / 0 failed`，覆盖 exact 八步状态、乱序/重复拒绝和 receipt 后置状态门，
规范化证据指纹为 `9f00ae268b6a4f52511884ec8943c1ffaab4630d808981e7b8a89020dba73019`。它没有启动 child，也没有
经过真实 session/wire authenticated ACK 或 process HMAC。startup、Linux child/session ELTP、成功 HTTP
写链、并发和故障矩阵仍未运行；状态为 `implementation_partially_verified /
targeted_local_contract_migration_http_and_wsl2_oracle_verified`。

本页只定义验收门，不重新定义 semantics；唯一语义来源是
[`external-pool-adapter-task-protocol-conformance-authority.md`](external-pool-adapter-task-protocol-conformance-authority.md)。

## 2. 必须命中的静态合同

源码审查必须同时证明：

- V272 只有 append-only receipt、append-only revocation 与 derived currentness view，两张表、一个 view；没有
  challenge、observation、mutable head、failed/running、signature 或 executor table；
- 两项 exact startup env 独立默认关闭；复用 custody primitive 但 instance/key/epoch/registry 独立，不依赖
  V269/V270 enabled，固定路由在 auth/role 后以 `503` 失败关闭；
- create 不接受 caller observation/result；server-owned runner/stateful oracle逐步生成六项 observation，单个
  `passed=true`、V249 declaration、V252 test plan 或 V268 no-work 均不能替代；
- ELTP v1 session root array 按 authority 的 14 个 digest exact 顺序编码，root/KDF domain bytes分别为
  `elon.external_pool_adapter.task_protocol_conformance.session.roots.v1\0` 与
  `elon.external_pool_adapter.task_protocol_conformance.session.kdf_salt.v1\0`；carrier不在数组中，synthetic
  lane/executor均为 non-production/no-v213-authority；第 12 项是本次 run 对 exact V268-controlled public
  fixture bytes 使用 fresh nonce 生成的 delivery root，不得复用或伪造历史 V268 delivery root；
- control kind只允许 BEGIN/REQUEST/RESPONSE/RECEIPT `1..4`，operation只允许 prepare/commit/cancel/
  reconcile/events `1..5`，authenticated ACK聚合每次 receipt而非新 op；request/exchange digest exact domain、
  big-endian、reserved=0、ordinal 1..64、15 秒 timeout与三类 262,144/65,536 byte上限均失败关闭；
- oracle 严格完成 A prepare/commit/replay/reconcile/events 与 B prepare/cancel/reconcile八次 exchange；A
  start count只能为 1，B必须以 reconcile得到 no-commit tombstone且 start/event count均为 0；
- execution carrier 只在 request 与 Store preflight/final Prepared 审计出现，不进入 Provider-neutral canonical
  receipt、HMAC material、derived view或公开 JSON；fixture subject 明确不是 production executor authority；
- canonical roots exact 包含 V249/V250/V252/V268 与 V272 profile/catalog；V239 仅历史 ancestry，不要求
  current、不进入 TTL，V252/V268 各自的 V237签名 currentness不得被绕过；
- preflight transaction → transaction 外 run → authenticated shutdown/reap/cleanup → fresh Prepared → final
  `BEGIN IMMEDIATE` 同一 checked-at reproof → pending seal/insert/commit/promote 顺序固定；没有 DB transaction
  或 connection 跨 child/session await；
- HMAC message绑定 canonical receipt digest、完整 task observation/session/delivery/transcript roots、expiry 与
  custody epoch digest；key/epoch原值与 seal-registry entry不持久化，重启后旧 row historical；
- expiry exact 为 `min(post-cleanup checked_at + 15s, V250 expiry, V252 expiry, V268 expiry)`，insert 开始/
  commit 前都未到期，fresh successor 重新 physical run；
- Store-private consumer 同 connection/checked-at 重验 head、revocation、TTL、process seal、全部 roots与 fresh
  Prepared carrier；类型 non-Clone/non-Debug/non-Serde，diagnostic view/GET不能铸造 authority；
- exact 三条 durable platform `admin|owner` collection route（虚拟 `local-owner` 固定 `403`）、strict
  DTO/error/redaction 合同，以及 Provider=`registering`、
  全部 readiness=false、全部业务 effect=`none`、V254 18 deny与打开 fence=`0`；derived view只能使用
  `relationally_current_requires_process_custody_and_prepared_reproof`，不得声明 current/ready。

## 3. 六能力动态判定矩阵

当前在 WSL2 GNU server target 上完成 direct stateful oracle 3/3；它直接调用 oracle transition，不等价于
启动 child、建立 ELTP session 或取得 wire authenticated receipt。完整动态矩阵仍须逐项闭合：

| 验收面 | 当前结果与完整门槛 |
|---|---|
| ordinals 1/6 prepare | direct oracle 已确认 A/B 进入 prepared，且乱序、重复 transition 拒绝；refA/seq1/refB 通过真实 ELTP observation 的完整绑定仍未运行。 |
| ordinals 2/3 commit | direct oracle 已确认 committed、`start_count=1`、replay 不增 start count 并生成 pending marker；same-idempotency wire replay 与 authenticated receipt 全绑定仍未运行。 |
| ordinal 4 reconcile | direct oracle 已确认 receipt 后置状态应用前拒绝 reconcile，应用后转为 running/resolved；真实 receipt、marker wire 绑定与矛盾漂移矩阵仍未运行。 |
| ordinal 5 events | direct oracle 已确认 terminal、2 events、exact replay equality 与 duplicate classification；cursor/root chain 和 gap/fork/conflict wire 矩阵仍未运行。 |
| ordinals 7/8 cancel | direct oracle 已确认 7 无 tombstone，8 才 terminal_no_start + tombstone，且 B start/event 为 0；ACK wire 绑定仍未运行。 |
| authenticated ACK | 未运行真实 ACK；direct oracle 只证明生产代码在 post-receipt 状态应用前拒绝 reconcile，不能替代 session receipt 验证。 |
| wire limits | 未运行；仍须覆盖 exact-length v1、big-endian、reserved=0、ordinal/timeout/size 上限及 delimiter/EOF/chunked/stream、generic TLS stream、未知 kind/op 拒绝。 |
| failure/cleanup | 未运行；仍须覆盖 timeout、stderr policy、协议失败、shutdown/reap/cgroup/scratch 任一失败零 receipt，以及响应取消后的 terminal cleanup。 |

receipt 必须保存每步 digest/size、状态、sequence、tombstone 与 oracle counters 的完整 host-derived observation；
只保存六项 `true` 或复用 caller transcript 应判验收失败。

## 4. Persistence、custody 与 HTTP 动态矩阵

| 验收面 | 当前结果与未来门槛 |
|---|---|
| fresh/repeat/reopen migration | Windows `v272_` 已通过：exact 2-table/1-view、schema 重装/重开一致、空业务行、完整性 UDF 与 18 个 V254 fence 原样在位；有效 receipt lineage/immutability 仍待 Linux execution fixture。 |
| startup matrix | 未运行；覆盖 disabled/path-present、enabled/path-missing/no-follow/controller/platform、与 V269/V270 enabled全部交叉组合。 |
| lineage/concurrency/crash | 未运行；证明唯一 genesis/successor、actor-bound replay、physical run非 exactly-once；commit前无row，rollback pending不授权，commit/promote gap仅同进程exact pending replay可promote，restart不可。 |
| process HMAC | 未运行；覆盖 canonical/observation/expiry/epoch任一 bit drift、DB伪造、错误 process registry、zeroize与 restart historical。 |
| root/TTL/revocation | 未运行；覆盖 V249/V250/V252/V268 successor/revoke/expiry、15 秒边界、V239 expiry无影响、historical latest revoke/replay。 |
| carrier neutrality | 未运行；跨 Provider exact neutral release可运行，carrier/root/source/launch漂移拒绝，durable/public/canonical JSON中无 carrier字段。 |
| 三条 HTTP | Windows Axum 已验证 401、member/local-owner 403、strict-shape 422 均先于 unavailable 503；201/200 replay、404/409、成功 currentness/revoke 与递归脱敏仍待 Linux runtime fixture。 |
| Store-private consumer | 未运行；同 connection/checked-at + fresh Prepared成功，缓存 GET、跨进程 receipt、过期/重启/撤销 receipt全部拒绝。 |

动态测试不得通过关闭、删除或缩窄 V254 fence 来制造 production route 成功；V272 positive fixture只在自己的
synthetic lane内运行，不创建 v213 或 market row。

## 5. 不属于 V272 的验收

以下均保持后置，不能记入 V272 passed：

- V273 actual v213 producer/worker、authenticated ACK/event ingress、production retry/recovery；
- stable production executor binding、route Adapter/version、credential、service actor、authorization、
  capability、seal 或 Start outbox；
- V249/V254/V255/V258/V259/V270 activation-rooted active refresh/successor；
- atomic Provider activation 与 V254 18-fence replacement matrix；
- Pool/Offer admission、真实任务执行、计量、market、settlement、部署或跨进程可携带外签证明。

## 6. 正式结论

V272 当前只能声明“Provider-neutral task-protocol conformance 合同已冻结，Windows contract/migration/HTTP
门卫及 WSL2 direct stateful oracle 已局部动态验证，Linux child/session server-run、wire authenticated ACK 与
process HMAC 尚未运行”。它可作为未来同进程 Store-private consumer 的输入，但不是 production executor、
route 或 activation authority。只有第 3、4 节全部动态矩阵通过并形成可复算指纹后，
才能提升实现状态；即使提升，Provider 仍为 `registering`、18 deny保持，atomic activation继续 NO-GO。
