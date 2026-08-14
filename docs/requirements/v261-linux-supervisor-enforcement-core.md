---
title: V261 Linux supervisor enforcement core
status: current
reviewed_at: 2026-08-14
owners: backend, security, ai-economy
---

# V261 Linux supervisor enforcement core

## 目标

在 Linux x86-64 上实现一个默认关闭、仅供 `compute_federation` 内部使用的外部矿池 Adapter supervisor enforcement core。该内核必须把 V259 固定的 Linux confinement 规则落实为真实 kernel 操作，并承接 V257 sealed capsule 与 V260 child session descriptor，形成可监督、可终止、可回收的单子进程生命周期。

本批只建立生产接线前的执行安全边界，不连接任何真实矿池，不读取或交付生产配置与凭据，不生成 probe、route、readiness、usage、settlement 或链上结果。

## 必须实现

1. 只允许 Linux x86-64；使用 `clone3`，同时要求 `CLONE_PIDFD` 与 `CLONE_INTO_CGROUP`，不允许 `fork`、`clone`、`Command` 或 PID fallback。
2. 每个 child 使用独立 cgroup v2 leaf；启动前写入 `pids.max=1`、`memory.max=256 MiB`、`memory.swap.max=0`、`memory.oom.group=1` 与 `cpu.max=100000 100000`。调用方必须提供已经委托且启用 cpu、memory、pids controller 的 parent，supervisor 不自行接管宿主 cgroup 根。
3. clone 时创建 user、mount、network、ipc、uts namespace，不创建 PID namespace；parent 显式写入一次性 uid/gid map，并设置 `setgroups=deny`。
4. child 创建 private mount propagation、空 tmpfs 根、`pivot_root`、私有 `/tmp` 0700；新根不得挂载 proc、sys 或 dev。
5. child descriptor 在 exec 时固定为：stdin/stdout/stderr 0/1/2、anonymous seqpacket fd3、sealed capsule fd4 且 CLOEXEC、32-byte seed fd5 且非 CLOEXEC；执行前 `close_range(6, UINT_MAX, CLOSE_RANGE_UNSHARE)`。
6. child 设置 V259 固定 rlimit、umask 0077、新 session、`dumpable=false`、`no_new_privileges=true`，并清空 effective/permitted/inheritable/ambient/bounding capability sets。
7. 安装 x86-64 seccomp BPF：错误架构或未知 syscall 使用 `KILL_PROCESS`；只允许 V259 bootstrap allowlist；`execveat` 仅允许 fd4 与 `AT_EMPTY_PATH`，`mmap`/`mprotect` 禁止 `PROT_EXEC`，网络、进程创建、mount/namespace、keyring、ptrace、BPF、perf 与 io_uring syscall 不得通过。
8. 唯一执行入口是 `execveat(4, "", ..., AT_EMPTY_PATH)`；不接受 path、shell、argv/env secret 或动态 fallback。
9. parent 仅通过 pidfd 发出 SIGTERM/SIGKILL，并通过 `waitid(P_PIDFD, ...)` 完成回收；Drop 必须失败关闭地终止并回收仍存活 child。
10. stderr 使用独立 nonblocking pipe，单次生命周期最多收集 1 MiB；发现溢出必须终止 child，且错误响应不得包含 stderr 原文。
11. child 退出并回收后移除 cgroup leaf 与宿主侧 scratch mountpoint；任一 setup、mapping、exec、协议或回收异常都不得返回“运行成功”。

## 验收标准

1. Windows 可运行 source-contract，证明模块仅在 Linux x86-64 编译，且没有网络、Store、Provider activation、真实 secret、HTTP、MCP、市场、结算或链上接线。
2. 完整 `elon-server` Linux-musl product 与 test target 可编译、链接。
3. WSL2 真实 Linux kernel fixture 以明确的委托 cgroup parent 启动 sealed minimal static ELF，验证 `clone3 + CLONE_PIDFD + CLONE_INTO_CGROUP`、五类 namespace、exact cgroup limits、exact rlimits、tmpfs/pivot root、capability 清零、NoNewPrivs、seccomp mode 与 fd3/fd5 跨 exec。
4. 正向 capsule 在读取并关闭 fd5 后只能观察到 fd0-3，通过 fd3 返回固定非敏感标记，随后由 pidfd 正常回收；cgroup 与 scratch 目录最终清理。
5. 调用网络 syscall 的 capsule 被 seccomp `KILL_PROCESS` 终止，不能返回成功标记。
6. 未委托 cgroup、缺少 controller、uid/gid map 失败、capsule fd 非 sealed/CLOEXEC 或 seed/session descriptor 不符合合同均失败关闭，不使用较弱 fallback。
7. pidfd 主动终止路径先 SIGTERM，超时后 SIGKILL，最终通过 `waitid(P_PIDFD)` 回收；不读取 `/proc` PID 作为控制句柄。
8. 功能注册表绑定当前需求与实现/test/document 证据，最终状态不得高于真实验收结果。

## 非目标

- 不组合 Store-private V250/V252/V253/V256/V257/V258/V259 roots，不读取真实 config/credential。
- 不执行 V260 mutual bootstrap 或 application frame；本批只证明 fd3/fd5 可以安全跨唯一 exec，完整 authenticated runtime wiring 留给后续版本。
- 不做 DNS、TLS、broker transport、upstream no-work probe、Provider activation、route 或 market admission。
- 不提供 HTTP、MCP、PC、Android 或商户 UI。
- 不部署生产 supervisor，不创建系统服务，不修改宿主 cgroup delegation 配置。
- 不宣称通用静态 ELF 已兼容 V259 syscall allowlist；fixture 只使用最小直接 syscall 程序。

## 依赖

- `compute-v260-authenticated-child-session-core` 必须保持 released 且证据 current。
- V257 sealed capsule 和 V259 supervisor/session policy 继续作为既有权威边界；V261 不修改其持久化状态机或公开 DTO。

## 预计实现范围

- `server/src/compute_federation/external_pool_adapter_linux_supervisor.rs`
- `server/src/compute_federation/external_pool_adapter_linux_supervisor/*.rs`
- `server/src/compute_federation/external_pool_adapter_linux_supervisor_source_contract_tests.rs`
- V260 bootstrap 的内部 descriptor transfer seam
- V257 capsule 的 compute-federation 内部只读 sealed image seam
- `docs/distributed-compute/` 下独立 authority 与 acceptance 文档
