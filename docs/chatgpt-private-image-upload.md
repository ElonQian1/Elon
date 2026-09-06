# ChatGPT private image upload

Capability: `android_chatgpt_private_image_attachment_upload_v1`.

Status: **implemented, offline verified, grouped Android and device acceptance
pending**. Adapter 270 is a source candidate, not a published APK. The earlier
78-byte text upload does not prove a real image upload or model image reading.

## Evidence and protocol

The public source fingerprints are shared with
[the private attachment contract](chatgpt-private-attachment-upload.md).
In `8b34dbc2-kjj15hg4y6iyx13p.js` (SHA-256
`9990fb9a8682917d0d790acf7b6aa78355e8520e4ffd2c5e0a183212d612d4b5`):

- The official upload dispatcher selects `I_.Multimodal` for image MIME types.
  `cKt` obtains decoded dimensions and bounds the usual image preparation to
  a 2048-pixel maximum edge. Its fallback resizes when an edge exceeds 2048 or
  the original bytes exceed 1 MiB.
- `SGt`/`jGt` creates the file with that use case and actual MIME/byte count.
  `VGt` uses the same legacy signed-blob and processing transaction already
  implemented for text, then calls `onFileUploaded` with `imageDimensions`.
- The current legacy processing body does not add width/height fields. The
  reservation `claim_and_finish` branch does, but is a different protocol and
  is not implemented here. Do not mix those request bodies.
- The official `uploadCompleted` store update places width/height directly in
  `fileSpec` alongside the file ID, name, size and MIME type. A generic ready
  text-file descriptor is insufficient for image association.

`4813494d-hrplraurzfyvxb10.js` defines `Multimodal` as `multimodal` in the upload
use-case enum, not the model tool label. No undocumented image-specific endpoint
was invented and no runtime credential was exported.

## Integration

The production camera/photo picker already normalizes static JPEG, PNG and WebP
through `AttachmentFiles.kt`, retaining decoded dimensions and a 4,000,000-pixel
budget. This existing shared path is unchanged. The native attachment gateway
now accepts those MIME types only when the existing metadata and byte count fit
that budget; missing dimensions and other formats retain the prior route.

`chatgpt_web_native_attachment_source.js` reuses the same selected-file lease and
64 KiB sequential bridge reads. `chatgpt_web_private_attachment_image.js` checks
the decoded dimensions against the native metadata. Small already-prepared images
reuse the same bytes; other images are bounded to a 2048-pixel edge using platform
bitmap/canvas primitives. PNG transparency and image MIME type are retained.
No image is uploaded from a server path or unrelated photo library entry.

Image preparation has a ten-second deadline, cancellation and late-decoder
cleanup. Bitmaps are closed and canvas allocations released on success, error,
cancellation and timeout. This is one explicit operation, not background polling.
These are resource-lifecycle properties, not a measured temperature improvement.

The private upload uses `use_case=multimodal`, no retrieval indexing and the
existing ordinary new/existing-chat context binding. Prepared image dimensions
are copied before asynchronous work. The final receipt must match the file and
processing terminal event; a contradictory server MIME is rejected. The composer
then confirms exact ready-store association including dimensions. The existing
native send owner alone sends the prompt after readiness.

Unavailable local preparation can select the original compatible route before
byte reading or private writes. Cancellation, a changed context or a failure
after private dispatch cannot automatically launch another upload or send.
Projects, temporary chats, GIF/SVG, PDF, multipart and image-library persistence
are not enabled by this change.

## Verification and next acceptance

The four focused Node suites passed 60 cases on 2026-09-06: image preparation,
private attachment transport, native byte source and composer integration. They
cover the complete production asset bundle parse, MIME/use-case selection,
dimension binding, unchanged text behavior, private POST/PUT/process/store flow,
no credential forwarding to blob storage, no write replay and graphics cleanup.
Browser decoders and HTTP responses are synthetic in these tests.

Three Kotlin policy tests are included for the grouped Android run. No Android
compilation, APK installation or actual browser decoder/device image transaction
is claimed in this batch. Acceptance must select a fixed synthetic image through
the production camera/photo workflow, verify private upload and image association,
then require a single model reply describing the fixture without supplying that
description in the prompt. Check both ordinary new and existing conversations,
and a cancellation without duplicate upload. Do not repeat the already-confirmed
voice, dictation or small-text protocol experiments for this capability.
