# Native ChatGPT File Library

## Scope

- Capability: `android_chatgpt_private_library_browser_v1`.
- Code: implemented for root/folder browsing, explicit search, cursor pagination,
  refresh and ordinary library-file download; published/installed in 1.1.1581.
- Verification: native catalogue and pagination data passed through 1582;
  ordinary saved-byte download passed in normal release 1.1.1584 with module 8.
  Native entry, search and ordinary-file action menu passed on normal 1584.
  Folder/back rendered interaction and additional file scopes remain pending.
- Catalog v5 fixes object-event publication and includes v4 cold-identity preparation;
  both patches are installed in 1581, replacing the defective 1579 catalog v3.
- This is the independent library, not the current conversation's attachment index.
  No conversation ID is fabricated to authorize a library download.

## Evidence

The retained public official assets from 2026-09-07 define `JSn`/`kF`:

- Conversation asset `conversation-small-owrec55n6vm0ekcc.js`, SHA-256
  `7973d518b083f0f3e23905a279ed019378481bdbdd10fc0196afe9fc7b3b7d35`.
- `GET /backend-api/files/library/nodes`; optional `parent_directory_id`, `cursor`
  and `q`; `hydrate_folder_thumbnails`, `include_folder_counts` and
  `include_saved_entities` are requested by the official browser.
- Node kinds are `directory` and `file`. File descriptors expose `id`, `name`,
  `mime_type`, `file_size_bytes` and optional source/artifact metadata.
- Official pagination deduplicates node IDs and stops repeated cursors.
- Ordinary `libfile_`/`libfile-` downloads reuse the existing
  [library binary owner](chatgpt-private-shared-library-download.md), without
  calling a fabricated conversation route or exporting authorization URLs.

Public source and synthetic fixtures are protocol evidence, not live API acceptance.

## Ownership And UI

- `chatgpt_web_private_library_catalog.js` owns reads, opaque folder handles,
  pagination tokens and eight bounded memory-cache entries. A fresh entry lasts
  60 seconds; refresh displays cached rows first and preserves them on failure.
- One current read is allowed. Each HTTP read has a 10-second response deadline
  and a 2 MiB body bound. Three consecutive failures cool down reads for 30 seconds.
  Each listing is capped at 500 items/eight pages; capped or malformed results are
  visibly partial, never a claimed complete empty library.
- Identity/document changes clear page caches. Late results after cancellation,
  account change, navigation or replacement cannot update the native catalogue.
- Catalog v4 joins the existing identity owner's single-flight acquisition when
  cached headers are absent. Warm reads do not reacquire. Its local wait is at most
  seven seconds, without cancelling other identity consumers. Identity preparation
  and catalog HTTP share a 14-second deadline below the native 16-second watcher.
  Missing identity is not an empty library or evidence of an unsupported feature.
- The native `WebChatLibraryBrowser` uses a recycled list and a bounded foreground
  receipt watcher. It has no permanent refresh loop. It shows a preset entry in
  the production feature menu even before the official sidebar is observed.
- Tapping a folder keeps the same panel open. Back traverses its navigation trail;
  it does not switch the underlying conversation. Search-result trails represent
  visited folders, not an invented canonical server ancestor path.
- Downloads reuse `ChatGptWebFileDownloadGateway` and `WebChatFileDownloadDialog`,
  including request-bound cancellation and saved-byte receipts. No second downloader
  or system-TTS/voice substitute is introduced. Drafts, RTC and the current URL are untouched.

Stable controls: `web-chat-feature:library`, `web-chat-library-browser`,
`web-chat-library-back`, `web-chat-library-refresh`, `web-chat-library-query`,
`web-chat-library-search`, `web-chat-library-more`, `web-chat-library-official`.
MCP commands: `chatgpt_list_library_files`, `chatgpt_cancel_library_files`,
`chatgpt_download_library_file`; `library_files` contains native rows/opaque handles,
not server IDs or pagination tokens. Final command receipts, not dispatch acceptance,
determine completion.

The preset library entry uses `web-chat-feature:library`. After official feature
sync it can instead use `chatgpt-feature:{opaque-id}:{label}`; the observed label
on 1584 was `资料库`. Acceptance must resolve the current entry, not confuse a
missing preset-only selector with a failed feature. The external
`scripts/invoke-library-ui-acceptance.ps1` resolves both, then invokes native
accessibility actions. It does not inject coordinates, install a test APK,
enable WebView debugging, or export the accessibility tree. Its SDK/runner
dependency remains test-only; an unsupported runner is a test failure.

### 1584 Native Menu Acceptance

On 2026-09-09, normal 1584 opened the production sidebar feature menu and the
native library. Search, back and refresh controls were present. An explicit
synthetic-file search and its ordinary-file menu exposed attach, download,
rename and soft-delete actions. No file mutation was invoked. Conversation and
draft were unchanged. Logs: `library-native-open-1584-20260909-032107-735` and
`library-native-menu-1584-20260909-032140-744`.
Earlier attempts used only the preset selector and failed, despite the synced
entry being visible. Their failures are not app-library or authentication failures.

On normal 1585 the library/search still opened, but its file-row accessibility
node was visible/enabled without `ACTION_CLICK`. The listener lived on the
outer `ListView`, not on the semantic row. The row now owns the same existing
folder/file action with current-provider/handle checks; the outer listener is
removed to avoid duplicate dispatch. The external `inspect_entry` step reports
only node flags/bounds. Log: `library-native-menu-1585-20260909-035524-240`.
The 1585 failure did not attempt an attachment association or file mutation.

## Remaining Acceptance And Gaps

Validated on 2026-09-08:

- 162 related JavaScript tests passed, including the catalogue and native-entry
  wiring contracts; log `library-js-final-20260908-220836-653`.
- 101 production adapter assets and their concatenated bundle parsed successfully.
- Grouped Release source compilation, 12 focused JVM tests and APK publication
  passed. APK 1.1.1579 was installed via `adb install -r`, preserving app data.
  Source: `20c08c8b9`; release log `library-batch-release-20260908-232230-150`.
  APK SHA-256: `8119319905b2af85a1c8024cf85638331b34fb42cfb51fc0e2f5c7437b1861a9`.
- Initial production inspection reached account selection. The user subsequently
  confirmed sign-in on 2026-09-09; do not repeat the old sign-in prerequisite.
- The follow-up catalog v4 patch passed 55 focused Node cases, including real shared
  auth-owner integration, cancellation, supersession, context changes and combined
  time budgets; log `library-cold-identity-final-20260908-235543-816`. This fixes an
  independently verified cold-cache gap; it does not claim to sign in a logged-out account.

2026-09-09 regression: foreground 1579 returned `library_ready`, but native
`library_files` was null. This is missing projection, not evidence of an empty
account. The catalog called `emit(type, value)` while the real adapter expects
`emitEvent({type, ...value})`. Its two unit-test fixtures copied the same wrong
callback and missed the failure. Catalog v5 uses the existing single-object
contract without changing the protocol parser or unrelated adapters. Three tests
running the actual catalog, directory dispatcher and full adapter failed before
the fix and passed afterward; 46 related Node cases pass. The shared synthetic
`webchat/private-library-bridge.json` also exercises the full Android envelope
parser and pending native-state owner. Release compilation and all 12 library
JVM tests passed (`library-bridge-jvm-20260909-002514-316`); the related Node
batch including attachment reuse passed (`library-bridge-node-20260909-003154-663`).
That final Node batch contains 54 passing cases.

### 1581 Device Evidence

- Release 1.1.1581 (code 1581), adapter 306, source `9bb66d9f3`, published and
  installed without clearing app data. Release log:
  `library-bridge-release-verified-jdk-20260909-003837-971`.
  APK SHA-256: `012dc0e76ee49778bba2cb3eeacab458cb29cd14b128694b43bbdface6772332`.
- Semantic production-handler reads now publish a non-null `library_files` snapshot
  bound to the exact request ID. The root first page contained 21 items, including
  one folder; `has_more=true`, so 21 is not a total-account count. Its folder read
  contained four items. A deliberately nonmatching synthetic search produced an
  actual empty items array, distinguishable from the old missing-event defect.
- Root reads and return-to-root reads also produced `library_cached`; observed MCP
  request durations were 109-1259 ms for cached reads and 1186-2362 ms for the
  sampled live reads. These are handler observations, not rendering benchmarks or
  proof of a thermal improvement. Log: `library-bridge-device-20260909-004854-701`.
- Nine root rows advertised download and attachment handles; these are capability
  descriptors, not completed binary downloads or composer associations. No file
  mutation, attachment association, message send or microphone operation was used.
- The subsequent rendered-menu step did not reach `web-chat-library-browser`.
  Accessibility and MCP surface observations diverged, including a different native
  screen; neither a login failure nor the cause of that divergence is established.
  Render-only log `library-bridge-native-render-20260909-010416-153` also failed.
  Do not label either whole smoke run successful merely because its read steps passed.

### 1582 Foreground And Download Evidence

On 2026-09-09, APK 1.1.1582 still uses adapter 306. A background MCP `ready`
snapshot coexisted with `com.elon.quant/.grids.host.HostedGridActivity` as the
actual foreground. A background catalogue attempt timed out; after bringing the
production native chat forward, its read succeeded. This does not establish
the cause of every earlier selector mismatch or timeout.

`Open-WebChatNativeChatSurface` now verifies the resumed native Activity as well
as the provider/composer snapshot. The APK MCP helper explicitly bypasses the
Windows proxy for loopback HTTP and refuses redirects, preserving the response
body deadline. Local proxy-trap, UTF-8, redirect, timeout and foreground tests
pass in PowerShell 5.1 and 7. These are host-script fixes, not an APK update.

Foreground handler acceptance (`library-foreground-read-20260909-015735-978`):
the root contained 21 items and the next read contained 41. Earlier rows were
retained, handles were unique and `has_more=true` remained explicit. Read times
were 2360/2309 ms, not rendering timings. Pagination data projection is verified;
the native More button and rendered browser are not yet accepted.

An existing small synthetic text fixture was selected without changing its
contents. Download failed before any saved bytes, with
`download_source_unsupported`; a focused diagnostic reproduced it in
`library-download-failure-20260909-020110-577`. The native state was `failed`,
received bytes zero. Do not count dispatch or a download handle as successful
storage. The final download source/redirect must be observed before changing
the allowlist; no unchecked origin was admitted. Conversation URL, draft,
streaming and dictation state were preserved; no microphone or mutation was used.

The binary-source failure is now diagnosed and fixed below. Rendered browser
root/folder/back/search/More controls remain pending; do not repeat successful
catalogue protocol or pagination research. Sign-in is not a current prerequisite.

### 1583 Library Download Redirect Fix

On 2026-09-09 the current production package was 1.1.1583, source `c54c063c1`.
A same-signed local diagnostic build enabled the existing research flag without
clearing data. An initial 1582 diagnostic install was safely rejected as a version
downgrade; the matching 1583 package was then used. No older business source was
installed over a newer source revision.

The owned app WebView network observer saw the actual request chain:
`GET /api/library/files/{id}/download` returned 302 to
`https://chatgpt.com/backend-api/estuary/content?...`, which returned 200,
`text/plain`. The old module rejected this successful same-origin response with
`download_source_unsupported`, before writing bytes. URLs, query values and
credentials were not exported. Log: `library-download-source-probe-20260909-024231-661`.

Module 8 accepts this route through the existing strict content-source validator;
it does not accept arbitrary same-origin paths or change direct-content requests'
no-redirect policy. The same body goes to the existing one-use native byte owner.
Login/HTML, unexpected JSON, hostile origin, userinfo, alternate port, fragment
and lookalike path cases remain rejected. The three focused suites passed 84
cases: `library-redirect-regression-20260909-024343-637`.

The updated asset was loaded into the diagnostic WebView after its normal adapter
attachment (opening the native surface reattaches the packaged asset). The native
download handler returned `download_saved`, with 78 received bytes; the resulting
78-byte test file was independently found in public Downloads at the receipt's
time. Conversation URL, draft and authenticated state were preserved. No message,
file mutation, microphone or second download GET was used for this successful case.
Log: `library-download-fix-device-20260909-024525-670`.

This is real byte-storage evidence for the patched asset, not yet evidence that
the normal release package includes the fix or that the library dialog was
visually accepted. The unmodified normal 1583 APK was restored and the temporary
debug port removed before release work. The next grouped acceptance should use
the newly published normal package, then cover the remaining native browser controls.

### 1584 Normal Release Acceptance

- Published and installed normal Release 1.1.1584 (1584), source `2a05ae2c6`;
  release log `library-redirect-release-20260909-025006-718`.
- Published and installed APK SHA-256 both equal
  `cd009347105c49b386ae55cf0494caa6413cbc820075987bd8285ba2dcf51c05`.
  The owned app WebView debug socket is absent; no runtime asset injection was used.
- One production-handler download passed with `download_saved`, native `saved`,
  78 received bytes matching the selected fixture size. Conversation URL, draft and
  authenticated state were preserved. The acceptance script fails unless these
  conditions hold: `library-download-1584-acceptance-20260909-025858-908`.
- The ordinary library redirect/storage defect is fixed, device verified and
  released. Reuse this capability; do not repeat protocol research without a new
  regression. Native dialog controls, shared/mounted scope variants and library
  mutations/associations retain their separate pending acceptance status.

Ordinary file rename, soft deletion and [composer association](chatgpt-private-library-attachment.md)
are also packaged in 1581; the [native mutation owner](chatgpt-private-library-mutations.md)
still needs live acceptance.
Independent-library external mounted-file download, saved-entity/artifact previews,
folder mutations and moving files are not implemented by this browser. Such
rows remain visible where their shape is recognized, without a false Download
button. The explicit official-library entry remains available. Existing
conversation-bound mounted-file download support is retained separately.
