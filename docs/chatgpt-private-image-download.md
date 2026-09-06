# Private conversation image downloads

Capability candidate: `android_chatgpt_private_conversation_image_download_v1`.
Status: **implemented, offline verified, grouped APK/device acceptance pending**.
Implementation commit: `31f1c10d4`. This is not a live private download pass.

## Confirmed official contract

The public assets inspected on 2026-09-07 are the same bytes previously saved
on 2026-09-06:

- `/cdn/assets/8b34dbc2-kjj15hg4y6iyx13p.js`, SHA-256
  `9990fb9a8682917d0d790acf7b6aa78355e8520e4ffd2c5e0a183212d612d4b5`:
  `jW` strips the image pointer and matches `metadata.attachments` by file ID.
  It supplies the image name, library file ID and conversation scope to `A5t`.
  `A5t`'s download action invokes `Xee()` with that file ID and conversation
  scope, plus any library file ID. `Xee` imports export `bf` from the shared
  asset; the pointer parser `s_` imports export `tp`.
- `/cdn/assets/4813494d-hrplraurzfyvxb10.js`, SHA-256
  `89c95d937bac1191e91d5ceb4872eb0c328d39a98ce05399093a663f18921aa0`:
  export `tp` is `TTt`, which strips `file-service://` or `sediment://`.
  Export `bf` is `pEt`, returning `fEt`. That existing download helper performs
  any library metadata resolution, then calls `dEt` with
  `checkContextScopesForConversationId` and `downloadIntent: true`.
  It does not use the image-preview URL as proof of download authorization.

The underlying endpoint and native lease are documented in
[private file downloads](chatgpt-private-file-download.md). No new endpoint,
credential type, Android HTTP identity store or alternate UI was invented.

## Implementation boundary

Private history projection version 4 exposes raw image descriptors only to its
page-local `fileSource` consumer. Ordinary history messages and file-index
projections still contain no raw pointers, file IDs or signed URLs. The selected
conversation branch determines which descriptor can be resolved.

Private file download version 3 registers simple `file-service://<id>` and
`sediment://<id>` pointers from user or assistant image parts. It uses the same
opaque, expiring selection handle and the existing native file-detail Download
action. The selected conversation is always checked by the authorization
request; project and library resolution retain the existing scope rules.

Matching attachment metadata supplies the filename and explicit image MIME
type. Without a filename, the native index uses `image.png`, following the
official helper's `.png` fallback but without exporting the file ID. That name
is not a conversion or proof of PNG bytes; without an explicit MIME type the
native download service uses the response metadata.

Duplicate matching attachments, truncated or malformed metadata, conflicting
projects, extra context scopes and non-image MIME metadata are rejected. Shared
library and connector descriptors, parameterized pointers, path-bearing pointers
and arbitrary URLs remain unsupported. Scope-like fields directly inside image
parts are also rejected rather than silently ignored. `library_download_id` and
attachment-level context scopes now reject broad file-download registration too.

Registration snapshots the target before asynchronous work. Account, document,
route or selection changes and cancellation prevent a late response from
enqueueing another file. Existing deadline, response-size, signed-origin and
one-use native lease checks are reused. A queue acknowledgement still means
only **queued**, never saved or downloaded successfully.

## Verification and next acceptance

Eight new tests exercise ordinary/user/assistant images, both pointer schemes,
project scope, matching filename/MIME/library scope, selected-branch isolation,
malformed metadata, mutation after selection and late cancellation. Six positive
tests failed against the prior implementation because it could not register
image downloads; all eight now pass.

The combined download, history projection, conversation-file index, private
transport, gallery and attachment-composer run passes 76 Node runner cases,
including the production asset bundle's syntax parse. No Kotlin/Gradle build,
APK publication, phone interaction or saved-byte verification occurred in this
batch. Existing voice, subtitles, dictation and Google paths were not changed.

Grouped device acceptance should reuse a synthetic image in an ordinary
conversation and one project/library-linked image. Select Download from the
production file detail, verify the actual saved bytes and MIME/open behavior,
and preserve the original draft, voice state and selected conversation. Do not
repeat protocol discovery or claim the queue receipt as transfer completion.
