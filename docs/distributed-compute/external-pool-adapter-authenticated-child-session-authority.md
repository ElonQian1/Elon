---
title: 外部矿池 Adapter authenticated child session core 权威
status: current
reviewed_at: 2026-08-14
owners: backend, security, ai-economy
implementation_status: implementation_partially_verified
---

# 外部矿池 Adapter authenticated child session core 权威

## 1. 唯一语义：预启动认证会话内核，不是 supervisor

V260 是 V259 declarative supervisor/session policy 之后的第一个真实 ephemeral 运行组件。它在 Linux x86-64 上创建匿名 Unix `SOCK_SEQPACKET` pair 和独立一次性 seed pipe，并在同一进程内把 host/child 两端提升为双向认证、严格有序、失败终止的 session。它只证明 IPC、密钥派生、握手和 frame gate；不创建或执行 child process，不执行 V257 capsule，不读取 V256 真实 config/credential，不连接 V258 target，也不形成 probe、route、readiness 或 activation authority。

当前 seed pipe 使用 `O_CLOEXEC`，因为本批明确不 spawn/exec。未来 supervisor 必须在受控 clone/exec 批次中独立实现 V259 规定的 fd3/fd5 拓扑：seed 只在唯一受控 exec 前清除 fd5 CLOEXEC，child 读满 32 bytes 后立即关闭。本批接口不得被误当作已经完成该跨 exec custody。

## 2. roots、密钥与 mutual bootstrap

调用方必须提交 exact lowercase nonzero SHA-256 profile、target、companion、capsule 与 bundle roots；实现再读取 server-fixed V259 policy catalog/digest，并拒绝 transport、protocol、ELSP、HKDF、HMAC、seed、nonce 或 key width 漂移。root transcript 逐标签绑定 policy/profile/target/companion/capsule/bundle，KDF salt 再绑定 policy/profile/target 和双方 32-byte nonce。

seed 和 nonce 由 `ring::rand::SystemRandom` 生成。HKDF-SHA256 extract/expand 派生 host-to-child 与 child-to-host 两把独立 key；`Secret32` 不实现 Clone、Copy、Debug 或 Serde，并由 `Zeroizing` 托管。固定尺寸 `ELS0` challenge、`ELS1` response 和 `ELS2` confirm 分别绑定 transcript、双方 nonce 和双向 HMAC proof；任一 root、seed、magic、version、length 或 proof 不一致都会 shutdown socket，并且不会产生 authenticated endpoint。

## 3. ELSP frame gate

application frame 固定为 20-byte header、payload 和 32-byte HMAC-SHA256 tag。header 包含 `ELSP` magic、version、kind、zero flags、big-endian sequence 与 payload length。kind 仅允许 control/config/credential；两方向序号独立从 1 开始并严格递增，每方向最多 1,048,576 帧。control/config 上限 1 MiB，credential 上限 64 KiB。

receive 先使用 server-fixed 1,048,628-byte buffer 和 `MSG_TRUNC` 取得一个完整 seqpacket，不使用 unauthenticated length 分配内存。实现依次校验 packet 下限、magic/version/flags、声明长度、HMAC、kind-specific limit 与 exact sequence，所有门通过后才复制并返回 zeroizing payload。MAC 绑定 direction、root transcript、header 和 payload，因此重放、乱序、跨方向反射、tag/payload 篡改、未知 kind、trailing bytes 与超限 frame 均失败关闭。

任何 send/receive 错误都会把 endpoint 置为 terminal、清零双向 key 并 shutdown socket；后续 send/receive 只能失败。Drop 同样清零 key 并 shutdown。该状态只存在内存，不写 DB、文件、日志、HTTP DTO 或环境变量。

## 4. 模块与可见性

实现仅在 Linux x86-64 编译，并私有于 `compute_federation`：

- `roots.rs`：V259 policy 与五项 runtime root 的 strict decode/transcript；
- `crypto.rs`：OS CSPRNG、HKDF-SHA256、HMAC-SHA256 与 zeroizing keys；
- `transport.rs`：anonymous seqpacket、ELSP frame、固定 buffer、sequence 与 terminal state；
- `bootstrap.rs`：seed pipe、fixed mutual proof 和 authenticated endpoint construction；
- `linux_tests.rs`：真实 Linux kernel socket/pipe 与 adversarial protocol fixture；
- `external_pool_adapter_supervisor_session_source_contract_tests.rs`：跨平台 no-effect 与 exact-source contract。

最大实现文件为 `transport.rs` 401 行；没有把 session 继续堆入 `compute_federation/mod.rs` 或既有 V259 巨型模块。模块没有 HTTP/MCP/router/main consumer，也没有 Store/migration/schema。

## 5. 已验证边界

Windows 项目统一验证执行 6 项 source-contract；Linux-musl 完整交叉构建分别生成 product 与 `elon-server` test binary；该静态测试 binary 随后在 WSL2 Ubuntu 的真实 Linux kernel 上执行 5 项测试，结果为 `5 passed / 0 failed / 1944 filtered out`。动态矩阵覆盖：

- real anonymous `SOCK_SEQPACKET`、`SO_ACCEPTCONN=0`、socket/pipe CLOEXEC 与 seed fd FIFO；
- host control 和 child config 双向正向 frame；
- root mismatch、corrupted seed、tampered bootstrap proof；
- frame tag tamper、replay、out-of-order、reflection、unknown kind、65,537-byte credential oversize；
- 首次 protocol error 后 endpoint terminal。

组合证据指纹为 `b3cd5820eb8c87f31e155008fb4b2b591dce9aec82aa361621fa089d9a1836a5`。详细命令、source validation receipt 与 fingerprint material 见对应 acceptance。

## 6. 仍未获得的 authority

V260 不证明 1 MiB control/config 在所有 Linux 默认 Unix socket buffer 上都可作为单个 seqpacket 成功发送；当前数值是拒绝超限的 protocol ceiling，不是 transport availability SLA。实现未调大 kernel buffer，也没有 fragmentation。后续 supervisor/transport 设计必须先通过 1 MiB boundary fixture，或以新的 server-fixed version 明确降低/分片，不能静默重解释 V259。

V260 也没有验证真实 spawn/exec、fd3/fd4/fd5 remap、seed post-exec custody、namespace/cgroup/seccomp/rlimit/pidfd/shutdown/reap、sealed capsule execution、production secret delivery、DNS/TLS/broker/upstream probe、Store-private same-transaction root composition、Provider readiness、route、market、usage、settlement、Sui 或部署。Provider 继续 `registering`，V254 18 项 absolute deny 原样保留。下一硬门是 supervisor 级 Linux enforcement 与受控 child lifecycle；完成前不得把本 session core 接到 production Adapter。
