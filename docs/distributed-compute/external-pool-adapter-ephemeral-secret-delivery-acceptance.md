---
title: 外部矿池 Adapter 易失配置与凭据交付验收边界
status: current
reviewed_at: 2026-08-15
owners: backend, security, ai-economy
implementation_status: implementation_partially_verified
verification_status: verified_source_cross_build_and_linux_kernel
---

# 外部矿池 Adapter 易失配置与凭据交付验收边界

## 本批状态

V263 已把 V256 Store-owned 短时 config/credential 与 V262 exec 后 authenticated runtime 组合。最终验证覆盖两组 Windows source-contract、Linux-musl `elon-server` 与静态 fixture 链接，以及 WSL2 Ubuntu root 下 18 项真实 kernel test；session 与 supervisor 各 `9 passed / 0 failed`，并确认 `V263_WSL_CGROUP_CLEAN=true`。

V263 只交付仓库固定的非生产测试字节。它不连接 V258 upstream，不激活 Provider，不写 route、market、usage、settlement 或链上状态，也没有发布或部署。

## 已执行验证

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-rust.ps1 -Domain agent-validation -- test --manifest-path server/Cargo.toml --bin elon-server external_pool_adapter_supervisor_session_source_contract_tests -- --nocapture

powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-rust.ps1 -Domain agent-validation -- test --manifest-path server/Cargo.toml --bin elon-server external_pool_adapter_runtime_bundle_source_contract_tests -- --nocapture

powershell -NoProfile -ExecutionPolicy Bypass -File scripts\cargo-cross.ps1 -- zigbuild --target x86_64-unknown-linux-musl --manifest-path server/Cargo.toml --locked --features external-pool-adapter-session-fixture --bin elon-server --bin elon-external-pool-adapter-session-fixture

wsl.exe -d Ubuntu -- bash --noprofile --norc '<build elon-server test binary without running it>'

wsl.exe -d Ubuntu -u root -- bash --noprofile --norc '<delegate cpu/memory/pids cgroup; run V260-V263 session and V261-V263 supervisor fixtures serially; prove cleanup>'
```

普通 Windows cross `cargo test` 因宿主未安装 `x86_64-linux-musl-gcc` 未作为动态证据；产品和 fixture 由 Zig cross-build 完整链接，动态测试二进制改由 WSL 原生 Linux toolchain 构建。未使用 Docker，也未启动第二份重复构建。

## 动态矩阵

1. 正向交付：child 完成 V262 mutual bootstrap 后，按 `begin -> config -> credential -> receipt -> commit -> ready` 接收 exact bytes；
2. root 绑定：generation、长度、CSPRNG nonce 与两段 SHA-256 共同形成 delivery root，child 独立重算后才确认；
3. 失败关闭：host material drift、root drift、错误首帧和 payload 越界均不形成 delivered authority；
4. 清零与收尾：fixture 在 shutdown ACK 前清零 generation、root、config 和 credential，host 有界回收 child；
5. 隔离继承：真实 exec 后继续保持 cgroup、namespace、private root、rlimit、capability、no-new-privileges 与 seccomp 约束；
6. 负向回归：network syscall、fd duplication 和未批准 poll shape 继续被 kernel 拒绝；
7. 资源清理：两组测试后父 cgroup 无进程和子目录残留；
8. 无副作用：测试不连接 upstream，不修改 Provider、route、market、usage、settlement、Sui、HTTP、MCP、PC 或 APK 状态。

## 工件与组合证据

| 证据 | SHA-256 |
|---|---|
| session source validation fingerprint | `bfe899c4957c8cd8b6c9397a712d106ab4657ae7eba83b9dec8dc3e67b96aa66` |
| runtime bundle source validation fingerprint | `8c27cfff873501dddd1a539e6798a62b89292e9d7c010a2d091aee17609253f4` |
| Linux-musl `elon-server` product | `b0d3c221fc15edaa17ab57f1c222dbe15002a4facbee66b7951824b114622ead` |
| Linux-musl session fixture | `d3960349bce2aba50aef23b6e38ec353d0b8c6f59684d29a13d043da75b04c8e` |
| WSL native `elon-server` test binary | `165a20b189ab0d50bd1065ffd25c20a6a1704d911ff45c258bda02be80bf9f7a` |
| final WSL kernel stdout | `5d0e1c53fecae6b4d018df430d4999e878de470eb59ed4c02ab88a2b2b46dc54` |

组合指纹按以下 UTF-8 material 逐行 LF 连接后取 SHA-256：

```text
v263-evidence-v1
session_source_validation=bfe899c4957c8cd8b6c9397a712d106ab4657ae7eba83b9dec8dc3e67b96aa66
runtime_bundle_source_validation=8c27cfff873501dddd1a539e6798a62b89292e9d7c010a2d091aee17609253f4
linux_product=b0d3c221fc15edaa17ab57f1c222dbe15002a4facbee66b7951824b114622ead
linux_fixture=d3960349bce2aba50aef23b6e38ec353d0b8c6f59684d29a13d043da75b04c8e
linux_test_binary=165a20b189ab0d50bd1065ffd25c20a6a1704d911ff45c258bda02be80bf9f7a
linux_kernel_output=5d0e1c53fecae6b4d018df430d4999e878de470eb59ed4c02ab88a2b2b46dc54
```

结果为 `57a93aace9f3359494eafacf9a383d4f1120a741cec7f82cd396706952fe99a7`。原始 WSL stdout 位于主仓库 Git 私有日志目录 `.git/ai-command-logs/v263-wsl-kernel-2-20260815-030044-845.stdout.log`，不是产品状态或发布工件。

## 未验收与禁止声明

- 未使用生产 config/credential、真实矿池账号或第三方 Adapter；
- 未连接 V258 hostname/port/SPKI，没有 DNS、TLS、TCP 或 authenticated no-work probe；
- 未验证生产 mount、生产 cgroup delegation、长时运行、并发多 child、OOM/CPU failure 或主机重启；
- 未创建 route/service actor、Provider activation/readiness、market admission、usage、settlement 或链上 effect；
- 未开放 HTTP/MCP/PC/APK 入口，未发布服务器或安装包。

因此只能声明 V263 的测试 capsule 已完成易失配置与凭据交付的 source、cross-build 与真实 Linux kernel 验收，不能声明生产外部矿池 Adapter、真实算力派发或结算已完成。
