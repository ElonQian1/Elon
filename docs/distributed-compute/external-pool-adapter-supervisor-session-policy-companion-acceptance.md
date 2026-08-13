---
title: 外部矿池 Adapter supervisor/session policy companion 验收边界
status: current
reviewed_at: 2026-08-14
owners: backend, security, ai-economy
implementation_status: implementation_uncompiled
verification_status: source_review_only
---

# 外部矿池 Adapter supervisor/session policy companion 验收边界

## 本批状态

V259 只交付 durable inert Domain、migration、Store、owner/admin Service/API、权威文档与源码合同。本批禁止编译、执行migration、运行测试或启动服务；实际执行证据固定为 `passed=0`。未执行 process/syscall、capsule exec、namespace/cgroup/seccomp/rlimit/pidfd、IPC/socketpair/session、secret读取或交付、DNS/TLS/network、probe/ACK/runtime identity、route/service actor、Provider activation、market、usage或 settlement。

## 待运行正向矩阵

- fresh database、V258→V259 upgrade、重复migration、文件重开与历史V249/V254/V255/V258逐字兼容；
- owner/admin policy GET返回同一 server-fixed policy/digest且零写；create fresh 201、exact actor-bound replay 200；
- fresh create在单一 Immediate事务、同一 near-now checked_at消费 current V258→V255 roots、fresh Prepared、Provider registering、V259及V257 policy roots；
- binding级唯一 genesis与单线 predecessor；target换代后 successor仍引用 binding structural latest companion，不能产生并行 current head；
- currentness重审 exact path/digests、latest/unrevoked、current V259/V258/V255 roots并返回九项 effect=`none`及七项 readiness=false；
- revoke fresh 201、exact replay 200；upstream/FS/policy漂移后仍能按historical exact authority撤销；已撤销latest可成为恢复 successor predecessor；
- public response递归脱敏所有 endpoint、secret、actor、idempotency及runtime/session private material。

## 待运行失败关闭矩阵

- 401/403/400/404/409/422完整状态；malformed JSON、顶层或 nested unknown field、body自报policy/actor/time/session/key/nonce/transcript/endpoint/secret/readiness均拒绝；
- path binding/candidate/profile/target/companion不一致，expected digest、server policy/capsule policy、Provider/Prepared/current target漂移，predecessor pair半空、遗漏latest、引用非latest、分叉，actor/idempotency material漂移；
- canonical JSON、materialized columns、digest、sequence、timestamp、status/effects/readiness漂移或直接SQL update/delete/replace；
- concurrent create/revoke只能有一个线性结果；所有失败比较前后 companion/revocation及下游表，证明零半写；
- currentness中任一 readiness=true，或源码触发任何 process、IPC、secret、network、probe、activation或market写入，均失败。
- fd拓扑必须逐项冻结为child IPC fd3跨exec、capsule fd4 CLOEXEC、seed fd5 `CLOEXEC=false`且在exec后/hello前精确读满32 bytes后关闭、`close_range`从fd6且UNSHARE、post-exec仅0/1/2/3/5、post-seed仅0/1/2/3、无network/target fd；seccomp须逐字冻结 `unknown_syscall_action="kill_process"`、`audit_arch_policy="x86_64_only_kill_other_arch"`、ordered bootstrap/runtime syscall arrays与5条argument rules。

## 源码合同与未来 seam

Service/API consumer源码扫描必须拒绝 `std::process::Command`、`tokio::process`、fork/exec/clone调用、namespace/cgroup/seccomp/rlimit/prctl/pidfd enforcement、socket/TCP/DNS/TLS、runtime bundle sensitive-byte consumer、probe、route与activation调用；Domain declarative catalog中的协议/syscall名称由另一项exact policy合同逐字冻结，不能误判为执行证据。migration源码合同必须冻结完整 receipt/policy投影、roots/lineage/timestamp/immutability，并校验V254 18 deny source SHA-256与trigger names exact parity。

future Store-private seam须证明同一Immediate事务与checked_at组合 current V259/V258/V255、V257 capsule、V256 locked bundle及V250/V252/V253 TTL roots，且authority不可Clone/Debug/Serde、raw endpoint与secret不越界。本批不实现或调用该 consumer。

因此本批只能记录 `source_review_only / passed=0`；未运行confinement fixture，ordered syscall catalog尚未证明足以启动真实 static ELF。不得宣称 supervisor、authenticated session、secret-safe delivery、Linux isolation、broker transport、probe、runtime readiness或production Adapter已验收。
