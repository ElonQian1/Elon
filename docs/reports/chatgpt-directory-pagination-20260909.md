# Private directory pagination, September 9

Later checkpoint: [1610 continuation and bounded retry acceptance](chatgpt-directory-continuation-20260909.md).
It supersedes the timeout/continuation status below, but not the remaining
full-account pagination and resource-measurement gaps.

Capability: `android_chatgpt_private_directory_pagination_v1`.
Code: implemented. Deterministic tests: verified. Device acceptance: partial.
Base: `3f9b73ba60f975ac6d13f20eb0d767331fd0832d`. Adapter: 309.

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
  into the adapter cache while the request still reports its failure. Failed partial
  reads do not emit a native completion snapshot: this would clear refresh ownership
  before the failure receipt and incorrectly mark a project failure as global.
  The previous native list remains visible until a settled successful snapshot.
- Passive project parsing no longer descends into embedded `conversations`, which
  could otherwise confuse a conversation title with its containing project title.
- Global cache completeness remains false: owned project metadata and ordinary
  history are not authority to remove separately fetched project histories or
  shared projects. Bounded truncation is exposed, not described as all history.
- Explicit disabling of private reads retains the legacy path. A transient
  identity/network failure in an enabled private read does not launch a DOM scan.
- Passive snapshots remain deduplicated, but every successful requested refresh
  emits its snapshot even when rows and paging metadata are unchanged. The native
  refresh coordinator settles on that event; omitting it left the coordinator busy
  and later project refreshes queued indefinitely.

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
## Initial Device Acceptance

APK 1.1.1604, source `d385425f557fde6e1c2dc4fce3c82ddf968c4e89`, was
published and installed with `install -r`; its local and online SHA-256 both equal
`09ea8f680b439096a7edc4f37c9f3ef1dacb9ffdcbb78b232aade6f4cf416894`.
On Xiaomi 14 Pro, production `social_ai` / `chatgpt_web`, adapter 307, authenticated
true and composer-ready false:

- Two explicitly requested global reads returned `directory_timeout` after 5641 ms
  and 9102 ms. Validated partial rows were retained. A later background result had
  six completed page reads; a subsequent explicit read succeeded as
  `directory_partial` after 12241 ms. This is evidence of bounded paging, not a
  complete-account or low-network-latency pass.
- The cache exposed 19 project entries after private refresh, up from five restored
  entries. Project selection kept the production sidebar open and selected the
  requested ID. A `directory_ready` receipt was observed, but the native action
  lacks a request ID; it is not sufficient proof of that project's final contents.
- The sidebar was restored to date mode and closed. The conversation path and
  zero-length input remained unchanged. No send, microphone, account mutation,
  Cookie reset or proxy change was performed. The structural protocol probe was stopped.
- Inspection of the failed-partial path found the native scope-settlement ordering
  issue described above. Adapter 308 suppresses failed completion snapshots; a
  focused global/project regression verifies the failure receipt and later recovery.
- A later cached nonempty-project request did not expose a matching completion
  within the bounded 16-second observation. Code inspection independently found
  unchanged snapshots suppressing the native coordinator's completion signal.
  Adapter 309 fixes this without disabling passive-update deduplication. Final
  acceptance must use the native refresh action alone, not interleave raw MCP
  directory commands with the coordinator's own requests.

## Final Release And Native Acceptance

Final release: **1.1.1606**, adapter **309**, source
`3a02cfb0c0a471b4ed061ed4866b923d1a92aa63`. The local and online APK hashes match:
`4ad308aa387155033fb83f3495dd68687768f2329e88d02ed125f6d0dd58686a`
(40,036,705 bytes). Release build/publication passed in 345.6 seconds;
`install -r` succeeded on the same Xiaomi. The intermediate 1605 package was
published but was not used as the final phone acceptance package.

The final Node suite passed 51 cases. Production/test Kotlin compilation and
26 JVM tests passed in four suites, including directory receipt JSON, refresh
coordination, directory state and document-generation isolation. Native MCP now
exposes `navigation.last_directory_refresh` from the existing per-action receipt
cache. It contains only result, allowlisted code, time and native/MCP source;
unrelated skin commands cannot hide it, and replaced documents cannot reuse it.

Final device cases used the production native actions only, with no interleaved
raw `chatgpt_list_conversations` command:

| Case | Observed result |
|---|---|
| Open cached native sidebar | 367 ms including MCP overhead; 200 cached conversations and 19 projects, drawer open |
| Native global refresh | Accepted with composer-ready false; fresh native `directory_timeout` receipt after 12,594 ms including polling; previous native cache retained |
| Select a cached nonempty project | Requested project selected and drawer remained open |
| First native project refresh | Fresh native `directory_ready` receipt in 903 ms including polling |
| Repeat same native project refresh | Another fresh native `directory_ready` receipt in 1,004 ms including polling; no stuck coordinator |
| Restore | Date section selected, drawer closed; original conversation path and empty input unchanged |

Both project results left the global cache timestamp unchanged, confirming
scoped rather than replacement global snapshots. Authentication was true and
composer-ready false during both successful project reads. No message, microphone,
file, account write, Cookie reset or proxy action occurred. UI evidence is semantic
production state, not a screenshot-based visual review.

Reuse the verified cache display, composer-independent project reads and repeated
refresh completion. The broader Goal remains active. The global read timeout is
still an actual acceptance gap: this batch does not prove stable full-account
paging, cold network latency, or a specific underlying network fault.
