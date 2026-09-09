# Owned directory continuation

Follow-up to [the 1610 pagination acceptance](chatgpt-directory-continuation-20260909.md).

## Scope

- `android_chatgpt_directory_owned_continuation_v1`: completed for bounded
  ordinary/project directory refresh, enabled by default, Release/device verified.
  Reuse this module; do not reinterpret this marker as full-account pagination.
- Private directory refresh version 5; directory request dispatcher version 12;
  Android adapter 311. Existing private directory cache remains version 14.
- No voice, dictation, login, cookie, transport identity, or independent proxy changes.

## Changes

Successful time-budget partial reads now publish their validated rows with an
explicit continuation flag and the originating command ID. The native refresh
owner accepts completion only for the matching ID and project scope. Passive
cache snapshots cannot settle active refreshes; late native responses after
cancellation/replacement are ignored. The retained DOM mode also tags its
snapshots and fences cancelled callbacks.

The existing coordinator schedules the next batch after 250 ms, retaining the
last dispatched scope for continuation and failures. New user navigation/actions
take priority. Continuation requires increasing page progress, is limited to ten
continuations and a 60-second cycle, and does not activate for capped or
unknown-cursor results. Those cases remain partial, not a complete empty account.

The existing identity/document/transport-bound checkpoint now covers project
contents as well as global history. At most eight scopes are retained for a
fixed 60 seconds, with cancellation/context-change invalidation. Completed
cycles are discarded, not used as an indefinite freshness cache.

Native rows stay visible during continuation. Same-section index updates preserve
the existing sidebar scroll offset; changing date/project/query resets the view.

## Verification

- 69 focused Node cases passed, including request ownership, cancellation, DOM
  opt-in compatibility, bounded continuation and project-cursor retry.
- Targeted Release compilation and 42 Android tests passed in 263.7 seconds:
  refresh session/coordinator, directory cache, structural diagnostic and private
  evidence sanitizer. A metadata-only diagnostic-source correction followed;
  the final Release build also compiles that change.
- Source-size guard passed before staging; rechecked including new files before commit.
- Real-device results are not inferred from these offline tests.

## Published phone evidence

APK **1.1.1611 (1611)**, adapter 311, source
`b3f871b3cb81711ef4f9f5350efd0f7114d0e013`.
Local/published SHA-256:
`b36893c85d32e354756699f3fdcfbaa2518450327521067976453478d889f0d1`
(40,043,125 bytes).
Publication completed in 414.4 seconds; pinned Xiaomi `install -r` succeeded.
The installed version and current adapter were verified through MCP.

Acceptance used the production native sidebar and its normal refresh action.
Opening the sidebar can itself schedule a stale-cache refresh; this is not an
isolated HTTP benchmark. No raw directory command or official-page UI was used.

| Case | Observed result |
|---|---|
| Cached sidebar open | 336 ms including MCP; original calendar retained |
| First observed global completion | At 18,162 ms: `directory_partial`, native collection still `loading`, 206 cached rows, 19 projects |
| Automatic follow-on completion | At 28,910 ms: native collection `ready`, still partial at configured cap; no additional manual refresh |
| Follow-on private read | 10,654 ms; ordinary pages=8, resumedPages=5, requests=3; project catalog resumedPages=1, requests=0; identity acquisition=0 ms |
| Project refresh | `directory_ready`; private duration 568 ms, one project-content request; native receipt observed in 1,165 ms |
| Project folder | Correct selected project, drawer remained open |
| Restore | Same conversation/input; original date section, drawer closed; authenticated, composer-ready=false, no dictation or streaming |

This demonstrates directory operation without composer readiness, retention of
completed pages, and native loading-to-ready settlement across subsequent
batches. Project timeout/suspension/cancellation variants are offline-verified,
not all fault-injected on the phone. Scroll-offset preservation is implemented
and Release-compiled; this semantic acceptance is not a visual screenshot or
pixel-level scroll-position check. No messages, microphone capture, login reset,
application-data clear, or independent proxy operation was performed.

Evidence logs in the repository Git common directory:
`directory-owned-continuation-node-20260909-195149-806`,
`directory-owned-continuation-android-20260909-195309-726`,
`publish-directory-owned-continuation-20260909-200509-338`, and
`directory-owned-phone-acceptance-20260909-201409-285`.

## Remaining

Full-account older-history pagination remains incomplete. JavaScript ordinary
history is capped at 200 rows, projects at 40, and the native persisted history
store at 200 rows. A real load-more design must preserve overflow/page boundaries,
handle cache eviction, and render larger directories efficiently; simply
incrementing the next offset after dropping part of a page would skip records.

This batch does not establish heat/battery improvement, all-project completeness,
or completion of the broader private-API Goal. Google remains last in priority.
