---
title: 节点插件 VFS RegistryLifecycle 16/16 动态权威
status: current
reviewed_at: 2026-08-30
owners: node, security
design_status: design_frozen
implementation_status: implementation_compiled
verification_status: WindowsDynamic_16_of_16
---

# Node plugin VFS RegistryLifecycle 16/16 dynamic authority

## 1. Authority status

This document is the current design authority for the A2b2 `RegistryLifecycle` dynamic family.
It freezes the exact 16-selector set, canonical child payload, lawful test-only stimuli, actual
event ledger and child/parent evidence chain before implementation is allowed to claim a result.

- lifecycle: `current`
- authority: `design_frozen`
- source: `implementation_compiled`
- evidence: `WindowsDynamic=16/16`
- current A2b2 Windows dynamic total: `117/117`; JointClose is separately closed at `36/36`
- accepted predecessor families: RegistrationShutdown `8/8`, Barrier `8/8`

No selector in this family is accepted merely because its static `Case` exists, a unit test
passes, or an injected error equals the expected SQLite result. The current `16/16` is accepted
only because one exact clean source commit produced all sixteen unique process-isolated records;
partial and earlier runs did not advance the numerator.

This family consumes the successful physical-close prefix already present in source, but it does
not require JointClose `36/36` or Unmap `49/49` to have been dynamically accepted first. It closes
only callback completion through logical route retirement. Production VFS activation remains out
of scope and unavailable.

## 2. Exact common identity and physical prefix

Every selector is bound to one real registered managed VFS, one exact main route and a real SQLite
`xClose`. Except for `success-shared-nonfinal`, the selected route is the only connection. The
shared success uses two distinct routes and SHM connections under the same registration/runtime.

The normalized `CaseKey` common fields are:

| Dimension | Frozen value |
|---|---|
| `path` | `Path::RegistryLifecycle` |
| `unmap_mode` | `UnmapMode::Keep` |
| `node` | `NodePrecondition::Live` before close |
| `shared_mask` / `exclusive_mask` | `0 / 0` |
| `cause` | `None` |
| `scope` | `TargetScope::RouteMain` |
| normalized registration/route/runtime/SHM identity | `1 / 1 / 1 / 1` |
| role / callback / occurrence | `Main / Close / 1` |
| mutation | selectors 1--14 `true`; selectors 15--16 `false` |
| lock uncertainty / domain terminal | `false / false` |
| registration phase | `Registered` |
| physical retry | `0` |

Before the selected registry phase, the real close has already proved exactly one successful raw
state take, method-table clear, callback begin, SHM detach, main unlock, main file close and
registry WAL-main close. Physical custody is released: node, view, mapping, DMS, SHM file, main
file and main-lock ownership are absent. The test harness may not replace this prefix with a mock
receipt or directly mutate registry state.

## 3. Exact selector set

The wire spellings and their frozen identity/result boundaries are:

| # | Wire selector | phase / timing / variant | Distinguishing actual result |
|---:|---|---|---|
| 1 | `callback-completion-before` | CallbackCompletion / BeforeCall / 0 | completion `0/0`; exact route quarantined |
| 2 | `callback-completion-native-uncertain` | CallbackCompletion / NativeUncertain / 0 | real completion attempt `1/0`; native observation `1` |
| 3 | `callback-completion-after-success-known` | CallbackCompletion / AfterSuccessKnown / 0 | completion `1/1`, then exact receipt retained |
| 4 | `connection-observation-before` | ConnectionObservation / BeforeCall / 0 | observation `0/0` |
| 5 | `connection-observation-outstanding-sidecar` | ConnectionObservation / Validation / 1 | real outstanding sidecar makes observation `1/0` |
| 6 | `connection-observation-after-success-known` | ConnectionObservation / AfterSuccessKnown / 0 | observation `1/1`; route passed AwaitingRetirement, then ended TerminalQuarantine with CompletionEvidence retained |
| 7 | `registry-route-removal-before` | RegistryRouteRemoval / BeforeCall / 0 | owner retirement `0/0` |
| 8 | `registry-route-removal-owner-native` | RegistryRouteRemoval / NativeUncertain / 1 | owner retirement `1/0`; route not removed |
| 9 | `registry-route-removal-publish-native` | RegistryRouteRemoval / NativeUncertain / 2 | owner retirement `1/1`; receipt publication fails; route removed |
| 10 | `registry-route-removal-after-success-known` | RegistryRouteRemoval / AfterSuccessKnown / 0 | owner retirement `1/1`, then receipt retained |
| 11 | `logical-route-removal-before` | LogicalRouteRemoval / BeforeCall / 0 | receipt claimed; logical mutation `0/0` |
| 12 | `logical-route-removal-claim-native` | LogicalRouteRemoval / NativeUncertain / 1 | receipt claim fails; logical mutation `0/0` |
| 13 | `logical-route-removal-index-native` | LogicalRouteRemoval / NativeUncertain / 2 | claim succeeds; exact index action `1/0` |
| 14 | `logical-route-removal-after-success-known` | LogicalRouteRemoval / AfterSuccessKnown / 0 | exact index action `1/1`; three names removed |
| 15 | `success-shared-nonfinal` | Success / Success / 0 | topology `TWO -> ONE`; sibling remains SQL-usable |
| 16 | `success-final` | Success / Success / 0 | topology `ONE -> EMPTY`; registration remains live for capture |

Selectors 1--14 use `TopologyKind::FinalConnection`. Selector 15 uses
`TopologyKind::SharedNonFinal`; selector 16 uses `TopologyKind::FinalConnection`.

Selectors 1--10 use `SqliteOutcome::IoerrClose`, which is the real VFS `xClose` callback result,
not the return value of the enclosing `sqlite3_close` API. Selectors 11--14 use
`SqliteOutcome::NotApplicable`: their real `xClose` completed successfully before the
fixture-level logical receipt/index boundary failed. Selectors 15--16 use `SqliteOutcome::Ok`.
For every accepted selector the `sqlite3_close` API itself succeeds. SQLite consumes the selected
final `Connection` even when `xClose` returns `SQLITE_IOERR_CLOSE`, so all final-topology selectors
observe post-close SQLite connection count `0`; shared success observes only its sibling (`1`).

## 4. Frozen count and custody rules

For every selector the common counts are:

```text
raw_state_take_attempt/success=1/1, raw_state_abandon=0, methods_clear=1
callback_begin=1, shm_detach=1
main_unlock_attempt/success=1/1
main_file_close_attempt/success=1/1
registry_close_attempt/success=1/1
vfs_unregister_attempt/success=0/0
physical_retry=0
```

The selected-phase counts are exactly those encoded by the static `close_registry.rs` cases.
Before/after injected steps report `fault_observe/fault_trigger=1/1`; lawful native observations
report `1/0`. Publication-native and receipt-claim-native are typed receipt boundary failures,
not ordinary injected operations, so their fault counts remain `0/0/0`.

All failures set `custody_retain=1`, forbid later callbacks and retain the exact linear receipt or
route custody. The accepted custody kind is frozen as follows; every unlisted custody kind is
zero:

| Selectors | Exact retained custody |
|---|---|
| 1--2 | terminal `CallbackLease=1` |
| 3--4, 6--8 | terminal `CompletionEvidence=1` |
| 5 | terminal `OtherTerminalCustody=1`, reason `ConnectionCloseUnproven`, real sidecar lease `1` |
| 9--11, 13 | controller retained typed registry-retirement receipt `(1,0,0)` |
| 12 | typed registry-retirement receipt remains in publication map `(0,1,0)` |
| 14 | controller retained typed logical-removal receipt `(0,0,1)` |
| 15--16 | terminal and controller custody all `0` |

Here each controller tuple is `(retained_registry, published_registry, retained_logical)`.
Selectors 1--14 prove `terminal.retention_count + controller.receipt_custody_count == 1`;
selectors 15--16 prove `0`. `WalMainPhysicalCustody` is always zero. Registry entry custody becomes
false only after owner retirement succeeded. Logical names remain three until the exact logical
removal succeeded, after which they are zero. VFS table/name/context custody remains present and
`root_deletable=false` for all sixteen observations.

Success removes one exact registry route and exactly its main/journal/WAL names. Shared success
leaves the sibling route, its three names and its live SQL/SHM custody untouched. Final success
leaves no route or logical name but still observes the registration as Registered; process exit,
not the family action, enables parent cleanup.

## 5. Required real call chain

Each child must exercise this chain once, without retry:

1. A real managed VFS registration creates one or two routed FULL_MUTEX SQLite connections.
2. The selected connection enters WAL runtime and establishes the exact live main/SHM target.
3. SQLite calls the installed main file's actual `xClose`; ABI raw state is taken and `pMethods`
   is cleared before the typed close operation runs.
4. The pinned WAL main physically performs Keep unmap, main unlock and main file close, then the
   registry consumes the typed physical receipt and releases main/SHM leases.
5. The Close callback lease is completed, the connection-closed receipt is observed, and the
   exact owner route is retired into a typed retirement receipt.
6. The test VFS publishes and claims that receipt, validates exact route custody, removes exactly
   the three logical names, and returns the final result.

The selected SQLite close is consumed once even on failure. A rusqlite `Connection::close` that
returns its connection is an out-of-family harness/API anomaly: it is retained to prevent a
second `xClose`, and the child fails before classification or payload emission. Typed receipt,
physical custody, sidecar sentinel or route entry that cannot safely continue is retained until
child exit. Drop, panic cleanup and parent validation must never invoke a second `xClose`.

The harness classifies a sealed physical-prefix failure as `XCloseRejected` only after the
rusqlite `Connection::close` API itself completed and the route-bound ledger proves retirement was
not published. Authorizer removal, a missing fixture connection, an API-level `Connection::close`
error, or an unavailable lifecycle observer is a harness failure and invalidates the child; none
may be projected into one of the sixteen selectors.

## 6. Lawful deterministic seams

Only the following test/Windows-isolated seams may make the frozen native branches reachable:

1. **Close callback native rejection** validates the exact Close lease, moves that real route to
   terminal state and then lets the real completion operation reject it.
2. **Outstanding sidecar** opens and stages a real Journal file before close. Only after callback
   completion and immediately before connection observation does it claim the selected route's
   real sidecar lease, bind that pinned file, and retain the pair through child exit.
3. **Owner-retirement native rejection** validates the typed connection-closed receipt and exact
   route before making the real owner retirement reject; no removal receipt may be fabricated.
4. **Retirement publication failure** runs only after owner retirement succeeded and consumes the
   real typed retirement receipt into terminal custody while publication returns failure.
5. **Retirement claim failure** fails before transferring the real published receipt; logical
   mutation is not attempted.
6. **Logical index native rejection** consumes the real receipt once, then fails before changing
   the exact-name index or custody-drop count.

Each seam is exact registration + route + phase + occurrence, one-shot and fail-closed. It may not
copy an expected `Case`, forge an OS/SQLite result, poison unrelated shared state or expose a
reusable raw handle, path or receipt.

## 7. Append-only actual ledger

A registration-scoped, route-keyed, append-only redacted ledger is the source for selected-phase
actual counts. It records only checked counters and stage outcomes for:

- raw close entry, callback begin and physical-close receipt;
- callback completion attempt/success;
- connection observation attempt/success;
- owner retirement attempt/success and receipt publication attempt/success;
- receipt claim attempt/success;
- logical index action attempt/success and exact removed-name count;
- pre/post registry route and logical-name totals.

The ledger must reject duplicate terminal stages, impossible ordering, overflow and cross-route
events. It carries no pointer, path, nonce, raw receipt or mutable custody. Existing SHM target,
route terminal-custody, registration and sibling observations independently bind physical and
retained state; the ledger cannot override them.

## 8. `a2b2rl1` canonical payload

The child payload is exactly:

```text
a2b2rl1,<selector>,<81 unsigned-decimal fields>
```

It contains exactly 83 comma-separated tokens. Every numeric token is canonical unsigned decimal;
signs, whitespace, empty values, leading zeroes and trailing fields are rejected. Re-encoding must
reproduce the input byte for byte. The field groups remain the common A2 layout `20/7/4/4/16/30`:

1. Identity fields 1..20: path, topology, unmap mode, node precondition, variant, lock masks,
   phase, absent cause, timing, failure class, exact target identity and SQLite outcome.
2. Outcome fields 21..27: mutation, lock uncertainty, domain terminal, registry route phase,
   logical route phase, registration phase and later-callback permission.
3. Pre/post topology fields 28..35: SQLite, SHM, registry-route and logical-name counts.
4. Retained custody fields 36..51: node, views, mappings, DMS, SHM/main files, main owner,
   main/SHM/callback leases, registry entry, logical names, VFS table/name/context and root release.
5. Count fields 52..81: the exact common A2 `Counts` order ending in custody retain and physical
   retry.

RegistryLifecycle-specific enum encoding is explicit and independent of Rust declaration order:

- topology: `FinalConnection=0`, `SharedNonFinal=1`;
- phase: `CallbackCompletion=0`, `ConnectionObservation=1`, `RegistryRouteRemoval=2`,
  `LogicalRouteRemoval=3`, `Success=4`;
- timing: `Validation=0`, `BeforeCall=1`, `NativeUncertain=2`, `AfterSuccessKnown=3`, `Success=4`;
- class: `None=0`, `RegistryRejected=1`;
- SQLite outcome: `Ok=0`, `IoerrClose=1`, `NotApplicable=2`;
- registry route phase: `Active=0`, `AwaitingRetirement=1`, `Removed=2`,
  `TerminalQuarantine=3`;
- logical route phase: `Indexed=0`, `Removed=1`, `Retained=2`;
- registration phase: `Registered=0`;
- DMS custody: `Absent=0`, `Shared=1`, `Released=2`, `OutcomeUncertain=3`.

An `a2b2rs1`, `a2b2br1`, different selector, noncanonical payload or normalized `CaseKey` outside
the exact sixteen is rejected.

## 9. Linear child/parent evidence

Every selector runs in a fresh child process under a fresh canonical parent-owned root. The child
validates root and nonce before VFS activity, runs the exact compile-time selector, captures one
canonical payload and emits one committed report. Terminal custody remains retained until exit.

The parent accepts only exact selector/root/source/environment/registration bindings, successful
child exit, exact payload bytes and matching payload commitment. It removes the same root only
after confirmed child exit and proves it absent before sealing one RegistryLifecycle dynamic
record. A child report without this cleanup receipt is not dynamic evidence.

The source commit embedded through `ELON_NODE_AGENT_GIT_SHA` must equal the clean tested commit.
All sixteen records must share that exact commit and environment. Duplicate, missing, aliased or
mixed-commit selectors invalidate the whole family.

## 10. Isolation and completion gate

- All new seams, observers, harnesses and runners remain behind test/Windows isolation.
- Production registration, ABI, storage schema, runtime defaults, network behavior and VFS open
  availability must not change.
- The two previously accepted payload versions and 16 prior dynamic records remain byte-stable.
- Every new Rust source leaf stays below 500 lines; existing large files receive only thin module
  declarations or delegation.
- Relevant contract/negative tests, close lifecycle tests, Barrier 8, RegistrationShutdown 8 and
  the managed VFS/registry regression set must remain green.

The family may advance only as one exact set:

```text
design_frozen / implementation_compiled / WindowsDynamic=16/16
```

At this family's acceptance point the A2b2 summary advanced from `16/117` to `32/117`, leaving
`85`. The later Unmap formal family advanced separately to `49/49`, taking the then-current A2b2
summary to `81/117`. JointClose was subsequently accepted at `36/36` on clean evidence commit
`bfa1a1180d220e9a4c8e39251414fc9a1b0a9ace`, so the current A2b2 summary is `117/117` and the
current wide regression is `266/266`. Map/Lock dynamic admission remains unopened; A2 therefore
still remains `implementation_not_dynamically_accepted`, and production VFS/open remains unavailable.

## 11. Formal evidence

- Tested clean commit: `95d910f0dbc167138f913861efafa20ff11295cc`; the remote task branch matched
  this SHA before execution. The implementation source was frozen in
  `a75769029ba4abf5e30002f64846c0f7099d9ae7`, and the commit-bound A2b1 owner graph/ledger was
  added in the tested commit.
- RegistryLifecycle exact-set validation fingerprint:
  `2fdc953b8485c373585905c66954c97b40d3d5324cae70747df86fc3f54d4168`.
- Command scope: `cargo test --manifest-path server/Cargo.toml --bin elon-pc-node
  a2c_registry_lifecycle_runner::registry_lifecycle_ --locked -- --nocapture --test-threads=1`,
  executed through `scripts/validate-rust.ps1 -Force` after binding
  `ELON_NODE_AGENT_GIT_SHA` to the tested SHA.
- Result: `16 passed / 0 failed / 1710 filtered`; validation was freshly executed, not reused.
  All 16 records have unique `a2b2rl1` selectors, embed the tested commit, report
  `child_exit=0`, and carry `parent_cleanup=deleted`.
- Environment: Windows build `10.0.26200`, `x86_64`, fixed NTFS, bundled SQLite `3.45.0`.
- Static and ownership guards on the same tested commit: RegistryLifecycle contract `5/5`
  (`41387759a6b4b10030fe1e9c1178b69a2a8d85ab15f8c50c78883ab514b5cc79`), raw-close witness
  `2/2` (`abde2cf133ef7ff1c669922cb481e8f101f92c164275a5daae1555747e24cd26`), and A2b1 review
  guards `4/4` (`e7ea6855df7e6f0677a985d214dfcf467585e79c938c2a1e54b7ce7b6cdd4ad5`).
- Predecessor revalidation: Barrier `8/8`
  (`193d258f7573209236b8231c5288c4ff165bc793c635cabbbc9a69d1b73ca610`) and
  RegistrationShutdown `8/8`
  (`467d069431e173387467062e5f50625cdb45c142af9e83de37312a9b8ad16a5e`).
- Regression evidence: registry owner/state/file-custody main set `45/45`
  (`fd0ac69aa07fec5898c5c106c55020810da29c2048418c2c7d33b34dca26c130`) and full
  `sqlite_vfs_policy` main set `142/142`
  (`78c3acc23ff5db33b78f105a8b6da4124708e6cdc5e18373f5540a6d7f66eab8`); all process-isolated
  child runs in those validations also passed. The wide run contains exactly 32 unique
  `(payload family, selector)` records for Barrier, RegistrationShutdown and RegistryLifecycle,
  all bound to the same tested commit with confirmed child exit and deleted parent roots.

These records advanced only RegistryLifecycle and the A2b2 aggregate at their historical evidence
commit. The later Unmap `49/49` evidence is independent and does not rewrite this family. Neither
family opens Map/Lock dynamic admission, implements JointClose, or makes production
VFS/open/Runtime/Ready reachable.
