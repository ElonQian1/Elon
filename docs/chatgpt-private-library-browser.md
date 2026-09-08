# Native ChatGPT File Library

## Scope

- Capability: `android_chatgpt_private_library_browser_v1`.
- Code: implemented for root/folder browsing, explicit search, cursor pagination,
  refresh and ordinary library-file download; published/installed in 1.1.1581.
- Verification: offline tests and native catalogue reads passed on 1581;
  rendered browser, pagination and saved-byte download acceptance remain pending.
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

Next acceptance: establish the foreground native surface/selector binding, then
sample browser root/folder/back/search/pagination and one saved-byte download.
Verify the original conversation, draft and voice state. Sign-in is not a current
prerequisite. Native data projection is verified; rendered UI acceptance is not.

Ordinary file rename, soft deletion and [composer association](chatgpt-private-library-attachment.md)
are also packaged in 1581; the [native mutation owner](chatgpt-private-library-mutations.md)
still needs live acceptance.
Independent-library external mounted-file download, saved-entity/artifact previews,
folder mutations and moving files are not implemented by this browser. Such
rows remain visible where their shape is recognized, without a false Download
button. The explicit official-library entry remains available. Existing
conversation-bound mounted-file download support is retained separately.
