# Private directory browsing

## Goal and status

Full ordinary/project history must be reachable from the production sidebar
without increasing the size of the recent-history snapshot or dropping part of
a provider page. This extends, rather than replaces, the completed bounded
[refresh owner](reports/chatgpt-directory-owned-continuation-20260909.md).

Current status: the page reader, typed native command/event path, and production
sidebar entry are implemented. Grouped Release Kotlin compilation and 59 native
tests passed. Device acceptance remains required; this is not a completed capability.

## Reader contract

`chatgpt_web_private_directory_browser.js` reuses the inspected
`chatgpt_web_private_directory_pages.js` decoder and endpoint selection, the
existing same-origin identity transport, and bounded JSON response reader.
It does not introduce an endpoint, scrape DOM, or copy credentials to Android.

- One explicit `read(scope, handle)` fetches one complete page. No automatic
  whole-account crawl, request replay, guessed next offset, or DOM fallback.
- Scopes are ordinary conversations, owned projects, or one canonical project.
  Ordinary offsets and opaque provider cursors stay inside the page owner.
- Results expose only opaque random page handles and caller-normalized rows.
  The caller must use the existing canonical directory mapper. Missing rows or
  invalid mapping fail rather than advancing after silently discarding records.
  Explicit local deletion/archive tombstones may omit exactly their known IDs;
  the remaining rows must map in order. Accepted local mutations invalidate old
  waypoints, preventing cached pages from resurrecting deleted records.
- Unknown pagination is distinct from a confirmed terminal empty page. Pages
  above the response/row bound fail explicitly, never silently truncate.
- Tickets bind document, transport instance, and account identity. Expiry or
  eviction returns `directory_page_expired`, not a silent restart at page zero.
- Repeated taps are single-flight. A newer selection, including a cached one,
  cancels the older pending selection; late results do not become current.
- At most 32 waypoints and 512 cached rows are retained for ten minutes. Evicting
  rows preserves a waypoint for an explicit refetch while that ticket remains.
  Recent-history cache/persistence is untouched.
- Identity acquisition has a seven-second deadline; one admitted GET has an
  eight-second deadline and one-MiB response bound. No write requests are used.
- Errors are allowlisted; provider error text, credentials, and raw cursors are
  not returned. The 64 most recent opaque cursors guard against repeated cycles.

The provider's ordinary offset API is not a transactional snapshot. Changes to
history during browsing may shift later offsets. Native integration must dedupe
by canonical identity, offer explicit refresh, and not claim a frozen complete
account snapshot from a sequence of requests made at different times.

## Offline evidence, 2026-09-09

55 tests passed across the new browser and existing pagination/continuation
suites. New cases cover all 280 ordinary rows (including the entire 196-223
page), 65 projects, and 250 project conversations. Additional cases cover cached
revisit, duplicate taps, supersession, scope isolation, forged handles, identity
changes, cancellation, cursor cycles, unknown ends, explicit retry at the same
waypoint, row/ticket eviction, expiry, and credential/error non-disclosure.

Log: `private-directory-browser-tests-20260909-221301-675` in Git's command-log
directory. These are synthetic protocol and state tests, not a live account
pagination result. Existing phone was absent from bounded ADB discovery.

## Required integration

1. Implemented: canonical mapper integration, typed `directory_page` event and
   `browse_directory_page` command. Results never enter the bounded recent cache.
2. Implemented: account-read admission, current pending-request ownership, and
   document/history invalidation; native consumer also checks scope/waypoint.
3. Implemented: production sidebar `查看全部会话` / `查看全部项目` entry. The original
   date strip and recent view remain. Explicit full browsing starts at provider
   page one, not a guessed offset derived from recent-cache size. ListView reuses
   rows; only the current page and at most 24 previous handles are retained by
   native UI. Project selection opens its page in-place, back returns to the
   catalog, and selecting a conversation uses existing production navigation.
4. Pending: native UI pagination and restoration acceptance in a grouped APK/device
   round. Offline tests must not be reported as live-account acceptance.

The explicit all-conversations view filters unassigned conversations within each
provider page. A page containing only project chats may be empty while Next is
still available. This is not evidence that the account has no unassigned chats.
Previous-page history is bounded; refresh always explicitly returns to page one.
Search and conversation mutation menus remain in the existing recent/current
conversation surfaces, not a new competing browser implementation.

Additional offline bridge evidence: 53 tests passed, including execution of the
actual asset chain, native-envelope mapping of all 250 fixture rows, tombstone
handling, unchanged recent cache, cancellation, and explicit waypoint retry.
Log: `directory-page-bridge-tests-20260909-225153-891`.

Android evidence: `directory-browser-android-tests-20260909-225313-245`, 59 passing
tests across page protocol, observed-state isolation, operation admission, and
the existing protocol parser. The final UI edge-state adjustments are compiled
again by the grouped release path, not treated as physical-device acceptance.
