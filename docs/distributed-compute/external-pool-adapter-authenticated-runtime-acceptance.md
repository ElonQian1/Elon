---
title: 外部矿池 Adapter exec 后 authenticated runtime 验收边界
status: current
reviewed_at: 2026-08-15
owners: backend, security, ai-economy
implementation_status: implementation_partially_verified
verification_status: historical_v262_fixture_superseded_v267_rerun_required
---

# 外部矿池 Adapter exec 后 authenticated runtime 验收边界

## 本批状态

V262 已把 V260 mutual authenticated session 放入 V261 真实 Linux child lifecycle。最终验证覆盖 Windows source-contract、Linux-musl product/fixture/test binary 完整链接，以及 WSL2 Ubuntu root 下 12 项真实 kernel test；结果为 V260 `5 passed`、V261 `5 passed`、V262 `2 passed`，无失败，并确认 `V262_WSL_CGROUP_CLEAN=true`。

V262 只验证仓库测试 capsule。它不读取真实 Secret、不连接 upstream、不启用 Provider、不写市场/计量/结算/链上状态，也没有发布或部署。

## V267 状态更正

V267 修复了下文历史 fixture 未覆盖的 exec 后 dumpable、Yama host gate、execveat 高位参数、
launch digest、seqpacket ancillary 与 cleanup 边界。旧 `12 passed` 运行的是 V1/source capsule
链，不能累计为当前 V267 验收。

当前结果严格为 `source_review_only / implementation_uncompiled / implementation_unrun`、
`passed=0 / failed=0`。V260/V261/V262 的全部正负向 kernel matrix 必须在 V2/derived launch
image 上重跑；现有 ignored runtime fixture 仍直接 seal 原测试 ELF，必须先改为经过 production
materializer，原样重跑不算 V267 evidence。在此之前本页历史验证标签只作 provenance。

## 已执行验证

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-rust.ps1 -Domain agent-validation -- test --manifest-path server/Cargo.toml --bin elon-server external_pool_adapter_supervisor_session_source_contract_tests -- --nocapture

powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-rust.ps1 -Domain agent-validation -- test --manifest-path server/Cargo.toml --bin elon-server external_pool_adapter_linux_supervisor_source_contract_tests -- --nocapture

powershell -NoProfile -ExecutionPolicy Bypass -File scripts\cargo-cross.ps1 -- zigbuild --target x86_64-unknown-linux-musl --manifest-path server/Cargo.toml --locked --bin elon-external-pool-adapter-session-fixture --features external-pool-adapter-session-fixture

powershell -NoProfile -ExecutionPolicy Bypass -File scripts\cargo-cross.ps1 -- test --target x86_64-unknown-linux-musl --manifest-path server/Cargo.toml --locked --features external-pool-adapter-session-fixture --bin elon-server --no-run

powershell -NoProfile -ExecutionPolicy Bypass -File scripts\cargo-cross.ps1 -- zigbuild --target x86_64-unknown-linux-musl --manifest-path server/Cargo.toml --locked --bin elon-server

wsl.exe -d Ubuntu -u root -- bash -lc '<create delegated cgroup parent; run V260, V261 and V262 kernel tests serially; prove leaf cleanup; remove parent>'
```

Windows direct binary 还执行：session source-contract `7/7`、Linux supervisor source-contract `8/8`、V259 policy companion 相关 migration/Store/source/HTTP `13/13`。Linux fixture 与 server test build 使用 `shared-cross-x86_64-unknown-linux-musl`，最终产物写入任务 worktree `.ai-tmp/cargo-cross-target/`；未创建新的版本号 target 缓存。

## 动态矩阵

1. 正向组合：sealed static Rust capsule 在唯一 exec 后从 fd3/fd5 完成 mutual bootstrap，并完成 host→child、child→host 和 authenticated shutdown 三个 control frame；
2. descriptor custody：seed 被读满并关闭，认证运行时只保留 fd0-3；
3. confinement 继承：child 保持 dedicated cgroup、五类 namespace、private root、固定 rlimit、capability 清零、no-new-privileges 与 seccomp；
4. root drift：任一固定 root 参数漂移时 child 退出 111，host 不获得 authenticated endpoint，pidfd 回收并清理 cgroup/scratch；
5. V260 回归：bootstrap root/seed/proof、frame tamper/replay/out-of-order/reflection/unknown/oversize 与 terminal behavior 继续通过；
6. V261 回归：clone3/cgroup/root/fd/limits、pidfd cleanup 与 network syscall kill 继续通过；
7. 新 seccomp negative：`F_DUPFD_CLOEXEC` 和未批准 `poll(nfds=2, timeout=0)` 均被 `SIGSYS` 终止；
8. 无副作用：fixture 无 V256 resolver、V258 transport、Store、Provider、market、usage、settlement、Sui、HTTP、MCP、PC 或 APK consumer。

## 工件与组合证据

| 证据 | SHA-256 |
|---|---|
| session source validation fingerprint | `95400bb7442a608ab52615b4098cd4a05e125b6eedcab845424eaefcfa64ed32` |
| Linux supervisor source validation fingerprint | `ccfbb0e2093733d3d63e5d89cd061cbd25560fb73554ed96f1a03a04483b732c` |
| Linux-musl `elon-server` product | `d3efed414197f4bcaf1f1c51f079e00a4a18a72e60870db6a031f686c56f11dc` |
| Linux-musl session fixture | `508543f0188dbf444df265e2ebe444f73e8f92377a9b1e6039d873e213393e51` |
| Linux-musl `elon-server` test binary | `c8eeeab33260305b8288a222a98d419d4ebea2136371dca1a2954c880a379f64` |
| final WSL kernel stdout | `9ac239f1a6b7ce69d473cd6c39923b6353bd8706fa3d5bbbc338bb10b7ef8984` |

组合指纹按以下 UTF-8 material 逐行 LF 连接后取 SHA-256：

```text
v262-evidence-v1
session_source_validation=95400bb7442a608ab52615b4098cd4a05e125b6eedcab845424eaefcfa64ed32
linux_supervisor_source_validation=ccfbb0e2093733d3d63e5d89cd061cbd25560fb73554ed96f1a03a04483b732c
linux_product=d3efed414197f4bcaf1f1c51f079e00a4a18a72e60870db6a031f686c56f11dc
linux_fixture=508543f0188dbf444df265e2ebe444f73e8f92377a9b1e6039d873e213393e51
linux_test_binary=c8eeeab33260305b8288a222a98d419d4ebea2136371dca1a2954c880a379f64
linux_kernel_output=9ac239f1a6b7ce69d473cd6c39923b6353bd8706fa3d5bbbc338bb10b7ef8984
```

结果为 `58cfb45b699180de2d65e8e65c2019732fa1fed8b611f3c6b96d03621d236a5d`。

原始 WSL stdout 位于主仓库 Git 私有日志目录 `.git/ai-command-logs/v262-wsl-kernel-fcntl-20260815-002428-597.stdout.log`，不是产品状态或发布工件。两个统一验证 summary 位于 `D:\rust\shared\rust-cache-v2\validation-v1\evidence\<fingerprint>\summary.json`。

## 未验收与禁止声明

- 未读取、解析或交付 V256 真实 config/credential；
- 未连接 V258 hostname/port/SPKI，没有 DNS、TLS、TCP 或 no-work probe；
- 未组合生产 Store-private current authority，也未验证历史生产库迁移；
- 未执行第三方 Adapter、动态链接二进制、并发多 child、长时运行、OOM/CPU failure 或生产 cgroup delegation；
- 未创建 route/service actor、Provider activation/readiness、market admission、usage、settlement 或链上 effect；
- 未开放 HTTP/MCP/PC/APK 入口，未发布服务器或安装包。

因此只能声明 V262 direct-seal 测试 capsule 的旧构建曾完成 source、cross-build 与 Linux
kernel fixture；current V267 runtime 为 `source_review_only / passed=0`，必须先接 production
derived launch materializer 再重验。不能声明生产外部矿池 Adapter、真实算力派发或结算已完成。
