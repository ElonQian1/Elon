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
- Excludes the observed `/ces/v1/rgstr` and `/ces/v1/telemetry/intake` statistics
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
passed on `v1.1.1540`; no microphone or actual protocol capture was performed.

These tests use synthetic fixtures and do not establish current vendor request
contracts. Device capture remains pending; see [delivery evidence](web-ai-private-native-remaining-batch.md#grouped-release). Endpoint
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
