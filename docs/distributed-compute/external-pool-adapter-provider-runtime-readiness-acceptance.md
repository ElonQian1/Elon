---
title: 外部矿池 Adapter Provider runtime readiness 验收
status: current
reviewed_at: 2026-08-15
owners: backend, security, ai-economy
implementation_status: implementation_uncompiled
verification_status: source_review_only
---

# 外部矿池 Adapter Provider runtime readiness 验收

## 1. 本批验收强度

V270 当前只接受源码与权威合同静态复核。遵守架构阶段禁令，本批没有编译 Rust、执行 migration、运行
单元/HTTP/SQLite/Linux fixture、启动 child、读取真实 Secret、打开 production bundle、连接 upstream 或
生成动态指纹。正式计数为 `passed=0 / failed=0`，状态为
`source_review_only / implementation_uncompiled / implementation_unrun`。

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

## 3. 明确未运行的动态矩阵

当前没有以下证据：

| 验收面 | 当前结果 |
|---|---|
| fresh/repeat V270 migration、旧库升级、文件重开 | 未运行 |
| receipt/revocation projection、direct SQL、并发 CAS、崩溃恢复 | 未运行 |
| startup enabled/disabled/path/controller/平台矩阵 | 未运行 |
| locked-memory/HMAC zeroize、重启 epoch 失效、commitment 重算 | 未运行 |
| V267 current V2 child、V256 Secret、V264 TLS、V265 ELNW 与 cleanup fault matrix | 未运行 |
| 五条进程内 HTTP、真实 TCP、生产 upstream/Secret | 未运行 |
| Provider activation、18 项 replacement guard、market admission | 不属于 V270 |

历史 V261-V265 fixture、V266 Profile V1 或 V268/V269 source review 不能替代这些结果；current V267-V270
组合必须在架构阶段结束后按风险重新编译和运行。

## 4. 正式验收结论

V270 仅冻结“一个 exact Provider-specific probe 在 cleanup 后形成可撤销、最长 15 秒、重启即失效的
durable readiness history”的源码合同。它不是 production deployment、SLA、Provider activation 或开放市场
验收。只有后续动态矩阵全部通过并产生可复算指纹后，才能把实现状态从 `implementation_uncompiled` 提升；
即使提升，`activation_ready=false` 与 V254 fences 也必须保持到独立 atomic activation/admission 批次。
