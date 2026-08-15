---
title: 外部矿池 Adapter authenticated child session core 验收边界
status: current
reviewed_at: 2026-08-15
owners: backend, security, ai-economy
implementation_status: implementation_partially_verified
verification_status: historical_v260_verified_v267_transport_source_review_only
---

# 外部矿池 Adapter authenticated child session core 验收边界

## 本批状态

V260 已完成独立 Linux x86-64 authenticated child-only session core，并通过 6 项 Windows source-contract、Linux-musl product/test 完整交叉构建和 5 项 WSL2 真实 kernel fixture。kernel 结果为 `5 passed / 0 failed / 1944 filtered out`，source-contract 验证指纹为 `da501d3c524f0b6c09d8d8e9ba44da4aebd36cf8bea5ae6eeb50a5bfe8db26fc`，receipt 为 `D:\rust\shared\rust-cache-v2\validation-v1\receipts\ce4f5e16731ddaea539b623bcb5990af706f95992839c9dd2ef4aef1399e3728.json`。

Linux kernel fixture 使用交叉构建出的 static musl `elon-server` test binary 在 WSL2 Ubuntu 内直接运行，不使用 Docker。测试执行时使用真实 `socketpair`、pipe、fcntl/fstat、poll、send/recv 和 shutdown syscalls；不是 mock 或源码模拟。

## V267 状态更正

V267 当前 session transport 新增 `MSG_CTRUNC`、`MSG_TRUNC` 与 ancillary control-data 拒绝。
下文命令、5 项 kernel fixture 和组合指纹发生在该变更之前，仍是 V260 旧源码 provenance，
不能证明当前 transport 已编译或运行。V267 对本页的新增验收为
`source_review_only / implementation_uncompiled / implementation_unrun`，
`passed=0 / failed=0`；至少须注入 `SCM_RIGHTS`、credentials、未知/截断 control data 并重跑
既有 frame/terminal 矩阵。

## 已执行验证

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-rust.ps1 -Domain agent-validation -- test --manifest-path server/Cargo.toml --bin elon-server external_pool_adapter_supervisor_session_source_contract_tests -- --nocapture

cargo zigbuild --locked --manifest-path server/Cargo.toml --target x86_64-unknown-linux-musl --bin elon-server

cargo test --no-run --locked --manifest-path server/Cargo.toml --target x86_64-unknown-linux-musl --bin elon-server

wsl.exe -d Ubuntu -- bash --noprofile --norc -lc '/mnt/d/rust/shared/target-v260-linux-musl/x86_64-unknown-linux-musl/debug/deps/elon_server-2c70de1c247189d1 external_pool_adapter_supervisor_session::linux_tests --nocapture --test-threads=1'
```

交叉 build/test-build 使用 `cargo-zigbuild` 生成的 Zig musl compiler/linker wrapper，临时目录与 target 均放在 D 盘。最终 product 和 test binary 均成功链接；随后只执行列出的 5 项 V260 tests。

## 动态矩阵

1. kernel topology：anonymous `SOCK_SEQPACKET`、非 listener、socket/seed fd CLOEXEC、seed fd 为 pipe；
2. happy path：mutual bootstrap 后 host-to-child control 与 child-to-host binary config 均保持 kind/payload；
3. bootstrap failure：profile root mismatch、真实 seed 翻转一位、response proof 翻转一位均使双方失败；
4. frame failure：tag tamper、replay、sequence 2 before 1、direction reflection、signed unknown kind、65,537-byte credential 均被拒绝；
5. terminal behavior：首次 protocol failure 后同一 endpoint 的 send/receive 持续失败。

source-contract 另冻结 V259 catalog/root binding、OS CSPRNG、HKDF/HMAC、non-Clone secret、fixed bootstrap、ELSP header/limits、fixed receive allocation、terminal zeroize，以及无 process/network/persistence/activation/real-secret effect。

## 组合证据指纹

组合指纹按以下 UTF-8 material 逐行 LF 连接后取 SHA-256：

```text
v260-evidence-v1
source_contract_validation=da501d3c524f0b6c09d8d8e9ba44da4aebd36cf8bea5ae6eeb50a5bfe8db26fc
linux_product_build=7456256137ffce4851c9d1bcc00fc5c2749e82a6a26666717821d110c5cfa73d
linux_test_build=398c2f912ff448e37c1edc097337775971b0e8da4800dda7ed6ffb6bcced9c29
linux_kernel_output=4f303c0d08427f62983407d4089c80db9bac887fc8897e1fd4cd7878312b8b63
```

结果为 `b3cd5820eb8c87f31e155008fb4b2b591dce9aec82aa361621fa089d9a1836a5`。

## 未验收与禁止声明

- 未执行 V257 capsule、未创建 child process、未验证 fd3/fd4/fd5 跨 exec topology；
- 未读取/交付 V256 真实 config/credential，测试 payload 只使用固定非敏感 bytes；
- 未执行 namespace/cgroup/seccomp/rlimit/pidfd/shutdown/reap supervisor fixture；
- 未验证 control/config 1 MiB positive boundary，Linux 默认 seqpacket buffer 可能在 protocol ceiling 前返回 transport error；
- 未创建 DNS/TLS/network/upstream probe、route/service actor、Provider activation/readiness、market、usage、settlement 或 chain effect；
- 未验证生产数据库、真实 Store-private root composition、真实 TCP、生产部署或长期压力。

因此下文 `verified_source_cross_build_and_linux_kernel` 只能标记 V260 旧 transport；V267
ancillary gate 当前为 `source_review_only / passed=0`。两者都不能证明 production supervisor、
secret delivery、runtime launch、broker transport、no-work probe 或 production Adapter 已完成。
