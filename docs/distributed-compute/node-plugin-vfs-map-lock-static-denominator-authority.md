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

The source cut is the production Windows `xShmMap` or `xShmLock` callback with every memory-safe
ABI/raw-slot state admitted by those callbacks. Test VFS registration, route preparation, fault
selectors, fault-controller failure, and test-only lifecycle controls are harness overlays: they
may stimulate or observe a production semantic leaf, but they are not production source leaves and
do not widen `N` or `M`.

The finite quotient is defined by the ordered source decisions actually read before the outermost
callback return. Inputs not read before an early terminal use one `not_reached`/`any_valid` cell and
must not be multiplied by Map mode, Lock action, slot, or prestate. Distinct source sites never
merge merely because they return the same result. Native error classes split when production code
projects them differently. Map loop ordinals and regions-to-create are bounded by the authority
budget (`1..=256`) and are mechanically expanded unless an executable equivalence proof removes
the exact ordinal from every Expected observation.

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

The equations are evaluated over one rooted acyclic decision graph per callback. A parent
continuation is replaced by its child leaves; parent/local fragment widths are never added to child
widths. The source universe is the disjoint union of included terminal leaf cells and proved
excluded leaf cells. Only complete root-to-terminal cases contribute to the denominator.

The closed denominators are the mechanically counted widths `N = |Map keys| = 43,476` and
`M = |Lock keys| = 8,668`. The complete reviewed universes also retain every proved exclusion:
Map has 324,561 source leaves (`43,476` included and `281,085` excluded), while Lock has 62,442
source leaves (`8,668` included and `53,774` excluded). Exclusions are part of the frozen authority
but never contribute to `N` or `M`.

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
authority=source_and_ledger_frozen
source_baseline=47cb2652321b42cc9689319075d253fe2275ace1
implementation=exact_guards_implemented
map_static_contract=43476/43476
map_source_universe=324561
map_leaf_ledger_sha256=0a756fe7f48ba5fb4634f8f2716d482e1382152f29ad7e300dd411c96e205333
map_manifest_sha256=0c51c3abe52f1a4f5ad1217c79ebd7393188452ff09659739ca6e1d93d205c19
lock_static_contract=8668/8668
lock_source_universe=62442
lock_leaf_ledger_sha256=23610b46e8217d396aea7a5367c2eed93f54c2488178d2ee8aa80c121425f082
lock_manifest_sha256=c690c2f5b78b68201bd5c0eacd4e6489e87bb4c6abf8ab584aa24e443795491e
windows_dynamic=not_opened
production_vfs_open=unavailable
```

The Map leaf ledger is checked in as 16 fixed, line-boundary parts because its canonical byte
stream exceeds the hosting provider's single-blob limit. Concatenating the parts in numeric order
reconstructs the exact canonical TSV bound by `map_leaf_ledger_sha256`; splitting does not change
the ledger, leaf identities, or manifest.
