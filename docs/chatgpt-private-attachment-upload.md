# ChatGPT private attachment upload

Capability: `android_chatgpt_private_attachment_upload_transport_v1`.
Status: **small-text private protocol verified; bounded native integration
implemented, grouped-device acceptance pending**. This is not yet an accepted
end-to-end attachment-chat capability. Do not repeat the confirmed protocol
acquisition or isolated small-text upload. Continue at the integration gaps below.

## Current evidence

On 2026-09-06, an isolated Release research build based on `99d5437cc` was
installed on the authorized Xiaomi with `adb install -r`. It retained version
`1.1.1540 (1540)` solely to allow restoring the exact installed package without
a downgrade or clearing data. It was never publicly published. Research APK
SHA-256: `1f42dab26e025ab28ac1406c95744b555158dcb6e2af45e9ba31d7758c4c1c82`.

Three distinct checks must not be conflated:

1. The corrected production native attachment smoke staged the fixed synthetic
   file, then waited for actual attachment readiness. Only
   `POST /backend-api/files/upload_reservations` returned 200. No byte upload was
   observed, and the smoke timed out instead of accepting a text-only reply.
   Native picker handoff is still broken; its exact cause is not yet established.
2. Supplying a synthetic `File` to the current official input's change handler
   bypassed that native handoff. Official `/backend-api/files` and
   `/backend-api/files/process_upload_stream` returned 200; the official file
   store contained one ready 78-byte file with a server file ID. This is a
   diagnostic control, not a production-native picker acceptance.
3. The new transport's own page-local private requests, without an official
   input click/change handler or imported vendor uploader, created a file,
   uploaded its bytes and received processing completion. It reused the existing
   private identity context. The synthetic file was 78 bytes, `text/plain`, with
   `use_case=ace_upload`, `store_in_library=false`, and
   `library_persistence_mode=required`. No message was sent and no association
   to the composer was claimed.

The real processing stream contained, in order:
`file.processing.started` (0), `file.processing.file_ready` (100), and
`file.processing.completed` (100). Its MIME type was `text/event-stream`, but
its body was newline-delimited JSON, not `data:`-framed SSE. Every record had
`file_id`, `event`, `message`, `progress` and `extra` fields. A file-ready event
or a progress value of 100 alone is not the final processing receipt.

After the control and private upload, the exact synthetic composer attachment
was removed, the synthetic draft cleared, and the UI returned to conversation
home. Remote synthetic artifacts were not deleted through an unverified API.
The original production APK was restored with `-r`; its installed hash was read
back and matched
`ef29913013d10a170e16a1ce7d8a2648377495edabeb3f0c6fb62c26eb67755c`.
The temporary CDP forward was removed. No Cookie/app-data reset, microphone
operation, account switch, proxy change, or public release occurred.

## Source contract

The owned research WebView supplied only its currently loaded static HTTPS JS
modules. Inline boot/session data and request headers were not exported.
Vendor source files remain outside Git; only protocol findings are recorded here.

- `/cdn/assets/8b34dbc2-kjj15hg4y6iyx13p.js`, SHA-256
  `9990fb9a8682917d0d790acf7b6aa78355e8520e4ffd2c5e0a183212d612d4b5`:
  creation (`SGt`/`jGt`), upload owner (`VGt`), byte transport (`WGt`/`qGt`),
  processing (`zGt`/`RGt`) and reservation claim (`BGt`).
- `/cdn/assets/conversation-small-hiw4wce20lu6te81.js`, SHA-256
  `296ec15ad991764de750c55f3c85b1643c8f385236b9402168fa4348696e37d1`:
  reservation validation/expiry, direct-blob headers and the NDJSON decoder.

These are evidence fingerprints, not pinned imports in production code.

The confirmed legacy path is:

1. `POST /backend-api/files` with file name, size, use case, MIME type,
   timezone offset, entry surface and explicit library preferences. The candidate
   requests `supports_direct_azure_multipart=false`; the small-file trial accepted
   this and returned `status`, `file_id` and `upload_url`.
2. PUT bytes to the returned signed HTTPS blob URL, with credentials omitted,
   no page authorization/workspace headers, and redirects rejected. The observed
   host was under `oaiusercontent.com`. Current Azure uploads use blob-type and
   API-version headers; static source also describes the AWS-signed variant.
3. `POST /backend-api/files/process_upload_stream` for that exact file ID and
   explicit retrieval/library metadata. Read the bounded complete NDJSON stream.
   Require matching file IDs and a final `file.processing.completed` event at 100.

The reservation optimization is a separate protocol:
`/backend-api/files/upload_reservations`, a short-lived upload URL, then
`/backend-api/files/upload_reservations/{reservation_id}/claim_and_finish`.
Creation/reservation does not mean bytes were accepted or indexing finished.
This path is documented but intentionally not implemented by the candidate.

## Ownership and limits

- `chatgpt_web_private_attachment_protocol.js` validates request context, blob
  destinations and processing receipts. `chatgpt_web_private_attachment_transport.js`
  owns one transaction, using the existing bounded JSON/text request helper and
  private identity context. The adapter 265 source candidate now loads these
  modules and connects a narrowly scoped native attachment owner. It has not
  been packaged or publicly released in this integration batch.
- No DOM lookup, menu click, file picker, imported vendor uploader, private
  credential persistence, automatic retry, automatic fallback, or background poll.
- Caller must bind provider, account, document and conversation generation through
  `isCurrent(binding)`. URL and cancellation guards also run between every stage.
  Options are snapshotted before awaiting authentication.
- Limits: one file at a time, 8 MiB per file, 120-character name, 7-second auth
  deadline, 15-second creation, 30-second upload and processing deadlines,
  256 KiB processing body and 256 records. Failure after dispatch reports possible
  side effects and a short cooldown; it never replays a write.
- Result says `associated=false`. A processed file is not a sent message.
  Signed URLs and identity headers are not included in completion/diagnostic
  receipts. Unknown server error details are reduced to a stable error code.
- Images, project/temporary contexts, custom upload headers, multipart, Estuary
  and unknown storage hosts are rejected explicitly. Do not infer support for
  these variants from the successful small-text trial.

## Next integration

1. Accept the new native integration from the ordinary production composer, not
   by supplying a browser `File` manually. Native byte handoff and official-store
   association are now implemented; their device verification is still pending.
2. Send exactly once and require a reply that reads the fixed fixture's first
   line without placing that content in the prompt. Existing native lifecycle
   smoke already enforces this. Do not count the control's file-store readiness
   or the private upload's processed receipt as this acceptance.
3. Extend only the remaining protocol variants above with actual evidence; then
   include the integrated owner in the next grouped APK and enable its verified
   scope. No repeated build or broad voice regression is needed for this module.

## Checks

`node --test scripts/test-chatgpt-web-private-attachment-transport.js`: 17 focused
checks passed, including no credential forwarding to blob storage, malformed and
truncated streams, intermediate file-ready events, matching file IDs, unsafe
destinations, cancellation, context changes, single-flight, timeout and no replay.
The actual small-text private upload was verified in the owned research WebView;
the final file-ID/terminal-event guards were subsequently covered by these tests.
Production native file selection and attachment-message association remain unverified.

## Native integration candidate

`ChatGptWebNativeAttachmentGateway` serves only the already staged, selected
FileProvider URI through a main-frame, exact-origin WebMessage listener. A random
lease is bound to the document token, page generation and URL, with a 120-second
lifetime. Reads are sequential 64 KiB chunks on one I/O executor. No arbitrary
path, URI, page authorization or Cookie is accepted from JavaScript. Cancellation,
navigation and teardown revoke the reader; truncation/expansion fails the read.

`chatgpt_web_native_attachment_source.js` assembles one bounded `File` without
opening an official file picker. The private send module uses the verified
create/PUT/process transport. Its current integration scope is **one plain-text
file, at most 8 MiB, in an empty ordinary new chat at the exact root URL**.
Existing conversations, projects, temporary chats, other MIME types and occupied
official composers still select the compatible official upload route before any
private write. An uncertain result after starting the private path does not
trigger a second upload automatically.

`chatgpt_web_private_attachment_composer.js` finds the current official
FilePickerContext via the file input's bounded React-fiber ancestry. This is still
a versioned website-runtime association, not an independent native send protocol.
The observed store uses `files$.set(...)`, `readyFiles$()` and
`hasUploadInProgress$()`. Association requires the final processing receipt,
matching file metadata, unchanged page/account/model/store identity, and exact
readback from the official ready-files signal. It never rewrites unrelated files.
Native readiness can then come from this confirmed store even if hidden DOM
attachment chips cannot be read. Consumed files are not recreated from a cache.

The existing `ChatGptWebSendOwner` still reserves the sole send slot and dispatches
text only after attachment readiness. Private upload receipts carry the reserved
request ID; a late failure cannot cancel a replacement send. The byte queue is
cleared only by a private association receipt or the existing official chooser,
and the native lease is revoked on failure, cancellation or completion.

The focused JS integration suites passed 37 checks, including parsing the complete
production adapter asset bundle, byte bounds, route/document/account/model changes,
store replacement, association readback, single-flight, and no write replay.
Release source compilation and 26 Android tests passed: reader 4, send owner 10,
tracker 12. Two stale send-owner tests were corrected to require actual file
readiness rather than generic transport completion, matching the already-fixed
reservation policy. This is not a full Android-suite or device pass. No APK,
microphone or proxy changes are part of this integration batch.
