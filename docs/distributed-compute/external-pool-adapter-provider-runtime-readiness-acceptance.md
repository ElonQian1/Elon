---
title: 外部矿池 Adapter Provider runtime readiness 验收
status: current
reviewed_at: 2026-08-15
owners: backend, security, ai-economy
implementation_status: implementation_partially_verified
verification_status: targeted_local_verified
---

# 外部矿池 Adapter Provider runtime readiness 验收

## 1. 本批验收强度

V270 已随完整 Windows `elon-server` 产品目标和 WSL2 `elon-server` test target 编译；编译闭合了 SQLite
scalar helper 生命周期、Store error conversion 与 Linux-only installation filesystem error import。当前 Windows
受管 `provider_runtime_readiness` 矩阵 17/17 通过，其中包含 15 项源码合同与 2 项真实 SQLite
migration/integrity 用例；独立 process custody 矩阵 3/3 通过。对应指纹分别为
`00371751088b9a58da143145f21e8c7ad02045eb6c224c70ed675a690b9b3512` 与
`d63a8d249322129fc6a2d662ad231482c52dc480f8287621f342297011abc9ab`。当前状态提升为
`implementation_partially_verified / targeted_local_verified`，但尚未运行 HTTP、Linux production fixture、
真实 child/Secret/upstream 或完整并发故障矩阵。

本页只记录证据强度，不重新定义 V270 semantics；authority 唯一来源是
[`external-pool-adapter-provider-runtime-readiness-authority.md`](external-pool-adapter-provider-runtime-readiness-authority.md)。

## 2. 已冻结的静态合同

源码审查必须同时命中以下边界，缺一项都不能提升状态：

- 独立三环境启动 gate，Linux x86-64、delegated cgroup-v2 parent 与 V256 bundle-root no-follow custody
  失败关闭；V269 signing-handoff enable 不得隐式启用 production readiness；
- locked process HMAC key/custody epoch 不持久化，receipt 只保存私有
  `runtime_bundle_identity_commitment` 与 `post_cleanup_observation_commitment`，重启后旧 row historical；
- 六份 Prepared 按 broker pre/post、delivery bundle/session、post-cleanup bundle/session 分工晚绑定，任何
  SQLite transaction、connection 或 Prepared installation handle 都不跨 network/child await；
- V265 callback 只能位于 authenticated shutdown、pidfd reap、cgroup/scratch cleanup 成功之后；cleanup
  失败没有 final callback 或 readiness insert；
- final `BEGIN IMMEDIATE` 同一 connection/checked_at 重新消费 V249/V250/V252/V253/V254/V255/V258/V259，
  exact join current V268 receipt，并在 receipt insert 前比较 Provider、installation、source/launch、policy、
  target、Secret bundle 与 observation commitments；
- expiry 是 post-cleanup checked-at 加 current probe timeout（最大 15 秒）和 V250/V252/V253/V268 expiry 的
  最小值；不得延长为新的 capability TTL；
- append-only receipt/revocation、binding-global linear predecessor、actor-bound idempotency、immutable/
  projection/receipt-integrity/no-replace guards，以及 replay 不等于 physical exactly-once；
- observed readiness 只有 process、IPC、Secret、broker、upstream probe、runtime launch 六项 true，
  `activation_ready=false`；historical currentness 七项全 false，九项业务 effect 全 `none`；
- exact 五条 full-nesting `provider-runtime-readiness-receipts` HTTP route：admin trigger/currentness/revocation 与 owner currentness/revocation；
  owner 无 trigger，body unknown fields 失败关闭，公开投影不泄露 endpoint、Secret、hash、commitment、path、
  FD、PID、cgroup、actor、idempotency 或 raw receipt JSON；
- Provider 保持 `registering`，V254 18 个 absolute deny、v213 constructor fence、route/activation/market/
  execution/usage/settlement 边界逐字保留。

## 3. 当前动态证据与剩余矩阵

当前证据边界如下：

| 验收面 | 当前结果 |
|---|---|
| fresh/repeat V270 migration、全新文件库重开 | 2 项通过；显式重装保持 schema exact，非法 canonical receipt 被 UDF 拒绝 |
| process custody seal、提交晋升、身份漂移、重启 epoch、最长 15 秒窗口 | 3 项通过 |
| 停在 V269 的旧库升级 | 未运行 |
| receipt/revocation projection、direct SQL、并发 CAS、崩溃恢复 | 未运行 |
| startup enabled/disabled/path/controller/平台矩阵 | 未运行 |
| locked-memory/HMAC zeroize、commitment 重算 | 未运行；仅 process epoch 隔离与 seal exactness 已通过 |
| V267 current V2 child、V256 Secret、V264 TLS、V265 ELNW 与 cleanup fault matrix | 未运行 |
| 五条进程内 HTTP、真实 TCP、生产 upstream/Secret | 未运行 |
| Provider activation、18 项 replacement guard、market admission | 不属于 V270 |

历史 V261-V265 fixture、V266 Profile V1、V267 kernel subset 或 V268/V269 source review 不能替代这些
结果；current V270 专属矩阵仍须按风险运行。

## 4. 正式验收结论

V270 仅冻结“一个 exact Provider-specific probe 在 cleanup 后形成可撤销、最长 15 秒、重启即失效的
durable readiness history”的源码合同。它不是 production deployment、SLA、Provider activation 或开放市场
验收。当前只能声明目标化的本地 migration、integrity 与 process custody 子集已验证，不能声明 production
runtime readiness 已验收。后续必须补齐第 3 节剩余矩阵；即使全部通过，`activation_ready=false` 与
V254 fences 也必须保持到独立 atomic activation/admission 批次。
