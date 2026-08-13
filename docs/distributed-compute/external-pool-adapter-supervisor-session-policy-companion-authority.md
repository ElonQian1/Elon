---
title: 外部矿池 Adapter supervisor/session policy companion 权威
status: current
reviewed_at: 2026-08-14
owners: backend, security, ai-economy
implementation_status: implementation_uncompiled
---

# 外部矿池 Adapter supervisor/session policy companion 权威

## 1. 唯一语义：durable policy companion，不是 session

V259 为 exact V258 upstream transport target 与 exact V255 runtime launch profile 追加 Provider-specific、server-fixed、append-only 的 supervisor/session policy companion。它封存 future Linux supervisor 必须采用的 child-only IPC、双层 framing、stream authentication、KDF/key custody、nonce/sequence/transcript、timeout、confinement、resource、egress及 shutdown/reap 规则，消除 V255 UTF-8 JCS control framing 与 V256 arbitrary sensitive bytes之间的未定义边界。

companion 不创建进程、namespace、cgroup、seccomp、pidfd、IPC/socketpair 或 session，不执行 capsule，不读取或交付 config/credential，不生成 nonce/key/transcript，不做 DNS/TLS/network或 upstream probe，也不创建 service actor、route、Provider activation、market、usage或 settlement authority。状态固定为 `supervisor_session_policy_companion_current_inert`，记录效果固定为 `supervisor_session_policy_companion_recorded_inert`，撤销效果固定为 `supervisor_session_policy_companion_revoked`；adapter、runtime、provider、credential、route、execution、usage、market与settlement九项 effect 全为 `none`。

`process_spawn_ready`、`ipc_session_ready`、`secret_delivery_ready`、`broker_connect_ready`、`upstream_probe_observed`、`runtime_launch_ready` 与 `activation_ready` 七项 readiness 全部固定为 false。policy currentness 只证明惰性治理根仍 current，绝不表示任何运行时能力已存在。

## 2. exact durable roots 与不持久化边界

每条 companion 直接封存 exact V258 `target_id/digest`、target-policy digest，exact V255 profile/candidate/delegation/provider-binding/registry/installation/Provider roots、launch-policy digest，以及 process-isolation、resource与network-egress policy ID/revision/digest。它还封存 V257 server-fixed capsule policy root `external_pool_adapter_entrypoint_capsule_policy_v1` revision 1 与 digest `710decef25b4d19b33f086239f55f809a513508eb5ba431967971ff89249604f`；这是静态 policy binding，不是 capsule observation。

V259 不保存 V257 ephemeral capsule、fd、bytes或 materialization observation，不保存 V256 config/credential bytes、hash、bundle generation或locator，不保存 V250/V252/V253 短 TTL head，也不保存 session key、KDF输出、nonce、sequence counter、transcript、PID/pidfd、cgroup path、DNS answer、certificate或网络结果。这些只能由 future Store-private execution seam 在同一近当前时间点临时组合并保持借用。

## 3. server-fixed policy

首版仅允许 Linux x86-64 future supervisor，并固定：

- control frame 使用 bounded canonical JCS，binary-sensitive frame使用独立 bounded binary envelope；不得把 arbitrary bytes base64回填 V255 的 1 MiB UTF-8 JCS frame；
- 每一帧都必须有方向、类型、严格序号、长度与 session transcript binding，并在交付给解析器或 secret consumer 前完成流式认证；禁止 unauthenticated length allocation、重放、乱序、跨方向反射与 trailing bytes；
- session key只能由 server-held ephemeral key material经 server-fixed KDF派生，绑定 exact profile/target/companion/capsule/bundle roots与双方 nonce；不得复用 credential作为 session key，不得经 argv/env/log/DB/HTTP/公开 DTO暴露；
- child只能得到 anonymous `SOCK_SEQPACKET` child-only IPC：fd3跨exec保留；sealed capsule fd4仅供唯一一次`execveat(..., AT_EMPTY_PATH)`且CLOEXEC；seed fd5固定 `seed_fd_cloexec=false` 并跨exec保留，child必须在exec后、发送hello前按 `post_exec_before_hello_v1` 精确读满32 bytes后立即关闭。`close_range`从fd6开始且UNSHARE；exec刚完成只允许0/1/2/3/5，seed读取并关闭后只允许0/1/2/3。child不得得到listener、upstream socket、DNS/TLS handle、target fd或其它网络能力；server broker独占V258 transport target；
- startup/handshake/probe/shutdown期限、frame/secret/stderr/temp limits、failure terminal、zeroize、close与reap均由 server policy固定，任一异常失败关闭。

Linux confinement 固定为 x86-64、`clone3` + `CLONE_PIDFD` + `CLONE_INTO_CGROUP` 且无 fallback；新 user/mount/network/ipc/uts namespaces，host PID namespace仅由 pidfd监督；private tmpfs root + pivot_root、无 proc/sys/dev、private `tmp` 0700、capabilities全清、`no_new_privileges=true`、`dumpable=false`、umask 0077、new session/no controlling TTY。cgroup v2 dedicated leaf 固定单进程、256 MiB memory、zero swap、OOM group、1 CPU quota；rlimit逐项固定，child没有 network、mount、namespace、keyring、ptrace、BPF、perf或 io_uring能力。执行只允许一次 `execveat` sealed capsule fd4/`AT_EMPTY_PATH`；不得用 path、shell或动态 fallback。

seccomp catalog 的 public JSON keys 固定为 `unknown_syscall_action="kill_process"` 与 `audit_arch_policy="x86_64_only_kill_other_arch"`，且 architecture 为 `x86_64`。bootstrap allowlist按顺序固定为 `read, write, close, recvmsg, sendmsg, exit, exit_group, rt_sigaction, rt_sigprocmask, rt_sigreturn, sigaltstack, brk, mmap, mprotect, munmap, madvise, futex, clock_gettime, arch_prctl, set_tid_address, set_robust_list, rseq, getrandom, getpid, gettid, prlimit64, execveat`；post-exec runtime allowlist按顺序固定为同一列表仅去掉 `execveat`。argument rules按顺序固定为 `execveat_fd4_empty_path_only, mmap_prot_exec_denied, mprotect_prot_exec_denied, fcntl_dup_denied, ioctl_denied`，同时固定禁止post-exec executable mapping、process creation、network syscall及mount/namespace/capability/keyring/ptrace/BPF/perf/io_uring类操作。

shutdown 只能 pidfd 信号与 waitid：先 SIGTERM，最多 5000 ms 后 SIGKILL并完成 reap；禁止 PID fallback。stderr上限 1 MiB，溢出终止；runtime temp上限 64 MiB。以上只是 future supervisor 的 declarative catalog，本批没有运行 confinement fixture、没有调用任何对应 syscall，也没有证明这些allowlist足以启动任一真实 static ELF。

## 4. Store、lineage、撤销与 future execution seam

fresh create 在单一 `BEGIN IMMEDIATE` 与同一 near-now `checked_at` 中消费 current V258 target authority；该 authority递归保持 exact current V255/V254/V249与 fresh Prepared。Store还重验 server policy、capsule-policy root、Provider `registering`、actor-bound idempotency与 structural predecessor，然后服务端派生 ID、digest、sequence、time、status、effects与 false readiness并追加 immutable receipt。

lineage 按 `provider_binding_id` 全局单线：唯一 genesis、唯一 `(binding, sequence)`、每个 predecessor最多一个 successor。fresh successor必须引用该 binding structural latest companion的 exact ID/digest，即使 latest已撤销或绑定旧 target；这样 target换代不会生成多个 companion head。fresh create必须绑定 current V258 target，历史 target不能创建新 companion。exact replay只恢复历史 exact material，不被后续 upstream或policy漂移改写。

currentness要求 companion是未撤销 structural latest，V259 policy、V258 target及递归 V255 roots全部 current且同一 `checked_at`。fresh revoke仅消费 historical exact companion/target/profile roots并要求 companion仍为 latest且未撤销，不要求 filesystem、upstream target或server policy继续 current，确保失效 authority仍能安全终止；撤销后可把已撤销 latest作为下一条 companion的 predecessor恢复。

future supervisor不得串联两个独立 Store调用或两个时间点。唯一允许的实现形态是 Store-private、non-Clone/non-Debug/non-Serde 的 borrowed authority，在同一 `BEGIN IMMEDIATE`、同一 `checked_at` 中组合 current V259→V258→V255 roots，以及 V257 preparation→V256 locked bundle/capsule与 Store-selected current V250/V252/V253，并在副作用前后复验 retained handles/TTL/roots。V259 本批只预留该组合边界，不提供 consumer，不执行副作用。

## 5. owner/admin HTTP 与公开投影

owner/admin 提供同形 policy、create、currentness、revoke 管理面，路径嵌套 exact binding/candidate/profile/target。create body只接受 expected target/profile/candidate/provider-binding/session-policy digests、optional predecessor pair、idempotency key与显式 confirmation；policy全部由server catalog派生。body不得提交 policy字段、actor、时间、session/nonce/key/transcript、endpoint/SPKI、secret、runtime observation或 readiness。

`me` 仅允许 binding owner；`admin` 仅允许 platform `admin|owner`。fresh create/revoke返回 201，exact replay返回 200；鉴权、输入与冲突状态沿既有 V255/V258 管理面保持一致。

公开成功响应必须递归移除 actor/owner、idempotency、confirmation、receipt JSON、endpoint/SNI/SPKI、credential/config locator/commitment/bytes/hash、installation/entrypoint path、service actor/route projection，以及 session key/KDF/nonce/transcript、PID/pidfd/cgroup/runtime locator。stable IDs/digests、policy公开字段、inert status、none effects和false readiness仅供审计/CAS，不转移执行权威。

## 6. 市场硬门与实现现实

V254 的 18 个 temporary absolute deny trigger必须名称及SQL body逐字保留。V259 companion、currentness或任何 policy digest都不能替代 atomic activation/readiness admission gate；Provider继续 `registering`。

截至 2026-08-14，V259 仅处于 `implementation_uncompiled / implementation_unrun / source_review_only / passed=0`。下一批仍须先实现 authenticated child-only IPC与真实 Linux enforcement；之后才可由 server broker绑定 V258 target执行 DNS/TLS并形成 authenticated no-work observation。未完成 route/service actor、atomic activation、可信计量与结算前不得开放 market fence。
