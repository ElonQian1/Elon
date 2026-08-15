---
title: 外部矿池 Provider 激活候选验收边界
status: current
reviewed_at: 2026-08-13
owners: backend, security, ai-economy
implementation_status: implementation_partially_verified
verification_status: local_rust_sqlite_axum_verified
---

# 外部矿池 Provider 激活候选验收边界

## 本批状态

V254 的 Domain、migration、Store、Service/API 与源码合同已编译，并通过 8 项 migration/源码合同测试与 2 项进程内 Axum HTTP 测试。正式验证指纹为 `806e156cb21d15700a304cf6d0843a793ef4a1c97c623af85dca985d0f4c000b`，回执为 `bbd5addbb5ae1bb71ca86d687fe6c38e2c0e6cebcdadf29511906e18e6278af9`。未连接 secret resolver/KMS/gateway、Sidecar/transport、Runner、Start send/ACK、真实矿池、可信 usage 或 settlement。

## 已运行本地矩阵

- migration/源码合同 8 项：fresh current schema 与重复 V254 migration、三张表的 persistence ABI、完整物化投影、静态 exact roots、线性 lineage 与撤销终态、temporary fence、升级 precheck；
- 进程内 Axum HTTP 2 项：owner/admin 鉴权，未知和 malformed body 拒绝，fresh create/revoke `201`、exact replay `200`，static currentness、dynamic preflight、递归脱敏、零经济副作用和 revoke 后失败关闭；
- fresh current schema 与重复 V254 migration 已动态执行；HTTP 测试夹具也会打开当前 Store 并执行完整当前 migration，但本批没有单独证明 V253→V254 文件升级或两次文件重开。

## 待运行扩展正向矩阵

- V253→V254 文件数据库 upgrade、两次文件重开与 V249-V253 历史逐字兼容；
- 响应丢失后的跨进程重试保持相同 receipt；
- 文件重开后 owner/admin currentness、dynamic preflight 与 revoke 历史保持一致；
- 多连接竞争、进程中断和磁盘故障下的原子性。

## 仍待扩展的失败关闭矩阵

- identifier、reason、idempotency material 或 replay actor 漂移的完整组合；
- installation tree missing/drift/unsafe，V249 binding/release/adoption/installation/Provider revision/digest/owner 漂移或 route projection collision；
- V250/V252/V253 receipt ID/digest 错误、过期、撤销、successor、key/root 漂移，或三者不是同次 Store currentness；
- delegation revoke 后 currentness、重复 active head、SQL update/delete/replace、canonical JSON 与物化列漂移；
- 任意路径出现 `active` Provider、v213 authority、Capacity/Offer/Job/Attempt/Start/usage/settlement 副作用。

## 零效果强制断言

每个 create/replay/currentness/preflight/revoke 成功后都必须比较写前写后：

- Provider status、policy revision 与 digest 原样为 exact `registering`；
- `compute_capacity_pools`、Offer、Price Snapshot、Job、Reservation 均零；
- Attempt activation、Execution Plan/Seal、Lease、dispatch 均零；
- v213 Adapter/version、credential/version/revocation、authorization/capability/seal、service actor authorization 均零；
- Start outbox、send attempts、remote ACK/observation/event 均零；
- usage declaration/snapshot/Execution Receipt、settlement/posting/ledger leg/付款均零。

V254 自己允许的唯一写效果是 delegation、candidate 与 owner revocation 表。固定 `none` 字段不能代替数据库差分证明。

## 仍未验收与 activation 禁线

已验收 Rust compile、fresh/repeat migration/源码合同与进程内 Axum HTTP 的上述 10 项。未验收 V253→V254 文件数据库原位升级、文件重开、并发/崩溃、真实 TCP、生产数据库、MCP/PC、真实 filesystem drift、secret custody、route、Start、Runner、market、usage 或 settlement。因此只能记录 `implementation_partially_verified`，不能记录 activation-ready 或 production-ready。

V254已用18个具名数据库trigger覆盖external-pool Provider/projection/route/CapacityPool/Offer与direct SQL；它不是runtime readiness证明。V275 docs-first只允许#1/#5-#12九个由同connection non-deterministic pending-plan UDF精确放行；#2-#4/#13-#18九个继续absolute deny，尤其active Provider INSERT和全部market写永不因V275放行。不得复用公开preflight receipt、先写Provider active再补route/runtime、把v213 seal冒充runtime readiness、只在HTTP层检查或删除trigger。V275分支当前严格`source_review_only / implementation_uncompiled / implementation_unrun / passed=0 / failed=0`；V254历史10项证据不覆盖pending-plan/direct-SQL/restart/原子回滚矩阵。
