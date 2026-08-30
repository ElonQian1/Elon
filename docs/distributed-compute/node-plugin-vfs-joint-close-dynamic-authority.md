---
version_status: current
reviewed_at: 2026-08-30
implementation_status: implementation_compiled
verification_status: WindowsDynamic_36_of_36
authority_scope: node-plugin-vfs-joint-close-windows-dynamic-v1
---

# Node plugin VFS JointClose 36-case dynamic authority

## 1. Authority status

This document is the single requirement and current evidence authority for the A2b2 `JointClose`
Windows dynamic family. The aggregate A2 contract remains in
[`node-plugin-vfs-fault-authority.md`](node-plugin-vfs-fault-authority.md), and aggregate acceptance
remains in [`node-plugin-vfs-fault-acceptance.md`](node-plugin-vfs-fault-acceptance.md). Those pages
consume this family's evidence; they do not define a second JointClose selector set.

Current accepted baseline at the start of this feature is:

- A2b2 `WindowsDynamic=81/117`;
- Barrier `8/8`, RegistrationShutdown `8/8`, RegistryLifecycle `16/16`, and Unmap `49/49`;
- JointClose `0/36`;
- Map/Lock has no accepted denominator and remains unopened;
- production VFS registration, SQLite open, `Connection`, process owner, Runtime, Ready, dispatch,
  Lease, settlement, and economic effects remain unavailable.

The 36 JointClose records are one atomic family. A partial runner, a typed schema, source review,
compilation, or any subset of passing cases does not advance the A2b2 numerator. Only an exact
clean-commit Windows family with every selector present once and no failed, duplicate, or unknown
member may advance JointClose from `0/36` to `36/36` and A2b2 from `81/117` to `117/117`.

Even `117/117` does not complete A2: the independent Map/Lock denominator must still be frozen and
dynamically accepted before production owner/VFS/open work may begin.

## 2. Frozen 36-selector family

Every selector maps to exactly one existing `Path::JointClose` static `CaseKey`. The static source
owners remain `a2b2_cases/close_physical.rs`, `a2b2_cases/close_registry.rs`, and the exact set in
`a2b2_cases/expected.rs`. Dynamic code may consume that set but must not replace or loosen it.

### 2.1 Raw, route, and callback prefix — 4

| Selector | Frozen phase/timing | Required observation |
|---|---|---|
| `raw-state-take-rejected` | `RawStateTake/Validation` | Real raw `xClose` enters once, exact state take fails, `pMethods` is not falsely reported cleared, and physical close never starts. |
| `begin-connection-close-rejected` | `BeginConnectionClose/BeforeCall` | Raw state is consumed once, route close admission rejects, and callback/SHM/main work remains zero. |
| `callback-admission-rejected` | `CallbackAdmission/BeforeCall` | `begin_connection_close` succeeds, the one Close callback admission rejects, and no physical action starts. |
| `callback-wrapper-before` | `MainFileClose/BeforeCall`, variant `1` | The exact file-operation wrapper rejects before entering the managed physical close chain. |

### 2.2 SHM lift — 20

The real WAL-main `xClose` must invoke `unmap_shm(Keep)`. Every failure below must prove
`main_unlock_attempt=0`, `main_file_close_attempt=0`, and `physical_retry=0`.

| Cause | Selectors |
|---|---|
| `ViewUnmap` | `shm-view-unmap-before`, `shm-view-unmap-native-uncertain`, `shm-view-unmap-after-known`, `shm-view-unmap-after-uncertain` |
| `MappingClose` | `shm-mapping-close-before`, `shm-mapping-close-native-uncertain`, `shm-mapping-close-after-known`, `shm-mapping-close-after-uncertain` |
| `DmsSharedRelease` | `shm-dms-release-before`, `shm-dms-release-native-uncertain`, `shm-dms-release-after-known`, `shm-dms-release-after-uncertain` |
| `ShmFileClose` | `shm-file-close-before`, `shm-file-close-native-retryable`, `shm-file-close-native-uncertain`, `shm-file-close-after-known`, `shm-file-close-after-uncertain` |
| `ConnectionDetach` | `shm-detach-before`, `shm-detach-after-known`, `shm-detach-after-uncertain` |

The SHM selectors may reuse the already accepted exact Windows Unmap platform adapters, but they
must execute through the real installed `xClose -> WAL-main close` chain while the owning
FULL_MUTEX Connection allocation remains retained. Direct `xShmUnmap`, a physical-subset runner,
or replayed Unmap evidence is not JointClose evidence.

### 2.3 Main close — 8

| Phase | Selectors |
|---|---|
| `MainLockRelease` | `main-lock-release-before`, `main-lock-release-native-uncertain-shared`, `main-lock-release-native-uncertain-reserved`, `main-lock-release-after-known` |
| `MainFileClose` | `main-file-close-before`, `main-file-close-native-retryable`, `main-file-close-native-uncertain`, `main-file-close-after-known` |

The native selectors require an exact Windows operation receipt. A synthetic observer, returned
enum, default counter, invalid reserved argument, or pre-native rejection cannot be relabelled as a
native failure. The two main-lock uncertainty selectors execute legal `UnlockFileEx` calls against
the exact live handle with, respectively, a held Shared range and a held Reserved-plus-Shared
topology; the return receipt is deliberately not read. The main-file retryable selector observes a
real `CloseHandle` rejection before custody is consumed and returns the still-live owner. In every
uncertain selector the exact call ran once, so the route plus main-file/lock custody becomes
terminal and cannot be retried. Because SHM teardown already succeeded, this does not falsely
poison the empty SHM/FileId domain.

The static keys distinguish the two main-lock uncertainty cases with variant `0` and `1`.
`pre_shared_mask` and `pre_exclusive_mask` remain SHM-slot masks and must not be overloaded with
main-database lock state. The canonical actual instead carries independent typed
`main_lock_prestate` and `main_lock_offset_class` fields: variant `0` requires
`Shared/SharedRange`, while variant `1` requires `ReservedShared/ReservedByte`. All other selectors
require both fields to be `NotApplicable`.

### 2.4 Physical-to-registry handoff — 4

| Selector | Frozen phase/timing | Required observation |
|---|---|---|
| `physical-success` | `Success/Success` | SHM, main lock, and main handle are released exactly once; the Close callback lease and registry leases remain for the next stage. This is not full route success. |
| `registry-wal-main-close-before` | `RegistryWalMainClose/BeforeCall` | Physical close receipt exists; registry close does not start and typed receipt/main/SHM lease custody is retained. |
| `registry-wal-main-close-native-uncertain` | `RegistryWalMainClose/NativeUncertain` | Registry close attempt is observed once, physical close is not repeated, and exact route custody is quarantined. |
| `registry-wal-main-close-after-known` | `RegistryWalMainClose/AfterSuccessKnown` | Registry close succeeds once before the injected post-success failure; physical close is not repeated and route retirement is not claimed. |

Callback completion, Connection observation, registry route removal, logical-name removal, and
shared/final route success are already the separate RegistryLifecycle `16/16` family. JointClose
must observe their counters as appropriate to the selected early terminal, but must not create a
second denominator or count those 16 cases again.

## 3. Required real execution chain

Every member must use a fresh parent-owned root, a unique non-default test VFS, a live route, and a
real SQLite `Connection` promoted to WAL-main. JointClose deliberately invokes the installed raw
`xClose` while that FULL_MUTEX Connection still owns the live allocation; it does not call
`Connection::close()`. The only accepted JointClose chain is:

```text
live Connection-owned main sqlite3_file
  -> installed sqlite3_io_methods.xClose (once, direct test invocation)
  -> take exact raw state and clear pMethods (once)
  -> begin exact route connection close
  -> admit one Close callback
  -> WAL-main unmap_shm(Keep)
  -> main lock-domain release
  -> exact main CloseHandle
  -> registry WAL-main receipt consumption
  -> retain the now-unusable Connection allocation without Drop
  -> callback completion / Connection observation / retirement (separate RegistryLifecycle-owned fixtures)
```

All 36 frozen JointClose cases therefore retain `sqlite_connections=1`: this is custody of the
owning allocation, not a claim that the Connection remains usable. After direct `xClose`, the child
must retain/forget the fixture and Connection without invoking SQLite or Drop. The separate
RegistryLifecycle 16 family uses fresh fixtures and real `Connection::close()` to prove Connection
observation and retirement. Neither family's `Drop`, panic recovery, child cleanup, nor fixture
teardown may invoke `xClose` or an OS close a second time.

The second-`xClose` law is exercised only while the Connection-owned allocation is still retained:
the test may retain the already captured real `xClose` function pointer, invoke it against the same
still-live allocation after `pMethods/state` were cleared, and require failure with zero additional
physical work. It must never dereference the cleared/null `pMethods`, call after SQLite frees the
allocation, or let the retained Connection Drop afterward.

## 4. Failure and custody laws

The following laws apply to every selector:

1. SHM failure stops the main close chain. No main unlock or main handle close may be attempted.
2. Once SHM detach or teardown succeeds, a later main failure never retries SHM work.
3. Main unlock failure retains the main failure, runtime generation, main/SHM leases, and exact
   route custody. Even when the native return receipt is unavailable, only that route and its main
   custody are terminalized; the already-empty SHM/FileId domain is not marked terminal.
4. Main handle failure retains live custody for retryable failure and terminal raw-handle custody
   for uncertain failure. Neither path retries the handle close.
5. Once physical close succeeds, callback or registry bookkeeping failure retains typed proof and
   leases in the registry/process domain; it never repeats physical close.
6. A route is not `Removed` until the existing RegistryLifecycle owner proves retirement. A
   logical route is not `Removed` until all three names are removed by that separate family.
7. Every retained failure is non-serializable and non-debuggable custody. Only redacted counts,
   enums, and opaque commitments may cross the child report boundary.
8. `physical_retry` is zero for all 36 selectors.

Ad-hoc `Box::leak` may keep a test process alive during an earlier fixture implementation, but it
is not sufficient JointClose evidence. The accepted implementation must move physical receipts,
main/SHM leases, callback receipts, and returned Connection owners into typed process-owner or
fixture-owned terminal custody whose redacted snapshot is observable.

## 5. Lawful deterministic seams

All seams are `cfg(all(test, windows))`, exact-target, one-shot, and installed before the selected
action. They must be bound to registration ID, route ordinal, runtime generation, SHM connection
ID, Main role, Close callback, phase, and occurrence.

- Existing SHM before/after/native adapters may be reused only through the real close chain.
- Main unlock and main handle native seams must execute the exact legal Windows boundary once. Main
  unlock has two held-lock topologies with a typed return-receipt-unavailable witness; main handle
  close additionally distinguishes observed retryable failure from return-receipt-unavailable.
- Raw-state and callback-admission seams must be allocation/route-bound and externally witnessed;
  no process-global flag, environment-selected identity, sleep ordering, or pointer selector is
  allowed.
- A success selector installs no fault seam. A natural precondition rejection must not be
  represented as a triggered fault.
- The parent chooses the libtest selector. The child environment may carry only the private root
  and parent-created nonce, never the case identity.

## 6. Canonical actual and selector equality

The dedicated canonical payload version is `a2b2jc1`. It must project the common A2b2 route record
in a fixed positional order, including:

- selector and exact target identity;
- path, final topology, Keep mode, phase, cause, timing, class, and SQLite outcome;
- mutation uncertainty, lock uncertainty, physical-domain terminal, registry/logical/registration
  phases, and later-callback status;
- pre/post SQLite, SHM, registry-route, and logical-name counts;
- node, view, mapping, DMS, SHM file, main file/lock owner, main/SHM/callback lease, registry entry,
  three logical names, VFS table/name/context, and root-deletable custody;
- every raw/callback/action/SHM/main/registry/connection/route/logical-name/unregister/fault/custody
  counter defined by the frozen A2b2 schema, including `physical_retry`;
- the independent typed main-lock prestate and selected native offset class described above.

The codec must reject aliases, whitespace drift, alternate numbers, unknown enum values, extra or
missing fields, non-canonical booleans, and non-canonical round trips. The validator must compare
the 36 dynamic keys with the existing 36 static keys by exact set equality and compare every
actual field against the selected frozen expected record. Comparing only selector names or count
36 is insufficient.

## 7. Child, parent, and atomic family evidence

Each selector runs in its own child/root. The child emits exactly one allow-listed bounded report
line containing canonical actual bytes plus opaque PID/nonce/root/registration commitments. The
parent must independently bind and consume:

- the real child process and successful exit;
- the exact tested commit compiled through `ELON_NODE_AGENT_GIT_SHA`;
- Windows build, architecture, fixed-volume kind, filesystem, and bundled SQLite version;
- canonical root and registration identity;
- exact canonical payload bytes and their commitment;
- cleanup of the same root after child exit.

Individual members remain non-formal implementation candidates. A linear 36-member reducer must
consume exactly one member for every frozen selector, reject duplicates/missing/cross-family or
mixed-environment members, bind a shared cohort, prove `HEAD` equals the compiled SHA, prove the
checkout is clean, and render all 36 records plus one family seal atomically. No partial collection
has a printable formal-record API.

## 8. Acceptance gate

JointClose may be recorded as `WindowsDynamic=36/36` only when one exact clean Windows commit has:

1. selector/static-key bijection and canonical `a2b2jc1` codec tests passing;
2. all 36 isolated real-close children passing with unique roots, registrations, and identities;
3. the atomic family reducer reporting 36 members, zero missing/extra/duplicate/failed cases, and
   one stable family fingerprint;
4. a source-contract guard proving all seams, payloads, runners, and observers are test-only; on
   the JointClose acceptance commit production `open()` remains unavailable, while any later
   separately accepted production activation must rerun every affected family on its new commit;
5. the complete relevant Rust target compiled with the exact Git SHA;
6. the existing Barrier `8/8`, RegistrationShutdown `8/8`, RegistryLifecycle `16/16`, Unmap
   `49/49`, managed SHM targeted tests, and wide `sqlite_vfs_policy` regression rerun on the same
   runtime-source commit;
7. a clean working tree at formal family capture time.

Any runtime-source change after the family run invalidates that formal run until the affected
family and wide regression are rerun on the new exact commit.

## 9. Non-goals and forbidden inference

This feature does not:

- open or register a production VFS;
- construct a production SQLite Connection or opened local authority;
- complete or open the Map/Lock denominator;
- complete A2 by itself;
- change Provider, Offer, Job, Lease, Receipt, capacity-future, pricing, settlement, Runtime,
  Ready, scheduling, dispatch, or funds;
- authorize a production process owner, download, Sidecar, v15 session, or workload execution;
- treat existing Unmap or RegistryLifecycle records as JointClose records.

## 10. Current formal evidence

The full gate passed on exact clean runtime-source commit
`83ed2a33c3e5e7dcfdecd253d94670eb0a78d71d`:

- the isolated family runner produced `36/36`, zero failed/missing/extra/duplicate selectors, and
  one `checkout=clean` family marker;
- validation fingerprint:
  `74d27ad6e4f39dc58441d3bfba93b3177ba1a375d4fdd3dfb9b250aa67a8e33d`;
- external validation receipt:
  `18461fa1ae3d03bd255c837adcb710436d754052387c0d1a67496a568ed3b97f`;
- cohort: `sha256:4ac04eb8bf92a0b6497e8c6e1787325ea035ca08cc6c50b4c624d677633ec956`;
  family seal: `sha256:5f06ef3d62e52aacee7f29d629ad3f1c1200df2476f9e7f344c41c86affc6b1a`;
- clean-commit fingerprint:
  `sha256:a1be5ca9c57c8ea4cac51e58152ef9e41f0dd8197ed8e11c3a94cafdb8d25e3c`;
- environment: Windows `10.0.26200`, `x86_64`, fixed NTFS, bundled SQLite `3.45.0`;
- the same runtime-source commit also passed Unmap `49/49`, managed SHM `11/11`, A2b1 owner
  guards `4/4`, production-target `cargo check`, and the wide `sqlite_vfs_policy` regression
  `266/266`.

JointClose therefore advances atomically to `WindowsDynamic=36/36`, and A2b2 advances from
`81/117` to `117/117`. This closes A2b2 only. Map/Lock still has no accepted denominator,
`StaticContract` or `WindowsDynamic` evidence, so A2 remains
`implementation_not_dynamically_accepted`; production VFS/open and every downstream runtime or
economic effect remain unavailable pending separately accepted activation work.
