---
title: 节点插件 VFS Lock 动态执行切片权威 V1
status: current
reviewed_at: 2026-09-01
owners: node, security
design_status: design_frozen
implementation_status: q5_q6_q7_source_written_uncompiled_unrun
verification_status: source_review_only_actual_not_run
authority_scope: backend-a2-map-lock-dynamic-quotient-authority-v1
---

# Node Plugin VFS Lock Dynamic Tranches Authority V1

## 1. Scope

本文维护 [`Map/Lock dynamic quotient authority`](node-plugin-vfs-map-lock-dynamic-quotient-authority.md)
中 Lock q5–q7 窄执行切片的精确成员、真实 lower 路径、回执和隔离约束。父权威仍唯一维护完整
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

## 5. Current evidence and production boundary

q7 的 matcher、catalog、source scope、child header、isolated runner、fixture、141-scalar payload、
ordinary route-preemption seam 与 source-level tests 已写入；full source scope 为 119 identities，q7 delta
为 13 identities，均绑定真实 lower 与 production completion 链。当前未运行 inventory 预期为
`3,034 present / 5,634 missing`，coverage 仍为 `0/8,668`。

本批按架构铺设约束没有运行 Cargo、编译、SQLite、Windows 或真实 runtime；因此仍是
`passed=0 failed=0 actual=not_run`，没有 actual record、reviewed inventory digest、frozen manifest、
`Qlock` 或 Windows numerator。它不打开生产 VFS/open、Runtime/Ready、Provider、Offer、Job、Attempt、
Lease、dispatch、market、settlement 或 funds effects。
