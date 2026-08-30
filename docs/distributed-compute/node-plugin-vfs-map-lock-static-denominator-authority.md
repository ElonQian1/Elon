# Node Plugin VFS Map/Lock Static Denominator Authority V1

## 1. Purpose

This authority defines the only path from the current A2b1 source-review fragments to countable
Map and Lock static contracts. It covers `xShmMap` and `xShmLock` source-terminal enumeration,
typed case keys, expected outcomes, exclusions, and exact-set validation. It does not open a
production VFS or publish WindowsDynamic evidence.

The existing 18 Map and 10 Lock records remain `legacy_non_denominator`. They may be traced into
the new inventories or explicitly superseded, but they must never be added together, reported as
`0/28` or `28/28`, or used to infer the final Map/Lock counts.

## 2. Frozen source scope

The reviewed source scope starts at the SQLite ABI callbacks and follows every supported or
defensive terminal through raw-state admission/abandon, route and callback ownership, managed SHM
initialization, mapping or byte-range locking, cleanup rewrite, quarantine, payload custody, and
ABI projection. A terminal belongs to exactly one operation family and exactly one frozen case.

Map must close the six success semantics: Extend cold-create, Extend warm-create, Extend reuse,
Observe warm-create, Observe reuse, and Observe not-present. The existing five pending graph nodes
(`ManagedDmsInitialization`, `ManagedMapCoordinator`, `ManagedRegionLoop`,
`ManagedInlineRegionCustody`, and `WalMainColdNodeWitness`) and both open frontiers
(`TypedMapOperation` and `RawFallbackCustodyAndRouteProjection`) must become closed source
continuations or explicit exclusions.

Lock must cover LockShared, LockExclusive, UnlockShared, and UnlockExclusive, including request
and mask validation, local coalescing, same-connection transitions, sibling contention, native
acquire/release success-busy-error outcomes, before/native/after-success fault phases,
cold-acquire initialization, callback/route completion, raw abandon, custody retention, and the
rule that unlock never initializes a node.

## 3. Countable record contract

Every frozen Map or Lock record must contain:

- one canonical `CaseKey` whose fields are sufficient to distinguish the source branch;
- one non-empty `SourceBranch` with ordered owner/symbol/site witnesses;
- one unique `Expected` vector derived from the reviewed source branch;
- one terminal disposition: returned, quarantined, abandoned, cleanup-rewritten, or excluded;
- explicit pointer, raw-slot, route, callback, mapping/lock, file/handle, payload, and cleanup
  custody where those axes are reachable;
- a source-bound exclusion reason and reachability/type proof for every excluded defensive branch.

The inventories are countable only when all of the following exact-set equations hold:

```text
frozen Map keys  == source-exhaustive Map terminal leaves
frozen Lock keys == source-exhaustive Lock terminal leaves
project(records) == unique CaseKey -> unique Expected
missing == extra == duplicate == unknown == 0
pending == open_frontier == unresolved_exclusion == 0
```

The final denominators are the mechanically counted widths `N = |Map keys|` and
`M = |Lock keys|`. This document intentionally does not predict either number before source
closure.

## 4. Required static guards

The implementation must provide separate exact guards for Map and Lock plus an aggregate A2b1
guard. The guards must reject at least missing and extra keys, duplicate projections, an empty
source branch, multiple Expected vectors for one key, unresolved pending/frontier state, an
unproved exclusion, legacy-record widening, and source-owner or ordered-witness drift.

All source-owner blob OIDs, normalized hashes, and symbol sentinels must be rebound to a
non-self-referential clean source baseline in current main ancestry. Static validation must run on
an exact clean evidence commit and include the production `elon-pc-node` target and the wider
`sqlite_vfs_policy` regression.

## 5. Acceptance and non-goals

This feature is accepted only when Map reports `StaticContract=N/N`, Lock reports
`StaticContract=M/M`, both counts come from frozen exact inventories, all guards pass, and an
independent review finds no missing terminal or invalid exclusion.

Acceptance does not publish Map or Lock WindowsDynamic records, does not complete A2, and does
not register a VFS, call production `sqlite3_open_v2`, create a production Connection or opened
authority, acquire a process fence, start Runtime/Ready, dispatch work, or create market or funds
effects. Those remain separate dependent features.

## 6. Current delivery state

```text
authority=design_frozen
implementation=planned
map_static_contract=not_counted
lock_static_contract=not_counted
windows_dynamic=not_opened
production_vfs_open=unavailable
```
