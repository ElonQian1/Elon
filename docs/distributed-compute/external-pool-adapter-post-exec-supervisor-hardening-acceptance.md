---
title: 外部矿池 Adapter post-exec supervisor hardening 验收边界
status: current
reviewed_at: 2026-08-15
owners: backend, security, ai-economy
implementation_status: implementation_uncompiled
verification_status: source_review_only
---

# 外部矿池 Adapter post-exec supervisor hardening 验收边界

## 本批状态

V267 已写入 source/launch 双 capsule、post-exec dumpable SET/GET stub、Yama
`ptrace_scope=2|3` 启动门、supervisor/session policy V2、execveat 高 32 位参数修正、
seqpacket truncation/ancillary 拒绝，以及有界 pidfd/资源清理可观察性源码。V257 source policy
仍为 V1；historical V1 session policy 与 current V2 分开验证；Store session root 绑定 launch
SHA-256，同时保留 source/launch digest 和 launch size 的私有 binding。

本批按架构阶段约束没有运行 Rust 编译、migration、测试或 Linux runtime。当前只能记录
`source_review_only / implementation_uncompiled / implementation_unrun`，结果为
`passed=0 / failed=0`。不得把静态源码扫描、rustfmt、diff 检查或文档门禁写成 runtime
security evidence。

## 已完成的静态边界复核

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

以上只说明源码意图与跨模块绑定已被人工复核，不证明编译器、SQLite、Linux loader、seccomp
BPF、Yama、Unix socket 或 cgroup/pidfd 在真实环境中按预期工作。

## 历史证据的适用范围

V257 的 `11 passed / 0 failed` 仍只证明旧 source capsule materialization；没有覆盖派生 launch
image 或 stub。V260 的 `5 passed` 没有覆盖 ancillary/control truncation 新门。V261、V262、
V263 与 V265 的历史 WSL2/kernel/组合结果分别保留原命令、工件和指纹，但运行对象没有当前
V267 post-exec dumpable、Yama、policy V2、launch root、seccomp 修正及完整 cleanup 行为。
当前 ignored seccomp fixture 只观察参数级 BPF；V262/V263/V265 ignored runtime fixture 仍直接
seal 原测试 ELF，没有经过 production source→derived-launch materializer。原样执行这些测试
也不能形成 V267 acceptance evidence。

因此这些结果不能累计为 V267 的 passed count，也不能继续证明当前 V261-V265 runtime chain
已通过动态安全验收。V266 的 `6 passed / 0 failed` 仍是冻结 supervisor/session V1 Profile 的
历史机器合同验证；当前尚无 V2 compatibility Profile 或 verifier evidence。

## 必须补跑的动态矩阵

1. Rust product/test 编译、fresh/repeat V267 migration、历史 V1 companion/revocation 回读、
   current view 只接受 V2，以及 V254 18 deny/V257 V1 root parity；
2. source capsule与 launch image 的 short/over-read、ELF header/program-header overflow、多个或
   非法 `PT_PHDR`、独立及 `PT_LOAD` 内嵌 file range、alignment、overlap、size/hash/seal 漂移；
3. 真实 `execveat` 下 stub 必须在原 entry 前完成 SET/GET dumpable，失败走 exit 127；允许的
   prctl 形状通过，其他 option/非零参数与 execveat flags 高位篡改被 seccomp 终止；
4. Yama 文件缺失、symlink、空值、非规范值、`0`、`1`、`2`、`3` 的 clone-before-effect
   矩阵，并确认生产 host 配置与运维升级策略；
5. 正常 ELSP/ELSD/ELNW、oversize packet、`MSG_TRUNC`、`MSG_CTRUNC`、`SCM_RIGHTS`、
   credentials、未知 ancillary 与 terminal fail-close；
6. 正常退出、SIGTERM、SIGKILL、timeout、stderr overflow、launch rollback、wait/reap、cgroup
   与 scratch 单独/同时失败的故障注入，并核对返回错误和固定脱敏 security log；
7. 先把 V262/V263/V265 fixture 改为调用 production materializer 并分别核对 source/launch
   roots，再在真实 Yama 2/3、clone/seccomp/stub 下重跑 V260-V265 Secret delivery、loopback
   TLS/no-work、pidfd reap 和资源清理；仅重跑当前 direct-seal ignored fixture 不算通过；
8. 发布新的 V2 machine compatibility Profile、signed verifier/runner evidence 前，验证其不会
   复用 V266 V1 digest、challenge 或报告。

## 禁止声明

本批没有验证生产 Linux kernel、root/capability 边界、真实第三方 ELF、生产 Secret、生产
upstream、长期/并发 child、OOM/CPU failure、宿主重启、真实监控、route、atomic activation、
market admission、可信用量或结算。Yama 2/3 是外部 host-admin 前提，不是对 root、内核或
宿主失陷的隔离证明。

在上述矩阵产生新 evidence 前，不得声明 V267 已编译、测试通过、kernel accepted、
production-safe、runtime ready 或 activation ready；`passed=0` 必须保持。
