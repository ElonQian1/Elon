# Library Source Audit On 1662

Read-only investigation on 2026-09-11, not a new completed capability.
Normal APK 1.1.1662 / adapter 346 was already installed. Wireless ADB and MCP
were reachable; the native ChatGPT provider reported authenticated and composer
ready. No microphone, upload, message, deletion, account switch or app-data clear
was performed. No new APK was built for this inspection.

## Live Observation

A library read failed with `library_read_failed` after 10492 ms. One explicit
refresh then succeeded. A bounded protocol probe observed
`GET /backend-api/files/library/nodes` returning HTTP 200, without dropped
records; the probe was stopped. This is not evidence of a general network outage.

The resulting first page had `stale=false`, `partial=false`, `has_more=true`:

| Kind / MIME | Rows | Download handles | Rename / trash / attach |
|---|---:|---:|---|
| Directory | 1 | 0 | None |
| PDF | 3 | 3 | All three enabled |
| JPEG | 1 | 1 | All three enabled |
| PNG, actionable | 2 | 2 | All three enabled |
| PNG, unresolved | 9 | 0 | All three absent |
| Text | 5 | 5 | All three enabled |

This is 21 first-page items, not the total library. A later MCP snapshot retained
these exact structural counts. Private titles, server IDs and source URLs were
not exported. No rendered action menu or saved-byte download was tested here.

The nine unresolved PNGs were a concrete native-action coverage gap, but their
cause was not established in build 1662. The MCP row omitted the source fields needed to
distinguish ordinary files, saved artifacts, cloud mounts and unsupported scopes.
`can_rename=false` alone does not identify any one of them. A matching byte size
with an accepted gallery image also does not establish that it is the same file.

## Public Runtime Evidence

The retained current public conversation asset is
`conversation-small-ft205i7yqa6zc2nj.js`, SHA-256
`a89952420338983e104e94be5fae8e9ae7a4169b9ea8a4ed30e639641e14f456`.

- `Uqn` (export `uDt`) maps library nodes using node/file IDs, parent directory,
  artifact and saved-entity metadata. `Y$t` distinguishes mounted provider
  sources from `chat_upload`; `saved_entity` has its own source type.
- `s4n` (export `Rxt`) excludes `deep_research_report` and delegates the remaining
  download eligibility to `dV`, which rejects flashcards and unsupported cloud
  export types. This is eligibility evidence, not a universal byte endpoint.
- Public `23e1194a-l1ao2vzh48b0r165.js` passes a resolved image URL as
  `downloadUrl` to its image viewer. The prefetch wrappers
  `5d847bf0-lci18yg5n8e271i1.js` and `03cc85d7-od8tz7jvn3e1re8s.js` are not
  download implementations. None establishes the unresolved live rows' source.

Existing owners remain authoritative: `chatgpt_web_private_library_catalog.js`
retains source metadata page-locally; `chatgpt_web_private_file_download.js`
selects the download route; `chatgpt_web_private_library_download.js` handles
validated library/mounted transactions. Do not add a parallel byte downloader,
convert thumbnails into originals, or guess a files/download ID from a title.

## Source Diagnostic On 1663

Normal Release 1.1.1663, source `a4935f803`, was published and installed with
`adb install -r` on the existing Xiaomi wireless transport. The new on-demand
`private_protocol_probe/library_sources` reads the existing page-local catalog;
it neither fetches data nor reads DOM itself. Both native validation and the
page owner restrict it to counts, closed source types and field-presence flags.
No filenames, IDs, URLs or credentials are included.

One explicit library refresh and diagnostic passed in 5.8 seconds total. The
21-row inventory was unchanged. All nine unresolved PNG rows had library node
IDs, ordinary backing-file IDs and a non-null artifact type outside the initial
diagnostic vocabulary. None had any reported cloud, saved-entity or project
scope flag. Conversation, draft and authenticated state remained unchanged.
This establishes an artifact-filter gap, not a mounted-cloud sample. It does
not yet establish the exact live artifact type or a successful download.

Evidence: `library-source-device-1663-20260911-20260911-181557-919` in the shared
AI command logs. The diagnostic's 15 JVM tests and JavaScript contracts passed
before publication. The diagnostic alone does not fix the missing actions.

## Generated Image Download Implementation

The retained public runtime explicitly recognizes `image_gen`. In `w3n`,
`z_e` / shared `fDt` first resolve backing-file metadata; a personal library
match supplies `libraryDownloadId` to `Vve` / shared `MDt`. `MDt` then uses the
existing `/api/library/files/{libraryId}/download` byte route. It does not need
a conversation navigation or a thumbnail-to-original conversion.

The targeted implementation reuses `registerLibraryFile`, `resolveDestination`
and the existing native byte lease. Only image_gen PNG/JPEG/WebP with bounded
library/backing IDs and no cloud/project/preview scope qualify. Metadata must
confirm the same personal library file before transfer. Unknown artifacts,
saved entities and contradictory ownership remain unclaimed. Rename, trash
and attachment are not enabled by this download-only change.

`test-chatgpt-web-private-library-generated-images.js` covers actual owner
composition, original-byte storage, immutable selection, unsupported sources,
metadata mismatch, cancellation, account/navigation changes and no error-path
replay. The diagnostic vocabulary now recognizes `image_gen` separately.
Real image_gen source classification and native saved-byte acceptance remain
pending until the new implementation is installed and exercised.

## Remaining Evidence

Use the extended diagnostic to confirm the live artifact type, then select one
matching row through the existing native library and verify a saved original.
The ordinary library and gallery saved-byte paths are already accepted; repeating
them cannot establish the missing source contract. Normal Release has WebView
CDP debugging disabled; a missing debug socket is not an ADB/network failure.

Mounted download/attachment still need a genuine authorized mounted sample;
this PNG inventory is not such a sample. Do not connect a new cloud account or
create unrelated cloud data merely to manufacture acceptance coverage.

See the [current batch map](../web-ai-private-native-remaining-batch.md) for
implemented, accepted and deferred scopes. Overall Goal remains active.
