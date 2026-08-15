---
title: 外部矿池 Adapter post-exec supervisor hardening 验收边界
status: current
reviewed_at: 2026-08-15
owners: backend, security, ai-economy
implementation_status: implementation_compiled
verification_status: local_windows_and_wsl2_kernel_verified
---

# 外部矿池 Adapter post-exec supervisor hardening 验收边界

## 本批状态

V267 已写入 source/launch 双 capsule、post-exec dumpable SET/GET stub、Yama
`ptrace_scope=2|3` 启动门、supervisor/session policy V2、execveat 高 32 位参数修正、
seqpacket truncation/ancillary 拒绝，以及有界 pidfd/资源清理可观察性源码。V257 source policy
仍为 V1；historical V1 session policy 与 current V2 分开验证；Store session root 绑定 launch
SHA-256，同时保留 source/launch digest 和 launch size 的私有 binding。

当前源码已完成 Windows 受管编译/定向测试与 WSL2/Linux 动态验证，状态为
`implementation_compiled / local_windows_and_wsl2_kernel_verified`。该结论只覆盖下述受控
fixture、Yama 2 和本机 cgroup v2 环境，不等于生产或第三方 Adapter 验收。

## 已完成的边界复核与动态验收

- V257 policy ID/revision/digest 与 source capsule exact roots 保持 V1，派生 launch image 是第二
  个匿名 sealed memfd；
- launch ELF entry point 先进入只允许 dumpable SET/GET 的 RX stub，再跳 source 原 entry；
- current V259 catalog 为 policy V2，historical V1 仍由 exact frozen builder 验证；V267 migration
  只重装 current policy projection，不改 receipt table 或 V257 roots；
- supervisor 在 clone 前只接受 exact Yama `2\n`/`3\n`，V2 seccomp 只允许零参数 dumpable
  `prctl`，execveat flags 高 32 位必须为 0；
- ELSP seqpacket 在 frame 解析前拒绝 `MSG_TRUNC | MSG_CTRUNC` 和 nonzero control length；
- Store session capsule root 使用 launch digest，profile/installation 与 source capsule 继续使用
  source digest；
- Provider、route、activation、execution、usage、market、settlement 与 Sui effect 没有新增
  authority，Provider 仍为 `registering`，V254 18 deny 保持。
- Windows 受管 `cargo check` 与 `cargo test ... v267_` 均通过；测试证据指纹分别为
  `b37abea53faab919fa04ba40985b5ecbb41c1594ab09fc4af838500d54ed5c5c` 和
  `6578adda332eb0404e25b8839a1ea4519e251506d0158a525f5a836f3859f62f`；
- WSL2 当前源码 `v267_` 为 `12 passed / 0 failed`；Yama 2 + delegated cgroup v2 的真实内核
  矩阵为 `12 passed / 0 failed`；
- session-core ancillary/truncation 为 `4 passed / 0 failed`；四项 lifecycle 双资源 cleanup
  fault case 均通过；
- 内核脚本在退出时确认 `Yama=1`、根 subtree control 为空、测试 cgroup 残留为 0。

这些结果证明当前提交在受控 Windows/WSL2 环境中的编译、SQLite migration、Linux loader、
seccomp BPF、Yama 2、Unix seqpacket 与 cgroup/pidfd 行为；未覆盖项仍受“禁止声明”约束。

## 历史证据的适用范围

V257、V260-V266 的旧指纹仍只属于各自历史提交，不能与本轮结果相加。当前 fixture 已改用
production source→derived-launch materializer，并在当前 V267 测试二进制上重新运行，因此本页
只引用本轮的 `12 + 12 + 4` 动态结果与 lifecycle 目标 case。V266 的 `6 passed / 0 failed` 仍仅
绑定冻结 supervisor/session V1 Profile。

V268 是唯一的 Profile V2 与 signed verifier/runner authority；它与 V269 当前只证明随完整
server/test target 编译，未运行其 migration、HTTP、signature、SQLite lineage 或 signer
handoff 动态矩阵。本页 V267 结果不得替代 V268/V269 验收。

## 动态矩阵进度

1. 已完成 Rust product/test 编译、fresh/repeat V267 migration、历史 V1 回读、current V2
   projection、V257 V1 root parity 与 V254 无新增 effect 复核；
2. 仍需补齐 source capsule与 launch image 的 short/over-read、ELF header/program-header overflow、多个或
   非法 `PT_PHDR`、独立及 `PT_LOAD` 内嵌 file range、alignment、overlap、size/hash/seal 漂移；
3. 已在真实 `execveat` 下通过 stub SET/GET dumpable、允许的 prctl 与 seccomp negative shape；
   exit 127 和全部参数组合仍需扩展故障注入；
4. 已验证 Yama `2`；仍需文件缺失、symlink、空值、非规范值、`0`、`1`、`3` 的
   clone-before-effect 矩阵及生产 host 运维策略；
5. 已验证 plain/oversize/`SCM_RIGHTS`/credentials；仍需显式 `MSG_CTRUNC`、未知 ancillary 和
   完整 ELSP/ELSD/ELNW terminal fail-close；
6. 已验证 pidfd reap、正常 cleanup 与 cgroup/scratch 单独/同时失败尝试；仍需 SIGKILL、timeout、
   stderr overflow、launch rollback 和固定脱敏 security log 的完整矩阵；
7. production materializer 已接入 V262/V263/V265 fixture，并在 Yama 2 下重跑测试 Secret、
   no-work 与 cleanup；仍需 Yama 3、真实生产 TLS/upstream/Secret 和长时/并发运行；
8. V268/V269 独立补跑 Profile V2、signed verifier/runner、HTTP/SQLite 与 signer handoff，不能
   复用 V266 V1 或本页 V267 结果。

## 禁止声明

本批没有验证生产 Linux kernel、root/capability 边界、真实第三方 ELF、生产 Secret、生产
upstream、长期/并发 child、OOM/CPU failure、宿主重启、真实监控、route、atomic activation、
market admission、可信用量或结算。Yama 2/3 是外部 host-admin 前提，不是对 root、内核或
宿主失陷的隔离证明。

允许声明 V267 已在当前提交完成 Windows 编译、WSL2 定向测试及 Yama 2 受控 kernel fixture
验收。不得声明 production-safe、第三方 Adapter compatible、runtime ready、activation ready
或 market ready，也不得把 V267 结果写成 V268/V269 signed compatibility 已动态通过。
