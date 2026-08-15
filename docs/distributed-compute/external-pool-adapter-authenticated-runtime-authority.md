---
title: 外部矿池 Adapter exec 后 authenticated runtime 权威
status: current
reviewed_at: 2026-08-15
owners: backend, security, ai-economy
implementation_status: implementation_partially_verified
verification_status: historical_v262_and_v267_wsl2_kernel_verified
---

# 外部矿池 Adapter exec 后 authenticated runtime 权威

## 1. 唯一语义：V260 会话在 V261 受限进程中的首次组合

V262 把 V260 的 mutual authenticated session core 真正放入 V261 的 Linux x86-64 `clone3`、cgroup v2、namespace、private tmpfs root、sealed capsule、seccomp 与 pidfd 生命周期。受限 capsule 只在唯一 `execveat(fd4, "", ..., AT_EMPTY_PATH)` 后，从 fd3/fd5 恢复匿名 `SOCK_SEQPACKET` 和一次性 32-byte seed，完成双向认证，并交换严格 HMAC、方向和序号保护的 ELSP control frame。

本批证明的是“真实 exec 后仍能在既有 confinement 内建立认证 IPC”。它不是生产 Adapter：不解析 V256 config/credential，不连接 V258 target，不产生 DNS、TLS、TCP、upstream probe、route、Provider readiness/activation、market、usage、settlement 或 Sui effect。Provider 继续 `registering`，V254 的 18 项 absolute deny 原样保留。

## 2. 单一协议实现与 authority 输入

V260 的 roots、HKDF/HMAC、bootstrap、ELSP frame 与 terminal fail-close 已迁入 Linux x86-64 专用 crate `elon-external-pool-adapter-session-core`。服务端 wrapper 负责加载并复验 server-fixed V259 policy；host 与测试 capsule 复用同一 core，不复制第二套密码学或 framing。

启动只传递六项严格小写、非零、64 字符 SHA-256 root：policy、profile、target、companion、capsule 与 bundle。它们是非敏感 authority identity，不是 config、credential、session key、Token、路径或命令。argv 固定为 argv0 加六个固定前缀参数，参数数量、顺序、前缀和字符集均由源码冻结；环境数组为空。任一 root 漂移都会使 bootstrap 失败，host 不获得 authenticated endpoint。

V262 没有获得 Store-private consumer authority，也没有把这些 root 组合为持久运行授权。未来生产 consumer 仍必须在同一事务、同一 `checked_at` 中借用 current V259/V258/V257/V256/V255 authority，并在副作用前后复验；本批固定测试 root 不能替代该组合。

## 3. descriptor custody、认证与 frame 生命周期

V261 launcher 在 clone 前把 child socket 改为 blocking，并保持固定 descriptor topology：fd0/fd1 为 `/dev/null`，fd2 为 bounded stderr，fd3 为 child seqpacket endpoint，fd4 为 sealed capsule，fd5 为一次性 seed，fd6 以上关闭。fd4 保持 CLOEXEC 并由 `execveat` 消费；fd5 仅为唯一 exec 清除 CLOEXEC，child 读满 seed 后立即关闭。认证完成后的 capsule 只保留 fd0-3。

host endpoint 使用 `ShutdownAndClose`，同进程 V260 child fixture 使用 `CloseOnly`，避免 child 侧错误关闭继承 socket 时对 host 执行不必要的双向 shutdown。任一 bootstrap、frame、child exit、stderr、timeout 或 cleanup 错误都保持 terminal fail-close；pidfd 是唯一 signal/wait authority，没有 PID fallback。

V262 fixture 的成功顺序固定为：host 发送 `v262.host.authenticated`，child 回送 `v262.child.authenticated`，host 再发送 `v262.shutdown`。这些只是固定非敏感验收 marker，不是业务 protocol、配置或凭据。

## 4. 实测后的 seccomp 最小修正

真实静态 Rust capsule 暴露了 V259 declarative allowlist 尚未覆盖的两个运行时形状。V262 没有宽泛放行，而是增加参数级 BPF 限制：

- `fcntl` 只允许在 fd3 或 fd5 上执行 `F_GETFD`；`F_DUPFD`、`F_DUPFD_CLOEXEC` 及其他 descriptor 操作继续 `KILL_PROCESS`；
- `poll` 只允许 Rust stdio 启动探测的 `nfds=3, timeout=0`，或认证 transport 的 `nfds=1, timeout=1..5000`；其他形状继续 `KILL_PROCESS`。

真实 kernel negative fixture 已分别证明 network `socket`、fcntl descriptor duplication 和未批准 `poll(nfds=2, timeout=0)` 以 `SIGSYS` 终止。该修正不增加网络、文件系统、process creation、mount、ptrace、keyring、BPF、perf 或 io_uring authority。

因为 server-fixed policy JSON 发生变化，policy digest 也随之变化。旧持久化 companion/profile 仍绑定其原 digest，不能被静默解释为新策略；currentness 继续按 exact digest 失败关闭。采用 V262 runtime 前必须通过线性后继 companion 或明确重新验证形成新绑定，不能覆盖旧记录或只修改展示状态。

## 5. 模块与发布边界

- `server/external-pool-adapter-session-core/`：host/capsule 共用的 roots、crypto、bootstrap 与 transport；
- `external_pool_adapter_supervisor_session.rs`：服务端 V259 policy wrapper 和类型化入口；
- `external_pool_adapter_linux_supervisor/launch.rs`：固定 root argv、fd3/fd5 与 clone lifecycle；
- `external_pool_adapter_linux_supervisor/seccomp.rs`：参数受限的 `fcntl`/`poll` BPF；
- `authenticated_runtime_tests.rs`：WSL/root 下的 exec 后正向与 root-drift fixture；
- `external_pool_adapter_session_fixture_main.rs`：仅测试构建的 static capsule。

fixture binary 只有显式 Cargo feature `external-pool-adapter-session-fixture` 才可构建，不是普通 `elon-server` 产品入口，也不进入发布脚本。V262 没有新增 Store、migration、HTTP、MCP、PC、APK 或 deployment consumer。

## 6. 已验证边界与下一硬门

Windows source-contract、Linux-musl product/fixture/test 完整链接和 WSL2 Ubuntu 真实 kernel fixture 均已通过。动态执行同时保持 V260 5 项协议测试、扩展后的 V261 5 项 confinement 测试和 V262 2 项组合测试，合计 `12 passed / 0 failed`；cgroup fixture 最终为空并成功清理。详细命令、工件 SHA-256 和组合证据指纹见对应 acceptance。

下一硬门是把 V256 的短时 config/credential authority 以 ephemeral、不可日志化、不可持久化的方式交付给已认证 child，并保持失败关闭；完成后才可由 server broker 使用 V258 target 做 DNS/TLS 与 upstream no-work probe。V262 不证明任一真实 Adapter 二进制兼容、Secret 已交付、外部矿池可连接、Provider 可激活或经济结算可执行。

## 7. V267 状态更正

V262 的历史 exec 后 fixture 使用 source capsule digest 和 supervisor/session V1，并没有
post-exec dumpable stub、Yama 2/3 gate、current policy V2、派生 launch root、ancillary rejection
或当前 cleanup 行为。普通 exec 重置 dumpable 的缺口意味着旧正向 bootstrap/control frame
不能作为当前 Secret 前置安全证明；旧 seccomp execveat flags 高位规则同样需要重新验证。

V267 已写入并编译上述修正，production materializer 也已接入 fixture。当前 WSL2/Yama 2
内核矩阵 `12 passed / 0 failed`，覆盖正向 bootstrap、root drift、seccomp negative、terminal
cleanup 与 kernel isolation。下文 V262 旧 `12 passed` 仍只作 provenance；Yama 3、第三方 ELF、
长期/并发 child 和生产故障矩阵仍须补跑。
