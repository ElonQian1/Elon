---
title: 外部矿池 Adapter Linux entrypoint capsule 验收边界
status: current
reviewed_at: 2026-08-14
owners: backend, security, ai-economy
verification_status: source_review_only
---

# 外部矿池 Adapter Linux entrypoint capsule 验收边界

## 本批状态

V257 只交付 Linux-only exact entrypoint capsule preparation、Store-private V250/V252/V253 同时点聚合、权威文档和 source-contract test 源码。按架构铺设阶段约束，本批不编译、不执行 migration、不运行测试或服务，不读取真实 mount/secret，不创建进程或网络连接；动态证据固定为 `passed=0`。

数据库 schema 保持 V255；没有 `migration_v257`、receipt、current view、HTTP/MCP/PC route。Windows 与其他平台固定 unavailable。Provider 保持 `registering`，`probe_observed=false`、`runtime_launch_ready=false`、`activation_ready=false`；V254 18 个 temporary absolute deny 原样保留。

## source-contract 待执行断言

静态测试源码必须锁定：

- capsule source 只能来自 fresh V249 Prepared 所持 exact entrypoint handle，caller/path/HTTP/DB 不能提供或重建 executable authority；
- Linux 使用匿名内存文件，copy 后设置 exact mode 与 complete write/grow/shrink seals；无命名临时文件、path reopen、Command/process、argv/env、IPC 或 network；
- source 的 regular/single-link/owner/mode 与 handle identity、size、SHA-256 在复制前后 exact；capsule 是 zero-link anonymous memfd，handle identity、size、SHA-256 在 seal 前后 exact；V249/V255 roots 任一漂移即失败关闭；
- V257 ephemeral capsule policy companion 固定为 `external_pool_adapter_entrypoint_capsule_policy_v1` revision 1；digest 必须来自权威页所列 exact ASCII/NUL-separated domain bytes，并等于 `710decef25b4d19b33f086239f55f809a513508eb5ba431967971ff89249604f`；它只在本次 preparation 内消费，不得写回、扩展或重解释 V255 的 45-field durable profile，V255 `deferred_to_runtime_supervisor` 保持；
- ELF gate 只接受 little-endian x86-64 ELF64 `EM_X86_64` static `ET_EXEC`，至少一个 `PT_LOAD`，非空 file ranges 不重叠，entry point 落在 executable `PT_LOAD`；拒绝 `PT_INTERP`、`PT_DYNAMIC`、`ET_DYN`、program-header 越界/溢出和任何 writable+executable segment；
- memfd permission exact `0500`，并同时具有防 grow、shrink、write 与 seal-set 变更的 exact 四项 seals；source 与 memfd size/SHA-256 exact；
- capsule capability、聚合 authority 与 V256 locked bytes 都无 Clone/Serde/content Debug，不暴露 path、fd number、bytes、secret hash 或 raw locator；
- Store 自己开启 `BEGIN IMMEDIATE`，自己生成 same near-now `checked_at`，内部选择 current V250/V252/V253 heads；caller 不能提交 checked_at 或 receipt/head ID/digest；
- V252 必须绑定同一 current V250 predecessor/root，V250/V252 必须绑定 exact V249 neutral release，V253 必须绑定 exact V249 Provider binding，三者与 V255/V256/Provider/release/config roots 同时 exact；
- V256 config/credential 只在只能返回 `Result<()>` 的短借用闭包中与 capsule handle 并置，本批没有 secret delivery consumer；
- 无 SQL write、migration、public API、runtime spawn syscall/Command、真实 probe/ACK 执行路径、Provider activation、route、market、usage、verification 或 settlement 副作用；内部 preparation 名称不能被解释为已执行 probe；
- fixed effects 只有 `materialized_ephemeral`，并固定 `probe_observed=false`、`runtime_launch_ready=false`、`activation_ready=false`；
- V254 18 个 market fence 名称/body 数量和规范化 SHA-256 `7d2971d0987e2c2939e0b212d4aedfa15a4b7cd3205e433eb7030f1371840de6` 不变。

这些断言即使未来执行通过，也只证明源码结构和选定 fixture；不证明生产 Linux kernel、memfd/seals、安装树、bundle mount、ACL、secret generation 或 runtime readiness。

## 待运行正向矩阵

- Linux fresh V249 Prepared entrypoint：retained handle exact、copy size/hash exact、capsule identity稳定、permission/seals complete；
- server-fixed companion policy root exact，合规 static ELF64 `ET_EXEC` 的 header/table/segments/entry point 全部有界且 W^X；
- 同一 Store-owned transaction/same near-now `checked_at` 下，current V255 + Store-selected V250/V252/V253 + V256 bundle + V249 entrypoint 全部 exact；
- V250/V252/V253 expiry 边界前 current，V252 exact predecessor/root 组合正确；
- authority 短借用完成后 Drop 释放 capsule handle 与 V256 locked bytes，崩溃/重启不能恢复旧 authority。

## 待运行失败关闭矩阵

- Windows/unsupported OS、匿名文件或任一 seal 不可用、source fd 不可 seek/read、短读/超读、size/hash/identity/metadata 漂移；
- ELF magic/class/data/machine/type/header/table/range 漂移，`PT_INTERP`、`PT_DYNAMIC`、`ET_DYN`、entry point 不在 executable load segment，或任一 W+X segment；
- source path reopen、命名临时文件、extra link、权限/属主不安全，或把 executable/secret 复制到普通 heap/String/log/DB；
- V249/V255/V256 任一 root 不 current，V250/V252/V253 head 缺失、被取代、撤销、过期或绑定错误，V252 predecessor 不是同次 current V250；
- caller 伪造 checked_at/head/receipt，SQLite transaction 回滚，filesystem 与 DB 观察窗口漂移，或 operator mount/locked-memory custody失败；
- 任一失败都不得留下 DB row、命名 capsule、进程、route、Provider version、市场或经济副作用。

## 仍未验收

未验收 Rust compile、unit/source-contract tests、Linux syscall/seal/permission/Drop 行为、生产文件系统与 kernel 配置、SQLite upgrade/reopen/concurrency/crash、Windows、真实 secret、secret delivery、Sidecar/IPC、process isolation、authenticated no-work probe、runtime identity、ACK/event、Provider activation、actor/route、Pool/Offer/Job/Attempt/Start、usage、verification 或 settlement。

因此本批只能记录：`implementation_uncompiled / implementation_unrun / source_review_only / passed=0`。V257 不是 runtime、probe 或 activation；后续不得因一次 capsule preparation 或三条签名 evidence current 而删除 V254 absolute deny 或宣称 production readiness。
