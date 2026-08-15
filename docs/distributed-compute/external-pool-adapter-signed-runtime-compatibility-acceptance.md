---
title: 外部矿池 Adapter 签名运行时兼容性验证验收
status: current
reviewed_at: 2026-08-15
owners: backend, security, ai-economy
implementation_status: implementation_compiled
verification_status: source_review_only
---

# 外部矿池 Adapter 签名运行时兼容性验证验收

## 1. 当前批完成条件

- 独立 Profile V2 与 runner/fixture catalog；V266 Profile V1 字节、digest 与历史证据不漂移。
- Provider-neutral release lineage；不得出现 production config/credential、target/SPKI/DNS 或
  secret-derived root 的 durable 字段。
- migration 268 创建 append-only challenge、run observation、signed receipt、revocation 与 diagnostic
  current view；exact JCS/digest/scalar projection、single-use、lineage、expiry 和 current-root guard fail closed。
- challenge 使用 Store-generated 32-byte nonce、5 分钟窗口，并要求 caller 显式提交 expected
  V249/Profile/runner/fixture catalog digests、exact V237 key record ID/digest/key ID 与 structural
  predecessor；Store 不得隐式选择“最新”key/root。
- Store-private runner 复用 V267 derived launch image与 hardened supervisor/session，使用固定 catalog
  路径/约束下由 release manifest 声明的四个 public resource handles，成功 shutdown/reap/cleanup 后才
  允许 observation INSERT；不得把 fixture bytes 描述成 server-authored。
- Prepared installation 必须 exact 匹配 V249 admission/package/source/capability-set roots；runner、server
  constant 与 current session policy 的 no-work probe timeout 必须三方相等。
- `single-use` 只冻结每个 challenge 最多一个 durable observation/receipt；事务外执行可能在并发调度下
  重复受限本地 fixture run，竞争者只回放首个 durable row。不得声称 physical exactly-once；未来
  orchestrator 必须按 challenge 串行并保持幂等。
- dedicated child session 使用 exact 11-root runtime-compatibility constructor、两个新 domain bytes 与
  11 个新 argv prefix；legacy production 6-root ABI 原样不动，且不得映射 target/companion/bundle/Secret槽位。
- signed receipt 只接受 durable observation identity与 signature；canonical observation/DB不得保存
  signature message bytes/digest，private run仅返回瞬时 challenge，record 时 RSA message 必须由 Store 从
  durable challenge + final observation 重建，避免 digest 循环。
- private run receipt 仅向未来可信 server orchestrator 提供最小 observation ID/digest 与瞬时 signature
  message；本批没有 production orchestrator/independent-signer handoff caller，端到端 signed workflow
  不可达，且不得为补链新增 HTTP run/observation route。
- platform-admin（认证角色 `admin|owner`）Profile/challenge/record/currentness/revoke HTTP；没有
  Provider-owner `/api/me` 对称面、run-observation create、process、network、Secret 或 activation route。
- public redaction、九项 `none` effect、全部 readiness false、Provider registering 与 V254 18 deny 保持。

## 2. 必须拒绝的源码/数据路径

- caller-supplied observation、nonce、timestamp、profile/policy object、fixture bytes或 source/launch roots；
- duplicate challenge observation、duplicate observation receipt、stale challenge、expired receipt、wrong
  predecessor、parallel successor、revoked head、non-current V249/V237/Profile/runner/fixture；
- missing/wrong-role/oversize/drifted fixture resource、source或derived launch digest drift；
- non-V267 launch image、Yama gate失败、bootstrap/delivery/no-work/shutdown/reap/cleanup任一步失败；
- malformed/base64-noncanonical signature、wrong message digest、inactive或错误 V237 key、idempotency drift；
- direct SQL update/delete/replace、noncanonical JSON、arbitrary digest、broken projection或 readiness/effect升级；
- HTTP body 中出现 config/credential bytes/hash、endpoint/SPKI/DNS/address、session/delivery root、PID/fd/path。
- predecessor 只给 ID 或只给 digest、caller 提交 actor/idempotency scope/nonce/time/raw observation/policy
  object/key material、challenge 隐式选择 active key、unknown JSON field 或非 canonical Base64。

## 3. 静态验收矩阵

source contract 至少锁定：四个 fixture path、Profile V1/V2 version split、V259 V2/V257 V1 policy roots、
V267 source+launch双根、V237 signature algorithm、challenge→observation→receipt唯一关系、release lineage、
private runner visibility、无 observation DTO/run route、currentness non-authoritative、九项 none、readiness
false、五条 exact admin route且无 `/api/me` 对称面、无 V213/Provider/market写，以及 V254 18 trigger
names与既有 source SHA-256；还必须证明 API/Service 不调用 private run seam，并把 production
orchestrator/independent-signer handoff 明确列为 deferred，而不是伪造一个 public caller。

两阶段 runner source contract 还必须锁 preflight commit→事务外 execution→fresh `BEGIN IMMEDIATE` recheck→
唯一 observation INSERT 的顺序，并同时锁 race 分支返回既有 observation replay；该合同证明 durable
single-use，不证明 physical exactly-once。

session source contract 必须逐项比对 11 个 lowerhex64 field、root/KDF 两个 exact NUL-terminated domain
bytes及同序 11 个 argv prefix，证明 Store runner、supervisor child 与 static fixture 使用同一 dedicated
constructor；同时锁定 legacy 6-root constructor/domain/prefix 不漂移，且 V268 source 不出现 production
target/companion/bundle slot映射。

API source contract 还必须锁定 challenge/record/revoke 三个 `deny_unknown_fields` body 的 exact 字段集、
authenticated platform-admin actor 由 server 注入、Store 内部派生 idempotency scope，以及
401/403/400/404/409/422/500 分类。公开递归投影必须删除 nonce、signature message/signature、完整
observation、source/launch identity、runner/process internals、actor/idempotency/confirmation/raw JSON 和
所有 production Secret/endpoint roots；challenge/verification 还要整键删除内嵌 `registry_release` 与
`fixture_resources`，防止 nested manifest generic `sha256` 绕过字段名过滤。Profile catalog 与 safe
currentness summary 保留，九项 `none` effect 与全 false readiness 不得被升级。

migration static test 必须比对 DDL/INSERT exact ordered columns和 placeholder数量，并验证 raw SQL 对 stale
root、错误 JCS/digest、重复消费、分叉、revoked/expired head 与任意 readiness/effect升级均 fail closed。
这些均为源码合同，不计为动态 passed。

## 4. 后续动态矩阵（尚未运行）

- Cargo compile/test、migration apply/reopen、owner/admin HTTP及 direct-SQL negative matrix；
- Linux x86-64 real derived-launch fixture、Yama 2/3、ancillary injection、timeout与cleanup fault injection；
- real V237 independent signer、wrong-key/revoked-key/replay/concurrent lineage；
- Profile V2 checked-in machine JSON/digest reproducibility与 current catalog drift；
- V260-V267 regression、V270 current readiness authority，以及未来 atomic activation 组合；历史
  `V268 + fresh V265` 计划不得绕过 V270 的 cleanup、短 TTL 与同进程 custody reproof。

完整 Windows product check 与 WSL2 `elon-server` test target 已包含 V268 源码并编译通过，但没有
运行本节专属动态矩阵，因此计数仍为 `passed=0`、`failed=0`。不得把 implementation 标为 verified，
不得改变 Provider/route/activation/market authority。V269 的默认关闭 admin courier caller 已随目标编译，
但没有 unattended signer transport、私钥托管或自动签名闭环；不得据此宣称端到端 signed workflow 已闭合。
