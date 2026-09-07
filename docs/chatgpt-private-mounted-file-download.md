# Mounted conversation-file downloads

Capability candidate: `android_chatgpt_private_mounted_file_download_v1`.
Implementation: scoped provider-file materialization and existing native download.
Verification: 185 related Node cases and 12 pure Kotlin/JUnit cases passed;
full Android integration and real account/device pending.
Delivery: source candidate for the next grouped APK, not `completed` or installed.
Adapter version: 297; file/history/library modules 12/6/6.

This extends the [existing library download owner](chatgpt-private-shared-library-download.md).
It does not create a second file picker, progress UI, byte-transfer worker,
identity cache or independent HTTP client. Previously accepted ordinary files,
audio, subtitles, dictation and read-aloud are unchanged.

## Source evidence

Retained public ChatGPT assets were hash-checked on 2026-09-08:

- `conversation-small-owrec55n6vm0ekcc.js`, SHA-256
  `7973d518b083f0f3e23905a279ed019378481bdbdd10fc0196afe9fc7b3b7d35`.
  `qR` posts `{file_id, name, mime_type, index_for_retrieval}` through the
  official same-origin client to `/files/library/mounted/materialize`.
  The download helper `a1n` passes `indexForRetrieval: false`, then downloads
  the returned `file_id` and `file_name` through the ordinary `Ore` owner.
  `z$n` serializes mounted attachment identity; the mounted-reference serializer
  separately emits `metadata.mounted_library_file_references` with
  `mounted_library_file_id` and `name`. The current composer supplies that
  collection to its message reference renderer, which opens the provider preview.
- `4813494d-o593jrji51wy4azk.js`, SHA-256
  `48563cd22f0dafe6c0b89220348fa3add81ff3abb82a62ed9d68a04d569cc375`.
  `Pjt`, `IQ` and `zQ` establish concrete Google Drive, Box and Dropbox file IDs;
  folder/root/collection IDs and unrelated provider URLs are not file downloads.
  The existing ordinary authorization and binary owners remain authoritative.

The attempted unauthenticated read of the lazy citation-preview asset timed out.
No authenticated request, mounted response, provider entitlement or device save
is inferred from public-source inspection. Citation-graph-only files remain a
separate research gap, not guessed `libfile` references.

## Integration and lifecycle

History projection 6 appends mounted references after existing image, ordinary
attachment and shared-reference rows. A matching attachment takes precedence,
repeated mounted IDs are deduplicated, and the selected branch/hidden-message
rules remain. Per-message 20 references, 100 index rows and 80 displayed messages
stay bounded; incomplete indexes report truncation. Native rows receive names
and opaque expiring download selections, never mounted IDs or provider URLs.

Library module 6 recognizes strict current metadata or an exact matching library
attachment. Source/provider conflicts, ordinary copied-file identities, alternate
preview targets and unrelated scope cannot fall through to this new POST.
File-download module 12 consumes the selected handle before materialization.
There is one write attempt, no automatic retry, no alternate writer, and no DOM
fallback after an ambiguous response. A new explicit selection is required after
failure. Repeated clicks during the same job report busy without restarting it.

The identity WebView issues the same-origin POST with a six-second JSON deadline,
a 64 KiB response limit and no retrieval indexing. It checks the returned
ordinary ID, name, MIME and size before existing scoped authorization. Account,
document, route, cancellation and project guards remain active at every await.
The existing 15-second preparation and 120-second byte-transfer limits remain.
Only a successful native save is reported as `download_saved`; a system-queued
signed download retains its distinct receipt. Failed writes never become success
or an instruction to replay a request. Cookies and request headers stay in-page.

## Remaining work and acceptance

- This batch covers concrete Drive/Box/Dropbox files and matching history
  metadata. Standalone browsing, SharePoint, other providers, citation-graph-only
  references and mounted files represented only by a separate preview remain gaps.
- Drive Docs/Sheets/Slides exports and validated resolved name/MIME handoff are
  now implemented below. Other provider-native formats remain unimplemented.
  Live export bytes, names, MIME types and final storage still require acceptance.
- Synthetic tests exercise production history/index/selection, exact POST and
  scoped authorization, immutable IDs, bytes, cancellation, identity changes,
  deadlines, repeat clicks, server errors and malformed/oversize responses.
  They do not establish real provider access, redirect behavior, rendered UI,
  network latency, battery consumption or Android storage success.
- In the grouped ChatGPT acceptance, use one existing user-authorized test file
  from a connected provider, verify its actual saved bytes and route, then cancel
  a separate transfer. Restore the original conversation/draft. No extra audio
  acceptance or Google AI protocol work is required for this download case.

Do not promote this candidate to `completed` based on an offline or build pass.

The first new-suite run failed against the unchanged owners because mounted-only
rows and download selections did not exist. The final combined run passed 119
Node runner cases across mounted, shared/personal library, ordinary, connector
downloads and history projection, with no failures/skips. Log stem:
`mounted-download-regression-20260908-20260908-055152-533`.
HTTP and native acknowledgements were synthetic. Android packaging and real
provider/file acceptance remain deferred to the grouped delivery.

The adapter upgrade case fails with target 295 because an already-loaded 295
bridge skips reinjection. Target 296 reloads the updated modules, retires the old
download selection registry once, and preserves the identity/audio owner objects.
This is a bootstrap contract test, not a live call or installed-APK acceptance.
No extra runtime asset, Kotlin branch or periodic background work was added.
The combined upgrade/download/transport/new-chat suite passed 177 Node runner
cases with zero failures/skips. Log stem:
`mounted-final-related-20260908-20260908-055915-448`.
This supersedes the 119-case checkpoint only for offline verification.

## Native export handoff

The same retained official source exposes `cB`/`n$` for supported Google-native
downloads, `fEt` for DOCX/XLSX/PPTX suffixes, and `SEt` for mounted MIME precedence.
Adapter 297 uses that source MIME in the existing materialization POST. It checks
the response name and exported type, using the evidenced format mapping only
when the optional response MIME is absent. Unknown Google-native types and Box
native-document formats remain rejected; no endpoint or credential is invented.

The native gateway advertises `resolvedFileVersion: 1`. Materialization attaches
`resolvedFile: {version, name, mediaType}` to byte `begin` or the signed enqueue
packet. Only a consumed current document/route lease can apply these fields;
identity, request ID, expiry and cancellation ownership are unchanged. The
native validator bounds types/names, rejects unknown fields, sanitizes paths and
preserves the suffix of long exported titles. Both byte storage/notifications
and DownloadManager reuse this resolved lease. Neither route opens the file.
Old APKs reject known exports before materialization instead of saving a false
format; ordinary old packets keep their original lease unchanged.

Eight new export cases failed on the preceding implementation, then all 185
related Node runner cases passed (`mounted-export-green-20260908-20260908-062132-706`).
The native policy/lease tests were freshly compiled with Kotlin 2.0.21 and all
12 JUnit cases passed (`download-metadata-native-20260908-20260908-061553-622`).
The gateway integration test checks source wiring; it does not compile/run the
Android gateway. This batch did not assemble, publish or install an APK, access
an authenticated provider file, or measure performance/thermal improvements.
