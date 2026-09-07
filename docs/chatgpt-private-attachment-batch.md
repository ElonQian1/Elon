# Private native attachment batches

Extension of `android_chatgpt_private_attachment_upload_transport_v1` and
`android_chatgpt_official_runtime_text_submit_v1`, reviewed 2026-09-07.
Status: source implemented, focused tests and SDK partial compilation passed;
grouped Android build and production-device acceptance pending. Not completed.

## Scope and ownership

The production picker already allows nine selections, but the native byte
gateway, private composer owner and prepared-action sender previously admitted
only one. This extension uses those existing owners for one batch, not a second
uploader or text sender. It keeps the existing MIME, image, temporary, project,
library and selected-branch contracts.

- Native gateway: validate all selected FileProvider URIs and file policies
  before issuing leases. One file retains descriptor v1; multiple files use a
  v2 envelope with up to nine unique v1 leases for one document and URL.
- The existing native reader still reads sequential 64 KiB chunks on one I/O
  executor. Each file remains at most 8 MiB; all leases share a 120-second
  lifetime. Cancellation immediately revokes every reader and closes them off
  the UI thread. No file path or credential is accepted from page JavaScript.
- Sender 17 validates the whole envelope and confirms every scope before byte
  reads or uploads. It runs existing per-file create/byte/process transactions
  sequentially, preserving image preparation, reuse, reservation and privacy
  behavior. A mixed project batch rechecks support for each file category.
- Every file keeps its own explicit upload-copy choice. A batch cancels the
  singleton picker-prewarm selection; eligible individual uploads can use their
  own reservation. Explicit-copy files cannot reuse or claim a reservation.
- Composer 16 publishes all ready objects in one store update only after all
  files complete. Intermediate processing never releases the native send owner.
  Removing one selected ready file preserves the other owned entries.
- Runtime submit 4 carries the exact ordered, frozen ready-file metadata and
  original File references in one official `prepared_action`. Acceptance clears
  only those submitted objects; later drafts and unrelated files survive.

Failure or cancellation after a private write never sends a partial batch,
replays the text, or switches to DOM upload automatically. Already processed
remote files may remain after a later-file failure; there is no speculative
remote deletion. An explicit retry can create new remote artifacts.

Nine is the existing host selection bound, not an assertion of the account's
current upload quota. Model/account limits and server rejection still apply.
The bridge does not set `skipChatAttachmentLimits` or reset rate limits. A
dedicated early read of the current official count/quota is not implemented.

## Protocol evidence

The already inspected public composer asset, SHA-256
`9990fb9a8682917d0d790acf7b6aa78355e8520e4ffd2c5e0a183212d612d4b5`,
has an official consumer passing an array of ready entries to `prepared_action`;
`EOn` consumes that array when formatting the request. The source and submission
boundaries are recorded in [runtime submission](chatgpt-official-runtime-text-submit.md).
Per-file upload endpoints are unchanged. This is not independent Android HTTP
generation: message policy and generation still use the official page runtime.

## Verification and grouped acceptance

305 focused Node cases passed with zero failures or skips: all
`scripts/test-chatgpt-web-private-attachment-*.js`, plus native byte source,
runtime attachment submission and runtime text submission suites. The batch
tests cover two/nine selections, atomic readiness, per-file copy, partial
failure, malformed late descriptors, cancellation, identity changes, temporary
scope, removal and one prepared-action dispatch. Existing singleton, image,
document, project, byte-route, reuse and reservation tests remain covered.

32 Kotlin/JUnit tests passed across upload choice, native policy/reader, picker
preparation and attachment send tracking. Current changed Kotlin sources,
including gateway and popup, compiled against Android SDK 36 and cached untouched
application dependencies. This is SDK partial compilation, not a full Gradle
consumer build, Android WebMessage execution or rendered UI acceptance.

One existing reuse test now awaits its asynchronous stage deterministically
instead of assuming that 100 event-loop spins finish a hash. Its assertions and
production reuse behavior are unchanged. The Kotlin runner's initially missing
JSON resource was restored to its classpath; no failing test was skipped.

In the grouped ChatGPT APK, use the production camera/photo/file flow to select
multiple fixed fixtures. Retain the private association receipt, verify all
ready entries, send once and check that the reply uses each fixture's contents.
Also check remove/cancel, a mixed image/document batch, project placement and
per-file copy. Phone responsiveness, real upload limits, latency and heat remain
unmeasured here. Do not repeat endpoint research while these checks wait.
