---
title: 外部矿池 Adapter Linux entrypoint capsule 权威
status: current
reviewed_at: 2026-08-14
owners: backend, security, ai-economy
implementation_status: implementation_uncompiled
---

# 外部矿池 Adapter Linux entrypoint capsule 权威

## 1. 唯一语义：exact executable materialization，不是进程

V257 为 exact V255 runtime launch profile 增加 Linux-only、服务端私有、按需且易失的 entrypoint capsule preparation 源码。它以 fresh V249 sealed installation authority 所持的已打开 entrypoint 为唯一 byte authority，把 exact executable bytes 复制到新的匿名、shmem-backed、没有命名文件系统 pathname 的 capsule file，完成复制前后 identity/size/SHA-256 复验，并把 write-sealed anonymous handle 与 V256 locked runtime bundle 一同封存在短生命周期 Store authority 中。

成功效果只允许描述为 `materialized_ephemeral`；`probe_observed=false`、`runtime_launch_ready=false`、`activation_ready=false`。V257 没有执行这些 bytes，不创建或启动进程，不调用 shell、Command、fork、exec、clone、container、VM 或 Sidecar，不建立 IPC/network，不传递 config/credential，不运行 authenticated no-work probe，也不产生 ACK、runtime identity 或健康观测。

capsule 是一次消费准备能力，不是 durable receipt、安装副本、cache、可执行路径、启动许可或 readiness。Drop、崩溃或进程重启后 handle 消失；不得从数据库、HTTP、日志或 path 重建。V257 不新增 migration/table/receipt/current view、HTTP/MCP/PC API，也不改变任何 Provider、route、market、usage 或 settlement 状态。

## 2. exact source 与 Linux custody

唯一 source 是 fresh V249 `PreparedExternalPoolAdapterInstallation` 内已经 no-follow 打开并保留的 exact entrypoint handle。调用方不能提供 source path、entrypoint bytes、digest、fd number、capsule name 或可执行参数。V255 的 private `entrypoint_relative_path` 只用于 Store 内 exact binding；V257 不把它重新解释为可供 path open 的 authority，也不从安装目录按字符串二次打开 entrypoint。

Linux capsule 必须满足：

- 只用匿名内存文件构造，不在 runtime bundle mount、安装树、临时目录或工作目录创建可命名文件；
- 创建时禁止继承 fd，并在写入完成后设置 exact executable/read-only permission 与不可再 grow/shrink/write 的 seals；
- copy 直接从 retained source handle 进入 capsule handle，不把 executable 全量复制到普通 `Vec`、String、环境变量、日志或数据库；
- source 在复制前后继续是同一 regular、single-link、owner/mode/identity/size authority；anonymous capsule 固定为 zero-link memfd，并在 seal 前后保持同一 handle identity；
- source size 和 SHA-256 必须同时等于 V249/V255 已签入的 entrypoint roots，capsule size/hash 必须与 source exact；短读、超读、漂移、seal 不完整或任一 OS 属性无法证明均失败关闭；
- 成功只由 Store sealed authority 私下持有不可 Clone/Serde/Debug 的 write-sealed handle capability；V257 callback 没有 handle getter，也不暴露 `/proc/self/fd/*`、本地 path、source fd/capsule fd 数字或 executable bytes。memfd 无命名 pathname 不等于能抵御同 uid 进程、宽松 procfs、调试器、root 或内核权限；部署必须另行限制这些读取面。

当前源码只实现 Linux。Windows 与其他平台固定 unavailable/fail-closed；不得用命名临时文件、宽 ACL、普通复制或路径 reopen 作降级。V257 也没有声称验证 IMA、fs-verity、签名加载器、内核完整性、恶意 root/debugger 或运行时内存篡改。

## 3. V257 ephemeral capsule policy companion

V255 的 45-field durable launch profile 保持原样，其中 `executable_verification_status=deferred_to_runtime_supervisor` 继续表示 ELF 结构审计尚未发生。V257 不回写、不扩展、不重新解释该 profile；它另外从服务端固定 catalog 取得 ephemeral capsule policy companion `external_pool_adapter_entrypoint_capsule_policy_v1`、revision `1` 及其 SHA-256 digest，并只在本次 preparation 内把 companion digest 与 exact profile/host/materialization roots 组合。digest material 是以下 exact ASCII bytes；字段顺序、大小写和 NUL 分隔不得改变：

```text
ELON-EXTERNAL-POOL-ADAPTER-ENTRYPOINT-CAPSULE-POLICY-V1\0revision=1\0linux\0x86_64\0elf64-le\0et_exec\0static-no-interp-no-dynamic\0no-wx\0sealed-memfd-v1
```

companion 不落库、不生成 receipt，也不因一次成功而改变 V255 currentness。

以上 146-byte material 的固定 digest 是 `710decef25b4d19b33f086239f55f809a513508eb5ba431967971ff89249604f`。

首版 companion 只接受 Linux x86-64 little-endian ELF64、`EM_X86_64`、System V 或 Linux OSABI 且 ABI version 0 的 exact static `ET_EXEC`。ELF header/program-header table 必须完整、有界且无溢出；至少一个 `PT_LOAD`，每个 `PT_LOAD` 的 file/memory size 均非零，file ranges 彼此不重叠，按 4096-byte x86-64 page 向外取整后的 memory mappings 也必须非空且彼此不重叠；entry point 必须落在一个 executable `PT_LOAD` 的 file-backed range 内。必须拒绝 `PT_INTERP`、`PT_DYNAMIC`、shared-object/PIE `ET_DYN`、可写且可执行的 segment、任何 W^X 冲突，以及 source/capsule size 与 program-header file range 不一致。memfd 必须 `fchmod(0500)`，并 exact 拥有防 grow、shrink、write 及 seal-set 变更的四项 seals。未显式允许的 ABI、machine、endianness、header/table shape 或 segment flags 均失败关闭。

这些限制只审计 executable 的静态结构，不证明指令安全、系统调用受限、动态 loader 不可达、运行时页权限维持、seccomp/cgroup/namespace 生效或程序会遵守 Sidecar 协议。capsule 也不是 locked secret memory，可能受宿主 swap/dump/procfs 配置影响。Windows PE 与 Linux dynamic executable 均不在 V257 支持范围。

## 4. Store-selected 同时点聚合

唯一消费入口位于 `crate::store` 的 sealed、非公开 authority。调用方只提交 exact Provider/profile identity、fresh V249 Prepared 与配置好的 V256 operator root；不能提交 `checked_at`，不能选择 V250/V252/V253 receipt/head，也不能传公开 currentness JSON、历史 DTO 或缓存投影。

Store 自己开启 `BEGIN IMMEDIATE`，生成同一个 canonical near-now `checked_at`，并在事务内重新组合：

1. fresh current V255 profile 与其 V254 candidate、V249 Provider binding/installation roots；
2. Store-selected current V250 vulnerability re-attestation head；
3. Store-selected current V252 sandbox re-attestation head，并验证其 exact current V250 predecessor/root；
4. Store-selected current V253 credential re-attestation head；
5. historical onboarding credential subject 与 V256 strict manifest/locked config+credential；
6. fresh V249 entrypoint handle 与本批 Linux exact capsule。

三条短 TTL evidence 必须在同一 `checked_at` current，且与 exact V249 neutral release、Provider binding、installation content、Provider/release/config/profile roots逐项一致。V257 不把 V250/V252/V253 写进 V255 durable profile，也不新增一份聚合 receipt；authority 只在事务和短借用期间存在。

SQLite、安装文件系统、operator mount 与匿名 capsule 不构成跨介质原子快照。retained handles 和同次 root 复验只证明本次 preparation 所见对象；它们不能把一次结果缓存为未来 launch authority。未来 supervisor 若要启动，仍须持有 fresh authority、在极短窗口重新审计数据库 TTL/roots 和 handles，并建立独立 authenticated probe/currentness 链。

## 5. V256 secret 与 executable 隔离

V257 复用 V256 的 strict manifest 与 locked-memory custody，但 capsule core 不读取 config/credential，也不接触 raw `vault-ref` locator。Store 聚合 authority 在一个只能返回 `Result<()>` 的短生命周期闭包期间私下同时保留 sealed capsule handle 与 V256 locked bytes；V257 callback 只看到固定 effect/readiness getter，既没有 handle getter，也不能借用 sensitive byte slices。本批没有任何 consumer 把 secret 写入 pipe、fd、argv、env、file、process 或 network。

V253 仍只证明 logical credential subject/commitment 的签名声明，没有观察 V256 `credential.bin` exact bytes。V250/V252 同样只认证签名报告，不证明本服务真实扫描或运行过本次 capsule。V257 的 exact executable copy 不补足这些现实缺口，不能从 roots 一致推导真实 secret、sandbox 或外部矿池认证已经发生。

## 6. 市场与激活硬门

Provider 必须继续 exact `registering`。V254 覆盖 direct SQL 与 versions 的 18 个 temporary market absolute deny trigger 逐字保留；capsule 或聚合 preparation 成功不能缩窄、替换或删除它们。V257 不创建 v213 service actor/credential/authorization/route/capability/seal/outbox，不推进 `registering -> active`，也不生成 Pool、Offer、Snapshot、Job、Reservation、Attempt、Start、usage、verification 或 settlement。

后续最小安全门仍包括：Linux authenticated supervisor、受约束的 secret delivery、Sidecar protocol/IPC/session authentication、exact runtime identity、no-work probe、ACK/event、资源/网络隔离，以及同一 atomic activation 中创建全部 Provider-specific runtime/route authority并提交 Provider 新版本。完整 readiness/currentness admission gate 动态落地以前，V254 18 deny 必须维持。

## 7. 实现现实

V257 源码只达到 `implementation_uncompiled / implementation_unrun / source_review_only / passed=0`。没有执行 Rust compile、migration、unit/source-contract test、Linux syscall fixture、服务、真实 mount、secret、process 或 network。文档中的 `must` 是 fail-closed 合同，不是动态证据；不能把源码存在表述为 Linux capsule 已验证、runtime 可启动或 production ready。
