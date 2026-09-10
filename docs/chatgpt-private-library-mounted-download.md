# Mounted Files In The Native Library

## Status

Capability: `android_chatgpt_private_library_mounted_download_v1`.
Implemented and offline verified on 2026-09-10. Included in installed normal
1639 (adapter 323); production saved-byte acceptance remains pending.
Not `completed`; see the [release receipt](reports/chatgpt-mounted-library-delivery-20260910.md).
Activation: adapter 313, file-download owner 17 and library-download module 9.
Source commit: `57435cf2d`.
Source ancestry and the installed 1639 package are independently verified;
see the [current delivery evidence](reports/chatgpt-regeneration-observation-1637.md#normal-1639-delivery-and-fallback-evidence).
Do not rebuild or reimplement this module merely to resume its device acceptance.

This closes the standalone catalogue-to-download gap, not a new cloud connector.
The existing native library could display mounted files, but
`registerLibraryFile` only admitted ordinary `libfile` entries. The conversation
file index already had a mounted-file download transaction. Both surfaces now
reuse that transaction and the same native progress, cancellation and save owner.

## Contract Evidence

Retained official public assets were hash-checked and inspected on 2026-09-10:

- `conversation-small-owrec55n6vm0ekcc.js`, SHA-256
  `7973d518b083f0f3e23905a279ed019378481bdbdd10fc0196afe9fc7b3b7d35`.
  `JSn/kF` supplies library nodes. `mWn/PYt` maps a concrete external node ID
  into its mounted provider source; a separate `file_id` may be a copied file,
  and `cloud_doc_url` is attribution. `e1n/cB` checks download/export eligibility.
  `a1n` calls `qR` to materialize the mounted ID with
  `{file_id, name, mime_type, index_for_retrieval: false}`, then passes the
  returned ordinary file ID/name to `Ore`.
- `4813494d-o593jrji51wy4azk.js`, SHA-256
  `48563cd22f0dafe6c0b89220348fa3add81ff3abb82a62ed9d68a04d569cc375`.
  Export `Jd` is `AEt`, imported as `Ore`. With no conversation or project
  supplied, `AEt/kEt` authorizes the returned file with `download_intent=true`
  and no fabricated conversation/project query parameters.

These are source observations, not successful new real-account requests.
The existing [mounted-file contract](chatgpt-private-mounted-file-download.md)
still controls canonical provider IDs, source MIME and permitted export formats.

## Ownership And Boundaries

- A native catalogue row with a concrete Drive, Box, Dropbox or SharePoint file
  ID can obtain an opaque download handle. Listing/cached redisplay performs no
  materialization or download. The existing native Download action initiates it.
- `catalogTarget` validates the node and delegates to the existing mounted
  validator. Node ID, source MIME and name are retained by the page-local owner;
  neither the native row nor its byte packets expose provider IDs/credentials.
- Standalone selection uses `/library`, never the current chat's conversation ID,
  temporary-chat state or project ID. The existing conversation-scoped download
  behavior remains unchanged. Source URLs are never fetched as file bytes.
- The existing materialization POST, ordinary authorization GET and native byte
  transaction are reused. Drive Docs/Sheets/Slides retain the existing
  DOCX/XLSX/PPTX export validation and resolved native filename/MIME handoff.
- Mounted handles are consumed before the POST. Repeated delivery of a consumed
  handle cannot repeat the write. An active duplicate reports busy. Listing
  again may offer a new explicit selection, not an automatic request replay.
- The existing two-minute selection lifetime, 15-second preparation budget,
  bounded metadata requests, byte timeout, cancellation and account/document/
  route checks remain. Cached handles also compare MIME, avoiding stale export
  metadata when an observed node changes type.
- External-account override rows, project-scoped nodes, alternate preview targets,
  saved artifacts, trashed rows, containers, unknown exports and URL-only cloud
  references remain unsupported here. Existing official entry points are kept.
  This does not broaden attachment association, rename/delete or folder writes.

## Verification

The new integration fixture runs the actual catalogue, file-download,
materialization, bounded HTTP and native packet owners together. Its DOM getter
throws. It covers five provider-ID forms, no write on listing, immutable selection,
unchanged ordinary/temporary/project conversations, all three Drive exports,
exact saved bytes, invalid scopes, cache identity/MIME, cancellation, timeout,
denial and duplicate dispatch. HTTP and Android save acknowledgements are fixtures.

The corrected fixture was also run with both download modules loaded directly
from base `a197af736` without reverting the worktree: 12 failures and one passing
unsupported-scope case, with no skipped/cancelled tests. Failures identify missing
mounted handles (`mounted-catalog-baseline-check-20260910-124427-746`). An earlier
test-fixture wait lacked an admission assertion and was stopped by the bounded
runner; it is not counted as a complete baseline run.

The final related suite passes 244 tests, zero failures/skips/cancellations:
`mounted-catalog-final-20260910-124350-243`, 1.8 seconds. Coverage includes existing
library/citation/image/connector downloads and one-time upgrade from adapter 312
without replacing identity/audio. All 108 production adapter assets and the
combined JavaScript bundle also parse successfully. This does not establish live provider access,
Android rendering/storage, network performance or thermal improvement.

Next acceptance: on the grouped APK, use one already-authorized synthetic mounted
file in the production native Library, tap its Download action, verify private
route and actual saved bytes, then restore the original panel/conversation/draft.
Do not connect a new provider or mutate a user's cloud file just to create a case.
