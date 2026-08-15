---
title: 外部矿池 Adapter post-exec supervisor hardening 权威
status: current
reviewed_at: 2026-08-15
owners: backend, security, ai-economy
implementation_status: implementation_compiled
verification_status: local_windows_and_wsl2_kernel_verified
---

# 外部矿池 Adapter post-exec supervisor hardening 权威

## 1. 唯一语义：修复 exec 边界，不新增运行或业务权限

V267 修复 V261-V265 运行链中四类安全与生命周期缺口：普通 Linux `execveat` 会重置
`dumpable`，所以 exec 前的 `PR_SET_DUMPABLE=0` 不能证明 Secret 交付期间仍不可被同 UID
进程附加；旧 seccomp `execveat` 参数过滤对 flags 高 32 位的分支方向错误；认证
`SOCK_SEQPACKET` 接收没有显式拒绝 control truncation/ancillary data；部分失败与 Drop
清理路径不能稳定暴露终止、reap、cgroup 或 scratch 清理失败。

本批只加固既有 ephemeral Linux supervisor/session 链。它不创建新的 Provider、route、
service actor、durable readiness、任务派发、用量、结算或链上 authority，也不把历史运行
证据解释成当前源码已验收。Provider 继续保持 `registering`，V254 的 18 项 temporary
absolute deny 原样保留；V257/V259 的既有 readiness 字段仍全部为 false。

## 2. V257 source capsule 与派生 launch image

V257 的 source capsule policy 仍是
`external_pool_adapter_entrypoint_capsule_policy_v1` revision 1，固定 digest 仍为
`710decef25b4d19b33f086239f55f809a513508eb5ba431967971ff89249604f`。source capsule
继续逐字复制 fresh V249 retained entrypoint，保留原始 size/SHA-256、`0500`、zero-link、
`FD_CLOEXEC` 与四项 seals；V267 不修改该持久化 root，也不把派生物冒充 V255 profile
绑定的 source executable。

在 source capsule 已封存并完成 static ELF64 x86-64 `ET_EXEC` 审计后，V267 在内存中再派生
一份独立 sealed launch image。派生器有界重写 ELF/program-header table，重定位所有
file-backed ranges，新增只读可执行 `PT_LOAD`，并把 ELF entry point 指向该 segment 内的
可信启动 stub。非 `PT_LOAD` 的有内容 header 必须保持可证明的完整 source range；program
header 数量、offset、alignment、virtual range、输出大小与算术均有上限，任一不一致失败关闭。
launch image 同样只存在于匿名 memfd，固定 `0500`、`FD_CLOEXEC` 和四项 seals，不产生
pathname、cache、数据库行或公开下载物。

Store 私有 session 的第 5 项 capsule root 改为派生 launch image SHA-256；ephemeral delivery
binding 同时保留 source capsule SHA-256、launch capsule SHA-256 和 launch size，避免把
二者混为同一 material。V255 profile、V257 source policy 与 installation 审计仍绑定 source
SHA-256；只有本次 exec/session 绑定 launch SHA-256。source digest 仍可作为既有 V249/V255
durable root 出现；V267 新增的 source/launch 并置 binding 与 launch digest 不进入公开 DTO、
日志或新 durable receipt。

## 3. exec 后 dumpable 复位与 ptrace host gate

launch stub 在任何原 Adapter 指令、fd5 seed 读取或 Secret frame 处理之前只执行：

1. `prctl(PR_SET_DUMPABLE, 0, 0, 0, 0)`；
2. `prctl(PR_GET_DUMPABLE, 0, 0, 0, 0)` 并要求结果 exact 0；
3. 成功后跳转 source ELF 的原 entry point；任一步失败固定 `exit_group(127)`。

V267 supervisor/session current catalog 升级为
`external_pool_adapter_supervisor_session_policy_v2` revision 2。V2 在 bootstrap/runtime
allowlist 增加 `prctl`，但参数过滤只允许上述 SET/GET dumpable 零参数形状；其他 `prctl`
继续 `KILL_PROCESS`。唯一 exec 规则改为
`single_execveat_derived_launch_capsule_fd_4_at_empty_path_v2`，并修正 flags 高 32 位必须为
0 的 classic BPF 分支。历史 V1 policy 仍按原 JCS shape/digest 冻结并可验证历史记录，不能
被 V2 静默重解释。

stub 从 exec 返回到首条指令之间仍存在宿主同 UID ptrace 风险。V267 因此在 clone、capsule
传递或其他 child 副作用之前，以 no-follow 固定路径读取
`/proc/sys/kernel/yama/ptrace_scope`，只接受 exact `2\n` 或 `3\n`；缺失、不可读、非规范、
`0` 或 `1` 全部失败关闭。该条件是部署时的 host-admin trust boundary，不是对 root、内核、
已获等价特权主体或被篡改宿主的防护声明。

## 4. authenticated seqpacket 与 ancillary data

V267 继续使用固定大小 receive allocation 和 `recvmsg(..., MSG_TRUNC)`。接收端在解析任何
ELSP header、长度、MAC 或 payload 前，必须拒绝 `MSG_TRUNC`、`MSG_CTRUNC` 及 nonzero
`msg_controllen`。session 不接受 `SCM_RIGHTS`、credentials 或其他 ancillary data，也不能
把 control buffer truncation 降级为普通 frame。

该检查只收紧 transport envelope，不改变 ELSP/ELSD/ELNW 的 root、sequence、HMAC、长度或
业务语义，也不授予 child 网络或额外 descriptor authority。

## 5. lifecycle 与清理可观察性

显式 wait、terminate、launch rollback 和 stderr overflow 路径必须在 pidfd 有界观察后 reap，
再分别尝试 cgroup leaf 与 scratch root 清理；一个清理失败不能阻止另一个清理尝试，终止、
reap 或任一资源清理失败必须返回 supervisor 错误。Drop 只是最后的失败关闭兜底：它使用
pidfd SIGKILL、有界 poll 与 waitid，并以固定 `security` target 记录 signal、timeout、reap 或
post-reap cleanup 失败，不能把 fallback 当作成功回执。

这些日志不得包含 stderr 原文、Secret、root digest 或本机 path。
可观察错误也不等于外部监控、报警、自动修复或 crash-recovery 已接线。

## 6. durable policy 迁移与兼容性 Profile

`migration_v267` 只在一个 `BEGIN IMMEDIATE` 中重装 V259 current policy JSON projection
trigger/view，使 fresh companion 绑定 current V2 catalog。它不创建、修改或删除 receipt
table，不覆盖历史 V1 companion/revocation，也不重装 V257 exact-root trigger；V257 policy
仍为 V1。

V266 checked-in runtime compatibility Profile V1 继续显式使用冻结的 supervisor/session V1
catalog，因此其 JSON、digest 与 `6 passed / 0 failed` 历史验证不因 V267 被重写。它也不能
证明 V267 V2、派生 launch image、Yama gate 或加固后的 transport/lifecycle 已兼容。

后续 V268 已独立拥有 Profile V2、challenge、Store-private runner observation、V237 signed
receipt 与 revocation/currentness 链。V267 不再定义第二套 V2 Profile，也不复用 V266 V1
digest；本页的 V267 Linux 动态证据只验收 hardening runtime，不自动验收 V268/V269 的
签名兼容性、HTTP、SQLite lineage 或 signer handoff。

## 7. 当前实现与动态证据

V267 当前为 `implementation_compiled / local_windows_and_wsl2_kernel_verified`，但不是
production ready：

- Windows 受管 `cargo check` 已通过，证据指纹为
  `b37abea53faab919fa04ba40985b5ecbb41c1594ab09fc4af838500d54ed5c5c`；受管
  `cargo test ... v267_` 已通过，证据指纹为
  `6578adda332eb0404e25b8839a1ea4519e251506d0158a525f5a836f3859f62f`；
- 当前重放后源码在 WSL2/Linux 重新编译并运行 `v267_`，`12 passed / 0 failed`，覆盖
  fresh/repeat migration、历史 V1 回读、current V2 projection 与源码合同；
- V262/V263/V265 fixture 已改为调用 production source→derived-launch materializer。以
  Yama `ptrace_scope=2` 和 delegated cgroup v2 父组运行 `linux_kernel_`，`12 passed / 0 failed`，
  覆盖 mutual authentication、root drift、测试 Secret 交付/归零、authenticated no-work、
  clone3/cgroup/namespaces、post-exec stub、seccomp negative shape、pidfd reap 与资源清理；
- session-core 的 plain seqpacket、oversize、`SCM_RIGHTS` 与 kernel credentials 四项测试为
  `4 passed / 0 failed`；lifecycle 筛选中四项双资源清理故障测试均通过；
- 内核矩阵退出后恢复 `Yama=1`，根 `cgroup.subtree_control` 恢复为空，遗留
  `elon-v267-*` cgroup 数量为 0。

本轮没有验证 Yama `3` 及缺失/symlink/空值/`0`/`1` 的完整 host gate 矩阵，也没有覆盖真实
第三方 Adapter、生产 Secret/upstream、长期或并发 child、OOM/CPU 故障、宿主重启、生产监控、
route、atomic activation、market admission、可信用量或结算。Provider 继续保持
`registering`，V254 的 18 项 temporary absolute deny 原样保留。
