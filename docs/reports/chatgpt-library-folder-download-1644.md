# Project Catalogue Download Regression

## Device Evidence

On 2026-09-11, normal 1.1.1644 (adapter 327) was accepted over wireless ADB on
the previously authorized Xiaomi handset. Production `social_ai/chatgpt_web`
was authenticated, with two existing messages and an empty draft.

The native Library root contained 104 rows after bounded pagination, with no
remaining cursor or partial flag. Its single directory contained four PNG files.
Those rows exposed Download but not Attach or Rename. These affordances alone
do **not** identify a mounted provider file. The initial working hypothesis that
the directory was mounted was not established.

`native-folder-download-1644-20260911-042942-767` used the actual native sidebar,
Library, directory, file menu and Download button, selecting one existing 2122-byte
PNG. It failed: the structural observer saw exactly one file request,
`GET /api/library/files/{id}/download`, HTTP 404, with zero dropped observations.
No saved-byte pass was recorded. The original conversation, message count, empty
draft and awake setting were restored. No message was sent, cloud source modified,
or filename, provider ID, credential or file content exported.

## Root Cause And Fix

The catalogue download registration accepted project `libfile` rows as personal
library references, discarding their backing `file_id` and project identity.
The existing metadata resolver was bypassed by `sharedLibraryFileId`, so the
personal-library anchor route was used even though mutation/attachment owners
already excluded project rows. The live 404 is consistent with this code defect;
source fields were not exported from the handset, so full scope confirmation is
part of corrected-build acceptance, not claimed from row affordances alone.

The phone's public runtime-assets diagnostic included these retained assets:

- `conversation-small-cudd01juo7e4yskq.js`, SHA-256
  `c3c57597bf6202630080f3e8f53cdb356175dd2ce646b0f3bed3d16a8970bea6`.
- `4813494d-fgzk5uadh2hlkw93.js`, SHA-256
  `853f1aec75ae5d51c679e0bbf2e8b62c0b514f81a2f1f2cbd24b0e47ecba6b80`.

The source was parsed locally and both hashes rechecked. `bqn` retains the node's
`gizmo_id`, backing identity and project classifier. The document preview passes
the personal-library anchor only for confirmed non-project files. Its imported
`dp` resolves to `DDt`; `DDt/sDt` resolves metadata before `EDt` authorizes a file
with the effective project and `download_intent=true`. These source observations
are not a successful request on the corrected installed build.

Download owner 19, adapter 328, preserves the project catalogue selection and
reuses the existing metadata, authorization, native transfer and save owners:

- Exact concrete backing and library IDs are captured; project scope is obtained
  from the source or authoritative metadata, never the currently open chat.
- Metadata must confirm matching file/library IDs and project ownership. A
  conflicting project or missing authoritative scope stops without another route.
- Standalone catalogue metadata requests omit absent conversation IDs instead of
  serializing `undefined`. Ordinary conversation requests keep their existing IDs.
- Cached handles include backing identity and project scope. Listing sends no
  download/materialization request; identity/document/route changes invalidate work.
- No guessed endpoint, new HTTP client, DOM activation, write replay, proxy change
  or attachment/mutation capability is added.

Scoped capability: `android_chatgpt_private_project_catalog_download_v1`.
Completed and default-enabled for the verified native project-catalogue PNG
download on normal 1648. Earlier failed candidates are retained below as history.
This does not complete mounted materialization, mounted attachment or citations.

## Verification

The focused fixture exercises 29 cases through the real download/JSON/native-byte
owners with a throwing DOM getter. It includes unrelated ordinary/project/temporary
routes, source scope, metadata mismatch, missing/invalid identities, changed owners,
cache keys, HTTP denial and ignored abort. Byte/HTTP acknowledgements are synthetic.

`project-catalog-download-related-20260911-043725-615` passes 191 tests, zero
failures/skips/cancellations, covering the new fixture and existing catalogue,
personal/mounted downloads, citations and image downloads. The native acceptance
script's parser/privacy guard also passes. This is not a temperature measurement.

The external Java acceptance step validates saved size and decodes the PNG **on
the phone**. It exports only booleans/counts/byte length, retains any successfully
downloaded copy, and never pulls file content to the PC. It requires explicit
phone-only-download authorization, a bounded selection, exact native file title,
and a unique newly saved filename. It does not create a new cloud fixture.

## Normal 1645 Follow-up

Normal 1.1.1645, adapter 328, source `a8fb45469fd2db7847346250a4dbcd4850574125`,
was published and automatically replacement-installed on the same Xiaomi. APK
SHA-256: `692e7e2df9a2f270452c7e8f7a856e725c229cdc1aeac7995ec79850be620e6e`.
Build/publication run `project-catalog-release-20260911-044220-165` passed.

`native-folder-download-1645-20260911-045545-484` failed after 41 seconds:
the actual native Download now issued metadata `/backend-api/files/{id}/simple`
and authorization `/backend-api/files/download/{id}`, both HTTP 200, with zero
dropped observations. The personal-library 404 is gone. The native receipt was
`download_source_unsupported`, received bytes zero; storage was **not** accepted.
Conversation, message count, empty draft and awake setting were restored.

The current diagnostic lacks the returned URL's category. Adapter 329 adds
on-demand `file_download_source` diagnostics with only fixed origin categories,
allowlisted route words, opaque segments and boolean formatting flags. The URL,
host identifier, query, signed values and filename never enter diagnostic output.
The observation expires with the identity/document or after two minutes and does
not broaden download admission, start network capture or issue another request.
139 related Node cases and the native acceptance contract pass. App-only run
`download-source-native-app-gate-20260911-051208-265` reports BUILD SUCCESSFUL;
the fresh XML contains 10 tests, zero failures/errors. The earlier root task
matched an unrelated module with no matching tests and is not counted as a pass,
despite its command wrapper's zero exit code. Corrected download completion still
requires actual saved bytes; this diagnostic alone is not a fix.

## Project Content Transport

Normal 1646 (adapter 329, source `af17947564d598995879976524be74370c896f2f`)
was published and replacement-installed. APK SHA-256:
`fb63775543bc749bd27feec181e7c8c33c59b7bd017dad94c28f1ec11ac2fc92`.
`native-folder-download-1646-20260911-052531-403` again restored the original
conversation and awake setting. Its diagnostic identifies a relative same-origin
`/api/library/files/{id}/{id}` route, without whitespace, credentials, port or
fragment. No content request or saved-byte success occurred.

The already inventoried shared public asset contains `CDt(libraryId, fileId)`:
`/api/library/files/{libraryId}/project-content?file_id={fileId}`. `DDt` downloads
the URL returned by `EDt`; it does not restrict returned sources to estuary or
external signed URLs. This gives a concrete source-backed candidate, not a guessed
request. Adapter 330 passes the verified metadata's paired library/file identity
to the existing content-byte owner and admits that **returned** project-content
URL only when both IDs match and `file_id` is its sole query parameter. Other
routes, external origins, extra/duplicate query keys and missing scope remain
rejected. Ordinary previews do not gain unscoped access to project content.

`project-content-download-20260911-052912-717` passes 220 related Node cases,
including actual owner/byte integration and rejected scope mismatches without
another fetch. This does not yet prove the installed phone's saved bytes.

## Adapter 331 Candidate

Normal 1647 (adapter 330, source `7fbf48a01d223da4838d73344bf44aa4d607dc43`)
was published and replacement-installed. APK SHA-256:
`00a401e6e2b76b7127f87f3530fa5a0f47527f760d84cc1275aa9aeb043663c8`.
`native-folder-download-1647-20260911-053914-925` still failed before any content
request. The diagnostic now confirms `/api/library/files/{id}/project-content`;
metadata and authorization returned 200. The original conversation, message count,
empty draft and awake setting were restored. No saved-byte pass is claimed.

Adapter 331 retains opaque server-returned query parameters while requiring exactly
one matching `file_id` and the verified library ID. Official `DDt` consumes the
returned address, not a locally reconstructed preview URL. The sole-query rule
was unnecessarily restrictive; whether it caused this device failure is not yet
proven. Source diagnostics v2 add only a closed binding-result enum and bounded
query count to distinguish missing scope, library mismatch and file-ID failures.
No query names/values or concrete IDs are exported. Native validation accepts v1
for compatibility and rejects unbounded or additional fields.

`project-content-binding-20260911-054916-818` passes 222 related Node tests.
Actual saved-byte acceptance remains required for this scoped capability.

## Normal 1648 Acceptance: Completed

Normal 1.1.1648, adapter 331, source `acfa027d513b09b0f94d5fccb9c0123cd4948b7e`,
was published and replacement-installed on the trusted Xiaomi. APK SHA-256:
`4b23c6ff9fb824154da6d2cd1bd86eff6c16dad8290c0c0f3472e0db598f3ea4`.
Release run `project-content-query-release-20260911-055558-519` passed with
server publication and unattended device update verified. App test run
`project-content-binding-native-20260911-054937-796` reported BUILD SUCCESSFUL;
the fresh XML records 11 tests with zero failures/errors. All 114 unique assembled
JavaScript assets and the combined bundle parsed successfully.

`native-folder-download-1648-20260911-060345-077` passed the actual production
sidebar -> Library -> project folder -> file -> Download path:

- Metadata, authorization and `/api/library/files/{id}/project-content` each
  returned 200, with zero dropped observations and no personal-library request.
- Native receipt was `download_saved`; 2122 bytes were received and one new PNG
  was saved and decoded on the phone. No file content was exported to the PC.
- Binding diagnostic was `matched`, query count **2**. Together with the old
  one-parameter guard and the now-successful transfer, this confirms that extra
  server-returned query parameters caused the 1647 rejection. Values stay private.
- The native action completed in 6991 ms (including semantic helper execution
  and saved-file checks); this is not a pure network latency measurement.
- Original conversation, message count, empty draft and awake setting restored;
  no messages sent, no source files changed, downloaded copy retained.

`android_chatgpt_private_project_catalog_download_v1`: `code_status=implemented`,
`verification_status=device_verified`, `completed=true`, `default_enabled=true`
for this scope. Reuse it without further research or expanded samples absent a
new regression. Mounted materialization/attachment, citation-only references,
large-transfer cancellation/crash and other content types remain separate cases.
