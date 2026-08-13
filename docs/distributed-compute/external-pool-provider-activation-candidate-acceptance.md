---
title: 外部矿池 Provider 激活候选验收边界
status: current
reviewed_at: 2026-08-13
owners: backend, security, ai-economy
implementation_status: implementation_uncompiled
verification_status: source_review_only
---

# 外部矿池 Provider 激活候选验收边界

## 本批状态

V254 的 Domain、migration、Store、Service/API 与源码合同已写入，但本批明确禁止编译、执行 migration、运行测试或启动服务。未连接 secret resolver/KMS/gateway、Sidecar/transport、Runner、Start send/ACK、真实矿池、可信 usage 或 settlement。实际执行证据固定为 `passed=0`；下列内容是待运行矩阵，不是通过声明。

## 待运行正向矩阵

- fresh database、V253→V254 upgrade、重复 migration、两次文件重开与 V249-V253 历史逐字兼容；
- exact V249 `Prepared` + owner 创建 delegation/candidate，同一 IMMEDIATE transaction 原子追加两行，服务端派生 actor/projection/compatibility/time；
- fresh create/revoke `201`、exact replay `200`，响应丢失重试保持相同 receipt；
- owner 与 admin static currentness `200`，状态必须为 `candidate_current_not_activation_ready`、closure 未实现且 `activation_ready=false`；
- owner 与 admin dynamic preflight 在同一 `checked_at` 消费 V249 Prepared + current V250/V252/V253，返回 `inputs_current` 但 `activation_ready=false`；
- owner revoke 后历史 receipt 保留，static currentness/preflight 均失败关闭；
- 所有成功响应通过递归 key 检查，不泄漏 actor、projection、route、credential locator、幂等、receipt JSON 或本机路径。

## 待运行失败关闭矩阵

- 无会话 `401`、其他已登录用户或非平台管理员 `403`、confirmation/reason/identifier/digest 非法 `400`、missing binding/candidate/delegation `404`、malformed/unknown body `422`；
- body 注入 service actor、actor phase、Provider/route/projection/logical binding ID、checked/issued time，一律 `422`；
- path binding 与 target binding 不同、非 owner create/revoke、expected digest、idempotency material 或 replay actor 漂移，一律零效果；
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

未验收 Rust compile、SQLite migration/upgrade/reopen/concurrency/crash、进程内或真实 TCP HTTP、生产数据库、MCP/PC、真实 filesystem drift、secret custody、route、Start、Runner、market、usage 或 settlement。因此只能记录 `implementation_uncompiled / implementation_unrun / passed=0`。

V254 已用数据库触发器对 `external_pool` CapacityPool active 与 Offer draft/active 安装 temporary absolute deny，覆盖 insert/update/version 与 direct SQL；它不是 runtime readiness 证明。后续 atomic activation 不得复用公开 preflight receipt 作为 authority，不得先写 Provider active 再补 route/runtime，也不得把 v213 control-plane sealed 记录当成 secret resolver/transport readiness；该批必须显式、原子地把 absolute deny 替换为完整 readiness/currentness admission gate，而不是删除 fence 或只在 HTTP 层放行。legacy Provider 的既有路径保持不变。
