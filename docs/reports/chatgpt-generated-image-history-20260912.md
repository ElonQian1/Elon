# Generated Images in Private File History

Date: 2026-09-12. Source evidence plus normal 1684 phone acceptance.

## Status and Scope

`android_chatgpt_generated_image_file_index_v1`:

- `code_status=completed`; `verification_status=device_verified`; default-enabled
  for the owned current generated-image scope. [Normal 1684 acceptance](chatgpt-image-files-1684.md)
  passed native file listing, one download, saved PNG decode and state restoration.
- Reuses the private history reader, selected-branch projection, opaque file
  registry, fresh download authorization and native save/cancel owner.
- Complements [generated bubble originals](chatgpt-generated-image-original-20260912.md).
  This file-index path does not need a rendered image, composer, DOM action or
  image fiber. The persistent identity WebView remains intentional.

## Root Cause and Protocol Evidence

The previous file projection accepted only user/assistant messages. In the
current public shared bundle `4813494d-gf2h57w5fiay19bd.js`, export `PY`/`Lx`
selects generated images with `zY`/`Ix`: author role `tool`, exact name
`t2uay3k.sj1i4kz`, content type `multimodal_text`, and no visually-hidden flag.
It extracts only `image_asset_pointer` parts, not arbitrary tool text.

Shared bundle SHA-256:
`6c015001732054f4143ef1922609407c540967762109dcd128bbf56706889c3e`.
The other renderer/download bundle hashes and the `lo`/`Xt`/`PDt` path are in
the linked original-download report. These were public runtime assets, not
private conversation or credential dumps.

`lo` passes the account-dependent watermark choice to `Xt`. Shared export
`bK`/`lw`, exposed by the existing runtime binder as `shared.mq()`, is a pure
session-store getter, not a React hook. `hasPaidSubscription()` determines the
same default variant as the website. The versioned `web_20260912` mapping is
required when a watermark variant exists. A missing account policy is a
preparation failure that can be retried, not evidence that images are unsupported.

This proved a projection gap in code. The later 1684 acceptance confirms the
owned generated sample now has a downloadable native file row. Raw private
history was not exported; other historical generated variants are not implied.

## Implementation Boundaries

- History projection 12 adds generated images to `files`/`fileSource` only.
  Ordinary transcript and context-source projection are unchanged. Tool text,
  hidden messages, internal attachments and citations are not exposed.
- Selected branch, duplicate-source rejection and existing row/part limits
  remain. Ignored tool metadata does not falsely mark the index truncated.
- New generated-download policy 1 validates both asset pointers. File downloader
  38 resolves policy only on an explicit Download, using the cached shared
  module or its existing bounded single-flight loader. No account endpoint or
  protocol field is guessed; no-watermark assets need no runtime policy read.
- Authorization uses the effective pointer, `conversation_id`, `inline=false`
  and `download_intent=true`, not a thumbnail or assumed project ownership.
- Every click rechecks policy and gets fresh authorization. Account, document,
  route, runtime or plan changes before handoff cancel the job. Returned file
  identity must match the chosen variant. Native code sees only opaque handles.
- Android adapter 364 registers the policy before the existing download owner.
  Existing gallery, uploaded-image, library, citation and connector paths remain.

## Verification

Two targeted Node runs passed: 75 and 230 tests (14 new tests overlap between
runs; the history script also checks 12 projection cases). All 127 production
adapter assets parsed. The shared regressions cover ordinary images, file
citations, libraries, mounted/shared references, source links and connectors.
The initial source batch did not compile Android; grouped release 1684 later
passed 29 Android unit tests, the normal APK build and production acceptance.

The [grouped report](chatgpt-image-files-1684.md) records the production Files
path, 671,371 saved bytes, PNG decode and restoration on the trusted Xiaomi.
No audio, private conversation changes, Cookie/data clearing or proxy changes
were required. Old DALL-E/image_gen names, other accounts/protocol builds,
shared-page variants and independent native generated-image transcript rendering
remain outside this accepted scope.
