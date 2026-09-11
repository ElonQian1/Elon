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

The nine unresolved PNGs are a concrete native-action coverage gap, but their
cause is not established. The current MCP row omits the source fields needed to
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

## Next Evidence Required

Inspect only an unresolved row's bounded source classification inside the
existing page owner: ID category, artifact type, saved-entity presence and scope
flags, without credentials or private values. Then trace that exact official
download path and reuse the existing owner for a targeted implementation/test.
The ordinary library and gallery saved-byte paths are already accepted; repeating
them cannot establish the missing source contract. Normal Release has WebView
CDP debugging disabled; a missing debug socket is not an ADB/network failure.

Mounted download/attachment still need a genuine authorized mounted sample;
this PNG inventory is not such a sample. Do not connect a new cloud account or
create unrelated cloud data merely to manufacture acceptance coverage.

See the [current batch map](../web-ai-private-native-remaining-batch.md) for
implemented, accepted and deferred scopes. Overall Goal remains active.
