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
Code implemented; offline verified; corrected-build device acceptance pending.
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

Next: install the grouped corrected Release and run this same native case once.
Require successful scoped authorization, native save receipt, phone-side bytes,
and restoration before marking this capability completed. A missing eligible
project node, network failure or another 404 must remain distinct from success.
