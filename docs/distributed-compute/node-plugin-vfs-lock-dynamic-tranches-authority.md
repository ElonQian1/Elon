---
title: 节点插件 VFS Lock 动态执行切片权威 V1
status: current
reviewed_at: 2026-09-01
owners: node, security
design_status: design_frozen
implementation_status: q5_q6_q7_q8_source_written_uncompiled_unrun
verification_status: source_review_only_actual_not_run
authority_scope: backend-a2-map-lock-dynamic-quotient-authority-v1
---

# Node Plugin VFS Lock Dynamic Tranches Authority V1

## 1. Scope

本文维护 [`Map/Lock dynamic quotient authority`](node-plugin-vfs-map-lock-dynamic-quotient-authority.md)
中 Lock q5–q8 窄执行切片的精确成员、lower 路径 source contract、回执形状和隔离约束。父权威仍唯一维护完整
`8,668` 静态分母、商集冻结、reviewed inventory、`Qlock` 与生产门控；本文不创建第二套
CaseKey、Expected、manifest 或 acceptance 状态。

## 2. q5 native-acquire NodeLive busy

`a2lockq5` 只匹配 44 个 `LockNativeAcquire + NodeLive + native-busy + Completed` acquire
range：8 个 shared 单槽与 36 个 exclusive 连续范围。独立 Win32 handle 持锁跨越 installed
`xShmLock`；receipt 绑定同 FileId/不同 handle、真实 Busy、零状态漂移与 cleanup，拒绝 synthetic
Busy 和 same-handle overlap。

q5 catalog 为 44 rows，SHA-256=
`b12bd411f7fa63f822e65a679351dfc103a6368e2887355d5b03c530fc162e2f`；它仍是
uncompiled/unrun source contract。

## 3. q6 local sibling-contention completed

`LockLocalSiblingContentionCompletedV1` 精确匹配 44 个
`LockLocalState + Completed + BusyNoMutation` member：8 个
`LockShared + SiblingExclusiveContention` 单槽与 36 个
`LockExclusive + SiblingAnyContention` 连续范围；不得解析 `leaf_id` 或吸收 q2–q5。

双连接 sibling 先经 installed `xShmLock` 持锁：shared case 使用同槽 exclusive，exclusive case
逐槽使用 shared；selected 再在真实 coordinator gate 返回 `SQLITE_BUSY`。receipt 要求 selected
零 native call、零 mutation/poison drift、setup/cleanup ledger 隔离，并在 callback 后显式清理
sibling。该批当时的未运行 inventory 预期为 `2,842 present / 5,826 missing`，已被 q7 当前聚合值取代。

## 4. q7 callback-completion route unknown

`LockCallbackCompletionRouteUnknownV1` 精确匹配 192 个 ordinary
`completion=RouteUnknown` member，只含六个子族：

- `LockNativeAcquire + NodeLive + Acquired`：44；
- `LockNativeAcquire + NodeLive + NativeBusy`：44；
- shared-local acquire success：8；
- local sibling contention Busy：44；
- native release success：44；
- shared-local release success：8。

shared 只允许 8 个单槽，exclusive 只允许 36 个合法连续范围。不得吸收 unsafe-retention、
native-error、pre-admission 或其他 terminal；不得改变 Map 或 q1–q6 selector/version 语义。

每个 child 必须先经真实 installed `xShmLock` 路径形成 lower result 与严格 receipt，再由 exact
request/route 绑定的 test-only one-shot seam 调用 production `retain_terminal_custody` 移除该 Lock
route，最后调用真实 `callback.complete()` 并得到 `UnknownOrRetired`。顺序固定为：

```text
real lower result
  -> exact route/request/outcome one-shot claim
  -> production terminal-custody retention and route removal
  -> real callback completion
  -> route-unknown receipt
```

receipt 必须封口 callback begin=`1`、completion attempt=`1`、route removal=`1`、completion
unknown=`1`，并保留相应 native/local/sibling lower receipt。不得注入 completion 结果、不得在 lower
operation 前预删 route、不得以 synthetic native error 代替生产路径。wrong route/request/outcome 不得
消费 claim；exact claim 只能成功一次；Map classifier 固定为 `None`，ordinary `Err` 不进入该 seam。

q7 catalog 精确为 192 data rows、39,203 bytes、LF-only、无 UTF-8 BOM，按 canonical expected-row
顺序逐行绑定 path/action/range 与 case/full seals；static fixture 再将这些 tuple/seals 精确绑定到 frozen
authority leaf。SHA-256=
`e9f509d52d294405dd1a7ae528c514a31ba4e0758959374b633bdca2b571d631`。六组为
上述语义顺序的 `44/44/8/44/44/8`；catalog physical order 为 `44/44/44/44/8/8`，且与
q1–q6 catalog member seals 零重叠。

## 5. q8 local protocol own-overlap or not-held completed

`LockLocalProtocolOwnOverlapOrNotHeldCompletedV1` 的 source matcher 只接受 88 个
`LockLocalState + RequestValidation + ProtocolViolation + Completed` member：

- own-overlap 共 44：`LockShared` 8 个单槽，`LockExclusive` 36 个八槽内非空连续范围；
- shared-not-held 共 8：`UnlockShared` 8 个单槽；
- exclusive-not-held 共 36：`UnlockExclusive` 36 个八槽内非空连续范围。

matcher 必须同时复核 typed source site、prestate、operation、phase、timing、occurrence、完整
Expected、`first/count/mask` 与 committed case/full-record seals，不解析 `leaf_id`。它明确排除 36 个
`ExclusiveRangeMismatch + Completed` member，并排除上述语义的全部 `RouteUnknown` member；任何其他
range-mismatch、route-unknown、q1–q7、native error、stored poison 或 admission rejection 也不得被吸收。

q8 的 child/fixture/ledger/payload 源码只描述下列未来 actual 路径，当前没有执行回执：own-overlap 先由同一
selected WAL-main connection 经 installed `xShmLock` 成功取得 exact range，再在 observation ledger arm 后对
同一 range 发起第二次 installed `xShmLock`；not-held 从已 attach、held masks=`0/0` 的 active selected route
直接发起 installed unlock。production managed-fs guard 分别提供
`NODE_MANAGED_SQLITE_SHM_LOCK_TRANSITION_NOT_UNLOCKED`、
`NODE_MANAGED_SQLITE_SHM_SHARED_UNLOCK_NOT_HELD` 或
`NODE_MANAGED_SQLITE_SHM_EXCLUSIVE_UNLOCK_NOT_HELD` witness，并经 ABI 投影为
`SQLITE_IOERR_SHMLOCK`。selected lower-ledger source contract 要求 managed attempt=`1`、managed
success=`0`、native lock/unlock attempt=`0`、local transition=`0`、before/after masks 不漂移；setup 与
cleanup installed callbacks 只服务 own-overlap，且位于 selected ledger 之外。

source payload 还要求 target callback begin/completion 各一次，callback lease 正常释放，exact route 保持
active、未被 q7 seam 移除，registration/logical route 与 selected connection identity 不漂移；parent 只可从
child exit、canonical payload、environment 与 root cleanup 的闭合形状构造私有候选回执。调用方提交的
Expected、result code、lower counters 或 digest 均不能替代该路径。上述均是 source-written receipt shape，
不构成 actual receipt、Windows record、动态接受或生产开放事实。

q8 catalog 精确为 88 data rows（另 1 header）、15,716 bytes、LF-only、无 UTF-8 BOM，SHA-256=
`8cb3fcef3eb2f65fe54694396cdcff32aef576dc5f299879f5e072699428c936`；其 88 对 member seals 与 q3–q7
既有 2,920 对 member seals 的交集为 0。

## 6. Current evidence and production boundary

q7 的 matcher、catalog、source scope、child header、isolated runner、fixture、141-scalar payload、
ordinary route-preemption seam 与 source-level tests 已写入；full source scope 为 119 identities，q7 delta
为 13 identities。q8 source scope 为 132 个唯一 identity：继承 q7 全量 119，加上已在全局
scope 但 q7 scope 漏列的 `stored_poison/payload` 1 项，再加 q8 delta 12 项。q8 的 exact matcher、catalog、
installed-callback child/fixture、guard-witness lower ledger、active-route completion payload 与 source-level tests
已写入，但没有 current reviewed source scope digest。当前未运行 inventory 预期为
`3,122 present / 5,546 missing / 8,668 total`，coverage 仍为
`0/8,668`。

本批按架构铺设约束没有运行 Cargo、编译、SQLite、Windows 或真实 runtime；因此仍是
`passed=0 failed=0 actual=not_run`，没有 actual record、reviewed inventory digest、frozen manifest、
`Qlock`（仍为 `unknown`）或 Windows numerator，`WindowsDynamic=not_opened`。最终 Lock 功能继续 blocked：
仍缺 5,546 个 program，且 compile/runtime/actual receipts/reviewed digest 全部缺失。它不打开生产 VFS/open、
Runtime/Ready、Provider、Offer、Job、Attempt、Lease、dispatch、market、settlement 或 funds effects。
