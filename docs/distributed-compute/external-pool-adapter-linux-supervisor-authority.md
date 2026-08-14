---
title: 外部矿池 Adapter Linux supervisor enforcement core 权威
status: current
reviewed_at: 2026-08-14
owners: backend, security, ai-economy
implementation_status: implementation_partially_verified
verification_status: verified_source_cross_build_and_linux_kernel
---

# 外部矿池 Adapter Linux supervisor enforcement core 权威

## 1. 唯一语义：受限子进程执行内核，不是生产 Adapter

V261 在 Linux x86-64 上把 V259 的 confinement policy 落实为真实 kernel 操作，并承接 V257 sealed capsule 与 V260 预启动 descriptor。它创建一个进入独立 cgroup 和 namespace 的子进程，以唯一的 `execveat(fd4, "", ..., AT_EMPTY_PATH)` 执行 capsule，并由 parent 使用 pidfd 监督、终止和回收。

本批只建立受控 child lifecycle。它没有执行 V260 mutual bootstrap 或 ELSP application frame，没有读取 V256 真实 config/credential，没有连接 V258 upstream target，也没有生成 probe、route、readiness、activation、usage、settlement 或链上结果。Provider 继续保持 `registering`，V254 的 18 项 absolute deny 保持不变。

## 2. 启动与 cgroup 权威

调用方必须显式提交已委托的 cgroup v2 parent。实现校验 parent 位于 cgroup2，并要求 cpu、memory、pids controller 已启用；它不会自行接管宿主根 cgroup，也不会使用较弱 fallback。

每次 launch 创建独立 leaf，并在 clone 前固定：

- `pids.max=1`；
- `memory.max=268435456`；
- `memory.swap.max=0`；
- `memory.oom.group=1`；
- `cpu.max=100000 100000`。

唯一 process creation primitive 是 `clone3`，同时要求 `CLONE_PIDFD` 与 `CLONE_INTO_CGROUP`，并创建 user、mount、network、ipc、uts namespace。V261 不创建 PID namespace，以便 parent 持有稳定宿主 PID 进行 setup 观察；生命周期控制只使用 pidfd，不用 `/proc` PID 发送信号或回收。

parent 在 child 进入后续 setup 前写入一次性 uid/gid map，并设置 `setgroups=deny`。任一 controller、leaf、clone、map 或握手失败都会终止并回收 child，再清理 leaf 和 scratch path。

## 3. 文件系统、描述符与执行权威

child 把 mount propagation 设为 private，在宿主 scratch mountpoint 创建空 tmpfs 根，建立私有 `/tmp` 0700，完成 `pivot_root` 后卸载旧根。新根不挂载 proc、sys 或 dev，不携带宿主文件系统路径。

exec 前描述符固定为：

- fd0/fd1：`/dev/null`；
- fd2：独立 nonblocking stderr pipe；
- fd3：V260 anonymous seqpacket child endpoint；
- fd4：V257 sealed capsule，保持 CLOEXEC；
- fd5：32-byte seed pipe，清除 CLOEXEC 以跨唯一 exec；
- fd6 及以上：`close_range(..., CLOSE_RANGE_UNSHARE)` 全部关闭。

capsule fd 必须是 regular memfd、mode `0500`、CLOEXEC，且同时具有 write/grow/shrink/seal 四项 seal。seed/session descriptor 必须符合 V260 内部 transfer contract。V261 不接受 path、shell、动态命令或 argv/env secret，也不提供 `Command`、`fork`、旧 `clone` 或 PID fallback。

测试 capsule 在 exec 后读满并关闭 fd5；真实 kernel fixture 观察到只剩 fd0-3。该结果只证明跨 exec custody 和最小 capsule，不代表任意静态 ELF 已兼容当前 seccomp allowlist。

## 4. confinement 与 seccomp

child 固定 umask 0077、独立 session、`dumpable=false`、`no_new_privileges=true`，清空 effective、permitted、inheritable、ambient 与 bounding capability sets，并应用 V259 固定的 file size、open files、process、address space、core、stack、memlock 与 CPU rlimit。

x86-64 classic BPF seccomp 对错误架构和未知 syscall 使用 `KILL_PROCESS`。allowlist 只覆盖 V259 bootstrap 所需 syscall；`execveat` 仅允许 fd4 + `AT_EMPTY_PATH`，`mmap`/`mprotect` 出现 `PROT_EXEC` 即终止。network、process creation、mount/namespace、keyring、ptrace、BPF、perf 和 io_uring syscall 不在 allowlist 中。

真实 fixture 已证明 capsule 调用 `socket(AF_INET, SOCK_STREAM, ...)` 时被 `SIGSYS` 终止；这不是网络模拟，也没有为测试临时放宽生产 filter。

## 5. pidfd 生命周期与资源清理

parent 只通过 `pidfd_send_signal` 发送 SIGTERM/SIGKILL，并通过 `waitid(P_PIDFD, ...)` 取得终态。主动终止先发送 SIGTERM，超过固定宽限后发送 SIGKILL；Drop 对仍存活 child 执行同样的失败关闭回收。

stderr 生命周期上限为 1 MiB，超过上限会终止 child；公开错误不包含 stderr 原文。child 回收后才移除 cgroup leaf 和 scratch mountpoint。cleanup 失败会作为 supervisor 错误返回，不能把部分清理误报为成功。

## 6. 模块边界

实现位于 `external_pool_adapter_linux_supervisor/`：

- `policy.rs`：冻结 V259 confinement 与 descriptor 合同；
- `cgroup.rs`：delegated parent、leaf、limits 与清理；
- `seccomp.rs`：x86-64 BPF 编译与安装；
- `child.rs`：namespace、root、fd、rlimit、capability 与 exec；
- `launch.rs`：输入复验、clone3 和 parent/child setup 协调；
- `lifecycle.rs`：pidfd wait/terminate、stderr bound 与 cleanup；
- `linux_tests.rs`：sealed minimal ELF 和真实 kernel fixture。

最大生产模块为 `launch.rs` 274 行，测试文件 498 行；没有把实现堆入 `compute_federation/mod.rs` 或 V260 模块。

## 7. 已验证边界与下一硬门

Windows source-contract、Linux-musl product/test 完整链接和 WSL2 Ubuntu 真实 kernel fixture 均已通过。动态矩阵覆盖 clone3/cgroup/namespaces/private root、exact fd、rlimit/capability/no-new-privileges/seccomp、pidfd terminate/wait，以及 cgroup/scratch cleanup。详细命令和组合证据见对应 acceptance。

下一硬门是把 V260 mutual authenticated session 真正运行在 V261 child lifecycle 内，并在不扩大权限的前提下接入 V256 ephemeral secret delivery；之后才可使用 V258 target 实现 broker TLS 和 upstream no-work probe。完成这些门之前，不得声明 production Adapter、真实外部矿池连接或 Provider 可用。
