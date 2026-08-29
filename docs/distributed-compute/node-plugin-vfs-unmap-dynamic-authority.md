---
title: 节点插件 VFS Unmap 49 项动态证据权威
status: current
reviewed_at: 2026-08-30
owners: node, security
design_status: design_frozen
implementation_status: implementation_compiled
verification_status: WindowsDynamic_49_of_49
---

# Node plugin VFS Unmap 49-case dynamic authority

## 1. Authority status

本文是 A2b2 `Unmap` 动态族的当前设计权威。它冻结并维护唯一 49-selector 集合、
`a2b2un1` canonical payload、真实 SQLite/Windows 观察链、父子进程证据与整族晋级门。

- lifecycle: `current`
- authority: `design_frozen`
- source: `implementation_compiled / formal_family_source=49/49`
- evidence: `WindowsDynamic=49/49`
- 当前 A2b2：`81/117`；剩余 `36`，全部为 JointClose
- 已接受前序：Barrier `8/8`、RegistrationShutdown `8/8`、RegistryLifecycle `16/16`

静态 `Case`、历史局部测试、编译成功或单条动态运行都不能提升本族。本族已在 exact clean
commit `da62f95b09287b79bc1f4c23780b95993cdd85a0`、同一受支持 Windows 环境上让 49 个唯一
selector 全部形成可验证记录，因此从 `0/49` 原子晋级为 `49/49`，A2b2 同步从 `32/117`
变为 `81/117`、剩余 `36`。不得删除、合并或重命名静态 case 来改变分母，也不得把分批实现
进度计入 numerator。

## 2. Frozen family partition

49 项严格分为 `SharedNonFinal=11`、`FinalKeep=23`、`FinalDelete=15`。每个 selector 只映射
现有 source-exhaustive `CaseKey` 的一个元素；wire 名不是新的 case owner。

### 2.1 SharedNonFinal — 11

| # | Wire selector | Frozen key distinction |
|---:|---|---|
| 1 | `shared-delete-request-validation` | Delete / RequestValidation / Validation / ProtocolViolation |
| 2 | `shared-keep-callback-admission` | Keep / CallbackAdmission / BeforeCall / RegistryRejected |
| 3 | `shared-keep-callback-wrapper-before` | Keep / ConnectionDetach / BeforeCall / IoBeforeMutation / variant 1 |
| 4 | `shared-keep-held-shared-lock` | Keep / HeldLockGate / Validation / shared mask 1 |
| 5 | `shared-keep-held-exclusive-lock` | Keep / HeldLockGate / Validation / exclusive mask 1 |
| 6 | `shared-keep-detach-before` | Keep / ConnectionDetach / BeforeCall / variant 0 |
| 7 | `shared-keep-detach-after-known` | Keep / ConnectionDetach / AfterSuccessKnown |
| 8 | `shared-keep-detach-after-uncertain` | Keep / ConnectionDetach / AfterSuccessUncertain |
| 9 | `shared-keep-completion-native-uncertain` | Keep / CallbackCompletion / NativeUncertain |
| 10 | `shared-keep-success` | Keep / Success |
| 11 | `shared-delete-success` | Delete request / Success; no physical delete because a sibling is live |

All eleven start with two exact live main routes and two SHM connections in one registered runtime.
Only the selected connection may detach. Success changes SHM connection topology `2 -> 1` while
both SQLite connections, both registry routes and all six logical names remain live; the sibling
must still map, lock and execute SQL. A non-final Delete request never authorizes teardown or exact
sibling deletion.

### 2.2 FinalKeep — 23

| # | Wire selector | Frozen key distinction |
|---:|---|---|
| 12 | `final-keep-view-unmap-before` | ViewUnmap / BeforeCall |
| 13 | `final-keep-view-unmap-native-uncertain` | ViewUnmap / NativeUncertain |
| 14 | `final-keep-view-unmap-after-known` | ViewUnmap / AfterSuccessKnown |
| 15 | `final-keep-view-unmap-after-uncertain` | ViewUnmap / AfterSuccessUncertain |
| 16 | `final-keep-mapping-close-before` | MappingClose / BeforeCall |
| 17 | `final-keep-mapping-close-native-uncertain` | MappingClose / NativeUncertain |
| 18 | `final-keep-mapping-close-after-known` | MappingClose / AfterSuccessKnown |
| 19 | `final-keep-mapping-close-after-uncertain` | MappingClose / AfterSuccessUncertain |
| 20 | `final-keep-dms-release-before` | DmsSharedRelease / BeforeCall |
| 21 | `final-keep-dms-release-native-uncertain` | DmsSharedRelease / NativeUncertain |
| 22 | `final-keep-dms-release-after-known` | DmsSharedRelease / AfterSuccessKnown |
| 23 | `final-keep-dms-release-after-uncertain` | DmsSharedRelease / AfterSuccessUncertain |
| 24 | `final-keep-file-close-before` | ShmFileClose / BeforeCall |
| 25 | `final-keep-file-close-native-retryable` | ShmFileClose / NativeRetryable |
| 26 | `final-keep-file-close-native-uncertain` | ShmFileClose / NativeUncertain |
| 27 | `final-keep-file-close-after-known` | ShmFileClose / AfterSuccessKnown |
| 28 | `final-keep-file-close-after-uncertain` | ShmFileClose / AfterSuccessUncertain |
| 29 | `final-keep-detach-before` | ConnectionDetach / BeforeCall |
| 30 | `final-keep-detach-after-known` | ConnectionDetach / AfterSuccessKnown |
| 31 | `final-keep-detach-after-uncertain` | ConnectionDetach / AfterSuccessUncertain |
| 32 | `final-keep-completion-native-uncertain` | CallbackCompletion / NativeUncertain |
| 33 | `final-keep-success-live-node` | Success / node precondition Live |
| 34 | `final-keep-success-node-absent` | Success / node precondition Absent |

The physical order is exact: all views -> mapping -> DMS shared lease -> SHM file -> selected
connection detach. A later phase may be reached only after every prior receipt was observed.
`NativeUncertain` and `AfterSuccessUncertain` are terminal, forbid retry and retain exact custody.
DMS native uncertainty additionally sets lock outcome uncertain. Within `FinalKeep`,
`NativeRetryable` exists only for SHM file close and is still classified
`OutcomeUncertainPoisoned` by the frozen static contract.

### 2.3 FinalDelete — 15

| # | Wire selector | Frozen key distinction |
|---:|---|---|
| 35 | `final-delete-auth-main-identity-missing` | DeleteAuthorization / Validation / ProtocolViolation / variant 1 |
| 36 | `final-delete-auth-main-or-generation-mismatch` | DeleteAuthorization / Validation / ProtocolViolation / variant 2 |
| 37 | `final-delete-auth-main-not-exclusive` | DeleteAuthorization / Validation / ProtocolViolation / variant 3 |
| 38 | `final-delete-auth-lock-state-uncertain` | DeleteAuthorization / Validation / OutcomeUncertainPoisoned / variant 4 |
| 39 | `final-delete-sibling-before` | ExactSiblingDelete / BeforeCall |
| 40 | `final-delete-sibling-native-retryable` | ExactSiblingDelete / NativeRetryable |
| 41 | `final-delete-sibling-native-uncertain` | ExactSiblingDelete / NativeUncertain |
| 42 | `final-delete-sibling-after-known` | ExactSiblingDelete / AfterSuccessKnown |
| 43 | `final-delete-sibling-after-uncertain` | ExactSiblingDelete / AfterSuccessUncertain |
| 44 | `final-delete-detach-before` | ConnectionDetach / BeforeCall / variant 1 |
| 45 | `final-delete-detach-after-known` | ConnectionDetach / AfterSuccessKnown / variant 1 |
| 46 | `final-delete-detach-after-uncertain` | ConnectionDetach / AfterSuccessUncertain / variant 1 |
| 47 | `final-delete-completion-native-uncertain` | CallbackCompletion / NativeUncertain / variant 1 |
| 48 | `final-delete-success-deleted` | Success / Deleted / variant 0 |
| 49 | `final-delete-success-not-found` | Success / NotFound / variant 1; observed token remains pending |

Delete authority must bind the selected route's real main identity, runtime generation and observed
Main-EXCLUSIVE lock. Variants 1--3 fail before teardown; variant 4 is terminal with uncertain lock
state. Exact sibling deletion runs only after final-node teardown. `NotFound` is a successful
idempotent unmap but is not a delete success: `fault_observe=1`, `fault_trigger=0`,
`fault_pending=1` must remain distinguishable from `Deleted`.

## 3. Common identity, outcome and custody

Every actual record is bound to one real registered managed VFS, exact registration, route ordinal,
runtime generation, SHM connection, main role, SHM callback and occurrence one. The normalized
static `CaseKey` uses registration/route/runtime/SHM identity `1/1/1/1`. Dynamic actual keeps its
real nonzero registration ID and current scenario's exact route/runtime/SHM identities; its
registration commitment must bind that real value. Common identity otherwise remains
`Path::Unmap`, `TargetScope::RouteMain`, role `Main`, callback `Shm`; only the selector-declared
topology, mode, node, variant, lock masks, phase, timing and class may differ.

The dynamic actual must equal the complete static `Expected`, not merely phase and SQLite result.
It therefore includes mutation and uncertainty flags, terminal status, later-callback permission,
registry/logical/registration phase, pre/post counts, all 16 custody fields and all 30 counters.
Failure returns the real `xShmUnmap` `SQLITE_IOERR`; success returns `SQLITE_OK`. Registration and
VFS table/name/context remain present. Active outcomes keep the main route callback-live; terminal
outcomes retain its registry/logical/physical custody in `TerminalQuarantine` while forbidding later
callbacks. This family does not perform `xClose` or route retirement. Parent root deletion is
allowed only after child exit, never as a claimed Unmap outcome field.

Failures before mutation retain retryable live custody only where the static case says so. Any
known prior mutation, post-success seam, uncertain native result or completion rejection must
preserve exact terminal/quarantine custody, forbid later callbacks and prevent Drop from retrying
an uncertain kernel action. A success consumes only SHM connection/node physical custody selected
by the case; main file, main lock owner, main lease, route, names and registration stay live.

## 4. Required real call chains and observers

Each child must execute the installed managed VFS callback, not call the coordinator directly:

1. Register a real test-only managed VFS under a fresh parent-owned canonical fixed-volume root on
   NTFS or ReFS.
2. Open one or two real FULL_MUTEX SQLite connections, enter WAL, establish the exact indexed
   main route and SHM connection, and create the selector's real node/view/mapping/lock prestate.
3. Invoke the installed main file's actual `xShmUnmap(keep|delete)` once. The registry callback
   lease, exact live target and runtime generation must be resolved by production-shaped routing.
4. Observe actual operation receipts and callback completion; never synthesize an `Expected` or
   classify from the injected error alone.
5. While the child is still alive, capture one immutable actual snapshot; retain unsafe custody
   until process exit.

Required independent observers are:

- SQLite connection liveness and sibling SQL usability;
- exact registry route/logical-name counts and unchanged registration identity;
- exact SHM connection membership, node/view/mapping/DMS/file custody and lock masks;
- main identity/generation/lock authority for Delete;
- callback lease begin/completion and later-callback admission state;
- ordered attempt/success counters for the selected action, detach and fault token;
- exact Deleted versus NotFound outcome and delete-token pending state;
- terminal/quarantine custody kind, mutation/lock uncertainty and absence of implicit retry.

An append-only registration+route+SHM keyed actual ledger may supply counters and ordering, but it
cannot override custody observers. It must reject duplicate stages, overflow, impossible order,
wrong occurrence and cross-route events, and must contain no raw pointer, path or transferable
receipt.

## 5. Lawful deterministic seams

All seams are `cfg(all(test, windows))`, exact registration + route + SHM connection + phase +
occurrence, one-shot and fail-closed. A selected seam that is not observed, is observed twice, or
fires on another route invalidates the child.

- Validation/admission/held-lock cases use real prestate or the existing registry callback gate;
  they may not forge a coordinator failure.
- `BeforeCall` and `AfterSuccess*` faults wrap the named real action. After-success injection is
  legal only after an independently checked success receipt.
- `NativeRetryable`/`NativeUncertain` need narrow Windows test adapters at the exact OS operation;
  a generic injected `io::Error`, mocked success receipt or timing relabel is forbidden.
- Delete authorization variants use real identity/generation/lock observations. Variant 4 may
  make only that exact lock-state query unavailable.
- CallbackCompletion rejection uses the real callback lease and route terminal transition.
- Deleted and NotFound use the real exact-sibling delete result; the seam may control filesystem
  prestate but may not rewrite the returned outcome.

The existing four physical-subset bridges for ViewUnmap, MappingClose, DMS release and SHM file
close may be reused only as low-level Windows witnesses. They do not observe full SQLite/route/
lease/callback/action/root state and never emit `WindowsDynamic`; the new implementation must
absorb them, not create a second case owner or count their historical runs.

## 6. `a2b2un1` canonical payload

The sole accepted child payload is:

```text
a2b2un1,<selector>,<81 unsigned-decimal fields>
```

It has exactly 83 comma-separated tokens and a maximum encoded size of 1024 ASCII bytes. Numeric
tokens are canonical unsigned decimal: no sign, whitespace, empty token, leading zero or trailing
field. Decode then re-encode must reproduce identical bytes. The 81 fields retain the common A2
layout `20/7/4/4/16/30`:

1. identity 1..20: path, topology, mode, node, variant, masks, phase, absent cause, timing, class,
   exact target identity, role, callback and occurrence;
2. outcome 21..27: mutation, lock uncertainty, domain terminal, registry route phase, logical
   route phase, registration phase and later-callback permission;
3. pre/post topology 28..35: SQLite, SHM, registry-route and logical-name counts;
4. retained custody 36..51: node, views, mappings, DMS, SHM/main files, main owner, main/SHM/
   callback leases, registry entry, logical names, VFS table/name/context and root release;
5. counters 52..81: common `Counts` order ending with fault pending, custody retain and retry.

Family-local enum codes are independent of Rust declaration order: topology
`SharedNonFinal=0, FinalConnection=1`; mode `Keep=0, Delete=1`; node `Live=0, Absent=1`;
phase `RequestValidation=0, CallbackAdmission=1, HeldLockGate=2, ConnectionDetach=3,
ViewUnmap=4, MappingClose=5, DmsSharedRelease=6, ShmFileClose=7,
DeleteAuthorization=8, ExactSiblingDelete=9, CallbackCompletion=10, Success=11`; timing
`Validation=0, BeforeCall=1, NativeRetryable=2, NativeUncertain=3, AfterSuccessKnown=4,
AfterSuccessUncertain=5, Success=6`; class `None=0, ProtocolViolation=1,
IoBeforeMutation=2, MutatedButKnown=3, OutcomeUncertainPoisoned=4, RegistryRejected=5`;
SQLite outcome `Ok=0, Ioerr=1`; route phase `Active=0, TerminalQuarantine=1`; logical phase
`Indexed=0, Retained=1`; registration `Registered=0`; DMS custody
`Absent=0, Shared=1, Released=2, OutcomeUncertain=3`. Fixed path/scope/role/callback and absent
cause each have code zero. Any `a2b2u1`, other A2 version, selector alias, unknown enum code or
normalized key outside the exact 49 is rejected.

## 7. Linear child/parent evidence

Every selector runs in a fresh child process and fresh canonical root. The child validates the
parent nonce/root binding before activity, runs one compile-time allow-listed selector, captures
exactly one payload, commits its digest, then exits. Runner errors, panic text, partial ledger output
and cleanup logs cannot match the payload channel. Commit and clean-checkout binding is a
parent-side responsibility, not a child claim.

The parent owns root creation and accepts only matching selector, nonce, child PID, environment,
registration and payload commitment. It requires successful child exit, validates all 81 fields
against the selector's static `Expected`, then deletes that exact root and proves it absent before
forming a non-Clone/non-Serde linear receipt. The historical SharedNonFinal runner additionally
verified that compiled `ELON_NODE_AGENT_GIT_SHA` equalled checkout HEAD and that the worktree was
clean before emitting `A2_UNMAP_IMPLEMENTATION_CANDIDATE_V1`; that 11-case cohort remains the first
historical candidate evidence and cannot be relabelled as a formal record. The current 49 individual
selector regression tests may each emit the same candidate marker, so a wide run can contain 49
candidate records; those records remain non-formal. The complete reducer revalidates all bindings
for all 49 selectors before emitting `A2_WINDOWS_DYNAMIC_V2` records and one family seal, while its
dedicated exact run rejects candidate-marker leakage. Cleanup before exit, a child-declared cleanup
receipt, wrong root, retained live process, SHA mismatch or dirty checkout invalidates the whole
cohort.

All 49 records have one exact clean `ELON_NODE_AGENT_GIT_SHA`, identical Windows build,
architecture, fixed-volume classification, accepted filesystem (`NTFS|ReFS`) and bundled SQLite
version. The formal family reducer consumes the clean-checkout receipt before emitting the complete
record set. Duplicate, missing, aliased, mixed-commit or mixed-environment selectors invalidate the
whole family. Partial cohorts remain implementation evidence only.

## 8. Implementation sequence and acceptance gate

Source is split into bounded leaves for selector catalog, actual ledger, codec, validator,
fixture/seams, child runner and parent record. Existing static `unmap_*` files remain the sole
`Case` owner. SharedNonFinal 11、FinalKeep 23、FinalDelete 15 and the 49-case formal reducer now all
exist and compile. Final paths use exact Windows adapters for retryable/uncertain native outcomes,
preserve terminal custody before fallible witnesses, and bind the post-raw SQL receipt only to the
same `Connection` and precompiled constant VM; that receipt must not be expanded into a claim that
the pager, database, VFS or retired SHM remains usable.

The implementation proves exact selector-set equality, canonical codec negative cases,
actual-versus-Expected comparison, wrong-route/phase/occurrence rejection, child-report isolation,
single-call/no-retry behavior and parent cleanup enforcement. After the architecture-stage execution
pause was explicitly lifted, all 49 ran process-isolated on one exact clean commit, followed by the
relevant managed-SHM、A2b1、production-check and wide regressions. Compile or wide-test success alone
remains insufficient dynamic evidence.

Formal evidence:

- tested clean commit: `da62f95b09287b79bc1f4c23780b95993cdd85a0`;
- command scope and target:

  ```powershell
  $env:ELON_NODE_AGENT_GIT_SHA = (git rev-parse HEAD).Trim()
  powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-rust.ps1 -Force -- test --locked --manifest-path server/Cargo.toml --bin elon-pc-node node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::abi::connection_fixture::managed_vfs::a2c_unmap_runner::unmap_windows_dynamic_family_49 -- --exact --nocapture --test-threads=1
  ```

- exact-family validation fingerprint: `2f9552efe2fd6be6c9fca67791e50cda77a0c9bc8c6ac98c1871d4d257c9e2e7`;
  external validation receipt profile: `54fba941caa967ef9aa1d39e1d8dd17942d344ee11efa17b50c586facffca0b7`;
- environment: Windows build `10.0.26200`, `x86_64`, fixed NTFS, bundled SQLite `3.45.0`;
- result: exactly 49 authority selectors, each unique, ordered and encoded as `a2b2un1` plus 81
  canonical unsigned fields (83 tokens total); all report `child_exit=0` and
  `parent_cleanup=deleted`; child/root/registration commitments are each 49-way unique;
- family seal: `cases=49`, commit equal to the tested HEAD, `checkout=clean`; independently
  recomputed cohort `sha256:55afa9bde0bee2945ff6cb0071ba3c661cfe44c07c90f4f9bdad7c7505329ed1`
  and family seal `sha256:faec6426d1b5363b1228f33aa05f4e452490bbb69d573fd2f8040dc634257481`
  match; outer libtest is `1 passed / 0 failed` with no candidate or child marker leakage;
- same-commit regressions: `sqlite_vfs_policy` `205/205`
  (`1ceaf594efa00061dca18ee61b511e4a8bbb863b6ff431101ceac3512b344698`), managed SHM `11/11`
  (`bf67e1168466c019712a42545553ec72265af824c09373b834f31f455458fb1a`), A2b1 `4/4`
  (`b8f61313a01561e159e2ce6752a6d3ee77407454c12c4eabc543d11383ec18f8`) and production
  `elon-pc-node` check (`1be807164c5a9c1e1c85e115f6457d948884b1537606ba706dce476750825d72`).

The only accepted promotion is:

```text
design_frozen / implementation_compiled / WindowsDynamic=49/49
A2b2: 32/117 -> 81/117; remaining: 36
```

## 9. Non-goals and forbidden inference

This authority does not implement or accept JointClose `36/36`, does not open the unfinished
Map/Lock denominator, and does not activate production VFS registration/open/Connection,
Planning A1, Runtime, Ready, dispatch, market or settlement. It changes no production ABI,
storage schema, runtime default, network behavior or registry ownership.

`49/49` proves only the Unmap family. It was not inferred from the SharedNonFinal 11-case candidate
or four historical physical bridges, and it cannot be converted into `117/117` by denominator
edits. The truthful current state is `implementation_compiled / WindowsDynamic=49/49`, A2b2 is
`81/117`, the remaining 36 cases are all JointClose, and A2 remains
`implementation_not_dynamically_accepted` until its independent completion gates are met.
