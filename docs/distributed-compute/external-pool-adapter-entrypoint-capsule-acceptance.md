---
title: 外部矿池 Adapter Linux entrypoint capsule 验收边界
status: current
reviewed_at: 2026-08-14
owners: backend, security, ai-economy
verification_status: verified_wsl2_linux_kernel_subset
---

# 外部矿池 Adapter Linux entrypoint capsule 验收边界

## 本批状态

V257 已在 WSL2 Ubuntu、Linux `6.18.33.2-microsoft-standard-WSL2`、x86-64、Rust/Cargo `1.97.0` 环境完成完整 `elon-server` 测试目标编译。4 项 Linux kernel fixture 与 7 项 source-contract 全部通过，合计 `11 passed / 0 failed`。本批只 materialize 生成的非生产 static ELF 到匿名 memfd，不执行 capsule，不读取真实 mount/secret，不创建 child process、IPC/session 或网络连接。

数据库 schema 保持 V255；没有 `migration_v257`、receipt、current view、HTTP/MCP/PC route。Windows 与其他平台固定 unavailable。Provider 保持 `registering`，`probe_observed=false`、`runtime_launch_ready=false`、`activation_ready=false`；V254 18 个 temporary absolute deny 原样保留。

## 动态内核验收

4 项 fixture 直接经过生产 `with_external_pool_adapter_entrypoint_capsule` 路径并证明：

- 合规 source 经真实 `memfd_create` 形成 zero-link capsule，mode exact `0500`，带 `FD_CLOEXEC` 和 `F_SEAL_WRITE | F_SEAL_GROW | F_SEAL_SHRINK | F_SEAL_SEAL`；
- source、capsule 与权威输入的 size/SHA-256 exact，sealed capsule 的写入、grow、shrink 和增加新 seal 均由 Linux kernel 以 `EPERM` 拒绝；
- source mode 非 `0600`、出现额外 hard link、digest/size 错误或 ELF 变为 `ET_DYN` 时，consumer callback 不会执行；
- callback 返回后 capsule 被 Drop，先前观测的 descriptor 经 `F_GETFD` 返回 `EBADF`。

## 已执行 source-contract

7 项静态合同全部通过并继续锁定：

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

这些合同与 fixture 只证明当前源码和选定 WSL2 kernel 行为，不证明生产安装树、bundle mount、ACL、secret generation 或 runtime readiness。

## 命令与证据

执行命令：

```text
wsl.exe -d Ubuntu --cd /mnt/d/wt/24584-b68911b5/server -- env CARGO_TARGET_DIR=/tmp/elon-v257-cargo-target /home/siwmm/.cargo/bin/cargo test --locked --bin elon-server linux_kernel_ -- --nocapture
wsl.exe -d Ubuntu --cd /mnt/d/wt/24584-b68911b5/server -- env CARGO_TARGET_DIR=/tmp/elon-v257-cargo-target /home/siwmm/.cargo/bin/cargo test --locked --bin elon-server external_pool_adapter_entrypoint_capsule_source_contract_tests -- --nocapture
```

结果分别为 `4 passed / 0 failed / 1934 filtered out` 与 `7 passed / 0 failed / 1931 filtered out`。首次 source-contract 执行有 1 个单行字面量假失败；修正为锁定同一调用的稳定片段后 7 项通过，生产实现未改。最终验证指纹为 `2ac6c80f0d9c83f193090535c454851f3055fd835e3217dd3ca43bda79a35ebd`，由环境、两组结果及 facade、Linux production、kernel tests、source-contract、requirement 五个文件 SHA-256 的 canonical material 生成。

## 仍未验收

未验收生产 Linux kernel/文件系统配置、真实 V249 安装树与 operator mount、V256 真实 secret/zeroization、Store 同事务全根动态 fixture、短读/超读并发漂移、SQLite upgrade/reopen/concurrency/crash、真实进程执行、supervisor、namespace/seccomp/cgroup/Landlock/AppArmor、secret delivery、Sidecar/IPC/session、authenticated no-work probe、runtime identity、ACK/event、Provider activation、actor/route、Pool/Offer/Job/Attempt/Start、usage、verification或settlement。

因此当前只能记录 `implementation_partially_verified / verified_wsl2_linux_kernel_subset`。V257 不是 runtime、probe 或 activation；不得因真实 memfd fixture 通过而删除 V254 absolute deny 或宣称 production readiness。
