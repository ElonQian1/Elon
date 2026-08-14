---
title: 外部矿池 Adapter Linux supervisor enforcement core 验收边界
status: current
reviewed_at: 2026-08-14
owners: backend, security, ai-economy
implementation_status: implementation_partially_verified
verification_status: verified_source_cross_build_and_linux_kernel
---

# 外部矿池 Adapter Linux supervisor enforcement core 验收边界

## 本批状态

V261 已完成独立 Linux x86-64 supervisor enforcement core，并通过 Windows source-contract、Linux-musl product/test 完整交叉构建和 3 项 WSL2 真实 kernel fixture。kernel 结果为 `3 passed / 0 failed / 1957 filtered out`，组合证据指纹为 `b13a0c11d6cfa57b0d246109c5255110aa67edfeb0a4af3e687e632af092bee1`。

Linux fixture 使用交叉构建出的 static musl `elon-server` test binary，在 WSL2 Ubuntu root 下通过明确创建的 delegated cgroup parent 执行；测试结束后确认 parent 无 process、无 child leaf，再移除固定 fixture cgroup。不使用 Docker、mock supervisor 或源码模拟。

## 已执行验证

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-rust.ps1 -Domain agent-validation -- test --manifest-path server/Cargo.toml --bin elon-server external_pool_adapter_linux_supervisor_source_contract_tests -- --nocapture

cargo zigbuild --locked --manifest-path server/Cargo.toml --target x86_64-unknown-linux-musl --bin elon-server

cargo test --locked --manifest-path server/Cargo.toml --target x86_64-unknown-linux-musl --bin elon-server --no-run

wsl.exe -d Ubuntu -u root -- bash -lc '<create delegated cgroup parent; run V261 linux_kernel_ ignored tests serially; verify and remove parent>'
```

测试目标构建复用 `D:\rust\shared\target-v261-linux-musl`，并使用 cargo-zigbuild 已生成的 Zig musl compiler/linker/archiver wrapper。最终 source-contract fingerprint 为 `831c853fca865b61f82752f047423a3febb6515e969055f677f975a4ac4c1cd5`；统一验证 receipt 为 `D:\rust\shared\rust-cache-v2\validation-v1\receipts\6d772218be12342413de672f00a891c9a9f49a631e61e185a1ab29d60aee44c4.json`。

## 动态矩阵

1. positive lifecycle：`clone3 + CLONE_PIDFD + CLONE_INTO_CGROUP` 成功，child 位于唯一 cgroup leaf；
2. namespace/root：user、mount、network、ipc、uts namespace 与 parent 不同，新根为 private tmpfs，未挂载宿主 proc/sys/dev；
3. descriptor custody：capsule 读满并关闭 fd5 后只观察到 fd0-3，fd3 可返回固定非敏感 marker；
4. confinement：uid/gid map、NoNewPrivs、seccomp mode、capability 清零和固定 rlimit 与 policy 一致；
5. cgroup limits：pids、memory、swap、OOM group 与 CPU quota 精确匹配；
6. seccomp kill：最小 capsule 调用 network socket syscall 后以 `SIGSYS` 终止；
7. pidfd termination：parent 通过 pidfd 发送 SIGTERM，`waitid(P_PIDFD)` 返回 signal 终态；
8. cleanup：正常退出和主动终止后 cgroup leaf、scratch mountpoint 与最终 fixture parent 均不存在。

source-contract 另冻结 Linux x86-64 cfg、唯一 clone3/execveat/pidfd 路径、sealed capsule、descriptor mapping、private root、capability/seccomp/rlimit 合同、bounded stderr，以及无 Store、Provider activation、HTTP、MCP、network target、market、settlement 或 chain effect。

## 组合证据指纹

以下 UTF-8 material 按 LF 连接后取 SHA-256：

```text
v261-evidence-v1
source_contract_validation=831c853fca865b61f82752f047423a3febb6515e969055f677f975a4ac4c1cd5
linux_product_build=70073f6e967682054f046a6441e895e3d1a0c273449bae711bf95cbd6cadb697
linux_test_build=317ef57b42fb5ac9092f75c5ce318dd00c0c7f12d1249cb87018de4b5f19f970
linux_kernel_output=bf3d606b84cac9b9e1becf2a8669611f73e7f60845578ea9c6ebea22fed83f17
```

组合结果为 `b13a0c11d6cfa57b0d246109c5255110aa67edfeb0a4af3e687e632af092bee1`。kernel 原始输出保存在任务外部证据路径 `D:\rust\shared\tmp-v261\v261-linux-kernel-tests-final.txt`；它不是运行时状态，也不进入产品包。

## 未验收与禁止声明

- 未在 exec 后运行 V260 mutual bootstrap 或 ELSP application frame；
- 未读取或交付 V256 真实 config/credential，测试 seed 为一次性非生产数据；
- 未连接 V258 hostname/port/SPKI，没有 DNS、TLS、TCP 或 upstream no-work probe；
- 未组合 Store-private V250/V252/V253/V256/V257/V258/V259 roots；
- 未生成 route、service actor、Provider activation/readiness、market admission、usage、settlement 或 chain effect；
- 未验证动态链接程序、通用第三方 Adapter、长时运行、并发多 child、OOM/CPU hard-failure、生产 cgroup delegation 或公网部署；
- 未改变 V254 18 项 absolute deny，Provider 仍为 `registering`。

因此只能声明 V261 独立 supervisor enforcement core 已通过 source、cross-build 和真实 Linux kernel 验收；不能声明生产外部矿池 Adapter、真实算力派发或结算已完成。
