# ChatGPT private PDF attachment upload

Capability scope: `android_chatgpt_private_pdf_attachment_upload_v1`.
Status: **implemented, compiled and published in 1541; focused Node/Android
checks passed; PDF production-device acceptance pending**. Reuse this extension of
[the existing upload transaction](chatgpt-private-attachment-upload.md); do not
build another uploader. It does not complete the whole attachment capability.

## Official contract

The public official asset inspected on 2026-09-06 is
`https://chatgpt.com/cdn/assets/8b34dbc2-kjj15hg4y6iyx13p.js`, SHA-256
`9990fb9a8682917d0d790acf7b6aa78355e8520e4ffd2c5e0a183212d612d4b5`.

- `VGt` selects the upload's model when the resolved file category is PDF or the
  name ends in `.pdf`. `jGt` passes that model as `x-oai-model-slug` on the
  same-origin `POST /backend-api/files` creation request. The legacy
  `process_upload_stream` request does not add that header.
- The official composer passes `currentModelId ?? currentModelConfig.id` to
  `KJ`, its file-drop handler; `KJ` forwards it as `modelSlug`. A model button's
  localized title or effort label is not this value.
- `wGt`'s project ingest branch indexes spreadsheet suffixes and flag-controlled
  images, not PDF. A writable project PDF keeps `use_case=gizmo`, project file
  metadata and the existing selected-branch origin contract.

These are current public-source observations, not a real PDF upload trace.
The earlier successful 78-byte text upload does not establish PDF acceptance.

## Implementation

The native MIME policy and sequential byte source now allow `application/pdf`
under the existing single-file 8 MiB limit. Bytes are not decoded as text,
rasterized, recompressed or converted to an image. No new picker or HTTP client
was introduced. Other document MIME types remain outside this extension.

The composer reads model IDs from the file input's bounded ancestor chain,
accepting only a branch proven current by the React root pointer. It checks the
host alternate when necessary; stale, ambiguous, missing or malformed model
state stays unknown. It never converts a label such as an effort setting to a
guessed slug. Unknown state selects the existing compatibility path before byte
reads or private writes, not an error claiming that PDF is unsupported.

The captured slug is copied before asynchronous authentication and guarded with
the page/account/store/branch identity throughout the upload. Only the create
request receives the PDF model header; the signed blob PUT receives neither it
nor page credentials. A changed model cancels without association or write replay,
including when the visible effort label has not changed.

Ordinary and temporary conversations reuse their existing persistence rules.
Writable new and existing projects reuse fresh permissions and selected-branch
binding. Read-only project dispatch, ingest-image flags, other document types,
reservation claims and multipart variants are not enabled by this PDF change.

Module versions: protocol 6, transport 5, composer 7, native byte source 3,
project 3, send owner 7. The send-owner version invalidates its older captured
dependencies on adapter reinjection. These extensions are packaged in
`v1.1.1541` (adapter 272); installation does not establish PDF runtime acceptance.

## Verification and next acceptance

The eight focused attachment Node suites passed **96 test-runner cases** on
2026-09-06, including complete production asset-bundle parsing. PDF tests cover
header separation, model snapshot/cancellation, current versus stale React
branches, binary chunk boundaries, native-byte-source through private transport
to ready-store association, temporary privacy, and new/existing project scope.
HTTP, file-picker context and project runtime are synthetic in these tests.

After the phone returned on 2026-09-06, the grouped Release production/unit-test
compilation and all 33 tests across seven attachment suites passed, including
the PDF MIME/size policy. `publish-apk.ps1` then published `v1.1.1541` from
`ac2f1662f` and installed it without clearing data on the authorized Xiaomi.
See [the grouped release record](web-ai-private-native-remaining-batch.md#grouped-release)
for its artifact digest and acceptance limits.

One plain-text production attachment send completed and its reply correctly
read the fixture's first line. It was not a PDF test, and a private-association
receipt was not retained, so it does not prove the integrated private PDF route.
No microphone was used. Actual PDF transfer and current runtime model binding
remain pending; do not rerun the unchanged compilation to address that gap.

For acceptance, use a small non-private PDF through the production chat's File
action. Check private create/blob/final processing and exact ready association,
then a single prompt that demonstrates reading the PDF. Confirm actual model
ownership, temporary non-library behavior and project placement; restore the
draft and conversation afterward. Missing runtime model props are a precise
remaining integration risk, not a reason to repeat the already-inspected upload
protocol or silently invent a model ID.
