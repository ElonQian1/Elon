---
title: 外部矿池 Adapter exec 后认证运行时 V1
status: current
reviewed_at: 2026-08-15
owners: backend, security, ai-economy
implementation_status: implementation_partially_verified
verification_status: verified_source_cross_build_and_linux_kernel
---

# 外部矿池 Adapter exec 后认证运行时 V1

## 目标

把 V260 的 ephemeral mutual authenticated session 放入 V261 的真实 Linux x86-64 `clone3`、cgroup、namespace、sealed capsule、seccomp 与 pidfd 生命周期中。受限 capsule 必须在唯一 `execveat` 后从 fd3/fd5 恢复 child bootstrap，完成双向认证，并至少完成一轮经过 ELSP HMAC、方向和严格序号校验的双向应用帧。

会话协议应抽成独立、聚焦的 Rust core，由服务端 host 与测试用 Adapter capsule 复用；不得在测试 capsule 中复制第二套 HKDF/HMAC/framing 实现。跨 exec 只允许传递固定 root 摘要、匿名 IPC 和一次性 seed：root 摘要是非敏感 SHA-256 身份，不得通过 argv/env 传递 config、credential、session key、Token、路径或命令。

## 非目标

- 不读取或解析 V256 config/credential，不实现生产 secret resolver。
- 不连接 V258 hostname/port，不做 DNS、TLS、上游网络或 no-work probe。
- 不创建 Provider activation、route、Pool、Offer、Job、Attempt、usage、settlement 或 Sui 交易。
- 不开放 HTTP、MCP、PC 或 APK 入口，不部署服务端，不把测试 capsule 声称为生产 Adapter。
- 不放宽 V254 的 18 项 temporary absolute deny，不改变 Provider `registering` 状态。

## 架构边界

1. 独立 session core 只负责 root transcript、一次性 seed、mutual bootstrap、ELSP frame 与 terminal fail-close；服务端 wrapper 继续负责校验 V259 server-fixed policy。
2. V261 launcher 只把六项严格小写、非零 SHA-256 root 作为固定位置的非敏感 argv 传给 sealed capsule；argv0、参数数量、前缀、长度和字符集均固定，环境保持空。
3. capsule runtime 只从固定 fd3/fd5 取得匿名 `SOCK_SEQPACKET` 与 32-byte seed；恢复前验证 fd 类型、CLOEXEC 和 socket 属性，读满 seed 后立即关闭 fd5。
4. host 只有在 child mutual bootstrap 成功后才获得 authenticated endpoint；握手失败、child 退出、stderr 超限或超时均通过 pidfd 终止、回收并清理 cgroup/scratch。
5. 动态验收 capsule 仅作为仓库测试工件构建，不进入生产发布路径，也不携带真实配置、凭据或网络能力。

## 验收标准

1. Windows/当前宿主 source-contract 证明 session core 独立模块、生产 server wrapper 无协议复制、测试 capsule 无 Secret/网络/激活入口。
2. Linux-musl 可完整链接服务端测试目标和静态测试 capsule；普通服务端构建不会把测试 capsule 作为运行入口。
3. WSL2/真实 Linux kernel fixture 证明 capsule 在 V261 exec 后通过 fd3/fd5 完成 V260 mutual bootstrap，并完成 host→child 与 child→host 两个认证帧。
4. 动态 fixture 证明 child 读 seed 后 fd5 关闭，运行时仅保留 fd0–3，且仍处于 V261 cgroup、namespace、private root、rlimit、capability、no-new-privileges 与 seccomp 边界。
5. root 参数漂移或 bootstrap 证明失败时，host 不产生 authenticated endpoint，child 被 pidfd 回收，cgroup 与 scratch 均清理，且不会产生网络、Provider、市场或经济副作用。
6. 现有 V260 协议对抗测试和 V261 kernel enforcement 测试保持通过，V254 absolute deny 不变。
7. 权威状态、acceptance 证据和 Feature Registry 明确区分“exec 后认证运行时已验证”与“真实 Secret、TLS/probe、生产 Adapter 仍未实现”。

## 预计实现范围

- `server/external-pool-adapter-session-core/`
- `server/src/compute_federation/external_pool_adapter_supervisor_session.rs`
- `server/src/compute_federation/external_pool_adapter_linux_supervisor/`
- `server/src/compute_federation/*source_contract_tests.rs`
- `server/Cargo.toml`
- `docs/distributed-compute/`
- `AI_CURRENT.md`

## 实现与验收结果

上述范围已完成，并保持测试 fixture 与产品入口隔离。Windows source-contract、Linux-musl product/fixture/test 完整链接及 WSL2 真实 kernel fixture 已通过；V260/V261/V262 动态测试合计 `12 passed / 0 failed`，组合证据指纹为 `58cfb45b699180de2d65e8e65c2019732fa1fed8b611f3c6b96d03621d236a5d`。权威边界和逐项证据见：

- `docs/distributed-compute/external-pool-adapter-authenticated-runtime-authority.md`
- `docs/distributed-compute/external-pool-adapter-authenticated-runtime-acceptance.md`

验收只证明 exec 后认证 IPC 组合；真实 Secret、TLS/upstream probe、Provider activation、市场/计量/结算和 Sui effect 仍为非目标且未实现。
