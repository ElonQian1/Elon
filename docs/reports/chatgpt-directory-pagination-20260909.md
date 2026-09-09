# Private directory pagination, September 9

Capability: `android_chatgpt_private_directory_pagination_v1`.
Code: implemented. Deterministic tests: verified. New device acceptance: pending.
Base: `3f9b73ba60f975ac6d13f20eb0d767331fd0832d`. Adapter: 307.

## Evidence And Scope

The current inspected official shared module is
[4813494d-bgyv5408fxme7xxv.js](https://chatgpt.com/cdn/assets/4813494d-bgyv5408fxme7xxv.js).
Its public, credential-free download has SHA-256
`72ed87dd6d8a5241abac73d9c720f8e92bf87bbee7980331fcf042fea64d14ca`,
matching the previously device-observed `web_20260909_b` profile.
No credentials, request headers or personal conversations are test fixtures.

- `apt` reads `/conversations` with offset/limit (28), ordered by update;
  its next offset is response `offset + limit`, compared with response `total`.
- `getProjectSidebar` reads `/gizmos/snorlax/sidebar` with a cursor,
  `conversations_per_gizmo=0`, `owned_only=true` and a limit parameter.
  The adjacent history reader uses a limit of 20. Each project resource is
  `item.gizmo.gizmo`; `zSt` filters entries by the `conversations` property.
- `iJ` starts project conversation reads at cursor `"0"`. It uses the same
  `/gizmos/{id}/conversations` endpoint as the old implementation, follows
  response `cursor`, and validates the `owner` property on its item rows.
- Additional project unlock headers in `Iw` remain a separate locked-project
  runtime capability. This batch does not invent or bypass them; locked project
  failures retain the cache and remain failures.

The previous implementation fetched one project page without the identity
owner's headers, replaced its entire cached project list, then always emitted
`complete=true`. Global refresh fetched only the first ordinary page and never
actively refreshed project metadata. These were implementation gaps, not proof
of missing provider capabilities or an input-box requirement.

## Implementation

- A small versioned pagination module describes only the inspected protocol.
  The existing identity owner, JSON reader, cache and native dispatcher are reused.
- Global refresh reads ordinary history and owned project metadata independently,
  with at most two concurrent HTTP reads. Opening a cached menu is unchanged.
- Project history follows its opaque cursor; a terminal response can replace only
  that project's cached rows. Partial, failed or malformed pagination never clears
  missing rows. Empty terminal projects now emit a native empty snapshot.
- Same-scope requests coalesce. Every asynchronous boundary checks account,
  document, identity-owner object and cancellation. There are no write requests,
  message replays or DOM waits in these reads.
- Reads are bounded to 200 conversations or 40 projects, ten pages and twelve
  seconds per collection, plus existing seven-second shared identity acquisition.
  Each HTTP read has a four-second maximum and a one-MiB response limit.
- Repeated cursors and duplicate-only pages stop. Missing pagination metadata
  means partial, never a complete empty list. Useful validated partial pages merge
  into the cache while the request still reports its failure.
- Passive project parsing no longer descends into embedded `conversations`, which
  could otherwise confuse a conversation title with its containing project title.
- Global cache completeness remains false: owned project metadata and ordinary
  history are not authority to remove separately fetched project histories or
  shared projects. Bounded truncation is exposed, not described as all history.
- Explicit disabling of private reads retains the legacy path. A transient
  identity/network failure in an enabled private read does not launch a DOM scan.

## Verification

The focused Node command `directory-paging-node-complete-20260909` passed 47
runner cases plus assertions in the existing directory/mutation script suites.
Cases include offset/cursor paging, owner replacement, late cancellation,
partial-page retention, empty terminal replacement, limits, malformed payloads,
project title provenance and actual bundled asset parsing.
Release production/test Kotlin compilation and 48 JVM cases in five directory,
collection and operation-readiness suites passed (255.1 seconds), command
`directory-paging-android-20260909`. The final Node rerun after title validation
also passed, `directory-paging-final-node-20260909`.
APK publication and device results will be recorded after the grouped build.
There is no claim here of measured phone latency or completion of the broader Goal.
