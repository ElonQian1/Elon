# Native ChatGPT File Library

## Scope

- Capability: `android_chatgpt_private_library_browser_v1`.
- Code: implemented for root/folder browsing, explicit search, cursor pagination,
  refresh and ordinary library-file download. Source-only for the grouped APK.
- Verification: targeted offline protocol/native tests; device acceptance deferred.
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
- Android Debug source/test compilation and 20 targeted JVM tests passed;
  log `library-native-final-20260908-220334-323`. No APK was assembled or released.
- Xiaomi wireless ADB reconnected, but the device remained locked on 1.1.1576.
  This source-only capability is not installed or device-accepted there.

Run one production-UI root/folder/back/search/pagination sample and one ordinary
saved-byte download after the grouped APK is installed. Verify the original conversation,
draft and any active voice session remain unchanged. Do not infer an actual speed
or temperature improvement from cache fixtures alone.

Ordinary file rename and soft deletion now have a source-only
[native mutation owner](chatgpt-private-library-mutations.md), pending grouped acceptance.
Independent-library external mounted-file download, saved-entity/artifact previews,
folder mutations and moving files are not implemented by this browser. Such
rows remain visible where their shape is recognized, without a false Download
button. The explicit official-library entry remains available. Existing
conversation-bound mounted-file download support is retained separately.
