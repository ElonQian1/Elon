---
title: 节点插件 VFS Barrier 8/8 动态权威
status: current
reviewed_at: 2026-08-29
owners: node, security
design_status: design_frozen
implementation_status: implementation_compiled
verification_status: WindowsDynamic_8_of_8
---

# Node plugin VFS Barrier 8/8 dynamic authority

## 1. Authority status

This document is the current design authority for the A2b2 `xShmBarrier` dynamic family. It
freezes the exact selector set, the observable payload ABI, the child/parent evidence chain and
the minimum test-only seams required before any Windows dynamic result may be recorded.

- lifecycle: `current`
- authority: `design_frozen`
- source: `implementation_compiled`
- evidence: `WindowsDynamic=8/8`
- exact evidence commit: `1d57b8d98a1fed70fe40ad6f1575f4b856226857`
- Barrier validation fingerprint: `c300720aa17bae7850f0d0088827b918c5b5d66c2842ff6d5b548e80c7d185f4`
- current A2b2 Windows dynamic total: `16/117`; remaining: `101`
- clean wide-regression commit: `2c6393745e84cccc3ec8d1e25a3f0092eb412988`
- clean wide-regression fingerprint: `2d498cb483459785c8593d89839097675f1d224d560e211ef4ff5c2529b5b57b`
  (`sqlite_vfs_policy` `121/121`)

The design text alone is not implementation evidence. The status above is backed by eight unique
process-isolated records from the exact clean evidence commit: Windows 10.0.26200 x86_64, fixed
NTFS, bundled SQLite 3.45.0, `8 passed / 0 failed / 1695 filtered`; every record reports
`child_exit=0` and `parent_cleanup=deleted`. No selector passes merely because its static `Case`
exists, a low-level unit test passes, or a fixture reports an expected value. Partial future reruns
must remain `0/8`; the accepted family is always one exact 8/8 set.

## 2. Common exact identity

All eight selectors bind the following common `CaseKey` dimensions:

| Dimension | Frozen value |
|---|---|
| `path` | `Path::Barrier` |
| `topology` | `TopologyKind::SharedNonFinal` |
| `unmap_mode` | `UnmapMode::NotApplicable` |
| `node` | `NodePrecondition::Live` |
| `shared_mask` / `exclusive_mask` | `0 / 0` |
| `cause` | `None` |
| `scope` | `TargetScope::RouteMain` |
| `registration_id` | `1` in the normalized `CaseKey`; the raw payload carries the real nonzero child registration id |
| `route_ordinal` / `runtime_generation` / `shm_connection_id` | `1 / 1 / 1` |
| `role` / `callback` / `occurrence` | `Main / Some(CallbackKind::Shm) / 1` |
| SQLite outcome | `SqliteOutcome::VoidNoResultCode` |
| mutation / lock uncertainty | `false / false` |
| registration phase | `RegistrationPhase::Registered` |
| pre topology | `sqlite=2, shm=2, registry_routes=2, logical_names=6` |
| post topology | `sqlite=2, shm=2, registry_routes=2, logical_names=6` |

`xShmBarrier` is a void ABI callback. The runner must not invent an SQLite result code, translate
the case into an SQL statement result, or treat a caller-supplied label as the observed outcome.

## 3. Exact selector and terminal set

The wire selector spellings and their mapping are frozen as follows. `variant` is zero unless
shown otherwise.

| Wire selector | `CaseKey` phase | timing | class | variant | route / logical | domain terminal | later callback | retained callback leases |
|---|---|---|---|---:|---|---:|---:|---:|
| `admission-rejected` | `CallbackAdmission` | `BeforeCall` | `RegistryRejected` | 0 | `TerminalQuarantine / Retained` | 0 | 0 | 0 |
| `wrapper-before` | `BarrierFence` | `BeforeCall` | `IoBeforeMutation` | 1 | `TerminalQuarantine / Retained` | 0 | 0 | 0 |
| `fence-before` | `BarrierFence` | `BeforeCall` | `IoBeforeMutation` | 0 | `TerminalQuarantine / Retained` | 1 | 0 | 1 |
| `fence-after` | `BarrierFence` | `AfterSuccessUncertain` | `OutcomeUncertainPoisoned` | 0 | `TerminalQuarantine / Retained` | 1 | 0 | 1 |
| `completion-before` | `CallbackCompletion` | `BeforeCall` | `RegistryRejected` | 0 | `TerminalQuarantine / Retained` | 0 | 0 | 1 |
| `completion-native-uncertain` | `CallbackCompletion` | `NativeUncertain` | `RegistryRejected` | 0 | `TerminalQuarantine / Retained` | 0 | 0 | 1 |
| `completion-after-success-known` | `CallbackCompletion` | `AfterSuccessKnown` | `RegistryRejected` | 0 | `TerminalQuarantine / Retained` | 0 | 0 | 0 |
| `success` | `Success` | `Success` | `None` | 0 | `Active / Indexed` | 0 | 1 | 0 |

Every failure retains `LIVE_CUSTODY` with the callback-lease adjustment shown above:
`node=true, views=1, mappings=1, dms=Shared, shm_file=true, main_file=true,
main_lock_owner=true, main_lease=true, shm_lease=true, registry_entry=true,
logical_names=3, vfs_table=true, vfs_name=true, vfs_context=true,
root_deletable=false`. Success has the same live custody and zero retained callback leases; its
custody is still live observation, not terminal retention.

## 4. Exact count expectations

The count order is frozen in section 6. The table abbreviates only nontrivial Barrier columns;
all omitted count fields are exactly zero.

| Selector | abandon | methods clear | callback begin/attempt/success | action attempt/success | fault observe/trigger/pending | custody retain |
|---|---:|---:|---|---|---|---:|
| `admission-rejected` | 1 | 1 | `0/0/0` | `0/0` | `0/0/0` | 1 |
| `wrapper-before` | 1 | 1 | `0/0/0` | `0/0` | `1/1/0` | 1 |
| `fence-before` | 1 | 1 | `1/0/0` | `0/0` | `1/1/0` | 1 |
| `fence-after` | 1 | 1 | `1/0/0` | `1/1` | `1/1/0` | 1 |
| `completion-before` | 1 | 1 | `1/0/0` | `1/1` | `1/1/0` | 1 |
| `completion-native-uncertain` | 1 | 1 | `1/1/0` | `1/1` | `1/0/0` | 1 |
| `completion-after-success-known` | 1 | 1 | `1/1/1` | `1/1` | `1/1/0` | 1 |
| `success` | 0 | 0 | `1/1/1` | `1/1` | `0/0/0` | 0 |

For every selector, `raw_state_take_attempt=0`, `raw_state_take_success=0` and
`physical_retry=0`. A failed void callback abandons installed raw state exactly once and clears
`pMethods` exactly once. Success preserves the installed raw state for later normal SQLite use.

## 5. Required real call chain

The dynamic runner must exercise this production-shaped chain rather than invoke a fault
controller in isolation:

1. A real two-Connection WAL fixture obtains SQLite's live main `sqlite3_file` through
   `SQLITE_FCNTL_FILE_POINTER` and invokes its actual `xShmBarrier` function pointer.
2. `sqlite_vfs_abi/io_shm.rs::barrier` enters `file_state::run_void`.
3. The main route passes through the test VFS route and
   `managed_vfs/fault_script/file.rs::shm_barrier` wrapper gate.
4. `registry/file_custody/operations.rs::shm_barrier` performs exact route callback admission,
   invokes the underlying SHM barrier and then completes the callback receipt.
5. `node_agent_managed_fs/sqlite_namespace_shm/barrier.rs::PinnedManagedSqliteShmConnection::barrier`
   executes the before probe, the real sequentially-consistent barrier fences and the
   after-success probe.
6. A protected-call failure returns only through the void failure path;
   `file_state::abandon_without_unwind` calls `raw_state::abandon_installed_state`, clears the raw
   method slot once and permanently retains terminal custody.

Two independent SQLite connections must share the same registered managed VFS and WAL main/SHM
domain. A single connection, a mock callback, a direct managed-fs call, or a synthesized SQLite
error result cannot satisfy this authority.

## 6. `a2b2br1` canonical payload

The child actual payload is exactly:

```text
a2b2br1,<selector>,<81 unsigned-decimal fields>
```

It contains exactly 83 comma-separated tokens. Every numeric token is canonical unsigned
decimal: `0` or a nonzero digit followed only by digits; signs, whitespace, empty fields, leading
zeroes and trailing fields are rejected. Re-encoding the parsed value must reproduce the input
byte for byte. The frozen layout is `20/7/4/4/16/30`:

### 6.1 Identity and exact target: fields 1..20

1. `path_is_barrier`
2. `topology_is_shared_non_final`
3. `unmap_is_not_applicable`
4. `node_is_live`
5. `variant`
6. `pre_shared_mask`
7. `pre_exclusive_mask`
8. `phase`
9. `cause_phase_is_none`
10. `timing`
11. `class`
12. `scope_is_route_main`
13. `registration_id`
14. `route_ordinal`
15. `runtime_generation`
16. `shm_connection_id`
17. `role_is_main`
18. `callback_is_shm`
19. `occurrence`
20. `sqlite_outcome_is_void_no_result_code`

Barrier-specific enum encoding is explicit and must not rely on Rust declaration order:

- `phase`: `CallbackAdmission=0`, `BarrierFence=1`, `CallbackCompletion=2`, `Success=3`.
- `timing`: `BeforeCall=0`, `NativeUncertain=1`, `AfterSuccessKnown=2`,
  `AfterSuccessUncertain=3`, `Success=4`.
- `class`: `None=0`, `IoBeforeMutation=1`, `OutcomeUncertainPoisoned=2`,
  `RegistryRejected=3`.

### 6.2 Outcome and terminal state: fields 21..27

21. `mutation_may_have_occurred`
22. `lock_outcome_uncertain`
23. `domain_terminal`
24. `registry_route_phase` (`Active=0`, `TerminalQuarantine=1`)
25. `logical_route_phase` (`Indexed=0`, `Retained=1`)
26. `registration_phase` (`Registered=0`)
27. `later_callback_allowed`

### 6.3 Topology: fields 28..35

28. `pre_sqlite_connections`
29. `pre_shm_connections`
30. `pre_registry_routes`
31. `pre_logical_names`
32. `post_sqlite_connections`
33. `post_shm_connections`
34. `post_registry_routes`
35. `post_logical_names`

### 6.4 Retained custody: fields 36..51

36. `node`
37. `views`
38. `mappings`
39. `dms` (`Absent=0`, `Shared=1`, `Released=2`, `OutcomeUncertain=3`)
40. `shm_file`
41. `main_file`
42. `main_lock_owner`
43. `main_lease`
44. `shm_lease`
45. `callback_leases`
46. `registry_entry`
47. `logical_names`
48. `vfs_table`
49. `vfs_name`
50. `vfs_context`
51. `root_deletable`

### 6.5 Counts: fields 52..81

52. `raw_state_take_attempt`
53. `raw_state_take_success`
54. `raw_state_abandon`
55. `methods_clear`
56. `callback_begin`
57. `callback_complete_attempt`
58. `callback_complete_success`
59. `selected_action_attempt`
60. `selected_action_success`
61. `shm_detach`
62. `main_unlock_attempt`
63. `main_unlock_success`
64. `main_file_close_attempt`
65. `main_file_close_success`
66. `registry_close_attempt`
67. `registry_close_success`
68. `connection_observe_attempt`
69. `connection_observe_success`
70. `registry_route_remove_attempt`
71. `registry_route_remove_success`
72. `logical_names_remove_attempt`
73. `logical_names_remove_success`
74. `logical_names_remove`
75. `vfs_unregister_attempt`
76. `vfs_unregister_success`
77. `fault_observe`
78. `fault_trigger`
79. `fault_pending`
80. `custody_retain`
81. `physical_retry`

The 81 fields preserve the existing A2 dynamic evidence shape, but `a2b2br1` is a separate
version and parser. A RegistrationShutdown `a2b2rs1` payload, a different selector, or a payload
whose normalized `CaseKey` differs from the frozen Barrier case is rejected.

## 7. Linear child/parent evidence

Each selector runs in its own fresh Windows child process. The evidence is valid only when the
following chain is linear and complete:

1. The parent is compiled with `ELON_NODE_AGENT_GIT_SHA` equal to the exact clean source commit,
   creates one fresh canonical root and selects exactly one frozen selector.
2. The child verifies the expected selector and canonical root, creates one registration and two
   routed SQLite connections, enters WAL/runtime and observes the real exact route/runtime/SHM
   identities.
3. The child installs at most one exact one-shot fault step, calls the real void `xShmBarrier`
   once, and derives every actual field from event observers, raw slot state and custody witnesses.
4. Failure children publish a redacted terminal-custody witness before inaccessible custody is
   permanently retained. The witness binds the exact route, terminal reason, registry/logical
   phase, main/SHM leases, callback lease count and retained physical resources; it carries no raw
   pointer or secret path.
5. The child canonicalizes one `a2b2br1` payload, cross-validates it against exactly one static
   `CaseKey`, emits exactly one committed report and exits. Terminal fixtures are intentionally
   retained until process exit; they are not dropped to manufacture cleanup evidence.
6. The parent accepts only a successful child exit, exact selector/root/source/environment match,
   exact payload bytes and payload commitment. It removes the same root only after child exit and
   proves that root absent before sealing the dynamic record.

No child report alone is a Windows dynamic record. The record must bind case key, exact payload,
child receipt, root identity, environment, source commit and parent cleanup receipt.

## 8. Implemented test-only seams and evidence closure

The five required seams are implemented behind the existing test/Windows isolation and are part
of the exact clean-commit evidence described in section 10:

1. **Barrier physical selector admission** — `ManagedSqliteShmFailurePhase::Barrier` is admitted by
   `managed_vfs/shm_fault_script.rs::supported_shm_phase`, with only
   `OutcomeUncertainPoisoned` accepted for its after-success form.
2. **Before/after fixture installation** —
   `connection.rs::ManagedSqliteRoutedConnectionFixture::install_shm_fault_script` installs the
   exact route's before-call and after-success arrays without weakening existing users.
3. **Real void ABI invocation and raw observation** — the sealed Windows-test helper beside
   `connection.rs::call_main_shm_unmap_keep` validates and calls the live `xShmBarrier`, observes
   `pMethods` before and after, and creates no result-code channel.
4. **Deterministic native completion rejection** —
   `lifecycle_faults.rs::ManagedTestLifecycleFaultController` installs and consumes the exact
   `BarrierCallbackCompletion/NativeFailure` step through a synchronization gate. The registry
   genuinely rejects `complete_with_receipt`; no sleep, race or synthesized native failure is
   used as evidence.
5. **Terminal custody witness** — a test-only redacted witness is captured at the real
   `registry/process_owner/lifecycle.rs::retain_terminal_custody` boundary and exposed through the
   registry test bridge/lifecycle observer, so the evidence does not depend on an active-route
   snapshot after the exact route enters permanent terminal custody.

Together with the reused low-level Barrier probe, wrapper-before path, registry barrier callback
lifecycle, two-Connection fixture and A2 child/root/environment/cleanup envelope, these seams
close the Barrier family's previously listed implementation gaps without opening production VFS.

## 9. Exact-set, fail-closed and isolation rules

- `Selector::ALL` contains exactly the eight wire selectors in section 3. Duplicate, missing,
  unknown or aliased selectors fail closed.
- Static selection must produce exactly one `barrier::cases()` item and exact equality with the
  independently frozen expected `CaseKey`; no nearest-match or field patching is allowed.
- Every installed fault is exact registration + route + role + runtime generation + SHM
  connection + phase + occurrence. Pending or multiply consumed steps fail the case.
- Observers are append-only projections of actual events. Expected `Case` values must never be
  copied into the actual payload.
- `completion-native-uncertain` requires a real registry rejection after a completion attempt;
  `native_failure()` as an unconsumed annotation is insufficient.
- Failed void callbacks must show installed methods before the call and cleared methods after the
  call. Success must show the methods remain installed.
- Domain terminal is true only for `fence-before` and `fence-after`; route-only failures must not
  poison the shared sibling domain.
- Tests and helpers remain behind test/Windows isolation and must not alter the production ABI,
  production registration surface, public API, runtime defaults, network behavior or storage
  schema.
- Every newly introduced Rust source leaf for this family must remain below 500 lines. Split
  selector, actual model, codec, validator, child, parent, observer and outcome responsibilities
  before a leaf reaches that limit.

## 10. Completion gate

Barrier moved from `WindowsDynamic=0/8` to `WindowsDynamic=8/8` only after the exact clean evidence
commit produced all eight isolated, canonical, exact-set records and the relevant static inventory,
payload negative tests, low-level Barrier tests, managed VFS regression and A2b1 source-owner
guards all passed. The A2b2 summary therefore advances from `8/117` to `16/117`.

The authoritative family state is now:

```text
design_frozen / implementation_compiled / WindowsDynamic=8/8
```

This closes only the Barrier family. Unmap remains `0/49`, JointClose `0/36`, Registry lifecycle
`0/16`, and Map/Lock dynamic admission remains unopened; A2 therefore stays
`implementation_not_dynamically_accepted`.
