# Private directory browsing

## Goal and status

Full ordinary/project history must be reachable from the production sidebar
without increasing the size of the recent-history snapshot or dropping part of
a provider page. This extends, rather than replaces, the completed bounded
[refresh owner](reports/chatgpt-directory-owned-continuation-20260909.md).

Current status: the page reader is implemented and offline verified. It is not
yet loaded or wired to a native production command/view. Native integration,
grouped Release build, and device acceptance remain required. This is not a
completed capability or a newly published APK.

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

1. Bind the reader to the existing canonical page-side directory mapper without
   routing paged results through its 200-row recent cache.
2. Add an owned native page protocol/consumer path independent of composer
   readiness, fencing replies by request, generation, account, and scope.
3. Integrate next/previous/loading/retry/expired state into the real sidebar,
   keeping its calendar and project navigation. Bound rendered views and keep
   older pages from displacing recently active conversations in persistent cache.
4. Verify actual native UI pagination and restoration in one grouped APK/device
   round. Do not replace this requirement with MCP-only or mock acceptance.
