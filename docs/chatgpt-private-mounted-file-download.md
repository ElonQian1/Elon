# Mounted conversation-file downloads

Capability candidate: `android_chatgpt_private_mounted_file_download_v1`.
Implementation: scoped provider-file materialization and existing native download.
Verification: 119 focused Node cases passed; real account/device pending.
Delivery: source candidate for the next grouped APK, not `completed` or installed.

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

Library module 5 recognizes strict current metadata or an exact matching library
attachment. Source/provider conflicts, ordinary copied-file identities, alternate
preview targets and unrelated scope cannot fall through to this new POST.
File-download module 11 consumes the selected handle before materialization.
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
- Provider-native exports can change file name/MIME. The current native lease
  fixes those fields before authorization, so changed exports and Google-native
  MIME types remain explicitly unsupported rather than saved under a false name.
  Next implementation: validated exported name/MIME handoff to the same native
  byte owner, then the source-evidenced Docs/Sheets/Slides download variants.
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
