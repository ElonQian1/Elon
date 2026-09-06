# Bounded private protocol evidence

Status: published/installed `v1.1.1540`, 2026-09-06. This is diagnostic infrastructure, not a
completed uploader, downloader, private sender, or share/delete implementation.
The outstanding contracts remain in [the batch work list](web-ai-private-native-remaining-batch.md).

## Purpose

The installed release can expose native semantic commands but not arbitrary
WebView debugging. Its old research observer also truncates ordinary command
details to 160 characters. Neither limitation should cause repeated screenshots,
guessed request bodies, or an APK rebuild for each missing endpoint.

The existing request observer now supports a bounded, explicit capture command.
It uses the existing JSON body deadline helper and native command ledger, not a
second transport, HTTP client, continuous network logger, or test UI.

## Native command

Call APK MCP `ui_control` with:

```json
{"action":"chatgpt_private_protocol_probe","mode":"start"}
```

Modes are `start`, `read`, `stop`, and `clear`. The dispatch receipt contains a
request ID; inspect the matching `command_requests` result in `ui_state`.
Its expected web action is `private_protocol_probe`. The detail is validated
JSON with schema `elon.private_protocol_probe.v1`, not an unbounded raw body.
Check the receipt's success and schema before using it as evidence.

Start once, perform a small group of authorized official operations, then read
and stop. Do not poll repeatedly or automatically restart capture. Unrelated
chat actions and microphone state are untouched. No command performs an HTTP
request, selects a file, starts recording, sends a message, or changes a login.

## Limits and ownership

- Default inactive, with no capture timer or body read.
- Only same-origin `/backend-api/`, `/api/`, and `/ces/` fetch/XHR requests.
- Excludes the observed `/ces/v1/rgstr`, `/ces/v1/t` and `/ces/v1/telemetry/intake` statistics
  routes before reading bodies or spending the 12-record budget. Other routes
  remain observable. This follow-up is source-only until the next grouped APK.
- Reuses the existing path sanitizer; removes query strings and normalizes
  encoded segments and long identifiers. Paths are structural diagnostics, not
  a credential source or proof that an endpoint is safe to replay.
- At most 12 records per start; automatic stop after 60 seconds, with an elapsed
  time check when resuming after background timer throttling.
- At most 64 KiB per JSON clone, a 2-second read deadline, 12 typed field paths
  per side, three nesting levels, and 12,000 output characters.
- Only method, normalized path, status, transport, body kind/read status and
  field names/types. No request headers, field values, filenames or file bytes.
- Binary and SSE bodies are skipped. JSON failure response types are observable
  without changing the status or consuming the original response.
- Stop aborts clone readers; late callbacks cannot update a stopped or restarted
  capture. Clear removes the probe's records, not older bounded native receipts.
- Native diagnostic receipts stop before the usual chat feedback/snapshot
  dispatch. Reading evidence does not refresh conversation content.
- No persistence, upload, retries, DOM scan, or new production menu. The Windows
  raw-response research feature is separate and is not enabled by this command.

## Verification

Focused Node tests cover inactive pass-through, request/response clone ownership,
error JSON, multipart metadata, bounded output, deadlines, stop/restart races,
origin filtering, XHR and diagnostic failures. Android tests cover strict result
validation, ordinary receipt bounds, MCP dispatch and document-generation gates.

2026-09-06 checks: 21 Node checks passed (20 focused cases plus the existing
research-probe suite); Android Kotlin compilation and 33 targeted unit tests
passed across evidence validation, MCP, existing protocol and file-index receipt
tests. The subsequent grouped Release build, publish and data-preserving install
passed on `v1.1.1540`; no microphone or actual protocol capture was performed
during that initial installation round. Later device results follow below.

These tests use synthetic fixtures and do not establish current vendor request
contracts. See subsequent [delivery evidence](web-ai-private-native-remaining-batch.md#grouped-release). Endpoint
field types alone are not sufficient to implement credential issuance, upload
finalization, idempotency or transaction ownership; verify those semantics before
replacing any working production path.

## First device capture, 2026-09-06

On installed `v1.1.1540` / adapter `264`, the production native composer staged
the synthetic `fixed_ascii_text_v1` fixture and invoked `send_input` once. Its
receipt entered `uploading`; no text-send receipt or new conversation was observed.
The 60-second capture recorded one JSON `POST /backend-api/files/{id}` with fields
`intended_use_case`, `entry_surface`, `requires_gizmo_id`, `store_in_library` and
`library_persistence_mode`. The path is sanitized, not a replayable endpoint.
All response records remained status `0` and became `cancelled` when the capture
lease expired. This does not establish an HTTP error or an upload contract.

The 12-record budget was also saturated by the two statistics routes above.
Their exact exclusion is covered by focused tests for body-read avoidance,
fetch/XHR, saturated budgets and preservation of unrelated routes. No additional
requests or retries are introduced. All 23 focused Node cases, the existing
research-probe suite and source-size checks passed; this follow-up was not
separately compiled or installed.

Device network checks found validated Wi-Fi but no active VPN and timeouts to
both vendor sites. The user then reported accelerator startup crashes. Crash
logs on accelerator `1.0.139 (140)` showed `UnsatisfiedLinkError` for
`ProxyCoreNative.nativeRestoreFakeIpState`. The existing accelerator task owns
that JNI/package repair; no proxy code or settings were changed by this task.
Resume protocol acceptance only after network recovery. Do not replay the
pending write automatically or mark attachment prepare/upload/finalize complete.

## Recovered-network capture

The accelerator owner installed `1.0.140 (141)` and confirmed cold-start survival,
the previously failing JNI restore, VPN operation and connectivity. This task
then resumed the existing `v1.1.1540` production UI without reinstalling it or
changing login state. The earlier upload owner was idle, no fixture was staged,
the exact synthetic draft was preserved, and no conversation had been created.

One new authorized native fixture/send attempt completed with no pending
attachments and an empty composer. A new conversation and non-streaming native
assistant acknowledgement were observed. The file reservation returned HTTP 200
with typed fields `eligible`, `reservation_id`, `upload_url`,
`upload_url_expires_at` and `reservation_expires_at`; no values were retained.
Official `/backend-api/f/conversation/prepare` and the conversation SSE request
both returned 200. The lease was stopped and cleared. The test conversation was
retained, and the UI returned through an empty new-chat state to conversation
home, with an empty draft, zero pending files and attachment owner `idle`.

This verifies the diagnostic on real requests and a working native-to-official
send/stream path after recovery. An acknowledgement alone does not prove file
contents were processed. It does not verify the complete upload/finalization
protocol, a private native uploader, or a private text sender. No credential or
challenge result was replayed. The capture also identified `/ces/v1/t` statistics
as additional budget pressure; its exact exclusion joins the source-only fix
for the next grouped build, with the same 23 focused cases and observer suite.

## Reservation completion regression

The recovered-network trace exposed a source-level bug in the existing official
upload owner: observer revision 1 treated any successful `POST /backend-api/files/`
child route as a completed file after 650 ms. The observed response only reserved
an upload URL. `ChatGptWebAttachmentSendTracker` then allowed that count to release
the text send without any new ready attachment in the composer. The prior native
acknowledgement therefore cannot certify attachment delivery or file processing.

Observer revision 2 retains wire schema 1 but emits progress hints only; generic
HTTP success never increments `completedCount` or schedules completion. Upgrade
cancels the old observer and reinjection is idempotent. The native tracker also
rejects legacy network completion counts, requires new ready attachments and an
available non-streaming composer, and excludes messages observed before dispatch
from send acknowledgement. Existing cancellation and bounded timeout remain.

The focused Node test reproduced the incorrect `completed` event before the fix
and passes with the corrected observer. Release Kotlin compilation and all 12
attachment tracker tests passed, covering multiple reservation receipts, old
attachments, pre-dispatch messages and composer gates. The attachment policy
suite also passed.

The existing native smoke previously asked the model to echo a supplied marker;
it now also requires the first line read from the synthetic file, without placing
that expected file content in the prompt. Its offline PowerShell contract rejects
prompt-only echoes, user messages and fragments spread across separate replies.
The same test verifies fixture agreement and Markdown escaping. All passed.
No new device send, microphone operation or APK publication was performed in this
correction round. This is a correction to the existing upload path and acceptance,
not a new private uploader or proof of the complete prepare/upload/finalize protocol.
It is source-only pending the next grouped APK and synthetic-file device acceptance.
